//! The noncombat drone has a descriptor but no battle callbacks.

use std::ffi::CStr;
use uqm_rust::ships::native::abi::*;

unsafe extern "C" {
    fn init_probe() -> *mut RACE_DESC;
}

#[test]
fn drone_descriptor_preserves_resources_stats_and_absent_callbacks() {
    unsafe {
        let address = init_probe();
        assert_eq!(address, init_probe());
        let d = *address;
        let info = d.ship_info;
        assert_eq!((info.ship_flags, info.ship_cost), (0, 0));
        assert_eq!((info.crew_level, info.max_crew), (1, 1));
        assert_eq!((info.energy_level, info.max_energy), (1, 1));
        assert!(info.race_strings_rsc.is_null());
        assert!(info.icons_rsc.is_null());
        assert_eq!(
            CStr::from_ptr(info.melee_icon_rsc),
            c"ship.drone.meleeicons"
        );
        assert!(info.race_strings.is_null());
        assert!(info.icons.is_null());
        assert!(info.melee_icon.is_null());
        assert_eq!(d.fleet.strength, 0);
        assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (0, 0));
        let c = d.characteristics;
        assert_eq!((c.max_thrust, c.thrust_increment), (0, 0));
        assert_eq!(
            (
                c.energy_regeneration,
                c.weapon_energy_cost,
                c.special_energy_cost
            ),
            (0, 0, 0)
        );
        assert_eq!(
            (
                c.energy_wait,
                c.turn_wait,
                c.thrust_wait,
                c.weapon_wait,
                c.special_wait,
                c.ship_mass
            ),
            (0, 0, 0, 0, 0, 0)
        );
        assert!(d.ship_data.ship_rsc.into_iter().all(|p| p.is_null()));
        assert!(d.ship_data.weapon_rsc.into_iter().all(|p| p.is_null()));
        assert!(d.ship_data.special_rsc.into_iter().all(|p| p.is_null()));
        assert!(d.ship_data.ship.into_iter().all(|p| p.is_null()));
        assert!(d.ship_data.weapon.into_iter().all(|p| p.is_null()));
        assert!(d.ship_data.special.into_iter().all(|p| p.is_null()));
        let captain = d.ship_data.captain_control;
        assert!(captain.captain_rsc.is_null());
        assert!(captain.background.is_null());
        assert!(captain.turn.is_null());
        assert!(captain.thrust.is_null());
        assert!(captain.weapon.is_null());
        assert!(captain.special.is_null());
        assert!(d.ship_data.victory_ditty_rsc.is_null());
        assert!(d.ship_data.ship_sounds_rsc.is_null());
        assert!(d.ship_data.victory_ditty.is_null());
        assert!(d.ship_data.ship_sounds.is_null());
        assert_eq!(
            (
                d.cyborg_control.ManeuverabilityIndex,
                d.cyborg_control.WeaponRange
            ),
            (0, 0)
        );
        assert!(d.cyborg_control.intelligence_func.is_none());
        assert!(d.preprocess_func.is_none());
        assert!(d.postprocess_func.is_none());
        assert!(d.init_weapon_func.is_none());
        assert!(d.uninit_func.is_none());
        assert!(d.data.is_null());
        assert!(d.CodeRef.is_null());
    }
}
