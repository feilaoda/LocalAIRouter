use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::error::{LocalOpenRouterError, Result};

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
    type Err = LocalOpenRouterError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "openai" => Ok(Self::OpenAi),
            "anthropic" => Ok(Self::Anthropic),
            "generic" => Ok(Self::Generic),
            other => Err(LocalOpenRouterError::Validation(format!(
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
    pub limit: Option<u32>,
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

#[cfg(test)]
mod tests {
    use super::extract_session_id;

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
}
