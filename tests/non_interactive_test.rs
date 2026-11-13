use std::process::Command;

#[test]
fn test_help_includes_interactive() {
    let output = Command::new("cargo")
        .args(["run", "--", "run", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--interactive"),
        "Help should mention --interactive flag: {stdout}"
    );
}
