use std::collections::{BTreeMap, VecDeque};
use std::convert::Infallible;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bytes::Bytes;
use chrono::Utc;
use futures::channel::mpsc;
use futures_util::stream::StreamExt;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::header::{self, HeaderMap, HeaderName, HeaderValue};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use localairouter_core::models::{
    AccountConverter, AccountInput, ApiProtocol, DailyStatsQuery, LogQuery, ProviderDefinition,
    ProviderInput, RequestLogInput, ResolvedAccount, RouteBindingInput,
};
use localairouter_core::{LocalAIRouterError, Repository, Result, extract_model};
use reqwest::{Client, Proxy};
use serde::Serialize;
use serde_json::Value as JsonValue;
use tokio::net::TcpListener;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};
use url::form_urlencoded;
use uuid::Uuid;

mod config;
mod converter;

pub use config::{
    DAEMON_ALLOW_LAN_ENV, DAEMON_BINARY_NAME, DAEMON_PARENT_PID_ENV, DAEMON_PORT_ENV,
    DEFAULT_TRACING_FILTER, DaemonConfig, LEGACY_DAEMON_ALLOW_LAN_ENV,
    LEGACY_DAEMON_PARENT_PID_ENV, LEGACY_DAEMON_PORT_ENV,
};

type BoxError = Box<dyn Error + Send + Sync>;
type ResponseBody = BoxBody<Bytes, BoxError>;
const MONITOR_PREVIEW_LIMIT: usize = 280;

#[derive(Clone)]
struct AppState {
    repository: Arc<Repository>,
    client: Client,
    monitor: Arc<MonitorFeed>,
    response_store: Arc<converter::ResponseStore>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MonitorEntry {
    id: String,
    log_id: Option<String>,
    started_at: String,
    updated_at: String,
    provider: String,
    model: Option<String>,
    account_id: Option<String>,
    method: String,
    path: String,
    status_code: Option<u16>,
    duration_ms: Option<u64>,
    error_text: Option<String>,
    upstream_url: Option<String>,
    network_mode: Option<String>,
    http_proxy_url: Option<String>,
    converter: Option<String>,
    request_preview: String,
    response_preview: String,
    phase: String,
    streamed: bool,
}

#[derive(Debug)]
struct MonitorFeed {
    capacity: usize,
    entries: Mutex<VecDeque<MonitorEntry>>,
}

impl MonitorFeed {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: Mutex::new(VecDeque::new()),
        }
    }

    fn start(
        &self,
        provider: &str,
        model: Option<String>,
        method: &Method,
        path: &str,
        request_body: &str,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        let now = timestamp();
        let entry = MonitorEntry {
            id: id.clone(),
            log_id: None,
            started_at: now.clone(),
            updated_at: now,
            provider: provider.to_owned(),
            model,
            account_id: None,
            method: method.to_string(),
            path: path.to_owned(),
            status_code: None,
            duration_ms: None,
            error_text: None,
            upstream_url: None,
            network_mode: None,
            http_proxy_url: None,
            converter: None,
            request_preview: preview_text(request_body),
            response_preview: String::new(),
            phase: "routing".into(),
            streamed: false,
        };
        let mut entries = self.entries.lock().expect("monitor feed");
        entries.push_front(entry);
        self.trim_locked(&mut entries);
        id
    }

    fn mark_routed(
        &self,
        id: &str,
        account_id: &str,
        model: Option<String>,
        request_body: &str,
        upstream_url: &str,
        network_mode: &str,
        http_proxy_url: Option<&str>,
        converter: Option<&str>,
    ) {
        self.update_entry(id, |entry| {
            entry.account_id = Some(account_id.into());
            entry.model = model;
            entry.upstream_url = Some(upstream_url.into());
            entry.network_mode = Some(network_mode.into());
            entry.http_proxy_url = http_proxy_url.map(str::to_owned);
            entry.converter = converter.map(str::to_owned);
            entry.request_preview = preview_text(request_body);
            entry.phase = "upstream".into();
        });
    }

    fn mark_response(&self, id: &str, status_code: u16, streamed: bool) {
        self.update_entry(id, |entry| {
            entry.status_code = Some(status_code);
            entry.streamed = streamed;
            entry.phase = if streamed {
                "streaming".into()
            } else {
                "response".into()
            };
        });
    }

    fn append_response_chunk(&self, id: &str, chunk: &[u8]) {
        let preview = preview_text(String::from_utf8_lossy(chunk).as_ref());
        if preview.is_empty() {
            return;
        }
        self.update_entry(id, |entry| {
            entry.phase = "streaming".into();
            entry.response_preview = merge_preview(&entry.response_preview, &preview);
        });
    }

    fn complete(
        &self,
        id: &str,
        status_code: Option<u16>,
        duration_ms: u64,
        error_text: Option<String>,
        response_body: Option<String>,
        streamed: bool,
    ) {
        self.update_entry(id, |entry| {
            entry.status_code = status_code.or(entry.status_code);
            entry.duration_ms = Some(duration_ms);
            entry.error_text = error_text.clone();
            entry.streamed = streamed;
            if let Some(response_body) = response_body.as_ref() {
                entry.response_preview = preview_text(response_body);
            }
            entry.phase = if entry.error_text.is_some() {
                "failed".into()
            } else {
                "completed".into()
            };
        });
    }

    fn attach_log(&self, id: &str, log_id: String) {
        self.update_entry(id, |entry| {
            entry.log_id = Some(log_id);
        });
    }

    fn query(&self, query: LogQuery) -> Vec<MonitorEntry> {
        let limit = query.limit.unwrap_or(50) as usize;
        let entries = self.entries.lock().expect("monitor feed");
        entries
            .iter()
            .filter(|entry| {
                query
                    .provider
                    .as_deref()
                    .map(|provider| provider == entry.provider)
                    .unwrap_or(true)
                    && query
                        .account_id
                        .as_deref()
                        .map(|account_id| entry.account_id.as_deref() == Some(account_id))
                        .unwrap_or(true)
                    && query
                        .status_code
                        .map(|status_code| entry.status_code == Some(status_code))
                        .unwrap_or(true)
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn update_entry<F>(&self, id: &str, mutator: F)
    where
        F: FnOnce(&mut MonitorEntry),
    {
        let mut entries = self.entries.lock().expect("monitor feed");
        if let Some(index) = entries.iter().position(|entry| entry.id == id) {
            if let Some(mut entry) = entries.remove(index) {
                mutator(&mut entry);
                entry.updated_at = timestamp();
                entries.push_front(entry);
                self.trim_locked(&mut entries);
            }
        }
    }

    fn trim_locked(&self, entries: &mut VecDeque<MonitorEntry>) {
        while entries.len() > self.capacity {
            let removal_index = entries
                .iter()
                .rposition(|entry| {
                    entry.phase != "routing"
                        && entry.phase != "upstream"
                        && entry.phase != "response"
                        && entry.phase != "streaming"
                })
                .unwrap_or(entries.len() - 1);
            entries.remove(removal_index);
        }
    }
}

pub async fn run() -> Result<()> {
    ignore_terminal_hangup();
    run_with_config(DaemonConfig::from_env()).await
}

#[cfg(unix)]
fn ignore_terminal_hangup() {
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
    }
}

#[cfg(not(unix))]
fn ignore_terminal_hangup() {}

pub async fn run_with_config(config: DaemonConfig) -> Result<()> {
    init_tracing(&config.tracing_filter);
    let port = config.port;
    let bind_addr = SocketAddr::from((config.bind_ip(), port));
    info!(
        "localairouter daemon booting on requested address {bind_addr} with monitor buffer {}",
        config.monitor_buffer_limit,
    );
    spawn_parent_watchdog(config.parent_pid);
    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(|error| LocalAIRouterError::Io(format!("failed to bind {bind_addr}: {error}")))?;
    let repository = Arc::new(Repository::new(port).await.map_err(|error| {
        LocalAIRouterError::Message(format!("failed to initialize repository: {error}"))
    })?);
    let client = Client::builder().no_gzip().build().map_err(map_http)?;
    let monitor = Arc::new(MonitorFeed::new(config.monitor_buffer_limit));
    let response_store = Arc::new(converter::ResponseStore::new());
    let state = AppState {
        repository,
        client,
        monitor,
        response_store,
    };
    let addr = listener.local_addr()?;
    info!("localairouter daemon listening on http://{addr}");

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let state = state.clone();
        tokio::spawn(async move {
            let service =
                service_fn(move |request| route_request(state.clone(), request, remote_addr));
            if let Err(error) = http1::Builder::new().serve_connection(io, service).await {
                warn!("connection error: {error}");
            }
        });
    }
}

async fn route_request(
    state: AppState,
    request: Request<Incoming>,
    _remote_addr: SocketAddr,
) -> std::result::Result<Response<ResponseBody>, Infallible> {
    let response = match handle_request(state, request).await {
        Ok(response) => response,
        Err(error) => error_response(error),
    };
    Ok(response)
}

async fn handle_request(
    state: AppState,
    request: Request<Incoming>,
) -> Result<Response<ResponseBody>> {
    if request.method() == Method::OPTIONS {
        return Ok(cors_response(empty_response(StatusCode::NO_CONTENT)));
    }

    let path = request.uri().path().to_owned();
    match (request.method().clone(), path.as_str()) {
        (Method::GET, "/health") => {
            let health = state.repository.health().await?;
            Ok(json_response(StatusCode::OK, &health))
        }

        (Method::GET, "/admin/providers") => {
            let providers = state.repository.list_providers().await?;
            Ok(json_response(StatusCode::OK, &providers))
        }
        (Method::POST, "/admin/providers") => {
            let provider = parse_json::<ProviderInput>(request).await?;
            let response = state.repository.upsert_provider(provider).await?;
            Ok(json_response(StatusCode::OK, &response))
        }
        (Method::DELETE, _) if path.starts_with("/admin/providers/") => {
            let slug = path.trim_start_matches("/admin/providers/");
            let response = state.repository.delete_provider(slug).await?;
            Ok(json_response(StatusCode::OK, &response))
        }
        (Method::GET, "/admin/accounts") => {
            let accounts = state.repository.list_accounts().await?;
            Ok(json_response(StatusCode::OK, &accounts))
        }
        (Method::POST, "/admin/accounts") => {
            let account = parse_json::<AccountInput>(request).await?;
            let response = state.repository.upsert_account(account).await?;
            Ok(json_response(StatusCode::OK, &response))
        }

        (Method::GET, "/admin/routes") => {
            let routes = state.repository.list_routes().await?;
            Ok(json_response(StatusCode::OK, &routes))
        }
        (Method::POST, "/admin/routes") => {
            let binding = parse_json::<RouteBindingInput>(request).await?;
            let response = state.repository.set_route_binding(binding).await?;
            Ok(json_response(StatusCode::OK, &response))
        }
        (Method::GET, _) if path.starts_with("/admin/logs/") => {
            let log_id = path.trim_start_matches("/admin/logs/");
            let log = state.repository.get_log(log_id).await?;
            Ok(json_response(StatusCode::OK, &log))
        }
        (Method::GET, _) if path.starts_with("/admin/stats/daily") => {
            let query = parse_daily_stats_query(request.uri().query());
            let stats = state.repository.query_daily_stats(query).await?;
            Ok(json_response(StatusCode::OK, &stats))
        }
        (Method::POST, "/admin/stats/rebuild-tokens") => {
            let report = state.repository.rebuild_total_tokens().await?;
            Ok(json_response(StatusCode::OK, &report))
        }
        (Method::GET, _) if path.starts_with("/admin/logs") => {
            let query = parse_log_query(request.uri().query());
            let logs = state.repository.query_logs(query).await?;
            Ok(json_response(StatusCode::OK, &logs))
        }
        (Method::GET, _) if path.starts_with("/admin/monitor") => {
            let query = parse_log_query(request.uri().query());
            let entries = state.monitor.query(query);
            Ok(json_response(StatusCode::OK, &entries))
        }
        (Method::GET, _) if path.starts_with("/admin/onboarding/") => {
            let target = path.trim_start_matches("/admin/onboarding/");
            let guide = state.repository.onboarding(target).await?;
            Ok(json_response(StatusCode::OK, &guide))
        }
        (Method::POST, _) if path.starts_with("/admin/accounts/") && path.ends_with("/disable") => {
            let account_id = path
                .trim_start_matches("/admin/accounts/")
                .trim_end_matches("/disable")
                .trim_end_matches('/');
            let account = state.repository.disable_account(account_id).await?;
            Ok(json_response(StatusCode::OK, &account))
        }
        (Method::DELETE, _) if path.starts_with("/admin/accounts/") => {
            let account_id = path.trim_start_matches("/admin/accounts/");
            let response = state.repository.delete_account(account_id).await?;
            Ok(json_response(StatusCode::OK, &response))
        }
        (Method::DELETE, _) if path.starts_with("/admin/routes/") => {
            let route_id = path.trim_start_matches("/admin/routes/");
            let response = state.repository.delete_route_binding(route_id).await?;
            Ok(json_response(StatusCode::OK, &response))
        }
        _ => match resolve_provider_for_path(&state, &path).await? {
            Some(provider) => proxy_request(state, provider, request).await,
            None => Err(LocalAIRouterError::NotFound(path)),
        },
    }
}

async fn resolve_provider_for_path(
    state: &AppState,
    path: &str,
) -> Result<Option<ProviderDefinition>> {
    let proxy_path = path
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or_default();
    if proxy_path.is_empty() {
        return Ok(None);
    }
    state
        .repository
        .find_provider_by_proxy_path(proxy_path)
        .await
}

async fn proxy_request(
    state: AppState,
    provider: ProviderDefinition,
    request: Request<Incoming>,
) -> Result<Response<ResponseBody>> {
    let method = request.method().clone();
    let request_headers = sanitize_headers(request.headers());
    let original_request_headers = request.headers().clone();
    let request_path = request.uri().path().to_owned();
    let query = request
        .uri()
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let provider_prefix = format!("/{}", provider.proxy_path);
    let upstream_path = request_path
        .strip_prefix(provider_prefix.as_str())
        .unwrap_or_default();
    let request_body = request
        .into_body()
        .collect()
        .await
        .map_err(map_http)?
        .to_bytes();
    let client_model = extract_model(&request_body);
    let initial_request_body_text = String::from_utf8_lossy(&request_body).into_owned();
    let start = Instant::now();
    let monitor_id = state.monitor.start(
        &provider.slug,
        client_model.clone(),
        &method,
        &request_path,
        &initial_request_body_text,
    );
    let routing_model = provider
        .default_model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .or(client_model.as_deref());
    let resolved = match state
        .repository
        .resolve_account(&provider.slug, routing_model)
        .await
    {
        Ok(resolved) => resolved,
        Err(error) => {
            state.monitor.complete(
                &monitor_id,
                None,
                start.elapsed().as_millis() as u64,
                Some(error.to_string()),
                None,
                false,
            );
            return Err(error);
        }
    };
    let default_model = resolved
        .account
        .default_model
        .as_deref()
        .or(resolved.provider.default_model.as_deref());
    let (request_body, client_request_body_text, model) =
        apply_account_default_model(request_body, default_model);
    let conversion = prepare_account_conversion(
        &resolved,
        &state.response_store,
        upstream_path,
        request_body,
    )?;
    let PreparedRequest {
        client_upstream_path,
        upstream_path,
        query: converted_query,
        body: request_body,
        logged_body_text: request_body_text,
        model: converted_model,
        converter_label,
    } = conversion;
    let query = converted_query.unwrap_or(query);
    let model = converted_model.or(model);
    let upstream_url = format!(
        "{}{}{}",
        resolved.upstream_base_url.trim_end_matches('/'),
        if upstream_path.is_empty() {
            "/"
        } else {
            upstream_path.as_str()
        },
        query
    );

    let upstream_client = match upstream_client_for_account(&state, &resolved).await {
        Ok(client) => client,
        Err(error) => {
            let network_mode = requested_network_mode(&resolved);
            state.monitor.mark_routed(
                &monitor_id,
                &resolved.account.id,
                model.clone(),
                &request_body_text,
                &upstream_url,
                network_mode,
                None,
                converter_label.as_deref(),
            );
            let request_body_text = prepend_log_diagnostics(
                &request_body_text,
                &client_upstream_path,
                &upstream_path,
                &upstream_url,
                converter_label.as_deref(),
                network_mode,
                None,
            );
            let status = match &error {
                LocalAIRouterError::Validation(_) | LocalAIRouterError::Message(_) => {
                    StatusCode::BAD_REQUEST
                }
                _ => StatusCode::SERVICE_UNAVAILABLE,
            };
            state.monitor.complete(
                &monitor_id,
                Some(status.as_u16()),
                start.elapsed().as_millis() as u64,
                Some(error.to_string()),
                None,
                false,
            );
            let inserted_log = state
                .repository
                .insert_log(RequestLogInput {
                    provider: resolved.provider.slug.clone(),
                    model,
                    account_id: Some(resolved.account.id.clone()),
                    method: method.to_string(),
                    path: request_path.clone(),
                    status_code: Some(status.as_u16()),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error_text: Some(error.to_string()),
                    request_headers: request_headers.clone(),
                    request_body: request_body_text,
                    response_headers: "{}".into(),
                    response_body: String::new(),
                    streamed: false,
                })
                .await;
            if let Ok(log) = inserted_log {
                state.monitor.attach_log(&monitor_id, log.id);
            }
            return Err(error);
        }
    };
    state.monitor.mark_routed(
        &monitor_id,
        &resolved.account.id,
        model.clone(),
        &request_body_text,
        &upstream_url,
        upstream_client.network_mode,
        upstream_client.http_proxy_url.as_deref(),
        converter_label.as_deref(),
    );
    let request_body_text = prepend_log_diagnostics(
        &request_body_text,
        &client_upstream_path,
        &upstream_path,
        &upstream_url,
        converter_label.as_deref(),
        upstream_client.network_mode,
        upstream_client.http_proxy_url.as_deref(),
    );

    let mut builder = upstream_client
        .client
        .request(
            reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(map_http)?,
            upstream_url.clone(),
        )
        .body(request_body.clone());

    for (name, value) in request_headers_for_upstream(&original_request_headers) {
        builder = builder.header(name, value);
    }

    builder = apply_provider_auth(builder, &resolved.provider, &resolved.api_key)?;
    if resolved.provider.protocol == ApiProtocol::Anthropic {
        builder = builder.header(
            "anthropic-version",
            original_request_headers
                .get("anthropic-version")
                .cloned()
                .unwrap_or_else(|| HeaderValue::from_static("2023-06-01")),
        );
    }

    let upstream_response = match builder.send().await {
        Ok(response) => response,
        Err(error) => {
            let error_text = upstream_transport_error_text(&upstream_client, &error);
            state.monitor.complete(
                &monitor_id,
                Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
                start.elapsed().as_millis() as u64,
                Some(error_text.clone()),
                None,
                false,
            );
            let inserted_log = state
                .repository
                .insert_log(RequestLogInput {
                    provider: resolved.provider.slug.clone(),
                    model,
                    account_id: Some(resolved.account.id.clone()),
                    method: method.to_string(),
                    path: request_path.clone(),
                    status_code: Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error_text: Some(error_text.clone()),
                    request_headers: request_headers.clone(),
                    request_body: request_body_text,
                    response_headers: "{}".into(),
                    response_body: String::new(),
                    streamed: false,
                })
                .await;
            if let Ok(log) = inserted_log {
                state.monitor.attach_log(&monitor_id, log.id);
            }
            return Err(LocalAIRouterError::Http(error_text));
        }
    };

    let status = StatusCode::from_u16(upstream_response.status().as_u16())
        .unwrap_or(StatusCode::SERVICE_UNAVAILABLE);
    let response_headers = sanitize_reqwest_headers(upstream_response.headers());
    let streamed = upstream_response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("text/event-stream"))
        .unwrap_or(false);

    if streamed {
        state
            .monitor
            .mark_response(&monitor_id, status.as_u16(), true);
        if converter_label.is_some() && converter::is_openai_responses_path(&client_upstream_path) {
            return Err(LocalAIRouterError::Validation(
                "responses-to-chat-completions converter currently buffers upstream streaming responses; the upstream request was sent as non-streaming".into(),
            ));
        }
        stream_response(
            state,
            monitor_id,
            resolved.provider.slug,
            request_headers,
            request_body_text,
            request_path,
            model,
            resolved.account.id,
            status,
            response_headers,
            upstream_response,
            method,
            start,
        )
        .await
    } else {
        state
            .monitor
            .mark_response(&monitor_id, status.as_u16(), false);
        let upstream_headers = upstream_response.headers().clone();
        let body = match upstream_response.bytes().await {
            Ok(body) => body,
            Err(error) => {
                let error = map_http(error);
                state.monitor.complete(
                    &monitor_id,
                    Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
                    start.elapsed().as_millis() as u64,
                    Some(error.to_string()),
                    None,
                    false,
                );
                let inserted_log = state
                    .repository
                    .insert_log(RequestLogInput {
                        provider: resolved.provider.slug,
                        model,
                        account_id: Some(resolved.account.id),
                        method: method.to_string(),
                        path: request_path,
                        status_code: Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_text: Some(error.to_string()),
                        request_headers,
                        request_body: request_body_text,
                        response_headers: sanitize_reqwest_headers(&upstream_headers),
                        response_body: String::new(),
                        streamed: false,
                    })
                    .await;
                if let Ok(log) = inserted_log {
                    state.monitor.attach_log(&monitor_id, log.id);
                }
                return Err(error);
            }
        };
        let converted_response = convert_account_response(
            &state.response_store,
            &client_upstream_path,
            converter_label.as_deref(),
            status,
            &client_request_body_text,
            body,
            &original_request_headers,
        )?;
        let body = converted_response.body;
        let response_body = converted_response.logged_body_text;
        let duration_ms = start.elapsed().as_millis() as u64;
        state.monitor.complete(
            &monitor_id,
            Some(status.as_u16()),
            duration_ms,
            None,
            Some(response_body.clone()),
            converted_response.streamed,
        );
        let inserted_log = state
            .repository
            .insert_log(RequestLogInput {
                provider: resolved.provider.slug,
                model,
                account_id: Some(resolved.account.id),
                method: method.to_string(),
                path: request_path,
                status_code: Some(status.as_u16()),
                duration_ms,
                error_text: None,
                request_headers,
                request_body: request_body_text,
                response_headers,
                response_body: response_body.clone(),
                streamed: converted_response.streamed,
            })
            .await;
        if let Ok(log) = inserted_log {
            state.monitor.attach_log(&monitor_id, log.id);
        }

        let mut response = Response::builder()
            .status(status)
            .body(full_body(body.clone()))
            .map_err(|error| LocalAIRouterError::Http(error.to_string()))?;
        copy_headers(&upstream_headers, response.headers_mut());
        if let Some(content_type) = converted_response.content_type {
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
        }
        Ok(cors_response(response))
    }
}

struct PreparedRequest {
    client_upstream_path: String,
    upstream_path: String,
    query: Option<String>,
    body: Bytes,
    logged_body_text: String,
    model: Option<String>,
    converter_label: Option<String>,
}

struct PreparedResponse {
    body: Bytes,
    logged_body_text: String,
    content_type: Option<&'static str>,
    streamed: bool,
}

fn prepare_account_conversion(
    resolved: &ResolvedAccount,
    response_store: &converter::ResponseStore,
    upstream_path: &str,
    request_body: Bytes,
) -> Result<PreparedRequest> {
    match resolved.account.converter {
        AccountConverter::None => {
            if let Some(converter_label) =
                auto_chat_completions_converter_label(resolved, upstream_path, &request_body)
            {
                return prepare_chat_completions_conversion(
                    response_store,
                    upstream_path,
                    request_body,
                    converter_label,
                );
            }
            let logged_body_text = String::from_utf8_lossy(&request_body).into_owned();
            Ok(PreparedRequest {
                client_upstream_path: upstream_path.to_owned(),
                upstream_path: upstream_path.to_owned(),
                query: None,
                model: extract_model(&request_body),
                body: request_body,
                logged_body_text,
                converter_label: None,
            })
        }
        AccountConverter::DeepSeekV4ToOpenAi => {
            if resolved.provider.protocol != ApiProtocol::OpenAi {
                return Err(LocalAIRouterError::Validation(
                    "deepseek v4 converter can only be used with OpenAI protocol providers".into(),
                ));
            }
            prepare_chat_completions_conversion(
                response_store,
                upstream_path,
                request_body,
                AccountConverter::DeepSeekV4ToOpenAi.as_str(),
            )
        }
    }
}

fn prepare_chat_completions_conversion(
    response_store: &converter::ResponseStore,
    upstream_path: &str,
    request_body: Bytes,
    converter_label: &str,
) -> Result<PreparedRequest> {
    let converted = converter::convert_deepseek_v4_request(
        response_store,
        upstream_path,
        request_body.clone(),
    )?;
    match converted {
        Some(converted) => Ok(PreparedRequest {
            client_upstream_path: upstream_path.to_owned(),
            upstream_path: converted.upstream_path,
            query: converted.query,
            body: converted.body,
            logged_body_text: converted.logged_body_text,
            model: converted.model,
            converter_label: Some(converter_label.into()),
        }),
        None => {
            let logged_body_text = String::from_utf8_lossy(&request_body).into_owned();
            Ok(PreparedRequest {
                client_upstream_path: upstream_path.to_owned(),
                upstream_path: upstream_path.to_owned(),
                query: None,
                model: extract_model(&request_body),
                body: request_body,
                logged_body_text,
                converter_label: None,
            })
        }
    }
}

fn auto_chat_completions_converter_label(
    resolved: &ResolvedAccount,
    upstream_path: &str,
    request_body: &[u8],
) -> Option<&'static str> {
    if resolved.provider.protocol != ApiProtocol::OpenAi
        || !converter::is_openai_responses_path(upstream_path)
    {
        return None;
    }
    let base_url = resolved.upstream_base_url.to_ascii_lowercase();
    if base_url.contains("opencode.ai/zen/go") {
        return Some("openai-chat-completions(auto)");
    }
    let model = extract_model(request_body)
        .or_else(|| resolved.account.default_model.clone())
        .or_else(|| resolved.provider.default_model.clone())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if model.starts_with("deepseek") || base_url.contains("deepseek") {
        return Some("deepseek-v4-to-openai(auto)");
    }
    None
}

fn prepend_log_diagnostics(
    body: &str,
    client_upstream_path: &str,
    upstream_path: &str,
    upstream_url: &str,
    converter_label: Option<&str>,
    network_mode: &str,
    http_proxy_url: Option<&str>,
) -> String {
    let diagnostics = serde_json::json!({
        "client_upstream_path": client_upstream_path,
        "upstream_path": upstream_path,
        "upstream_url": upstream_url,
        "network_mode": network_mode,
        "http_proxy_url": http_proxy_url,
        "converter": converter_label,
    });
    let Ok(diagnostics) = serde_json::to_string(&diagnostics) else {
        return body.to_owned();
    };
    match serde_json::from_str::<JsonValue>(body) {
        Ok(mut value) => {
            if let Some(object) = value.as_object_mut() {
                object.insert(
                    "_localairouter".into(),
                    serde_json::from_str(&diagnostics).unwrap_or(JsonValue::Null),
                );
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| body.to_owned())
            } else {
                body.to_owned()
            }
        }
        Err(_) if body.trim().is_empty() => {
            format!(r#"{{"_localairouter":{diagnostics}}}"#)
        }
        Err(_) => body.to_owned(),
    }
}

fn convert_account_response(
    response_store: &converter::ResponseStore,
    upstream_path: &str,
    converter_label: Option<&str>,
    status: StatusCode,
    request_body_text: &str,
    response_body: Bytes,
    original_request_headers: &HeaderMap<HeaderValue>,
) -> Result<PreparedResponse> {
    if converter_label.is_some() {
        let converted = converter::convert_deepseek_v4_response(
            upstream_path,
            status,
            request_body_text,
            response_body,
        )?;
        if status.is_success() && converter::is_openai_responses_path(upstream_path) {
            let _ = response_store.store_response(request_body_text, &converted.body);
        }
        if status.is_success()
            && converter::is_openai_responses_path(upstream_path)
            && client_wants_responses_stream(original_request_headers, request_body_text)
        {
            let body = converter::response_json_to_sse(&converted.body)?;
            let logged_body_text = String::from_utf8_lossy(&body).into_owned();
            Ok(PreparedResponse {
                body,
                logged_body_text,
                content_type: Some("text/event-stream"),
                streamed: true,
            })
        } else {
            let content_type = if converter::is_openai_responses_path(upstream_path) {
                Some("application/json")
            } else {
                None
            };
            Ok(PreparedResponse {
                body: converted.body,
                logged_body_text: converted.logged_body_text,
                content_type,
                streamed: false,
            })
        }
    } else {
        let logged_body_text = String::from_utf8_lossy(&response_body).into_owned();
        Ok(PreparedResponse {
            body: response_body,
            logged_body_text,
            content_type: None,
            streamed: false,
        })
    }
}

fn apply_provider_auth(
    builder: reqwest::RequestBuilder,
    provider: &ProviderDefinition,
    api_key: &str,
) -> Result<reqwest::RequestBuilder> {
    let header_name = reqwest::header::HeaderName::from_bytes(provider.auth_header.as_bytes())
        .map_err(|error| {
            LocalAIRouterError::Validation(format!(
                "invalid auth header `{}`: {error}",
                provider.auth_header
            ))
        })?;
    let header_value = match provider.auth_prefix.as_deref() {
        Some(prefix) if !prefix.is_empty() => format!("{prefix} {api_key}"),
        _ => api_key.to_owned(),
    };
    let header_value = reqwest::header::HeaderValue::from_str(&header_value).map_err(|error| {
        LocalAIRouterError::Validation(format!(
            "invalid auth header value for provider `{}`: {error}",
            provider.slug
        ))
    })?;
    Ok(builder.header(header_name, header_value))
}

struct UpstreamClient {
    client: Client,
    network_mode: &'static str,
    http_proxy_url: Option<String>,
}

async fn upstream_client_for_account(
    state: &AppState,
    resolved: &ResolvedAccount,
) -> Result<UpstreamClient> {
    if !resolved.account.use_http_proxy {
        return Ok(UpstreamClient {
            client: state.client.clone(),
            network_mode: "direct",
            http_proxy_url: None,
        });
    }

    let settings = state.repository.app_settings().await?;
    let proxy_url = settings.http_proxy_url.ok_or_else(|| {
        LocalAIRouterError::Validation(
            "HTTP proxy URL is required when account HTTP proxy is enabled".into(),
        )
    })?;
    let proxy = Proxy::all(&proxy_url).map_err(|error| {
        LocalAIRouterError::Validation(format!("invalid HTTP proxy URL `{proxy_url}`: {error}"))
    })?;
    let client = Client::builder()
        .no_gzip()
        .proxy(proxy)
        .build()
        .map_err(map_http)?;
    Ok(UpstreamClient {
        client,
        network_mode: "proxy",
        http_proxy_url: Some(proxy_url),
    })
}

fn requested_network_mode(resolved: &ResolvedAccount) -> &'static str {
    if resolved.account.use_http_proxy {
        "proxy"
    } else {
        "direct"
    }
}

fn upstream_transport_error_text(network: &UpstreamClient, error: &reqwest::Error) -> String {
    match (network.network_mode, network.http_proxy_url.as_deref()) {
        ("proxy", Some(proxy_url)) => {
            format!("upstream request failed via HTTP proxy {proxy_url}: {error}")
        }
        ("proxy", None) => format!("upstream request failed via HTTP proxy: {error}"),
        _ => format!("upstream request failed directly: {error}"),
    }
}

#[allow(clippy::too_many_arguments)]
async fn stream_response(
    state: AppState,
    monitor_id: String,
    provider_slug: String,
    request_headers: String,
    request_body: String,
    request_path: String,
    model: Option<String>,
    account_id: String,
    status: StatusCode,
    response_headers: String,
    upstream_response: reqwest::Response,
    method: Method,
    start: Instant,
) -> Result<Response<ResponseBody>> {
    let upstream_headers = upstream_response.headers().clone();
    let mut stream = upstream_response.bytes_stream();
    let (sender, receiver) = mpsc::unbounded::<std::result::Result<Frame<Bytes>, BoxError>>();
    let repository = state.repository.clone();
    let monitor = state.monitor.clone();

    tokio::spawn(async move {
        let mut buffer = Vec::new();
        let mut error_text = None;
        while let Some(item) = stream.next().await {
            match item {
                Ok(chunk) => {
                    buffer.extend_from_slice(&chunk);
                    monitor.append_response_chunk(&monitor_id, &chunk);
                    if sender.unbounded_send(Ok(Frame::data(chunk))).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    error_text = Some(error.to_string());
                    let _ = sender.unbounded_send(Err(Box::new(error)));
                    break;
                }
            }
        }
        let response_body = String::from_utf8_lossy(&buffer).into_owned();
        monitor.complete(
            &monitor_id,
            Some(status.as_u16()),
            start.elapsed().as_millis() as u64,
            error_text.clone(),
            Some(response_body.clone()),
            true,
        );
        let inserted_log = repository
            .insert_log(RequestLogInput {
                provider: provider_slug,
                model,
                account_id: Some(account_id),
                method: method.to_string(),
                path: request_path,
                status_code: Some(status.as_u16()),
                duration_ms: start.elapsed().as_millis() as u64,
                error_text,
                request_headers,
                request_body,
                response_headers,
                response_body,
                streamed: true,
            })
            .await;
        if let Ok(log) = inserted_log {
            monitor.attach_log(&monitor_id, log.id);
        }
    });

    let body = BodyExt::boxed(StreamBody::new(receiver));
    let mut response = Response::builder()
        .status(status)
        .body(body)
        .map_err(|error| LocalAIRouterError::Http(error.to_string()))?;
    copy_headers(&upstream_headers, response.headers_mut());
    Ok(cors_response(response))
}

fn parse_log_query(raw: Option<&str>) -> LogQuery {
    let mut query = LogQuery::default();
    for (key, value) in form_urlencoded::parse(raw.unwrap_or_default().as_bytes()) {
        match key {
            key if key == "provider" && !value.is_empty() => {
                query.provider = Some(value.into_owned())
            }
            key if (key == "accountId" || key == "account_id") && !value.is_empty() => {
                query.account_id = Some(value.into_owned())
            }
            key if (key == "sessionId" || key == "session_id") && !value.is_empty() => {
                query.session_id = Some(value.into_owned())
            }
            key if key == "statusCode" || key == "status_code" => {
                query.status_code = value.parse().ok()
            }
            key if (key == "createdFrom" || key == "created_from") && !value.is_empty() => {
                query.created_from = Some(value.into_owned())
            }
            key if (key == "createdTo" || key == "created_to") && !value.is_empty() => {
                query.created_to = Some(value.into_owned())
            }
            key if key == "limit" => query.limit = value.parse().ok(),
            _ => {}
        }
    }
    query
}

fn parse_daily_stats_query(raw: Option<&str>) -> DailyStatsQuery {
    let mut query = DailyStatsQuery::default();
    for (key, value) in form_urlencoded::parse(raw.unwrap_or_default().as_bytes()) {
        match key {
            key if key == "days" => query.days = value.parse().ok(),
            key if key == "utcOffsetMinutes" || key == "utc_offset_minutes" => {
                query.utc_offset_minutes = value.parse().ok()
            }
            _ => {}
        }
    }
    query
}

async fn parse_json<T: serde::de::DeserializeOwned>(request: Request<Incoming>) -> Result<T> {
    let bytes = request
        .into_body()
        .collect()
        .await
        .map_err(map_http)?
        .to_bytes();
    serde_json::from_slice(&bytes)
        .map_err(|error| LocalAIRouterError::Validation(format!("invalid JSON payload: {error}")))
}

fn json_response<T: Serialize>(status: StatusCode, value: &T) -> Response<ResponseBody> {
    let body = serde_json::to_vec(value)
        .unwrap_or_else(|_| b"{\"error\":\"serialization failure\"}".to_vec());
    let mut response = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(full_body(body))
        .expect("json response");
    response = cors_response(response);
    response
}

fn error_response(error: LocalAIRouterError) -> Response<ResponseBody> {
    let status = match &error {
        LocalAIRouterError::Validation(_) => StatusCode::BAD_REQUEST,
        LocalAIRouterError::NotFound(_) => StatusCode::NOT_FOUND,
        LocalAIRouterError::Http(_) => StatusCode::SERVICE_UNAVAILABLE,
        LocalAIRouterError::Sqlite(_) | LocalAIRouterError::Io(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
        LocalAIRouterError::Message(_) => StatusCode::BAD_REQUEST,
    };
    let payload = serde_json::json!({
        "error": error.to_string(),
        "status": status.as_u16(),
    });
    json_response(status, &payload)
}

fn full_body<T: Into<Bytes>>(body: T) -> ResponseBody {
    Full::new(body.into())
        .map_err(|never| match never {})
        .boxed()
}

fn empty_response(status: StatusCode) -> Response<ResponseBody> {
    Response::builder()
        .status(status)
        .body(
            Empty::<Bytes>::new()
                .map_err(|never| match never {})
                .boxed(),
        )
        .expect("empty response")
}

fn cors_response(mut response: Response<ResponseBody>) -> Response<ResponseBody> {
    let headers = response.headers_mut();
    headers.insert(
        HeaderName::from_static("access-control-allow-origin"),
        HeaderValue::from_static("*"),
    );
    headers.insert(
        HeaderName::from_static("access-control-allow-methods"),
        HeaderValue::from_static("GET,POST,DELETE,OPTIONS"),
    );
    headers.insert(
        HeaderName::from_static("access-control-allow-headers"),
        HeaderValue::from_static("Content-Type,Authorization,X-API-Key,Anthropic-Version"),
    );
    response
}

fn sanitize_headers(headers: &HeaderMap<HeaderValue>) -> String {
    let mut sanitized = BTreeMap::<String, String>::new();
    for (name, value) in headers {
        if is_sensitive_header(name.as_str()) {
            sanitized.insert(name.as_str().to_string(), "[redacted]".into());
        } else {
            sanitized.insert(
                name.as_str().to_string(),
                value.to_str().unwrap_or("<binary>").to_string(),
            );
        }
    }
    serde_json::to_string_pretty(&sanitized).unwrap_or_else(|_| "{}".into())
}

fn sanitize_reqwest_headers(headers: &reqwest::header::HeaderMap) -> String {
    let mut sanitized = BTreeMap::<String, String>::new();
    for (name, value) in headers {
        if is_sensitive_header(name.as_str()) {
            sanitized.insert(name.as_str().to_string(), "[redacted]".into());
        } else {
            sanitized.insert(
                name.as_str().to_string(),
                value.to_str().unwrap_or("<binary>").to_string(),
            );
        }
    }
    serde_json::to_string_pretty(&sanitized).unwrap_or_else(|_| "{}".into())
}

fn apply_account_default_model(
    request_body: Bytes,
    default_model: Option<&str>,
) -> (Bytes, String, Option<String>) {
    let Some(default_model) = default_model
        .map(str::trim)
        .filter(|default_model| !default_model.is_empty())
    else {
        let model = extract_model(&request_body);
        let request_body_text = String::from_utf8_lossy(&request_body).into_owned();
        return (request_body, request_body_text, model);
    };

    let Ok(mut json) = serde_json::from_slice::<JsonValue>(&request_body) else {
        let model = extract_model(&request_body);
        let request_body_text = String::from_utf8_lossy(&request_body).into_owned();
        return (request_body, request_body_text, model);
    };
    let Some(object) = json.as_object_mut() else {
        let model = extract_model(&request_body);
        let request_body_text = String::from_utf8_lossy(&request_body).into_owned();
        return (request_body, request_body_text, model);
    };

    object.insert("model".into(), JsonValue::String(default_model.to_owned()));
    match serde_json::to_vec(&json) {
        Ok(body) => {
            let request_body = Bytes::from(body);
            let request_body_text = String::from_utf8_lossy(&request_body).into_owned();
            (
                request_body,
                request_body_text,
                Some(default_model.to_owned()),
            )
        }
        Err(_) => {
            let model = extract_model(&request_body);
            let request_body_text = String::from_utf8_lossy(&request_body).into_owned();
            (request_body, request_body_text, model)
        }
    }
}

fn request_headers_for_upstream(
    original: &HeaderMap<HeaderValue>,
) -> Vec<(HeaderName, HeaderValue)> {
    original
        .iter()
        .filter(|(name, _)| !is_hop_by_hop(name.as_str()) && !is_sensitive_header(name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

fn copy_headers(source: &reqwest::header::HeaderMap, target: &mut HeaderMap<HeaderValue>) {
    for (name, value) in source {
        if !is_hop_by_hop(name.as_str()) {
            target.insert(name.clone(), value.clone());
        }
    }
}

fn client_wants_responses_stream(
    headers: &HeaderMap<HeaderValue>,
    request_body_text: &str,
) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("text/event-stream"))
        .unwrap_or(false)
        || serde_json::from_str::<JsonValue>(request_body_text)
            .ok()
            .and_then(|value| value.get("stream").and_then(JsonValue::as_bool))
            .unwrap_or(false)
}

fn is_sensitive_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization" | "proxy-authorization" | "x-api-key"
    )
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "host"
            | "content-length"
    )
}

fn map_http(error: impl ToString) -> LocalAIRouterError {
    LocalAIRouterError::Http(error.to_string())
}

fn init_tracing(tracing_filter: &str) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_filter)
        .with_target(false)
        .compact()
        .try_init();
}

fn timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn preview_text(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return String::new();
    }
    truncate_chars(&compact, MONITOR_PREVIEW_LIMIT)
}

fn merge_preview(existing: &str, next: &str) -> String {
    if existing.is_empty() {
        return next.to_owned();
    }
    preview_text(&format!("{existing} {next}"))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let truncated: String = value.chars().take(max_chars).collect();
    if value.chars().count() > max_chars {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn spawn_parent_watchdog(parent_pid: Option<u32>) {
    let Some(parent_pid) = parent_pid else {
        return;
    };

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(2)).await;
            if !parent_process_alive(parent_pid) {
                warn!("desktop parent process {parent_pid} is gone; shutting down daemon");
                std::process::exit(0);
            }
        }
    });
}

#[cfg(unix)]
fn parent_process_alive(parent_pid: u32) -> bool {
    let rc = unsafe { libc::kill(parent_pid as i32, 0) };
    if rc == 0 {
        return true;
    }
    let error = std::io::Error::last_os_error();
    !matches!(error.raw_os_error(), Some(libc::ESRCH))
}

#[cfg(not(unix))]
fn parent_process_alive(_parent_pid: u32) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{apply_account_default_model, prepare_account_conversion};
    use bytes::Bytes;
    use localairouter_core::models::{
        Account, AccountConverter, ApiProtocol, ProviderDefinition, ResolvedAccount,
    };

    #[test]
    fn account_default_model_overrides_existing_request_model() {
        let (body, text, model) = apply_account_default_model(
            Bytes::from_static(br#"{"model":"gpt-4.1","messages":[]}"#),
            Some("gpt-5.4"),
        );
        assert_eq!(model.as_deref(), Some("gpt-5.4"));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["model"].as_str(),
            Some("gpt-5.4")
        );
        assert!(text.contains("\"model\":\"gpt-5.4\""));
    }

    #[test]
    fn account_default_model_is_inserted_when_missing() {
        let (body, _, model) =
            apply_account_default_model(Bytes::from_static(br#"{"messages":[]}"#), Some("sonnet"));
        assert_eq!(model.as_deref(), Some("sonnet"));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["model"].as_str(),
            Some("sonnet")
        );
    }

    #[test]
    fn request_body_stays_unchanged_without_account_default_model() {
        let original = Bytes::from_static(br#"{"model":"gpt-4.1","messages":[]}"#);
        let (body, _, model) = apply_account_default_model(original.clone(), None);
        assert_eq!(model.as_deref(), Some("gpt-4.1"));
        assert_eq!(body, original);
    }

    #[test]
    fn deepseek_responses_request_auto_uses_converter() {
        let resolved = resolved_account(AccountConverter::None);
        let store = crate::converter::ResponseStore::new();
        let prepared = prepare_account_conversion(
            &resolved,
            &store,
            "/responses",
            Bytes::from_static(br#"{"model":"deepseek-v4-pro","input":"hello"}"#),
        )
        .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&prepared.body).unwrap();

        assert_eq!(prepared.client_upstream_path, "/responses");
        assert_eq!(prepared.upstream_path, "/chat/completions");
        assert_eq!(
            prepared.converter_label.as_deref(),
            Some("deepseek-v4-to-openai(auto)")
        );
        assert_eq!(body["model"].as_str(), Some("deepseek-v4-pro"));
        assert!(body["messages"].is_array());
    }

    #[test]
    fn opencode_go_responses_request_auto_uses_chat_completions() {
        let resolved =
            resolved_account_with_base_url("https://opencode.ai/zen/go/v1", AccountConverter::None);
        let store = crate::converter::ResponseStore::new();
        let prepared = prepare_account_conversion(
            &resolved,
            &store,
            "/responses",
            Bytes::from_static(br#"{"model":"glm-5","input":"hello"}"#),
        )
        .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&prepared.body).unwrap();

        assert_eq!(prepared.client_upstream_path, "/responses");
        assert_eq!(prepared.upstream_path, "/chat/completions");
        assert_eq!(
            format!("{}{}", resolved.upstream_base_url, prepared.upstream_path),
            "https://opencode.ai/zen/go/v1/chat/completions"
        );
        assert_eq!(
            prepared.converter_label.as_deref(),
            Some("openai-chat-completions(auto)")
        );
        assert_eq!(body["model"].as_str(), Some("glm-5"));
        assert!(body["messages"].is_array());
    }

    #[test]
    fn configured_deepseek_converter_uses_chat_completions_path() {
        let resolved = resolved_account(AccountConverter::DeepSeekV4ToOpenAi);
        let store = crate::converter::ResponseStore::new();
        let prepared = prepare_account_conversion(
            &resolved,
            &store,
            "/responses",
            Bytes::from_static(br#"{"model":"other-model","input":"hello"}"#),
        )
        .unwrap();

        assert_eq!(prepared.upstream_path, "/chat/completions");
        assert_eq!(
            prepared.converter_label.as_deref(),
            Some("deepseek-v4-to-openai")
        );
    }

    fn resolved_account(converter: AccountConverter) -> ResolvedAccount {
        resolved_account_with_base_url("https://api.deepseek.com/v1", converter)
    }

    fn resolved_account_with_base_url(
        upstream_base_url: &str,
        converter: AccountConverter,
    ) -> ResolvedAccount {
        ResolvedAccount {
            provider: ProviderDefinition {
                slug: "codex".into(),
                display_name: "Codex".into(),
                protocol: ApiProtocol::OpenAi,
                base_url: "https://api.openai.com".into(),
                default_model: None,
                proxy_path: "codex".into(),
                auth_header: "Authorization".into(),
                auth_prefix: Some("Bearer".into()),
                enabled: true,
                is_builtin: true,
                created_at: "2026-01-01T00:00:00Z".into(),
                updated_at: "2026-01-01T00:00:00Z".into(),
            },
            account: Account {
                id: "acct_deepseek".into(),
                provider: "codex".into(),
                name: "DeepSeek".into(),
                base_url: Some(upstream_base_url.into()),
                default_model: None,
                converter,
                use_http_proxy: false,
                enabled: true,
                note: None,
                api_key_masked: None,
                api_key: None,

                created_at: "2026-01-01T00:00:00Z".into(),
                updated_at: "2026-01-01T00:00:00Z".into(),
            },
            upstream_base_url: upstream_base_url.into(),
            api_key: "sk-test".into(),
        }
    }
}
