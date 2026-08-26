use crate::config::ProviderConfig;
use crate::model::{DocumentTarget, ReviewFinding, ReviewIssue, RuleDefinition};
use serde_json::{json, Value};
use std::time::Duration;

pub trait SemanticProvider {
    fn can_handle(&self, rule: &RuleDefinition) -> bool;
    fn review(
        &self,
        rule: &RuleDefinition,
        target: &DocumentTarget,
    ) -> Result<Option<ReviewFinding>, ReviewIssue>;
}

pub struct ConfiguredProvider {
    pub config: ProviderConfig,
}

impl SemanticProvider for ConfiguredProvider {
    fn can_handle(&self, rule: &RuleDefinition) -> bool {
        let capabilities = self.config.metadata().capabilities;
        self.config.has_credentials()
            && (rule.kind != "semantic-vision"
                || capabilities.iter().any(|c| c == "vision-language"))
            && rule.requires.iter().all(|need| capabilities.contains(need))
    }
    fn review(
        &self,
        _rule: &RuleDefinition,
        _target: &DocumentTarget,
    ) -> Result<Option<ReviewFinding>, ReviewIssue> {
        let rule = _rule;
        let target = _target;
        let provider = self
            .config
            .name
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();
        let model = self
            .config
            .model
            .clone()
            .unwrap_or_else(|| "gpt-4o-mini".into());
        let key = self.config.api_key.clone();
        let endpoint = self
            .config
            .endpoint
            .clone()
            .unwrap_or_else(|| match provider.as_str() {
                "ollama" => "http://localhost:11434/v1/chat/completions".into(),
                "openrouter" => "https://openrouter.ai/api/v1/chat/completions".into(),
                _ => "https://api.openai.com/v1/chat/completions".into(),
            });
        let prompt = format!("Evaluate this manuscript target against the rule. Return JSON only with keys status, evidence, explanation, suggestion, confidence. Rule: {}. Target type: {}. Text: {}", rule.description.as_deref().unwrap_or(""), target.target_type, target.text);
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(30))
            .build();
        let response = if provider == "anthropic" {
            let request = json!({"model": model, "max_tokens": 600, "messages": [{"role": "user", "content": prompt}]});
            let mut req = agent.post(self.config.endpoint.as_deref().unwrap_or("https://api.anthropic.com/v1/messages")).set("anthropic-version", "2023-06-01").set("content-type", "application/json");
            if let Some(k) = key.as_deref() { req = req.set("x-api-key", k); }
            req.send_json(request)
        } else {
            let request = json!({"model": model, "messages": [{"role": "user", "content": prompt}], "response_format": {"type": "json_object"}});
            let mut req = agent.post(&endpoint).set("content-type", "application/json");
            if let Some(k) = key.as_deref() { req = req.set("authorization", &format!("Bearer {k}")); }
            req.send_json(request)
        }.map_err(|err| ReviewIssue { kind: "provider_error".into(), message: format!("semantic provider request failed: {err}"), rule_id: Some(rule.id.clone()) })?;
        let body: Value = response.into_json().map_err(|err| ReviewIssue {
            kind: "provider_response".into(),
            message: format!("provider returned invalid JSON: {err}"),
            rule_id: Some(rule.id.clone()),
        })?;
        let content = if provider == "anthropic" {
            body.get("content")
                .and_then(|v| v.get(0))
                .and_then(|v| v.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        } else {
            body.get("choices")
                .and_then(|v| v.get(0))
                .and_then(|v| v.get("message"))
                .and_then(|v| v.get("content"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        let result: Value = serde_json::from_str(&content).map_err(|err| ReviewIssue {
            kind: "provider_response".into(),
            message: format!("provider content was not structured JSON: {err}"),
            rule_id: Some(rule.id.clone()),
        })?;
        let status = result
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("uncertain")
            .to_string();
        if status == "pass" {
            return Ok(None);
        }
        Ok(Some(ReviewFinding {
            id: format!("semantic-{}", rule.id),
            rule_id: rule.id.clone(),
            source_guideline: rule.source.clone(),
            status,
            severity: rule.severity.clone(),
            confidence: result
                .get("confidence")
                .and_then(Value::as_f64)
                .map(|v| v as f32),
            target: target.clone(),
            evidence: result
                .get("evidence")
                .and_then(Value::as_str)
                .unwrap_or("")
                .into(),
            explanation: result
                .get("explanation")
                .and_then(Value::as_str)
                .unwrap_or("")
                .into(),
            suggestion: result
                .get("suggestion")
                .and_then(Value::as_str)
                .map(str::to_string),
        }))
    }
}
