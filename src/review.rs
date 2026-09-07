use crate::guidelines::{RuleRegistry, validate_rule};
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
        if let Err(err) = validate_rule(rule) {
            issues.push(rule_issue(
                "guideline",
                rule,
                format!("invalid rule: {err}"),
            ));
            continue;
        }
        if matches!(
            rule.scope.as_str(),
            "sentence" | "table_row" | "code_line" | "pdf_page"
        ) {
            issues.push(rule_issue(
                "skipped",
                rule,
                format!("scope {} is not yet extracted", rule.scope),
            ));
            continue;
        }
        let scoped: Vec<_> = targets
            .iter()
            .filter(|t| in_scope(&rule.scope, &t.target_type))
            .collect();
        if scoped.is_empty() {
            issues.push(rule_issue(
                "not_applicable",
                rule,
                format!("no {} targets were found", rule.scope),
            ));
            continue;
        }
        let semantic = matches!(
            rule.kind.as_str(),
            "semantic-text" | "semantic-vision" | "cross-modal"
        );
        if semantic && !provider.is_some_and(|p| p.can_handle(rule)) {
            issues.push(rule_issue(
                "skipped",
                rule,
                "provider capability or credentials unavailable".into(),
            ));
            continue;
        }
        let regex = if rule.check.check_type == "regex" {
            Some(Regex::new(rule.check.pattern.as_deref().unwrap()).expect("validated regex"))
        } else {
            None
        };
        for target in scoped {
            if semantic {
                match provider {
                    Some(p) if p.can_handle(rule) => match p.review(rule, target) {
                        Ok(Some(mut f)) => {
                            f.id = format!("finding-{seq}");
                            seq += 1;
                            findings.push(f);
                        }
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
            } else {
                match deterministic(rule, target, seq, regex.as_ref()) {
                    Ok(Some(f)) => {
                        seq += 1;
                        findings.push(f);
                    }
                    Ok(None) => {}
                    Err(message) => issues.push(rule_issue(
                        "skipped",
                        rule,
                        format!("{}: {message}", target.id),
                    )),
                }
            }
        }
    }
    (findings, issues)
}

fn in_scope(scope: &str, target: &str) -> bool {
    scope == target || (scope == "code" && target == "code_block")
}

fn rule_issue(kind: &str, rule: &RuleDefinition, message: String) -> ReviewIssue {
    ReviewIssue {
        kind: kind.into(),
        message,
        rule_id: Some(rule.id.clone()),
    }
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
    }))
}
