//! An interrupted supervisor must take its child with it.
//!
//! Before inbound signal handling existed, a supervisor that received SIGINT
//! or SIGTERM died where it stood and left the process it was supervising
//! running, holding its run lock and leaving its owned-process record stale.

use std::process::Command;
use std::time::{Duration, Instant};

use uqm_rust::automation::child_session::{
    command_executable_digest, ChildSession, ChildSessionConfig,
};
use uqm_rust::automation::interrupt;

fn config(root: &std::path::Path, command: &Command) -> ChildSessionConfig {
    ChildSessionConfig {
        stdout_log: root.join("out.log"),
        stderr_log: root.join("err.log"),
        stdout_budget: 1 << 20,
        stderr_budget: 1 << 20,
        timeout: Duration::from_secs(30),
        grace: Duration::from_millis(500),
        executable_digest: command_executable_digest(command).expect("shell digest"),
    }
}

fn process_is_live(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // SAFETY: signal zero performs an existence check only.
    unsafe { libc::kill(pid, 0) == 0 }
}

#[test]
fn an_interrupted_supervisor_tears_down_the_process_it_supervises() {
    let root = tempfile::tempdir().expect("tempdir");
    interrupt::clear();
    interrupt::install().expect("install interruption handling");

    // A child that ignores SIGTERM, so only a real escalation ends it.
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "trap '' TERM; while :; do sleep 1; done"]);
    let session =
        ChildSession::spawn(command, config(root.path(), &Command::new("/bin/sh"))).expect("spawn");
    let pid = session.pid();
    assert!(process_is_live(pid), "the child must be running");

    // The supervisor is interrupted mid-run.
    // SAFETY: raise delivers to this thread, where the handler is installed.
    assert_eq!(unsafe { libc::raise(libc::SIGTERM) }, 0);

    let started = Instant::now();
    let failure = session
        .finish_observing(|_| match interrupt::interrupted() {
            Some(signal) => Err(interrupt::interruption_error(signal)),
            None => Ok(()),
        })
        .expect_err("an interrupted run must not report success");

    assert!(
        failure.to_string().contains("interrupted by signal"),
        "the failure must say why the run stopped: {failure}"
    );
    assert!(
        !process_is_live(pid),
        "an interrupted supervisor must not leave its child running"
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "teardown must not wait out the full run timeout"
    );
    assert!(failure.receipt.term_sent, "escalation must be recorded");

    interrupt::clear();
}
