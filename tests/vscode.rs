use advisor_review::{
    model::{DocumentTarget, ReviewFinding, ReviewReport, SourceSpan, TargetAnchor},
    report,
};

fn finding(severity: &str, location: Option<SourceSpan>) -> ReviewFinding {
    ReviewFinding {
        rule_id: "prose.clear-referent".into(),
        status: "concern".into(),
        severity: severity.into(),
        target: DocumentTarget {
            anchor: TargetAnchor {
                sources: location.into_iter().collect(),
                ..Default::default()
            },
            ..Default::default()
        },
        evidence: "this result".into(),
        explanation: "The referent is unclear.\nAdd context.".into(),
        suggestion: Some("Name the result being discussed.".into()),
        ..Default::default()
    }
}

#[test]
fn diagnostics_map_severity_and_normalize_messages() {
    let mut report = ReviewReport::new(
        "paper".into(),
        "main.tex".into(),
        "main.pdf".into(),
        Default::default(),
    );
    report.findings.push(finding(
        "suggestion",
        Some(SourceSpan {
            file: "sections/results.tex".into(),
            start_line: 9,
            start_column: Some(4),
            ..Default::default()
        }),
    ));
    let lines = report::vscode_diagnostics(&report);
    assert_eq!(lines.len(), 1);
    assert_eq!(
        lines[0],
        "sections/results.tex(9,4): info: [prose.clear-referent] concern The referent is unclear. Add context. Evidence: this result Suggestion: Name the result being discussed."
    );
}

#[test]
fn diagnostics_use_primary_source_and_skip_unlocated_findings() {
    let mut report = ReviewReport::new(
        "paper".into(),
        "main.tex".into(),
        "main.pdf".into(),
        Default::default(),
    );
    let mut located = finding(
        "error",
        Some(SourceSpan {
            file: "main.tex".into(),
            start_line: 3,
            start_column: None,
            ..Default::default()
        }),
    );
    located.target.anchor.sources.push(SourceSpan {
        file: "included.tex".into(),
        start_line: 7,
        start_column: Some(2),
        ..Default::default()
    });
    report.findings.push(located);
    report.findings.push(finding("warning", None));

    let lines = report::vscode_diagnostics(&report);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("main.tex(3,1): error:"));
}
