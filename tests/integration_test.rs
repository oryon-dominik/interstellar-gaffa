use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

/// Where a test writes its throwaway files: cargo's scratch dir under target/,
/// never the package root.
fn scratch(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(name)
}

#[test]
fn test_gaffa_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("gaffa"));
    assert!(stdout.contains("Procfile"));
}

#[test]
fn test_gaffa_no_args() {
    let output = Command::new("cargo")
        .args(["run"])
        .output()
        .expect("Failed to execute command");

    // The new binary prints help to stderr when no command is given
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cross-platform process manager for Procfile-based applications"));
    assert!(stderr.contains("Usage:"));
}

#[test]
fn test_gaffa_run_nonexistent_procfile() {
    let output = Command::new("cargo")
        .args(["run", "--", "run", "-p", "nonexistent.procfile"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to read Procfile"));
}

#[test]
fn test_gaffa_with_test_procfile() {
    // Create a simple test Procfile
    let procfile_content = if cfg!(windows) {
        "echo: cmd /c echo Hello from test"
    } else {
        "echo: echo 'Hello from test'"
    };
    let procfile = scratch("test_integration.procfile");
    std::fs::write(&procfile, procfile_content).expect("Failed to write test procfile");

    // Start gaffa with the test procfile in a separate thread
    let spawned = procfile.clone();
    let handle = thread::spawn(move || {
        let mut child = Command::new("cargo")
            .args(["run", "--", "run", "-p"])
            .arg(&spawned)
            .spawn()
            .expect("Failed to start gaffa");

        // Let it run for a bit
        thread::sleep(Duration::from_secs(2));

        // Kill the process
        let _ = child.kill();
        child.wait().expect("Failed to wait for child");
    });

    handle.join().expect("Thread panicked");

    // Cleanup
    let _ = std::fs::remove_file(&procfile);
}

#[test]
fn test_gaffa_log_file() {
    // Create a simple test Procfile
    let procfile_content = if cfg!(windows) {
        "logger: cmd /c echo Log this message"
    } else {
        "logger: echo 'Log this message'"
    };
    let procfile = scratch("test_log.procfile");
    let log_file = scratch("test_output.log");
    std::fs::write(&procfile, procfile_content).expect("Failed to write test procfile");
    // A failed earlier run leaves its rotated files behind; start from none.
    for p in rotated_logs() {
        let _ = std::fs::remove_file(p);
    }

    let mut child = Command::new("cargo")
        .args(["run", "--", "run", "-p"])
        .arg(&procfile)
        .arg("--log-file")
        .arg(&log_file)
        .spawn()
        .expect("Failed to start gaffa");

    // Wait for the line instead of a fixed time: under gaffa, pwsh took 1.7 to
    // 2.4 s to print its first line on Windows (measured 2026-09-28), and a
    // 2 s sleep killed gaffa before the child had written anything.
    let deadline = Instant::now() + Duration::from_secs(30);
    let logged = loop {
        let found = rotated_logs().into_iter().any(|p| {
            std::fs::read_to_string(p)
                .map(|content| content.contains("logger"))
                .unwrap_or(false)
        });
        if found || Instant::now() > deadline {
            break found;
        }
        thread::sleep(Duration::from_millis(100));
    };

    let _ = child.kill();
    child.wait().expect("Failed to wait for child");

    let rotated = rotated_logs();
    let _ = std::fs::remove_file(&procfile);
    for p in &rotated {
        let _ = std::fs::remove_file(p);
    }
    let _ = std::fs::remove_file(&log_file);

    assert!(
        !rotated.is_empty(),
        "No rotated test_output-*.log file was created"
    );
    assert!(
        logged,
        "the logger line never reached the log file within 30 s"
    );
}

#[test]
fn test_redirected_stdout_carries_no_escape_sequence() {
    // Terminal restoration wrote ESC[?25h ESC[0m into a redirected log on exit.
    let procfile_content = if cfg!(windows) {
        "quiet: cmd /c echo plain"
    } else {
        "quiet: echo plain"
    };
    let procfile = scratch("test_redirect.procfile");
    std::fs::write(&procfile, procfile_content).expect("Failed to write test procfile");

    let output = Command::new("cargo")
        .args(["run", "--", "run", "-p"])
        .arg(&procfile)
        .output()
        .expect("Failed to run gaffa");
    let _ = std::fs::remove_file(&procfile);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("plain"),
        "the process output is missing: {stdout:?}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "escape sequence in redirected stdout: {stdout:?}"
    );
}

/// gaffa rotates `test_output.log` to `test_output-YYYY-MM-DD_NNN.log` beside it
/// (`rotate_log_path` in src/bin/gaffa.rs), so in the scratch dir.
fn rotated_logs() -> Vec<PathBuf> {
    std::fs::read_dir(env!("CARGO_TARGET_TMPDIR"))
        .expect("Failed to read the scratch directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("test_output-") && n.ends_with(".log"))
                .unwrap_or(false)
        })
        .collect()
}

#[test]
fn test_gaffa_specific_processes() {
    // Create a test Procfile with multiple processes
    let procfile_content = if cfg!(windows) {
        "web: cmd /c echo Web server\nworker: cmd /c echo Worker process\nscheduler: cmd /c echo Scheduler"
    } else {
        "web: echo 'Web server'\nworker: echo 'Worker process'\nscheduler: echo 'Scheduler'"
    };
    let procfile = scratch("test_specific.procfile");
    std::fs::write(&procfile, procfile_content).expect("Failed to write test procfile");

    // Start gaffa with only specific processes
    let spawned = procfile.clone();
    let handle = thread::spawn(move || {
        let mut child = Command::new("cargo")
            .args(["run", "--", "run", "-p"])
            .arg(&spawned)
            .args(["web", "worker"])
            .spawn()
            .expect("Failed to start gaffa");

        // Let it run for a bit
        thread::sleep(Duration::from_secs(2));

        // Kill the process
        let _ = child.kill();
        child.wait().expect("Failed to wait for child");
    });

    handle.join().expect("Thread panicked");

    // Cleanup
    let _ = std::fs::remove_file(&procfile);
}
