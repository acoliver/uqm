//! Mycon native entry point and callbacks linked against the engine archive.

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::*;

use std::ffi::CStr;
use uqm_rust::ships::native::{abi::*, element};

#[test]
fn descriptor_and_plasmoid_callbacks_match_mycon_c_data() {
    battle_context::run(|| unsafe {
        let d = descriptor(SPECIES_ID_MYCON_ID);
        assert_eq!(d.ship_info.ship_flags, (FIRES_FORE | SEEKING_WEAPON) as u16);
        assert_eq!(d.ship_info.ship_cost, 21);
        assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (20, 20));
        assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (40, 40));
        assert_eq!(
            CStr::from_ptr(d.ship_info.race_strings_rsc),
            c"ship.mycon.text"
        );
        assert_eq!(CStr::from_ptr(d.ship_info.icons_rsc), c"ship.mycon.icons");
        assert_eq!(
            CStr::from_ptr(d.ship_info.melee_icon_rsc),
            c"ship.mycon.meleeicons"
        );
        assert_eq!(d.fleet.strength, 194);
        assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (6392, 2200));
        assert_eq!(
            (
                d.characteristics.max_thrust,
                d.characteristics.thrust_increment
            ),
            (27, 9)
        );
        assert_eq!(
            (
                d.characteristics.weapon_energy_cost,
                d.characteristics.special_energy_cost
            ),
            (20, 40)
        );
        assert_eq!(
            (
                d.characteristics.energy_wait,
                d.characteristics.turn_wait,
                d.characteristics.thrust_wait,
                d.characteristics.weapon_wait,
                d.characteristics.special_wait,
                d.characteristics.ship_mass
            ),
            (4, 6, 6, 5, 0, 7)
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.ship_rsc[0]),
            c"ship.mycon.graphics.podship.large"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.weapon_rsc[2]),
            c"ship.mycon.graphics.plasma.small"
        );
        assert!(d.ship_data.special_rsc.iter().all(|r| r.is_null()));
        assert_eq!(d.cyborg_control.WeaponRange, 3200);
        assert!(d.callbacks()[4] != 0);
        assert!(d.callbacks()[2] != 0);
        assert!(d.callbacks()[3] != 0);
        assert!(d.callbacks()[1] == 0);
    })
}

#[test]
fn plasma_launch_and_postprocess_boundaries_match_c() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let mut fixture = Ship::bare(&_battle, SPECIES_ID_MYCON_ID, 0);
        let ship = fixture.ptr();
        (*ship).next.location = POINT { x: 100, y: 200 };
        let slots = fixture.weapon(1);
        let handles: [HELEMENT; 1] = [fixture.weapon_record(slots[0]).cast()];
        assert!(!handles[0].is_null());
        let plasma = element(handles[0]);
        assert_eq!((*plasma).__bindgen_anon_1.life_span, 143);
        assert_eq!((*plasma).__bindgen_anon_2.hit_points, 10);
        assert_eq!((*plasma).__bindgen_anon_3.mass_points, 10);
        assert_eq!((*plasma).__bindgen_anon_4.turn_wait, 3);
        assert_eq!((*plasma).__bindgen_anon_5.blast_offset, 0);
        (*plasma).next = (*plasma).current;
        (*plasma).__bindgen_anon_1.life_span = 100;
        uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
            (*plasma).preprocess_func.unwrap(),
            plasma,
        );
        assert_eq!((*plasma).__bindgen_anon_2.hit_points, 7);
        assert_eq!((*plasma).__bindgen_anon_3.mass_points, 7);
        assert_eq!((*plasma).__bindgen_anon_4.turn_wait, 2);
        (*plasma).__bindgen_anon_2.hit_points = 3;
        uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
            (*plasma).preprocess_func.unwrap(),
            plasma,
        );
        assert_eq!((*plasma).__bindgen_anon_1.life_span, 39);
        assert_eq!((*plasma).__bindgen_anon_3.mass_points, 3);
        assert_eq!((*plasma).__bindgen_anon_4.turn_wait, 1);
        (*ship).__bindgen_anon_2.crew_level = 17;
        fixture.ship_mut().cur_status_flags = SPECIAL as u16;
        fixture.postprocess();
        assert_eq!((*ship).__bindgen_anon_2.crew_level, 20);
        assert_eq!(fixture.descriptor_mut().ship_info.energy_level, 0);
        assert_eq!(fixture.ship_mut().special_counter, 0);

        (*ship).__bindgen_anon_2.crew_level = 10;
        fixture.descriptor_mut().ship_info.energy_level = 39;
        fixture.postprocess();
        assert_eq!((*ship).__bindgen_anon_2.crew_level, 10);
        assert_eq!(fixture.descriptor_mut().ship_info.energy_level, 39);
        FreeElement(handles[0]);
    })
}
