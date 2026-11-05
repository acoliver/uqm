//! Descriptor construction from static resource tokens, inside the accessor boundary.
use crate::engine::battle::{
    record::CHARACTERISTIC_STUFF,
    views::{Info, Intelligence},
    Race, RaceDesc, RaceResources, RaceTemplate,
};
use crate::graphics::native_abi::POINT;
const MAX_ENERGY: u8 = 24;
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 8;
const SHIP_MASS: u8 = 6;
const SPECIAL_ENERGY_COST: u8 = 2;
const MAX_CREW: u16 = 20;
const MAX_THRUST: u16 = 24;
const THRUST_INCREMENT: u16 = 3;
const TURN_WAIT: u8 = 4;
const THRUST_WAIT: u8 = 0;
const WEAPON_ENERGY_COST: u8 = 3;
const WEAPON_WAIT: u8 = 0;
const SPECIAL_WAIT: u8 = 0;
const LONG_RANGE_WEAPON: i32 = 1000 << 2;
const INFINITE_RADIUS: u16 = u16::MAX;
const MAX_UNIVERSE: i32 = 9999;

pub fn descriptor(race: &'static Race) -> RaceDesc {
    RaceTemplate {
        info: Info {
            ship_flags: 128 | 4,
            ship_cost: 15,
            crew_level: MAX_CREW,
            max_crew: MAX_CREW,
            energy_level: MAX_ENERGY,
            max_energy: MAX_ENERGY,
        },
        fleet_strength: INFINITE_RADIUS,
        fleet_location: POINT {
            x: (MAX_UNIVERSE >> 1) as i16,
            y: (MAX_UNIVERSE >> 1) as i16,
        },
        characteristics: CHARACTERISTIC_STUFF {
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
        },
        resources: RaceResources {
            strings: Some(c"ship.androsynth.text"),
            icons: Some(c"ship.androsynth.icons"),
            melee_icon: Some(c"ship.androsynth.meleeicons"),
            ship: [
                Some(c"ship.androsynth.graphics.guardian.large"),
                Some(c"ship.androsynth.graphics.guardian.medium"),
                Some(c"ship.androsynth.graphics.guardian.small"),
            ],
            weapon: [
                Some(c"ship.androsynth.graphics.bubble.large"),
                Some(c"ship.androsynth.graphics.bubble.medium"),
                Some(c"ship.androsynth.graphics.bubble.small"),
            ],
            special: [
                Some(c"ship.androsynth.graphics.blazer.large"),
                Some(c"ship.androsynth.graphics.blazer.medium"),
                Some(c"ship.androsynth.graphics.blazer.small"),
            ],
            captain: Some(c"ship.androsynth.graphics.captain"),
            victory: Some(c"ship.androsynth.ditty"),
            sounds: Some(c"ship.androsynth.sounds"),
        },
        intelligence: Intelligence {
            maneuverability: 0,
            weapon_range: (LONG_RANGE_WEAPON >> 2) as u16,
        },
        ..RaceTemplate::default()
    }
    .descriptor(race)
}
