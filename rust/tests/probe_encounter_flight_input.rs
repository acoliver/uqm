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
