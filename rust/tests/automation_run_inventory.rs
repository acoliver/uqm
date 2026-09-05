//! A finished run must be able to prove it left nothing behind.
//!
//! The per-session receipt says the child exited. It does not say whether the
//! run still owns anything, which is the question that matters before a
//! supervisor claims a clean teardown.

use std::process::Command;
use std::time::Duration;

use uqm_rust::automation::child_session::{
    command_executable_digest, ownership_artifacts, ChildSession, ChildSessionConfig, RunLock,
};
use uqm_rust::automation::inventory;

fn process_is_live(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // SAFETY: signal zero performs an existence check only.
    unsafe { libc::kill(pid, 0) == 0 }
}

#[test]
fn a_completed_run_accounts_for_every_resource_it_owned() {
    let parent = tempfile::tempdir().expect("tempdir");
    let run_root = parent.path().join("output");
    std::fs::create_dir(&run_root).expect("create run root");
    let digest = command_executable_digest(&Command::new("/bin/sh")).expect("shell digest");

    let mut ownership = RunLock::acquire(&run_root, &digest).expect("take ownership");

    let mut command = Command::new("/bin/sh");
    command.args(["-c", "exit 0"]);
    let config = ChildSessionConfig {
        stdout_log: run_root.join("out.log"),
        stderr_log: run_root.join("err.log"),
        stdout_budget: 1 << 20,
        stderr_budget: 1 << 20,
        timeout: Duration::from_secs(30),
        grace: Duration::from_millis(500),
        executable_digest: digest.clone(),
    };
    let receipt = ChildSession::spawn(command, config)
        .expect("spawn")
        .finish()
        .expect("finish");

    // Still holding ownership: the inventory must say so rather than pass.
    let held = inventory::collect(&receipt, &ownership_artifacts(&run_root), &process_is_live);
    assert!(
        !held.proves_no_leak(),
        "an unreleased run lock is a leak: {}",
        held.summary()
    );
    assert!(held.summary().contains("run lock guard"));

    ownership.release().expect("release ownership");

    let released = inventory::collect(&receipt, &ownership_artifacts(&run_root), &process_is_live);
    assert!(
        released.proves_no_leak(),
        "a released run owns nothing: {}",
        released.summary()
    );
    assert_eq!(released.supervised, receipt.identity);
    assert_eq!(released.summary(), "run released every resource it owned");
}
