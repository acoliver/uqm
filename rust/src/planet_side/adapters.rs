//! Production-facing PlanetSide adapters available without sharing gameplay state.

use super::runtime::{AdapterError, PlanetSideClock, PlanetSideInput, ShipStatusPort, Tick};
use super::session::{ShipDelta, ShipStatus};
use super::simulation::FrameInput;
#[cfg(not(test))]
use crate::c_bindings::controller_input::{CurrentInputState, CONTROL_TEMPLATE_NUM_TEMPLATES};
#[cfg(not(test))]
use crate::c_bindings::player_control_template;
#[cfg(not(test))]
use crate::engine::World as WorldOwner;
#[cfg(not(test))]
use crate::engine::WorldData as WorldFields;

#[cfg(not(test))]
const KEY_UP: usize = 0;
#[cfg(not(test))]
const KEY_LEFT: usize = 2;
#[cfg(not(test))]
const KEY_RIGHT: usize = 3;
#[cfg(not(test))]
const KEY_WEAPON: usize = 4;
#[cfg(not(test))]
const KEY_SPECIAL: usize = 5;
#[cfg(not(test))]
const KEY_ESCAPE: usize = 6;
#[cfg(not(test))]
const NUM_TEMPLATES: usize = CONTROL_TEMPLATE_NUM_TEMPLATES as usize;

/// Input adapter reading the resolved player-one control template.
pub struct CffiPlanetSideInput;

impl CffiPlanetSideInput {
    pub fn poll_observed(
        &mut self,
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
    ) -> Result<FrameInput, AdapterError> {
        use crate::engine::observe::{CampaignView, InputView};
        let campaign =
            CampaignView::for_input(InputView::BeforeUpdate, || CampaignView::capture(cx.world));
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
            cx.with_legacy(|| runtime.update());
        }
        if cx
            .with_legacy(|| io.observer.input_admitted(&InputView::AfterUpdate, &None))
            .stop
        {
            return Ok(FrameInput {
                abort: true,
                ..FrameInput::default()
            });
        }
        cx.with_legacy(|| self.poll())
    }
}

impl PlanetSideInput for CffiPlanetSideInput {
    fn poll(&mut self) -> Result<FrameInput, AdapterError> {
        Ok(read_player_one_input())
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

#[cfg(not(test))]
fn read_player_one_input() -> FrameInput {
    // The game loop updates these globals synchronously before invoking the
    // active input callback, so one snapshot cannot race another game tick.
    unsafe {
        let Some(template) = player_control_template(0).map(|template| template as usize) else {
            return FrameInput::default();
        };
        if template >= NUM_TEMPLATES {
            return FrameInput::default();
        }
        let key = &CurrentInputState.key[template];
        FrameInput {
            turn_left: key[KEY_LEFT] != 0,
            turn_right: key[KEY_RIGHT] != 0,
            thrust: key[KEY_UP] != 0,
            fire: key[KEY_WEAPON] != 0,
            takeoff: key[KEY_ESCAPE] != 0 || key[KEY_SPECIAL] != 0,
            abort: WorldOwner::current().read(crate::mainloop::c_extern::get_current_activity)
                & 0x4000
                != 0,
        }
    }
}

#[cfg(test)]
fn read_player_one_input() -> FrameInput {
    FrameInput::default()
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
pub struct CffiShipStatus;

impl CffiShipStatus {
    /// Capture the ship values needed to initialize and settle one trip.
    #[must_use]
    pub fn snapshot(&self) -> ShipStatus {
        #[cfg(not(test))]
        #[cfg(not(test))]
        unsafe {
            let world = WorldFields::current();
            let sis = std::ptr::addr_of!((**world).flagship).read();
            ShipStatus {
                crew_enlisted: sis.crew_enlisted,
                landers: sis.num_landers,
                total_element_mass: sis.total_element_mass,
                element_amounts: sis.element_amounts,
                total_bio_mass: sis.total_bio_mass,
            }
        }
        #[cfg(test)]
        {
            ShipStatus {
                crew_enlisted: 0,
                landers: 0,
                total_element_mass: 0,
                element_amounts: [0; 8],
                total_bio_mass: 0,
            }
        }
    }

    /// Return the current flagship mineral storage capacity.
    #[must_use]
    pub fn storage_capacity(&self) -> u16 {
        #[cfg(not(test))]
        {
            GetStorageBayCapacity()
        }
        #[cfg(test)]
        {
            0
        }
    }
}

impl ShipStatusPort for CffiShipStatus {
    fn apply(&mut self, delta: &ShipDelta) -> Result<(), AdapterError> {
        #[cfg(not(test))]
        #[cfg(not(test))]
        unsafe {
            if delta.crew != 0 {
                DeltaSISGauges(delta.crew, 0, 0);
            }
            WorldFields::current().write(|world| {
                let sis = &mut world.flagship;
                if delta.landers < 0 {
                    sis.num_landers = sis.num_landers.saturating_sub(delta.landers.unsigned_abs());
                } else if delta.landers > 0 {
                    sis.num_landers = sis.num_landers.saturating_add(delta.landers as u8);
                }
            });
            if delta.landers != 0 {
                draw_landers();
            }
            WorldFields::current().write(|world| {
                let sis = &mut world.flagship;
                for (current, added) in sis.element_amounts.iter_mut().zip(delta.element_amounts) {
                    *current = current.saturating_add(added);
                }
                sis.total_element_mass = sis.total_element_mass.saturating_add(delta.element_mass);
                sis.total_bio_mass = sis.total_bio_mass.saturating_add(delta.biological_mass);
            });
            if delta.element_mass != 0 {
                DrawStorageBays(0);
            }
        }
        #[cfg(test)]
        let _ = delta;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_linked_input_is_neutral() {
        assert_eq!(read_player_one_input(), FrameInput::default());
    }

    #[test]
    fn non_linked_clock_is_available_for_unit_tests() {
        let mut clock = CffiPlanetSideClock;
        assert_eq!(clock.now(), Tick(0));
        assert_eq!(clock.sleep_until(Tick(1)), Ok(()));
    }

    #[test]
    fn non_linked_ship_snapshot_is_neutral() {
        let mut ship = CffiShipStatus;
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
                }
            }
            fn checkpoint(&mut self, _: &CheckpointView<'_>) {}
            fn presented(&mut self, _: &mut Presented) {}
        }

        for stop in [
            Some(InputView::BeforeUpdate),
            Some(InputView::AfterUpdate),
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
            let input = CffiPlanetSideInput
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
                expected.push((InputView::AfterUpdate, None));
            }
            assert_eq!(observer.inputs, expected);
        }
    }
}
