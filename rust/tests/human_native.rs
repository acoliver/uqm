//! Exercise the race entry point and registered callbacks against the linked engine.

#[path = "support/battle_context.rs"]
mod battle_context;

use std::{ffi::CStr, mem, ptr};
use uqm_rust::ships::native::{abi::*, element};

unsafe extern "C" {
    fn init_human() -> *mut RACE_DESC;
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> i32;
    fn UninitQueue(queue: *mut QUEUE) -> i32;
    fn InitDisplayList();
    fn FreeElement(handle: HELEMENT);
}

#[test]
fn cruiser_descriptor_and_registered_weapon_and_defense_callbacks() {
    battle_context::run(|| {
        unsafe {
            let template = init_human();
            assert_eq!(template, init_human());
            let mut desc = *template;
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
            assert!(desc.cyborg_control.intelligence_func.is_some());
            assert!(desc.preprocess_func.is_none());
            assert!(desc.uninit_func.is_none());
            assert!(desc.data.is_null());
            assert!(desc.CodeRef.is_null());

            assert_ne!(
                InitQueue(&raw mut disp_q, 8, mem::size_of::<ELEMENT>() as u16),
                0
            );
            InitDisplayList();
            let mut ss: STARSHIP = mem::zeroed();
            ss.RaceDescPtr = &raw mut desc;
            ss.ShipFacing = 0;
            let mut ship: ELEMENT = mem::zeroed();
            ship.pParent = (&raw mut ss).cast();
            ship.state_flags = PLAYER_SHIP as u16;
            ship.next.location = POINT { x: 1000, y: 2000 };
            let mut weapons = [ptr::null_mut(); 1];
            let fire = desc.init_weapon_func.unwrap();
            assert_eq!(fire(&raw mut ship, weapons.as_mut_ptr()), 1);
            let nuke = element(weapons[0]);
            assert!(!nuke.is_null());
            assert_eq!((*nuke).__bindgen_anon_1.life_span, 60);
            assert_eq!((*nuke).__bindgen_anon_2.hit_points, 1);
            assert_eq!((*nuke).__bindgen_anon_3.mass_points, 4);
            assert_eq!((*nuke).__bindgen_anon_4.turn_wait, 3);
            assert_eq!((*nuke).__bindgen_anon_5.blast_offset, 8);
            (*nuke).next = (*nuke).current;
            let preprocess = (*nuke).preprocess_func.unwrap();
            for expected_wait in [2, 1, 0, 3] {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    preprocess, nuke,
                );
                assert_eq!((*nuke).__bindgen_anon_4.turn_wait, expected_wait);
            }
            (*nuke).__bindgen_anon_1.life_span = 50;
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                preprocess, nuke,
            );
            let mut expected: VELOCITY_DESC = mem::zeroed();
            SetVelocityVector(&raw mut expected, 80, 0);
            assert_eq!((*nuke).velocity.vector.width, expected.vector.width);
            assert_eq!((*nuke).velocity.vector.height, expected.vector.height);
            FreeElement(weapons[0]);

            // Special creates a deferred scan only when pressed and off cooldown.
            let postprocess = desc.postprocess_func.unwrap();
            postprocess(&raw mut ship);
            assert!(disp_q.tail.is_null());
            ss.cur_status_flags = SPECIAL as u16;
            ss.special_counter = 1;
            postprocess(&raw mut ship);
            assert!(disp_q.tail.is_null());
            ss.special_counter = 0;
            postprocess(&raw mut ship);
            let defense_handle = disp_q.tail;
            let defense = element(defense_handle);
            assert!(!defense.is_null());
            assert_eq!(
                (*defense).state_flags,
                (APPEARING | NONSOLID | FINITE_LIFE) as u16
            );
            assert_eq!((*defense).pParent, ship.pParent);
            assert!((*defense).death_func.is_some());
            ss.hShip = (&raw mut ship).cast();
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*defense).death_func.unwrap(),
                defense,
            );
            assert_eq!(
                desc.ship_info.energy_level, 18,
                "no targets, no energy payment"
            );
            assert_eq!(ss.special_counter, 0);
            UninitQueue(&raw mut disp_q);
        }
    })
}
