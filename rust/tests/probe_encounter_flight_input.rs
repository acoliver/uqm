//! Hyperspace flight in the probe-encounter scenarios must use player flight
//! keys and must prove movement against the live game position (issue #214).
//!
//! `tap_menu_key` writes `ImmediateInputState.menu[]`, which menus read.
//! Ship flight reads `key[PlayerControls[0]]`, so a menu-key hold flies
//! nothing. These tests read the scenario sources themselves.

use serde_json::Value;
use std::path::PathBuf;

const SCRIPTS: [&str; 7] = [
    "probe-encounter-v1",
    "probe-encounter-v2",
    "probe-encounter-v3",
    "probe-encounter-v4",
    "probe-encounter-v5",
    "probe-encounter-v6",
    "probe-encounter-v7",
];

/// Menu taps used for the game menu are a few ticks long; a flight leg holds
/// the key for at least this many ticks.
const FLIGHT_HOLD_TICKS: u64 = 100;

fn script_steps(name: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).expect("read scenario script");
    let document: Value = serde_json::from_str(&text).expect("scenario script is JSON");
    assert_eq!(document["name"], name, "script name is preserved");
    document["steps"].as_array().expect("steps array").clone()
}

fn is_flight_hold(step: &Value) -> bool {
    matches!(step["key"].as_str(), Some("up" | "down" | "left" | "right"))
        && step["hold"].as_u64().unwrap_or(0) >= FLIGHT_HOLD_TICKS
}

#[test]
fn long_flight_holds_are_player_keys_not_menu_keys() {
    for name in SCRIPTS {
        for (index, step) in script_steps(name).iter().enumerate() {
            let action = step["action"].as_str().unwrap_or_default();
            assert!(
                !(action == "tap_menu_key" && is_flight_hold(step)),
                "{name} step {index} holds menu key {} for {} ticks; flight reads player keys",
                step["key"],
                step["hold"]
            );
        }
    }
}

#[test]
fn flight_player_keys_are_thrust_or_turns_only() {
    for name in SCRIPTS {
        for (index, step) in script_steps(name).iter().enumerate() {
            if step["action"] != "tap_player_key" {
                continue;
            }
            assert!(
                matches!(step["key"].as_str(), Some("thrust" | "left" | "right")),
                "{name} step {index} uses player key {} which does not fly the ship",
                step["key"]
            );
            assert_eq!(step["value"], 1, "{name} step {index} presses the key");
        }
    }
}

#[test]
fn each_scenario_proves_movement_from_the_live_position() {
    for name in SCRIPTS {
        let steps = script_steps(name);
        let first_thrust = steps
            .iter()
            .position(|step| step["action"] == "tap_player_key" && step["key"] == "thrust")
            .unwrap_or_else(|| panic!("{name} never thrusts"));
        let recorded = steps[..first_thrust]
            .iter()
            .rposition(|step| step["action"] == "record_hyperspace_position")
            .unwrap_or_else(|| panic!("{name} records no position before its first thrust"));
        let asserted = steps[first_thrust..]
            .iter()
            .position(|step| step["action"] == "assert_hyperspace_displacement")
            .unwrap_or_else(|| panic!("{name} never asserts displacement after thrusting"))
            + first_thrust;
        assert!(recorded < first_thrust && first_thrust < asserted);
        assert!(
            steps[asserted]["minimum_logical_units"]
                .as_u64()
                .unwrap_or(0)
                > 0,
            "{name} displacement minimum must be positive"
        );
    }
}

#[test]
fn v3_and_v4_keep_their_hold_lengths_and_capture_order() {
    for (name, hold) in [("probe-encounter-v3", 2000), ("probe-encounter-v4", 3000)] {
        let steps = script_steps(name);
        let legs: Vec<&Value> = steps
            .iter()
            .filter(|step| step["action"] == "tap_player_key")
            .collect();
        assert_eq!(legs.len(), 4, "{name} keeps four flight legs");
        for leg in legs {
            assert_eq!(leg["hold"], hold, "{name} keeps its hold length");
            assert_eq!(leg["settle"], 5);
        }
        let captures: Vec<&str> = steps
            .iter()
            .filter(|step| step["action"] == "capture")
            .filter_map(|step| step["label"].as_str())
            .collect();
        assert_eq!(captures.first(), Some(&"new-game-confirmed"));
        assert_eq!(captures.last(), Some(&"probe-encounter-final"));
    }
}

#[test]
fn every_candidate_script_validates_and_binds_displacement_checkpoints() {
    use uqm_rust::automation::native_checkpoint::{CheckpointClass, NativeCheckpointPlan};
    use uqm_rust::automation::{parse_script, validate_script, Action};

    for name in SCRIPTS {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join(format!("{name}.json"));
        let bytes = std::fs::read(&path).expect("read script");
        let document = parse_script(&bytes, &path).expect("script parses");
        let script = validate_script(document, &path).expect("script validates");
        let plan = NativeCheckpointPlan::derive(&bytes, &script).expect("checkpoint plan");
        let displacement_checkpoints = plan
            .checkpoints
            .iter()
            .filter(|checkpoint| {
                matches!(checkpoint.action, Action::AssertHyperspaceDisplacement(_))
                    && checkpoint.class == CheckpointClass::Assertion
            })
            .count();
        let displacement_steps = script
            .steps()
            .iter()
            .filter(|action| matches!(action, Action::AssertHyperspaceDisplacement(_)))
            .count();
        assert!(displacement_steps >= 1, "{name} asserts displacement");
        assert_eq!(displacement_checkpoints, displacement_steps);
    }
}
