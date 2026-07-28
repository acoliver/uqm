//! Production-facing PlanetSide adapters available without sharing gameplay state.

use super::runtime::{AdapterError, PlanetSideClock, PlanetSideInput, ShipStatusPort, Tick};
use super::session::{ShipDelta, ShipStatus};
use super::simulation::FrameInput;

#[cfg(feature = "linked_c_archive")]
const KEY_UP: usize = 0;
#[cfg(feature = "linked_c_archive")]
const KEY_LEFT: usize = 2;
#[cfg(feature = "linked_c_archive")]
const KEY_RIGHT: usize = 3;
#[cfg(feature = "linked_c_archive")]
const KEY_WEAPON: usize = 4;
#[cfg(feature = "linked_c_archive")]
const KEY_SPECIAL: usize = 5;
#[cfg(feature = "linked_c_archive")]
const KEY_ESCAPE: usize = 6;
#[cfg(feature = "linked_c_archive")]
const NUM_KEYS: usize = 7;
#[cfg(feature = "linked_c_archive")]
const NUM_TEMPLATES: usize = 6;
#[cfg(feature = "linked_c_archive")]
const NUM_MENU_KEYS: usize = 24;

/// Input adapter reading the resolved player-one control template.
pub struct CffiPlanetSideInput;

impl PlanetSideInput for CffiPlanetSideInput {
    fn poll(&mut self) -> Result<FrameInput, AdapterError> {
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            if crate::automation::input_ffi::rust_automation_service_do_input() != 0 {
                return Ok(FrameInput {
                    abort: true,
                    ..FrameInput::default()
                });
            }
            UpdateInputState();
            if crate::automation::input_ffi::rust_automation_after_input_update() != 0 {
                return Ok(FrameInput {
                    abort: true,
                    ..FrameInput::default()
                });
            }
        }
        Ok(read_player_one_input())
    }
}

#[cfg(feature = "linked_c_archive")]
#[repr(C)]
struct ControllerInputState {
    key: [[i32; NUM_KEYS]; NUM_TEMPLATES],
    menu: [i32; NUM_MENU_KEYS],
}

#[cfg(feature = "linked_c_archive")]
extern "C" {
    static CurrentInputState: ControllerInputState;
    static PlayerControls: [i32; 2];
    #[link_name = "GlobData"]
    static mut GLOB_DATA: crate::comm::locdata::CGlobData;
    fn GetTimeCounter() -> u32;
    fn SleepThreadUntil(wake_time: u32);
    fn UpdateInputState();
    fn DeltaSISGauges(crew_delta: i16, fuel_delta: i16, resunit_delta: i32);
    fn DrawLanders();
    fn DrawStorageBays(refresh: i32);
    fn GetStorageBayCapacity() -> u16;
}

#[cfg(feature = "linked_c_archive")]
fn read_player_one_input() -> FrameInput {
    // The game loop updates these globals synchronously before invoking the
    // active input callback, so one snapshot cannot race another game tick.
    unsafe {
        let template = PlayerControls[0];
        if !(0..NUM_TEMPLATES as i32).contains(&template) {
            return FrameInput::default();
        }
        let key = &CurrentInputState.key[template as usize];
        FrameInput {
            turn_left: key[KEY_LEFT] != 0,
            turn_right: key[KEY_RIGHT] != 0,
            thrust: key[KEY_UP] != 0,
            fire: key[KEY_WEAPON] != 0,
            takeoff: key[KEY_ESCAPE] != 0 || key[KEY_SPECIAL] != 0,
            abort: crate::mainloop::c_extern::get_current_activity() & 0x4000 != 0,
        }
    }
}

#[cfg(not(feature = "linked_c_archive"))]
fn read_player_one_input() -> FrameInput {
    FrameInput::default()
}

/// Main-thread clock adapter preserving the established pump-and-flush sleep path.
pub struct CffiPlanetSideClock;

impl PlanetSideClock for CffiPlanetSideClock {
    fn now(&self) -> Tick {
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            Tick(GetTimeCounter())
        }
        #[cfg(not(feature = "linked_c_archive"))]
        {
            Tick(0)
        }
    }

    fn sleep_until(&mut self, deadline: Tick) -> Result<(), AdapterError> {
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            SleepThreadUntil(deadline.0);
        }
        #[cfg(not(feature = "linked_c_archive"))]
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
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            let sis = std::ptr::addr_of!(GLOB_DATA.sis_state).read();
            ShipStatus {
                crew_enlisted: sis.crew_enlisted,
                landers: sis.num_landers,
                total_element_mass: sis.total_element_mass,
                element_amounts: sis.element_amounts,
                total_bio_mass: sis.total_bio_mass,
            }
        }
        #[cfg(not(feature = "linked_c_archive"))]
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
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            GetStorageBayCapacity()
        }
        #[cfg(not(feature = "linked_c_archive"))]
        {
            0
        }
    }
}

impl ShipStatusPort for CffiShipStatus {
    fn apply(&mut self, delta: &ShipDelta) -> Result<(), AdapterError> {
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            if delta.crew != 0 {
                DeltaSISGauges(delta.crew, 0, 0);
            }

            let sis = std::ptr::addr_of_mut!(GLOB_DATA.sis_state);
            if delta.landers < 0 {
                (*sis).num_landers = (*sis)
                    .num_landers
                    .saturating_sub(delta.landers.unsigned_abs());
                DrawLanders();
            } else if delta.landers > 0 {
                (*sis).num_landers = (*sis).num_landers.saturating_add(delta.landers as u8);
                DrawLanders();
            }

            for (current, added) in (*sis).element_amounts.iter_mut().zip(delta.element_amounts) {
                *current = current.saturating_add(added);
            }
            (*sis).total_element_mass =
                (*sis).total_element_mass.saturating_add(delta.element_mass);
            (*sis).total_bio_mass = (*sis).total_bio_mass.saturating_add(delta.biological_mass);
            if delta.element_mass != 0 {
                DrawStorageBays(0);
            }
        }
        #[cfg(not(feature = "linked_c_archive"))]
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
}
