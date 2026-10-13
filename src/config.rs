use crate::model::ProviderMetadata;
use anyhow::{Result, ensure};
use std::env;

#[derive(Clone, Default)]
pub struct ProviderConfig {
    pub name: Option<String>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub openrouter_http_referer: Option<String>,
    pub openrouter_app_title: Option<String>,
    pub claude_code_bin: Option<String>,
    pub codex_bin: Option<String>,
}

impl ProviderConfig {
    pub fn from_values(name: Option<String>, model: Option<String>) -> Self {
        let name = name
            .or_else(|| env::var("ADVISOR_REVIEW_PROVIDER").ok())
            .map(|name| name.trim().to_lowercase());
        let model = model.or_else(|| env::var("ADVISOR_REVIEW_MODEL").ok());
        let endpoint = env::var("ADVISOR_REVIEW_ENDPOINT").ok();
        let api_key = env::var("ADVISOR_REVIEW_API_KEY").ok().or_else(|| {
            name.as_deref()
                .and_then(|p| match p.to_lowercase().as_str() {
                    "openai" => env::var("OPENAI_API_KEY").ok(),
                    "anthropic" => env::var("ANTHROPIC_API_KEY").ok(),
                    "openrouter" => env::var("OPENROUTER_API_KEY").ok(),
                    _ => None,
                })
        });
        let openrouter_http_referer = env::var("OPENROUTER_HTTP_REFERER").ok();
        let openrouter_app_title = env::var("OPENROUTER_APP_TITLE").ok();
        let claude_code_bin = env::var("ADVISOR_REVIEW_CLAUDE_CODE_BIN").ok();
        let codex_bin = env::var("ADVISOR_REVIEW_CODEX_BIN").ok();
        Self {
            name,
            model,
            endpoint,
            api_key,
            openrouter_http_referer,
            openrouter_app_title,
            claude_code_bin,
            codex_bin,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(name) = self.name.as_deref() {
            ensure!(
                matches!(
                    name.to_ascii_lowercase().as_str(),
                    "openai" | "anthropic" | "openrouter" | "ollama" | "claude-code" | "codex-cli"
                ),
                "unknown provider; use openai, anthropic, openrouter, ollama, claude-code, or codex-cli"
            );
            let cli_provider = matches!(
                name.to_ascii_lowercase().as_str(),
                "claude-code" | "codex-cli"
            );
            if !cli_provider {
                ensure!(
                    self.model.as_deref().is_some_and(|m| !m.trim().is_empty()),
                    "a model is required when a provider is selected; set --model or ADVISOR_REVIEW_MODEL"
                );
            } else if let Some(model) = self.model.as_deref() {
                ensure!(!model.trim().is_empty(), "model cannot be empty");
            }
        } else {
            ensure!(self.model.is_none(), "--model requires a provider");
        }
        if let Some(endpoint) = self.endpoint.as_deref() {
            ensure!(
                endpoint.starts_with("https://") || endpoint.starts_with("http://"),
                "provider endpoint must use http:// or https://"
            );
        }
        Ok(())
    }

    pub fn metadata(&self) -> ProviderMetadata {
        let capabilities = self
            .name
            .as_deref()
            .map(|name| match name.to_lowercase().as_str() {
                "openai" | "anthropic" | "openrouter" => {
                    vec!["semantic-text".into(), "structured-output".into()]
                }
                "ollama" => vec!["semantic-text".into()],
                "claude-code" | "codex-cli" => vec!["semantic-text".into()],
                _ => vec![],
            })
            .unwrap_or_default();
        ProviderMetadata {
            provider: self.name.clone(),
            model: self.model.clone(),
            capabilities,
        }
    }

    pub fn has_credentials(&self) -> bool {
        self.name
            .as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case("ollama"))
            || self.name.as_deref().is_some_and(|n| {
                n.eq_ignore_ascii_case("claude-code") || n.eq_ignore_ascii_case("codex-cli")
            })
            || self
                .api_key
                .as_deref()
                .is_some_and(|key| !key.trim().is_empty())
    }
}

impl std::fmt::Debug for ProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderConfig")
            .field("name", &self.name)
            .field("model", &self.model)
            .field("endpoint", &self.endpoint.as_ref().map(|_| "[configured]"))
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("openrouter_http_referer", &self.openrouter_http_referer)
            .field("openrouter_app_title", &self.openrouter_app_title)
            .field("claude_code_bin", &self.claude_code_bin)
            .field("codex_bin", &self.codex_bin)
            .finish()
    }
}
