//! Production-facing PlanetSide adapters available without sharing gameplay state.

use super::runtime::{AdapterError, PlanetSideClock, PlanetSideInput, ShipStatusPort, Tick};
use super::session::{ShipDelta, ShipStatus};
use super::simulation::FrameInput;
use std::cell::RefCell;
use uqm_engine::battle::Cx;
use uqm_engine::{input::InputControls, world::activity::ActivityControl};

/// Input adapter retaining the resolved player-one controls and activity signal.
pub struct CffiPlanetSideInput {
    controls: InputControls,
    activity: ActivityControl,
}

impl CffiPlanetSideInput {
    pub fn new(controls: InputControls, activity: ActivityControl) -> Self {
        Self { controls, activity }
    }

    pub fn poll_observed(
        &mut self,
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut uqm_engine::observe::Io<'_>,
    ) -> Result<FrameInput, AdapterError> {
        use uqm_engine::observe::InputView;
        if io
            .observer
            .input_admitted(&InputView::BeforeUpdate, cx)
            .stop
        {
            return Ok(FrameInput {
                abort: true,
                ..FrameInput::default()
            });
        }
        #[cfg(not(test))]
        {
            let runtime = cx.kernel.input_runtime();
            runtime.update(cx);
        }
        let activity = cx.world.activity();
        if io
            .observer
            .input_admitted(&InputView::AfterUpdate(activity), cx)
            .stop
        {
            return Ok(FrameInput {
                abort: true,
                ..FrameInput::default()
            });
        }
        self.poll()
    }
}

impl PlanetSideInput for CffiPlanetSideInput {
    fn poll(&mut self) -> Result<FrameInput, AdapterError> {
        Ok(read_player_one_input(&self.controls, &self.activity))
    }
}

fn read_player_one_input(input: &InputControls, activity: &ActivityControl) -> FrameInput {
    use uqm_engine::input_port::frames::{
        KEY_ESCAPE, KEY_LEFT, KEY_RIGHT, KEY_SPECIAL, KEY_UP, KEY_WEAPON,
    };
    let snapshot = input.current();
    let key = &snapshot.key[input.player_controls()[0] as usize];
    FrameInput {
        turn_left: key[KEY_LEFT as usize] != 0,
        turn_right: key[KEY_RIGHT as usize] != 0,
        thrust: key[KEY_UP as usize] != 0,
        fire: key[KEY_WEAPON as usize] != 0,
        takeoff: key[KEY_ESCAPE as usize] != 0 || key[KEY_SPECIAL as usize] != 0,
        abort: activity.activity() & 0x4000 != 0,
    }
}

/// Main-thread clock adapter preserving the established pump-and-flush sleep path.
pub struct CffiPlanetSideClock(pub uqm_engine::threading::pacing::GameThreadPacer);
impl PlanetSideClock for CffiPlanetSideClock {
    fn now(&self) -> Tick {
        Tick(self.0.now())
    }
    fn sleep_until(&mut self, deadline: Tick) -> Result<(), AdapterError> {
        self.0.sleep_until(deadline.0);
        Ok(())
    }
}

pub(super) fn publish_trip(
    io: &RefCell<uqm_engine::observe::Io<'_>>,
    trip: &super::telemetry::TripTelemetry,
) {
    io.borrow_mut()
        .observer
        .checkpoint(&uqm_engine::observe::CheckpointView::PlanetSide(
            trip.snapshot(),
        ));
}

/// Publish copied values before the clock pumps native presentation callbacks.
pub struct ObservedClock<'a, 'observer, K> {
    pub clock: K,
    pub telemetry: std::rc::Rc<RefCell<super::telemetry::TripTelemetry>>,
    pub io: &'a RefCell<uqm_engine::observe::Io<'observer>>,
}

impl<K: PlanetSideClock> PlanetSideClock for ObservedClock<'_, '_, K> {
    fn now(&self) -> Tick {
        self.clock.now()
    }

    fn sleep_until(&mut self, deadline: Tick) -> Result<(), AdapterError> {
        publish_trip(self.io, &self.telemetry.borrow());
        self.clock.sleep_until(deadline)
    }
}

/// Production ship-state adapter over the current flagship status owner.
pub struct CffiShipStatus<'a, 'game> {
    pub game: &'a RefCell<Cx<'game>>,
}

impl CffiShipStatus<'_, '_> {
    /// Capture the ship values needed to initialize and settle one trip.
    #[must_use]
    pub fn snapshot(&self) -> ShipStatus {
        let game = self.game.borrow();
        let sis = &game.world.data.flagship;
        ShipStatus {
            crew_enlisted: sis.crew_enlisted,
            landers: sis.num_landers,
            total_element_mass: sis.total_element_mass,
            element_amounts: sis.element_amounts,
            total_bio_mass: sis.total_bio_mass,
        }
    }

    /// Return the current flagship mineral storage capacity.
    #[must_use]
    pub fn storage_capacity(&self) -> u16 {
        crate::ui::sis::capacity::storage_capacity(&self.game.borrow().world.data.flagship)
    }
}

impl ShipStatusPort for CffiShipStatus<'_, '_> {
    fn apply(&mut self, delta: &ShipDelta) -> Result<(), AdapterError> {
        if delta.crew != 0 {
            let mut game = self.game.borrow_mut();
            let Cx { kernel, world, .. } = &mut *game;
            crate::ui::sis::gauges::delta_sis_gauges(kernel, world, delta.crew, 0, 0);
        }
        {
            let mut game = self.game.borrow_mut();
            let sis = &mut game.world.data.flagship;
            if delta.landers < 0 {
                sis.num_landers = sis.num_landers.saturating_sub(delta.landers.unsigned_abs());
            } else if delta.landers > 0 {
                sis.num_landers = sis.num_landers.saturating_add(delta.landers as u8);
            }
        }
        if delta.landers != 0 {
            let game = self.game.borrow();
            crate::ui::sis::gauges::draw_landers(game.kernel, game.world);
        }
        {
            let mut game = self.game.borrow_mut();
            let sis = &mut game.world.data.flagship;
            for (current, added) in sis.element_amounts.iter_mut().zip(delta.element_amounts) {
                *current = current.saturating_add(added);
            }
            sis.total_element_mass = sis.total_element_mass.saturating_add(delta.element_mass);
            sis.total_bio_mass = sis.total_bio_mass.saturating_add(delta.biological_mass);
        }
        if delta.element_mass != 0 {
            let game = self.game.borrow();
            crate::ui::sis::gauges::draw_storage_bays(game.kernel, game.world, false);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn pacing_requires_initialized_display() {
        if !uqm_engine::test_support::native_process::own_graphics_process(concat!(
            module_path!(),
            "::pacing_requires_initialized_display"
        )) {
            return;
        }
        let owner = uqm_engine::graphics::context::LoadedContext::create(c"uninitialized pacing");
        let mut pacer = uqm_engine::threading::pacing::GameThreadPacer::new(owner.lease());
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pacer.sleep_until(0);
        }))
        .is_err());
    }

    #[test]
    #[serial_test::serial]
    fn nonzero_settlement_updates_only_the_supplied_campaign_and_draws_real_gauges() {
        super::super::resources::tests::with_packaged_resources(|owners| {
            use crate::starbase::sis_modules::ModuleType;
            use uqm_engine::graphics::{
                context::ContextLease, drawable::ContextFrame, drawable::LoadedDrawable,
            };
            let frames =
                LoadedDrawable::load(&owners.kernel.content(), c"graphics.flagshipstatus").unwrap();
            owners.kernel.flag_stat_frame = frames.lease().frame(0).unwrap();
            let context = ContextLease::require(owners.kernel.status_context);
            context.set_frame(ContextFrame::retain(owners.kernel.screen));
            let sis = &mut owners.world.data.flagship;
            sis.module_slots[0] = ModuleType::CrewPod as u8;
            sis.module_slots[1] = ModuleType::StorageBay as u8;
            sis.crew_enlisted = 20;
            sis.num_landers = 2;
            sis.total_element_mass = 10;
            sis.element_amounts[0] = 10;
            sis.total_bio_mass = 4;
            let untouched = uqm_engine::Game::new();
            let mut battle = uqm_engine::battle::Battle::new();
            {
                let game = RefCell::new(Cx::new(
                    &mut battle,
                    &mut owners.world,
                    &mut owners.kernel,
                    uqm_engine::math::random,
                ));
                let mut ship = CffiShipStatus { game: &game };
                assert_eq!(ship.storage_capacity(), 500);
                let mut delta = ShipDelta {
                    crew: -3,
                    landers: -1,
                    element_mass: 25,
                    biological_mass: 3,
                    ..Default::default()
                };
                delta.element_amounts[2] = 25;
                ship.apply(&delta).unwrap();
                let status = ship.snapshot();
                assert_eq!(
                    (
                        status.crew_enlisted,
                        status.landers,
                        status.total_element_mass,
                        status.total_bio_mass
                    ),
                    (17, 1, 35, 7)
                );
                assert_eq!(status.element_amounts, [10, 0, 25, 0, 0, 0, 0, 0]);
            }
            assert_eq!(untouched.world.data.flagship.crew_enlisted, 0);
            assert_eq!(untouched.world.data.flagship.num_landers, 0);
            context.flush();
        });
    }

    #[test]
    fn copied_observation_is_published_before_the_clock_pumps_presentations() {
        use std::rc::Rc;
        use uqm_engine::observe::{CheckpointView, Injected, InputView, Io, Observer, Presented};
        use uqm_engine::observe_port::planet_side::PlanetSideSnapshot;

        struct Recording(Rc<RefCell<Vec<PlanetSideSnapshot>>>);
        impl Observer for Recording {
            fn input_admitted(
                &mut self,
                _: &InputView,
                _: &mut uqm_engine::battle::Cx<'_>,
            ) -> Injected {
                Default::default()
            }
            fn checkpoint(&mut self, view: &CheckpointView<'_>) {
                if let CheckpointView::PlanetSide(snapshot) = view {
                    self.0.borrow_mut().push((*snapshot).clone());
                }
            }
            fn presented(&mut self, _: &mut Presented) {}
        }
        struct Pump(Rc<RefCell<Vec<PlanetSideSnapshot>>>);
        impl PlanetSideClock for Pump {
            fn now(&self) -> Tick {
                Tick(17)
            }
            fn sleep_until(&mut self, deadline: Tick) -> Result<(), AdapterError> {
                assert_eq!(deadline, Tick(41));
                let copied = self.0.borrow();
                assert_eq!(copied.len(), 1);
                assert_eq!(copied[0].observation.mineral_pickups, 1);
                assert_eq!(copied[0].observation.frames, 0);
                Ok(())
            }
        }
        let copies = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording(copies.clone());
        let io = RefCell::new(Io {
            observer: &mut observer,
        });
        let trip = super::super::telemetry::tests::trip();
        trip.borrow_mut().mineral_pickup();
        let mut clock = ObservedClock {
            clock: Pump(copies.clone()),
            telemetry: trip.clone(),
            io: &io,
        };
        assert_eq!(clock.now(), Tick(17));
        clock.sleep_until(Tick(41)).unwrap();
        trip.borrow_mut().mineral_pickup();
        assert_eq!(copies.borrow()[0].observation.mineral_pickups, 1);
    }

    #[test]
    fn non_linked_input_is_neutral() {
        let input = InputControls::default();
        let game = uqm_engine::Game::new();
        assert_eq!(
            read_player_one_input(&input, &game.world.activity_control()),
            FrameInput::default()
        );
    }

    #[test]
    #[serial_test::serial]
    fn clock_uses_the_actual_counter_and_pumps_an_elapsed_deadline() {
        let mut game = uqm_engine::Game::new();
        let display =
            uqm_engine::test_support::campaign_display::CampaignDisplay::install(&mut game)
                .unwrap();
        let context =
            uqm_engine::graphics::context::ContextLease::require(game.kernel.status_context);
        let mut clock =
            CffiPlanetSideClock(uqm_engine::threading::pacing::GameThreadPacer::new(context));
        let before = uqm_engine::time::counter_abi::get_time_counter();
        assert!(clock.now().0 >= before);
        clock.sleep_until(Tick(before)).unwrap();
        assert!(clock.now().0 >= before);
        let deadline = Tick(clock.now().0.wrapping_add(2));
        clock.sleep_until(deadline).unwrap();
        assert!(clock.now().0.wrapping_sub(deadline.0) < u32::MAX / 2);
        assert_ne!(clock.now(), Tick(0));
        drop(clock);
        display.finish(game);
    }

    #[test]
    fn storage_capacity_uses_the_retained_campaign_without_a_published_owner() {
        let mut owners = uqm_engine::Game::new();
        owners.world.data.flagship.module_slots[7] =
            crate::starbase::sis_modules::ModuleType::StorageBay as u8;
        let mut battle = uqm_engine::battle::Battle::new();
        let game = RefCell::new(Cx::new(
            &mut battle,
            &mut owners.world,
            &mut owners.kernel,
            uqm_engine::math::random,
        ));
        let ship = CffiShipStatus { game: &game };
        assert_eq!(ship.storage_capacity(), 500);
        game.borrow_mut().world.data.flagship.module_slots[8] =
            crate::starbase::sis_modules::ModuleType::StorageBay as u8;
        assert_eq!(ship.storage_capacity(), 1000);
    }

    #[test]
    fn non_linked_ship_snapshot_is_neutral() {
        let mut owners = uqm_engine::Game::new();
        let mut battle = uqm_engine::battle::Battle::new();
        let game = RefCell::new(Cx::new(
            &mut battle,
            &mut owners.world,
            &mut owners.kernel,
            uqm_engine::math::random,
        ));
        let mut ship = CffiShipStatus { game: &game };
        assert_eq!(
            ship.snapshot(),
            ShipStatus {
                crew_enlisted: 0,
                landers: 0,
                total_element_mass: 0,
                element_amounts: [0; 8],
                total_bio_mass: 0,
            }
        );
        assert_eq!(ship.storage_capacity(), 0);
        assert_eq!(ship.apply(&ShipDelta::default()), Ok(()));
    }

    #[test]
    #[serial_test::serial]
    fn observed_input_uses_the_supplied_campaign_and_honors_both_stop_boundaries() {
        use uqm_engine::{
            battle::{Battle, Cx},
            observe::{CampaignView, CheckpointView, Injected, InputView, Io, Observer, Presented},
            Game,
        };

        struct Recording {
            stop: Option<InputView>,
            inputs: Vec<(InputView, Option<u16>)>,
        }
        impl Observer for Recording {
            fn input_admitted(&mut self, view: &InputView, cx: &mut Cx<'_>) -> Injected {
                let world = CampaignView::for_input(*view, || CampaignView::capture(cx));
                self.inputs
                    .push((*view, world.as_ref().map(|world| world.activity)));
                Injected {
                    stop: Some(*view) == self.stop,
                    seed: None,
                    planet_side: None,
                    comm_response: None,
                }
            }
            fn checkpoint(&mut self, _: &CheckpointView<'_>) {}
            fn presented(&mut self, _: &mut Presented) {}
        }

        for stop in [
            Some(InputView::BeforeUpdate),
            Some(InputView::AfterUpdate(0x1234)),
            None,
        ] {
            let mut game = Game::new();
            let mut battle = Battle::new();
            let mut cx = Cx::new(
                &mut battle,
                &mut game.world,
                &mut game.kernel,
                uqm_engine::math::random,
            );
            cx.world.set_activity(0x1234);
            let mut observer = Recording {
                stop,
                inputs: Vec::new(),
            };
            let input =
                CffiPlanetSideInput::new(InputControls::default(), cx.world.activity_control())
                    .poll_observed(
                        &mut cx,
                        &mut Io {
                            observer: &mut observer,
                        },
                    )
                    .unwrap();
            assert_eq!(input.abort, stop.is_some());
            let mut expected = vec![(InputView::BeforeUpdate, Some(0x1234))];
            if stop != Some(InputView::BeforeUpdate) {
                expected.push((InputView::AfterUpdate(0x1234), None));
            }
            assert_eq!(observer.inputs, expected);
        }
    }
}
