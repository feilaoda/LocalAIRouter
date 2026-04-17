use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::error::{LocalAIRouterError, Result};

pub const DEFAULT_MONITOR_BUFFER_LIMIT: u32 = 200;
pub const DEFAULT_LOG_RETENTION_DAYS: u32 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiProtocol {
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "generic")]
    Generic,
}

impl ApiProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Generic => "generic",
        }
    }
}

impl Display for ApiProtocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ApiProtocol {
    type Err = LocalAIRouterError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "openai" => Ok(Self::OpenAi),
            "anthropic" => Ok(Self::Anthropic),
            "generic" => Ok(Self::Generic),
            other => Err(LocalAIRouterError::Validation(format!(
                "unsupported protocol `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDefinition {
    pub slug: String,
    pub display_name: String,
    pub protocol: ApiProtocol,
    pub base_url: String,
    pub proxy_path: String,
    pub auth_header: String,
    pub auth_prefix: Option<String>,
    pub enabled: bool,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    pub slug: String,
    pub display_name: String,
    pub protocol: ApiProtocol,
    pub base_url: String,
    pub proxy_path: String,
    pub auth_header: String,
    pub auth_prefix: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub version: String,
    pub started_at: String,
    pub db_path: String,
    pub initialized: bool,
    pub unlocked: bool,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub daemon_port: u16,
    pub monitor_buffer_limit: u32,
    pub log_retention_days: u32,
    pub logs_dir: String,
    pub default_logs_dir: String,
    pub data_root: String,
    pub database_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsInput {
    pub daemon_port: u16,
    #[serde(default = "default_monitor_buffer_limit")]
    pub monitor_buffer_limit: u32,
    #[serde(default = "default_log_retention_days")]
    pub log_retention_days: u32,
    pub logs_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRebuildReport {
    pub total_logs: u64,
    pub rebuilt_logs: u64,
    pub updated_logs: u64,
    pub skipped_logs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockRequest {
    pub master_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevealSecretRequest {
    pub master_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockResponse {
    pub initialized: bool,
    pub unlocked: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevealedSecret {
    pub account_id: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub base_url: Option<String>,
    pub enabled: bool,
    pub note: Option<String>,
    pub has_secret: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
    pub id: Option<String>,
    pub provider: String,
    pub name: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub note: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

fn default_monitor_buffer_limit() -> u32 {
    DEFAULT_MONITOR_BUFFER_LIMIT
}

fn default_log_retention_days() -> u32 {
    DEFAULT_LOG_RETENTION_DAYS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteBinding {
    pub id: String,
    pub provider: String,
    pub model_prefix: Option<String>,
    pub account_id: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteBindingInput {
    pub provider: String,
    pub model_prefix: Option<String>,
    pub account_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogQuery {
    pub provider: Option<String>,
    pub account_id: Option<String>,
    pub session_id: Option<String>,
    pub status_code: Option<u16>,
    pub created_from: Option<String>,
    pub created_to: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DailyStatsQuery {
    pub days: Option<u32>,
    pub utc_offset_minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStatsPoint {
    pub day: String,
    pub request_count: u64,
    pub success_count: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLog {
    pub id: String,
    pub created_at: String,
    pub provider: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub account_id: Option<String>,
    pub method: String,
    pub path: String,
    pub status_code: Option<u16>,
    pub duration_ms: u64,
    pub error_text: Option<String>,
    pub total_tokens: u64,
    pub request_headers: String,
    pub request_body: String,
    pub response_headers: String,
    pub response_body: String,
    pub log_file_path: Option<String>,
    pub streamed: bool,
}

#[derive(Debug, Clone)]
pub struct RequestLogInput {
    pub provider: String,
    pub model: Option<String>,
    pub account_id: Option<String>,
    pub method: String,
    pub path: String,
    pub status_code: Option<u16>,
    pub duration_ms: u64,
    pub error_text: Option<String>,
    pub request_headers: String,
    pub request_body: String,
    pub response_headers: String,
    pub response_body: String,
    pub streamed: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedAccount {
    pub provider: ProviderDefinition,
    pub account: Account,
    pub upstream_base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarExample {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingGuide {
    pub target: String,
    pub title: String,
    pub base_url: String,
    pub env: Vec<EnvVarExample>,
    pub snippet: String,
    pub notes: Vec<String>,
}

pub fn extract_model(body: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    value
        .get("model")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

pub fn extract_session_id(
    request_headers: &str,
    request_body: &str,
    response_headers: &str,
    response_body: &str,
) -> Option<String> {
    extract_session_id_from_headers(request_headers)
        .or_else(|| extract_session_id_from_text(request_body))
        .or_else(|| extract_session_id_from_headers(response_headers))
        .or_else(|| extract_session_id_from_text(response_body))
}

fn extract_session_id_from_headers(headers: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(headers).ok()?;
    let object = value.as_object()?;
    object.iter().find_map(|(name, value)| {
        let normalized = name.trim().to_ascii_lowercase();
        if !matches!(
            normalized.as_str(),
            "x-session-id"
                | "session-id"
                | "anthropic-session-id"
                | "openai-session-id"
                | "x-conversation-id"
                | "conversation-id"
                | "x-thread-id"
                | "thread-id"
        ) {
            return None;
        }
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn extract_session_id_from_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    extract_session_id_from_json(trimmed.as_bytes()).or_else(|| {
        trimmed.lines().find_map(|line| {
            let candidate = line
                .strip_prefix("data:")
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| line.trim());
            if candidate.is_empty() || candidate == "[DONE]" {
                return None;
            }
            extract_session_id_from_json(candidate.as_bytes())
        })
    })
}

fn extract_session_id_from_json(bytes: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    find_session_id(&value)
}

fn find_session_id(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in [
                "session_id",
                "sessionId",
                "resume_session_id",
                "resumeSessionId",
                "conversation_id",
                "conversationId",
                "thread_id",
                "threadId",
            ] {
                if let Some(session_id) = map
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    return Some(session_id.to_owned());
                }
            }
            map.values().find_map(find_session_id)
        }
        serde_json::Value::Array(values) => values.iter().find_map(find_session_id),
        _ => None,
    }
}

pub fn extract_total_tokens(response_body: &str) -> Option<u64> {
    let trimmed = response_body.trim();
    if trimmed.is_empty() {
        return None;
    }

    extract_total_tokens_from_json(trimmed.as_bytes()).or_else(|| {
        trimmed
            .lines()
            .filter_map(|line| {
                let candidate = line
                    .strip_prefix("data:")
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| line.trim());
                if candidate.is_empty() || candidate == "[DONE]" {
                    return None;
                }
                extract_total_tokens_from_json(candidate.as_bytes())
            })
            .max()
    })
}

fn extract_total_tokens_from_json(bytes: &[u8]) -> Option<u64> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    find_total_tokens(&value)
}

fn find_total_tokens(value: &serde_json::Value) -> Option<u64> {
    match value {
        serde_json::Value::Object(map) => {
            let current = total_tokens_from_usage_object(map);
            let nested = map.values().filter_map(find_total_tokens).max();
            current.into_iter().chain(nested).max()
        }
        serde_json::Value::Array(values) => values.iter().filter_map(find_total_tokens).max(),
        _ => None,
    }
}

fn total_tokens_from_usage_object(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Option<u64> {
    let prompt_tokens = token_value(map.get("prompt_tokens"));
    let input_tokens = token_value(map.get("input_tokens"));
    let cached_tokens = map
        .get("input_tokens_details")
        .and_then(serde_json::Value::as_object)
        .and_then(|details| token_value(details.get("cached_tokens")));
    let cache_creation_tokens = token_value(map.get("cache_creation_input_tokens"));
    let cache_read_tokens = token_value(map.get("cache_read_input_tokens"));
    let completion_tokens = token_value(map.get("completion_tokens"));
    let output_tokens = token_value(map.get("output_tokens"));
    let total_tokens = token_value(map.get("total_tokens"));

    let cache_discount = cached_tokens.unwrap_or(0) + cache_read_tokens.unwrap_or(0);
    let input_base = prompt_tokens.or(input_tokens);
    let input = match input_base {
        Some(base) => Some(
            base.saturating_sub(cache_discount) + cache_creation_tokens.unwrap_or(0),
        ),
        None => cache_creation_tokens,
    };
    let output = completion_tokens.or(output_tokens);

    match total_tokens {
        Some(total) => Some(total.saturating_sub(cache_discount)),
        None => match (input, output) {
            (Some(input), Some(output)) => Some(input + output),
            (Some(input), None) => Some(input),
            (None, Some(output)) => Some(output),
            (None, None) => None,
        },
    }
}

fn token_value(value: Option<&serde_json::Value>) -> Option<u64> {
    value.and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().filter(|number| *number >= 0).map(|number| number as u64))
    })
}

#[cfg(test)]
mod tests {
    use super::{extract_session_id, extract_total_tokens};

    #[test]
    fn extracts_session_id_from_request_json() {
        let session_id = extract_session_id(
            "{}",
            r#"{"model":"gpt-5","metadata":{"session_id":"sess_123"}}"#,
            "{}",
            "",
        );
        assert_eq!(session_id.as_deref(), Some("sess_123"));
    }

    #[test]
    fn extracts_session_id_from_headers_and_sse_body() {
        let from_headers = extract_session_id(r#"{"x-session-id":"hdr_456"}"#, "", "{}", "");
        assert_eq!(from_headers.as_deref(), Some("hdr_456"));

        let from_sse = extract_session_id(
            "{}",
            "",
            "{}",
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"conversation_id\":\"conv_789\"}}\n\n",
        );
        assert_eq!(from_sse.as_deref(), Some("conv_789"));
    }

    #[test]
    fn extracts_total_tokens_from_json_usage() {
        let total = extract_total_tokens(
            r#"{"id":"resp_123","usage":{"prompt_tokens":120,"completion_tokens":80,"total_tokens":200}}"#,
        );
        assert_eq!(total, Some(200));
    }

    #[test]
    fn extracts_total_tokens_from_sse_usage() {
        let total = extract_total_tokens(
            "event: message_start\ndata: {\"message\":{\"usage\":{\"input_tokens\":64,\"output_tokens\":16}}}\n\ndata: {\"usage\":{\"total_tokens\":96}}\n\ndata: [DONE]\n",
        );
        assert_eq!(total, Some(96));
    }

    #[test]
    fn extracts_effective_total_tokens_when_cached_tokens_are_reported() {
        let total = extract_total_tokens(
            r#"{"usage":{"input_tokens":192662,"input_tokens_details":{"cached_tokens":192512},"output_tokens":893,"total_tokens":193555}}"#,
        );
        assert_eq!(total, Some(1043));
    }
}
