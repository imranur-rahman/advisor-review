mod common;
use advisor_review::{
    guidelines::{self, RuleRegistry},
    latex,
    manuscript::mapped_view,
    model::{
        DocumentTarget, EvidenceReference, ReviewFinding, ReviewIssue, ReviewReport, RuleDefinition,
    },
    providers::{SemanticProvider, SemanticReview, semantic_prompt},
    review::{self, ReviewOptions},
};
use std::{cell::RefCell, fs};

fn fixture(text: &str, rules: &str) -> (Vec<DocumentTarget>, RuleRegistry) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.tex"), text).unwrap();
    fs::write(dir.path().join("rules.yaml"), rules).unwrap();
    (
        latex::parse_project(&dir.path().join("main.tex"), dir.path()).unwrap(),
        guidelines::load(dir.path()).unwrap(),
    )
}

#[test]
fn four_scopes_execute_independently_and_filters_are_visible() {
    let (targets, rules) = fixture(
        "\\section{Methods}\nVery weak. Very vague.",
        "rules:\n - {id: s, scope: sentence, kind: text, check: {type: forbid, pattern: Very}}\n - {id: p, scope: paragraph, kind: text, check: {type: forbid, pattern: Very}}\n - {id: c, scope: section, kind: text, check: {type: forbid, pattern: Very}}\n - {id: d, scope: manuscript, kind: text, check: {type: forbid, pattern: Very}}",
    );
    let all = review::run_with_options(&targets, &rules, None, &ReviewOptions::default());
    assert_eq!(all.findings.len(), 5);
    assert_eq!(all.coverage["sentence"].completed, 2);
    for scope in ["paragraph", "section", "document"] {
        assert_eq!(all.coverage[scope].completed, 1);
    }
    let selected = review::run_with_options(
        &targets,
        &rules,
        None,
        &ReviewOptions {
            scopes: vec!["sentence".into()],
            ..Default::default()
        },
    );
    assert_eq!(selected.findings.len(), 2);
    assert!(selected.issues.is_empty());
    assert!(!selected.coverage["document"].selected);
    assert_eq!(selected.coverage["document"].evaluations, 0);
}

#[test]
fn views_section_selector_and_subsection_policy_are_respected() {
    let (targets, rules) = fixture(
        "\\section{Methods}\n\\emph{Precise} measurements.\n\\subsection{Detail}\nVague detail.\n\\section{Results}\nVague result.",
        "rules:\n - {id: direct, scope: section, section: Methods, include_subsections: false, kind: text, check: {type: forbid, pattern: Vague}}\n - {id: subtree, scope: section, section: Methods, kind: text, check: {type: forbid, pattern: Vague}}\n - {id: raw, scope: paragraph, text_view: source, kind: text, check: {type: forbid, pattern: '\\emph'}}\n - {id: prose, scope: paragraph, kind: text, check: {type: forbid, pattern: '\\emph'}}\n - {id: heading, scope: heading, section: Methods, kind: text, check: {type: contains, pattern: Methods}}",
    );
    assert!(rules.issues.is_empty(), "{:?}", rules.issues);
    let result = review::run_with_options(&targets, &rules, None, &ReviewOptions::default());
    assert_eq!(
        result
            .findings
            .iter()
            .map(|f| f.rule_id.as_str())
            .collect::<Vec<_>>(),
        vec!["raw", "subtree"]
    );
    assert_eq!(result.coverage["heading"].completed, 1);
}

#[derive(Default)]
struct Fake {
    calls: RefCell<Vec<(String, String, Vec<String>)>>,
    fail_sentence: bool,
    fail_second_chunk: bool,
}

impl SemanticProvider for Fake {
    fn can_handle(&self, _: &RuleDefinition) -> bool {
        true
    }
    fn review(
        &self,
        _: &RuleDefinition,
        _: &DocumentTarget,
    ) -> Result<Option<ReviewFinding>, ReviewIssue> {
        panic!("contextual path expected")
    }
    fn review_context(
        &self,
        rule: &RuleDefinition,
        target: &DocumentTarget,
        context: &[DocumentTarget],
        phase: &str,
        budget: usize,
    ) -> Result<SemanticReview, ReviewIssue> {
        assert!(semantic_prompt(rule, target, context, phase).len() <= budget);
        self.calls.borrow_mut().push((
            target.target_type.clone(),
            phase.into(),
            context.iter().map(|c| c.id.clone()).collect(),
        ));
        if (self.fail_sentence && target.target_type == "sentence")
            || (self.fail_second_chunk && phase == "chunk" && target.id.ends_with(":chunk-1"))
        {
            return Err(ReviewIssue {
                kind: "provider_error".into(),
                message: "test failure".into(),
                ..Default::default()
            });
        }
        if phase == "synthesis" || (self.fail_second_chunk && phase == "chunk") {
            let excerpt = if phase == "synthesis" {
                &context[0]
            } else {
                target
            };
            let quote = excerpt.text.chars().take(12).collect::<String>();
            return Ok(SemanticReview {
                summary: "Synthesis complete on excerpts.".into(),
                findings: vec![ReviewFinding {
                    target: target.clone(),
                    rule_id: rule.id.clone(),
                    status: "concern".into(),
                    evidence: quote.clone(),
                    evidence_spans: vec![EvidenceReference {
                        target_id: excerpt.id.clone(),
                        start: 0,
                        end: quote.len(),
                        sources: mapped_view(excerpt, false).slice(0..quote.len()).spans(),
                        quote,
                    }],
                    ..Default::default()
                }],
            });
        }
        Ok(SemanticReview {
            summary: "Rule checked in supplied text.".into(),
            findings: vec![],
        })
    }
}

#[test]
fn context_is_separate_and_failure_at_one_level_does_not_block_others() {
    let (targets, rules) = fixture(
        "\\section{Methods}\nIt works. This is clear.\n\nNext paragraph.",
        "rules:\n - {id: s, scope: sentence, kind: semantic-text, context: paragraph, description: Check clarity, check: {type: semantic}}\n - {id: d, scope: document, kind: semantic-text, description: Check structure, check: {type: semantic}}",
    );
    let provider = Fake {
        fail_sentence: true,
        ..Default::default()
    };
    let run =
        review::run_with_options(&targets, &rules, Some(&provider), &ReviewOptions::default());
    assert_eq!(run.coverage["sentence"].failed, 3);
    assert_eq!(run.coverage["document"].completed, 1);
    for (scope, _, context) in provider.calls.borrow().iter() {
        if scope == "sentence" {
            assert_eq!(context.len(), 1);
            assert!(context[0].starts_with("paragraph-"));
        }
    }
}

#[test]
fn oversized_manuscript_is_chunked_synthesized_and_marked_partial() {
    let text = format!(
        "\\section{{Methods}}\n{}",
        "Calibrated instruments produced accurate results.\n\n".repeat(100)
    );
    let (targets, rules) = fixture(
        &text,
        "id: global\nscope: document\nkind: semantic-text\ndescription: Check consistency.\ncheck: {type: semantic}",
    );
    let provider = Fake::default();
    let run = review::run_with_options(
        &targets,
        &rules,
        Some(&provider),
        &ReviewOptions {
            max_input_bytes: 2048,
            ..Default::default()
        },
    );
    assert_eq!(run.coverage["document"].partial, 1);
    assert!(run.issues.iter().any(|i| i.kind == "partial_coverage"));
    assert!(
        provider
            .calls
            .borrow()
            .iter()
            .filter(|(_, phase, _)| phase == "chunk")
            .count()
            > 1
    );
    assert!(
        provider
            .calls
            .borrow()
            .iter()
            .any(|(_, phase, _)| phase == "synthesis")
    );
    assert_eq!(run.findings[0].target.id, "document");
    let evidence = &run.findings[0].evidence_spans[0];
    assert_eq!(evidence.target_id, "document");
    assert_eq!(
        &run.findings[0].target.text[evidence.start..evidence.end],
        evidence.quote
    );
    assert!(!evidence.sources.is_empty());
}

#[test]
fn chunk_failure_preserves_findings_and_original_evidence_offsets() {
    let (targets, rules) = fixture(
        &"Unicode café measurements are precise.\n\n".repeat(150),
        "id: global\nscope: document\nkind: semantic-text\ndescription: Check consistency.\ncheck: {type: semantic}",
    );
    let provider = Fake {
        fail_second_chunk: true,
        ..Default::default()
    };
    let run = review::run_with_options(
        &targets,
        &rules,
        Some(&provider),
        &ReviewOptions {
            max_input_bytes: 2048,
            ..Default::default()
        },
    );
    assert_eq!(run.coverage["document"].partial, 1);
    assert_eq!(run.coverage["document"].completed, 0);
    assert!(
        run.issues
            .iter()
            .any(|issue| issue.kind == "provider_error")
    );
    assert!(run.findings.len() > 2);
    assert!(run.findings.iter().any(|f| f.evidence_spans[0].start > 0));
    for finding in &run.findings {
        assert_eq!(finding.target.id, "document");
        for evidence in &finding.evidence_spans {
            assert_eq!(evidence.target_id, "document");
            assert_eq!(
                &finding.target.text[evidence.start..evidence.end],
                evidence.quote
            );
            assert!(!evidence.sources.is_empty());
        }
    }
    assert!(
        provider
            .calls
            .borrow()
            .iter()
            .any(|(_, phase, _)| phase == "synthesis")
    );
}

#[test]
fn oversized_local_context_is_skipped_without_a_provider_request() {
    let (targets, rules) = fixture(
        &"A complete sentence. ".repeat(200),
        "id: local\nscope: sentence\nkind: semantic-text\ncontext: paragraph\ndescription: Check precision.\ncheck: {type: semantic}",
    );
    let provider = Fake::default();
    let run = review::run_with_options(
        &targets,
        &rules,
        Some(&provider),
        &ReviewOptions {
            max_input_bytes: 2048,
            ..Default::default()
        },
    );
    assert_eq!(run.coverage["sentence"].skipped, 200);
    assert_eq!(run.coverage["sentence"].completed, 0);
    assert!(run.issues.iter().all(|issue| issue.kind == "budget"));
    assert!(provider.calls.borrow().is_empty());
}

#[test]
fn programmatic_rule_defaults_match_yaml_defaults() {
    let parsed: RuleDefinition = serde_yaml::from_str("id: defaults").unwrap();
    let defaults = RuleDefinition {
        id: "defaults".into(),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(parsed).unwrap(),
        serde_json::to_value(defaults).unwrap()
    );
}

#[test]
fn cli_exports_hierarchy_coverage_and_scope_groups() {
    let root = tempfile::tempdir().unwrap();
    common::fixture(root.path());
    fs::write(root.path().join("guidelines/rules.yaml"), "rules:\n - {id: s, scope: sentence, kind: text, check: {type: forbid, pattern: very}}\n - {id: p, scope: paragraph, kind: text, check: {type: forbid, pattern: very}}\n - {id: c, scope: section, kind: text, check: {type: forbid, pattern: very}}\n - {id: d, scope: document, kind: text, check: {type: forbid, pattern: very}}").unwrap();
    let output = common::cli(root.path()).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: ReviewReport =
        serde_json::from_slice(&fs::read(root.path().join("review/findings.json")).unwrap())
            .unwrap();
    assert_eq!(report.schema_version, "2.0");
    assert_eq!(report.findings.len(), 4);
    assert!(
        report
            .targets
            .iter()
            .any(|t| t.target_type == "sentence" && t.parent_id.is_some())
    );
    let markdown = fs::read_to_string(root.path().join("review/findings.md")).unwrap();
    for scope in ["sentence", "paragraph", "section", "document"] {
        assert!(markdown.contains(&format!("## {scope} —")));
    }
    assert!(markdown.contains("Coverage by scope"));
    let output = common::cli(root.path())
        .args(["--scopes", "sentence,paragraph"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: ReviewReport =
        serde_json::from_slice(&fs::read(root.path().join("review/findings.json")).unwrap())
            .unwrap();
    assert_eq!(report.findings.len(), 2);
    assert!(!report.coverage["document"].selected);
    assert_eq!(
        common::cli(root.path())
            .args(["--scopes", "typo"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
}
