//! Runs a rule's example tests through the normal review path.

use crate::guidelines::RuleRegistry;
use crate::model::{DocumentTarget, RuleDefinition};
use crate::providers::SemanticProvider;
use crate::review::{self, ReviewOptions, canonical_scope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    Failed(String),
    Skipped(String),
}

#[derive(Debug, Clone)]
pub struct ExampleResult {
    pub rule_id: String,
    /// `should_fire[i]` or `should_pass[i]`.
    pub example: String,
    pub text: String,
    pub outcome: Outcome,
}

impl ExampleResult {
    pub fn label(&self) -> &'static str {
        match self.outcome {
            Outcome::Passed => "PASS",
            Outcome::Failed(_) => "FAIL",
            Outcome::Skipped(_) => "SKIP",
        }
    }
}

pub fn is_semantic(rule: &RuleDefinition) -> bool {
    matches!(
        rule.kind.as_str(),
        "semantic-text" | "semantic-vision" | "cross-modal"
    )
}

/// Runs every example of one rule. Each example is one synthetic target at the
/// rule's scope. Section selectors and context do not apply to a lone example.
pub fn run(
    rule: &RuleDefinition,
    provider: Option<&dyn SemanticProvider>,
    max_input_bytes: usize,
) -> Vec<ExampleResult> {
    let Some(tests) = &rule.tests else {
        return vec![];
    };
    let mut rule = rule.clone();
    rule.section = None;
    rule.context = "none".into();
    rule.status = None;
    let registry = RuleRegistry {
        active: vec![rule.clone()],
        ..Default::default()
    };
    let options = ReviewOptions {
        scopes: vec![],
        max_input_bytes,
    };
    let examples = tests
        .should_fire
        .iter()
        .enumerate()
        .map(|(i, t)| (format!("should_fire[{i}]"), t, true))
        .chain(
            tests
                .should_pass
                .iter()
                .enumerate()
                .map(|(i, t)| (format!("should_pass[{i}]"), t, false)),
        );
    let mut results = vec![];
    for (label, text, expect_fire) in examples {
        let target = DocumentTarget {
            id: format!("example-{}", label.replace(['[', ']'], "")),
            target_type: canonical_scope(&rule.scope).into(),
            text: text.clone(),
            raw_text: text.clone(),
            ..Default::default()
        };
        let run = review::run_with_options(&[target], &registry, provider, &options);
        let status = run
            .evaluations
            .iter()
            .find(|e| e.target_id.is_some())
            .map(|e| e.status.as_str())
            .unwrap_or("not_applicable");
        let detail = || {
            run.issues
                .iter()
                .map(|i| i.message.clone())
                .collect::<Vec<_>>()
                .join("; ")
        };
        let outcome = match status {
            "completed" => {
                // Semantic rules fire only on violation or concern findings.
                let fired = run.findings.iter().any(|f| {
                    !is_semantic(&rule) || matches!(f.status.as_str(), "violation" | "concern")
                });
                if fired == expect_fire {
                    Outcome::Passed
                } else if expect_fire {
                    Outcome::Failed("rule did not fire".into())
                } else {
                    Outcome::Failed(format!(
                        "rule fired: {}",
                        run.findings
                            .iter()
                            .map(|f| format!("{} ({})", f.evidence, f.status))
                            .collect::<Vec<_>>()
                            .join("; ")
                    ))
                }
            }
            "skipped" => Outcome::Skipped(detail()),
            "not_applicable" => Outcome::Skipped(format!("scope {} cannot run", rule.scope)),
            _ => Outcome::Failed(detail()),
        };
        results.push(ExampleResult {
            rule_id: rule.id.clone(),
            example: label,
            text: text.clone(),
            outcome,
        });
    }
    results
}
