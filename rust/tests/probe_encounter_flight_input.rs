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

/// The scripts that fly more hyperspace than the opening tank allows.
const PREPARED: [&str; 2] = ["probe-encounter-v3", "probe-encounter-v4"];

fn action_of(step: &Value) -> &str {
    step["action"].as_str().unwrap_or_default()
}

fn is_tap(step: &Value, key: &str) -> bool {
    action_of(step) == "tap_menu_key" && step["key"] == key
}

fn resources(step: &Value) -> Option<(Option<u64>, Option<u64>)> {
    (action_of(step) == "assert_ship_resources")
        .then(|| (step["fuel_on_board"].as_u64(), step["res_units"].as_u64()))
}

fn position(steps: &[Value], from: usize, wanted: impl Fn(&Value) -> bool) -> usize {
    steps[from..]
        .iter()
        .position(wanted)
        .map(|offset| offset + from)
        .expect("expected step is missing")
}

/// Everything before the first flight step: the preparation.
fn prep_of(name: &str) -> Vec<Value> {
    let steps = script_steps(name);
    let fly = position(&steps, 0, |step| {
        action_of(step) == "navigate_to_hyperspace"
    });
    steps[..fly].to_vec()
}

#[test]
fn prepared_flights_start_from_the_real_opening_state_and_end_with_a_full_tank() {
    for name in PREPARED {
        let steps = script_steps(name);
        let fly = position(&steps, 0, |step| {
            action_of(step) == "navigate_to_hyperspace"
        });
        let new_game = position(&steps, 0, |step| action_of(step) == "assert_activity");
        let all: Vec<_> = steps[..fly]
            .iter()
            .enumerate()
            .filter_map(|(index, step)| resources(step).map(|found| (index, found)))
            .collect();
        assert_eq!(
            all.first().map(|(_, found)| *found),
            Some((Some(1000), Some(0))),
            "{name} must first prove the new-game tank and zero RU"
        );
        assert!(all[0].0 > new_game, "{name} checks resources in a new game");
        assert_eq!(
            all.last().map(|(_, found)| *found),
            Some((Some(6000), Some(1500))),
            "{name} must prove 6000 raw fuel and 1500 RU right before it flies"
        );
        assert_eq!(
            fly - all.last().unwrap().0,
            1,
            "{name} flies immediately after the last resource proof"
        );
    }
}

#[test]
fn the_reward_is_proven_between_the_commander_conversation_and_any_purchase() {
    for name in PREPARED {
        let steps = prep_of(name);
        let reward = position(&steps, 0, |step| {
            resources(step) == Some((Some(1000), Some(2500)))
        });
        let docked = steps[..reward]
            .iter()
            .rposition(|step| action_of(step) == "assert_mode" && step["mode"] == "starbase")
            .unwrap_or_else(|| panic!("{name} never proves the Starbase menu is active"));
        let farewell = steps[..docked]
            .iter()
            .rposition(|step| action_of(step) == "select_communication_response")
            .expect("goodbye response");
        assert_eq!(
            steps[farewell]["index"], 1,
            "{name} says goodbye to the Commander"
        );
        let first_purchase = position(&steps, 0, |step| is_tap(step, "up"));
        assert!(
            reward < first_purchase,
            "{name} must hold the 2500 RU reward before buying fuel"
        );
        // The reward is only paid when the Commander greets the moon base, so
        // the visit that pays it must come after the Luna pickup visit.
        let pickup = position(&steps, 0, |step| {
            action_of(step) == "wait_for_planet_side_end" && step["outcome"] == "returned"
        });
        let commander_visits: Vec<usize> = steps
            .iter()
            .enumerate()
            .filter(|(_, step)| action_of(step) == "wait_for_dispatch" && step["encounter"] == 2)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(
            commander_visits.len(),
            2,
            "{name} visits the Commander twice"
        );
        assert!(pickup < commander_visits[0] && commander_visits[1] < reward);
    }
}

#[test]
fn fifty_unit_purchases_buy_five_thousand_raw_fuel_for_one_thousand_ru() {
    for name in PREPARED {
        let steps = prep_of(name);
        let enter = position(&steps, 0, |step| {
            resources(step) == Some((Some(1000), Some(2500)))
        });
        let leave = position(&steps, enter, |step| {
            resources(step) == Some((Some(6000), Some(1500)))
        });
        let purchases = steps[enter..leave]
            .iter()
            .filter(|step| is_tap(step, "up"))
            .count();
        assert_eq!(purchases, 50, "{name}: +100 raw per press, 20 RU per press");
        assert_eq!(1000 + purchases * 100, 6000);
        assert_eq!(2500 - purchases * 20, 1500);
        // Nothing other than Outfit navigation and purchases happens in between.
        for step in &steps[enter + 1..leave] {
            let ordinary = action_of(step) == "tap_menu_key"
                && matches!(step["key"].as_str(), Some("up" | "down" | "select"));
            assert!(
                ordinary,
                "{name}: unexpected step between reward and purchase proof: {step}"
            );
        }
    }
}

#[test]
fn preparation_uses_only_player_and_menu_inputs_and_queries() {
    const ALLOWED: [&str; 18] = [
        "wait_for_main_menu_ready",
        "tap_menu_key",
        "set_menu_key",
        "wait_input_ticks",
        "capture",
        "assert_activity",
        "assert_dispatch",
        "assert_mode",
        "assert_communication_responses",
        "assert_ship_resources",
        "navigate_to_planet",
        "navigate_to_moon",
        "select_planet_menu",
        "wait_for_planet_side_start",
        "wait_for_planet_side_end",
        "wait_for_dispatch",
        "wait_for_communication_end",
        "select_communication_response",
    ];
    for name in PREPARED {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join(format!("{name}.json"));
        let document: Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(
            document.get("start_scene").is_none(),
            "{name} must not start from a scene that skips the pickup"
        );
        for step in prep_of(name) {
            assert!(
                ALLOWED.contains(&action_of(&step)),
                "{name}: {} is not a plain input or query",
                action_of(&step)
            );
        }
    }
}

#[test]
fn the_flight_after_preparation_is_the_original_flight() {
    let expected: [(&str, u64, &[&str]); 2] = [
        (
            "probe-encounter-v3",
            2000,
            &[
                "new-game-confirmed",
                "hyperspace-start",
                "fly-up-long-1",
                "fly-right-long-1",
                "fly-up-long-2",
                "fly-right-long-2",
                "probe-encounter-final",
            ],
        ),
        (
            "probe-encounter-v4",
            3000,
            &[
                "new-game-confirmed",
                "hyperspace-start",
                "fly-down-long",
                "fly-left-long",
                "fly-down-long-2",
                "fly-left-long-2",
                "probe-encounter-final",
            ],
        ),
    ];
    for (name, hold, captures) in expected {
        let steps = script_steps(name);
        let labels: Vec<&str> = steps
            .iter()
            .filter(|step| action_of(step) == "capture")
            .filter_map(|step| step["label"].as_str())
            .collect();
        assert_eq!(
            labels, captures,
            "{name} keeps exactly its original captures"
        );
        let fly = position(&steps, 0, |step| {
            action_of(step) == "navigate_to_hyperspace"
        });
        let flight_taps: Vec<&Value> = steps[fly..]
            .iter()
            .filter(|step| action_of(step) == "tap_player_key")
            .collect();
        assert_eq!(flight_taps.len(), 4);
        assert!(flight_taps.iter().all(|tap| tap["hold"] == hold));
        let displacement = steps[fly..]
            .iter()
            .filter(|step| action_of(step) == "assert_hyperspace_displacement")
            .count();
        assert_eq!(displacement, 2, "{name} keeps both displacement proofs");
        // The preparation adds no capture of its own.
        assert!(prep_of(name)
            .iter()
            .all(|step| action_of(step) != "capture" || {
                matches!(step["label"].as_str(), Some("new-game-confirmed"))
            }));
    }
}

#[test]
fn the_wall_clock_watchdog_is_not_raised_to_fit_the_preparation() {
    for name in PREPARED {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join(format!("{name}.json"));
        let document: Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(document["budgets"]["max_wallclock_seconds"], 720, "{name}");
        let ticks = document["budgets"]["max_input_ticks"].as_u64().unwrap();
        let presentations = document["budgets"]["max_presentations"].as_u64().unwrap();
        assert_eq!(
            presentations,
            2 * ticks,
            "{name} keeps two presentations per tick"
        );
    }
}

#[test]
fn both_prepared_scripts_share_one_identical_preparation() {
    let v3 = prep_of("probe-encounter-v3");
    let v4 = prep_of("probe-encounter-v4");
    assert_eq!(v3, v4);
}
