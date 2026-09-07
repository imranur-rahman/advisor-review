use advisor_review::{
    guidelines, latex,
    model::{DocumentTarget, ReviewFinding, ReviewIssue, RuleDefinition},
    providers::{SemanticProvider, SemanticReview},
    review::{self, ReviewOptions},
};
use std::{cell::RefCell, fs, path::Path};

fn registry() -> guidelines::RuleRegistry {
    guidelines::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("guidelines")).unwrap()
}

#[derive(Default)]
struct RecordingProvider(RefCell<Vec<(String, Vec<String>)>>);

impl SemanticProvider for RecordingProvider {
    fn can_handle(&self, _: &RuleDefinition) -> bool {
        true
    }

    fn review(
        &self,
        _: &RuleDefinition,
        _: &DocumentTarget,
    ) -> Result<Option<ReviewFinding>, ReviewIssue> {
        panic!("expected contextual review")
    }

    fn review_context(
        &self,
        rule: &RuleDefinition,
        _: &DocumentTarget,
        context: &[DocumentTarget],
        _: &str,
        _: usize,
    ) -> Result<SemanticReview, ReviewIssue> {
        self.0.borrow_mut().push((
            rule.id.clone(),
            context.iter().map(|t| t.target_type.clone()).collect(),
        ));
        Ok(SemanticReview {
            findings: vec![],
            summary: "Mock review completed.".into(),
        })
    }
}

#[test]
fn writing_pack_loads_explicit_semantic_rules_and_keeps_checklist_as_candidate() {
    let registry = registry();
    assert!(registry.issues.is_empty(), "{:?}", registry.issues);
    assert!(registry.conflicts.is_empty());
    assert_eq!(registry.active.len(), 3);
    assert_eq!(registry.candidates.len(), 1);
    assert!(registry.candidates[0].source.ends_with("writing-donts.md"));
    assert!(
        registry.candidates[0]
            .text
            .contains("https://www.ignorance.ai/p/the-field-guide-to-ai-slop")
    );
    for rule in &registry.active {
        assert!(rule.active);
        assert_eq!(rule.kind, "semantic-text");
        assert_eq!(rule.check.check_type, "semantic");
        assert_eq!(rule.severity, "suggestion");
        assert!(
            rule.source
                .as_ref()
                .unwrap()
                .ends_with("writing-donts.yaml")
        );
        assert!(
            rule.description
                .as_ref()
                .unwrap()
                .contains("never AI authorship")
        );
        assert!(rule.check.pattern.is_none());
    }
}

#[test]
fn writing_pack_runs_at_intended_levels_with_context_and_reports_missing_provider() {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("main.tex");
    fs::write(
        &main,
        "\\section{Methods}\nFirst measurement.\n\nSecond measurement.",
    )
    .unwrap();
    let targets = latex::parse_project(&main, dir.path()).unwrap();
    let registry = registry();
    let provider = RecordingProvider::default();
    let run = review::run_with_options(
        &targets,
        &registry,
        Some(&provider),
        &ReviewOptions::default(),
    );
    assert!(run.issues.is_empty());
    assert_eq!(run.coverage["sentence"].completed, 2);
    assert_eq!(run.coverage["paragraph"].completed, 2);
    assert_eq!(run.coverage["section"].completed, 1);
    assert_eq!(run.coverage["document"].evaluations, 0);
    for (id, context) in provider.0.borrow().iter() {
        match id.as_str() {
            "writing.sentence.metaphor" | "writing.paragraph.filler" => {
                assert_eq!(context, &["paragraph"])
            }
            "writing.section.repetition" => assert!(context.is_empty()),
            _ => panic!("unexpected rule: {id}"),
        }
    }
    let run = review::run_with_options(
        &targets,
        &registry,
        None,
        &ReviewOptions {
            scopes: vec!["sentence".into()],
            ..Default::default()
        },
    );
    assert_eq!(run.coverage["sentence"].skipped, 2);
    assert_eq!(run.coverage["paragraph"].evaluations, 0);
    assert_eq!(run.issues.len(), 2);
    assert!(run.issues.iter().all(|issue| issue.kind == "skipped"));
    assert!(run.findings.is_empty());
}
