#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
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
use localopenrouter_core::AppPaths;
use serde::Serialize;
use tauri::{Manager, Url};
use tracing::{error, warn};

const DAEMON_BINARY_NAME: &str = "localopenrouter-daemon";
const DAEMON_PORT_ENV: &str = "LOCALOPENROUTER_PORT";
const LEGACY_DAEMON_PORT_ENV: &str = "LOCALROUTER_PORT";
const DAEMON_PARENT_PID_ENV: &str = "LOCALOPENROUTER_PARENT_PID";
const LEGACY_DAEMON_PARENT_PID_ENV: &str = "LOCALROUTER_PARENT_PID";
const DEFAULT_DAEMON_PORT: u16 = 7331;
const UI_DEV_ENV: &str = "LOCALOPENROUTER_UI_DEV";
const UI_DEV_PORT_ENV: &str = "LOCALOPENROUTER_UI_DEV_PORT";
const DEFAULT_UI_DEV_PORT: u16 = 7456;

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
    tracing_subscriber::fmt()
        .with_env_filter("localopenrouter_desktop=info")
        .with_target(false)
        .compact()
        .init();

    let supervisor = DaemonSupervisor::new();
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
        .manage(supervisor)
        .invoke_handler(tauri::generate_handler![
            daemon_status,
            start_daemon,
            stop_daemon,
            restart_daemon,
            open_daemon_log,
            open_log_file,
            open_logs_root
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
        .run(tauri::generate_context!())
        .inspect_err(|error| error!("tauri run failure: {error}"))
        .ok();
    shutdown_supervisor.shutdown();
}

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
    let parent_pid = std::process::id().to_string();
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    let stderr = stdout.try_clone()?;
    command
        .env(DAEMON_PORT_ENV, &port)
        .env(DAEMON_PARENT_PID_ENV, &parent_pid)
        .env(LEGACY_DAEMON_PORT_ENV, &port)
        .env(LEGACY_DAEMON_PARENT_PID_ENV, &parent_pid)
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
    configured_port(DAEMON_PORT_ENV, LEGACY_DAEMON_PORT_ENV, DEFAULT_DAEMON_PORT)
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

fn select_spawn_spec() -> Result<SpawnSpec> {
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

fn configured_port(primary_env: &str, legacy_env: &str, default_port: u16) -> u16 {
    parse_port_env(primary_env)
        .or_else(|| parse_port_env(legacy_env))
        .unwrap_or(default_port)
}

fn parse_port_env(name: &str) -> Option<u16> {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|port| *port != 0)
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
            .join("localopenrouter-core")
            .join("Cargo.toml"),
        root.join("crates")
            .join("localopenrouter-daemon")
            .join("Cargo.toml"),
        root.join("crates").join("localopenrouter-core").join("src"),
        root.join("crates")
            .join("localopenrouter-daemon")
            .join("src"),
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
    if !cfg!(debug_assertions) || !env_flag_enabled(UI_DEV_ENV) {
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

    let port = parse_port_env(UI_DEV_PORT_ENV).unwrap_or(DEFAULT_UI_DEV_PORT);
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

fn env_flag_enabled(name: &str) -> bool {
    matches!(
        env::var(name).ok().as_deref().map(str::trim),
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
        return Err("absolute paths are not allowed".into());
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("path traversal is not allowed".into());
    }
    Ok(AppPaths::discover()
        .map_err(|error| error.to_string())?
        .root
        .join(relative))
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

fn daemon_log_file() -> Result<PathBuf> {
    let root = AppPaths::discover()?.root.join("daemon");
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
