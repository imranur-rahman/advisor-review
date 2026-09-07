mod common;
use advisor_review::{discover, guidelines, latex, model::ReviewReport, pdf, report, review};
use std::{fs, path::Path};

#[test]
fn documented_relative_paths_and_document_scope_work() {
    let root = tempfile::tempdir().unwrap();
    common::fixture(root.path());
    let output = common::cli(root.path()).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: ReviewReport =
        serde_json::from_slice(&fs::read(root.path().join("review/findings.json")).unwrap())
            .unwrap();
    assert!(!report.is_incomplete());
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, "prose.avoid-very");
    assert_eq!(
        report.findings[0]
            .target
            .anchor
            .source
            .as_ref()
            .unwrap()
            .file,
        "sections/body.tex"
    );
    assert_eq!(
        report.findings[0]
            .target
            .anchor
            .source
            .as_ref()
            .unwrap()
            .start_line,
        1
    );
    assert!(report.issues.iter().any(|i| i.kind == "pdf_mapping"));
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.target.anchor.pdf.is_none())
    );
}

#[test]
fn discovery_rejects_explicit_missing_and_ambiguous_sources() {
    let root = tempfile::tempdir().unwrap();
    common::fixture(root.path());
    let project = root.path().join("paper");
    let rules = root.path().join("guidelines");
    assert!(
        discover::discover(&project, &rules, Some(Path::new("missing.tex")), None)
            .unwrap_err()
            .to_string()
            .contains("explicit")
    );
    fs::rename(project.join("main.tex"), project.join("paper.tex")).unwrap();
    assert!(
        discover::discover(&project, &rules, None, None)
            .unwrap_err()
            .to_string()
            .contains("multiple")
    );
    let inputs = discover::discover(
        &project,
        &rules,
        Some(Path::new("paper.tex")),
        Some(Path::new("main.pdf")),
    )
    .unwrap();
    assert_eq!(
        inputs.main_tex,
        project.canonicalize().unwrap().join("paper.tex")
    );
    fs::remove_file(project.join("sections/body.tex")).unwrap();
    assert!(discover::discover(&project, &rules, None, None).is_ok());
}

#[test]
fn missing_cli_inputs_exit_two() {
    let root = tempfile::tempdir().unwrap();
    common::fixture(root.path());
    assert_eq!(
        common::cli(root.path())
            .args(["--main-tex", "missing.tex"])
            .status()
            .unwrap()
            .code(),
        Some(2)
    );
    assert_eq!(
        common::cli(root.path())
            .args(["--provider", "typo", "--model", "test"])
            .status()
            .unwrap()
            .code(),
        Some(2)
    );
    assert_eq!(
        common::cli(root.path())
            .args(["--provider", "anthropic"])
            .status()
            .unwrap()
            .code(),
        Some(2)
    );
}

#[test]
fn invalid_rules_and_skipped_checks_preserve_results_and_exit_one() {
    let root = tempfile::tempdir().unwrap();
    common::fixture(root.path());
    fs::write(root.path().join("guidelines/bad.yaml"), "rules:\n  - id: bad\n    scope: paragraph\n    kind: text\n    check: {type: regex, pattern: '['}\n  - id: later-valid\n    scope: paragraph\n    kind: text\n    check: {type: forbid, pattern: vague}\n  - id: unsupported\n    scope: pdf_page\n    kind: text\n    check: {type: contains, pattern: Introduction}\n").unwrap();
    let output = common::cli(root.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let result: ReviewReport =
        serde_json::from_slice(&fs::read(root.path().join("review/findings.json")).unwrap())
            .unwrap();
    assert_eq!(result.findings.len(), 2);
    assert!(result.issues.iter().any(|i| i.kind == "guideline"));
    assert!(
        result
            .issues
            .iter()
            .any(|i| i.rule_id.as_deref() == Some("unsupported") && i.kind == "skipped")
    );
    assert!(report::markdown(&result).contains("Review incomplete"));
}

#[test]
fn pdf_parser_rejects_spoofed_and_truncated_files() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("test.pdf");
    for bytes in [
        b"Not a PDF /Type /Page".as_slice(),
        b"%PDF-1.7\n",
        b"%PDF-1.7\n/Type /Pages /Type /Page\n%%EOF",
    ] {
        fs::write(&path, bytes).unwrap();
        assert!(pdf::inspect(&path).is_err());
    }
    common::write_pdf(&path);
    assert_eq!(pdf::inspect(&path).unwrap().page_count, 1);
    common::fixture(root.path());
    fs::write(root.path().join("paper/main.pdf"), b"not a PDF").unwrap();
    assert_eq!(
        common::cli(root.path()).output().unwrap().status.code(),
        Some(1)
    );
}

#[test]
fn extraction_ignores_comments_and_code_and_retains_captions() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("main.tex");
    fs::write(
        &source,
        r"% \input{missing}
\section{Introduction}
\emph{very visible} and 10\%. % private

\begin{abstract}
very visible abstract
\end{abstract}
\begin{figure}
\includegraphics[width=2in]{plot}
\caption{Full caption with \emph{nested} text.}
\label{fig:plot}
\end{figure}
\begin{lstlisting}
\input{not-a-real-include}
% literal code
\end{lstlisting}
See \cite{one} and \cite{two}.
",
    )
    .unwrap();
    let targets = latex::parse_project(&source, root.path()).unwrap();
    let figures: Vec<_> = targets
        .iter()
        .filter(|t| t.target_type == "figure")
        .collect();
    assert_eq!(figures.len(), 1);
    assert!(figures[0].text.contains("Full caption with"));
    assert_eq!(figures[0].label.as_deref(), Some("fig:plot"));
    assert_eq!(figures[0].anchor.source.as_ref().unwrap().start_line, 8);
    assert_eq!(figures[0].anchor.source.as_ref().unwrap().end_line, 12);
    let paragraphs: Vec<_> = targets
        .iter()
        .filter(|t| t.target_type == "paragraph")
        .collect();
    assert!(paragraphs.iter().any(|t| t.text.contains("very visible")));
    assert!(
        paragraphs
            .iter()
            .any(|t| t.text.contains("very visible abstract"))
    );
    assert!(
        paragraphs
            .iter()
            .all(|t| !t.text.contains("private") && !t.text.contains("literal code"))
    );
    assert_eq!(
        targets
            .iter()
            .filter(|t| t.target_type == "citation")
            .count(),
        2
    );
    assert_eq!(
        targets
            .iter()
            .filter(|t| t.target_type == "document")
            .count(),
        1
    );
    assert!(
        targets
            .iter()
            .find(|t| t.target_type == "code_block")
            .unwrap()
            .text
            .contains("% literal code")
    );
}

#[test]
fn missing_includes_are_not_silently_ignored() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.tex"), "\\input{missing}").unwrap();
    assert!(latex::parse_project(&root.path().join("main.tex"), root.path()).is_err());
}

#[test]
fn unsupported_scopes_and_missing_asset_evidence_are_explicit() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.tex"), "\\includegraphics{missing}").unwrap();
    fs::write(root.path().join("rules.yaml"), "rules:\n  - id: pixels\n    scope: figure\n    kind: asset\n    check: {type: min_pixels, value: 100}\n  - id: page\n    scope: pdf_page\n    kind: text\n    check: {type: forbid, pattern: vague}\n").unwrap();
    let targets = latex::parse_project(&root.path().join("main.tex"), root.path()).unwrap();
    let registry = guidelines::load(root.path()).unwrap();
    let (findings, issues) = review::run(&targets, &registry, None);
    assert!(findings.is_empty());
    assert_eq!(issues.len(), 2);
    assert!(issues.iter().all(|i| i.kind == "skipped"));
}

#[test]
fn document_missing_phrase_produces_one_finding() {
    let root = tempfile::tempdir().unwrap();
    common::fixture(root.path());
    fs::write(root.path().join("guidelines/rules.yaml"), "id: missing\nscope: document\nkind: text\ncheck: {type: contains, pattern: Acknowledgments}").unwrap();
    let output = common::cli(root.path()).output().unwrap();
    assert!(output.status.success());
    let report: ReviewReport =
        serde_json::from_slice(&fs::read(root.path().join("review/findings.json")).unwrap())
            .unwrap();
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].target.target_type, "document");
    assert!(report.findings[0].target.anchor.source.is_none());
}

#[test]
fn no_rules_and_unavailable_semantic_provider_are_incomplete() {
    let root = tempfile::tempdir().unwrap();
    common::fixture(root.path());
    for rules in [
        "rules: []",
        "id: semantic\nscope: paragraph\nkind: semantic-text\ndescription: Assess clarity.\ncheck: {type: semantic}",
    ] {
        fs::write(root.path().join("guidelines/rules.yaml"), rules).unwrap();
        assert_eq!(
            common::cli(root.path()).output().unwrap().status.code(),
            Some(1)
        );
        let report: ReviewReport =
            serde_json::from_slice(&fs::read(root.path().join("review/findings.json")).unwrap())
                .unwrap();
        assert_eq!(
            report.issues.iter().filter(|i| i.kind == "skipped").count(),
            if rules == "rules: []" { 1 } else { 2 }
        );
    }
}

#[test]
fn extensionless_assets_provide_dpi_and_invalid_width_is_skipped() {
    let root = tempfile::tempdir().unwrap();
    // A real 1x1 PNG (signature, IHDR, IDAT, and IEND).
    let png = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5,
        1, 1, 39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    fs::write(root.path().join("plot.png"), png).unwrap();
    fs::write(
        root.path().join("main.tex"),
        "\\includegraphics[width=2in]{plot}\n\\includegraphics[width=0in]{plot}\n",
    )
    .unwrap();
    fs::write(
        root.path().join("rules.yaml"),
        "id: dpi\nscope: figure\nkind: asset\ncheck: {type: min_effective_dpi, value: 300}",
    )
    .unwrap();
    let targets = latex::parse_project(&root.path().join("main.tex"), root.path()).unwrap();
    let registry = guidelines::load(root.path()).unwrap();
    let (findings, issues) = review::run(&targets, &registry, None);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].target.facts["effective_dpi"], 0.5);
    assert!(
        findings[0]
            .target
            .anchor
            .asset
            .as_ref()
            .unwrap()
            .ends_with("plot.png")
    );
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].kind, "skipped");
}

#[test]
fn nested_tables_have_unique_cells_and_unclosed_environments_fail() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.tex");
    fs::write(
        &path,
        "\\begin{table}\n\\begin{tabular}{cc}\na & b \\\\\n\\end{tabular}\n\\end{table}\n",
    )
    .unwrap();
    let targets = latex::parse_project(&path, root.path()).unwrap();
    assert_eq!(
        targets
            .iter()
            .filter(|t| t.target_type == "table_cell")
            .count(),
        2
    );
    fs::write(&path, "\\begin{table}\n").unwrap();
    assert!(latex::parse_project(&path, root.path()).is_err());
}
