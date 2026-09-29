use std::process::Command;

#[test]
fn test_interactive_flag() {
    // Test that --interactive flag is accepted
    let output = Command::new("cargo")
        .args(["run", "--", "run", "--interactive", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Run processes from Procfile") || output.status.success());
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_help_includes_interactive() {
        // Test that help text includes the --interactive option
        let output = Command::new("cargo")
            .args(["run", "--", "run", "--help"])
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("--interactive")
                || stdout.contains("-i")
                || stdout.contains("Terminal UI"),
            "Help should mention --interactive flag"
        );
    }
}
