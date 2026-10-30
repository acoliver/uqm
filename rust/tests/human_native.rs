//! Exercise the Cruiser descriptor and Native callbacks against the linked engine.

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::*;
use std::{ffi::CStr, mem};
use uqm_rust::ships::native::{abi::*, element};

#[test]
fn cruiser_descriptor_and_registered_weapon_and_defense_callbacks() {
    battle_context::run(|| {
        let battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(&battle, SPECIES_ID_EARTHLING_ID, 0);
            let desc = descriptor(SPECIES_ID_EARTHLING_ID);
            assert_eq!(
                desc.ship_info.ship_flags,
                (FIRES_FORE | SEEKING_WEAPON | POINT_DEFENSE) as u16
            );
            assert_eq!(desc.ship_info.ship_cost, 11);
            assert_eq!(desc.ship_info.crew_level, 18);
            assert_eq!(desc.ship_info.max_crew, 18);
            assert_eq!(desc.ship_info.energy_level, 18);
            assert_eq!(desc.ship_info.max_energy, 18);
            assert_eq!(
                CStr::from_ptr(desc.ship_info.race_strings_rsc),
                c"ship.earthling.text"
            );
            assert_eq!(
                CStr::from_ptr(desc.ship_data.weapon_rsc[0]),
                c"ship.earthling.graphics.saturn.large"
            );
            assert_eq!(desc.characteristics.special_energy_cost, 4);
            assert_eq!(desc.characteristics.special_wait, 9);
            assert_eq!(desc.cyborg_control.WeaponRange, 4000);
            assert!(desc.race.is_some());
            assert_eq!(desc.callbacks()[0], 0);
            assert_eq!(desc.callbacks()[1], 0);
            assert_ne!(desc.callbacks()[2], 0);
            assert_ne!(desc.callbacks()[3], 0);
            assert_ne!(desc.callbacks()[4], 0);
            assert!(desc.data.is_null());
            assert!(desc.code_ref.is_null());

            ship.element_mut().next.location = POINT { x: 1000, y: 2000 };
            let weapons = ship.weapon(1);
            let nuke = ship.weapon_record(weapons[0]);
            assert_eq!((*nuke).__bindgen_anon_1.life_span, 60);
            assert_eq!((*nuke).__bindgen_anon_2.hit_points, 1);
            assert_eq!((*nuke).__bindgen_anon_3.mass_points, 4);
            assert_eq!((*nuke).__bindgen_anon_4.turn_wait, 3);
            assert_eq!((*nuke).__bindgen_anon_5.blast_offset, 8);
            for expected_wait in [2, 1, 0, 3] {
                call_element("ships::races::human_native::nuke_preprocess", id(nuke));
                assert_eq!((*nuke).__bindgen_anon_4.turn_wait, expected_wait);
            }
            (*nuke).__bindgen_anon_1.life_span = 50;
            call_element("ships::races::human_native::nuke_preprocess", id(nuke));
            let mut expected: VELOCITY_DESC = mem::zeroed();
            SetVelocityVector(&raw mut expected, 80, 0);
            assert_eq!((*nuke).velocity.vector.width, expected.vector.width);
            assert_eq!((*nuke).velocity.vector.height, expected.vector.height);
            FreeElement(nuke.cast());

            // Special creates a deferred scan only when pressed and off cooldown.
            let before = queued();
            ship.postprocess();
            assert_eq!(queued(), before);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.ship_mut().special_counter = 1;
            ship.postprocess();
            assert_eq!(queued(), before);
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            let defense = element(disp_q.tail);
            assert_ne!(defense, ship.ptr());
            assert_eq!(
                (*defense).state_flags,
                (APPEARING | NONSOLID | FINITE_LIFE) as u16
            );
            assert_eq!((*defense).pParent, (*ship.ptr()).pParent);
            assert!((*defense).death_func.is_some());
            call_element(
                "ships::races::human_native::spawn_point_defense",
                id(defense),
            );
            assert_eq!(
                ship.descriptor_mut().ship_info.energy_level,
                18,
                "no targets, no energy payment"
            );
            assert_eq!(ship.ship_mut().special_counter, 0);
        }
    })
}
