use super::observer::Handle;
use super::{campaign_state, coordinator::AutomationObserver, ending::Ending};
use crate::engine::observe::{CheckpointView, Observer};
use std::sync::Arc;

fn observer() -> (tempfile::TempDir, Handle) {
    let source = br#"{"version":1,"name":"owned-observation","steps":[{"action":"finish"}],"budgets":{"max_input_ticks":10,"max_presentations":10,"max_wallclock_seconds":60}}"#;
    let script = super::validate_script(
        super::parse_script(source, "test.json").unwrap(),
        "test.json",
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let observer = AutomationObserver::new(
        script,
        directory.path().into(),
        crate::engine::World::new().activity_control(),
        Default::default(),
        Default::default(),
    );
    (directory, Handle(observer))
}

fn assert_idle(owner: &AutomationObserver) {
    let ui = &owner.ui;
    assert!(ui.verify_game_options_active().is_err());
    assert_eq!(ui.communication_responses(), (0, 0, false));
    assert_eq!(ui.communication_lifecycle(), (false, 0));
    assert_eq!(ui.communication_replay_observation(), (0, false));
    assert!(!ui.starbase_menu_active());
    assert!(!ui.encounter_menu_active());
    assert_eq!(
        ui.planet_menu_observation(),
        (0, super::ui_observation::PlanetMenuPhase::Inactive)
    );
    assert_eq!(ui.ship_picker_observation(), (0, false));
    assert_eq!(ui.ship_picker_cursor(), crate::pickship::Cursor::START);
    assert_eq!(ui.ship_escape_generation(), 0);
    assert_eq!(*owner.ending.lock(), Ending::default());
    assert_eq!(owner.scene.lock().dispatch_observation(), (0, None, None));
    assert_eq!(owner.scene.lock().active_scene(), None);
    #[cfg(feature = "debug-process")]
    assert!(owner.pending_native_presentation.lock().is_none());
}

#[test]
fn checkpoint_observations_belong_to_each_observer() {
    let (_first_output, mut first) = observer();
    let (_second_output, second) = observer();
    assert!(!Arc::ptr_eq(&first.0, &second.0));
    for event in [
        CheckpointView::GameOptions(true),
        CheckpointView::CommunicationStarted,
        CheckpointView::CommunicationResponses(3),
        CheckpointView::ResponsesReady(true),
        CheckpointView::CommunicationReplay(true),
        CheckpointView::StarbaseMenu(true),
        CheckpointView::EncounterMenu(true),
        CheckpointView::PlanetMenu(3),
        CheckpointView::ShipPicker(true),
        CheckpointView::ShipPickerCursor { x: 1, y: 1 },
        CheckpointView::ShipEscaped,
        CheckpointView::Won,
        CheckpointView::PresentationStarted,
        CheckpointView::PresentationCompleted,
        CheckpointView::CreditsStarted,
        CheckpointView::CreditsReady,
        CheckpointView::CreditsCompleted,
    ] {
        first.checkpoint(&event);
    }
    let ui = &first.0.ui;
    assert!(ui.verify_game_options_active().is_ok());
    assert_eq!(ui.communication_responses(), (1, 3, true));
    assert_eq!(ui.communication_lifecycle(), (true, 0));
    assert_eq!(ui.communication_replay_observation(), (1, true));
    assert!(ui.starbase_menu_active() && ui.encounter_menu_active());
    assert_eq!(
        ui.planet_menu_observation(),
        (1, super::ui_observation::PlanetMenuPhase::Dispatch)
    );
    assert_eq!(ui.ship_picker_observation(), (1, true));
    assert_eq!(
        ui.ship_picker_cursor(),
        crate::pickship::Cursor { x: 1, y: 1 }
    );
    assert_eq!(ui.ship_escape_generation(), 1);
    assert!(first.0.ending.lock().credits_completed);
    assert_idle(&second.0);

    first.checkpoint(&CheckpointView::CommunicationEnded);
    assert_eq!(first.0.ui.communication_responses(), (1, 3, false));
    assert_eq!(first.0.ui.communication_lifecycle(), (false, 1));
    assert_idle(&second.0);
    drop(first);
    let (_third_output, third) = observer();
    assert_idle(&third.0);
}

#[test]
fn campaign_reads_the_supplied_observers_ending_and_encounter_state() {
    use campaign_state::{LiveCampaign, StateKey, StateSource};
    let (_output, mut handle) = observer();
    handle.checkpoint(&CheckpointView::Won);
    handle.checkpoint(&CheckpointView::PresentationStarted);
    handle.checkpoint(&CheckpointView::PresentationCancelled);
    handle.checkpoint(&CheckpointView::PresentationCompleted);
    handle.checkpoint(&CheckpointView::CreditsStarted);
    handle.checkpoint(&CheckpointView::EncounterMenu(true));
    let world = campaign_state::empty_view();
    let campaign = LiveCampaign {
        world: &world,
        ui: &handle.0.ui,
        ending: *handle.0.ending.lock(),
        flagship_destroyed: false,
    };
    assert_eq!(campaign.read(StateKey::Ending(0)), Some(1));
    assert_eq!(campaign.read(StateKey::Ending(1)), Some(1));
    assert_eq!(campaign.read(StateKey::Ending(2)), Some(0));
    assert_eq!(campaign.read(StateKey::Ending(3)), Some(0));
    assert_eq!(campaign.read(StateKey::EncounterMenu), Some(1));
}

#[test]
fn scene_dispatch_observations_do_not_cross_observer_runs() {
    use super::scenario::AutomationScene;
    let (_first_output, mut first) = observer();
    let (_second_output, second) = observer();
    first
        .0
        .scene
        .lock()
        .begin(AutomationScene::SolProbeEncounter);
    first.checkpoint(&CheckpointView::EncounterDispatch(24));
    first.checkpoint(&CheckpointView::DialogueDispatch(18));
    assert_eq!(
        first.0.scene.lock().dispatch_observation(),
        (1, Some(24), Some(18))
    );
    assert!(first
        .0
        .scene
        .lock()
        .verify(AutomationScene::SolProbeEncounter)
        .is_ok());
    assert_eq!(
        second.0.scene.lock().dispatch_observation(),
        (0, None, None)
    );
    assert_eq!(second.0.scene.lock().active_scene(), None);
    first.checkpoint(&CheckpointView::EncounterDispatch(24));
    assert_eq!(
        first.0.scene.lock().dispatch_observation(),
        (1, Some(24), None)
    );
    first.checkpoint(&CheckpointView::DialogueDispatch(0));
    assert!(first
        .0
        .scene
        .lock()
        .verify(AutomationScene::SolProbeEncounter)
        .is_err());
    assert!(second.0.scene.lock().verify_dispatch(24, 18).is_err());
}
