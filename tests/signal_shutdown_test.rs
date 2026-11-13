//! A signal to gaffa stops every process it started. Each child leads its own
//! process group, so a child gaffa forgets outlives it.
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
