use std::fs;
use std::process::Command;

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
    fs::write(project.join("main.pdf"), b"%PDF-1.7\n").unwrap();
    fs::write(guidelines.join("advisor.yaml"), "id: prose.avoid-very\nscope: paragraph\nkind: text\nseverity: warning\ncheck:\n  type: forbid\n  pattern: very\n  message: Avoid vague intensifiers.\n  suggestion: Replace it with a measurable claim.\n").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_advisor-review"))
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
}
