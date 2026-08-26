use std::process::Command;

#[test]
fn help_command_runs() {
    let output = Command::new(env!("CARGO_BIN_EXE_advisor-review"))
        .arg("--help")
        .output()
        .expect("CLI should start");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Review LaTeX manuscripts"));
}
