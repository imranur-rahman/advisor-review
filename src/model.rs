use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: &str = "2.0";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SourceSpan {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_byte: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_byte: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_column: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextMapping {
    pub start: usize,
    pub end: usize,
    pub source: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MappedText {
    pub text: String,
    pub mappings: Vec<TextMapping>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceSpan>,
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
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub order: usize,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub depth: Option<usize>,
    #[serde(default)]
    pub raw_text: String,
    #[serde(default)]
    pub text_mappings: Vec<TextMapping>,
    #[serde(default)]
    pub raw_mappings: Vec<TextMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_prose: Option<MappedText>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_source: Option<MappedText>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(default = "default_text_view")]
    pub text_view: String,
    #[serde(default = "default_context")]
    pub context: String,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default = "default_true")]
    pub include_subsections: bool,
}

impl Default for RuleDefinition {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: None,
            scope: String::new(),
            kind: String::new(),
            severity: default_severity(),
            priority: 0,
            description: None,
            source: None,
            requires: vec![],
            check: RuleCheck::default(),
            active: false,
            text_view: default_text_view(),
            context: default_context(),
            section: None,
            include_subsections: true,
        }
    }
}

fn default_text_view() -> String {
    "prose".into()
}
fn default_context() -> String {
    "none".into()
}
fn default_true() -> bool {
    true
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
    #[serde(default)]
    pub evidence_spans: Vec<EvidenceReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvidenceReference {
    pub target_id: String,
    pub quote: String,
    pub start: usize,
    pub end: usize,
    pub sources: Vec<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScopeCoverage {
    pub selected: bool,
    pub targets: usize,
    pub rules: usize,
    pub evaluations: usize,
    pub completed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub partial: usize,
    pub not_applicable: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationRecord {
    pub rule_id: String,
    pub target_id: Option<String>,
    pub scope: String,
    pub status: String,
    pub findings: usize,
    pub text_view: String,
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
    #[serde(default)]
    pub targets: Vec<DocumentTarget>,
    #[serde(default)]
    pub coverage: BTreeMap<String, ScopeCoverage>,
    #[serde(default)]
    pub evaluations: Vec<EvaluationRecord>,
}

impl ReviewReport {
    pub fn is_incomplete(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| !matches!(issue.kind.as_str(), "pdf_mapping" | "not_applicable"))
    }

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
            targets: vec![],
            coverage: BTreeMap::new(),
            evaluations: vec![],
        }
    }
}
