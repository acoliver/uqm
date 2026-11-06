use uqm_rust::engine::World;

#[test]
fn activity_observation_uses_the_live_world_without_bridge_publication() {
    let mut world = World::new();
    let observation = world.activity_control();
    assert_eq!(observation.activity(), 0);
    world.set_activity(0x1004);
    assert_eq!(observation.activity(), 0x1004);
    world.set_activity(3);
    assert_eq!(observation.raise_flags(0x4000), 3);
    assert_eq!(world.activity(), 0x4003);
}

#[test]
fn resetting_game_data_keeps_existing_observers_attached_to_the_same_word() {
    let mut world = World::new();
    let observation = world.activity_control();
    world.set_activity(u16::MAX);
    world.data.game.glob_flags = 0x42;
    world.reset_game_data();
    assert_eq!(world.activity(), 0);
    assert_eq!(observation.activity(), 0);
    assert_eq!(world.data.game.glob_flags, 0x42);
    observation.raise_flags(0x4000);
    assert_eq!(world.activity(), 0x4000);
}

#[test]
fn activity_signals_do_not_cross_world_owners() {
    let mut first = World::new();
    let mut second = World::new();
    first.set_activity(4);
    second.set_activity(3);
    let signal = first.activity_control();
    assert_eq!(signal.raise_flags(0x1000), 4);
    assert_eq!(first.activity(), 0x1004);
    assert_eq!(second.activity(), 3);
}

#[test]
fn an_observer_can_be_sent_without_sending_or_sharing_the_world() {
    let mut world = World::new();
    world.set_activity(4);
    let signal = world.activity_control();
    std::thread::scope(|scope| {
        scope.spawn(move || assert_eq!(signal.raise_flags(0x4000), 4));
    });
    assert_eq!(world.activity(), 0x4004);
}

#[test]
fn presentation_terminal_reads_and_signals_the_supplied_world_without_a_bridge() {
    use uqm_rust::automation::{parse_script, validate_script, AutomationObserver};

    let script = br#"{
        "version": 1, "name": "presentation-activity",
        "budgets": {"max_input_ticks": 100, "max_presentations": 100, "max_wallclock_seconds": 30},
        "steps": [{"action": "finish"}]
    }"#;
    let script = validate_script(
        parse_script(script, "activity.json").unwrap(),
        "activity.json",
    )
    .unwrap();
    let directory = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let mut world = World::new();
    let observer = AutomationObserver::new(
        script,
        directory.path().into(),
        world.activity_control(),
        uqm_rust::engine::input::InputControls::default(),
        Default::default(),
    );
    world.set_activity(0x0403);
    // Missing frame evidence deliberately terminates from the presentation callback.
    assert!(observer.process_present(None));
    assert_eq!(world.activity(), 0x4403);
    let summary = directory.path().join("summary.json");
    observer.write_replay_summary(&summary).unwrap();
    let summary: serde_json::Value =
        serde_json::from_slice(&std::fs::read(summary).unwrap()).unwrap();
    assert_eq!(summary["final_activity"], 0x0403);
    world.set_activity(4);
    assert!(observer.process_present(None));
    observer
        .write_replay_summary(&directory.path().join("second.json"))
        .unwrap();
    let second: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("second.json")).unwrap())
            .unwrap();
    assert_eq!(second["final_activity"], 0x0403);
}
