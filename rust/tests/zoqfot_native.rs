mod native_ship_common;
use native_ship_common::*;
use std::mem;
use uqm_rust::ships::native::{abi::*, cosine, element, frame_index, sine};
unsafe extern "C" {
    fn init_zoqfotpik() -> *mut RACE_DESC;
}
#[test]
fn descriptor_and_spit_animation_preserve_jitter_rng_and_speed() {
    let _battle = Battle::new(32);
    unsafe {
        let mut ship = Ship::new(init_zoqfotpik, 0);
        assert_eq!(ship.desc.ship_info.ship_flags, FIRES_FORE as u16);
        assert_eq!(
            (
                ship.desc.ship_info.crew_level,
                ship.desc.ship_info.energy_level
            ),
            (10, 10)
        );
        assert_eq!(
            (ship.desc.fleet.known_loc.x, ship.desc.fleet.known_loc.y),
            (3761, 5333)
        );
        assert_eq!(ship.desc.cyborg_control.WeaponRange, 400);
        let h = with_rng(19, 0, || ship.weapon(1))[0];
        let e = element(h);
        assert_eq!(
            (
                (*e).__bindgen_anon_1.life_span,
                (*e).__bindgen_anon_2.hit_points,
                (*e).__bindgen_anon_3.mass_points
            ),
            (10, 1, 1)
        );
        assert_eq!(velocity(&(*e).velocity), toward(16 << ONE_SHIFT << 1, 0));
        TFB_SeedRandom(19);
        let angle = ((TFB_Random() as COUNT) % 3).wrapping_sub(1);
        with_rng(19, 1, || ((*e).preprocess_func.unwrap())(e));
        assert_eq!(frame_index((*e).next.image.frame), 1);
        assert_eq!((*e).__bindgen_anon_4.turn_wait, 2);
        let mut v: VELOCITY_DESC = mem::zeroed();
        let speed = (15 << ONE_SHIFT) << 1 << VELOCITY_SHIFT;
        SetVelocityComponents(
            &mut v,
            cosine(i32::from(angle), speed),
            sine(i32::from(angle), speed),
        );
        assert_eq!(velocity(&(*e).velocity), velocity(&v));
        for expected in [1, 0] {
            with_rng(19, 0, || ((*e).preprocess_func.unwrap())(e));
            assert_eq!((*e).__bindgen_anon_4.turn_wait, expected);
            assert_eq!(frame_index((*e).next.image.frame), 1);
        }
        with_rng(19, 0, || ((*e).preprocess_func.unwrap())(e));
        assert_eq!(frame_index((*e).next.image.frame), 2);
        assert_eq!((*e).__bindgen_anon_4.turn_wait, 2);
        FreeElement(h);
    }
}
#[test]
fn tongue_chain_extends_in_facing_direction_and_stops_at_zero() {
    let _battle = Battle::new(64);
    unsafe {
        let mut ship = Ship::new(init_zoqfotpik, 0);
        ship.ss.ShipFacing = 0;
        ship.ss.cur_status_flags = SPECIAL as u16;
        with_rng(3, 0, || (ship.desc.postprocess_func.unwrap())(ship.ptr()));
        assert_eq!(ship.desc.ship_info.energy_level, 3);
        assert_eq!(ship.ss.special_counter, 6);
        let mut e = element(disp_q.tail);
        assert_eq!((*e).__bindgen_anon_4.turn_wait, 6);
        for remaining in (0..6).rev() {
            (*e).next = (*e).current;
            let y = (*e).next.location.y;
            with_rng(3, 0, || ((*e).postprocess_func.unwrap())(e));
            e = element(disp_q.tail);
            assert_eq!((*e).__bindgen_anon_4.turn_wait, remaining);
            assert_eq!((*e).current.location.y, y - 8);
            assert_eq!(
                (
                    (*e).__bindgen_anon_1.life_span,
                    (*e).__bindgen_anon_3.mass_points
                ),
                (1, 12)
            );
            assert_eq!(velocity(&(*e).velocity), [0; 9]);
        }
        let all = queued();
        assert_eq!(all.len(), 8);
        ((*e).postprocess_func.unwrap())(e);
        assert_eq!(queued(), all);
    }
}
#[test]
fn first_tongue_collision_damages_but_later_frames_only_cancel_extension() {
    let _battle = Battle::new(64);
    unsafe {
        let mut ship = Ship::new(init_zoqfotpik, 0);
        let enemy = Ship::new(init_zoqfotpik, 1);
        ship.ss.cur_status_flags = SPECIAL as u16;
        (ship.desc.postprocess_func.unwrap())(ship.ptr());
        let tongue = element(disp_q.tail);
        let mut a = POINT { x: 100, y: 100 };
        let mut b = POINT { x: 110, y: 100 };
        with_rng(7, 0, || {
            ((*tongue).collision_func.unwrap())(tongue, &mut a, enemy.ptr(), &mut b)
        });
        assert_eq!(enemy.desc.ship_info.crew_level, 0);
        assert_eq!(ship.ss.special_counter, 0);
        assert_eq!((*tongue).__bindgen_anon_4.turn_wait, 0);
        assert_ne!((*tongue).state_flags & NONSOLID as u16, 0);
        // A later frame has no damage window even if it still has mass 12.
        ship.ss.special_counter = 5;
        let mut target: ELEMENT = mem::zeroed();
        target.__bindgen_anon_1.life_span = 1;
        (*tongue).__bindgen_anon_4.turn_wait = 4;
        let before = queued();
        ((*tongue).collision_func.unwrap())(tongue, &mut a, &mut target, &mut b);
        assert_eq!(ship.ss.special_counter, 1);
        assert_eq!(queued(), before);
    }
}
#[test]
fn tongue_allocation_failure_still_spends_energy_and_sets_counter() {
    let _battle = Battle::new(1);
    unsafe {
        let mut ship = Ship::new(init_zoqfotpik, 0);
        ship.ss.cur_status_flags = SPECIAL as u16;
        (ship.desc.postprocess_func.unwrap())(ship.ptr());
        assert_eq!(ship.desc.ship_info.energy_level, 3);
        assert_eq!(ship.ss.special_counter, 6);
        assert_eq!(queued(), vec![ship.handle]);
    }
}
#[test]
fn ai_tongue_range_and_facing_suppress_spit_without_energy_gate() {
    let _battle = Battle::new(32);
    unsafe {
        let mut ship = Ship::new(init_zoqfotpik, 0);
        let enemy = Ship::new(init_zoqfotpik, 1);
        (*enemy.ptr()).next.location = POINT { x: 4000, y: 3900 };
        ship.desc.ship_info.energy_level = 0;
        for (turn, face, tongue) in [(4, 0, true), (4, 1, false), (5, 0, false)] {
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize] = EVALUATE_DESC {
                ObjectPtr: enemy.ptr(),
                facing: 0,
                which_turn: turn,
                MoveState: MOVEMENT_STATE_ENTICE,
            };
            ship.ss.ShipFacing = face;
            ship.ss.ship_input_state = WEAPON as u8;
            with_rng(5, 0, || ship.intelligence(&mut concerns));
            assert_eq!(ship.ss.ship_input_state & SPECIAL as u8 != 0, tongue);
            assert_eq!(ship.ss.ship_input_state & WEAPON as u8, 0);
            assert_eq!(ship.ss.weapon_counter, 0);
            assert_eq!(
                concerns[ENEMY_SHIP_INDEX as usize].MoveState,
                if turn <= 4 {
                    MOVEMENT_STATE_PURSUE
                } else {
                    MOVEMENT_STATE_ENTICE
                }
            );
        }
    }
}
