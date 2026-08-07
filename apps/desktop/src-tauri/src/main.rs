#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
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
    AccountConverter, AccountInput, AppPaths, AppSettings, AppSettingsInput, DAEMON_API_VERSION,
    Repository, RouteBindingInput, load_app_settings, save_app_settings,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use tauri::menu::{MenuBuilder, MenuEvent, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{ActivationPolicy, AppHandle, Manager, RunEvent, Url, WindowEvent};
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
#[cfg(target_os = "macos")]
const MACOS_TRAY_ICON_PNG: &[u8] = include_bytes!("../icons/tray-template.png");
const LOCAL_ROUTER_MANAGED_SECRET: &str = "localairouter-managed";

#[derive(Default)]
struct DaemonRuntime {
    child: Option<Child>,
    external_running: bool,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DaemonHealthProbe {
    #[serde(default)]
    api_version: u32,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillInfo {
    id: String,
    name: String,
    description: String,
    source: String,
    source_label: String,
    root_path: String,
    skill_path: String,
    skill_file_path: String,
    updated_at: Option<String>,
    codex_linked: bool,
    agents_linked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillImportPreview {
    name: String,
    description: String,
    source_path: String,
    skill_file_path: String,
    directory_name: String,
    relative_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillImportRequest {
    source_path: String,
    target_source: String,
    conflict_strategy: String,
    cleanup_root: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillAgentLinkRequest {
    skill_id: String,
    agent: String,
    enabled: bool,
    #[serde(default)]
    replace_existing: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillGitScanRequest {
    git_url: String,
    git_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillGitScanResult {
    checkout_path: String,
    candidates: Vec<SkillImportPreview>,
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

        let port = configured_daemon_port();
        let log_file = daemon_log_file()?;
        if let Some(health) = daemon_health_probe(port) {
            if daemon_health_is_compatible(&health) {
                runtime.external_running = true;
                runtime.pid = None;
                runtime.started_at = None;
                runtime.launch_mode = Some("external".into());
                runtime.command_path = None;
                runtime.last_error = None;
                runtime.last_exit = Some(format!("attached to existing daemon on port {port}"));
                append_supervisor_event(
                    &log_file,
                    format!("desktop: attached to existing daemon on port {port}"),
                )?;
                return self.snapshot_locked(&runtime);
            }

            append_supervisor_event(
                &log_file,
                format!(
                    "desktop: incompatible daemon on port {port} (api version {}), trying to replace it",
                    health.api_version
                ),
            )?;
            if !terminate_external_daemon_on_port(port, &log_file)? {
                anyhow::bail!(
                    "incompatible daemon is already listening on port {port}; quit the old LocalAIRouter process or restart the daemon"
                );
            }
            wait_for_daemon_shutdown(port, Duration::from_secs(5), Duration::from_millis(100));
        }

        runtime.external_running = false;
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
        wait_for_daemon_health(port, Duration::from_secs(12), Duration::from_millis(250));
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
        } else if runtime.external_running || daemon_health_available(configured_daemon_port()) {
            let port = configured_daemon_port();
            if terminate_external_daemon_on_port(port, &log_file)? {
                runtime.last_exit = Some(format!("stopped external daemon on port {port}"));
                runtime.last_error = None;
            } else {
                runtime.last_exit = Some("external daemon is not managed by this app".into());
            }
        } else {
            runtime.last_exit = Some("already stopped".into());
        }
        runtime.external_running = false;
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
            if runtime.external_running && !daemon_health_available(configured_daemon_port()) {
                runtime.external_running = false;
                runtime.last_exit = Some("external daemon disappeared".into());
            }
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
                runtime.external_running = false;
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
        let port = configured_daemon_port();
        Ok(DaemonStatus {
            running: runtime.child.is_some()
                || runtime.external_running
                || daemon_health_available(port),
            port,
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

    if let Err(error) = persist_startup_env_settings_overrides() {
        warn!("failed to persist startup env settings overrides: {error:#}");
    }

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
            set_app_locale,
            list_skills,
            open_skill_path,
            open_skills_root,
            pick_skill_import_directory,
            import_skill,
            scan_git_skills,
            cleanup_git_skill_checkout,
            set_skill_agent_link
        ])
        .setup(move |app| {
            if let Some(ui_dev_server) = &ui_dev_server {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(error) = window.navigate(ui_dev_server.url.clone()) {
                        warn!("failed to navigate main window to ui dev server: {error}");
                    }
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .inspect_err(|error| error!("tauri build failure: {error}"))
        .map(|app| {
            app.run(|app_handle, event| {
                if matches!(event, RunEvent::Ready) {
                    #[cfg(target_os = "macos")]
                    sync_macos_app_icon(app_handle);
                    if let Err(error) = ensure_tray(app_handle) {
                        warn!("failed to initialize tray on ready event: {error:#}");
                    }
                }
            });
        })
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
fn save_app_settings_command(
    supervisor: tauri::State<'_, DaemonSupervisor>,
    input: AppSettingsInput,
) -> std::result::Result<AppSettings, String> {
    let paths = AppPaths::discover().map_err(|error| error.to_string())?;
    let previous = load_app_settings(&paths).map_err(|error| error.to_string())?;
    let was_running = supervisor
        .status()
        .map(|status| status.running)
        .unwrap_or(false);
    let saved = save_app_settings(&paths, &input).map_err(|error| error.to_string())?;
    let restart_required = previous.daemon_port != saved.daemon_port
        || previous.allow_lan_access != saved.allow_lan_access;
    if was_running && restart_required {
        supervisor.restart().map_err(|error| error.to_string())?;
    }
    Ok(saved)
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

#[tauri::command]
fn list_skills() -> std::result::Result<Vec<SkillInfo>, String> {
    discover_skills().map_err(|error| error.to_string())
}

#[tauri::command]
fn open_skill_path(skill_id: String, target: Option<String>) -> std::result::Result<(), String> {
    let skills = discover_skills().map_err(|error| error.to_string())?;
    let skill = skills
        .into_iter()
        .find(|skill| skill.id == skill_id)
        .ok_or_else(|| format!("skill `{skill_id}` not found"))?;
    let path = match target.as_deref() {
        Some("file") => PathBuf::from(skill.skill_file_path),
        _ => PathBuf::from(skill.skill_path),
    };
    reveal_path(path)
}

#[tauri::command]
fn open_skills_root(source: Option<String>) -> std::result::Result<(), String> {
    let roots = skill_roots();
    let selected = match source.as_deref() {
        Some(source) if !source.trim().is_empty() => roots
            .into_iter()
            .find(|root| root.source == source)
            .ok_or_else(|| format!("skills root `{source}` not found"))?,
        _ => roots
            .iter()
            .find(|root| root.source == "local-store")
            .cloned()
            .ok_or_else(|| "no skills root found".to_owned())?,
    };
    if !selected.path.exists() {
        if selected.readonly {
            return Err(format!(
                "skills root not found: {}",
                selected.path.display()
            ));
        }
        fs::create_dir_all(&selected.path)
            .with_context(|| format!("failed to create skills root {}", selected.path.display()))
            .map_err(|error| error.to_string())?;
    }
    reveal_path(selected.path)
}

#[tauri::command]
async fn pick_skill_import_directory() -> std::result::Result<Option<SkillImportPreview>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = pick_folder_native("Select a skill directory", None)? else {
            return Ok(None);
        };
        preview_skill_import_path(&PathBuf::from(path))
            .map(Some)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn import_skill(request: SkillImportRequest) -> std::result::Result<SkillInfo, String> {
    import_skill_from_request(request).map_err(|error| error.to_string())
}

#[tauri::command]
async fn scan_git_skills(
    request: SkillGitScanRequest,
) -> std::result::Result<SkillGitScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        scan_git_skills_from_request(request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn cleanup_git_skill_checkout(checkout_path: String) -> std::result::Result<(), String> {
    remove_git_skill_checkout_root(&checkout_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_skill_agent_link(request: SkillAgentLinkRequest) -> std::result::Result<SkillInfo, String> {
    set_skill_agent_link_from_request(request).map_err(|error| error.to_string())
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
    AppPaths::discover()
        .ok()
        .and_then(|paths| load_app_settings(&paths).ok())
        .map(|settings| settings.daemon_port)
        .or_else(|| {
            parse_port_env_list(&[
                DAEMON_PORT_ENV,
                LEGACY_DAEMON_PORT_ENV,
                OLDER_DAEMON_PORT_ENV,
            ])
        })
        .unwrap_or(DEFAULT_DAEMON_PORT)
}

fn daemon_health_available(port: u16) -> bool {
    daemon_health_probe(port).is_some()
}

fn daemon_health_is_compatible(health: &DaemonHealthProbe) -> bool {
    health.api_version >= DAEMON_API_VERSION
}

fn daemon_health_probe(port: u16) -> Option<DaemonHealthProbe> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(250)) else {
        return None;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(750)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
    if stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return None;
    }

    let mut response = String::new();
    if stream.read_to_string(&mut response).is_err() {
        return None;
    }
    let is_ok = response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200");
    if !is_ok {
        return None;
    }
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .or_else(|| response.split_once("\n\n").map(|(_, body)| body))?;
    serde_json::from_str(body.trim()).ok()
}

fn wait_for_daemon_health(port: u16, timeout: Duration, interval: Duration) -> bool {
    let started = std::time::Instant::now();
    while started.elapsed() < timeout {
        if daemon_health_available(port) {
            return true;
        }
        thread::sleep(interval);
    }
    daemon_health_available(port)
}

fn wait_for_daemon_shutdown(port: u16, timeout: Duration, interval: Duration) -> bool {
    let started = std::time::Instant::now();
    while started.elapsed() < timeout {
        if !daemon_health_available(port) {
            return true;
        }
        thread::sleep(interval);
    }
    !daemon_health_available(port)
}

fn configured_allow_lan_access() -> bool {
    AppPaths::discover()
        .ok()
        .and_then(|paths| load_app_settings(&paths).ok())
        .map(|settings| settings.allow_lan_access)
        .or_else(|| {
            parse_bool_env_list(&[
                DAEMON_ALLOW_LAN_ENV,
                LEGACY_DAEMON_ALLOW_LAN_ENV,
                OLDER_DAEMON_ALLOW_LAN_ENV,
            ])
        })
        .unwrap_or(false)
}

fn persist_startup_env_settings_overrides() -> Result<()> {
    let env_port = parse_port_env_list(&[
        DAEMON_PORT_ENV,
        LEGACY_DAEMON_PORT_ENV,
        OLDER_DAEMON_PORT_ENV,
    ]);
    let env_allow_lan = parse_bool_env_list(&[
        DAEMON_ALLOW_LAN_ENV,
        LEGACY_DAEMON_ALLOW_LAN_ENV,
        OLDER_DAEMON_ALLOW_LAN_ENV,
    ]);
    if env_port.is_none() && env_allow_lan.is_none() {
        return Ok(());
    }

    let paths = AppPaths::discover()?;
    let current = load_app_settings(&paths)?;
    let input = AppSettingsInput {
        daemon_port: env_port.unwrap_or(current.daemon_port),
        allow_lan_access: env_allow_lan.unwrap_or(current.allow_lan_access),
        http_proxy_url: current.http_proxy_url,
        monitor_buffer_limit: current.monitor_buffer_limit,
        log_retention_days: current.log_retention_days,
        logs_dir: Some(current.logs_dir),
    };
    save_app_settings(&paths, &input)?;
    Ok(())
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

#[derive(Debug, Clone)]
struct SkillRoot {
    source: String,
    source_label: String,
    path: PathBuf,
    readonly: bool,
    link_target: bool,
}

fn skill_roots() -> Vec<SkillRoot> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    vec![
        SkillRoot {
            source: "local-store".into(),
            source_label: "Local Store".into(),
            path: localairouter_skills_root(),
            readonly: false,
            link_target: false,
        },
        SkillRoot {
            source: "codex-user".into(),
            source_label: "Codex Link".into(),
            path: home.join(".codex").join("skills"),
            readonly: false,
            link_target: true,
        },
        SkillRoot {
            source: "codex-system".into(),
            source_label: "Codex System".into(),
            path: home.join(".codex").join("skills").join(".system"),
            readonly: true,
            link_target: false,
        },
        SkillRoot {
            source: "agents-user".into(),
            source_label: "Agents Link".into(),
            path: home.join(".agents").join("skills"),
            readonly: false,
            link_target: true,
        },
    ]
}

fn localairouter_skills_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".localairouter")
        .join("skills")
}

fn discover_skills() -> Result<Vec<SkillInfo>> {
    let mut skills = Vec::new();
    for root in skill_roots() {
        if root.path.exists() {
            collect_skills_from_root(&root, &mut skills)?;
        }
    }
    skills.sort_by(|left, right| {
        left.source_label
            .cmp(&right.source_label)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    skills.dedup_by(|left, right| left.skill_file_path == right.skill_file_path);
    Ok(skills)
}

fn collect_skills_from_root(root: &SkillRoot, skills: &mut Vec<SkillInfo>) -> Result<()> {
    let entries = fs::read_dir(&root.path)
        .with_context(|| format!("failed to read skills root {}", root.path.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", root.path.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if root.link_target && is_managed_agent_skill_link(&path) {
            continue;
        }
        if root.source != "codex-system"
            && path.file_name().and_then(|name| name.to_str()) == Some(".system")
        {
            continue;
        }
        let skill_file = path.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }
        skills.push(build_skill_info(root, &path)?);
    }
    Ok(())
}

fn build_skill_info(root: &SkillRoot, path: &Path) -> Result<SkillInfo> {
    let skill_file = path.join("SKILL.md");
    let contents = fs::read_to_string(&skill_file)
        .with_context(|| format!("failed to read {}", skill_file.display()))?;
    let (name, description) = parse_skill_frontmatter(&contents);
    let fallback_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("skill")
        .to_owned();
    let name = name.unwrap_or_else(|| fallback_name.clone());
    let updated_at = fs::metadata(&skill_file)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(system_time_to_rfc3339);
    Ok(SkillInfo {
        id: format!("{}:{}", root.source, fallback_name),
        name,
        description: description.unwrap_or_default(),
        source: root.source.clone(),
        source_label: root.source_label.clone(),
        root_path: root.path.to_string_lossy().into_owned(),
        skill_path: path.to_string_lossy().into_owned(),
        skill_file_path: skill_file.to_string_lossy().into_owned(),
        updated_at,
        codex_linked: root.source == "local-store"
            && is_local_skill_linked_to_agent(path, "codex-user"),
        agents_linked: root.source == "local-store"
            && is_local_skill_linked_to_agent(path, "agents-user"),
    })
}

fn is_managed_agent_skill_link(path: &Path) -> bool {
    if !is_symlink(path) {
        return false;
    }
    path.canonicalize()
        .map(|target| target.starts_with(localairouter_skills_root()))
        .unwrap_or(false)
}

fn is_local_skill_linked_to_agent(store_path: &Path, agent_source: &str) -> bool {
    let Some(agent_root) = skill_roots()
        .into_iter()
        .find(|root| root.source == agent_source)
    else {
        return false;
    };
    find_agent_link_for_store_skill(store_path, &agent_root).is_some()
}

fn find_agent_link_for_store_skill(store_path: &Path, agent_root: &SkillRoot) -> Option<PathBuf> {
    let entries = fs::read_dir(&agent_root.path).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if is_symlink_to(&path, store_path) {
            return Some(path);
        }
    }
    None
}

fn preview_skill_import_path(path: &Path) -> Result<SkillImportPreview> {
    let source = path
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", path.display()))?;
    if !source.is_dir() {
        anyhow::bail!("selected path is not a directory: {}", source.display());
    }
    let skill_file = source.join("SKILL.md");
    if !skill_file.exists() {
        anyhow::bail!(
            "selected directory does not contain SKILL.md: {}",
            source.display()
        );
    }
    let contents = fs::read_to_string(&skill_file)
        .with_context(|| format!("failed to read {}", skill_file.display()))?;
    let (name, description) = parse_skill_frontmatter(&contents);
    let directory_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .map(sanitize_skill_directory_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| {
            sanitize_skill_directory_name(name.as_deref().unwrap_or("imported-skill"))
        });
    Ok(SkillImportPreview {
        name: name.unwrap_or_else(|| directory_name.clone()),
        description: description.unwrap_or_default(),
        source_path: source.to_string_lossy().into_owned(),
        skill_file_path: skill_file.to_string_lossy().into_owned(),
        directory_name,
        relative_path: None,
    })
}

fn import_skill_from_request(request: SkillImportRequest) -> Result<SkillInfo> {
    let preview = preview_skill_import_path(Path::new(&request.source_path))?;
    let store_root = skill_roots()
        .into_iter()
        .find(|root| root.source == "local-store")
        .context("local skill store root not found")?;
    fs::create_dir_all(&store_root.path).with_context(|| {
        format!(
            "failed to create local skills store {}",
            store_root.path.display()
        )
    })?;

    let source = PathBuf::from(&preview.source_path);
    let mut store_target = store_root.path.join(&preview.directory_name);
    let conflict_strategy = request.conflict_strategy.trim();
    if store_target.exists() {
        let same_directory = source.canonicalize().ok() == store_target.canonicalize().ok();
        match conflict_strategy {
            "overwrite" if same_directory => {}
            "overwrite" => {
                fs::remove_dir_all(&store_target)
                    .with_context(|| format!("failed to remove {}", store_target.display()))?;
                copy_dir_recursive(&source, &store_target).with_context(|| {
                    format!("failed to import skill into {}", store_target.display())
                })?;
            }
            "cancel" => {
                anyhow::bail!("skill already exists at {}", store_target.display());
            }
            _ => {
                store_target = unique_skill_target_path(&store_root.path, &preview.directory_name);
                copy_dir_recursive(&source, &store_target).with_context(|| {
                    format!("failed to import skill into {}", store_target.display())
                })?;
            }
        }
    } else {
        copy_dir_recursive(&source, &store_target)
            .with_context(|| format!("failed to import skill into {}", store_target.display()))?;
    }

    let target_root = skill_roots()
        .into_iter()
        .find(|root| root.source == request.target_source)
        .with_context(|| format!("skills root `{}` not found", request.target_source))?;
    if target_root.readonly {
        anyhow::bail!("system skills cannot be modified");
    }
    let info = if target_root.link_target {
        let link_path = link_skill_into_agent_root(&store_target, &target_root, conflict_strategy)?;
        build_skill_info(&target_root, &link_path)?
    } else {
        build_skill_info(&store_root, &store_target)?
    };
    cleanup_skill_import_root(request.cleanup_root.as_deref(), &source)?;
    Ok(info)
}

fn set_skill_agent_link_from_request(request: SkillAgentLinkRequest) -> Result<SkillInfo> {
    let store_root = skill_roots()
        .into_iter()
        .find(|root| root.source == "local-store")
        .context("local skill store root not found")?;
    let (_, directory_name) = request
        .skill_id
        .split_once(':')
        .context("invalid skill id")?;
    if request.skill_id.split_once(':').map(|(source, _)| source) != Some("local-store")
        || directory_name.contains('/')
        || directory_name.contains('\\')
        || directory_name == "."
        || directory_name == ".."
        || directory_name.contains("..")
    {
        anyhow::bail!("agent links can only be toggled for local store skills");
    }
    let store_path = store_root.path.join(directory_name);
    if !store_path.join("SKILL.md").exists() {
        anyhow::bail!("local skill `{}` was not found", request.skill_id);
    }

    let agent_source = match request.agent.as_str() {
        "codex" | "codex-user" => "codex-user",
        "agents" | "agents-user" => "agents-user",
        other => anyhow::bail!("unsupported skill agent `{other}`"),
    };
    let agent_root = skill_roots()
        .into_iter()
        .find(|root| root.source == agent_source)
        .with_context(|| format!("skills root `{agent_source}` not found"))?;
    if request.enabled {
        let strategy = if request.replace_existing {
            "backup-replace"
        } else {
            "cancel"
        };
        link_skill_into_agent_root(&store_path, &agent_root, strategy)?;
    } else {
        unlink_skill_from_agent_root(&store_path, &agent_root)?;
    }
    build_skill_info(&store_root, &store_path)
}

fn unique_skill_target_path(root: &Path, directory_name: &str) -> PathBuf {
    let base = sanitize_skill_directory_name(directory_name);
    let mut index = 2;
    loop {
        let candidate = root.join(format!("{base}-{index}"));
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

fn link_skill_into_agent_root(
    store_target: &Path,
    target_root: &SkillRoot,
    conflict_strategy: &str,
) -> Result<PathBuf> {
    fs::create_dir_all(&target_root.path).with_context(|| {
        format!(
            "failed to create skills root {}",
            target_root.path.display()
        )
    })?;
    let directory_name = store_target
        .file_name()
        .and_then(|name| name.to_str())
        .context("skill store target has no directory name")?;
    if let Some(existing) = find_agent_link_for_store_skill(store_target, target_root) {
        return Ok(existing);
    }
    let mut link_path = target_root.path.join(directory_name);
    if link_path.exists() || fs::symlink_metadata(&link_path).is_ok() {
        match conflict_strategy {
            "overwrite" if is_symlink(&link_path) => {
                fs::remove_file(&link_path)
                    .with_context(|| format!("failed to remove {}", link_path.display()))?;
            }
            "backup-replace" => {
                let backup_path = backup_existing_agent_skill_path(&link_path)?;
                fs::rename(&link_path, &backup_path).with_context(|| {
                    format!(
                        "failed to backup {} to {}",
                        link_path.display(),
                        backup_path.display()
                    )
                })?;
            }
            "overwrite" => {
                anyhow::bail!(
                    "refusing to replace non-symlink agent skill path {}",
                    link_path.display()
                );
            }
            "cancel" => {
                anyhow::bail!("agent skill path already exists at {}", link_path.display());
            }
            _ => {
                link_path = unique_skill_target_path(&target_root.path, directory_name);
            }
        }
    }
    create_dir_symlink(store_target, &link_path)?;
    Ok(link_path)
}

fn unlink_skill_from_agent_root(store_target: &Path, target_root: &SkillRoot) -> Result<()> {
    let Some(link_path) = find_agent_link_for_store_skill(store_target, target_root) else {
        return Ok(());
    };
    fs::remove_file(&link_path).with_context(|| format!("failed to remove {}", link_path.display()))
}

fn backup_existing_agent_skill_path(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .context("agent skill path has no parent directory")?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("agent skill path has no file name")?;
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let mut candidate = parent.join(format!("{name}.backup-{timestamp}"));
    let mut index = 2;
    while candidate.exists() || fs::symlink_metadata(&candidate).is_ok() {
        candidate = parent.join(format!("{name}.backup-{timestamp}-{index}"));
        index += 1;
    }
    Ok(candidate)
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn is_symlink_to(path: &Path, target: &Path) -> bool {
    if !is_symlink(path) {
        return false;
    }
    match (path.canonicalize(), target.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(unix)]
fn create_dir_symlink(target: &Path, link_path: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link_path).with_context(|| {
        format!(
            "failed to link {} -> {}",
            link_path.display(),
            target.display()
        )
    })
}

#[cfg(windows)]
fn create_dir_symlink(target: &Path, link_path: &Path) -> Result<()> {
    std::os::windows::fs::symlink_dir(target, link_path).with_context(|| {
        format!(
            "failed to link {} -> {}",
            link_path.display(),
            target.display()
        )
    })
}

fn scan_git_skills_from_request(request: SkillGitScanRequest) -> Result<SkillGitScanResult> {
    let git_url = request.git_url.trim();
    if git_url.is_empty() {
        anyhow::bail!("Git URL is required.");
    }
    let checkout_path = create_git_skill_checkout_dir()?;
    if let Err(error) = clone_skill_repo(git_url, request.git_ref.as_deref(), &checkout_path) {
        let _ = fs::remove_dir_all(&checkout_path);
        return Err(error);
    }
    let candidates = match scan_skill_candidates_in_checkout(&checkout_path) {
        Ok(candidates) => candidates,
        Err(error) => {
            let _ = fs::remove_dir_all(&checkout_path);
            return Err(error);
        }
    };
    if candidates.is_empty() {
        let _ = fs::remove_dir_all(&checkout_path);
        anyhow::bail!("no SKILL.md files found in Git repository");
    }
    Ok(SkillGitScanResult {
        checkout_path: checkout_path.to_string_lossy().into_owned(),
        candidates,
    })
}

fn create_git_skill_checkout_dir() -> Result<PathBuf> {
    let base = git_skill_checkout_base_dir();
    fs::create_dir_all(&base).with_context(|| format!("failed to create {}", base.display()))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let path = base.join(format!("checkout-{}-{now}", std::process::id()));
    fs::create_dir_all(&path).with_context(|| format!("failed to create {}", path.display()))?;
    Ok(path)
}

fn git_skill_checkout_base_dir() -> PathBuf {
    env::temp_dir().join("localairouter-git-skills")
}

fn clone_skill_repo(git_url: &str, git_ref: Option<&str>, checkout_path: &Path) -> Result<()> {
    let mut failures = Vec::new();

    match run_git_clone(git_url, git_ref, checkout_path, false) {
        Ok(()) => return Ok(()),
        Err(error) => failures.push(format!("git clone: {error}")),
    }

    match run_git_clone(git_url, git_ref, checkout_path, true) {
        Ok(()) => return Ok(()),
        Err(error) => failures.push(format!("git clone with HTTP/1.1: {error}")),
    }

    if let Some(repo) = parse_github_repo_url(git_url) {
        match checkout_github_archive(&repo, git_ref, checkout_path) {
            Ok(()) => return Ok(()),
            Err(error) => failures.push(format!("GitHub zip fallback: {error}")),
        }
    }

    anyhow::bail!(
        "failed to checkout Git repository:\n- {}",
        failures.join("\n- ")
    )
}

fn run_git_clone(
    git_url: &str,
    git_ref: Option<&str>,
    checkout_path: &Path,
    force_http1: bool,
) -> std::result::Result<(), String> {
    reset_empty_checkout_dir(checkout_path)
        .map_err(|error| format!("failed to prepare checkout directory: {error}"))?;
    let mut command = Command::new("git");
    if force_http1 {
        command.arg("-c").arg("http.version=HTTP/1.1");
    }
    command.arg("clone").arg("--depth").arg("1");
    let git_ref = git_ref.map(str::trim).filter(|value| !value.is_empty());
    if let Some(git_ref) = git_ref {
        command.arg("--branch").arg(git_ref);
    }
    command.arg(git_url).arg(checkout_path);
    run_command_capture(&mut command)
}

fn run_command_capture(command: &mut Command) -> std::result::Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("failed to run command: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let message = if stderr.is_empty() { stdout } else { stderr };
    if message.is_empty() {
        Err(format!("exited with status {}", output.status))
    } else {
        Err(message)
    }
}

#[derive(Debug, Clone)]
struct GitHubRepo {
    owner: String,
    repo: String,
}

fn parse_github_repo_url(value: &str) -> Option<GitHubRepo> {
    let mut rest = value.trim().trim_end_matches('/').to_owned();
    if let Some(stripped) = rest.strip_prefix("https://github.com/") {
        rest = stripped.to_owned();
    } else if let Some(stripped) = rest.strip_prefix("http://github.com/") {
        rest = stripped.to_owned();
    } else if let Some(stripped) = rest.strip_prefix("git://github.com/") {
        rest = stripped.to_owned();
    } else if let Some(stripped) = rest.strip_prefix("ssh://git@github.com/") {
        rest = stripped.to_owned();
    } else if let Some(stripped) = rest.strip_prefix("git@github.com:") {
        rest = stripped.to_owned();
    } else {
        return None;
    }
    let mut segments = rest.split('/');
    let owner = segments.next()?.trim();
    let repo = segments.next()?.trim().trim_end_matches(".git");
    if owner.is_empty()
        || repo.is_empty()
        || owner.contains("..")
        || repo.contains("..")
        || owner.contains('\\')
        || repo.contains('\\')
    {
        return None;
    }
    Some(GitHubRepo {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
    })
}

fn checkout_github_archive(
    repo: &GitHubRepo,
    git_ref: Option<&str>,
    checkout_path: &Path,
) -> Result<()> {
    let zip_path = checkout_path.with_extension("zip");
    let archive_dir = checkout_path.with_extension("archive");
    let mut failures = Vec::new();

    for url in github_archive_urls(repo, git_ref) {
        reset_empty_checkout_dir(checkout_path)?;
        let _ = fs::remove_file(&zip_path);
        let _ = fs::remove_dir_all(&archive_dir);
        match download_github_archive(&url, &zip_path)
            .and_then(|_| extract_zip_archive(&zip_path, &archive_dir))
            .and_then(|_| materialize_github_archive_checkout(&archive_dir, checkout_path))
        {
            Ok(()) => {
                let _ = fs::remove_file(&zip_path);
                let _ = fs::remove_dir_all(&archive_dir);
                return Ok(());
            }
            Err(error) => {
                failures.push(format!("{url}: {error}"));
            }
        }
    }

    let _ = fs::remove_file(&zip_path);
    let _ = fs::remove_dir_all(&archive_dir);
    anyhow::bail!("{}", failures.join("\n"))
}

fn github_archive_urls(repo: &GitHubRepo, git_ref: Option<&str>) -> Vec<String> {
    let base = format!("https://github.com/{}/{}", repo.owner, repo.repo);
    let git_ref = git_ref.map(str::trim).filter(|value| !value.is_empty());
    if let Some(git_ref) = git_ref {
        return vec![
            format!("{base}/archive/refs/heads/{git_ref}.zip"),
            format!("{base}/archive/refs/tags/{git_ref}.zip"),
            format!("{base}/archive/{git_ref}.zip"),
        ];
    }
    vec![
        format!("{base}/archive/HEAD.zip"),
        format!("{base}/archive/refs/heads/main.zip"),
        format!("{base}/archive/refs/heads/master.zip"),
    ]
}

fn download_github_archive(url: &str, zip_path: &Path) -> Result<()> {
    let mut command = Command::new("curl");
    command
        .arg("-L")
        .arg("--fail")
        .arg("--silent")
        .arg("--show-error")
        .arg("--connect-timeout")
        .arg("20")
        .arg("--max-time")
        .arg("180")
        .arg("--output")
        .arg(zip_path)
        .arg(url);
    run_command_capture(&mut command).map_err(|error| anyhow::anyhow!("curl failed: {error}"))
}

fn extract_zip_archive(zip_path: &Path, archive_dir: &Path) -> Result<()> {
    fs::create_dir_all(archive_dir)
        .with_context(|| format!("failed to create {}", archive_dir.display()))?;

    let mut ditto = Command::new("ditto");
    ditto.arg("-x").arg("-k").arg(zip_path).arg(archive_dir);
    match run_command_capture(&mut ditto) {
        Ok(()) => return Ok(()),
        Err(ditto_error) => {
            let _ = fs::remove_dir_all(archive_dir);
            fs::create_dir_all(archive_dir)
                .with_context(|| format!("failed to recreate {}", archive_dir.display()))?;
            let mut unzip = Command::new("unzip");
            unzip.arg("-q").arg(zip_path).arg("-d").arg(archive_dir);
            run_command_capture(&mut unzip).map_err(|unzip_error| {
                anyhow::anyhow!("ditto failed: {ditto_error}; unzip failed: {unzip_error}")
            })?;
        }
    }
    Ok(())
}

fn materialize_github_archive_checkout(archive_dir: &Path, checkout_path: &Path) -> Result<()> {
    let mut entries = fs::read_dir(archive_dir)
        .with_context(|| format!("failed to read {}", archive_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect::<Vec<_>>();
    entries.sort();
    let archive_root = if entries.len() == 1 && entries[0].is_dir() {
        entries.remove(0)
    } else {
        archive_dir.to_path_buf()
    };
    copy_dir_contents(&archive_root, checkout_path)
}

fn copy_dir_contents(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)
        .with_context(|| format!("failed to create directory {}", target.display()))?;
    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", source.display()))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)
            .with_context(|| format!("failed to stat {}", source_path.display()))?;
        if metadata.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn reset_empty_checkout_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))
}

fn scan_skill_candidates_in_checkout(checkout_path: &Path) -> Result<Vec<SkillImportPreview>> {
    let checkout = checkout_path
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", checkout_path.display()))?;
    let mut candidates = Vec::new();
    if checkout.join("SKILL.md").exists() {
        candidates.push(checkout.clone());
    }
    collect_skill_candidate_paths(&checkout, &mut candidates)?;
    candidates.sort();
    candidates.dedup();
    candidates
        .into_iter()
        .map(|path| {
            let mut preview = preview_skill_import_path(&path)?;
            preview.relative_path = path
                .strip_prefix(&checkout)
                .ok()
                .map(|relative| relative.to_string_lossy().into_owned())
                .filter(|relative| !relative.is_empty())
                .or_else(|| Some(".".to_owned()));
            Ok(preview)
        })
        .collect()
}

fn collect_skill_candidate_paths(path: &Path, candidates: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))? {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", path.display()))?;
        let entry_path = entry.path();
        let file_name = entry.file_name();
        if file_name.to_string_lossy() == ".git" {
            continue;
        }
        let metadata = fs::symlink_metadata(&entry_path)
            .with_context(|| format!("failed to stat {}", entry_path.display()))?;
        if metadata.is_dir() {
            if entry_path.join("SKILL.md").exists() {
                candidates.push(entry_path);
            } else {
                collect_skill_candidate_paths(&entry_path, candidates)?;
            }
        }
    }
    Ok(())
}

fn cleanup_skill_import_root(cleanup_root: Option<&str>, imported_source: &Path) -> Result<()> {
    let Some(cleanup_root) = cleanup_root
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let cleanup_root = PathBuf::from(cleanup_root)
        .canonicalize()
        .with_context(|| format!("failed to resolve cleanup root {cleanup_root}"))?;
    let allowed_root = git_skill_checkout_base_dir()
        .canonicalize()
        .unwrap_or_else(|_| git_skill_checkout_base_dir());
    if !cleanup_root.starts_with(&allowed_root) {
        anyhow::bail!(
            "refusing to cleanup non-temporary path {}",
            cleanup_root.display()
        );
    }
    if !imported_source
        .canonicalize()
        .unwrap_or_else(|_| imported_source.to_path_buf())
        .starts_with(&cleanup_root)
    {
        anyhow::bail!(
            "refusing to cleanup unrelated path {}",
            cleanup_root.display()
        );
    }
    remove_git_skill_checkout_path(&cleanup_root)?;
    Ok(())
}

fn remove_git_skill_checkout_root(cleanup_root: &str) -> Result<()> {
    let cleanup_root = cleanup_root.trim();
    if cleanup_root.is_empty() {
        return Ok(());
    }
    let cleanup_root = PathBuf::from(cleanup_root);
    if !cleanup_root.exists() {
        return Ok(());
    }
    let cleanup_root = cleanup_root
        .canonicalize()
        .with_context(|| format!("failed to resolve cleanup root {}", cleanup_root.display()))?;
    let allowed_root = git_skill_checkout_base_dir()
        .canonicalize()
        .unwrap_or_else(|_| git_skill_checkout_base_dir());
    if !cleanup_root.starts_with(&allowed_root) {
        anyhow::bail!(
            "refusing to cleanup non-temporary path {}",
            cleanup_root.display()
        );
    }
    remove_git_skill_checkout_path(&cleanup_root)?;
    Ok(())
}

fn remove_git_skill_checkout_path(cleanup_root: &Path) -> Result<()> {
    if cleanup_root.exists() {
        fs::remove_dir_all(cleanup_root)
            .with_context(|| format!("failed to remove {}", cleanup_root.display()))?;
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)
        .with_context(|| format!("failed to create directory {}", target.display()))?;
    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", source.display()))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)
            .with_context(|| format!("failed to stat {}", source_path.display()))?;
        if metadata.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn sanitize_skill_directory_name(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if matches!(ch, '-' | '_' | ' ' | '.') && !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "skill".to_owned()
    } else {
        trimmed
    }
}

fn parse_skill_frontmatter(contents: &str) -> (Option<String>, Option<String>) {
    let trimmed = contents.trim_start();
    let Some(rest) = trimmed.strip_prefix("---") else {
        return (None, None);
    };
    let Some((frontmatter, _body)) = rest.split_once("\n---") else {
        return (None, None);
    };
    let mut name = None;
    let mut description = None;
    for line in frontmatter.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = unquote_frontmatter_value(value.trim());
        match key.trim() {
            "name" if !value.is_empty() => name = Some(value),
            "description" if !value.is_empty() => description = Some(value),
            _ => {}
        }
    }
    (name, description)
}

fn unquote_frontmatter_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let quoted = (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'');
        if quoted {
            return value[1..value.len() - 1].to_owned();
        }
    }
    value.to_owned()
}

fn system_time_to_rfc3339(time: SystemTime) -> Option<String> {
    let duration = time.duration_since(UNIX_EPOCH).ok()?;
    chrono::DateTime::<Utc>::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
        .map(|time| time.to_rfc3339())
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
        converter: AccountConverter::None,
        use_http_proxy: false,
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
        converter: AccountConverter::None,
        use_http_proxy: false,
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

#[cfg(unix)]
fn terminate_external_daemon_on_port(port: u16, log_file: &Path) -> Result<bool> {
    let pids = local_daemon_pids_on_port(port)?;
    if pids.is_empty() {
        return Ok(false);
    }

    let mut signaled = false;
    for pid in &pids {
        append_supervisor_event(
            log_file,
            format!("desktop: stopping external daemon pid {pid} on port {port}"),
        )?;
        signal_process(*pid, libc::SIGTERM)?;
        signaled = true;
    }

    if wait_for_daemon_shutdown(port, Duration::from_secs(4), Duration::from_millis(100)) {
        return Ok(signaled);
    }

    for pid in &pids {
        append_supervisor_event(
            log_file,
            format!("desktop: force stopping external daemon pid {pid} on port {port}"),
        )?;
        signal_process(*pid, libc::SIGKILL)?;
    }
    wait_for_daemon_shutdown(port, Duration::from_secs(2), Duration::from_millis(100));
    Ok(signaled)
}

#[cfg(not(unix))]
fn terminate_external_daemon_on_port(_port: u16, _log_file: &Path) -> Result<bool> {
    Ok(false)
}

#[cfg(unix)]
fn local_daemon_pids_on_port(port: u16) -> Result<Vec<u32>> {
    let output = Command::new("lsof")
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-t"])
        .output();
    let Ok(output) = output else {
        return Ok(Vec::new());
    };
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut pids = Vec::new();
    for line in stdout.lines() {
        let Some(pid) = line.trim().parse::<u32>().ok() else {
            continue;
        };
        if process_command_contains(pid, DAEMON_BINARY_NAME)? {
            pids.push(pid);
        }
    }
    Ok(pids)
}

#[cfg(unix)]
fn process_command_contains(pid: u32, needle: &str) -> Result<bool> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()?;
    if !output.status.success() {
        return Ok(false);
    }
    Ok(String::from_utf8_lossy(&output.stdout).contains(needle))
}

#[cfg(unix)]
fn signal_process(pid: u32, signal: i32) -> Result<()> {
    let rc = unsafe { libc::kill(pid as i32, signal) };
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
        .env("LANG", "en_US.UTF-8")
        .env("LC_CTYPE", "en_US.UTF-8")
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

    if cfg!(debug_assertions) {
        if let Some(path) = workspace_daemon_binary().filter(|path| daemon_binary_is_fresh(path)) {
            return Ok(binary_spawn_spec(path));
        }

        if let Some(manifest) = workspace_manifest() {
            let binary = build_workspace_daemon_binary(&manifest)?;
            return Ok(binary_spawn_spec(binary));
        }

        if let Some(path) = workspace_daemon_binary() {
            return Ok(binary_spawn_spec(path));
        }

        let manifest = workspace_manifest().context("failed to resolve workspace Cargo.toml")?;
        return Ok(cargo_spawn_spec(manifest));
    }

    if let Some(path) = sibling_daemon_binary() {
        return Ok(binary_spawn_spec(path));
    }

    Err(anyhow::anyhow!(
        "bundled daemon binary `{DAEMON_BINARY_NAME}` was not found; rebuild the app package so the daemon sidecar is included"
    ))
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
        #[cfg(target_os = "macos")]
        {
            tray.set_icon(Some(load_macos_tray_icon()?))
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            tray.set_icon_as_template(true)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            reinforce_macos_tray_template(&tray);
        }
        return Ok(());
    }

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(tray_tooltip(&locale))
        .show_menu_on_left_click(true);
    #[cfg(target_os = "macos")]
    {
        builder = builder.icon(load_macos_tray_icon()?).icon_as_template(true);
    }
    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder
        .build(app)
        .map(|tray| {
            #[cfg(target_os = "macos")]
            reinforce_macos_tray_template(&tray);
            tray
        })
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn load_macos_tray_icon() -> Result<tauri::image::Image<'static>> {
    tauri::image::Image::from_bytes(MACOS_TRAY_ICON_PNG)
        .map(|icon| icon.to_owned())
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("failed to load tray-template.png")
}

#[cfg(target_os = "macos")]
fn reinforce_macos_tray_template<R: tauri::Runtime>(tray: &tauri::tray::TrayIcon<R>) {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSImage, NSImageScaling};
    use objc2_foundation::{NSData, NSSize};

    if let Err(error) = tray.with_inner_tray_icon(|inner| {
        if let Some(status_item) = inner.ns_status_item() {
            if let Some(mtm) = MainThreadMarker::new() {
                if let Some(button) = status_item.button(mtm) {
                    let data = NSData::with_bytes(MACOS_TRAY_ICON_PNG);
                    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
                        image.setSize(NSSize::new(18.0, 18.0));
                        image.setTemplate(true);
                        button.setImageScaling(NSImageScaling::ScaleProportionallyDown);
                        button.setImage(Some(&image));
                    }
                }
            }
        }
    }) {
        warn!("failed to reinforce macOS tray template image: {error}");
    }
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
    pick_folder_native("Select traffic logs directory", initial_path)
}

fn pick_folder_native(
    prompt: &str,
    initial_path: Option<String>,
) -> std::result::Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let initial_path = initial_path
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .filter(|path| path.exists());
        let mut script = format!(
            "set chosenFolder to choose folder with prompt \"{}\"",
            escape_applescript_string(prompt)
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
        let _ = prompt;
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
    let mut candidates = daemon_binary_candidates_in(parent);

    #[cfg(target_os = "macos")]
    if parent.file_name().and_then(|name| name.to_str()) == Some("MacOS") {
        if let Some(contents_dir) = parent.parent() {
            candidates.extend(daemon_binary_candidates_in(&contents_dir.join("Resources")));
            candidates.extend(daemon_binary_candidates_in(
                &contents_dir.join("Frameworks"),
            ));
        }
    }

    candidates.into_iter().find(|candidate| candidate.exists())
}

fn daemon_binary_candidates_in(dir: &Path) -> Vec<PathBuf> {
    vec![
        dir.join(daemon_binary_name()),
        dir.join(format!("{}.exe", daemon_binary_name())),
    ]
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
        SkillRoot, build_skill_info, copy_dir_recursive, find_agent_link_for_store_skill,
        import_claude_account_contents, import_codex_account_contents, link_skill_into_agent_root,
        parse_github_repo_url, preview_skill_import_path, scan_skill_candidates_in_checkout,
        sync_claude_config_contents, sync_codex_config_contents, unique_skill_target_path,
        unlink_skill_from_agent_root,
    };
    use std::fs;
    use std::path::PathBuf;

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

    #[test]
    fn skill_import_preview_and_copy_preserve_definition() {
        let workspace = test_temp_dir("skill-import-preview-and-copy");
        let source = workspace.join("source skill");
        fs::create_dir_all(source.join("scripts")).expect("create source");
        fs::write(
            source.join("SKILL.md"),
            r#"---
name: "Source Skill"
description: "A local test skill"
---

Use this skill in tests.
"#,
        )
        .expect("write skill definition");
        fs::write(source.join("scripts").join("run.sh"), "echo ok\n").expect("write nested file");

        let preview = preview_skill_import_path(&source).expect("preview skill");
        assert_eq!(preview.name, "Source Skill");
        assert_eq!(preview.description, "A local test skill");
        assert_eq!(preview.directory_name, "source-skill");

        let target_root = workspace.join("target");
        fs::create_dir_all(target_root.join("source-skill")).expect("create conflict");
        let target = unique_skill_target_path(&target_root, &preview.directory_name);
        copy_dir_recursive(&source, &target).expect("copy skill");

        assert_eq!(
            target.file_name().and_then(|name| name.to_str()),
            Some("source-skill-2")
        );
        assert!(target.join("SKILL.md").exists());
        assert!(target.join("scripts").join("run.sh").exists());

        let root = SkillRoot {
            source: "codex-user".into(),
            source_label: "Codex Link".into(),
            path: target_root,
            readonly: false,
            link_target: true,
        };
        let info = build_skill_info(&root, &target).expect("build skill info");
        assert_eq!(info.name, "Source Skill");
        assert_eq!(info.description, "A local test skill");

        fs::remove_dir_all(workspace).expect("cleanup");
    }

    #[test]
    fn git_skill_scan_finds_root_and_nested_skill_directories() {
        let workspace = test_temp_dir("git-skill-scan");
        fs::write(
            workspace.join("SKILL.md"),
            r#"---
name: Root Skill
description: Root description
---
"#,
        )
        .expect("write root skill");
        let nested = workspace.join("packages").join("nested-skill");
        fs::create_dir_all(nested.join("assets")).expect("create nested skill");
        fs::write(
            nested.join("SKILL.md"),
            r#"---
name: Nested Skill
description: Nested description
---
"#,
        )
        .expect("write nested skill");
        fs::write(nested.join("assets").join("template.txt"), "template").expect("write asset");

        let candidates = scan_skill_candidates_in_checkout(&workspace).expect("scan checkout");
        let names = candidates
            .iter()
            .map(|candidate| candidate.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"Root Skill"));
        assert!(names.contains(&"Nested Skill"));
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.relative_path.as_deref() == Some("."))
        );
        assert!(
            candidates.iter().any(
                |candidate| candidate.relative_path.as_deref() == Some("packages/nested-skill")
            )
        );

        fs::remove_dir_all(workspace).expect("cleanup");
    }

    #[test]
    fn github_repo_url_parser_accepts_https_and_ssh_forms() {
        let https = parse_github_repo_url("https://github.com/ConardLi/garden-skills/")
            .expect("parse https url");
        assert_eq!(https.owner, "ConardLi");
        assert_eq!(https.repo, "garden-skills");

        let ssh = parse_github_repo_url("git@github.com:ConardLi/garden-skills.git")
            .expect("parse ssh url");
        assert_eq!(ssh.owner, "ConardLi");
        assert_eq!(ssh.repo, "garden-skills");
    }

    #[cfg(unix)]
    #[test]
    fn link_skill_into_agent_root_points_to_store_copy() {
        let workspace = test_temp_dir("skill-link-agent-root");
        let store_skill = workspace.join("store").join("source-skill");
        fs::create_dir_all(store_skill.join("assets")).expect("create store skill");
        fs::write(
            store_skill.join("SKILL.md"),
            r#"---
name: Linked Skill
description: Linked description
---
"#,
        )
        .expect("write skill definition");
        fs::write(store_skill.join("assets").join("template.txt"), "template")
            .expect("write asset");

        let agent_root = SkillRoot {
            source: "codex-user".into(),
            source_label: "Codex Link".into(),
            path: workspace.join("codex-skills"),
            readonly: false,
            link_target: true,
        };
        let link_path =
            link_skill_into_agent_root(&store_skill, &agent_root, "rename").expect("link skill");

        assert!(link_path.join("SKILL.md").exists());
        assert!(link_path.join("assets").join("template.txt").exists());
        assert!(
            fs::symlink_metadata(&link_path)
                .expect("link metadata")
                .file_type()
                .is_symlink()
        );

        fs::remove_dir_all(workspace).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn link_skill_into_agent_root_renames_when_real_directory_exists() {
        let workspace = test_temp_dir("skill-link-agent-conflict");
        let store_skill = workspace.join("store").join("source-skill");
        fs::create_dir_all(&store_skill).expect("create store skill");
        fs::write(
            store_skill.join("SKILL.md"),
            r#"---
name: Linked Skill
---
"#,
        )
        .expect("write skill definition");

        let agent_root = SkillRoot {
            source: "agents-user".into(),
            source_label: "Agents Link".into(),
            path: workspace.join("agent-skills"),
            readonly: false,
            link_target: true,
        };
        fs::create_dir_all(agent_root.path.join("source-skill")).expect("create real conflict dir");
        let link_path =
            link_skill_into_agent_root(&store_skill, &agent_root, "rename").expect("link skill");

        assert_eq!(
            link_path.file_name().and_then(|name| name.to_str()),
            Some("source-skill-2")
        );
        assert!(find_agent_link_for_store_skill(&store_skill, &agent_root).is_some());

        unlink_skill_from_agent_root(&store_skill, &agent_root).expect("unlink skill");
        assert!(find_agent_link_for_store_skill(&store_skill, &agent_root).is_none());
        assert!(agent_root.path.join("source-skill").is_dir());

        fs::remove_dir_all(workspace).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn link_skill_into_agent_root_backs_up_existing_directory_when_requested() {
        let workspace = test_temp_dir("skill-link-agent-backup");
        let store_skill = workspace.join("store").join("source-skill");
        fs::create_dir_all(&store_skill).expect("create store skill");
        fs::write(
            store_skill.join("SKILL.md"),
            r#"---
name: Linked Skill
---
"#,
        )
        .expect("write skill definition");

        let agent_root = SkillRoot {
            source: "agents-user".into(),
            source_label: "Agents Link".into(),
            path: workspace.join("agent-skills"),
            readonly: false,
            link_target: true,
        };
        let existing = agent_root.path.join("source-skill");
        fs::create_dir_all(&existing).expect("create real conflict dir");
        fs::write(existing.join("old.txt"), "old").expect("write old file");

        let link_path = link_skill_into_agent_root(&store_skill, &agent_root, "backup-replace")
            .expect("link skill");

        assert_eq!(link_path, existing);
        assert!(
            fs::symlink_metadata(&link_path)
                .expect("link metadata")
                .file_type()
                .is_symlink()
        );
        let backup_exists = fs::read_dir(&agent_root.path)
            .expect("read agent root")
            .flatten()
            .any(|entry| {
                let path = entry.path();
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("source-skill.backup-"))
                    .unwrap_or(false)
                    && path.join("old.txt").exists()
            });
        assert!(backup_exists);

        fs::remove_dir_all(workspace).expect("cleanup");
    }

    fn test_temp_dir(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("localairouter-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }
}
