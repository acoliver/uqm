//! Descriptor construction from static resource tokens, inside the accessor boundary.
use crate::engine::battle::{
    record::CHARACTERISTIC_STUFF,
    views::{Info, Intelligence},
    Race, RaceDesc, RaceResources, RaceTemplate,
};
use crate::graphics::native_abi::POINT;

pub fn descriptor(race: &'static Race) -> RaceDesc {
    RaceTemplate {
        info: Info {
            ship_flags: 128 | 8 | 32,
            ship_cost: 12,
            crew_level: 20,
            max_crew: 20,
            energy_level: 40,
            max_energy: 40,
        },
        characteristics: CHARACTERISTIC_STUFF {
            max_thrust: 21,
            thrust_increment: 7,
            energy_regeneration: 1,
            weapon_energy_cost: 1,
            special_energy_cost: 2,
            energy_wait: 8,
            turn_wait: 6,
            thrust_wait: 4,
            weapon_wait: 0,
            special_wait: 7,
            ship_mass: 6,
        },
        fleet_strength: 900 / 11 * 2,
        fleet_location: POINT { x: 4412, y: 1558 },
        intelligence: Intelligence {
            weapon_range: 50 << 2,
            ..Intelligence::default()
        },
        resources: RaceResources {
            strings: Some(c"ship.vux.text"),
            icons: Some(c"ship.vux.icons"),
            melee_icon: Some(c"ship.vux.meleeicons"),
            ship: [
                Some(c"ship.vux.graphics.intruder.large"),
                Some(c"ship.vux.graphics.intruder.medium"),
                Some(c"ship.vux.graphics.intruder.small"),
            ],
            weapon: [Some(c"ship.vux.graphics.slime"), None, None],
            special: [
                Some(c"ship.vux.graphics.limpets.large"),
                Some(c"ship.vux.graphics.limpets.medium"),
                Some(c"ship.vux.graphics.limpets.small"),
            ],
            captain: Some(c"ship.vux.graphics.captain"),
            victory: Some(c"ship.vux.ditty"),
            sounds: Some(c"ship.vux.sounds"),
        },
        ..RaceTemplate::default()
    }
    .descriptor(race)
}
