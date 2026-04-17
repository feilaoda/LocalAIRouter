use crate::error::{LocalAIRouterError, Result};
use crate::models::{EnvVarExample, OnboardingGuide, ProviderDefinition};

pub const DEFAULT_PORT: u16 = 7321;

pub fn guide_for_target(
    target: &str,
    port: u16,
    provider: &ProviderDefinition,
) -> Result<OnboardingGuide> {
    let port = if port == 0 { DEFAULT_PORT } else { port };
    let base_url = format!("http://127.0.0.1:{port}/{}", provider.proxy_path);
    match target {
        "codex" => Ok(OnboardingGuide {
            target: "codex".into(),
            title: format!("Codex via {}", provider.display_name),
            base_url: base_url.clone(),
            env: vec![
                EnvVarExample {
                    key: "OPENAI_BASE_URL".into(),
                    value: base_url.clone(),
                },
                EnvVarExample {
                    key: "OPENAI_API_KEY".into(),
                    value: "localairouter-managed".into(),
                },
            ],
            snippet: format!(
                "export OPENAI_BASE_URL=\"{base_url}\"\nexport OPENAI_API_KEY=\"localairouter-managed\""
            ),
            notes: vec![
                format!(
                    "Point any OpenAI-compatible CLI to the local /{} namespace.",
                    provider.proxy_path
                ),
                "The API key is a placeholder for the client; LocalAIRouter injects the real upstream key."
                    .into(),
            ],
        }),
        "claude-code" => Ok(OnboardingGuide {
            target: "claude-code".into(),
            title: format!("Claude Code via {}", provider.display_name),
            base_url: base_url.clone(),
            env: vec![
                EnvVarExample {
                    key: "ANTHROPIC_BASE_URL".into(),
                    value: base_url.clone(),
                },
                EnvVarExample {
                    key: "ANTHROPIC_API_KEY".into(),
                    value: "localairouter-managed".into(),
                },
            ],
            snippet: format!(
                "export ANTHROPIC_BASE_URL=\"{base_url}\"\nexport ANTHROPIC_API_KEY=\"localairouter-managed\""
            ),
            notes: vec![
                format!(
                    "Point Claude Code to the local /{} namespace.",
                    provider.proxy_path
                ),
                "Keep the client configured explicitly; LocalAIRouter does not modify system proxy settings."
                    .into(),
            ],
        }),
        other => Err(LocalAIRouterError::NotFound(format!(
            "unsupported onboarding target `{other}`"
        ))),
    }
}
