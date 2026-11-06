//! Functional cover for the conversation decisions automation makes.
//!
//! The coordinator decides when a scripted run may skip NPC speech and when it
//! must wait. Getting that wrong does not crash anything: the run simply drifts
//! out of step with the conversation and the recorded evidence stops matching
//! the game. These decisions are therefore worth exercising against a real
//! script rather than trusting by inspection.
//!
//! This test owns its observer; its binary isolates production UI observations.

use std::path::PathBuf;

use uqm_rust::automation::coordinator::AutomationObserver;
use uqm_rust::automation::script::{parse_script, validate_script};

const SCRIPT: &str = r#"{
  "version": 1,
  "name": "conversation-decisions",
  "budgets": {
    "max_input_ticks": 1000,
    "max_presentations": 1000,
    "max_wallclock_seconds": 30
  },
  "steps": [
    {"action": "select_communication_response", "index": 1, "max_ticks": 50},
    {"action": "wait_for_communication_end", "minimum_completions": 1, "max_ticks": 50},
    {"action": "finish"}
  ]
}"#;

fn application_started() -> uqm_rust::app::Application {
    let document = parse_script(SCRIPT.as_bytes(), "conversation-decisions.json")
        .expect("the script fixture must parse");
    let script =
        validate_script(document, "conversation-decisions.json").expect("and must validate");
    let output_root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("conversation-decisions");
    std::fs::create_dir_all(&output_root).expect("evidence root");
    {
        let game = uqm_rust::engine::Game::new();
        let input = uqm_rust::engine::input::InputControls::default();
        let observer = Some(AutomationObserver::new(
            script,
            output_root,
            game.world.activity_control(),
            input.clone(),
            game.kernel.configuration.clone(),
        ));
        {
            let mut application = uqm_rust::app::Application::new(game, observer);
            application.install_input(input);
            application
        }
    }
}

#[test]
fn conversation_decisions_follow_the_script_step_in_hand() {
    battle_context::run(|_| {
        let _application = application_started();
        assert!(
            _application.observer().is_some(),
            "a validated script must activate the coordinator"
        );

        // The first step selects a response. Nothing has been observed yet, so
        // there is no pending list to skip speech for.
        assert!(
            !_application
                .observer()
                .unwrap()
                .should_skip_pending_communication_speech(),
            "with no observed responses there is nothing to skip toward"
        );

        // A response action exists in the chain, but only for a list that actually
        // offers the index the script asks for.
        assert!(
            _application
                .observer()
                .unwrap()
                .pending_communication_response_action(2),
            "a two-response list satisfies the scripted index 1"
        );
        assert!(
            !_application
                .observer()
                .unwrap()
                .pending_communication_response_action(1),
            "a one-response list does not offer index 1"
        );
        assert!(
            !_application
                .observer()
                .unwrap()
                .pending_communication_response_action(0),
            "an empty list offers nothing"
        );

        // The closing-speech decision belongs to the step in hand. The current step
        // selects a response, so the conversation is not being wound up yet.
        assert!(
            !_application
                .observer()
                .unwrap()
                .should_skip_communication_closing_speech(),
            "a response step is not a wait-for-end step"
        );
    });
}

#[path = "support/battle_context.rs"]
mod battle_context;
