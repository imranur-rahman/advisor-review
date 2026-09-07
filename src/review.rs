use crate::guidelines::{RuleRegistry, validate_rule};
use crate::manuscript::{mapped_view, section_title};
use crate::model::{
    DocumentTarget, EvaluationRecord, EvidenceReference, ReviewFinding, ReviewIssue,
    RuleDefinition, ScopeCoverage,
};
use crate::providers::{SemanticProvider, semantic_prompt};
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;

pub const SCOPES: &[&str] = &[
    "document",
    "section",
    "heading",
    "paragraph",
    "sentence",
    "figure",
    "table",
    "table_row",
    "table_cell",
    "equation",
    "code_block",
    "code_line",
    "citation",
    "reference",
    "pdf_page",
    "environment",
];

pub fn canonical_scope(scope: &str) -> &str {
    match scope {
        "manuscript" => "document",
        "code" => "code_block",
        other => other,
    }
}

#[derive(Debug, Clone)]
pub struct ReviewOptions {
    pub scopes: Vec<String>,
    pub max_input_bytes: usize,
}

impl Default for ReviewOptions {
    fn default() -> Self {
        Self {
            scopes: vec![],
            max_input_bytes: 32_768,
        }
    }
}

impl ReviewOptions {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.max_input_bytes >= 1024,
            "max-input-bytes must be at least 1024"
        );
        for scope in &self.scopes {
            anyhow::ensure!(
                SCOPES.contains(&canonical_scope(scope)),
                "unknown scope: {scope}"
            );
        }
        Ok(())
    }
    fn selects(&self, scope: &str) -> bool {
        self.scopes.is_empty()
            || self
                .scopes
                .iter()
                .any(|s| canonical_scope(s) == canonical_scope(scope))
    }
}

#[derive(Default)]
pub struct ReviewRun {
    pub findings: Vec<ReviewFinding>,
    pub issues: Vec<ReviewIssue>,
    pub coverage: BTreeMap<String, ScopeCoverage>,
    pub evaluations: Vec<EvaluationRecord>,
}

pub fn run(
    targets: &[DocumentTarget],
    registry: &RuleRegistry,
    provider: Option<&dyn SemanticProvider>,
) -> (Vec<ReviewFinding>, Vec<ReviewIssue>) {
    let result = run_with_options(targets, registry, provider, &ReviewOptions::default());
    (result.findings, result.issues)
}

pub fn run_with_options(
    targets: &[DocumentTarget],
    registry: &RuleRegistry,
    provider: Option<&dyn SemanticProvider>,
    options: &ReviewOptions,
) -> ReviewRun {
    let mut result = ReviewRun::default();
    if let Err(err) = options.validate() {
        result.issues.push(ReviewIssue {
            kind: "configuration".into(),
            message: err.to_string(),
            rule_id: None,
        });
        return result;
    }
    for scope in SCOPES {
        result.coverage.insert(
            (*scope).into(),
            ScopeCoverage {
                selected: options.selects(scope),
                targets: targets.iter().filter(|t| t.target_type == *scope).count(),
                ..Default::default()
            },
        );
    }
    for rule in &registry.active {
        let scope = canonical_scope(&rule.scope);
        if !options.selects(scope) {
            continue;
        }
        result.coverage.entry(scope.into()).or_default().rules += 1;
        if let Err(err) = validate_rule(rule) {
            result
                .issues
                .push(rule_issue("guideline", rule, err.to_string()));
            record(&mut result, rule, None, "failed", 0);
            continue;
        }
        if matches!(scope, "table_row" | "code_line" | "pdf_page") {
            result.issues.push(rule_issue(
                "skipped",
                rule,
                format!("scope {scope} is not yet extracted"),
            ));
            record(&mut result, rule, None, "skipped", 0);
            continue;
        }
        let scoped: Vec<_> = targets
            .iter()
            .filter(|t| {
                t.target_type == scope
                    && rule
                        .section
                        .as_ref()
                        .is_none_or(|title| section_title(t, targets).as_ref() == Some(title))
            })
            .collect();
        if scoped.is_empty() {
            result.issues.push(rule_issue(
                "not_applicable",
                rule,
                format!("no matching {scope} targets"),
            ));
            record(&mut result, rule, None, "not_applicable", 0);
            continue;
        }
        let semantic = matches!(
            rule.kind.as_str(),
            "semantic-text" | "semantic-vision" | "cross-modal"
        );
        let regex = if rule.check.check_type == "regex" {
            Some(Regex::new(rule.check.pattern.as_deref().unwrap()).expect("validated regex"))
        } else {
            None
        };
        for original in scoped {
            let target = select_view(original, rule);
            let before = result.findings.len();
            let status;
            if semantic {
                if let Some(provider) = provider.filter(|p| p.can_handle(rule)) {
                    let context = context_for(original, targets, rule);
                    let (findings, issues, partial) =
                        semantic_review(provider, rule, &target, &context, options.max_input_bytes);
                    status = if partial {
                        "partial"
                    } else if issues.is_empty() {
                        "completed"
                    } else if issues
                        .iter()
                        .all(|i| i.kind == "budget" || i.kind == "skipped")
                    {
                        "skipped"
                    } else {
                        "failed"
                    };
                    result.findings.extend(findings);
                    result.issues.extend(issues);
                } else {
                    result.issues.push(rule_issue(
                        "skipped",
                        rule,
                        format!(
                            "{}: provider capability or credentials unavailable",
                            target.id
                        ),
                    ));
                    status = "skipped";
                }
            } else {
                match deterministic(rule, &target, before, regex.as_ref()) {
                    Ok(Some(mut finding)) => {
                        if let Some(pattern) = rule
                            .check
                            .pattern
                            .as_deref()
                            .filter(|_| rule.check.check_type == "forbid")
                        {
                            let start = target.text.find(pattern).unwrap();
                            finding.evidence_spans.push(EvidenceReference {
                                target_id: target.id.clone(),
                                quote: pattern.into(),
                                start,
                                end: start + pattern.len(),
                                sources: mapped_view(&target, false)
                                    .slice(start..start + pattern.len())
                                    .spans(),
                            });
                        }
                        result.findings.push(finding);
                        status = "completed";
                    }
                    Ok(None) => status = "completed",
                    Err(message) => {
                        result.issues.push(rule_issue(
                            "skipped",
                            rule,
                            format!("{}: {message}", target.id),
                        ));
                        status = "skipped";
                    }
                }
            }
            let count = result.findings.len() - before;
            record(&mut result, rule, Some(original), status, count);
        }
    }
    for (index, finding) in result.findings.iter_mut().enumerate() {
        finding.id = format!("finding-{index}");
    }
    result
}

fn record(
    result: &mut ReviewRun,
    rule: &RuleDefinition,
    target: Option<&DocumentTarget>,
    status: &str,
    findings: usize,
) {
    let scope = canonical_scope(&rule.scope);
    let coverage = result.coverage.entry(scope.into()).or_default();
    if status != "not_applicable" {
        coverage.evaluations += 1;
    }
    match status {
        "completed" => coverage.completed += 1,
        "failed" => coverage.failed += 1,
        "skipped" => coverage.skipped += 1,
        "partial" => coverage.partial += 1,
        _ => coverage.not_applicable += 1,
    }
    result.evaluations.push(EvaluationRecord {
        rule_id: rule.id.clone(),
        target_id: target.map(|t| t.id.clone()),
        scope: scope.into(),
        status: status.into(),
        findings,
        text_view: rule.text_view.clone(),
    });
}

pub fn select_view(target: &DocumentTarget, rule: &RuleDefinition) -> DocumentTarget {
    let mut selected = target.clone();
    let view = if target.target_type == "section" && !rule.include_subsections {
        if rule.text_view == "source" {
            target.direct_source.clone()
        } else {
            target.direct_prose.clone()
        }
    } else {
        None
    }
    .unwrap_or_else(|| mapped_view(target, rule.text_view == "source"));
    selected.text = view.text;
    selected.text_mappings = view.mappings;
    selected
}

pub fn context_for(
    target: &DocumentTarget,
    targets: &[DocumentTarget],
    rule: &RuleDefinition,
) -> Vec<DocumentTarget> {
    let mut context = vec![];
    let mut node = target;
    let desired = match rule.context.as_str() {
        "paragraph" | "neighbors" => "paragraph",
        "section" => "section",
        _ => return context,
    };
    for _ in 0..targets.len() + 1 {
        if node.target_type == desired {
            break;
        }
        let Some(parent) = targets
            .iter()
            .find(|t| Some(&t.id) == node.parent_id.as_ref())
        else {
            return context;
        };
        node = parent;
    }
    if node.id != target.id {
        context.push(select_view(node, rule));
    }
    if rule.context == "neighbors" {
        let siblings: Vec<_> = targets
            .iter()
            .filter(|t| t.target_type == "paragraph" && t.parent_id == node.parent_id)
            .collect();
        if let Some(index) = siblings.iter().position(|t| t.id == node.id) {
            if index > 0 {
                context.push(select_view(siblings[index - 1], rule));
            }
            if let Some(next) = siblings.get(index + 1) {
                context.push(select_view(next, rule));
            }
        }
    }
    context
}

fn rule_issue(kind: &str, rule: &RuleDefinition, message: String) -> ReviewIssue {
    ReviewIssue {
        kind: kind.into(),
        message,
        rule_id: Some(rule.id.clone()),
    }
}

fn semantic_review(
    provider: &dyn SemanticProvider,
    rule: &RuleDefinition,
    target: &DocumentTarget,
    context: &[DocumentTarget],
    budget: usize,
) -> (Vec<ReviewFinding>, Vec<ReviewIssue>, bool) {
    if semantic_prompt(rule, target, context, "direct").len() <= budget {
        return match provider.review_context(rule, target, context, "direct", budget) {
            Ok(review) => (review.findings, vec![], false),
            Err(issue) => (vec![], vec![issue], false),
        };
    }
    if !matches!(target.target_type.as_str(), "section" | "document") {
        return (
            vec![],
            vec![rule_issue(
                "budget",
                rule,
                format!(
                    "{} plus requested context exceeds {budget} input bytes",
                    target.id
                ),
            )],
            false,
        );
    }
    let mut findings = vec![];
    let mut issues = vec![rule_issue(
        "partial_coverage",
        rule,
        format!(
            "{} requires chunk review; synthesis of notes and excerpts cannot establish complete global coverage",
            target.id
        ),
    )];
    let mut notes = String::new();
    let mut excerpts = vec![];
    let mut excerpt_offsets = BTreeMap::new();
    let target_view = mapped_view(target, false);
    let mut start = 0;
    let mut chunk_index = 0;
    while start < target.text.len() {
        let mut end = (start + budget / 2).min(target.text.len());
        while !target.text.is_char_boundary(end) {
            end -= 1;
        }
        if end < target.text.len() {
            if let Some(boundary) = target.text[start..end].rfind("\n\n") {
                if boundary > 0 {
                    end = start + boundary + 2;
                }
            }
        }
        let mut chunk = DocumentTarget {
            target_type: target.target_type.clone(),
            parent_id: target.parent_id.clone(),
            title: target.title.clone(),
            ..Default::default()
        };
        loop {
            let view = target_view.slice(start..end);
            chunk.text = view.text;
            chunk.text_mappings = view.mappings;
            chunk.id = format!("{}:chunk-{chunk_index}", target.id);
            if semantic_prompt(rule, &chunk, &[], "chunk").len() <= budget {
                break;
            }
            end = start + (end - start) / 2;
            while !target.text.is_char_boundary(end) {
                end -= 1;
            }
            if end <= start {
                issues.push(rule_issue(
                    "budget",
                    rule,
                    "rule instructions alone exceed input budget".into(),
                ));
                return (findings, issues, true);
            }
        }
        match provider.review_context(rule, &chunk, &[], "chunk", budget) {
            Ok(review) => {
                if notes.len() < budget / 4 {
                    let remaining = budget / 4 - notes.len();
                    notes.push_str(&format!(
                        "Chunk {chunk_index}: {}\n",
                        prefix(&review.summary, remaining.min(512))
                    ));
                }
                // Supply an original excerpt for synthesis, independently of model notes.
                let excerpt = mapped_view(&chunk, false).slice(0..prefix(&chunk.text, 256).len());
                let mut evidence_target = chunk.clone();
                evidence_target.text = excerpt.text;
                evidence_target.text_mappings = excerpt.mappings;
                excerpt_offsets.insert(evidence_target.id.clone(), start);
                excerpts.push(evidence_target);
                for mut finding in review.findings {
                    finding.target = target.clone();
                    for evidence in &mut finding.evidence_spans {
                        if evidence.target_id == chunk.id {
                            evidence.target_id = target.id.clone();
                            evidence.start += start;
                            evidence.end += start;
                        }
                    }
                    findings.push(finding);
                }
            }
            Err(err) => issues.push(err),
        }
        start = end;
        chunk_index += 1;
    }
    let mut synthesis = target.clone();
    synthesis.text = notes;
    synthesis.text_mappings.clear();
    while !excerpts.is_empty()
        && semantic_prompt(rule, &synthesis, &excerpts, "synthesis").len() > budget
    {
        excerpts.pop();
    }
    if !synthesis.text.is_empty() && !excerpts.is_empty() {
        match provider.review_context(rule, &synthesis, &excerpts, "synthesis", budget) {
            Ok(review) => {
                for mut finding in review.findings {
                    finding.target = target.clone();
                    // Convert transient chunk references to the original target and offset.
                    for evidence in &mut finding.evidence_spans {
                        if let Some(offset) = excerpt_offsets.get(&evidence.target_id) {
                            evidence.start += offset;
                            evidence.end += offset;
                            evidence.target_id = target.id.clone();
                        }
                    }
                    findings.push(finding);
                }
            }
            Err(err) => issues.push(err),
        }
    } else {
        issues.push(rule_issue(
            "budget",
            rule,
            "no bounded original evidence available for synthesis".into(),
        ));
    }
    (findings, issues, true)
}

fn prefix(text: &str, max: usize) -> &str {
    let mut end = max.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn deterministic(
    rule: &RuleDefinition,
    target: &DocumentTarget,
    seq: usize,
    regex: Option<&Regex>,
) -> Result<Option<ReviewFinding>, String> {
    let check = &rule.check;
    let kind = check.check_type.as_str();
    let mut evidence = target.text.clone();
    let matched;
    match kind {
        "regex" => {
            matched = regex.expect("compiled regex").is_match(&target.text);
        }
        "forbid" => {
            matched = check
                .pattern
                .as_deref()
                .map(|p| target.text.contains(p))
                .unwrap_or(false);
        }
        "contains" => {
            matched = !check
                .pattern
                .as_deref()
                .map(|p| target.text.contains(p))
                .unwrap_or(false);
        }
        "min_pixels" => {
            let min = check.value.as_ref().and_then(Value::as_u64).unwrap_or(0);
            let pixels = target
                .facts
                .get("pixel_width")
                .and_then(Value::as_u64)
                .filter(|v| *v > 0)
                .ok_or("pixel width unavailable")?
                .checked_mul(
                    target
                        .facts
                        .get("pixel_height")
                        .and_then(Value::as_u64)
                        .filter(|v| *v > 0)
                        .ok_or("pixel height unavailable")?,
                )
                .ok_or("pixel area overflow")?;
            matched = pixels > 0 && pixels < min;
            evidence = format!("{} pixels", pixels);
        }
        "min_effective_dpi" => {
            let min = check.value.as_ref().and_then(Value::as_f64).unwrap_or(0.0);
            let dpi = target
                .facts
                .get("effective_dpi")
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite() && *v > 0.0)
                .ok_or("effective DPI unavailable")?;
            matched = dpi < min;
            evidence = format!("{dpi:.1} DPI");
        }
        "environment_exists" => {
            matched = target.facts.contains_key("environment");
        }
        _ => return Err(format!("unsupported check: {kind}")),
    }
    if !matched {
        return Ok(None);
    }
    Ok(Some(ReviewFinding {
        id: format!("finding-{seq}"),
        rule_id: rule.id.clone(),
        source_guideline: rule.source.clone(),
        status: "violation".into(),
        severity: rule.severity.clone(),
        confidence: Some(1.0),
        target: target.clone(),
        evidence,
        explanation: check
            .message
            .clone()
            .or_else(|| rule.description.clone())
            .unwrap_or_else(|| "Rule condition matched".into()),
        suggestion: check.suggestion.clone(),
        evidence_spans: vec![],
    }))
}
