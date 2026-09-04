//! Failure fixtures issue #29 requires, driven against real processes and the
//! real transport rather than against models of them.
//!
//! Three classes had no such coverage: a child that dies from a signal rather
//! than a non-zero exit, a previous run's process that is still alive when the
//! next run starts, and the consequence of an acknowledgement that never
//! reached its sender.

use std::process::Command;
use std::time::Duration;

use uqm_rust::automation::child_session::{
    capture_identity, command_executable_digest, record_owned_process, ChildSession,
    ChildSessionConfig, StaleProcessScan,
};
use uqm_rust::automation::proof::PreflightCheck;
use uqm_rust::automation::trace::{AckOutcome, OrderedCommit, RecordKind, TraceRecord};
use uqm_rust::automation::transport::{
    AckKind, AckTraceContext, CommandId, TransportPacket, TransportState, PROTOCOL_VERSION,
};

fn config(root: &std::path::Path) -> ChildSessionConfig {
    ChildSessionConfig {
        stdout_log: root.join("out.log"),
        stderr_log: root.join("err.log"),
        stdout_budget: 1 << 20,
        stderr_budget: 1 << 20,
        timeout: Duration::from_secs(30),
        grace: Duration::from_millis(500),
        executable_digest: command_executable_digest(&Command::new("/bin/sh"))
            .expect("shell digest"),
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
fn a_child_that_dies_by_signal_is_recorded_as_a_crash_not_an_exit() {
    let root = tempfile::tempdir().expect("tempdir");
    let mut command = Command::new("/bin/sh");
    // The child crashes itself. Nothing in the supervisor asked it to stop.
    command.args(["-c", "kill -SEGV $$"]);

    let receipt = ChildSession::spawn(command, config(root.path()))
        .expect("spawn")
        .finish()
        .expect("a crash is a completed observation, not a supervision failure");

    assert_eq!(
        receipt.signal,
        Some(libc::SIGSEGV),
        "the terminating signal must be reported"
    );
    assert_eq!(
        receipt.exit_code, None,
        "a signalled process has no exit code to report"
    );
    assert!(
        !receipt.term_sent && !receipt.kill_sent,
        "the supervisor must not claim credit for a self-inflicted crash"
    );
    assert!(receipt.orphan_check_passed, "the group must still be empty");
    assert!(!process_is_live(receipt.identity.pid));
}

#[test]
fn a_previous_runs_live_process_is_reclaimed_before_the_next_run_starts() {
    let parent = tempfile::tempdir().expect("tempdir");
    let run_root = parent.path().join("output");
    std::fs::create_dir(&run_root).expect("create run root");

    // Stand in for a previous run that exited without reaping its child.
    let mut leftover = Command::new("/bin/sleep")
        .arg("120")
        .spawn()
        .expect("spawn leftover");
    let digest = command_executable_digest(&Command::new("/bin/sleep")).expect("sleep digest");
    let identity = capture_identity(leftover.id(), &digest).expect("capture leftover identity");
    record_owned_process(&run_root, &identity).expect("record the leftover");
    assert!(
        process_is_live(identity.pid),
        "the leftover must be running"
    );

    let (preflight, scan) =
        PreflightCheck::establish(&run_root, Duration::from_secs(5), true, true)
            .expect("a verified leftover is reclaimable");

    match scan {
        StaleProcessScan::Reclaimed {
            identity: reclaimed,
            ..
        } => assert_eq!(reclaimed, identity),
        other => panic!("expected the leftover to be reclaimed, got {other:?}"),
    }
    assert!(preflight.no_matching_processes);
    assert!(preflight.passes(), "the next run may now start");

    let status = leftover.wait().expect("reap the reclaimed leftover");
    assert!(!status.success(), "it was signalled, not asked to exit");
    assert!(!process_is_live(identity.pid));
}

#[test]
fn an_acknowledgement_that_never_arrives_makes_the_retry_a_visible_replay() {
    // A sender that does not see its acknowledgement retries the command. The
    // retry must not execute twice, and both the original and the rejected
    // retry must be attributable in retained evidence.
    let nonce = [42u8; 32];
    let commit = OrderedCommit::new();
    let mut state = TransportState::new(nonce, false);
    state.attach_trace(commit.clone());

    let packet = TransportPacket {
        version: PROTOCOL_VERSION,
        nonce,
        command_id: CommandId::TapDown.as_u8(),
        command: Vec::new(),
    };
    let context = AckTraceContext {
        run: 1,
        input_seen: 0,
        present_seen: 0,
        elapsed_ms: 0,
    };

    let first = state.authenticate(&packet, context);
    assert_eq!(first.kind, AckKind::Accepted);

    // The acknowledgement is lost in transit, so the sender sends it again.
    let retry = state.authenticate(&packet, context);
    assert_eq!(
        retry.kind,
        AckKind::RejectedReplay,
        "a lost acknowledgement must not cause the command to run twice"
    );

    let mut published = Vec::new();
    commit
        .publish_all(&mut published)
        .expect("publish the acknowledgement records");
    let records: Vec<TraceRecord> = String::from_utf8(published)
        .expect("utf8 trace")
        .lines()
        .map(TraceRecord::from_jsonl)
        .collect::<Result<Vec<_>, _>>()
        .expect("parse trace");

    let acknowledgements: Vec<&TraceRecord> = records
        .iter()
        .filter(|record| record.kind == RecordKind::CommandAcknowledgement)
        .collect();
    assert_eq!(
        acknowledgements.len(),
        2,
        "both the accepted command and the rejected retry must be recorded"
    );
    let outcomes: Vec<AckOutcome> = acknowledgements
        .iter()
        .map(|record| {
            record
                .command_acknowledgement
                .as_ref()
                .expect("acknowledgement payload")
                .outcome
        })
        .collect();
    assert_eq!(
        outcomes,
        vec![AckOutcome::Accepted, AckOutcome::RejectedReplay]
    );
}
