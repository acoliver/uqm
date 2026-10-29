#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::*;

use std::{ffi::CStr, mem};
use uqm_rust::ships::native::{abi::*, element};

use battle::Battle as Battlefield;

#[test]
fn descriptor_matches_druuge_c_data() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let d = descriptor(SPECIES_ID_DRUUGE_ID);
            assert_eq!(d.ship_info.ship_flags, FIRES_FORE as u16);
            assert_eq!(
                (
                    d.ship_info.ship_cost,
                    d.ship_info.crew_level,
                    d.ship_info.max_crew
                ),
                (17, 14, 14)
            );
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (32, 32));
            assert_eq!(
                CStr::from_ptr(d.ship_info.race_strings_rsc),
                c"ship.druuge.text"
            );
            assert_eq!(CStr::from_ptr(d.ship_info.icons_rsc), c"ship.druuge.icons");
            assert_eq!(
                CStr::from_ptr(d.ship_info.melee_icon_rsc),
                c"ship.druuge.meleeicons"
            );
            assert_eq!(d.fleet.strength, 254);
            assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (9500, 2792));
            let c = d.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (20, 2, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (4, 16, 50)
            );
            assert_eq!(
                (
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait,
                    c.special_wait,
                    c.ship_mass
                ),
                (4, 1, 10, 30, 5)
            );
            assert_eq!(d.cyborg_control.WeaponRange, 2400);
            assert!(d.callbacks()[1] != 0 && d.callbacks()[2] != 0);
            assert!(d.callbacks()[3] != 0 && d.callbacks()[4] != 0);
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                assert_eq!(
                    CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                    format!("ship.druuge.graphics.mauler.{size}")
                );
                assert_eq!(
                    CStr::from_ptr(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                    format!("ship.druuge.graphics.cannon.{size}")
                );
                assert!(d.ship_data.special_rsc[i].is_null());
            }
        }
    })
}

#[test]
fn cannon_launch_uses_mass_driver_missile_parameters() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_DRUUGE_ID, 0);
            ship.ship_mut().ShipFacing = 5;
            ship.element_mut().next.location = POINT { x: 100, y: 200 };
            let slots = ship.weapon(1);
            let out = [ship.weapon_record(slots[0]).cast()];
            let missile = element(out[0]);
            assert!(!missile.is_null());
            assert_eq!((*missile).__bindgen_anon_1.life_span, 20);
            assert_eq!((*missile).__bindgen_anon_2.hit_points, 4);
            assert_eq!((*missile).__bindgen_anon_3.mass_points, 6);
            assert_eq!((*missile).__bindgen_anon_5.blast_offset, 6);
            assert_ne!((*missile).state_flags & IGNORE_SIMILAR as u16, 0);
            FreeElement(out[0]);
        }
    })
}

#[test]
fn furnace_burns_one_crew_and_adds_sixteen_energy_at_boundaries() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_DRUUGE_ID, 0);
            (*ship.el()).__bindgen_anon_2.crew_level = 2;
            ship.descriptor_mut().ship_info.energy_level = 15;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.preprocess();
            assert_eq!((*ship.el()).__bindgen_anon_2.crew_level, 1);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 31);
            assert_eq!(ship.ship_mut().special_counter, 30);

            ship.descriptor_mut().ship_info.energy_level = 32;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.ship_mut().special_counter = 0;
            ship.preprocess();
            assert_eq!((*ship.el()).__bindgen_anon_2.crew_level, 1);
            assert_eq!(ship.ship_mut().cur_status_flags & SPECIAL as u16, 0);
        }
    })
}

#[test]
fn intelligence_selects_pursuit_and_furnace_for_weapon_energy_shortfall() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_DRUUGE_ID, 0);
            let mut enemy = Ship::bare(&_battle, SPECIES_ID_DRUUGE_ID, 1);
            ship.descriptor_mut().ship_info.energy_level = 3;
            ship.element_mut().__bindgen_anon_2.crew_level = 14;
            (*ship.el()).__bindgen_anon_5.thrust_wait = 0;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.el();
            concerns[ENEMY_SHIP_INDEX as usize].which_turn = 10;
            ship.intelligence(&mut *ship.ptr(), &mut concerns, 0);
            assert_eq!(
                concerns[ENEMY_SHIP_INDEX as usize].MoveState,
                MOVEMENT_STATE_PURSUE
            );
            assert_eq!((*ship.el()).__bindgen_anon_5.thrust_wait, 11);
            assert_ne!(
                ship.ship_mut().ship_input_state & (WEAPON | SPECIAL) as u8,
                0
            );
            assert_eq!(
                ship.ship_mut().ship_input_state & SPECIAL as u8,
                SPECIAL as u8
            );
        }
    })
}

#[test]
fn cannon_recoil_is_applied_only_on_the_firing_frame_and_caps_speed() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_DRUUGE_ID, 0);
            ship.ship_mut().ShipFacing = 4;
            ship.ship_mut().cur_status_flags = (WEAPON | SHIP_AT_MAX_SPEED) as u16;
            ship.ship_mut().weapon_counter = 9;
            ship.postprocess();
            let (mut dx, mut dy) = (0, 0);
            GetCurrentVelocityComponents(&raw mut ship.element_mut().velocity, &mut dx, &mut dy);
            assert_eq!((dx, dy), (0, 0));
            ship.ship_mut().weapon_counter = 10;
            ship.postprocess();
            GetCurrentVelocityComponents(&raw mut ship.element_mut().velocity, &mut dx, &mut dy);
            assert_eq!((dx, dy), (-768, 0));
            assert_eq!(
                ship.ship_mut().cur_status_flags & SHIP_AT_MAX_SPEED as u16,
                0
            );
            for _ in 0..5 {
                ship.postprocess();
            }
            GetCurrentVelocityComponents(&raw mut ship.element_mut().velocity, &mut dx, &mut dy);
            assert_eq!((dx, dy), (-3072, 0));
        }
    })
}
