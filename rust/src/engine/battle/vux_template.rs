//! Descriptor construction from static resource tokens, inside the accessor boundary.
#[cfg(not(test))]
use super::record::*;
#[cfg(not(test))]
use crate::graphics::native_abi::POINT;
#[cfg(not(test))]
pub fn descriptor(race: &'static super::Race) -> crate::engine::battle::RaceDesc {
    let mut d = RACE_DESC::EMPTY;
    d.ship_info.ship_flags = (128 | 8 | 32) as _;
    d.ship_info.ship_cost = 12;
    d.ship_info.crew_level = 20;
    d.ship_info.max_crew = 20;
    d.ship_info.energy_level = 40;
    d.ship_info.max_energy = 40;
    d.ship_info.race_strings_rsc = c"ship.vux.text".as_ptr();
    d.ship_info.icons_rsc = c"ship.vux.icons".as_ptr();
    d.ship_info.melee_icon_rsc = c"ship.vux.meleeicons".as_ptr();
    d.fleet.strength = (900 / 11 * 2) as u16;
    d.fleet.known_loc = POINT { x: 4412, y: 1558 };
    d.characteristics.max_thrust = 21;
    d.characteristics.thrust_increment = 7;
    d.characteristics.energy_regeneration = 1;
    d.characteristics.weapon_energy_cost = 1;
    d.characteristics.special_energy_cost = 2;
    d.characteristics.energy_wait = 8;
    d.characteristics.turn_wait = 6;
    d.characteristics.thrust_wait = 4;
    d.characteristics.weapon_wait = 0;
    d.characteristics.special_wait = 7;
    d.characteristics.ship_mass = 6;
    d.ship_data.ship_rsc = [
        c"ship.vux.graphics.intruder.large".as_ptr(),
        c"ship.vux.graphics.intruder.medium".as_ptr(),
        c"ship.vux.graphics.intruder.small".as_ptr(),
    ];
    d.ship_data.weapon_rsc = [
        c"ship.vux.graphics.slime".as_ptr(),
        std::ptr::null(),
        std::ptr::null(),
    ];
    d.ship_data.special_rsc = [
        c"ship.vux.graphics.limpets.large".as_ptr(),
        c"ship.vux.graphics.limpets.medium".as_ptr(),
        c"ship.vux.graphics.limpets.small".as_ptr(),
    ];
    d.ship_data.captain_control.captain_rsc = c"ship.vux.graphics.captain".as_ptr();
    d.ship_data.victory_ditty_rsc = c"ship.vux.ditty".as_ptr();
    d.ship_data.ship_sounds_rsc = c"ship.vux.sounds".as_ptr();
    d.cyborg_control.WeaponRange = (50 << 2) as u16;
    d.race = Some(race);
    crate::engine::battle::RaceDesc::from_legacy(d)
}
