use crate::model::ProviderMetadata;
use std::env;

#[derive(Debug, Clone, Default)]
pub struct ProviderConfig {
    pub name: Option<String>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
}

impl ProviderConfig {
    pub fn from_values(name: Option<String>, model: Option<String>) -> Self {
        let name = name.or_else(|| env::var("ADVISOR_REVIEW_PROVIDER").ok());
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
        Self {
            name,
            model,
            endpoint,
            api_key,
        }
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
        self.name.as_deref() == Some("ollama") || self.api_key.is_some()
    }
}
