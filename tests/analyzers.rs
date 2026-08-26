use advisor_review::{guidelines, latex};
use std::fs;

#[test]
fn parses_included_and_specialized_targets() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("paper");
    let rules = root.path().join("guidelines");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&rules).unwrap();
    fs::write(project.join("main.tex"), "\\input{body}\n").unwrap();
    fs::write(
        project.join("body.tex"),
        r#"\\section{Results}
\\begin{tabular}{cc}
one & two \\\\
\\end{tabular}
See \\cite{smith} and Figure~\\ref{fig:x}.
\\begin{lstlisting}
const x = 1;
\\end{lstlisting}
"#,
    )
    .unwrap();
    let targets = latex::parse_project(&project.join("main.tex"), &project).unwrap();
    assert!(targets.iter().any(|t| t.target_type == "section"));
    assert!(targets.iter().any(|t| t.target_type == "table"));
    assert!(targets.iter().any(|t| t.target_type == "table_cell"));
    assert!(targets.iter().any(|t| t.target_type == "citation"));
    assert!(targets.iter().any(|t| t.target_type == "reference"));
    assert!(targets.iter().any(|t| t.target_type == "code_block"));
}

#[test]
fn loads_structured_rules_and_keeps_prose_as_candidate() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("advisor.md");
    fs::write(&file, "# Advisor\nAvoid vague prose.\n\n```rule\nid: prose.no-very\nscope: paragraph\nkind: text\ncheck:\n  type: forbid\n  pattern: very\n```\n").unwrap();
    let registry = guidelines::load(root.path()).unwrap();
    assert_eq!(registry.active.len(), 1);
    assert_eq!(registry.candidates.len(), 1);
    assert_eq!(
        registry.active[0].source.as_deref(),
        Some(file.to_str().unwrap())
    );
}

#[test]
fn invalid_structured_rule_is_reported_not_activated() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("bad.yaml"),
        "id: missing-check\nscope: paragraph\nkind: text\n",
    )
    .unwrap();
    let registry = guidelines::load(root.path()).unwrap();
    assert!(registry.active.is_empty());
    assert!(!registry.issues.is_empty());
}
