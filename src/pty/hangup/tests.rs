//! The guard on which process groups teardown may signal.

use super::*;

#[test]
fn a_group_is_never_everyone_init_or_this_process() {
    // SAFETY: `getpgrp` takes no arguments and cannot fail.
    let own = unsafe { libc::getpgrp() };

    for pgid in [-1, 0, 1, own] {
        assert_eq!(Group::new(pgid),
                   None,
                   "pgid {pgid} must never be signalled");
    }
    assert_eq!(Group::of_process(0), None);
    assert_eq!(Group::of_process(1), None, "init's group");
    assert_eq!(Group::of_process(std::process::id()),
               None,
               "this process's own group");
}

#[test]
fn hanging_up_a_gone_group_is_not_an_error() {
    // A PID far above any real one: its group can't exist.
    let gone = Group::new(libc::pid_t::MAX - 1).expect("a plausible pgid");
    assert!(hang_up(vec![gone]).is_ok());
}
