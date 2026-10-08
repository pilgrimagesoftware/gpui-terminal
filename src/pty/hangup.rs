//! Ending a PTY child and everything it started (unix).
//!
//! `portable-pty`'s `kill` sends SIGHUP to the child's PID alone. The child is
//! a session leader (`setsid` before exec), and when it exits the kernel hangs
//! up only the terminal's foreground process group - so a descendant that
//! ignores SIGHUP (`nohup`, `trap '' HUP`), or one in another group, outlives
//! the terminal that started it. Teardown instead signals whole process
//! groups - the child's own, and the terminal's foreground one - with SIGHUP,
//! then SIGKILLs whatever is still there after [`PTY_KILL_GRACE`].
//!
//! The grace runs on a thread of its own: a transport method is called with
//! the terminal's lock held and must not block.

use std::io;
use std::thread;
use std::time::Instant;

use crate::consts::{PTY_KILL_GRACE, PTY_KILL_POLL};

/// A process group it is safe to signal: never `0` (this process's group),
/// `1` (init's) or anything at or below - which `killpg` would read as "every
/// process" - and never this process's own group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Group(libc::pid_t);

impl Group {
    pub(super) fn new(pgid: libc::pid_t) -> Option<Self> {
        // SAFETY: `getpgrp` takes no arguments and cannot fail.
        let own = unsafe { libc::getpgrp() };
        (pgid > 1 && pgid != own).then_some(Self(pgid))
    }

    /// The group process `pid` leads or belongs to, if it is one [`Self::new`]
    /// allows. `None` once `pid` is gone.
    pub(super) fn of_process(pid: u32) -> Option<Self> {
        let pid = libc::pid_t::try_from(pid).ok().filter(|pid| *pid > 1)?;
        // SAFETY: `getpgid` only reads the group of `pid`; a gone or foreign
        // `pid` makes it fail with -1, which `new` rejects.
        Self::new(unsafe { libc::getpgid(pid) })
    }

    fn signal(self, signal: libc::c_int) -> io::Result<()> {
        // SAFETY: `killpg` only sends a signal. `self.0` is above 1 and not
        // this process's group (see `new`), so it reaches neither every
        // process nor this one.
        if unsafe { libc::killpg(self.0, signal) } == 0 {
            Ok(())
        }
        else {
            Err(io::Error::last_os_error())
        }
    }

    /// Whether any process is still in the group (a zombie counts).
    fn alive(self) -> bool {
        self.signal(0).is_ok()
    }
}

/// Hangs up `groups`: SIGHUP to each, then SIGCONT so a stopped member wakes
/// to take it, as the kernel does for an orphaned group. Whatever is still in
/// them after [`PTY_KILL_GRACE`] gets SIGKILL, from a background thread - this
/// returns at once.
///
/// A group already gone is not an error; any other failure to signal is
/// returned, after the rest have been tried.
pub(super) fn hang_up(mut groups: Vec<Group>) -> io::Result<()> {
    groups.dedup();
    let mut failure = None;
    groups.retain(|group| match group.signal(libc::SIGHUP) {
              Ok(()) => {
                  let _ = group.signal(libc::SIGCONT);
                  true
              }
              Err(error) if error.raw_os_error() == Some(libc::ESRCH) => false,
              Err(error) => {
                  failure.get_or_insert(error);
                  false
              }
          });
    if !groups.is_empty() {
        let spawned = thread::Builder::new().name("gpui-terminal-pty-hangup".into())
                                            .spawn({
                                                let groups = groups.clone();
                                                move || kill_survivors(&groups)
                                            });
        // No thread to wait on: don't leave survivors, kill them now.
        if spawned.is_err() {
            groups.iter().for_each(|group| {
                             let _ = group.signal(libc::SIGKILL);
                         });
        }
    }
    failure.map_or(Ok(()), Err)
}

/// Waits up to [`PTY_KILL_GRACE`] for `groups` to empty, then SIGKILLs any
/// that haven't.
fn kill_survivors(groups: &[Group]) {
    let deadline = Instant::now() + PTY_KILL_GRACE;
    while Instant::now() < deadline && groups.iter().any(|group| group.alive()) {
        thread::sleep(PTY_KILL_POLL);
    }
    for group in groups.iter().filter(|group| group.alive()) {
        let _ = group.signal(libc::SIGKILL);
    }
}

#[cfg(test)]
mod tests;
