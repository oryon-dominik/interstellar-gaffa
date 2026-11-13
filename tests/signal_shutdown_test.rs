//! Gaffa leaves nothing behind — no process on a signal or a failed UI, no
//! terminal controls in redirected output. Each child leads its own process
//! group, so a child gaffa forgets outlives it.
#![cfg(unix)]

use std::fs::File;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(20);

/// A Procfile whose one process writes its pid to a file, then sleeps under that pid.
fn procfile_writing_pid(name: &str) -> (PathBuf, PathBuf) {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pid_path = dir.join("sleeper.pid");
    let procfile = dir.join("Procfile");
    std::fs::write(
        &procfile,
        format!(
            "sleeper: echo $$ > {} && exec sleep 300\n",
            pid_path.display()
        ),
    )
    .unwrap();
    (procfile, pid_path)
}

/// Gaffa in non-interactive mode, its output discarded.
fn spawn_plain(procfile: &Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_gaffa"))
        .args(["run", "--procfile", procfile.to_str().unwrap()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .unwrap()
}

/// Gaffa's terminal UI on a pseudo terminal, its screen drained in the background.
fn spawn_interactive(procfile: &Path) -> Child {
    let (mut master, mut terminal) = (0, 0);
    let size = libc::winsize {
        ws_row: 24,
        ws_col: 100,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let opened = unsafe {
        libc::openpty(
            &mut master,
            &mut terminal,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &size,
        )
    };
    assert_eq!(opened, 0, "openpty failed");
    let terminal = unsafe { OwnedFd::from_raw_fd(terminal) };

    let mut command = Command::new(env!("CARGO_BIN_EXE_gaffa"));
    command
        .args([
            "run",
            "--interactive",
            "--procfile",
            procfile.to_str().unwrap(),
        ])
        .stdin(terminal.try_clone().unwrap())
        .stdout(terminal.try_clone().unwrap())
        .stderr(terminal);
    // A session of its own with the pseudo terminal as controlling terminal.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 || libc::ioctl(0, libc::TIOCSCTTY as _, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let gaffa = command.spawn().unwrap();

    let mut screen = unsafe { File::from_raw_fd(master) };
    std::thread::spawn(move || std::io::copy(&mut screen, &mut std::io::sink()));
    gaffa
}

fn wait_for_pid(pid_path: &Path) -> i32 {
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        if let Some(pid) = std::fs::read_to_string(pid_path)
            .ok()
            .and_then(|text| text.trim().parse().ok())
        {
            return pid;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("the sleeper never wrote {}", pid_path.display());
}

fn wait_for_exit(gaffa: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        if let Some(status) = gaffa.try_wait().unwrap() {
            return status;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = gaffa.kill();
    panic!("gaffa did not exit within {TIMEOUT:?} of the signal");
}

/// Whether the process still runs; an orphan is killed after answering, so no test leaks one.
fn survived(pid: i32) -> bool {
    let alive = unsafe { libc::kill(pid, 0) } == 0;
    if alive {
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }
    alive
}

/// Start gaffa, signal it once its process runs, and return its exit and whether the process survived.
fn signal_gaffa(name: &str, interactive: bool, signal: libc::c_int) -> (ExitStatus, bool) {
    let (procfile, pid_path) = procfile_writing_pid(name);
    let mut gaffa = if interactive {
        spawn_interactive(&procfile)
    } else {
        spawn_plain(&procfile)
    };
    let sleeper = wait_for_pid(&pid_path);

    unsafe { libc::kill(gaffa.id() as i32, signal) };
    let status = wait_for_exit(&mut gaffa);
    (status, survived(sleeper))
}

#[test]
fn test_sigint_to_the_terminal_ui_stops_its_processes() {
    let (_, survived) = signal_gaffa("signal_ui_sigint", true, libc::SIGINT);

    assert!(!survived, "the process outlived gaffa");
}

#[test]
fn test_sigterm_to_the_terminal_ui_stops_its_processes() {
    let (_, survived) = signal_gaffa("signal_ui_sigterm", true, libc::SIGTERM);

    assert!(!survived, "the process outlived gaffa");
}

#[test]
fn test_sigint_stops_the_processes_and_exits_130() {
    let (status, survived) = signal_gaffa("signal_plain_sigint", false, libc::SIGINT);

    assert!(!survived, "the process outlived gaffa");
    assert_eq!(status.code(), Some(130));
}

#[test]
fn test_sigterm_stops_the_processes_and_exits_130() {
    let (status, survived) = signal_gaffa("signal_plain_sigterm", false, libc::SIGTERM);

    assert!(!survived, "the process outlived gaffa");
    assert_eq!(status.code(), Some(130));
}

/// Gaffa's terminal UI in a session of its own and without a terminal, so the UI
/// fails while it starts; its output is captured.
fn spawn_interactive_without_terminal(procfile: &Path) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_gaffa"));
    command
        .args([
            "run",
            "--interactive",
            "--procfile",
            procfile.to_str().unwrap(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.spawn().unwrap()
}

#[test]
fn test_a_terminal_ui_that_cannot_start_leaves_no_process_behind() {
    let (procfile, pid_path) = procfile_writing_pid("ui_without_terminal");
    let mut gaffa = spawn_interactive_without_terminal(&procfile);

    let status = wait_for_exit(&mut gaffa);
    // A process gaffa left behind still writes its pid after gaffa is gone.
    std::thread::sleep(Duration::from_secs(1));

    assert!(!status.success(), "the UI started without a terminal");
    if let Some(sleeper) = std::fs::read_to_string(&pid_path)
        .ok()
        .and_then(|text| text.trim().parse().ok())
    {
        assert!(!survived(sleeper), "the process outlived gaffa");
    }
}

#[test]
fn test_a_terminal_ui_without_terminal_writes_no_terminal_controls() {
    let (procfile, _) = procfile_writing_pid("ui_without_terminal_output");

    let output = spawn_interactive_without_terminal(&procfile)
        .wait_with_output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains('\x1b'),
        "escape sequences on stdout: {stdout:?}"
    );
    assert!(!stderr.contains("stty"), "stty complained: {stderr}");
}
