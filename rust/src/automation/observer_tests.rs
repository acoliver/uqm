use super::observer::Handle;
use super::{campaign_state, coordinator::AutomationObserver, ending::Ending};
use crate::engine::observe::{CheckpointView, InputView, Observer};
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
    assert_eq!(ui.star_map(), None);
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
        CheckpointView::CommunicationResponses {
            count: 3,
            choices: &Default::default(),
        },
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

#[test]
fn starting_another_scene_resets_dispatch_and_requires_new_observations() {
    use super::scenario::{scene_plan, AutomationScene, SceneActivationBoundary, SceneError};
    let (_output, mut handle) = observer();
    handle
        .0
        .scene
        .lock()
        .begin(AutomationScene::SolProbeEncounter);
    handle.checkpoint(&CheckpointView::EncounterDispatch(24));
    handle.checkpoint(&CheckpointView::DialogueDispatch(18));
    let scene = AutomationScene::StarbaseCommander;
    handle.0.scene.lock().begin(scene);
    assert_eq!(handle.0.scene.lock().active_scene(), Some(scene));
    assert_eq!(
        handle.0.scene.lock().dispatch_observation(),
        (0, None, None)
    );
    assert_eq!(
        handle.0.scene.lock().verify_dispatch(24, 18),
        Err(SceneError::EncounterNotDispatched)
    );
    let plan = scene_plan(scene, SceneActivationBoundary::GameInitialized).unwrap();
    handle.checkpoint(&CheckpointView::DialogueDispatch(
        plan.expected_dialogue_conversation,
    ));
    assert_eq!(
        handle.0.scene.lock().dispatch_observation(),
        (0, None, Some(plan.expected_dialogue_conversation))
    );
    assert_eq!(
        handle.0.scene.lock().verify(scene),
        Err(SceneError::EncounterNotDispatched)
    );
    handle.checkpoint(&CheckpointView::EncounterDispatch(
        plan.expected_encounter_conversation,
    ));
    assert_eq!(
        handle.0.scene.lock().verify_dispatch(
            plan.expected_encounter_conversation,
            plan.expected_dialogue_conversation
        ),
        Err(SceneError::DialogueNotDispatched)
    );
    handle.checkpoint(&CheckpointView::DialogueDispatch(
        plan.expected_dialogue_conversation,
    ));
    assert_eq!(handle.0.scene.lock().verify(scene), Ok(plan));
    assert_eq!(
        handle.0.scene.lock().verify_dispatch(
            plan.expected_encounter_conversation,
            plan.expected_dialogue_conversation
        ),
        Ok(())
    );
    assert_eq!(
        handle.0.scene.lock().dispatch_observation(),
        (
            1,
            Some(plan.expected_encounter_conversation),
            Some(plan.expected_dialogue_conversation)
        )
    );
}

#[test]
fn lander_checkpoints_are_copied_into_the_supplied_observer() {
    use crate::engine::observe_port::planet_side::{MineralDeposit, PlanetSideSnapshot};
    let (_first_output, mut first) = observer();
    let (_second_output, second) = observer();
    let mut trip = PlanetSideSnapshot::default();
    trip.observation.generation = 4;
    trip.observation.active = true;
    trip.observation.phase = 4;
    trip.observation.current_x = 30;
    trip.observation.current_y = 60;
    trip.observation.facing = 7;
    trip.observation.mineral_units[6] = 5;
    trip.deposits = Some(vec![MineralDeposit {
        x: 35,
        y: 65,
        category: 6,
    }]);
    trip.energy_nodes = vec![(31, 80, 90)];
    trip.energy_retrieved = 1 << 31;
    first.checkpoint(&CheckpointView::PlanetSide(&trip));
    let copied = first.0.planet_side.lock().clone();
    assert_eq!(copied, trip);
    trip.energy_nodes.clear();
    trip.deposits = None;
    assert_eq!(*first.0.planet_side.lock(), copied);
    assert_eq!(*second.0.planet_side.lock(), PlanetSideSnapshot::default());
    let mineral = super::planet_minerals::PlanetSnapshot::from_observation(
        copied.observation,
        copied.deposits.clone(),
    );
    assert_eq!(mineral.generation, 4);
    assert_eq!(
        mineral.position,
        crate::planet_side::model::SurfacePoint { x: 30, y: 60 }
    );
    assert_eq!(mineral.facing, 7);
    assert_eq!(mineral.units[6], 5);
    assert_eq!(mineral.deposits, copied.deposits);
    assert!(mineral.active);
    first.checkpoint(&CheckpointView::PlanetSide(&trip));
    assert_eq!(*first.0.planet_side.lock(), trip);
    assert_eq!(*second.0.planet_side.lock(), PlanetSideSnapshot::default());
}

#[test]
fn lander_fixture_requests_belong_to_each_observer() {
    use crate::engine::observe_port::planet_side::FixtureLayout;
    let (_first_output, first) = observer();
    let (_second_output, second) = observer();
    first
        .0
        .planet_side_fixtures
        .lock()
        .queue(FixtureLayout::CollisionParity)
        .unwrap();
    assert!(second.0.planet_side_fixtures.lock().take().is_none());
    assert_eq!(
        first.0.planet_side_fixtures.lock().take(),
        Some(FixtureLayout::CollisionParity)
    );
    assert!(first.0.planet_side_fixtures.lock().take().is_none());
    first
        .0
        .planet_side_fixtures
        .lock()
        .queue(FixtureLayout::CollisionParity)
        .unwrap();
    second
        .0
        .planet_side_fixtures
        .lock()
        .queue(FixtureLayout::MineralGallery)
        .unwrap();
    first.0.finalize(0).unwrap();
    assert!(first.0.planet_side_fixtures.lock().take().is_none());
    assert_eq!(
        second.0.planet_side_fixtures.lock().take(),
        Some(FixtureLayout::MineralGallery)
    );
}

#[test]
fn lander_admission_consumes_only_its_observers_request_and_sequence() {
    use crate::engine::observe_port::planet_side::FixtureLayout;
    let (_first_output, mut first) = observer();
    let (_second_output, mut second) = observer();
    first
        .0
        .planet_side_fixtures
        .lock()
        .queue(FixtureLayout::MineralGallery)
        .unwrap();
    let first_trip = first
        .input_admitted(&InputView::PlanetSideBegin, &None)
        .planet_side
        .unwrap();
    assert_eq!(first_trip.generation, 1);
    assert_eq!(first_trip.fixture, Some(FixtureLayout::MineralGallery));
    let next_trip = first
        .input_admitted(&InputView::PlanetSideBegin, &None)
        .planet_side
        .unwrap();
    assert_eq!(next_trip.generation, 2);
    assert_eq!(next_trip.fixture, None);
    let other_trip = second
        .input_admitted(&InputView::PlanetSideBegin, &None)
        .planet_side
        .unwrap();
    assert_eq!(other_trip.generation, 1);
    assert_eq!(other_trip.fixture, None);
}

#[test]
fn report_visibility_belongs_to_the_supplied_observer_and_survives_trip_updates() {
    let (_first_output, mut first) = observer();
    let (_second_output, second) = observer();
    first.checkpoint(&CheckpointView::PlanetSideReport(true));
    assert!(*first.0.planet_side_report.lock());
    assert!(!*second.0.planet_side_report.lock());
    first.checkpoint(&CheckpointView::PlanetSide(&Default::default()));
    assert!(*first.0.planet_side_report.lock());
    first.checkpoint(&CheckpointView::PlanetSideReport(false));
    assert!(!*first.0.planet_side_report.lock());
    assert!(!*second.0.planet_side_report.lock());
}

#[test]
fn map_observation_freezes_projection_and_cursor_per_observer_and_clears_on_exit() {
    use crate::engine::observe_port::starmap::{Projection, StarMapView};
    use crate::graphics::native_abi::POINT;
    let (_first_output, mut first) = observer();
    let (_second_output, second) = observer();
    let mut view = StarMapView {
        cursor: POINT { x: 1000, y: 2000 },
        projection: Projection {
            origin: POINT { x: 4999, y: 4999 },
            zoom: 1,
            width: 242,
            height: 227,
        },
    };
    let published = view;
    first.checkpoint(&CheckpointView::StarMap(Some(view)));
    view.cursor.x = 9000;
    view.projection.zoom = 4;
    assert_ne!(view, published);
    assert_eq!(first.0.ui.star_map(), Some(published));
    assert_eq!(second.0.ui.star_map(), None);
    let mut world = crate::engine::World::new();
    world.story = Some(crate::engine::world::story::StoryState::new());
    let mut campaign = crate::engine::observe::CampaignView::from_world(&world);
    campaign.instant_move = true;
    let target = super::star_flight::StarTarget::resolve("Sol").unwrap();
    let observation = super::input_ffi::star_flight_observation(&campaign, target, &first.0.ui);
    assert_eq!(observation.cursor, Some((1000, 2000)));
    let (x, y) = target.coordinates();
    assert_eq!(
        observation.cursor_delta,
        (
            published.projection.x(x.into()) - published.projection.x(1000),
            published.projection.y(y.into()) - published.projection.y(2000),
        )
    );
    assert!(observation.instant_move);
    first.checkpoint(&CheckpointView::StarMap(None));
    assert_eq!(first.0.ui.star_map(), None);
    assert_eq!(
        super::input_ffi::star_flight_observation(&campaign, target, &first.0.ui).cursor,
        None
    );
    assert_idle(&second.0);
}
