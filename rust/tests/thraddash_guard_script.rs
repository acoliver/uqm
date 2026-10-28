use serde_json::Value;

#[test]
fn homeworld_crash_regression_observes_duration_before_normal_loss() {
    let script: Value =
        serde_json::from_str(include_str!("../scripts/thraddash-guard-v1.json")).unwrap();
    assert_eq!(script["start_save"]["path"], "saves/thraddash-diverted.sav");
    assert_eq!(script["budgets"]["max_input_ticks"], 300_000);
    assert_eq!(script["budgets"]["max_presentations"], 600_000);
    assert_eq!(script["budgets"]["max_wallclock_seconds"], 1_200);

    let steps = script["steps"].as_array().unwrap();
    let duration = steps
        .iter()
        .position(|step| step["action"] == "wait_for_battle_frames")
        .expect("the battle must run past the #379 crash point");
    let crash_frames = 314 * uqm_rust::battle::BATTLE_FRAME_RATE;
    assert_eq!(steps[duration]["minimum"], crash_frames + 1);
    assert_eq!(steps[duration]["max_ticks"], crash_frames + 2);
    assert_eq!(steps[duration - 1]["action"], "tap_menu_key");
    assert_eq!(steps[duration - 1]["key"], "select");

    let loss = &steps[duration + 1];
    assert_eq!(loss["action"], "wait_for_state");
    assert_eq!(
        loss["predicate"]["all"][0]["key"],
        "game:flagship_destroyed"
    );
    assert_eq!(loss["predicate"]["all"][0]["op"], "eq");
    assert_eq!(loss["predicate"]["all"][0]["value"], 1);
    assert_eq!(steps[duration + 2]["action"], "wait_for_main_menu_ready");
    assert!(!steps.iter().any(|step| step["action"] == "fight_encounter"));
}
