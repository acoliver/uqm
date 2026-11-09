#[test]
fn zoom_clipping_is_local_to_each_visit() {
    let world = world();
    let mut first = super::SolarSystem::new(&world, crate::engine::StarId(82));
    let second = super::SolarSystem::new(&world, crate::engine::StarId(82));
    assert_eq!(first.scale_rect, Default::default());
    first.scale_rect.corner.x = 42;
    assert_eq!(second.scale_rect, Default::default());
}

#[test]
fn scoped_flight_effects_apply_on_both_sides_of_the_input_callback() {
    use super::*;
    use crate::engine::world::generation::SolarFlight;
    use std::cell::Cell;
    let mut game = crate::engine::Game::new();
    game.world = world();
    let mut solar = SolarSystem::new(&game.world, StarId(82));
    let independent = SolarSystem::new(&game.world, StarId(82));
    let effects = Cell::new(Some(SolarFlight {
        max_ship_speed: 40,
        turn_wait: 3,
    }));
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0);
    cx.solar_flight = Some(&effects);
    let mut observer = crate::engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let mut frame = SolarInput {
        solar: &mut solar,
        step: |cx: &mut Cx<'_>, _: &mut Io<'_>, solar: &mut SolarSystem| {
            assert_eq!((solar.max_ship_speed, solar.turn_wait), (40, 3));
            cx.solar_flight.unwrap().set(Some(SolarFlight {
                max_ship_speed: 64,
                turn_wait: 5,
            }));
            false
        },
    };
    assert!(!frame.frame(&mut cx, &mut io));
    assert!(effects.get().is_none());
    assert_eq!((frame.solar.max_ship_speed, frame.solar.turn_wait), (64, 5));
    assert_eq!((independent.max_ship_speed, independent.turn_wait), (0, 0));
}

use super::*;
use crate::engine::{world::generation::Body, world::StarId, World};

fn world() -> World {
    let mut world = World::new();
    world.set_catalog(std::borrow::Cow::Borrowed(
        &super::super::tables::STARMAP_ARRAY,
    ));
    world
}

#[test]
#[serial_test::serial]
fn frame_admission_publishes_before_input_and_nested_input_uses_the_copy() {
    use crate::engine::observe::{CampaignView, Observer, Presented};
    struct Recording {
        ui: crate::automation::ui_observation::UiObservation,
        admissions: Vec<(i16, bool)>,
    }
    impl Observer<Option<CampaignView>> for Recording {
        fn input_admitted(&mut self, _: &InputView, _: &Option<CampaignView>) -> Injected {
            let snapshot = self
                .ui
                .solar_snapshot()
                .expect("solar checkpoint precedes admission");
            self.admissions
                .push((snapshot.planets[2].display_origin.x, snapshot.in_orbit));
            Injected::default()
        }
        fn checkpoint(&mut self, view: &CheckpointView<'_>) {
            if let CheckpointView::Solar(snapshot) = view {
                self.ui.observe_solar(*snapshot);
            }
        }
        fn presented(&mut self, _: &mut Presented) {}
    }
    fn stop(_: &mut Cx<'_>, _: &mut Io<'_>, _: &mut SolarSystem) -> bool {
        false
    }

    let mut game = crate::engine::Game::new();
    game.world = world();
    game.world.current_star = Some(StarId(82));
    let mut solar = SolarSystem::new(&game.world, StarId(82));
    solar.sys.sun.num_planets = 9;
    solar.sys.planets[2].image.origin.x = 37;
    solar.enter_inner(2);
    solar.enter_orbit(Body::Planet(2));
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0);
    let mut observer = Recording {
        ui: Default::default(),
        admissions: Vec::new(),
    };
    {
        let mut io = Io {
            observer: &mut observer,
        };
        let mut frame = SolarInput {
            solar: &mut solar,
            step: stop,
        };
        frame.admit(&mut cx, &mut io, InputView::Nested);
        frame.solar.sys.planets[2].image.origin.x = 90;
        frame.solar.finish_orbit();
        {
            let child = cx.reborrow();
            let snapshot = child
                .solar_frame
                .expect("context loan retains copied frame");
            assert_eq!(snapshot.planets[2].display_origin.x, 37);
            assert!(snapshot.in_orbit);
        }
        assert_eq!(cx.nested_input(&mut io, |_| ()), Some(()));
        frame.admit(&mut cx, &mut io, InputView::Nested);
    }
    assert_eq!(observer.admissions, [(37, true), (37, true), (90, false)]);
    observer.ui.begin_communication();
    assert_eq!(
        observer.ui.solar_snapshot().unwrap().selected,
        Some(Body::Planet(2))
    );
    observer.ui.observe_solar(None);
    assert!(observer.ui.solar_snapshot().is_none());
}

#[test]
fn required_star_and_generator_remain_fixed_when_campaign_selection_changes() {
    let mut world = world();
    let index = world.catalog()[82].index;
    let system = SolarSystem::new(&world, StarId(82));
    world.current_star = Some(StarId(99));
    assert_eq!(system.star(), StarId(82));
    assert_eq!(system.sys.star_index(), index);
    assert_eq!(system.location(), SystemLocation::Outer);
    assert_eq!(system.selected_body(), None);
}

#[test]
fn inner_and_orbit_transitions_keep_selected_body_and_persistence_key() {
    let world = world();
    let mut system = SolarSystem::new(&world, StarId(82));
    system.sys.sun.num_planets = 9;
    system.sys.planets[2].num_planets = 2;
    system.enter_inner(2);
    assert_eq!(system.location(), SystemLocation::Inner(2));
    assert_eq!(system.selected_body(), Some(Body::Planet(2)));
    assert!(!system.in_orbit());
    let moon = Body::Moon { planet: 2, moon: 1 };
    system.enter_orbit(moon);
    assert!(system.in_orbit());
    assert_eq!(system.selected_body(), Some(moon));
    let location = system.body_location(moon);
    assert_eq!((location.star, location.planet, location.moon), (82, 2, 2));
    assert_eq!(location.node_key(31), (82, 2, 2, 31));
    assert_eq!(moon.saved_orbit(), 3);
    system.finish_orbit();
    assert_eq!(system.selected_body(), Some(moon));
    assert!(!system.in_orbit());
    system.enter_inner(2);
    assert_eq!(system.selected_body(), Some(Body::Planet(2)));
    system.leave_inner();
    assert_eq!(system.location(), SystemLocation::Outer);
    assert_eq!(system.selected_body(), None);
}

#[test]
fn copied_frame_retains_geometry_and_live_orbit_flag_without_borrowing_the_session() {
    let world = world();
    let mut system = SolarSystem::new(&world, StarId(82));
    system.sys.sun.num_planets = 9;
    system.sys.sun.radius = 8192;
    system.sys.planets[2].num_planets = 1;
    system.sys.planets[2].image.origin.x = 37;
    system.sys.moons[0].data_index = 12;
    system.enter_inner(2);
    let moon = Body::Moon { planet: 2, moon: 0 };
    system.enter_orbit(moon);
    let snapshot = system.snapshot((320, 240));
    let copy = snapshot;
    system.sys.planets[2].image.origin.x = 90;
    system.finish_orbit();
    assert_eq!(snapshot.screen, (320, 240));
    assert_eq!(snapshot.star, StarId(82));
    assert_eq!(copy.planets[2].display_origin.x, 37);
    assert_eq!(copy.moons[0].data_index, 12);
    assert_eq!(copy.inner_planet, Some(2));
    assert_eq!(copy.selected, Some(moon));
    assert!(copy.in_orbit);
    assert!(!copy.in_ip_flight);
    let interrupted = system.snapshot((320, 240));
    assert!(!interrupted.in_orbit);
    assert!(!interrupted.in_ip_flight);
    assert_eq!(interrupted.selected, Some(moon));
}

#[test]
fn orbit_rejects_a_body_from_another_inner_system() {
    let world = world();
    let mut system = SolarSystem::new(&world, StarId(82));
    system.sys.sun.num_planets = 9;
    system.enter_inner(2);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        system.enter_orbit(Body::Planet(3));
    }))
    .is_err());
}
