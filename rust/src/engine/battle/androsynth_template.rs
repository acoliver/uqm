//! Descriptor construction from static resource tokens, inside the accessor boundary.
#[cfg(not(test))]
use super::record::*;
#[cfg(not(test))]
use crate::graphics::native_abi::POINT;
#[cfg(not(test))]
const MAX_ENERGY: u8 = 24;
#[cfg(not(test))]
const ENERGY_REGENERATION: u8 = 1;
#[cfg(not(test))]
const ENERGY_WAIT: u8 = 8;
#[cfg(not(test))]
const SHIP_MASS: u8 = 6;
#[cfg(not(test))]
const SPECIAL_ENERGY_COST: u8 = 2;
#[cfg(not(test))]
const MAX_CREW: u16 = 20;
#[cfg(not(test))]
const MAX_THRUST: u16 = 24;
#[cfg(not(test))]
const THRUST_INCREMENT: u16 = 3;
#[cfg(not(test))]
const TURN_WAIT: u8 = 4;
#[cfg(not(test))]
const THRUST_WAIT: u8 = 0;
#[cfg(not(test))]
const WEAPON_ENERGY_COST: u8 = 3;
#[cfg(not(test))]
const WEAPON_WAIT: u8 = 0;
#[cfg(not(test))]
const SPECIAL_WAIT: u8 = 0;
#[cfg(not(test))]
const LONG_RANGE_WEAPON: i32 = 1000 << 2;
#[cfg(not(test))]
const INFINITE_RADIUS: u16 = u16::MAX;
#[cfg(not(test))]
const MAX_UNIVERSE: i32 = 9999;
#[cfg(not(test))]
pub fn descriptor(race: &'static super::Race) -> crate::engine::battle::RaceDesc {
    // All zero fields are null handles, absent callbacks, or zero counters.
    let mut desc = RACE_DESC::EMPTY;
    desc.ship_info.ship_flags = (128 | 4) as u16;
    desc.ship_info.ship_cost = 15;
    desc.ship_info.crew_level = MAX_CREW;
    desc.ship_info.max_crew = MAX_CREW;
    desc.ship_info.energy_level = MAX_ENERGY;
    desc.ship_info.max_energy = MAX_ENERGY;
    desc.ship_info.race_strings_rsc = c"ship.androsynth.text".as_ptr();
    desc.ship_info.icons_rsc = c"ship.androsynth.icons".as_ptr();
    desc.ship_info.melee_icon_rsc = c"ship.androsynth.meleeicons".as_ptr();
    desc.fleet.strength = INFINITE_RADIUS;
    desc.fleet.known_loc = POINT {
        x: (MAX_UNIVERSE >> 1) as i16,
        y: (MAX_UNIVERSE >> 1) as i16,
    };
    desc.characteristics = CHARACTERISTIC_STUFF {
        max_thrust: MAX_THRUST,
        thrust_increment: THRUST_INCREMENT,
        energy_regeneration: ENERGY_REGENERATION,
        weapon_energy_cost: WEAPON_ENERGY_COST,
        special_energy_cost: SPECIAL_ENERGY_COST,
        energy_wait: ENERGY_WAIT,
        turn_wait: TURN_WAIT,
        thrust_wait: THRUST_WAIT,
        weapon_wait: WEAPON_WAIT,
        special_wait: SPECIAL_WAIT,
        ship_mass: SHIP_MASS,
    };
    desc.ship_data.ship_rsc = [
        c"ship.androsynth.graphics.guardian.large".as_ptr(),
        c"ship.androsynth.graphics.guardian.medium".as_ptr(),
        c"ship.androsynth.graphics.guardian.small".as_ptr(),
    ];
    desc.ship_data.weapon_rsc = [
        c"ship.androsynth.graphics.bubble.large".as_ptr(),
        c"ship.androsynth.graphics.bubble.medium".as_ptr(),
        c"ship.androsynth.graphics.bubble.small".as_ptr(),
    ];
    desc.ship_data.special_rsc = [
        c"ship.androsynth.graphics.blazer.large".as_ptr(),
        c"ship.androsynth.graphics.blazer.medium".as_ptr(),
        c"ship.androsynth.graphics.blazer.small".as_ptr(),
    ];
    desc.ship_data.captain_control.captain_rsc = c"ship.androsynth.graphics.captain".as_ptr();
    desc.ship_data.victory_ditty_rsc = c"ship.androsynth.ditty".as_ptr();
    desc.ship_data.ship_sounds_rsc = c"ship.androsynth.sounds".as_ptr();
    desc.cyborg_control.ManeuverabilityIndex = 0;
    desc.cyborg_control.WeaponRange = (LONG_RANGE_WEAPON >> 2) as u16;
    desc.race = Some(race);
    crate::engine::battle::RaceDesc::from_legacy(desc)
}
