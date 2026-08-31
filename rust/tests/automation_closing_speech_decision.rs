//! Functional cover for the decision to skip a conversation's closing speech.
//!
//! Its counterpart in `automation_conversation_decisions` proves the answer is
//! no while the script is still selecting a response. This proves the answer is
//! yes once the script is waiting for the conversation to end, which is the
//! case that lets a scripted run reach the next scene instead of sitting
//! through the farewell.
//!
//! The coordinator is a process-wide singleton set once, so the two cases need
//! separate test binaries to hold different scripts.

use std::path::PathBuf;

use uqm_rust::automation::coordinator::Coordinator;
use uqm_rust::automation::script::{parse_script, validate_script};

const SCRIPT: &str = r#"{
  "version": 1,
  "name": "closing-speech-decision",
  "budgets": {
    "max_input_ticks": 1000,
    "max_presentations": 1000,
    "max_wallclock_seconds": 30
  },
  "steps": [
    {"action": "wait_for_communication_end", "minimum_completions": 1, "max_ticks": 50},
    {"action": "finish"}
  ]
}"#;

#[test]
fn waiting_for_the_conversation_to_end_skips_the_closing_speech() {
    let document = parse_script(SCRIPT.as_bytes(), "closing-speech-decision.json")
        .expect("the script fixture must parse");
    let script =
        validate_script(document, "closing-speech-decision.json").expect("and must validate");
    let output_root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("closing-speech-decision");
    std::fs::create_dir_all(&output_root).expect("evidence root");
    Coordinator::init(script, output_root);

    assert!(
        Coordinator::is_active(),
        "a validated script must activate the coordinator"
    );
    assert!(
        Coordinator::should_skip_communication_closing_speech(),
        "a wait-for-end step means the run is only waiting for the conversation to close"
    );
    // The same step is not a response selection, so nothing is pending there.
    assert!(
        !Coordinator::pending_communication_response_action(4),
        "a wait-for-end step selects no response, whatever the list offers"
    );
}
