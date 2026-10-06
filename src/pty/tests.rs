//! A real shell on a real PTY.

use std::sync::mpsc;
use std::time::Duration;

use super::*;
use crate::{Terminal, TerminalBuilder};

/// Failsafe for tests that drive a real PTY subprocess. Far longer than the
/// work takes (milliseconds), because its only job is to fail a hung test
/// rather than block forever: spawning a shell through a PTY on a machine
/// busy compiling the rest of the workspace can take seconds, and a tight
/// budget is what made these flaky before.
const PTY_TIMEOUT: Duration = Duration::from_secs(60);

fn shell(builder: TerminalBuilder) -> Terminal<PtyTransport> {
    let command = PtyCommand::new("/bin/sh").cwd(std::env::temp_dir());
    builder.connect(|sink| PtyTransport::spawn(&command, sink))
           .expect("/bin/sh spawns on a pty")
}

#[test]
fn strip_host_terminal_env_removes_host_identity_vars() {
    // SAFETY: no other test in this process reads these keys, and each is
    // removed again before the test returns.
    unsafe {
        std::env::set_var("WARP_TEST_SESSION_UUID", "test-session");
        std::env::set_var("TERM_PROGRAM", "WarpTerminal");
        std::env::set_var("PLAIN_TEST_VAR", "kept");
    }

    let command = PtyCommand::new("/bin/sh").to_builder();

    assert_eq!(command.get_env("WARP_TEST_SESSION_UUID"), None);
    assert_eq!(command.get_env("TERM_PROGRAM"), None);
    assert_eq!(command.get_env("PLAIN_TEST_VAR"),
               Some(std::ffi::OsStr::new("kept")));

    // SAFETY: as above.
    unsafe {
        std::env::remove_var("WARP_TEST_SESSION_UUID");
        std::env::remove_var("TERM_PROGRAM");
        std::env::remove_var("PLAIN_TEST_VAR");
    }
}

#[test]
fn an_explicit_env_wins_over_the_strip() {
    let command = PtyCommand::new("/bin/sh").env("TERM_PROGRAM", "Knot")
                                            .to_builder();

    assert_eq!(command.get_env("TERM_PROGRAM"),
               Some(std::ffi::OsStr::new("Knot")));
}

#[test]
fn output_and_exit_status_reach_the_terminal() {
    let (output_tx, output_rx) = mpsc::channel();
    let (exit_tx, exit_rx) = mpsc::channel();
    let terminal = shell(TerminalBuilder::new().on_output(move |bytes| {
                                                   let _ = output_tx.send(bytes.to_vec());
                                               })
                                               .on_exit(move |report| {
                                                   let _ = exit_tx.send(report);
                                               }));

    terminal.write(b"printf ready; exit 3\n")
            .expect("the shell takes input");
    let report = exit_rx.recv_timeout(PTY_TIMEOUT).expect("the shell exits");
    let output = output_rx.try_iter().flatten().collect::<Vec<_>>();

    assert_eq!(report, ExitReport::new(Some(3)));
    assert_eq!(terminal.exit_report(), Some(report));
    assert!(String::from_utf8_lossy(&output).contains("ready"));
}

#[test]
fn process_id_names_the_child_until_it_exits() {
    let (exit_tx, exit_rx) = mpsc::channel();
    let terminal = shell(TerminalBuilder::new().on_exit(move |report| {
                                                   let _ = exit_tx.send(report);
                                               }));

    let pid = terminal.process_id()
                      .expect("a running shell has a process");
    assert!(pid > 1, "got {pid}");

    terminal.write(b"exit 0\n").expect("the shell takes input");
    exit_rx.recv_timeout(PTY_TIMEOUT).expect("the shell exits");

    assert_eq!(terminal.process_id(),
               None,
               "the process is gone once it exits");
}

#[test]
fn resize_reaches_the_pty() {
    let terminal = shell(TerminalBuilder::new());
    terminal.resize(GridSize { columns: 100,
                               rows:    40, })
            .expect("the pty resizes");

    terminal.write(b"stty size\n")
            .expect("the shell takes input");

    let deadline = std::time::Instant::now() + PTY_TIMEOUT;
    while !terminal.with_grid(|grid| {
                       (0..grid.size().rows).any(|row| grid.row_text(row).contains("40 100"))
                   })
    {
        assert!(std::time::Instant::now() < deadline,
                "pty size was never reported as 40 rows x 100 cols");
        thread::sleep(Duration::from_millis(20));
    }
}
