//! Source contracts for unplayed, player-driven Sol orbit proposals.

use serde_json::{json, Value};
use std::path::PathBuf;
use uqm_rust::automation::native_checkpoint::{CheckpointClass, NativeCheckpointPlan};
use uqm_rust::automation::suite::{required_suite, Domain, MATRIX};
use uqm_rust::automation::{parse_script, validate_script, Action};

const SCENARIOS: [(&str, u8); 2] = [
    ("real-sol-earth-orbit", 2),
    ("real-sol-pluto-orbit-scan", 8),
];

fn script_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join(format!("{name}.json"))
}

fn document(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(script_path(name)).unwrap()).unwrap()
}

fn validate(value: &Value) -> Result<(), String> {
    let path = script_path(value["name"].as_str().unwrap());
    let bytes = serde_json::to_vec(value).unwrap();
    let parsed = parse_script(&bytes, &path).map_err(|error| error.to_string())?;
    validate_script(parsed, &path)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[test]
fn complete_sources_validate_and_preserve_the_existing_new_game_probe_inputs() {
    let reference = document("explore-planet-v1");
    for (name, _) in SCENARIOS {
        let value = document(name);
        validate(&value).unwrap();
        assert_eq!(value["name"], name);
        assert_eq!(value["version"], reference["version"]);
        for field in ["start_save", "start_scene", "seed", "fixture"] {
            assert!(value.get(field).is_none(), "{name}: no injected {field}");
        }
        let steps = value["steps"].as_array().unwrap();
        assert_eq!(&steps[..9], &reference["steps"].as_array().unwrap()[..9]);
        assert_eq!(steps.last().unwrap(), &json!({"action": "finish"}));
        assert_eq!(value["budgets"]["max_wallclock_seconds"], 300);
        assert_eq!(value["budgets"]["max_input_ticks"], 25000);
        assert_eq!(value["budgets"]["max_presentations"], 50000);
    }
}

#[test]
fn complete_destination_tails_require_real_arrival_orbit_and_unchanged_resources() {
    for (name, planet) in SCENARIOS {
        let value = document(name);
        let steps = value["steps"].as_array().unwrap();
        let body = if planet == 8 { "pluto" } else { "earth" };
        let resources = json!({
            "action": "assert_ship_resources", "fuel_on_board": 1000, "res_units": 0
        });
        let mut expected = vec![
            resources.clone(),
            json!({"action": "navigate_to_planet", "planet": planet, "max_ticks": 6000}),
            json!({"action": "capture", "label": format!("{body}-inner-system")}),
            json!({"action": "navigate_to_orbit", "planet": planet, "max_ticks": 6000}),
            json!({"action": "select_planet_menu", "phase": "orbit", "max_ticks": 500}),
            json!({"action": "assert_mode", "mode": "orbit"}),
            resources.clone(),
            json!({"action": "capture", "label": format!("{body}-orbit-menu")}),
        ];
        if planet == 8 {
            expected.extend([
                json!({"action": "select_planet_menu", "phase": "auto_scan", "max_ticks": 500}),
                json!({"action": "select_planet_menu", "phase": "dispatch", "max_ticks": 500}),
                json!({"action": "tap_menu_key", "key": "cancel", "value": 1, "hold": 2, "settle": 30}),
                json!({"action": "assert_mode", "mode": "orbit"}),
                resources,
                json!({"action": "capture", "label": "pluto-orbit-scanned"}),
            ]);
        }
        expected.push(json!({"action": "finish"}));
        assert_eq!(&steps[9..], expected);
    }
}

#[test]
fn arrival_scan_and_assertion_steps_have_ordered_native_checkpoint_obligations() {
    for (name, _) in SCENARIOS {
        let path = script_path(name);
        let bytes = std::fs::read(&path).unwrap();
        let script = validate_script(parse_script(&bytes, &path).unwrap(), &path).unwrap();
        let plan = NativeCheckpointPlan::derive(&bytes, &script).unwrap();
        let mut expected = vec![
            (0, CheckpointClass::SemanticWait),
            (3, CheckpointClass::Assertion),
            (4, CheckpointClass::Navigation),
            (6, CheckpointClass::Assertion),
            (7, CheckpointClass::SemanticWait),
            (8, CheckpointClass::Assertion),
            (9, CheckpointClass::Assertion),
            (10, CheckpointClass::Navigation),
            (11, CheckpointClass::Capture),
            (12, CheckpointClass::Navigation),
            (13, CheckpointClass::Navigation),
            (14, CheckpointClass::Assertion),
            (15, CheckpointClass::Assertion),
            (16, CheckpointClass::Capture),
        ];
        if name == "real-sol-pluto-orbit-scan" {
            expected.extend([
                (17, CheckpointClass::Navigation),
                (18, CheckpointClass::Navigation),
                (20, CheckpointClass::Assertion),
                (21, CheckpointClass::Assertion),
                (22, CheckpointClass::Capture),
            ]);
        }
        let actual: Vec<_> = plan
            .checkpoints
            .iter()
            .map(|checkpoint| (checkpoint.action_index, checkpoint.class))
            .collect();
        assert_eq!(actual, expected);
        for checkpoint in &plan.checkpoints {
            assert_eq!(checkpoint.action, script.steps()[checkpoint.action_index]);
        }
        assert!(matches!(script.steps()[12], Action::NavigateToOrbit(_)));
        assert!(!plan.linked_floors);
        assert!(!plan.expects_quit);
    }
}

#[test]
fn closed_schema_rejects_unknown_root_action_and_navigation_fields() {
    for (name, _) in SCENARIOS {
        for pointer in ["", "/steps/10", "/steps/12", "/steps/13"] {
            let mut value = document(name);
            value
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("teleport".into(), json!(true));
            let error = validate(&value).unwrap_err();
            assert!(error.contains("teleport"), "{error}");
        }
        let mut value = document(name);
        value["steps"][10]["action"] = json!("teleport");
        assert!(validate(&value).unwrap_err().contains("teleport"));
    }
}

#[test]
fn invalid_targets_zero_navigation_bounds_and_insufficient_budgets_are_rejected() {
    for (name, _) in SCENARIOS {
        for index in [10, 12] {
            let mut value = document(name);
            value["steps"][index]["planet"] = json!(9);
            assert!(validate(&value).is_err());
            let mut value = document(name);
            value["steps"][index]["max_ticks"] = json!(0);
            assert!(validate(&value).is_err());
        }
        let mut value = document(name);
        value["budgets"]["max_input_ticks"] = json!(1);
        assert!(validate(&value).is_err());
        let mut value = document(name);
        value["steps"].as_array_mut().unwrap().pop();
        assert!(validate(&value).is_err());
    }
}

#[test]
fn executable_inventory_adds_only_new_game_and_navigation_claims_with_default_deadlines() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let authority: Value =
        serde_json::from_slice(&std::fs::read(root.join("ci/gates.json")).unwrap()).unwrap();
    let pins = authority["native_acceptance"]["scenario_scripts"]
        .as_array()
        .unwrap();
    for (name, _) in SCENARIOS {
        assert!(required_suite().contains(&name));
        let row = MATRIX.iter().find(|row| row.scenario == name).unwrap();
        assert_eq!(row.domains, [Domain::NewGame, Domain::NavigationOrbitScan]);
        assert!(row.fixture.is_none());
        assert!(!row.composed_journey);
        let expected = json!({"path": format!("rust/scripts/{name}.json")});
        assert_eq!(pins.iter().filter(|pin| **pin == expected).count(), 1);
    }
}
