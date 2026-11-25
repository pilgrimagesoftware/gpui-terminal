//! Teardown ends what the child started, not just the child: a descendant
//! that ignores SIGHUP - which the child's own hangup, and the kernel's when a
//! session leader exits, both leave running - is killed once the grace is up.

use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::*;
use crate::{Terminal, TerminalBuilder};

/// How long a test waits for a process to come up or go away - long, so a
/// machine busy compiling doesn't make it flaky; the work takes milliseconds.
const WAIT: Duration = Duration::from_secs(30);

/// Runs `script` in `sh -c` on a PTY.
fn run(script: &str) -> Terminal<PtyTransport> {
    let command = PtyCommand::new("/bin/sh").arg("-c").arg(script);
    TerminalBuilder::new().connect(|sink| PtyTransport::spawn(&command, sink))
                          .expect("/bin/sh spawns on a pty")
}

/// The PID `path` comes to hold.
fn pid_in(path: &Path) -> libc::pid_t {
    let deadline = Instant::now() + WAIT;
    loop {
        if let Some(pid) = std::fs::read_to_string(path).ok()
                                                        .and_then(|text| text.trim().parse().ok())
        {
            return pid;
        }
        assert!(Instant::now() < deadline, "the script never wrote its PID");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn alive(pid: libc::pid_t) -> bool {
    // SAFETY: signal 0 only checks that `pid` exists; it is above 1.
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Kills `pid` if a failed test left it running.
struct Reap(libc::pid_t);

impl Drop for Reap {
    fn drop(&mut self) {
        if self.0 > 1 && alive(self.0) {
            // SAFETY: as in `alive`; SIGKILL to a stray test process.
            unsafe { libc::kill(self.0, libc::SIGKILL) };
        }
    }
}

/// Whether `pid` goes away within [`WAIT`].
fn goes_away(pid: libc::pid_t) -> bool {
    let deadline = Instant::now() + WAIT;
    while Instant::now() < deadline {
        if !alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn terminate_kills_a_descendant_that_ignores_sighup() {
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("pid");
    // `exec` keeps the subshell's PID, and an ignored signal stays ignored
    // across it: `sleep` shrugs off SIGHUP, in the shell's process group.
    let terminal = run(&format!("(trap '' HUP; exec sleep 300) & echo $! > '{}'; wait",
                                pidfile.display()));
    let pid = pid_in(&pidfile);
    let _reap = Reap(pid);
    assert!(alive(pid));

    let started = Instant::now();
    terminal.terminate().expect("terminate succeeds");
    assert!(started.elapsed() < crate::consts::PTY_KILL_GRACE,
            "terminate returns without waiting out the grace");

    assert!(goes_away(pid),
            "the SIGHUP-ignoring descendant is killed too");
}

#[test]
fn dropping_the_transport_kills_a_descendant_that_ignores_sighup() {
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("pid");
    let (exit_tx, exit_rx) = mpsc::channel();
    let command = PtyCommand::new("/bin/sh").arg("-c").arg(format!(
        "(trap '' HUP; exec sleep 300) & echo $! > '{}'; wait",
        pidfile.display()
    ));
    let terminal = TerminalBuilder::new().on_exit(move |report| {
                                             let _ = exit_tx.send(report);
                                         })
                                         .connect(|sink| PtyTransport::spawn(&command, sink))
                                         .expect("/bin/sh spawns on a pty");
    let pid = pid_in(&pidfile);
    let _reap = Reap(pid);

    drop(terminal);

    assert!(goes_away(pid), "dropping ends the whole group");
    exit_rx.recv_timeout(WAIT)
           .expect("the shell's exit is still reported");
}

/// Knot's 16-hour hang: the reader thread used to hold the child's lock
/// across its blocking `wait`, so `terminate` - which took that lock to find
/// the child - waited for an exit it was meant to cause. A child that closes
/// every handle on the terminal but runs on gives the reader end-of-file, so
/// it blocks in `wait`; `terminate` must still return at once, and end it.
///
/// Linux only: there the master reads end-of-file (EIO) as soon as the last
/// slave handle closes. macOS's master doesn't until the child exits, so the
/// reader can't be in `wait` while the child lives - nor can it deadlock.
#[cfg(target_os = "linux")]
#[test]
fn terminate_returns_while_the_reader_is_blocked_in_wait() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("closed");
    let pidfile = dir.path().join("pid");
    let terminal = run(&format!("exec </dev/null >/dev/null 2>&1; echo $$ > '{}'; \
                                 touch '{}'; exec sleep 300",
                                pidfile.display(),
                                marker.display()));
    let pid = pid_in(&pidfile);
    let _reap = Reap(pid);
    let deadline = Instant::now() + WAIT;
    while !marker.exists() {
        assert!(Instant::now() < deadline,
                "the child never closed the terminal");
        std::thread::sleep(Duration::from_millis(20));
    }
    // Give the reader time to see end-of-file and enter `wait`.
    std::thread::sleep(Duration::from_millis(300));
    assert!(alive(pid), "the child runs on");

    let (done_tx, done_rx) = mpsc::channel();
    let terminal = std::sync::Arc::new(terminal);
    std::thread::spawn({
        let terminal = std::sync::Arc::clone(&terminal);
        move || {
            let _ = done_tx.send(terminal.terminate());
        }
    });
    let result = done_rx.recv_timeout(Duration::from_secs(5))
                        .expect("terminate returns while the reader waits on the child");
    result.expect("terminate succeeds");
    assert!(goes_away(pid), "and the child is ended");
}
