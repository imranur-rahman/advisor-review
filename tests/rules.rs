use advisor_review::{
    guidelines,
    model::{DocumentTarget, ReviewReport},
    report, review,
};
use std::fs;

fn load(yaml: &str) -> guidelines::RuleRegistry {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("rules.yaml"), yaml).unwrap();
    guidelines::load(root.path()).unwrap()
}

#[test]
fn invalid_definitions_never_activate() {
    for fragment in [
        "scope: typo\nkind: text\ncheck: {type: forbid, pattern: very}",
        "scope: paragraph\nkind: typo\ncheck: {type: forbid, pattern: very}",
        "scope: paragraph\nkind: text\ncheck: {type: frobid, pattern: very}",
        "scope: paragraph\nkind: text\ncheck: {type: regex, pattern: '['}",
        "scope: paragraph\nkind: text\ncheck: {type: forbid}",
        "scope: paragraph\nkind: text\ncheck: {type: contains, pattern: ''}",
        "scope: figure\nkind: asset\ncheck: {type: min_pixels, value: -1}",
        "scope: figure\nkind: asset\ncheck: {type: min_pixels, value: 0.5}",
        "scope: figure\nkind: asset\ncheck: {type: min_effective_dpi, value: 0}",
        "scope: paragraph\nkind: text\ncheck: {type: forbid, pattern: very, typo: true}",
        "scope: paragraph\nkind: semantic-text\ncheck: {type: semantic}",
        "scope: paragraph\nkind: text\nseverity: typo\ncheck: {type: forbid, pattern: very}",
    ] {
        let registry = load(&format!("id: invalid\n{fragment}\n"));
        assert!(registry.active.is_empty(), "{fragment}");
        assert_eq!(registry.issues.len(), 1, "{fragment}");
    }
}

#[test]
fn malformed_items_do_not_discard_valid_siblings() {
    let registry = load(
        "rules:\n  - {scope: paragraph}\n  - {id: valid, scope: paragraph, kind: text, check: {type: forbid, pattern: very}}\n",
    );
    assert_eq!(registry.active.len(), 1);
    assert_eq!(registry.active[0].id, "valid");
    assert_eq!(registry.issues.len(), 1);
}

#[test]
fn markdown_recovers_after_bad_blocks_and_reports_unclosed_blocks() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("rules.md"), "```rule\n[bad\n```\n```rule\nid: valid\nscope: paragraph\nkind: text\ncheck: {type: forbid, pattern: very}\n```\n```rule\nid: unclosed\n").unwrap();
    let registry = guidelines::load(root.path()).unwrap();
    assert_eq!(registry.active.len(), 1);
    assert_eq!(registry.issues.len(), 2);
}

#[test]
fn duplicate_ids_are_rejected_across_files() {
    let root = tempfile::tempdir().unwrap();
    for name in ["a.yaml", "b.yaml"] {
        fs::write(
            root.path().join(name),
            "id: duplicate\nscope: paragraph\nkind: text\ncheck: {type: forbid, pattern: very}",
        )
        .unwrap();
    }
    let registry = guidelines::load(root.path()).unwrap();
    assert!(registry.active.is_empty());
    assert_eq!(registry.issues.len(), 2);
}

#[test]
fn independent_rules_do_not_conflict_and_priority_is_enforced() {
    let registry = load(
        "rules:\n  - {id: require, scope: paragraph, kind: text, priority: 100, check: {type: contains, pattern: very}}\n  - {id: forbid, scope: paragraph, kind: text, priority: 10, check: {type: forbid, pattern: very}}\n  - {id: independent, scope: paragraph, kind: text, check: {type: forbid, pattern: vague}}\n",
    );
    assert_eq!(registry.active.len(), 2);
    assert_eq!(registry.conflicts.len(), 1);
    assert!(registry.issues.is_empty());
    let target = DocumentTarget {
        id: "p1".into(),
        target_type: "paragraph".into(),
        text: "very clear".into(),
        ..Default::default()
    };
    let (findings, issues) = review::run(&[target], &registry, None);
    assert!(findings.is_empty());
    assert!(issues.is_empty());
    let mut result = ReviewReport::new(
        "paper".into(),
        "main.tex".into(),
        "main.pdf".into(),
        Default::default(),
    );
    result.conflicts = registry.conflicts;
    assert!(report::markdown(&result).contains("higher priority wins: require"));
}

#[test]
fn tied_conflicts_are_not_executed() {
    let registry = load(
        "rules:\n  - {id: require, scope: paragraph, kind: text, check: {type: contains, pattern: very}}\n  - {id: forbid, scope: paragraph, kind: text, check: {type: forbid, pattern: very}}\n",
    );
    assert!(registry.active.is_empty());
    assert_eq!(registry.conflicts.len(), 1);
    assert_eq!(registry.issues.len(), 1);
}

#[test]
fn highest_priority_resolves_entire_conflict_group() {
    let registry = load(
        "rules:\n  - {id: low, scope: paragraph, kind: text, priority: 1, check: {type: contains, pattern: very}}\n  - {id: middle, scope: paragraph, kind: text, priority: 2, check: {type: forbid, pattern: very}}\n  - {id: high, scope: paragraph, kind: text, priority: 3, check: {type: contains, pattern: very}}\n",
    );
    assert_eq!(
        registry
            .active
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        vec!["high", "low"]
    );
    assert!(registry.issues.is_empty());
}
