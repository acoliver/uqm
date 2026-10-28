#[path = "support/battle_context.rs"]
mod battle_context;

mod native_ship_common;
use native_ship_common::*;
use std::{mem, ptr};
use uqm_rust::ships::native::{abi::*, element, frame_index, put_element};
unsafe extern "C" {
    fn init_black_urquan() -> *mut RACE_DESC;
}

#[test]
fn descriptor_saw_launch_hold_release_decelerate_and_tracking() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(init_black_urquan, 0);
            assert_eq!(ship.desc.ship_info.ship_cost, 30);
            assert_eq!(
                (
                    ship.desc.ship_info.crew_level,
                    ship.desc.ship_info.energy_level
                ),
                (42, 42)
            );
            assert_eq!(
                (ship.desc.fleet.known_loc.x, ship.desc.fleet.known_loc.y),
                (6000, 6250)
            );
            assert_eq!(ship.desc.cyborg_control.WeaponRange, 200);
            ship.ss.ShipFacing = 4;
            let h = with_rng(9, 0, || ship.weapon(1))[0];
            let saw = element(h);
            assert_eq!(
                (
                    (*saw).__bindgen_anon_1.life_span,
                    (*saw).__bindgen_anon_2.hit_points,
                    (*saw).__bindgen_anon_3.mass_points
                ),
                (64, 10, 4)
            );
            assert_eq!(velocity(&(*saw).velocity), toward(64, 4));
            ship.ss.cur_status_flags = WEAPON as u16;
            ship.desc.characteristics.special_wait = 0;
            with_rng(9, 0, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*saw).preprocess_func.unwrap(),
                    saw,
                )
            });
            assert_eq!((*saw).__bindgen_anon_1.life_span, 65);
            assert_eq!(frame_index((*saw).next.image.frame), 1);
            ship.ss.cur_status_flags = 0;
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*saw).preprocess_func.unwrap(),
                saw,
            );
            assert_eq!((*saw).__bindgen_anon_1.life_span, 33);
            ship.desc.characteristics.special_wait = 0;
            let decelerate = (*saw).preprocess_func.unwrap();
            for speed in [1024, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1, 0] {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    decelerate, saw,
                );
                let mut dx = 0;
                let mut dy = 0;
                GetCurrentVelocityComponents(&raw mut (*saw).velocity, &mut dx, &mut dy);
                assert_eq!((dx, dy), (speed, 0));
                ship.desc.characteristics.special_wait = 0;
            }
            with_rng(9, 0, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*saw).preprocess_func.unwrap(),
                    saw,
                )
            });
            assert_eq!(velocity(&(*saw).velocity), [0; 9]);
            FreeElement(h);
        }
    })
}
#[test]
fn ninth_saw_expires_and_postprocess_requeues_after_ship_with_intersection_points() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(init_black_urquan, 0);
            let h = ship.weapon(1)[0];
            let saw = element(h);
            put_element(h);
            (*saw).next.location = POINT { x: 1235, y: 2347 };
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*saw).postprocess_func.unwrap(),
                saw,
            );
            let all = queued();
            assert_eq!(all.len(), 3);
            let copy = element(all[1]);
            assert_eq!(all[2], h);
            assert_eq!((*saw).__bindgen_anon_1.life_span, 0);
            assert!((*copy).postprocess_func.is_none());
            assert_eq!(
                ((*copy).current.location.x, (*copy).current.location.y),
                (1235, 2347)
            );
            assert_eq!(
                (
                    (*copy).IntersectControl.EndPoint.x,
                    (*copy).IntersectControl.EndPoint.y
                ),
                (1235 >> ONE_SHIFT, 2347 >> ONE_SHIFT)
            );
            assert_eq!(
                (*copy).state_flags & (APPEARING | CHANGING | PRE_PROCESS) as u16,
                0
            );
            assert_ne!((*copy).state_flags & POST_PROCESS as u16, 0);
            ship.desc.characteristics.special_wait = 8;
            ship.ss.cur_status_flags = WEAPON as u16;
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*copy).preprocess_func.unwrap(),
                copy,
            );
            assert_eq!((*copy).__bindgen_anon_1.life_span, 1);
            assert_ne!((*copy).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(ship.desc.characteristics.special_wait, 9);
            ship.ss.old_status_flags = WEAPON as u16;
            (ship.desc.preprocess_func.unwrap())(ship.ptr());
            assert_eq!(ship.desc.characteristics.special_wait, 0);
            assert_eq!(ship.ss.weapon_counter, 1);
        }
    })
}
#[test]
fn fried_launches_sixteen_clouds_in_facing_order_with_inherited_velocity() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(init_black_urquan, 0);
            ship.ss.cur_status_flags = SPECIAL as u16;
            SetVelocityComponents(&raw mut (*ship.ptr()).velocity, 41, -21);
            with_rng(12, 0, || (ship.desc.postprocess_func.unwrap())(ship.ptr()));
            assert_eq!(ship.desc.ship_info.energy_level, 21);
            assert_eq!(ship.ss.special_counter, 9);
            let all = queued();
            assert_eq!(all.len(), 17);
            for (face, &h) in all[1..].iter().enumerate() {
                let e = element(h);
                assert_eq!(
                    (
                        (*e).__bindgen_anon_1.life_span,
                        (*e).__bindgen_anon_2.hit_points,
                        (*e).__bindgen_anon_3.mass_points
                    ),
                    (47, 100, 3)
                );
                assert_eq!((*e).__bindgen_anon_4.turn_wait, 1);
                let mut expected: VELOCITY_DESC = mem::zeroed();
                SetVelocityVector(&mut expected, 16, face as COUNT);
                DeltaVelocityComponents(&mut expected, 41, -21);
                assert_eq!(velocity(&(*e).velocity), velocity(&expected));
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*e).preprocess_func.unwrap(),
                    e,
                );
                assert_eq!((*e).__bindgen_anon_4.turn_wait, 0);
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*e).preprocess_func.unwrap(),
                    e,
                );
                assert_eq!(frame_index((*e).next.image.frame), 1);
                assert_eq!((*e).__bindgen_anon_4.turn_wait, 2);
                assert!((*e).hTarget.is_null());
            }
        }
    })
}
#[test]
fn saw_collision_splinters_and_cloud_damage_depends_on_ship_flag() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(init_black_urquan, 0);
            let h = ship.weapon(1)[0];
            let saw = element(h);
            let mut target: ELEMENT = mem::zeroed();
            target.playerNr = 1;
            target.__bindgen_anon_1.life_span = 2;
            let mut p = POINT { x: 100, y: 200 };
            uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                (*saw).collision_func.unwrap(),
                saw,
                &mut p,
                &mut target,
                &mut p,
            );
            assert_eq!((*saw).__bindgen_anon_1.life_span, 5);
            assert_eq!(frame_index((*saw).next.image.frame), 2);
            assert_eq!((*saw).state_flags & DISAPPEARING as u16, 0);
            assert_ne!((*saw).state_flags & NONSOLID as u16, 0);
            FreeElement(h);
            ship.ss.cur_status_flags = SPECIAL as u16;
            (ship.desc.postprocess_func.unwrap())(ship.ptr());
            let cloud = element(disp_q.tail);
            let collide = (*cloud).collision_func.unwrap();
            uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                collide,
                cloud,
                &mut p,
                &mut target,
                &mut p,
            );
            assert_eq!((*cloud).__bindgen_anon_3.mass_points, 50);
            target.state_flags = PLAYER_SHIP as u16;
            // Already collided: weapon_collision returns without needing target parent.
            uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                collide,
                cloud,
                &mut p,
                &mut target,
                &mut p,
            );
            assert_eq!((*cloud).__bindgen_anon_3.mass_points, 3);
        }
    })
}
#[test]
fn ai_crew_threat_boundaries_and_no_target_do_not_draw_rng() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(init_black_urquan, 0);
            let mut threat: ELEMENT = mem::zeroed();
            threat.state_flags = CREW_OBJECT as u16;
            for (turn, pursue) in [(8, true), (9, false)] {
                let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
                    ObjectPtr: &mut threat,
                    facing: 0,
                    which_turn: turn,
                    MoveState: MOVEMENT_STATE_ENTICE,
                };
                with_rng(4, 0, || ship.intelligence(&mut concerns));
                assert_eq!(
                    concerns[ENEMY_WEAPON_INDEX as usize].MoveState,
                    if pursue {
                        MOVEMENT_STATE_PURSUE
                    } else {
                        MOVEMENT_STATE_ENTICE
                    }
                );
                assert_eq!(ship.ss.ship_input_state & SPECIAL as u8 != 0, pursue);
            }
            let mut concerns = mem::zeroed();
            with_rng(4, 0, || ship.intelligence(&mut concerns));
            assert_eq!(ship.ss.ship_input_state & SPECIAL as u8, 0);
            let mut out = ptr::null_mut();
            assert_eq!(
                (ship.desc.init_weapon_func.unwrap())(ship.ptr(), &mut out),
                1
            );
            FreeElement(out);
        }
    })
}
