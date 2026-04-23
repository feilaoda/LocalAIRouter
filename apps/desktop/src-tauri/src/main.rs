#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream, UdpSocket};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use chrono::Utc;
use localairouter_core::{
    AccountInput, AppPaths, AppSettings, AppSettingsInput, Repository, RouteBindingInput,
    load_app_settings, save_app_settings,
};
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use tauri::menu::{MenuBuilder, MenuEvent, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{ActivationPolicy, AppHandle, Manager, Url, WindowEvent};
use toml_edit::{DocumentMut, Item, Table, value};
use tracing::{error, warn};

const DAEMON_BINARY_NAME: &str = "localairouter-daemon";
const DAEMON_PORT_ENV: &str = "LOCALAIROUTER_PORT";
const LEGACY_DAEMON_PORT_ENV: &str = "LOCALOPENROUTER_PORT";
const OLDER_DAEMON_PORT_ENV: &str = "LOCALROUTER_PORT";
const DAEMON_ALLOW_LAN_ENV: &str = "LOCALAIROUTER_ALLOW_LAN";
const LEGACY_DAEMON_ALLOW_LAN_ENV: &str = "LOCALOPENROUTER_ALLOW_LAN";
const OLDER_DAEMON_ALLOW_LAN_ENV: &str = "LOCALROUTER_ALLOW_LAN";
const DAEMON_PARENT_PID_ENV: &str = "LOCALAIROUTER_PARENT_PID";
const LEGACY_DAEMON_PARENT_PID_ENV: &str = "LOCALOPENROUTER_PARENT_PID";
const OLDER_DAEMON_PARENT_PID_ENV: &str = "LOCALROUTER_PARENT_PID";
const DEFAULT_DAEMON_PORT: u16 = 16321;
const UI_DEV_ENV: &str = "LOCALAIROUTER_UI_DEV";
const LEGACY_UI_DEV_ENV: &str = "LOCALOPENROUTER_UI_DEV";
const UI_DEV_PORT_ENV: &str = "LOCALAIROUTER_UI_DEV_PORT";
const LEGACY_UI_DEV_PORT_ENV: &str = "LOCALOPENROUTER_UI_DEV_PORT";
const DEFAULT_UI_DEV_PORT: u16 = 7456;
const TRAY_ID: &str = "localairouter-tray";
const TRAY_MENU_OPEN_DASHBOARD: &str = "tray.open_dashboard";
const TRAY_MENU_QUIT: &str = "tray.quit";
const TRAY_MENU_SET_DEFAULT_PREFIX: &str = "tray.set_default";
const TRAY_MENU_PROVIDER_HEADING_PREFIX: &str = "tray.provider_heading";
#[cfg(target_os = "macos")]
const MACOS_APP_ICON_PNG: &[u8] = include_bytes!("../icons/icon.png");
const LOCAL_ROUTER_MANAGED_SECRET: &str = "localairouter-managed";

#[derive(Default)]
struct DaemonRuntime {
    child: Option<Child>,
    pid: Option<u32>,
    started_at: Option<String>,
    launch_mode: Option<String>,
    command_path: Option<String>,
    last_error: Option<String>,
    last_exit: Option<String>,
}

#[derive(Clone)]
struct DaemonSupervisor {
    runtime: Arc<Mutex<DaemonRuntime>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DaemonStatus {
    running: bool,
    port: u16,
    pid: Option<u32>,
    started_at: Option<String>,
    launch_mode: Option<String>,
    command_path: Option<String>,
    log_file_path: String,
    last_error: Option<String>,
    last_exit: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CodexConfigSyncResult {
    config_path: String,
    profile_key: String,
    base_url: String,
    default_provider_updated: bool,
    previous_model_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeConfigSyncResult {
    config_path: String,
    base_url: String,
}

struct SpawnSpec {
    program: PathBuf,
    args: Vec<String>,
    launch_mode: &'static str,
    command_path: String,
}

#[derive(Debug, Clone)]
struct UiDevServer {
    url: Url,
}

#[derive(Clone)]
struct LocaleState {
    current: Arc<Mutex<String>>,
}

#[derive(Debug, Clone)]
struct TrayProviderState {
    slug: String,
    display_name: String,
    enabled: bool,
    default_account_id: Option<String>,
    accounts: Vec<TrayAccountState>,
}

#[derive(Debug, Clone)]
struct TrayAccountState {
    id: String,
    name: String,
}

impl LocaleState {
    fn new() -> Self {
        Self {
            current: Arc::new(Mutex::new(detect_locale())),
        }
    }

    fn get(&self) -> String {
        self.current.lock().expect("locale mutex").clone()
    }

    fn set(&self, value: &str) -> String {
        let normalized = normalize_locale(value);
        *self.current.lock().expect("locale mutex") = normalized.clone();
        normalized
    }
}

impl DaemonSupervisor {
    fn new() -> Self {
        Self {
            runtime: Arc::new(Mutex::new(DaemonRuntime::default())),
        }
    }

    fn start(&self) -> Result<DaemonStatus> {
        let mut runtime = self.runtime.lock().expect("daemon runtime mutex");
        self.sync_locked(&mut runtime);
        if runtime.child.is_some() {
            return self.snapshot_locked(&runtime);
        }

        let log_file = daemon_log_file()?;
        let (child, spec) = spawn_daemon_process(&log_file)?;
        let pid = child.id();
        runtime.child = Some(child);
        runtime.pid = Some(pid);
        runtime.started_at = Some(timestamp());
        runtime.launch_mode = Some(spec.launch_mode.into());
        runtime.command_path = Some(spec.command_path);
        runtime.last_error = None;
        runtime.last_exit = None;
        append_supervisor_event(&log_file, format!("desktop: daemon started with pid {pid}"))?;
        thread::sleep(Duration::from_millis(150));
        self.sync_locked(&mut runtime);
        self.snapshot_locked(&runtime)
    }

    fn stop(&self) -> Result<DaemonStatus> {
        let mut runtime = self.runtime.lock().expect("daemon runtime mutex");
        self.sync_locked(&mut runtime);
        let log_file = daemon_log_file()?;
        if let Some(mut child) = runtime.child.take() {
            let pid = child.id();
            append_supervisor_event(&log_file, format!("desktop: stopping daemon pid {pid}"))?;
            let stopped_as = terminate_child(&mut child)?;
            runtime.last_exit = Some(format!("stopped: {stopped_as}"));
            runtime.last_error = None;
        } else {
            runtime.last_exit = Some("already stopped".into());
        }
        runtime.pid = None;
        runtime.started_at = None;
        self.snapshot_locked(&runtime)
    }

    fn restart(&self) -> Result<DaemonStatus> {
        let _ = self.stop()?;
        self.start()
    }

    fn status(&self) -> Result<DaemonStatus> {
        let mut runtime = self.runtime.lock().expect("daemon runtime mutex");
        self.sync_locked(&mut runtime);
        self.snapshot_locked(&runtime)
    }

    fn shutdown(&self) {
        if let Err(error) = self.stop() {
            warn!("failed to stop daemon during shutdown: {error:#}");
        }
    }

    fn sync_locked(&self, runtime: &mut DaemonRuntime) {
        let Some(child) = runtime.child.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                runtime.last_exit = Some(exit_status_label(status));
                if !status.success() {
                    runtime.last_error = read_last_log_line().ok().flatten();
                }
                runtime.pid = None;
                runtime.started_at = None;
                runtime.child = None;
            }
            Ok(None) => {
                runtime.pid = Some(child.id());
            }
            Err(error) => {
                runtime.last_error = Some(format!("failed to inspect daemon process: {error}"));
            }
        }
    }

    fn snapshot_locked(&self, runtime: &DaemonRuntime) -> Result<DaemonStatus> {
        Ok(DaemonStatus {
            running: runtime.child.is_some(),
            port: configured_daemon_port(),
            pid: runtime.pid,
            started_at: runtime.started_at.clone(),
            launch_mode: runtime.launch_mode.clone(),
            command_path: runtime.command_path.clone(),
            log_file_path: daemon_log_file()?.to_string_lossy().into_owned(),
            last_error: runtime.last_error.clone(),
            last_exit: runtime.last_exit.clone(),
        })
    }
}

fn main() {
    ignore_terminal_hangup();

    tracing_subscriber::fmt()
        .with_env_filter("localairouter_desktop=info")
        .with_target(false)
        .compact()
        .init();

    let supervisor = DaemonSupervisor::new();
    let locale_state = LocaleState::new();
    if let Err(error) = supervisor.start() {
        error!("daemon did not start during setup: {error:#}");
    }
    let ui_dev_server = match start_ui_dev_server() {
        Ok(server) => server,
        Err(error) => {
            warn!("ui dev server startup failed: {error:#}");
            None
        }
    };

    let shutdown_supervisor = supervisor.clone();
    tauri::Builder::default()
        .on_menu_event(handle_tray_menu_event)
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                hide_app_to_tray(window.app_handle());
            }
        })
        .manage(supervisor)
        .manage(locale_state)
        .invoke_handler(tauri::generate_handler![
            daemon_status,
            start_daemon,
            stop_daemon,
            restart_daemon,
            open_daemon_log,
            open_log_file,
            open_logs_root,
            pick_logs_directory,
            get_app_settings,
            save_app_settings_command,
            local_lan_ip,
            write_clipboard_text,
            sync_codex_config,
            sync_claude_config,
            import_codex_account,
            import_claude_account,
            refresh_tray_menu,
            set_app_locale
        ])
        .setup(move |app| {
            if let Some(ui_dev_server) = &ui_dev_server {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(error) = window.navigate(ui_dev_server.url.clone()) {
                        warn!("failed to navigate main window to ui dev server: {error}");
                    }
                }
            }
            #[cfg(target_os = "macos")]
            sync_macos_app_icon(app.handle());
            ensure_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .inspect_err(|error| error!("tauri run failure: {error}"))
        .ok();
    shutdown_supervisor.shutdown();
}

#[cfg(unix)]
fn ignore_terminal_hangup() {
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
    }
}

#[cfg(not(unix))]
fn ignore_terminal_hangup() {}

#[tauri::command]
fn daemon_status(
    supervisor: tauri::State<'_, DaemonSupervisor>,
) -> std::result::Result<DaemonStatus, String> {
    supervisor.status().map_err(|error| error.to_string())
}

#[tauri::command]
fn start_daemon(
    supervisor: tauri::State<'_, DaemonSupervisor>,
) -> std::result::Result<DaemonStatus, String> {
    supervisor.start().map_err(|error| error.to_string())
}

#[tauri::command]
fn stop_daemon(
    supervisor: tauri::State<'_, DaemonSupervisor>,
) -> std::result::Result<DaemonStatus, String> {
    supervisor.stop().map_err(|error| error.to_string())
}

#[tauri::command]
fn restart_daemon(
    supervisor: tauri::State<'_, DaemonSupervisor>,
) -> std::result::Result<DaemonStatus, String> {
    supervisor.restart().map_err(|error| error.to_string())
}

#[tauri::command]
fn open_daemon_log() -> std::result::Result<(), String> {
    let path = daemon_log_file().map_err(|error| error.to_string())?;
    if !path.exists() {
        return Err(format!("daemon log not found: {}", path.display()));
    }
    reveal_path(path)
}

#[tauri::command]
fn open_log_file(relative_path: String) -> std::result::Result<(), String> {
    let path = resolve_storage_path(relative_path)?;
    if !path.exists() {
        return Err(format!("log file not found: {}", path.display()));
    }
    reveal_path(path)
}

#[tauri::command]
fn open_logs_root() -> std::result::Result<(), String> {
    let path = AppPaths::discover()
        .map_err(|error| error.to_string())?
        .logs;
    if !path.exists() {
        return Err(format!("log root not found: {}", path.display()));
    }
    reveal_path(path)
}

#[tauri::command]
async fn pick_logs_directory(
    initial_path: Option<String>,
) -> std::result::Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || pick_logs_directory_native(initial_path))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
fn get_app_settings() -> std::result::Result<AppSettings, String> {
    let paths = AppPaths::discover().map_err(|error| error.to_string())?;
    load_app_settings(&paths).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_app_settings_command(input: AppSettingsInput) -> std::result::Result<AppSettings, String> {
    let paths = AppPaths::discover().map_err(|error| error.to_string())?;
    save_app_settings(&paths, &input).map_err(|error| error.to_string())
}

#[tauri::command]
fn local_lan_ip() -> Option<String> {
    detect_primary_lan_ip().map(|ip| ip.to_string())
}

#[tauri::command]
fn write_clipboard_text(text: String) -> std::result::Result<(), String> {
    write_clipboard_text_native(&text).map_err(|error| error.to_string())
}

#[tauri::command]
fn sync_codex_config(
    provider_slug: String,
    provider_name: String,
    base_url: String,
) -> std::result::Result<CodexConfigSyncResult, String> {
    sync_codex_config_to_disk(&provider_slug, &provider_name, &base_url)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn sync_claude_config(base_url: String) -> std::result::Result<ClaudeConfigSyncResult, String> {
    sync_claude_config_to_disk(&base_url).map_err(|error| error.to_string())
}

#[tauri::command]
fn import_codex_account() -> std::result::Result<AccountInput, String> {
    import_codex_account_from_disk().map_err(|error| error.to_string())
}

#[tauri::command]
fn import_claude_account() -> std::result::Result<AccountInput, String> {
    import_claude_account_from_disk().map_err(|error| error.to_string())
}

#[tauri::command]
async fn refresh_tray_menu(
    app: AppHandle,
    locale_state: tauri::State<'_, LocaleState>,
) -> std::result::Result<(), String> {
    refresh_tray_menu_for_app_async(&app, locale_state.get())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_app_locale(
    locale: String,
    app: AppHandle,
    locale_state: tauri::State<'_, LocaleState>,
) -> std::result::Result<String, String> {
    let normalized = locale_state.set(&locale);
    refresh_tray_menu_for_app_async(&app, normalized.clone())
        .await
        .map_err(|error| error.to_string())?;
    Ok(normalized)
}

fn spawn_daemon_process(log_file: &Path) -> Result<(Child, SpawnSpec)> {
    let spec = select_spawn_spec()?;
    append_supervisor_event(
        log_file,
        format!(
            "desktop: launching daemon via {} ({})",
            spec.launch_mode, spec.command_path
        ),
    )?;

    let mut command = Command::new(&spec.program);
    for arg in &spec.args {
        command.arg(arg);
    }
    configure_child(&mut command, log_file)?;
    let child = command
        .spawn()
        .with_context(|| format!("failed to spawn daemon using {}", spec.command_path))?;
    Ok((child, spec))
}

fn configure_child(command: &mut Command, log_file: &Path) -> Result<()> {
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent)?;
    }
    let port = configured_daemon_port().to_string();
    let allow_lan = if configured_allow_lan_access() {
        "1"
    } else {
        "0"
    };
    let parent_pid = std::process::id().to_string();
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    let stderr = stdout.try_clone()?;
    command
        .env(DAEMON_PORT_ENV, &port)
        .env(DAEMON_ALLOW_LAN_ENV, allow_lan)
        .env(DAEMON_PARENT_PID_ENV, &parent_pid)
        .env(LEGACY_DAEMON_PORT_ENV, &port)
        .env(LEGACY_DAEMON_ALLOW_LAN_ENV, allow_lan)
        .env(LEGACY_DAEMON_PARENT_PID_ENV, &parent_pid)
        .env(OLDER_DAEMON_PORT_ENV, &port)
        .env(OLDER_DAEMON_ALLOW_LAN_ENV, allow_lan)
        .env(OLDER_DAEMON_PARENT_PID_ENV, &parent_pid)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    #[cfg(unix)]
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    Ok(())
}

fn configured_daemon_port() -> u16 {
    let settings_port = AppPaths::discover()
        .ok()
        .and_then(|paths| load_app_settings(&paths).ok())
        .map(|settings| settings.daemon_port)
        .unwrap_or(DEFAULT_DAEMON_PORT);
    configured_port(
        &[
            DAEMON_PORT_ENV,
            LEGACY_DAEMON_PORT_ENV,
            OLDER_DAEMON_PORT_ENV,
        ],
        settings_port,
    )
}

fn configured_allow_lan_access() -> bool {
    let settings_allow_lan = AppPaths::discover()
        .ok()
        .and_then(|paths| load_app_settings(&paths).ok())
        .map(|settings| settings.allow_lan_access)
        .unwrap_or(false);
    parse_bool_env_list(&[
        DAEMON_ALLOW_LAN_ENV,
        LEGACY_DAEMON_ALLOW_LAN_ENV,
        OLDER_DAEMON_ALLOW_LAN_ENV,
    ])
    .unwrap_or(settings_allow_lan)
}

fn detect_primary_lan_ip() -> Option<IpAddr> {
    for target in ["8.8.8.8:80", "1.1.1.1:80"] {
        let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) else {
            continue;
        };
        if socket.connect(target).is_err() {
            continue;
        }
        let Ok(addr) = socket.local_addr() else {
            continue;
        };
        let ip = addr.ip();
        if !ip.is_loopback() && !ip.is_unspecified() {
            return Some(ip);
        }
    }
    None
}

fn sync_codex_config_to_disk(
    provider_slug: &str,
    provider_name: &str,
    base_url: &str,
) -> Result<CodexConfigSyncResult> {
    let config_path = codex_config_path()?;
    let current = fs::read_to_string(&config_path).unwrap_or_default();
    let (updated, profile_key, default_provider_updated, previous_model_provider) =
        sync_codex_config_contents(&current, provider_slug, provider_name, base_url)?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&config_path, updated)?;
    Ok(CodexConfigSyncResult {
        config_path: config_path.to_string_lossy().into_owned(),
        profile_key,
        base_url: base_url.trim().into(),
        default_provider_updated,
        previous_model_provider,
    })
}

fn sync_claude_config_to_disk(base_url: &str) -> Result<ClaudeConfigSyncResult> {
    let config_path = claude_config_path()?;
    let current = fs::read_to_string(&config_path).unwrap_or_default();
    let updated = sync_claude_config_contents(&current, base_url)?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&config_path, updated)?;
    Ok(ClaudeConfigSyncResult {
        config_path: config_path.to_string_lossy().into_owned(),
        base_url: base_url.trim().into(),
    })
}

fn codex_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory for ~/.codex")?;
    Ok(home.join(".codex").join("config.toml"))
}

fn codex_auth_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory for ~/.codex")?;
    Ok(home.join(".codex").join("auth.json"))
}

fn claude_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory for ~/.claude")?;
    Ok(home.join(".claude").join("settings.json"))
}

fn import_codex_account_from_disk() -> Result<AccountInput> {
    let config_path = codex_config_path()?;
    let current = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let auth = codex_auth_path()
        .ok()
        .filter(|path| path.exists())
        .and_then(|path| fs::read_to_string(path).ok());
    import_codex_account_contents(&current, auth.as_deref())
}

fn import_claude_account_from_disk() -> Result<AccountInput> {
    let config_path = claude_config_path()?;
    let current = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    import_claude_account_contents(&current)
}

fn sync_codex_config_contents(
    current: &str,
    provider_slug: &str,
    provider_name: &str,
    base_url: &str,
) -> Result<(String, String, bool, Option<String>)> {
    let provider_slug = provider_slug.trim();
    if provider_slug.is_empty() {
        return Err(anyhow::anyhow!("provider slug is required"));
    }
    let base_url = base_url.trim();
    if base_url.is_empty() {
        return Err(anyhow::anyhow!("base_url is required"));
    }

    let mut document = if current.trim().is_empty() {
        DocumentMut::new()
    } else {
        current
            .parse::<DocumentMut>()
            .context("failed to parse ~/.codex/config.toml")?
    };

    let managed_profile_key = codex_profile_key(provider_slug);
    let profile_name = if provider_name.trim().is_empty() {
        format!("LocalAIRouter {provider_slug}")
    } else {
        format!("LocalAIRouter {}", provider_name.trim())
    };

    let previous_model_provider = document
        .get("model_provider")
        .and_then(Item::as_str)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let default_provider_updated = previous_model_provider.is_none();
    let profile_key = previous_model_provider
        .clone()
        .unwrap_or_else(|| managed_profile_key.clone());
    if default_provider_updated {
        document["model_provider"] = value(profile_key.clone());
    }

    let providers = ensure_table(document.as_table_mut(), "model_providers")?;
    let provider = ensure_child_table(providers, &profile_key)?;
    if !default_provider_updated {
        reconcile_codex_base_url_backups(provider, base_url);
    }
    provider["base_url"] = value(base_url);
    if default_provider_updated {
        provider["name"] = value(profile_name);
        provider["wire_api"] = value("responses");
        provider["requires_openai_auth"] = value(true);
    }

    Ok((
        document.to_string(),
        profile_key,
        default_provider_updated,
        previous_model_provider,
    ))
}

fn sync_claude_config_contents(current: &str, base_url: &str) -> Result<String> {
    let base_url = base_url.trim();
    if base_url.is_empty() {
        return Err(anyhow::anyhow!("base_url is required"));
    }

    let mut root = if current.trim().is_empty() {
        JsonValue::Object(JsonMap::new())
    } else {
        serde_json::from_str::<JsonValue>(current)
            .context("failed to parse ~/.claude/settings.json")?
    };

    let root_object = root.as_object_mut().ok_or_else(|| {
        anyhow::anyhow!("expected ~/.claude/settings.json to contain a JSON object")
    })?;

    let env_value = root_object
        .entry("env")
        .or_insert_with(|| JsonValue::Object(JsonMap::new()));
    let env_object = env_value.as_object_mut().ok_or_else(|| {
        anyhow::anyhow!("expected `env` in ~/.claude/settings.json to be a JSON object")
    })?;
    reconcile_claude_env_backups(env_object, "ANTHROPIC_BASE_URL", base_url);
    reconcile_claude_env_backups(env_object, "ANTHROPIC_API_KEY", "localairouter-managed");
    env_object.insert("ANTHROPIC_BASE_URL".into(), json!(base_url));
    env_object.insert("ANTHROPIC_API_KEY".into(), json!("localairouter-managed"));

    let mut output = serde_json::to_string_pretty(&root)?;
    output.push('\n');
    Ok(output)
}

fn import_codex_account_contents(current: &str, auth_json: Option<&str>) -> Result<AccountInput> {
    let document = current
        .parse::<DocumentMut>()
        .context("failed to parse ~/.codex/config.toml")?;
    let provider_key = document
        .get("model_provider")
        .and_then(Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            document
                .get("model_providers")
                .and_then(Item::as_table)
                .and_then(|providers| providers.iter().next().map(|(key, _)| key.to_owned()))
        })
        .ok_or_else(|| anyhow::anyhow!("no model provider found in ~/.codex/config.toml"))?;
    let provider = document
        .get("model_providers")
        .and_then(Item::as_table)
        .and_then(|providers| providers.get(&provider_key))
        .and_then(Item::as_table)
        .ok_or_else(|| anyhow::anyhow!("missing [model_providers.{provider_key}] table"))?;

    let name = provider
        .get("name")
        .and_then(Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| provider_key.clone());
    let base_url = preferred_remote_url(codex_base_url_candidates(provider))
        .filter(|value| value != "https://api.openai.com");
    let api_key = resolve_codex_api_key(provider, auth_json)?;

    Ok(AccountInput {
        id: None,
        provider: "codex".into(),
        name,
        base_url,
        default_model: None,
        api_key: Some(api_key),
        note: None,
        enabled: true,
    })
}

fn import_claude_account_contents(current: &str) -> Result<AccountInput> {
    let root = serde_json::from_str::<JsonValue>(current)
        .context("failed to parse ~/.claude/settings.json")?;
    let env = root
        .as_object()
        .and_then(|object| object.get("env"))
        .and_then(JsonValue::as_object)
        .ok_or_else(|| {
            anyhow::anyhow!("expected `env` in ~/.claude/settings.json to be a JSON object")
        })?;

    let mut secret_candidates = Vec::new();
    if let Some(value) = json_string(env.get("ANTHROPIC_API_KEY")) {
        secret_candidates.push(value);
    }
    if let Some(value) = json_string(env.get("ANTHROPIC_AUTH_TOKEN")) {
        secret_candidates.push(value);
    }
    secret_candidates.extend(json_backup_values(env, "ANTHROPIC_API_KEY"));
    secret_candidates.extend(json_backup_values(env, "ANTHROPIC_AUTH_TOKEN"));
    let api_key = preferred_secret(secret_candidates).ok_or_else(|| {
        anyhow::anyhow!("no usable Anthropic secret found in ~/.claude/settings.json")
    })?;

    let base_url = preferred_remote_url(claude_base_url_candidates(env))
        .filter(|value| value != "https://api.anthropic.com");

    Ok(AccountInput {
        id: None,
        provider: "claude-code".into(),
        name: "Claude Code".into(),
        base_url,
        default_model: None,
        api_key: Some(api_key),
        note: None,
        enabled: true,
    })
}

fn reconcile_claude_env_backups(
    env_object: &mut JsonMap<String, JsonValue>,
    key: &str,
    next: &str,
) {
    let mut candidates = Vec::new();
    if let Some(existing) = env_object
        .get(key)
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
    {
        candidates.push(existing);
    }
    candidates.extend(existing_claude_env_backups(env_object, key));

    clear_claude_env_backups(env_object, key);

    let mut unique_values = Vec::new();
    for candidate in candidates {
        if candidate == next || unique_values.iter().any(|value| value == &candidate) {
            continue;
        }
        unique_values.push(candidate);
    }

    for (index, backup_value) in unique_values.into_iter().enumerate() {
        let backup_key = format!("{key}{}", index + 1);
        env_object.insert(backup_key, json!(backup_value));
    }
}

fn existing_claude_env_backups(env_object: &JsonMap<String, JsonValue>, key: &str) -> Vec<String> {
    let mut backup_values = env_object
        .iter()
        .filter_map(|(entry_key, value)| {
            parse_claude_env_backup_index(entry_key, key).and_then(|index| {
                value
                    .as_str()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .map(|text| (index, text.to_owned()))
            })
        })
        .collect::<Vec<_>>();
    backup_values.sort_by_key(|(index, _)| *index);
    backup_values
        .into_iter()
        .map(|(_, value)| value)
        .collect::<Vec<_>>()
}

fn clear_claude_env_backups(env_object: &mut JsonMap<String, JsonValue>, key: &str) {
    let backup_keys = env_object
        .keys()
        .filter(|entry_key| parse_claude_env_backup_index(entry_key, key).is_some())
        .cloned()
        .collect::<Vec<_>>();
    for backup_key in backup_keys {
        env_object.remove(&backup_key);
    }
}

fn parse_claude_env_backup_index(entry_key: &str, key: &str) -> Option<usize> {
    entry_key
        .strip_prefix(key)
        .and_then(|suffix| suffix.parse::<usize>().ok())
        .filter(|index| *index > 0)
}

fn claude_base_url_candidates(env_object: &JsonMap<String, JsonValue>) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(value) = json_string(env_object.get("ANTHROPIC_BASE_URL")) {
        candidates.push(value);
    }
    candidates.extend(json_backup_values(env_object, "ANTHROPIC_BASE_URL"));
    candidates
}

fn json_backup_values(env_object: &JsonMap<String, JsonValue>, key: &str) -> Vec<String> {
    let mut values = env_object
        .iter()
        .filter_map(|(entry_key, value)| {
            parse_claude_env_backup_index(entry_key, key)
                .and_then(|index| json_string(Some(value)).map(|text| (index, text)))
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|(index, _)| *index);
    values.into_iter().map(|(_, value)| value).collect()
}

fn json_string(value: Option<&JsonValue>) -> Option<String> {
    value
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn preferred_remote_url<I>(candidates: I) -> Option<String>
where
    I: IntoIterator<Item = String>,
{
    for candidate in candidates {
        let normalized = candidate.trim();
        if normalized.is_empty() {
            continue;
        }
        let normalized = normalized.to_owned();
        if !is_loopback_base_url(&normalized) {
            return Some(normalized);
        }
    }
    None
}

fn is_loopback_base_url(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized.starts_with("http://127.0.0.1:")
        || normalized.starts_with("https://127.0.0.1:")
        || normalized.starts_with("http://localhost:")
        || normalized.starts_with("https://localhost:")
        || normalized.starts_with("http://[::1]:")
        || normalized.starts_with("https://[::1]:")
}

fn preferred_secret<I>(candidates: I) -> Option<String>
where
    I: IntoIterator<Item = String>,
{
    candidates
        .into_iter()
        .map(|candidate| candidate.trim().to_owned())
        .find(|candidate| {
            !candidate.is_empty()
                && candidate != LOCAL_ROUTER_MANAGED_SECRET
                && candidate != "***MASKED***"
        })
}

fn codex_base_url_candidates(provider: &Table) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(base_url) = provider
        .get("base_url")
        .and_then(Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        candidates.push(base_url.to_owned());
    }

    let mut backups = provider
        .iter()
        .filter_map(|(key, item)| {
            parse_codex_import_base_url_index(key).and_then(|index| {
                item.as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| (index, value.to_owned()))
            })
        })
        .collect::<Vec<_>>();
    backups.sort_by_key(|(index, _)| *index);
    candidates.extend(backups.into_iter().map(|(_, value)| value));
    candidates
}

fn parse_codex_import_base_url_index(key: &str) -> Option<usize> {
    key.strip_prefix("base_url")
        .and_then(|suffix| suffix.parse::<usize>().ok())
}

fn resolve_codex_api_key(provider: &Table, auth_json: Option<&str>) -> Result<String> {
    let mut candidates = Vec::new();
    for key in [
        "api_key",
        "openai_api_key",
        "OPENAI_API_KEY",
        "auth_token",
        "token",
    ] {
        if let Some(value) = provider
            .get(key)
            .and_then(Item::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            candidates.push(value.to_owned());
        }
    }

    for env_key_field in [
        "api_key_env",
        "api_key_env_var",
        "openai_api_key_env",
        "openai_api_key_env_var",
        "auth_token_env",
        "auth_token_env_var",
        "env_key",
    ] {
        if let Some(env_name) = provider
            .get(env_key_field)
            .and_then(Item::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if let Ok(value) = env::var(env_name) {
                candidates.push(value);
            }
        }
    }

    if let Some(auth_json) = auth_json {
        candidates.extend(codex_auth_json_candidates(auth_json)?);
    }

    if let Ok(value) = env::var("OPENAI_API_KEY") {
        candidates.push(value);
    }

    preferred_secret(candidates).ok_or_else(|| {
        anyhow::anyhow!(
            "no usable OpenAI secret found in ~/.codex/config.toml or ~/.codex/auth.json"
        )
    })
}

fn codex_auth_json_candidates(current: &str) -> Result<Vec<String>> {
    let root =
        serde_json::from_str::<JsonValue>(current).context("failed to parse ~/.codex/auth.json")?;
    let object = root
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("expected ~/.codex/auth.json to contain a JSON object"))?;
    let mut candidates = Vec::new();
    if let Some(value) = json_string(object.get("OPENAI_API_KEY")) {
        candidates.push(value);
    }
    let mut backups = object
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix("OPENAI_API_KEY-")
                .and_then(|suffix| suffix.parse::<usize>().ok())
                .and_then(|index| json_string(Some(value)).map(|candidate| (index, candidate)))
        })
        .collect::<Vec<_>>();
    backups.sort_by_key(|(index, _)| *index);
    candidates.extend(backups.into_iter().map(|(_, value)| value));
    Ok(candidates)
}

fn reconcile_codex_base_url_backups(provider: &mut Table, next_base_url: &str) {
    let mut candidates = Vec::new();
    if let Some(existing_base_url) = provider
        .get("base_url")
        .and_then(Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
    {
        candidates.push(existing_base_url);
    }
    candidates.extend(existing_codex_base_url_backups(provider));

    clear_codex_base_url_backups(provider);

    let mut unique_values = Vec::new();
    for candidate in candidates {
        if candidate == next_base_url || unique_values.iter().any(|value| value == &candidate) {
            continue;
        }
        unique_values.push(candidate);
    }

    for (index, backup_value) in unique_values.into_iter().enumerate() {
        let backup_key = format!("base_url{}", index + 1);
        provider[backup_key.as_str()] = value(backup_value);
    }
}

fn existing_codex_base_url_backups(provider: &Table) -> Vec<String> {
    let mut backup_values = provider
        .iter()
        .filter_map(|(key, item)| {
            parse_codex_base_url_backup_index(key).and_then(|index| {
                item.as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| (index, value.to_owned()))
            })
        })
        .collect::<Vec<_>>();
    backup_values.sort_by_key(|(index, _)| *index);
    backup_values
        .into_iter()
        .map(|(_, value)| value)
        .collect::<Vec<_>>()
}

fn clear_codex_base_url_backups(provider: &mut Table) {
    let keys = provider
        .iter()
        .filter_map(|(key, _)| parse_codex_base_url_backup_index(key).map(|_| key.to_owned()))
        .collect::<Vec<_>>();
    for key in keys {
        provider.remove(key.as_str());
    }
}

fn parse_codex_base_url_backup_index(key: &str) -> Option<usize> {
    key.strip_prefix("base_url")
        .and_then(|suffix| suffix.parse::<usize>().ok())
        .filter(|index| *index > 0)
}

fn codex_profile_key(provider_slug: &str) -> String {
    let sanitized: String = provider_slug
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches('_');
    if sanitized.is_empty() {
        "localairouter_codex".into()
    } else {
        format!("localairouter_{sanitized}")
    }
}

fn ensure_table<'a>(table: &'a mut Table, key: &str) -> Result<&'a mut Table> {
    if !table.contains_key(key) {
        table[key] = Item::Table(Table::new());
    }
    table[key]
        .as_table_mut()
        .with_context(|| format!("expected `{key}` to be a table in ~/.codex/config.toml"))
}

fn ensure_child_table<'a>(table: &'a mut Table, key: &str) -> Result<&'a mut Table> {
    if !table.contains_key(key) {
        table[key] = Item::Table(Table::new());
    }
    table[key].as_table_mut().with_context(|| {
        format!("expected `model_providers.{key}` to be a table in ~/.codex/config.toml")
    })
}

fn terminate_child(child: &mut Child) -> Result<String> {
    #[cfg(unix)]
    {
        let pgid = child.id() as i32;
        signal_process_group(pgid, libc::SIGTERM)?;
        for _ in 0..20 {
            if let Some(status) = child.try_wait()? {
                return Ok(exit_status_label(status));
            }
            thread::sleep(Duration::from_millis(100));
        }

        signal_process_group(pgid, libc::SIGKILL)?;
        for _ in 0..20 {
            if let Some(status) = child.try_wait()? {
                return Ok(exit_status_label(status));
            }
            thread::sleep(Duration::from_millis(100));
        }

        let status = child.wait()?;
        return Ok(exit_status_label(status));
    }

    #[cfg(not(unix))]
    {
        child.kill()?;
        let status = child.wait()?;
        Ok(exit_status_label(status))
    }
}

#[cfg(unix)]
fn signal_process_group(pgid: i32, signal: i32) -> Result<()> {
    let rc = unsafe { libc::killpg(pgid, signal) };
    if rc == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if matches!(error.raw_os_error(), Some(libc::ESRCH)) {
        Ok(())
    } else {
        Err(error.into())
    }
}

#[cfg(target_os = "macos")]
fn write_clipboard_text_native(text: &str) -> Result<()> {
    write_to_clipboard_command("/usr/bin/pbcopy", &[], text)
}

#[cfg(target_os = "windows")]
fn write_clipboard_text_native(text: &str) -> Result<()> {
    write_to_clipboard_command("cmd", &["/C", "clip"], text)
}

#[cfg(target_os = "linux")]
fn write_clipboard_text_native(text: &str) -> Result<()> {
    let commands: &[(&str, &[&str])] = &[
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
    ];
    let mut errors = Vec::new();
    for (program, args) in commands {
        match write_to_clipboard_command(program, args, text) {
            Ok(()) => return Ok(()),
            Err(error) => errors.push(format!("{program}: {error}")),
        }
    }
    anyhow::bail!(
        "no supported clipboard command succeeded: {}",
        errors.join("; ")
    )
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "linux")))]
fn write_clipboard_text_native(_text: &str) -> Result<()> {
    anyhow::bail!("native clipboard is unsupported on this platform")
}

fn write_to_clipboard_command(program: &str, args: &[&str], text: &str) -> Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to start clipboard command `{program}`"))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .context("failed to open clipboard command stdin")?;
        stdin
            .write_all(text.as_bytes())
            .with_context(|| format!("failed to write to clipboard command `{program}`"))?;
    }
    let status = child
        .wait()
        .with_context(|| format!("failed to wait for clipboard command `{program}`"))?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "clipboard command `{program}` exited with {}",
            exit_status_label(status)
        )
    }
}

fn select_spawn_spec() -> Result<SpawnSpec> {
    if let Ok(path) = env::var("LOCALAIROUTER_DAEMON_PATH") {
        return Ok(binary_spawn_spec(PathBuf::from(path)));
    }
    if let Ok(path) = env::var("LOCALOPENROUTER_DAEMON_PATH") {
        return Ok(binary_spawn_spec(PathBuf::from(path)));
    }
    if let Ok(path) = env::var("LOCALROUTER_DAEMON_PATH") {
        return Ok(binary_spawn_spec(PathBuf::from(path)));
    }

    if let Some(path) = sibling_daemon_binary().filter(|path| daemon_binary_is_fresh(path)) {
        return Ok(binary_spawn_spec(path));
    }

    if let Some(path) = workspace_daemon_binary().filter(|path| daemon_binary_is_fresh(path)) {
        return Ok(binary_spawn_spec(path));
    }

    if cfg!(debug_assertions) {
        if let Some(manifest) = workspace_manifest() {
            let binary = build_workspace_daemon_binary(&manifest)?;
            return Ok(binary_spawn_spec(binary));
        }
    }

    if let Some(path) = sibling_daemon_binary() {
        return Ok(binary_spawn_spec(path));
    }

    if let Some(path) = workspace_daemon_binary() {
        return Ok(binary_spawn_spec(path));
    }

    let manifest = workspace_manifest().context("failed to resolve workspace Cargo.toml")?;
    Ok(cargo_spawn_spec(manifest))
}

fn ensure_tray(app: &AppHandle) -> Result<()> {
    let locale_state = app.state::<LocaleState>();
    refresh_tray_menu_for_app(app, &locale_state)
}

fn refresh_tray_menu_for_app(app: &AppHandle, locale_state: &LocaleState) -> Result<()> {
    let locale = locale_state.get();
    let providers = tauri::async_runtime::block_on(load_tray_provider_state_async())?;
    apply_tray_menu(app, &locale, providers)
}

async fn refresh_tray_menu_for_app_async(app: &AppHandle, locale: String) -> Result<()> {
    let providers = load_tray_provider_state_async().await?;
    let app_handle = app.clone();
    app.run_on_main_thread(move || {
        if let Err(error) = apply_tray_menu(&app_handle, &locale, providers) {
            warn!("failed to apply tray menu on main thread: {error:#}");
        }
    })
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(())
}

fn apply_tray_menu(app: &AppHandle, locale: &str, providers: Vec<TrayProviderState>) -> Result<()> {
    let menu = build_tray_menu(app, locale, &providers)?;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu))
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let _ = tray.set_tooltip(Some(tray_tooltip(&locale)));
        return Ok(());
    }

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(tray_tooltip(&locale))
        .show_menu_on_left_click(true);
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder
        .build(app)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(())
}

fn build_tray_menu(
    app: &AppHandle,
    locale: &str,
    providers: &[TrayProviderState],
) -> Result<tauri::menu::Menu<tauri::Wry>> {
    let open_dashboard = MenuItemBuilder::with_id(
        TRAY_MENU_OPEN_DASHBOARD,
        tray_text(locale, "Open LocalAIRouter"),
    )
    .build(app)
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let quit = MenuItemBuilder::with_id(TRAY_MENU_QUIT, tray_text(locale, "Quit"))
        .build(app)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    let mut builder = MenuBuilder::new(app).item(&open_dashboard).separator();

    for (index, provider) in providers.iter().enumerate() {
        let heading = MenuItemBuilder::with_id(
            tray_provider_heading_menu_id(&provider.slug),
            provider.display_name.clone(),
        )
        .enabled(false)
        .build(app)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        builder = builder.item(&heading);

        if !provider.enabled {
            let item = MenuItemBuilder::with_id(
                format!("tray.provider.{}.disabled", provider.slug),
                tray_text(locale, "Disabled"),
            )
            .enabled(false)
            .build(app)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            builder = builder.item(&item);
        } else if provider.accounts.is_empty() {
            let item = MenuItemBuilder::with_id(
                format!("tray.provider.{}.empty", provider.slug),
                tray_text(locale, "No Accounts"),
            )
            .enabled(false)
            .build(app)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            builder = builder.item(&item);
        } else {
            for account in &provider.accounts {
                let item = MenuItemBuilder::with_id(
                    tray_account_menu_id(&provider.slug, &account.id),
                    tray_account_label(
                        provider.default_account_id.as_deref() == Some(account.id.as_str()),
                        &account.name,
                    ),
                )
                .build(app)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
                builder = builder.item(&item);
            }
        }

        if index + 1 < providers.len() {
            builder = builder.separator();
        }
    }

    if !providers.is_empty() {
        builder = builder.separator();
    }
    builder
        .item(&quit)
        .build()
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

fn handle_tray_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref();
    tracing::info!("tray/menu event received: {id}");
    if id == TRAY_MENU_OPEN_DASHBOARD {
        if let Err(error) = open_dashboard_window(app) {
            warn!("failed to open dashboard from tray: {error:#}");
        }
        return;
    }
    if id.starts_with(TRAY_MENU_PROVIDER_HEADING_PREFIX) {
        return;
    }
    if id == TRAY_MENU_QUIT {
        app.exit(0);
        return;
    }
    if let Some((provider_slug, account_id)) = parse_tray_account_menu_id(id) {
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = set_default_account_from_tray(&provider_slug, &account_id).await {
                warn!("failed to switch tray default account: {error:#}");
                return;
            }
            let locale = app_handle
                .try_state::<LocaleState>()
                .map(|locale_state| locale_state.get())
                .unwrap_or_else(detect_locale);
            if let Err(error) = refresh_tray_menu_for_app_async(&app_handle, locale).await {
                warn!("failed to refresh tray after account switch: {error:#}");
            }
            notify_frontend_refresh(&app_handle);
        });
    }
}

fn open_dashboard_window(app: &AppHandle) -> Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    show_app_from_tray(app);
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    let _ = window.eval(
        r#"window.dispatchEvent(new CustomEvent("localairouter:navigate", { detail: "dashboard" }));"#,
    );
    Ok(())
}

#[cfg(target_os = "macos")]
fn hide_app_to_tray(app: &AppHandle) {
    let app_handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = app_handle.hide();
        let _ = app_handle.set_activation_policy(ActivationPolicy::Accessory);
    });
}

#[cfg(not(target_os = "macos"))]
fn hide_app_to_tray(_app: &AppHandle) {}

#[cfg(target_os = "macos")]
fn show_app_from_tray(app: &AppHandle) {
    let app_handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = app_handle.set_activation_policy(ActivationPolicy::Regular);
        let _ = app_handle.show();
        apply_macos_app_icon();
        activate_macos_application();
    });
}

#[cfg(not(target_os = "macos"))]
fn show_app_from_tray(_app: &AppHandle) {}

#[cfg(target_os = "macos")]
fn sync_macos_app_icon(app: &AppHandle) {
    let _ = app.run_on_main_thread(apply_macos_app_icon);
}

#[cfg(target_os = "macos")]
fn apply_macos_app_icon() {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let app = NSApplication::sharedApplication(mtm);
    let data = NSData::with_bytes(MACOS_APP_ICON_PNG);
    if let Some(app_icon) = NSImage::initWithData(NSImage::alloc(), &data) {
        unsafe { app.setApplicationIconImage(Some(&app_icon)) };
    }
}

#[cfg(target_os = "macos")]
fn activate_macos_application() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let app = NSApplication::sharedApplication(mtm);
    app.unhide(None);
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
}

fn notify_frontend_refresh(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.eval(r#"window.dispatchEvent(new CustomEvent("localairouter:refresh"));"#);
    }
}

async fn set_default_account_from_tray(provider_slug: &str, account_id: &str) -> Result<()> {
    let binding = RouteBindingInput {
        provider: provider_slug.into(),
        model_prefix: None,
        account_id: account_id.into(),
    };
    let url = format!("http://127.0.0.1:{}/admin/routes", configured_daemon_port());
    let response = reqwest::Client::new()
        .post(&url)
        .json(&binding)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "daemon rejected tray default switch with status {status}: {body}"
        ));
    }
    Ok(())
}

async fn load_tray_provider_state_async() -> Result<Vec<TrayProviderState>> {
    let repo = Repository::new_with_paths(AppPaths::discover()?, configured_daemon_port()).await?;
    let providers = repo.list_providers().await?;
    let accounts = repo.list_accounts().await?;
    let routes = repo.list_routes().await?;

    Ok(providers
        .into_iter()
        .map(|provider| {
            let default_account_id = routes
                .iter()
                .find(|route| route.provider == provider.slug && route.model_prefix.is_none())
                .map(|route| route.account_id.clone());
            let accounts = accounts
                .iter()
                .filter(|account| account.provider == provider.slug && account.enabled)
                .map(|account| TrayAccountState {
                    id: account.id.clone(),
                    name: account.name.clone(),
                })
                .collect();
            TrayProviderState {
                slug: provider.slug,
                display_name: provider.display_name,
                enabled: provider.enabled,
                default_account_id,
                accounts,
            }
        })
        .collect())
}

fn tray_account_menu_id(provider_slug: &str, account_id: &str) -> String {
    format!("{TRAY_MENU_SET_DEFAULT_PREFIX}::{provider_slug}::{account_id}")
}

fn tray_provider_heading_menu_id(provider_slug: &str) -> String {
    format!("{TRAY_MENU_PROVIDER_HEADING_PREFIX}::{provider_slug}")
}

fn tray_account_label(selected: bool, account_name: &str) -> String {
    if selected {
        format!("✓ {account_name}")
    } else {
        format!("  {account_name}")
    }
}

fn parse_tray_account_menu_id(value: &str) -> Option<(String, String)> {
    let prefix = format!("{TRAY_MENU_SET_DEFAULT_PREFIX}::");
    let trimmed = value.strip_prefix(&prefix)?;
    let mut parts = trimmed.splitn(2, "::");
    let provider_slug = parts.next()?.trim();
    let account_id = parts.next()?.trim();
    if provider_slug.is_empty() || account_id.is_empty() {
        return None;
    }
    Some((provider_slug.into(), account_id.into()))
}

fn detect_locale() -> String {
    env::var("LOCALAIROUTER_LOCALE")
        .or_else(|_| env::var("LOCALOPENROUTER_LOCALE"))
        .or_else(|_| env::var("LANG"))
        .map(|value| normalize_locale(&value))
        .unwrap_or_else(|_| "en".into())
}

fn normalize_locale(value: &str) -> String {
    if value.to_ascii_lowercase().starts_with("zh") {
        "zh-CN".into()
    } else {
        "en".into()
    }
}

fn tray_tooltip(locale: &str) -> String {
    if locale == "zh-CN" {
        "LocalAIRouter 菜单".into()
    } else {
        "LocalAIRouter Menu".into()
    }
}

fn tray_text(locale: &str, value: &str) -> String {
    if locale != "zh-CN" {
        return value.into();
    }
    match value {
        "Open LocalAIRouter" => "打开 LocalAIRouter".into(),
        "Quit" => "退出".into(),
        "No Accounts" => "无帐号".into(),
        "Disabled" => "已禁用".into(),
        _ => value.into(),
    }
}

fn binary_spawn_spec(path: PathBuf) -> SpawnSpec {
    SpawnSpec {
        command_path: path.to_string_lossy().into_owned(),
        program: path,
        args: Vec::new(),
        launch_mode: "binary",
    }
}

fn cargo_spawn_spec(manifest: PathBuf) -> SpawnSpec {
    let manifest_path = manifest.to_string_lossy().into_owned();
    SpawnSpec {
        program: PathBuf::from("cargo"),
        args: vec![
            "run".into(),
            "--manifest-path".into(),
            manifest_path.clone(),
            "-p".into(),
            DAEMON_BINARY_NAME.into(),
        ],
        launch_mode: "cargo-run",
        command_path: format!("cargo run --manifest-path {manifest_path} -p {DAEMON_BINARY_NAME}"),
    }
}

fn build_workspace_daemon_binary(manifest: &Path) -> Result<PathBuf> {
    let mut command = Command::new("cargo");
    command
        .arg("build")
        .arg("--manifest-path")
        .arg(manifest)
        .arg("-p")
        .arg(DAEMON_BINARY_NAME)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = command
        .status()
        .with_context(|| format!("failed to run cargo build via {}", manifest.display()))?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "cargo build failed for {DAEMON_BINARY_NAME} via {}",
            manifest.display()
        ));
    }
    workspace_daemon_binary().context("built daemon binary not found in target directory")
}

fn configured_port(names: &[&str], default_port: u16) -> u16 {
    parse_port_env_list(names).unwrap_or(default_port)
}

fn parse_port_env(name: &str) -> Option<u16> {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|port| *port != 0)
}

fn parse_port_env_list(names: &[&str]) -> Option<u16> {
    names.iter().find_map(|name| parse_port_env(name))
}

fn parse_bool_env(name: &str) -> Option<bool> {
    env::var(name).ok().map(|value| {
        matches!(
            value.trim(),
            "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
        )
    })
}

fn parse_bool_env_list(names: &[&str]) -> Option<bool> {
    names.iter().find_map(|name| parse_bool_env(name))
}

fn daemon_binary_is_fresh(binary: &Path) -> bool {
    let Some(binary_modified) = modified_time(binary) else {
        return false;
    };
    let Some(root) = workspace_root() else {
        return true;
    };
    let watched_paths = [
        root.join("Cargo.toml"),
        root.join("crates")
            .join("localairouter-core")
            .join("Cargo.toml"),
        root.join("crates")
            .join("localairouter-daemon")
            .join("Cargo.toml"),
        root.join("crates").join("localairouter-core").join("src"),
        root.join("crates").join("localairouter-daemon").join("src"),
    ];
    watched_paths
        .into_iter()
        .filter_map(|path| latest_mtime(&path))
        .max()
        .map(|latest_source| latest_source <= binary_modified)
        .unwrap_or(true)
}

fn latest_mtime(path: &Path) -> Option<SystemTime> {
    if path.is_file() {
        return modified_time(path);
    }
    if !path.is_dir() {
        return None;
    }

    let mut latest = modified_time(path);
    let entries = fs::read_dir(path).ok()?;
    for entry in entries.flatten() {
        let entry_latest = latest_mtime(&entry.path());
        latest = match (latest, entry_latest) {
            (Some(current), Some(candidate)) => Some(current.max(candidate)),
            (None, Some(candidate)) => Some(candidate),
            (current, None) => current,
        };
    }
    latest
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn start_ui_dev_server() -> Result<Option<UiDevServer>> {
    if !cfg!(debug_assertions) || !env_flag_enabled(&[UI_DEV_ENV, LEGACY_UI_DEV_ENV]) {
        return Ok(None);
    }

    let Some(root) = workspace_root().map(|root| root.join("apps").join("desktop").join("ui"))
    else {
        warn!("ui dev mode requested, but the workspace root could not be resolved");
        return Ok(None);
    };
    if !root.join("index.html").exists() {
        warn!(
            "ui dev mode requested, but the UI source directory is missing at {}",
            root.display()
        );
        return Ok(None);
    }

    let port = parse_port_env_list(&[UI_DEV_PORT_ENV, LEGACY_UI_DEV_PORT_ENV])
        .unwrap_or(DEFAULT_UI_DEV_PORT);
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => listener,
        Err(error) => {
            warn!("ui dev server failed to bind on 127.0.0.1:{port}: {error}");
            return Ok(None);
        }
    };
    let url = Url::parse(&format!("http://127.0.0.1:{port}/"))
        .context("failed to construct ui dev server url")?;
    thread::spawn(move || run_ui_dev_server(listener, root));
    Ok(Some(UiDevServer { url }))
}

fn env_flag_enabled(names: &[&str]) -> bool {
    matches!(
        names
            .iter()
            .find_map(|name| env::var(name).ok())
            .as_deref()
            .map(str::trim),
        Some("1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
    )
}

fn run_ui_dev_server(listener: TcpListener, root: PathBuf) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_ui_dev_request(stream, &root) {
                    warn!("ui dev server request failed: {error:#}");
                }
            }
            Err(error) => warn!("ui dev server accept failed: {error}"),
        }
    }
}

fn handle_ui_dev_request(mut stream: TcpStream, root: &Path) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    let mut buffer = [0_u8; 8192];
    let read = stream.read(&mut buffer)?;
    if read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..read]);
    let request_line = request.lines().next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or("/");
    if method != "GET" && method != "HEAD" {
        write_http_response(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain; charset=utf-8",
            b"Method Not Allowed",
            method == "HEAD",
        )?;
        return Ok(());
    }

    let path = target.split('?').next().unwrap_or("/");
    if path == "/__dev__/version" {
        let version = ui_dev_version(root);
        write_http_response(
            &mut stream,
            "200 OK",
            "text/plain; charset=utf-8",
            version.as_bytes(),
            method == "HEAD",
        )?;
        return Ok(());
    }

    let Some(asset_path) = resolve_ui_dev_asset(root, path) else {
        write_http_response(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"Not Found",
            method == "HEAD",
        )?;
        return Ok(());
    };
    let body = fs::read(&asset_path)?;
    write_http_response(
        &mut stream,
        "200 OK",
        content_type_for_path(&asset_path),
        &body,
        method == "HEAD",
    )?;
    Ok(())
}

fn resolve_ui_dev_asset(root: &Path, request_path: &str) -> Option<PathBuf> {
    let trimmed = request_path.trim_start_matches('/');
    let relative = if trimmed.is_empty() {
        PathBuf::from("index.html")
    } else {
        PathBuf::from(trimmed)
    };
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return None;
    }

    let candidate = root.join(relative);
    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

fn ui_dev_version(root: &Path) -> String {
    latest_mtime(root)
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|| "0".into())
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store, no-cache, must-revalidate\r\nPragma: no-cache\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()?;
    Ok(())
}

fn resolve_storage_path(relative_path: String) -> std::result::Result<PathBuf, String> {
    let relative = PathBuf::from(relative_path);
    if relative.is_absolute() {
        return Ok(relative);
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("path traversal is not allowed".into());
    }
    let paths = AppPaths::discover().map_err(|error| error.to_string())?;
    let as_root = paths.root.join(&relative);
    if as_root.exists()
        || relative
            .components()
            .next()
            .map(|component| component.as_os_str() == "logs")
            .unwrap_or(false)
    {
        Ok(as_root)
    } else {
        Ok(paths.logs.join(relative))
    }
}

fn reveal_path(path: PathBuf) -> std::result::Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(&path);
        command
    };

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(&path);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(&path);
        command
    };

    let status = command
        .status()
        .map_err(|error| format!("failed to open {}: {error}", path.as_path().display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to open {}: exited with status {status}",
            path.as_path().display()
        ))
    }
}

fn pick_logs_directory_native(
    initial_path: Option<String>,
) -> std::result::Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let initial_path = initial_path
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .filter(|path| path.exists());
        let mut script = String::from(
            "set chosenFolder to choose folder with prompt \"Select traffic logs directory\"",
        );
        if let Some(path) = initial_path {
            script.push_str(" default location POSIX file \"");
            script.push_str(&escape_applescript_string(&path.to_string_lossy()));
            script.push('"');
        }
        script.push_str("\nPOSIX path of chosenFolder");

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|error| format!("failed to open folder picker: {error}"))?;
        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if selected.is_empty() {
                return Ok(None);
            }
            return Ok(Some(PathBuf::from(selected).to_string_lossy().into_owned()));
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("-128") {
            return Ok(None);
        }
        return Err(stderr.trim().to_owned());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = initial_path;
        Ok(None)
    }
}

#[cfg(target_os = "macos")]
fn escape_applescript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn daemon_log_file() -> Result<PathBuf> {
    let root = AppPaths::discover()?.logs;
    fs::create_dir_all(&root)?;
    Ok(root.join("daemon.log"))
}

fn append_supervisor_event(log_file: &Path, message: String) -> Result<()> {
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    writeln!(file, "[{}] {message}", timestamp())?;
    Ok(())
}

fn read_last_log_line() -> Result<Option<String>> {
    let log_file = daemon_log_file()?;
    if !log_file.exists() {
        return Ok(None);
    }
    let file = fs::File::open(log_file)?;
    let reader = BufReader::new(file);
    let mut last = None;
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            last = Some(trimmed.to_owned());
        }
    }
    Ok(last)
}

fn sibling_daemon_binary() -> Option<PathBuf> {
    let executable = env::current_exe().ok()?;
    let parent = executable.parent()?;
    let candidates = [
        parent.join(daemon_binary_name()),
        parent.join(format!("{}.exe", daemon_binary_name())),
    ];
    candidates.into_iter().find(|candidate| candidate.exists())
}

fn workspace_daemon_binary() -> Option<PathBuf> {
    let root = workspace_root()?;
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let candidates = [
        root.join("target").join(profile).join(daemon_binary_name()),
        root.join("target")
            .join(profile)
            .join(format!("{}.exe", daemon_binary_name())),
    ];
    candidates.into_iter().find(|candidate| candidate.exists())
}

fn workspace_manifest() -> Option<PathBuf> {
    let manifest = workspace_root()?.join("Cargo.toml");
    manifest.exists().then_some(manifest)
}

fn workspace_root() -> Option<PathBuf> {
    let mut starts = Vec::new();
    if let Ok(current_dir) = env::current_dir() {
        starts.push(current_dir);
    }
    if let Ok(executable) = env::current_exe() {
        if let Some(parent) = executable.parent() {
            starts.push(parent.to_path_buf());
        }
    }
    starts.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    starts
        .into_iter()
        .find_map(|start| find_workspace_root_from(&start))
}

fn find_workspace_root_from(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|ancestor| {
        let manifest = ancestor.join("Cargo.toml");
        if manifest.exists() && is_workspace_manifest(&manifest) {
            Some(ancestor.to_path_buf())
        } else {
            None
        }
    })
}

fn is_workspace_manifest(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|contents| contents.contains("[workspace]"))
        .unwrap_or(false)
}

fn daemon_binary_name() -> &'static str {
    DAEMON_BINARY_NAME
}

fn exit_status_label(status: ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        if let Some(code) = status.code() {
            return format!("exit code {code}");
        }
        if let Some(signal) = status.signal() {
            return format!("signal {signal}");
        }
    }

    status
        .code()
        .map(|code| format!("exit code {code}"))
        .unwrap_or_else(|| status.to_string())
}

fn timestamp() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::{
        import_claude_account_contents, import_codex_account_contents, sync_claude_config_contents,
        sync_codex_config_contents,
    };

    #[test]
    fn codex_sync_updates_existing_default_provider_base_url_in_place() {
        let input = r#"
model_provider = "packycode"
model = "gpt-5.4"

[model_providers.packycode]
name = "packycode"
base_url = "https://example.com/v1"
wire_api = "responses"
requires_openai_auth = true
extra_flag = true

[features]
multi_agent = true
"#;

        let (updated, profile_key, default_provider_updated, previous_model_provider) =
            sync_codex_config_contents(input, "codex", "Codex", "http://127.0.0.1:16321/codex")
                .expect("sync should succeed");

        assert_eq!(profile_key, "packycode");
        assert!(!default_provider_updated);
        assert_eq!(previous_model_provider.as_deref(), Some("packycode"));
        assert!(updated.contains("model_provider = \"packycode\""));
        assert!(updated.contains("[model_providers.packycode]"));
        assert!(updated.contains("base_url = \"http://127.0.0.1:16321/codex\""));
        assert!(updated.contains("base_url1 = \"https://example.com/v1\""));
        assert!(updated.contains("extra_flag = true"));
        assert!(updated.contains("[features]"));
        assert!(updated.contains("multi_agent = true"));
        assert!(!updated.contains("[model_providers.localairouter_codex]"));
    }

    #[test]
    fn codex_sync_uses_next_backup_slot_when_base_url1_exists() {
        let input = r#"
model_provider = "packycode"

[model_providers.packycode]
base_url = "https://example.com/v2"
base_url1 = "https://example.com/v1"
"#;

        let (updated, profile_key, default_provider_updated, previous_model_provider) =
            sync_codex_config_contents(input, "codex", "Codex", "http://127.0.0.1:16321/codex")
                .expect("sync should succeed");

        assert_eq!(profile_key, "packycode");
        assert!(!default_provider_updated);
        assert_eq!(previous_model_provider.as_deref(), Some("packycode"));
        assert!(updated.contains("base_url = \"http://127.0.0.1:16321/codex\""));
        assert!(updated.contains("base_url1 = \"https://example.com/v2\""));
        assert!(updated.contains("base_url2 = \"https://example.com/v1\""));
    }

    #[test]
    fn codex_sync_deduplicates_backups_when_switching_between_two_local_urls() {
        let input = r#"
model_provider = "packycode"

[model_providers.packycode]
base_url = "http://127.0.0.1:7322/codex"
base_url1 = "http://127.0.0.1:16321/codex"
base_url2 = "http://127.0.0.1:7322/codex"
"#;

        let (updated, profile_key, default_provider_updated, previous_model_provider) =
            sync_codex_config_contents(input, "codex", "Codex", "http://127.0.0.1:16321/codex")
                .expect("sync should succeed");

        assert_eq!(profile_key, "packycode");
        assert!(!default_provider_updated);
        assert_eq!(previous_model_provider.as_deref(), Some("packycode"));
        assert!(updated.contains("base_url = \"http://127.0.0.1:16321/codex\""));
        assert!(updated.contains("base_url1 = \"http://127.0.0.1:7322/codex\""));
        assert!(!updated.contains("base_url2 ="));
        assert!(!updated.contains("base_url3 ="));
    }

    #[test]
    fn codex_sync_sets_default_provider_when_missing() {
        let (updated, profile_key, default_provider_updated, previous_model_provider) =
            sync_codex_config_contents("", "codex", "Codex", "http://127.0.0.1:16321/codex")
                .expect("sync should succeed");

        assert_eq!(profile_key, "localairouter_codex");
        assert!(default_provider_updated);
        assert_eq!(previous_model_provider, None);
        assert!(updated.contains("model_provider = \"localairouter_codex\""));
        assert!(updated.contains("[model_providers.localairouter_codex]"));
        assert!(updated.contains("base_url = \"http://127.0.0.1:16321/codex\""));
        assert!(updated.contains("wire_api = \"responses\""));
        assert!(updated.contains("requires_openai_auth = true"));
    }

    #[test]
    fn claude_sync_initializes_settings_json_with_env() {
        let updated = sync_claude_config_contents("", "http://127.0.0.1:16321/claude-code")
            .expect("sync should succeed");

        assert!(updated.contains("\"env\""));
        assert!(updated.contains("\"ANTHROPIC_BASE_URL\": \"http://127.0.0.1:16321/claude-code\""));
        assert!(updated.contains("\"ANTHROPIC_API_KEY\": \"localairouter-managed\""));
    }

    #[test]
    fn claude_sync_preserves_existing_settings_and_merges_env() {
        let input = r#"{
  "permissions": {
    "allow": ["Bash(git status)"]
  },
  "env": {
    "CLAUDE_CODE_ENABLE_TELEMETRY": "1",
    "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
    "ANTHROPIC_API_KEY": "sk-ant-old"
  }
}"#;

        let updated = sync_claude_config_contents(input, "http://127.0.0.1:16321/claude-code")
            .expect("sync should succeed");

        assert!(updated.contains("\"permissions\""));
        assert!(updated.contains("\"CLAUDE_CODE_ENABLE_TELEMETRY\": \"1\""));
        assert!(updated.contains("\"ANTHROPIC_BASE_URL\": \"http://127.0.0.1:16321/claude-code\""));
        assert!(updated.contains("\"ANTHROPIC_BASE_URL1\": \"https://api.anthropic.com\""));
        assert!(updated.contains("\"ANTHROPIC_API_KEY\": \"localairouter-managed\""));
        assert!(updated.contains("\"ANTHROPIC_API_KEY1\": \"sk-ant-old\""));
    }

    #[test]
    fn claude_sync_deduplicates_env_backups_when_switching_between_two_urls() {
        let input = r#"{
  "env": {
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:7322/claude-code",
    "ANTHROPIC_BASE_URL1": "http://127.0.0.1:16321/claude-code",
    "ANTHROPIC_BASE_URL2": "http://127.0.0.1:7322/claude-code"
  }
}"#;

        let updated = sync_claude_config_contents(input, "http://127.0.0.1:16321/claude-code")
            .expect("sync should succeed");

        assert!(updated.contains("\"ANTHROPIC_BASE_URL\": \"http://127.0.0.1:16321/claude-code\""));
        assert!(updated.contains("\"ANTHROPIC_BASE_URL1\": \"http://127.0.0.1:7322/claude-code\""));
        assert!(!updated.contains("\"ANTHROPIC_BASE_URL2\""));
    }

    #[test]
    fn claude_import_prefers_real_auth_token_and_remote_base_url() {
        let input = r#"{
  "env": {
    "ANTHROPIC_API_KEY": "localairouter-managed",
    "ANTHROPIC_AUTH_TOKEN": "sk-ant-real",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:16321/claude-code",
    "ANTHROPIC_BASE_URL1": "https://www.packyapi.com"
  }
}"#;

        let account = import_claude_account_contents(input).expect("import should succeed");

        assert_eq!(account.provider, "claude-code");
        assert_eq!(account.name, "Claude Code");
        assert_eq!(
            account.base_url.as_deref(),
            Some("https://www.packyapi.com")
        );
        assert_eq!(account.api_key.as_deref(), Some("sk-ant-real"));
    }

    #[test]
    fn codex_import_prefers_selected_provider_remote_base_url_and_auth_json_key() {
        let config = r#"
model_provider = "packycode"

[model_providers.packycode]
name = "packycode"
base_url = "http://127.0.0.1:16321/codex"
base_url1 = "http://127.0.0.1:7332/codex"
base_url2 = "https://codex-api.packycode.com/v1"
wire_api = "responses"
requires_openai_auth = true
"#;
        let auth = r#"{
  "OPENAI_API_KEY": "sk-openai-primary"
}"#;

        let account =
            import_codex_account_contents(config, Some(auth)).expect("import should succeed");

        assert_eq!(account.provider, "codex");
        assert_eq!(account.name, "packycode");
        assert_eq!(
            account.base_url.as_deref(),
            Some("https://codex-api.packycode.com/v1")
        );
        assert_eq!(account.api_key.as_deref(), Some("sk-openai-primary"));
    }
}
