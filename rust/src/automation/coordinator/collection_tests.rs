//! The coordinator audits the production game queues at native checkpoints
//! and on input ticks. Each test builds a coordinator, which is process-global,
//! so it re-runs itself in a child process.

use super::*;
use crate::automation::collection_invariants::{validate_label, LabelVerdict, SelectedQueue};
use crate::collections::queue::{Link, Queue};

/// The five `GAME_STATE` queues the injected sampler reads.
struct GameQueues([Box<Queue>; 5]);

// SAFETY: the queues are only reached through `GAME_QUEUES`, whose mutex
// serialises every access.
unsafe impl Send for GameQueues {}

static GAME_QUEUES: Mutex<Option<GameQueues>> = Mutex::new(None);

fn index(which: SelectedQueue) -> usize {
    SelectedQueue::ALL.iter().position(|q| *q == which).unwrap()
}

fn with_queue<R>(which: SelectedQueue, action: impl FnOnce(&mut Queue) -> R) -> R {
    let mut guard = GAME_QUEUES.lock();
    action(&mut guard.as_mut().unwrap().0[index(which)])
}

fn put_link(which: SelectedQueue) -> *mut Link {
    with_queue(which, |queue| {
        let link = queue.alloc_link();
        assert!(!link.is_null());
        // SAFETY: `link` was just allocated from this queue.
        unsafe { queue.put(link) };
        link.cast()
    })
}

fn sample_game_queues() -> CollectionSnapshot {
    let guard = GAME_QUEUES.lock();
    let queues = &guard.as_ref().unwrap().0;
    CollectionSnapshot::observe(|which| &queues[index(which)])
}

fn run_in_child(test: &str, marker: &str) -> bool {
    if std::env::var_os(marker).is_some() {
        *GAME_QUEUES.lock() = Some(GameQueues(std::array::from_fn(|_| Box::new(Queue::new()))));
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("automation::coordinator::collection_tests::{test}"),
            "--nocapture",
        ])
        .env(marker, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

const TWO_CHECKPOINTS: &[u8] = br#"{"version":1,"name":"collections","budgets":{"max_input_ticks":100,"max_presentations":100,"max_wallclock_seconds":60},"steps":[{"action":"wait_for_main_menu_ready"},{"action":"wait_for_main_menu_ready"},{"action":"finish"}]}"#;

fn start(
    temp: &tempfile::TempDir,
) -> (
    super::super::native_checkpoint::NativeCheckpointPlan,
    &'static Coordinator,
) {
    let script = crate::automation::validate_script(
        crate::automation::parse_script(TWO_CHECKPOINTS, "test.json").unwrap(),
        "test.json",
    )
    .unwrap();
    let plan =
        super::super::native_checkpoint::NativeCheckpointPlan::derive(TWO_CHECKPOINTS, &script)
            .unwrap();
    Coordinator::init_with_collection_source(
        script,
        temp.path().into(),
        Some(65536),
        sample_game_queues,
    );
    (plan, Coordinator::get().unwrap())
}

fn frame(count: u64) -> crate::automation::capture::PresentedFrame {
    crate::automation::capture::PresentedFrame {
        count,
        generation: 0,
        width: 1,
        height: 1,
        rgba: vec![1, 2, 3, 255],
    }
}

fn read_trace(temp: &tempfile::TempDir) -> Vec<TraceRecord> {
    std::fs::read_to_string(temp.path().join("trace.jsonl"))
        .unwrap()
        .lines()
        .map(|line| TraceRecord::from_jsonl(line).unwrap())
        .collect()
}

fn collection_labels(records: &[TraceRecord]) -> Vec<&str> {
    records
        .iter()
        .filter_map(|record| record.label.as_deref())
        .filter(|label| label.starts_with("collection_invariant"))
        .collect()
}

#[test]
fn each_checkpoint_records_the_queue_audit_taken_at_that_checkpoint() {
    if run_in_child(
        "each_checkpoint_records_the_queue_audit_taken_at_that_checkpoint",
        "UQM_COLLECTION_CHECKPOINT_CHILD",
    ) {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let (plan, coord) = start(&temp);
    assert!(!Coordinator::process_main_menu_ready());
    assert!(!Coordinator::process_main_menu_ready());

    // Before game initialisation every queue is a zeroed C global.
    assert!(!Coordinator::process_present(Some(frame(1))));
    // The game then builds its pools and uses them.
    with_queue(SelectedQueue::Encounter, |queue| {
        assert!(queue.init(3, 24));
    });
    put_link(SelectedQueue::Encounter);
    put_link(SelectedQueue::Encounter);
    assert!(!coord.process_input_inner());
    assert!(!Coordinator::process_present(Some(frame(2))));
    assert!(coord.process_input_inner());
    assert_eq!(Coordinator::finalize(0).unwrap(), 0);

    let records = read_trace(&temp);
    assert_eq!(
        collection_labels(&records),
        [
            "collection_invariants:avail_race_q=absent:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=absent:built_ship_q=absent",
            "collection_invariants:avail_race_q=absent:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=intact/3/2/1/0:built_ship_q=absent",
        ]
    );
    for (position, record) in records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.kind == RecordKind::Checkpoint)
    {
        let audit = &records[position - 1];
        assert_eq!(audit.kind, RecordKind::SemanticAssertion);
        assert_eq!(
            validate_label(audit.label.as_deref().unwrap()),
            Ok(LabelVerdict::Intact)
        );
        assert!(record.presentation.is_some());
    }
    assert_eq!(plan.committed(&records).unwrap().len(), 2);
}

#[test]
fn a_broken_queue_at_a_checkpoint_fails_the_run_before_the_checkpoint_commits() {
    if run_in_child(
        "a_broken_queue_at_a_checkpoint_fails_the_run_before_the_checkpoint_commits",
        "UQM_COLLECTION_CHECKPOINT_FAULT_CHILD",
    ) {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let (plan, coord) = start(&temp);
    assert!(!Coordinator::process_main_menu_ready());
    with_queue(SelectedQueue::IpGroup, |queue| {
        assert!(queue.init(4, 24));
    });
    let first = put_link(SelectedQueue::IpGroup);
    let second = put_link(SelectedQueue::IpGroup);
    // SAFETY: both links are slots of the pool; this models C corrupting a
    // `succ` field.
    unsafe { (*second).succ = first.cast() };

    assert!(Coordinator::process_present(Some(frame(1))));
    assert_eq!(
        coord.inner.lock().terminal_class,
        Some(TerminalClass::SemanticMismatch)
    );
    let _ = Coordinator::finalize(0);

    let records = read_trace(&temp);
    assert_eq!(
        collection_labels(&records),
        ["collection_invariant_failed:queue=ip_group_q:fault=cycle:slot=0"]
    );
    assert!(records
        .iter()
        .all(|record| record.kind != RecordKind::Checkpoint));
    assert_eq!(plan.committed(&records).unwrap().len(), 0);
}

#[test]
fn an_input_tick_stops_on_a_broken_queue_and_stays_silent_while_intact() {
    if run_in_child(
        "an_input_tick_stops_on_a_broken_queue_and_stays_silent_while_intact",
        "UQM_COLLECTION_TICK_CHILD",
    ) {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let (_, coord) = start(&temp);
    with_queue(SelectedQueue::BuiltShip, |queue| {
        assert!(queue.init(2, 24));
    });
    let link = put_link(SelectedQueue::BuiltShip);
    assert!(!coord.process_input_inner());
    assert!(!coord.process_input_inner());
    assert_eq!(coord.inner.lock().terminal_class, None);

    // SAFETY: `link` is a slot of the pool; this models C corrupting `pred`.
    unsafe { (*link).pred = link.cast() };
    assert!(coord.process_input_inner());
    assert_eq!(
        coord.inner.lock().terminal_class,
        Some(TerminalClass::SemanticMismatch)
    );
    let _ = Coordinator::finalize(0);

    let records = read_trace(&temp);
    assert_eq!(
        collection_labels(&records),
        ["collection_invariant_failed:queue=built_ship_q:fault=broken_pred:slot=0"]
    );
}
