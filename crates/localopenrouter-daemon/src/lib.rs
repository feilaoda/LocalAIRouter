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
use localopenrouter_core::models::{
    AccountInput, ApiProtocol, LogQuery, ProviderDefinition, ProviderInput, RequestLogInput,
    RevealSecretRequest, RouteBindingInput, UnlockRequest,
};
use localopenrouter_core::{LocalOpenRouterError, Repository, Result, extract_model};
use reqwest::Client;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};
use uuid::Uuid;

mod config;

pub use config::{
    DAEMON_BINARY_NAME, DAEMON_PARENT_PID_ENV, DAEMON_PORT_ENV, DEFAULT_TRACING_FILTER,
    DaemonConfig, LEGACY_DAEMON_PARENT_PID_ENV, LEGACY_DAEMON_PORT_ENV,
};

type BoxError = Box<dyn Error + Send + Sync>;
type ResponseBody = BoxBody<Bytes, BoxError>;
const MONITOR_FEED_CAP: usize = 200;
const MONITOR_PREVIEW_LIMIT: usize = 280;

#[derive(Clone)]
struct AppState {
    repository: Arc<Repository>,
    client: Client,
    monitor: Arc<MonitorFeed>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MonitorEntry {
    id: String,
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

    fn mark_routed(&self, id: &str, account_id: &str) {
        self.update_entry(id, |entry| {
            entry.account_id = Some(account_id.into());
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
    run_with_config(DaemonConfig::from_env()).await
}

pub async fn run_with_config(config: DaemonConfig) -> Result<()> {
    init_tracing(&config.tracing_filter);
    let port = config.port;
    info!("localopenrouter daemon booting on requested port {port}");
    spawn_parent_watchdog(config.parent_pid);
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|error| {
            LocalOpenRouterError::Io(format!("failed to bind 127.0.0.1:{port}: {error}"))
        })?;
    let repository = Arc::new(Repository::new(port).await.map_err(|error| {
        LocalOpenRouterError::Message(format!("failed to initialize repository: {error}"))
    })?);
    let client = Client::builder().no_gzip().build().map_err(map_http)?;
    let monitor = Arc::new(MonitorFeed::new(MONITOR_FEED_CAP));
    let state = AppState {
        repository,
        client,
        monitor,
    };
    let addr = listener.local_addr()?;
    info!("localopenrouter daemon listening on http://{addr}");

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
        (Method::POST, "/admin/unlock") => {
            let unlock = parse_json::<UnlockRequest>(request).await?;
            let response = state.repository.unlock(&unlock.master_password).await?;
            Ok(json_response(StatusCode::OK, &response))
        }
        (Method::POST, "/admin/lock") => {
            state.repository.lock().await;
            Ok(json_response(
                StatusCode::OK,
                &localopenrouter_core::models::UnlockResponse {
                    initialized: state.repository.is_initialized().await?,
                    unlocked: false,
                    message: "vault locked".into(),
                },
            ))
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
        (Method::POST, _) if path.starts_with("/admin/accounts/") && path.ends_with("/reveal") => {
            let account_id = path
                .trim_start_matches("/admin/accounts/")
                .trim_end_matches("/reveal")
                .trim_end_matches('/');
            let reveal = parse_json::<RevealSecretRequest>(request).await?;
            let response = state
                .repository
                .reveal_account_secret(account_id, &reveal.master_password)
                .await?;
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
            None => Err(LocalOpenRouterError::NotFound(path)),
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
    let request_body_text = String::from_utf8_lossy(&request_body).into_owned();
    let model = extract_model(&request_body);
    let start = Instant::now();
    let monitor_id = state.monitor.start(
        &provider.slug,
        model.clone(),
        &method,
        &request_path,
        &request_body_text,
    );
    let resolved = match state
        .repository
        .resolve_account(&provider.slug, model.as_deref())
        .await
    {
        Ok(resolved) => {
            state.monitor.mark_routed(&monitor_id, &resolved.account.id);
            resolved
        }
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
    let upstream_url = format!(
        "{}{}{}",
        resolved.upstream_base_url.trim_end_matches('/'),
        if upstream_path.is_empty() {
            "/"
        } else {
            upstream_path
        },
        query
    );

    let mut builder = state
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
            state.monitor.complete(
                &monitor_id,
                Some(StatusCode::BAD_GATEWAY.as_u16()),
                start.elapsed().as_millis() as u64,
                Some(error.to_string()),
                None,
                false,
            );
            let _ = state
                .repository
                .insert_log(RequestLogInput {
                    provider: resolved.provider.slug.clone(),
                    model,
                    account_id: Some(resolved.account.id.clone()),
                    method: method.to_string(),
                    path: request_path.clone(),
                    status_code: Some(StatusCode::BAD_GATEWAY.as_u16()),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error_text: Some(error.to_string()),
                    request_headers: request_headers.clone(),
                    request_body: request_body_text,
                    response_headers: "{}".into(),
                    response_body: String::new(),
                    streamed: false,
                })
                .await;
            return Err(LocalOpenRouterError::Http(error.to_string()));
        }
    };

    let status = StatusCode::from_u16(upstream_response.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
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
                    Some(status.as_u16()),
                    start.elapsed().as_millis() as u64,
                    Some(error.to_string()),
                    None,
                    false,
                );
                return Err(error);
            }
        };
        let response_body = String::from_utf8_lossy(&body).into_owned();
        let duration_ms = start.elapsed().as_millis() as u64;
        state.monitor.complete(
            &monitor_id,
            Some(status.as_u16()),
            duration_ms,
            None,
            Some(response_body.clone()),
            false,
        );
        let _ = state
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
                streamed: false,
            })
            .await;

        let mut response = Response::builder()
            .status(status)
            .body(full_body(body))
            .map_err(|error| LocalOpenRouterError::Http(error.to_string()))?;
        copy_headers(&upstream_headers, response.headers_mut());
        Ok(cors_response(response))
    }
}

fn apply_provider_auth(
    builder: reqwest::RequestBuilder,
    provider: &ProviderDefinition,
    api_key: &str,
) -> Result<reqwest::RequestBuilder> {
    let header_name = reqwest::header::HeaderName::from_bytes(provider.auth_header.as_bytes())
        .map_err(|error| {
            LocalOpenRouterError::Validation(format!(
                "invalid auth header `{}`: {error}",
                provider.auth_header
            ))
        })?;
    let header_value = match provider.auth_prefix.as_deref() {
        Some(prefix) if !prefix.is_empty() => format!("{prefix} {api_key}"),
        _ => api_key.to_owned(),
    };
    let header_value = reqwest::header::HeaderValue::from_str(&header_value).map_err(|error| {
        LocalOpenRouterError::Validation(format!(
            "invalid auth header value for provider `{}`: {error}",
            provider.slug
        ))
    })?;
    Ok(builder.header(header_name, header_value))
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
        let _ = repository
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
    });

    let body = BodyExt::boxed(StreamBody::new(receiver));
    let mut response = Response::builder()
        .status(status)
        .body(body)
        .map_err(|error| LocalOpenRouterError::Http(error.to_string()))?;
    copy_headers(&upstream_headers, response.headers_mut());
    Ok(cors_response(response))
}

fn parse_log_query(raw: Option<&str>) -> LogQuery {
    let mut query = LogQuery::default();
    for pair in raw
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
    {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default();
        match key {
            "provider" if !value.is_empty() => query.provider = Some(value.into()),
            "accountId" | "account_id" if !value.is_empty() => {
                query.account_id = Some(value.into())
            }
            "sessionId" | "session_id" if !value.is_empty() => {
                query.session_id = Some(value.into())
            }
            "statusCode" | "status_code" => query.status_code = value.parse().ok(),
            "limit" => query.limit = value.parse().ok(),
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
        .map_err(|error| LocalOpenRouterError::Validation(format!("invalid JSON payload: {error}")))
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

fn error_response(error: LocalOpenRouterError) -> Response<ResponseBody> {
    let status = match &error {
        LocalOpenRouterError::Validation(_) => StatusCode::BAD_REQUEST,
        LocalOpenRouterError::Locked => StatusCode::LOCKED,
        LocalOpenRouterError::NotFound(_) => StatusCode::NOT_FOUND,
        LocalOpenRouterError::Http(_) => StatusCode::BAD_GATEWAY,
        LocalOpenRouterError::Sqlite(_)
        | LocalOpenRouterError::Crypto(_)
        | LocalOpenRouterError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        LocalOpenRouterError::Message(_) => StatusCode::BAD_REQUEST,
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

fn map_http(error: impl ToString) -> LocalOpenRouterError {
    LocalOpenRouterError::Http(error.to_string())
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
