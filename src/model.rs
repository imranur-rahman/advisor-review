use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceSpan {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdfAnchor {
    pub page: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box: Option<[f32; 4]>,
    pub mapping_quality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TargetAnchor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf: Option<PdfAnchor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentTarget {
    pub id: String,
    #[serde(rename = "type")]
    pub target_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub text: String,
    pub anchor: TargetAnchor,
    #[serde(default)]
    pub facts: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleCheck {
    #[serde(rename = "type", default)]
    pub check_type: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub suggestion: Option<String>,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(flatten)]
    pub parameters: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleDefinition {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub check: RuleCheck,
    #[serde(default)]
    pub active: bool,
}

fn default_severity() -> String {
    "warning".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleCandidate {
    pub id: String,
    pub source: String,
    pub text: String,
    pub reason: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderMetadata {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReviewFinding {
    pub id: String,
    pub rule_id: String,
    pub source_guideline: Option<String>,
    pub status: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    pub target: DocumentTarget,
    pub evidence: String,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReviewIssue {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleConflict {
    pub rule_ids: Vec<String>,
    pub target_scope: String,
    pub resolution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewReport {
    pub schema_version: String,
    pub project: String,
    pub main_tex: String,
    pub pdf: String,
    pub provider: ProviderMetadata,
    pub findings: Vec<ReviewFinding>,
    pub candidates: Vec<RuleCandidate>,
    pub conflicts: Vec<RuleConflict>,
    pub issues: Vec<ReviewIssue>,
}

impl ReviewReport {
    pub fn new(project: String, main_tex: String, pdf: String, provider: ProviderMetadata) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            project,
            main_tex,
            pdf,
            provider,
            findings: vec![],
            candidates: vec![],
            conflicts: vec![],
            issues: vec![],
        }
    }
}
