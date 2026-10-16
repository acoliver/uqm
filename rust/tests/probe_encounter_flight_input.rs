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
fn flight_player_keys_are_thrust_only_because_turns_are_verified_actions() {
    for name in SCRIPTS {
        for (index, step) in script_steps(name).iter().enumerate() {
            if step["action"] != "tap_player_key" {
                continue;
            }
            assert_eq!(
                step["key"], "thrust",
                "{name} step {index} taps player key {}; a turn is a turn_flagship step",
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

/// How many ticks a `turn_flagship` step may take before it fails. A turn of
/// four facings took about 50 input ticks when measured in a real window, so
/// this is more than twice that.
const MAX_TURN_TICKS: u64 = 120;

/// A flight leg is one `thrust` tap. Each is bracketed by a position snapshot
/// immediately before it and a displacement proof immediately after it, so no
/// scenario can pass while the ship sits against the universe edge.
#[test]
fn every_thrust_leg_is_bracketed_by_a_position_record_and_a_displacement_proof() {
    for name in SCRIPTS {
        let steps = script_steps(name);
        let mut legs = 0;
        for (index, step) in steps.iter().enumerate() {
            if action_of(step) != "tap_player_key" || step["key"] != "thrust" {
                continue;
            }
            legs += 1;
            assert!(
                index > 0 && action_of(&steps[index - 1]) == "record_hyperspace_position",
                "{name} step {index}: a thrust leg must follow record_hyperspace_position"
            );
            let proof = steps
                .get(index + 1)
                .unwrap_or_else(|| panic!("{name} ends on a thrust leg"));
            assert_eq!(
                action_of(proof),
                "assert_hyperspace_displacement",
                "{name} step {index}: a thrust leg must be followed by a displacement proof"
            );
            assert!(proof["minimum_logical_units"].as_u64().unwrap_or(0) >= 16);
        }
        assert!(legs >= 3, "{name} flies {legs} legs");
    }
}

/// A key hold cannot say how far the ship turned: 60 input ticks turned the
/// flagship five facings (112.5 degrees), not four. A `turn_flagship` step
/// holds the key until the live `STARSHIP.ShipFacing` has changed by exactly
/// the stated number of facings and fails otherwise, so a right angle is four
/// facings of sixteen and every turn in every scenario is that proof.
#[test]
fn every_turn_is_a_turn_flagship_step_that_proves_a_right_angle() {
    for name in SCRIPTS {
        let steps = script_steps(name);
        let mut turns = 0;
        for (index, step) in steps.iter().enumerate() {
            if action_of(step) != "turn_flagship" {
                continue;
            }
            turns += 1;
            assert_eq!(step["direction"], "right", "{name} step {index}");
            assert_eq!(step["facings"], 4, "{name} step {index}: a right angle");
            let max_ticks = step["max_ticks"].as_u64().unwrap_or(0);
            assert!(
                (1..=MAX_TURN_TICKS).contains(&max_ticks),
                "{name} step {index}: bounded by {MAX_TURN_TICKS} ticks, not {max_ticks}"
            );
        }
        assert!(turns >= 2, "{name} turns {turns} times");
    }
}

/// Without fuel the flagship cannot thrust (`sis_ship.c`), and without room
/// the leg ends at the universe edge. Sol sits about 27 800 logical units from
/// the nearest edges and the flagship covers about 12 per thrust tick, so the
/// flight's total thrust is bounded: 1 000 raw fuel pays for about 1 100 ticks
/// (measured 0.009 HUD fuel per thrust tick), 6 000 raw fuel for far more than
/// the 2 200 ticks that also keep a straight run inside the margin.
#[test]
fn total_thrust_stays_inside_the_fuel_and_the_edge_margin() {
    for name in SCRIPTS {
        let total: u64 = script_steps(name)
            .iter()
            .filter(|step| action_of(step) == "tap_player_key" && step["key"] == "thrust")
            .map(|step| step["hold"].as_u64().unwrap_or(0))
            .sum();
        let limit = if PREPARED.contains(&name) { 2200 } else { 800 };
        assert!(
            total <= limit,
            "{name} thrusts {total} ticks, over its {limit} tick limit"
        );
    }
}

/// A square of four legs and three right angles closes on its start. Sol's
/// hyperspace point is the start, so the fourth leg ends there and the game
/// enters the Sol system (a real-window run of v3 closed to within 76 logical
/// units on each axis). The scripts end on that arrival and assert it instead of turning
/// again, which fails with `flagship_turn_wrong_mode:observed=Solar`.
const SQUARE_SCRIPTS: [&str; 5] = [
    "probe-encounter-v1",
    "probe-encounter-v2",
    "probe-encounter-v3",
    "probe-encounter-v4",
    "probe-encounter-v5",
];

/// IN_INTERPLANETARY, the activity of the Sol system.
const IN_INTERPLANETARY: u64 = 4;

/// Ticks given to the hyperspace to Solar transition after the fourth leg.
const RETURN_WAIT_TICKS: u64 = 100;

fn square_leg_holds() -> [(&'static str, u64); 5] {
    [
        ("probe-encounter-v1", 100),
        ("probe-encounter-v2", 80),
        ("probe-encounter-v3", 500),
        ("probe-encounter-v4", 550),
        ("probe-encounter-v5", 50),
    ]
}

#[test]
fn v1_to_v5_fly_a_four_leg_square_with_three_right_angle_turns() {
    for (name, hold) in square_leg_holds() {
        let steps = script_steps(name);
        let legs: Vec<&Value> = steps
            .iter()
            .filter(|step| matches!(action_of(step), "tap_player_key" | "turn_flagship"))
            .collect();
        assert_eq!(legs.len(), 7, "{name} keeps four legs and three turns");
        let mut turned = 0;
        for (index, leg) in legs.iter().enumerate() {
            if index % 2 == 0 {
                assert_eq!(action_of(leg), "tap_player_key");
                assert_eq!(leg["key"], "thrust");
                assert_eq!(leg["hold"], hold, "{name} keeps its leg length");
            } else {
                assert_eq!(action_of(leg), "turn_flagship");
                assert_eq!(leg["direction"], "right", "{name} turns the same way");
                turned += leg["facings"].as_u64().unwrap();
            }
        }
        assert_eq!(
            turned, 12,
            "{name}: three right angles are twelve of sixteen facings; a fourth \
             would be flown from Sol, where the ship is no longer in hyperspace"
        );
    }
}

#[test]
fn v1_to_v5_end_by_asserting_the_return_to_sol() {
    for name in SQUARE_SCRIPTS {
        let steps = script_steps(name);
        let last_leg = steps
            .iter()
            .rposition(|step| action_of(step) == "tap_player_key")
            .expect("last thrust leg");
        assert_eq!(
            action_of(&steps[last_leg + 1]),
            "assert_hyperspace_displacement",
            "{name}: the fourth leg is still proven"
        );
        let tail: Vec<&Value> = steps[last_leg + 2..]
            .iter()
            .filter(|step| action_of(step) != "capture")
            .collect();
        let tail_actions: Vec<&str> = tail.iter().map(|step| action_of(step)).collect();
        assert_eq!(
            tail_actions,
            [
                "wait_input_ticks",
                "assert_activity",
                "wait_input_ticks",
                "finish"
            ],
            "{name}: after the fourth leg it waits, asserts Sol, waits, finishes"
        );
        assert_eq!(tail[0]["count"], RETURN_WAIT_TICKS, "{name}");
        assert_eq!(tail[1]["mask"], 255, "{name}");
        assert_eq!(tail[1]["equals"], IN_INTERPLANETARY, "{name}");
        let after_leg: Vec<&str> = steps[last_leg + 2..]
            .iter()
            .filter_map(|step| step["label"].as_str())
            .collect();
        assert!(
            after_leg.ends_with(&["returned-to-sol", "probe-encounter-final"]),
            "{name}: {after_leg:?}"
        );
        let asserted = steps[last_leg..]
            .iter()
            .position(|step| action_of(step) == "assert_activity")
            .unwrap()
            + last_leg;
        let returned = steps[last_leg..]
            .iter()
            .position(|step| step["label"] == "returned-to-sol")
            .unwrap()
            + last_leg;
        assert!(asserted < returned, "{name}: the capture follows the proof");
        assert!(
            steps[last_leg..]
                .iter()
                .all(|step| action_of(step) != "turn_flagship"),
            "{name} does not turn after the fourth leg"
        );
    }
}

/// The flagship menu of hyperspace has no Cancel handler (`DoHyperspaceMenu`,
/// hyper.c): Cancel opens it and only selecting NAVIGATION (or the star map)
/// returns to flight. With the default PC menu, Up from STARMAP wraps to
/// NAVIGATION (`PreviousMenuState`, menu.c), and Down from GAME_MENU reaches it.
/// Every visit in v6 and v7 therefore ends with a Select on NAVIGATION, and
/// the next thrust leg proves flight resumed.
#[test]
fn every_flagship_menu_visit_leaves_through_navigation() {
    for name in ["probe-encounter-v6", "probe-encounter-v7"] {
        let steps = script_steps(name);
        let mut visits = 0;
        let mut index = 0;
        while index < steps.len() {
            if !is_tap(&steps[index], "cancel") {
                index += 1;
                continue;
            }
            visits += 1;
            let mut cursor = index + 1;
            let mut keys = Vec::new();
            while cursor < steps.len() && action_of(&steps[cursor]) != "record_hyperspace_position"
            {
                if action_of(&steps[cursor]) == "tap_menu_key" {
                    keys.push(steps[cursor]["key"].as_str().unwrap().to_string());
                }
                if action_of(&steps[cursor]) == "turn_flagship"
                    || action_of(&steps[cursor]) == "wait_input_ticks"
                {
                    break;
                }
                cursor += 1;
            }
            let expected: &[&str] = if name.ends_with("v6") {
                &["up", "select"]
            } else {
                &["up", "up", "select", "cancel", "down", "select"]
            };
            assert_eq!(keys, expected, "{name} visit {visits}");
            if name.ends_with("v7") {
                assert!(
                    steps[index..cursor]
                        .iter()
                        .any(|step| action_of(step) == "assert_game_options"),
                    "{name} visit {visits} proves Game options opened"
                );
            }
            index = cursor;
        }
        assert_eq!(visits, 5, "{name} visits the flagship menu five times");
        assert!(
            steps.iter().all(|step| !step["label"]
                .as_str()
                .is_some_and(|label| label.starts_with("after-cancel"))),
            "{name}: a Cancel does not leave the menu, so no capture may say it did"
        );
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
            resources(step) == Some(post_landing_state(name))
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
            resources(step) == Some(post_landing_state(name))
        });
        let leave = position(&steps, enter, |step| {
            resources(step) == Some((Some(6000), Some(1500)))
        });
        let purchases = steps[enter..leave]
            .iter()
            .filter(|step| is_tap(step, "up"))
            .count();
        assert_eq!(purchases, 50, "{name}: +100 raw per press, 20 RU per press");
        // Outfit entry rounds the post-landing fuel to a whole unit first.
        let (landed_fuel, reward_ru) = post_landing_state(name);
        let rounded = round_fuel_on_outfit_entry(u32::try_from(landed_fuel.unwrap()).unwrap())
            .expect("rounds");
        assert_eq!(rounded as usize + purchases * 100, 6000);
        assert_eq!(reward_ru.unwrap() as usize - purchases * 20, 1500);
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
    const ALLOWED: [&str; 19] = [
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
        "wait_for_communication_responses",
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
fn the_flight_after_preparation_is_the_square_route() {
    let captures: &[&str] = &[
        "new-game-confirmed",
        "hyperspace-start",
        "leg-1",
        "turn-1",
        "leg-2",
        "turn-2",
        "leg-3",
        "turn-3",
        "leg-4",
        "returned-to-sol",
        "probe-encounter-final",
    ];
    for name in PREPARED {
        let steps = script_steps(name);
        let labels: Vec<&str> = steps
            .iter()
            .filter(|step| action_of(step) == "capture")
            .filter_map(|step| step["label"].as_str())
            .collect();
        assert_eq!(labels, captures, "{name} keeps exactly its route captures");
        let fly = position(&steps, 0, |step| {
            action_of(step) == "navigate_to_hyperspace"
        });
        let flight_taps: Vec<&Value> = steps[fly..]
            .iter()
            .filter(|step| action_of(step) == "tap_player_key")
            .collect();
        assert_eq!(flight_taps.len(), 4, "four thrust legs");
        let turns = steps[fly..]
            .iter()
            .filter(|step| action_of(step) == "turn_flagship")
            .count();
        assert_eq!(turns, 3, "{name} proves every turn");
        let displacement = steps[fly..]
            .iter()
            .filter(|step| action_of(step) == "assert_hyperspace_displacement")
            .count();
        assert_eq!(displacement, 4, "{name} proves every leg");
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

/// The second Commander dispatch only starts the conversation: the NPC
/// callback registers its response choices afterwards, while speech plays.
/// The immediate response-count assertion must therefore follow a bounded
/// wait for the current conversation's choices, and the assertion itself must
/// stay exactly as strict as before.
#[test]
fn the_second_commander_response_assertion_follows_a_bounded_response_wait() {
    for name in PREPARED {
        let steps = prep_of(name);
        let dispatches: Vec<usize> = steps
            .iter()
            .enumerate()
            .filter(|(_, step)| action_of(step) == "wait_for_dispatch" && step["encounter"] == 2)
            .map(|(index, _)| index)
            .collect();
        let second_dispatch = dispatches[1];
        let assertion = position(&steps, second_dispatch, |step| {
            action_of(step) == "assert_communication_responses"
        });
        assert_eq!(
            steps[assertion],
            serde_json::json!({"action": "assert_communication_responses", "minimum": 2}),
            "{name} keeps the original assertion unchanged"
        );
        assert_eq!(
            assertion - second_dispatch,
            2,
            "{name}: exactly one step separates the dispatch from the assertion"
        );
        let wait = &steps[second_dispatch + 1];
        assert_eq!(
            action_of(wait),
            "wait_for_communication_responses",
            "{name}"
        );
        assert_eq!(wait["minimum"], 2, "{name} waits for as many as it asserts");
        let max_ticks = wait["max_ticks"].as_u64().expect("bounded wait");
        assert!((1..=2000).contains(&max_ticks), "{name}: bounded wait");
        assert_eq!(
            steps[assertion + 1]["action"],
            "select_communication_response",
            "{name} still chooses only after the assertion"
        );
        assert_eq!(steps[assertion + 1]["index"], 1, "{name} says goodbye");
    }
}

// ---------------------------------------------------------------------------
// Resource conservation of the preparation (issue #214).
//
// The expected fuel and RU of every `assert_ship_resources` in the
// preparation are derived here from the game's own rules and data: the
// opening tank and fuel price in `globdata.c`, Luna's density and radius in
// `gensol.c`, the landing cost and Outfit rounding of the Rust port, and the
// Commander's reward in `starbas.c`. Nothing below copies an observed value.
// ---------------------------------------------------------------------------

use uqm_rust::planet_side::selection::landing_fuel_needed;
use uqm_rust::starbase::sis_modules::{FUEL_RESERVE, FUEL_TANK_CAPACITY, FUEL_TANK_SCALE};
use uqm_rust::starbase::{
    change_fuel_quantity, round_fuel_on_outfit_entry, FuelChange, FuelKey, FuelRuState,
};

fn c_source(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("sc2/src/uqm")
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"))
}

/// The unsigned integer that directly follows `needle` in `text`.
fn number_after(text: &str, needle: &str) -> u32 {
    let start = text
        .find(needle)
        .unwrap_or_else(|| panic!("source no longer contains {needle:?}"))
        + needle.len();
    let digits: String = text[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("no number after {needle:?}"))
}

/// The state the preparation must be in once the Starbase menu is up: the
/// opening tank minus exactly one Luna landing, plus the Commander's reward.
struct DerivedPreparation {
    opening_fuel: u32,
    opening_res_units: u32,
    fuel_price: u8,
    landing_cost: u32,
    reward: u32,
}

impl DerivedPreparation {
    fn from_game_data(opening_res_units: u32) -> Self {
        let globdata = c_source("globdata.c");
        let opening_fuel = number_after(&globdata, "GLOBAL_SIS (FuelOnBoard) = ") * FUEL_TANK_SCALE;
        let fuel_price = u8::try_from(number_after(&globdata, "GLOBAL (FuelCost) = "))
            .expect("fuel price fits a byte");

        // Luna is the second moon of Earth; `gensol.c` fixes its density and
        // radius, and `CalcGravity` (`calc.c`) is density * radius / 100.
        let gensol = c_source("planets/generate/gensol.c");
        let moon_properties = &gensol[gensol
            .find("// World is a moon.")
            .expect("moon properties block")..];
        let earth_moons = &moon_properties[moon_properties
            .find("case 2: /* moons of EARTH */")
            .expect("Earth moons block")..];
        let earth_moons = &earth_moons[..earth_moons
            .find("case 4: /* moons of JUPITER */")
            .expect("Jupiter moons block")];
        let density = number_after(earth_moons, "PlanetInfo.PlanetDensity = ");
        let radius = number_after(earth_moons, "PlanetInfo.PlanetRadius = ");
        let gravity = u16::try_from(density * radius / 100).expect("gravity fits");
        let landing_cost = u32::from(landing_fuel_needed(
            gravity,
            u16::try_from(FUEL_TANK_SCALE).expect("scale fits"),
        ));

        let reward = number_after(
            &c_source("comm/starbas/starbas.c"),
            "DeltaSISGauges (0, 0, ",
        );
        Self {
            opening_fuel,
            opening_res_units,
            fuel_price,
            landing_cost,
            reward,
        }
    }

    fn after_landing_and_reward(&self) -> (u32, u32) {
        (
            self.opening_fuel - self.landing_cost,
            self.opening_res_units + self.reward,
        )
    }
}

fn derived_preparation(name: &str) -> DerivedPreparation {
    let steps = prep_of(name);
    let opening = steps
        .iter()
        .find_map(resources)
        .expect("opening resource proof");
    assert_eq!(
        opening.0,
        Some(u64::from(
            DerivedPreparation::from_game_data(0).opening_fuel
        )),
        "{name} opens with the game's starting tank"
    );
    DerivedPreparation::from_game_data(
        u32::try_from(opening.1.expect("opening RU")).expect("RU fits"),
    )
}

/// The fuel and RU the script must expect once the Starbase menu is up.
fn post_landing_state(name: &str) -> (Option<u64>, Option<u64>) {
    let (fuel, res_units) = derived_preparation(name).after_landing_and_reward();
    (Some(u64::from(fuel)), Some(u64::from(res_units)))
}

#[test]
fn luna_landing_costs_thirty_raw_fuel() {
    let derived = DerivedPreparation::from_game_data(0);
    assert_eq!(derived.opening_fuel, 1000);
    assert_eq!(derived.landing_cost, 30);
}

/// The preparation lands on Luna exactly once and flies no hyperspace before
/// the Starbase menu, so the only fuel deduction that precedes the check is
/// one Luna landing.
#[test]
fn the_starbase_menu_check_expects_the_opening_tank_minus_one_luna_landing() {
    for name in PREPARED {
        let steps = prep_of(name);
        let docked = position(&steps, 0, |step| {
            action_of(step) == "assert_mode" && step["mode"] == "starbase"
        });
        let landings = steps[..docked]
            .iter()
            .filter(|step| action_of(step) == "wait_for_planet_side_start")
            .count();
        assert_eq!(landings, 1, "{name} lands exactly once before the check");
        let derived = derived_preparation(name);
        let (fuel, res_units) = derived.after_landing_and_reward();
        assert_eq!(
            resources(&steps[docked + 1]),
            Some((Some(u64::from(fuel)), Some(u64::from(res_units)))),
            "{name} must expect {} raw fuel less one {}-raw Luna landing and the reward",
            derived.opening_fuel,
            derived.landing_cost
        );
    }
}

/// Entering Outfit rounds to the nearest whole unit, then 50 ordinary
/// purchases of one unit each end exactly at the script's final proof.
#[test]
fn outfit_entry_rounding_and_fifty_purchases_conserve_to_the_final_proof() {
    for name in PREPARED {
        let derived = derived_preparation(name);
        let (fuel, res_units) = derived.after_landing_and_reward();
        let rounded = round_fuel_on_outfit_entry(fuel).expect("rounds");
        assert_eq!(rounded, 1000, "{name}: 970 rounds half up to the unit");
        let mut state = FuelRuState {
            fuel_on_board: rounded,
            res_units,
        };
        let capacity = FUEL_RESERVE + FUEL_TANK_CAPACITY;
        let steps = prep_of(name);
        let enter = position(&steps, 0, |step| {
            resources(step).is_some_and(|found| found.1 == Some(u64::from(res_units)))
        });
        let leave = steps
            .iter()
            .rposition(|step| resources(step).is_some())
            .expect("final proof");
        let purchases = steps[enter..leave]
            .iter()
            .filter(|step| is_tap(step, "up"))
            .count();
        for _ in 0..purchases {
            let FuelChange::Apply(delta) =
                change_fuel_quantity(state, capacity, derived.fuel_price, FuelKey::UnitUp)
                    .expect("purchase arithmetic")
            else {
                panic!("{name}: a purchase was rejected");
            };
            state = state.apply(delta, capacity).expect("applies");
        }
        assert_eq!(
            resources(&steps[leave]),
            Some((
                Some(u64::from(state.fuel_on_board)),
                Some(u64::from(state.res_units))
            )),
            "{name} final proof is what the purchases really leave"
        );
    }
}
