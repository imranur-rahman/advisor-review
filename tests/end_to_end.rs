use std::fs;
use std::process::Command;
mod common;

#[test]
fn review_writes_json_and_markdown_findings() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("paper");
    let guidelines = root.path().join("guidelines");
    let output = root.path().join("review");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&guidelines).unwrap();
    fs::write(
        project.join("main.tex"),
        "\\section{Intro}\nThis is very vague.\n",
    )
    .unwrap();
    common::write_pdf(&project.join("main.pdf"));
    fs::write(guidelines.join("advisor.yaml"), "id: prose.avoid-very\nscope: paragraph\nkind: text\nseverity: warning\ncheck:\n  type: forbid\n  pattern: very\n  message: Avoid vague intensifiers.\n  suggestion: Replace it with a measurable claim.\n").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_advisor-review"))
        .env_remove("ADVISOR_REVIEW_PROVIDER")
        .env_remove("ADVISOR_REVIEW_MODEL")
        .env_remove("ADVISOR_REVIEW_ENDPOINT")
        .args(["review", "--project"])
        .arg(&project)
        .args(["--guidelines"])
        .arg(&guidelines)
        .args(["--output"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let json = fs::read_to_string(output.join("findings.json")).unwrap();
    assert!(json.contains("prose.avoid-very"));
    assert!(json.contains("paragraph"));
    let markdown = fs::read_to_string(output.join("findings.md")).unwrap();
    assert!(markdown.contains("Avoid vague intensifiers"));
    assert!(markdown.contains("**Status:** ✅ Complete"));
    assert!(markdown.contains("warning · violation"));
    assert!(String::from_utf8_lossy(&result.stdout).contains("review complete"));
    assert!(String::from_utf8_lossy(&result.stdout).contains("findings.md"));
}

#[test]
fn vscode_output_emits_source_diagnostics_and_keeps_reports() {
    let root = tempfile::tempdir().unwrap();
    common::fixture(root.path());
    let output = common::cli(root.path())
        .args(["--format", "vscode"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sections/body.tex(1,1): warning:"));
    assert!(stdout.contains("[prose.avoid-very] violation"), "{stdout}");
    assert!(stdout.contains("Evidence: very"));
    let matcher = regex::Regex::new(r"^(.+)\((\d+),(\d+)\): (error|warning|info): (.*)$").unwrap();
    assert!(stdout.lines().any(|line| matcher.is_match(line)));
    assert!(root.path().join("review/findings.json").exists());
    assert!(root.path().join("review/findings.md").exists());
}
