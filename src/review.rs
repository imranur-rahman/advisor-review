use crate::guidelines::RuleRegistry;
use crate::model::{DocumentTarget, ReviewFinding, ReviewIssue, RuleDefinition};
use crate::providers::SemanticProvider;
use regex::Regex;
use serde_json::Value;

pub fn run(
    targets: &[DocumentTarget],
    registry: &RuleRegistry,
    provider: Option<&dyn SemanticProvider>,
) -> (Vec<ReviewFinding>, Vec<ReviewIssue>) {
    let mut findings = vec![];
    let mut issues = vec![];
    let mut seq = 0;
    for rule in &registry.active {
        for target in targets
            .iter()
            .filter(|t| in_scope(&rule.scope, &t.target_type))
        {
            if rule.kind == "semantic-text"
                || rule.kind == "semantic-vision"
                || rule.kind == "cross-modal"
            {
                match provider {
                    Some(p) if p.can_handle(rule) => match p.review(rule, target) {
                        Ok(Some(f)) => findings.push(f),
                        Ok(None) => {}
                        Err(mut e) => {
                            e.rule_id = Some(rule.id.clone());
                            issues.push(e);
                        }
                    },
                    _ => issues.push(ReviewIssue {
                        kind: "skipped".into(),
                        message: format!(
                            "provider capability or credentials unavailable for rule {}",
                            rule.id
                        ),
                        rule_id: Some(rule.id.clone()),
                    }),
                }
            } else if let Some(f) = deterministic(rule, target, seq) {
                seq += 1;
                findings.push(f);
            }
        }
    }
    (findings, issues)
}

fn in_scope(scope: &str, target: &str) -> bool {
    scope == "document" || scope == target || (scope == "code" && target == "code_block")
}

fn deterministic(
    rule: &RuleDefinition,
    target: &DocumentTarget,
    seq: usize,
) -> Option<ReviewFinding> {
    let check = &rule.check;
    let kind = check.check_type.as_str();
    let mut evidence = target.text.clone();
    let matched;
    match kind {
        "regex" => {
            let pattern = check.pattern.as_deref().unwrap_or_default();
            matched = Regex::new(pattern)
                .ok()
                .map(|r| r.is_match(&target.text))
                .unwrap_or(false);
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
                .unwrap_or(0)
                * target
                    .facts
                    .get("pixel_height")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
            matched = pixels > 0 && pixels < min;
            evidence = format!("{} pixels", pixels);
        }
        "min_effective_dpi" => {
            let min = check.value.as_ref().and_then(Value::as_f64).unwrap_or(0.0);
            let dpi = target.facts.get("effective_dpi").and_then(Value::as_f64);
            matched = dpi.map(|v| v < min).unwrap_or(false);
            evidence = dpi
                .map(|v| format!("{v:.1} DPI"))
                .unwrap_or_else(|| "effective DPI unavailable".into());
        }
        "environment_exists" => {
            matched = target.target_type == "environment";
        }
        _ => return None,
    }
    if !matched {
        return None;
    }
    Some(ReviewFinding {
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
    })
}
