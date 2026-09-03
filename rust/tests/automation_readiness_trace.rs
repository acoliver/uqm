//! Functional cover for main-menu readiness evidence.
//!
//! When the scheduler observes `MainMenuReady`, the run must retain a
//! readiness record carrying what became ready and the production
//! observation that proved it. Before schema 2 this observation was folded
//! into a labelled semantic assertion, so readiness was indistinguishable
//! from any other assertion in retained evidence.
//!
//! The coordinator is a process-wide singleton, so this lives in its own
//! test binary where it can be initialised once and driven deterministically.

use std::path::PathBuf;

use uqm_rust::automation::coordinator::{
    Coordinator, MAIN_MENU_READINESS_OBSERVATION, MAIN_MENU_READINESS_SUBJECT,
};
use uqm_rust::automation::script::{parse_script, validate_script};
use uqm_rust::automation::trace::{RecordKind, TraceRecord};

const SCRIPT: &str = r#"{
  "version": 1,
  "name": "readiness-trace",
  "budgets": {
    "max_input_ticks": 1000,
    "max_presentations": 1000,
    "max_wallclock_seconds": 30
  },
  "steps": [
    {"action": "wait_for_main_menu_ready"},
    {"action": "finish"}
  ]
}"#;

fn coordinator_started() -> bool {
    let document = parse_script(SCRIPT.as_bytes(), "readiness-trace.json")
        .expect("the script fixture must parse");
    let script = validate_script(document, "readiness-trace.json").expect("and must validate");
    let output_root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("readiness-trace");
    std::fs::create_dir_all(&output_root).expect("evidence root");
    Coordinator::init(script, output_root);
    Coordinator::is_active()
}

#[test]
fn main_menu_ready_emits_readiness_record() {
    assert!(
        coordinator_started(),
        "a validated script must activate the coordinator"
    );

    assert!(
        !Coordinator::process_main_menu_ready(),
        "readiness advances to the non-terminal finish step"
    );

    let mut sink = Vec::new();
    uqm_rust::automation::input_ffi::get_runtime()
        .expect("the coordinator runtime is initialized")
        .commit
        .publish_all(&mut sink)
        .expect("the ordered commit publishes every submitted record");
    let text = String::from_utf8(sink).expect("the trace is UTF-8 JSONL");
    let records: Vec<TraceRecord> = text
        .lines()
        .map(TraceRecord::from_jsonl)
        .collect::<Result<_, _>>()
        .expect("every trace line parses as a schema-2 record");

    assert_eq!(
        records
            .iter()
            .map(|record| &record.kind)
            .collect::<Vec<_>>(),
        vec![&RecordKind::RunStart, &RecordKind::Readiness],
        "launch is followed by readiness evidence"
    );
    for record in &records {
        assert_eq!(record.schema, TraceRecord::SCHEMA);
        assert_eq!(record.run, 1);
    }
    let readiness = records[1]
        .readiness
        .as_ref()
        .expect("the readiness record carries its payload");
    assert_eq!(readiness.subject, MAIN_MENU_READINESS_SUBJECT);
    assert_eq!(readiness.observation, MAIN_MENU_READINESS_OBSERVATION);
    assert!(
        !readiness.observation.is_empty(),
        "readiness must name the observation that proved it"
    );
}
