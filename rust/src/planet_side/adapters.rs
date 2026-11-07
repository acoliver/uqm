//! Production-facing PlanetSide adapters available without sharing gameplay state.

use crate::engine::battle::Cx;
use std::cell::RefCell;

use super::runtime::{AdapterError, PlanetSideClock, PlanetSideInput, ShipStatusPort, Tick};
use super::session::{ShipDelta, ShipStatus};
use super::simulation::FrameInput;
use crate::engine::{input::InputControls, world::activity::ActivityControl};

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
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
    ) -> Result<FrameInput, AdapterError> {
        use crate::engine::observe::{CampaignView, InputView};
        let campaign = CampaignView::for_input(InputView::BeforeUpdate, || {
            CampaignView::capture(cx.world, cx.kernel)
        });
        if cx
            .with_legacy(|| {
                io.observer
                    .input_admitted(&InputView::BeforeUpdate, &campaign)
            })
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
        if cx
            .with_legacy(|| {
                io.observer
                    .input_admitted(&InputView::AfterUpdate(activity), &None)
            })
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

#[cfg(not(test))]
use crate::threading::pacing::linked::sleep_thread_until as SleepThreadUntil;
#[cfg(not(test))]
use crate::time::counter_abi::GetTimeCounter;
#[cfg(not(test))]
use crate::ui::sis::capacity::GetStorageBayCapacity;
#[cfg(not(test))]
use crate::ui::sis::draw_landers;
#[cfg(not(test))]
use crate::ui::sis::DeltaSISGauges;
#[cfg(not(test))]
use crate::ui::sis::DrawStorageBays;

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
pub struct CffiPlanetSideClock;

impl PlanetSideClock for CffiPlanetSideClock {
    fn now(&self) -> Tick {
        #[cfg(not(test))]
        {
            Tick(GetTimeCounter())
        }
        #[cfg(test)]
        {
            Tick(0)
        }
    }

    fn sleep_until(&mut self, deadline: Tick) -> Result<(), AdapterError> {
        #[cfg(not(test))]
        unsafe {
            SleepThreadUntil(deadline.0);
        }
        #[cfg(test)]
        let _ = deadline;
        Ok(())
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
        #[cfg(not(test))]
        {
            self.game
                .borrow_mut()
                .with_legacy(|| GetStorageBayCapacity())
        }
        #[cfg(test)]
        {
            0
        }
    }
}

impl ShipStatusPort for CffiShipStatus<'_, '_> {
    fn apply(&mut self, delta: &ShipDelta) -> Result<(), AdapterError> {
        #[cfg(not(test))]
        if delta.crew != 0 {
            // SAFETY: the retained game publishes the live gauge owners for this call.
            self.game
                .borrow_mut()
                .with_legacy(|| unsafe { DeltaSISGauges(delta.crew, 0, 0) });
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
        #[cfg(not(test))]
        if delta.landers != 0 {
            // SAFETY: native drawing runs while the retained game owns the graphics state.
            self.game
                .borrow_mut()
                .with_legacy(|| unsafe { draw_landers() });
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
        #[cfg(not(test))]
        if delta.element_mass != 0 {
            // SAFETY: the storage display reads the game published for this call.
            self.game
                .borrow_mut()
                .with_legacy(|| unsafe { DrawStorageBays(0) });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_linked_input_is_neutral() {
        let input = InputControls::default();
        let game = crate::engine::Game::new();
        assert_eq!(
            read_player_one_input(&input, &game.world.activity_control()),
            FrameInput::default()
        );
    }

    #[test]
    fn non_linked_clock_is_available_for_unit_tests() {
        let mut clock = CffiPlanetSideClock;
        assert_eq!(clock.now(), Tick(0));
        assert_eq!(clock.sleep_until(Tick(1)), Ok(()));
    }

    #[test]
    fn non_linked_ship_snapshot_is_neutral() {
        let mut owners = crate::engine::Game::new();
        let mut battle = crate::engine::battle::Battle::new();
        let game = RefCell::new(Cx::new(
            &mut battle,
            &mut owners.world,
            &mut owners.kernel,
            crate::math::random,
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
        use crate::engine::{
            battle::{Battle, Cx},
            observe::{CampaignView, CheckpointView, Injected, InputView, Io, Observer, Presented},
            Game,
        };

        struct Recording {
            stop: Option<InputView>,
            inputs: Vec<(InputView, Option<u16>)>,
        }
        impl Observer<Option<CampaignView>> for Recording {
            fn input_admitted(
                &mut self,
                view: &InputView,
                world: &Option<CampaignView>,
            ) -> Injected {
                self.inputs
                    .push((*view, world.as_ref().map(|world| world.activity)));
                Injected {
                    stop: Some(*view) == self.stop,
                    seed: None,
                    planet_side: None,
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
                crate::math::random,
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
