#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use std::mem;
use uqm_rust::ships::native::{abi::*, frame_index};

#[test]
fn descriptor_saw_launch_hold_release_decelerate_and_tracking() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_KOHR_AH_ID, 0);
            assert_eq!(ship.descriptor_mut().ship_info.ship_cost, 30);
            assert_eq!(
                (
                    ship.descriptor_mut().ship_info.crew_level,
                    ship.descriptor_mut().ship_info.energy_level
                ),
                (42, 42)
            );
            assert_eq!(
                (
                    ship.descriptor_mut().fleet.known_loc.x,
                    ship.descriptor_mut().fleet.known_loc.y
                ),
                (6000, 6250)
            );
            assert_eq!(ship.descriptor_mut().cyborg_control.WeaponRange, 200);
            ship.ship_mut().ship_facing = 4;
            let h = with_rng(9, 0, || ship.weapon(1))[0];
            let saw = ship.weapon_record(h);
            let h = saw.cast();
            assert_eq!(((*saw).life, (*saw).hits, (*saw).mass), (64, 10, 4));
            assert_eq!(velocity(&(*saw).velocity), toward(64, 4));
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            ship.descriptor_mut().characteristics.special_wait = 0;
            with_rng(9, 0, || {
                hooks::invoke_element_pointer((*saw).preprocess_func.unwrap(), saw)
            });
            assert_eq!((*saw).life, 65);
            assert_eq!(frame_index((*saw).next.image.frame.raw()), 1);
            ship.ship_mut().cur_status_flags = 0;
            hooks::invoke_element_pointer((*saw).preprocess_func.unwrap(), saw);
            assert_eq!((*saw).life, 33);
            ship.descriptor_mut().characteristics.special_wait = 0;
            let decelerate = (*saw).preprocess_func.unwrap();
            for speed in [1024, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1, 0] {
                hooks::invoke_element_pointer(decelerate, saw);
                let (velocity_x, velocity_y) = ((*saw).velocity).get_current_components();
                let (mut dx, mut dy) = (velocity_x as i16, velocity_y as i16);
                assert_eq!((dx, dy), (speed, 0));
                ship.descriptor_mut().characteristics.special_wait = 0;
            }
            with_rng(9, 0, || {
                hooks::invoke_element_pointer((*saw).preprocess_func.unwrap(), saw)
            });
            assert_eq!(velocity(&(*saw).velocity), [0; 9]);
            battle_context::dispose(h);
        }
    })
}
#[test]
fn ninth_saw_expires_and_postprocess_requeues_after_ship_with_intersection_points() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_KOHR_AH_ID, 0);
            let h = ship.weapon(1)[0];
            let saw = ship.weapon_record(h);
            let h = saw.cast();
            battle::put(h);
            (*saw).next.location = POINT { x: 1235, y: 2347 };
            hooks::invoke_element_pointer((*saw).postprocess_func.unwrap(), saw);
            let all = queued();
            assert_eq!(all.len(), 3);
            let copy = battle::element(all[1]);
            assert_eq!(all[2], h);
            assert_eq!((*saw).life, 0);
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
            ship.descriptor_mut().characteristics.special_wait = 8;
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            hooks::invoke_element_pointer((*copy).preprocess_func.unwrap(), copy);
            assert_eq!((*copy).life, 1);
            assert_ne!((*copy).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(ship.descriptor_mut().characteristics.special_wait, 9);
            ship.ship_mut().old_status_flags = WEAPON as u16;
            ship.preprocess();
            assert_eq!(ship.descriptor_mut().characteristics.special_wait, 0);
            assert_eq!(ship.ship_mut().weapon_counter, 1);
        }
    })
}
#[test]
fn fried_launches_sixteen_clouds_in_facing_order_with_inherited_velocity() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_KOHR_AH_ID, 0);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ((*ship.ptr()).velocity).set_components(i32::from(41), i32::from(-21));
            with_rng(12, 0, || ship.postprocess());
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 21);
            assert_eq!(ship.ship_mut().special_counter, 9);
            let all = queued();
            assert_eq!(all.len(), 17);
            for (face, &h) in all[1..].iter().enumerate() {
                let e = battle::element(h);
                assert_eq!(((*e).life, (*e).hits, (*e).mass), (47, 100, 3));
                assert_eq!((*e).turn, 1);
                let mut expected: VELOCITY_DESC = mem::zeroed();
                SetVelocityVector(&mut expected, 16, face as COUNT);
                (expected).delta_components(i32::from(41), i32::from(-21));
                assert_eq!(velocity(&(*e).velocity), velocity(&expected));
                hooks::invoke_element_pointer((*e).preprocess_func.unwrap(), e);
                assert_eq!((*e).turn, 0);
                hooks::invoke_element_pointer((*e).preprocess_func.unwrap(), e);
                assert_eq!(frame_index((*e).next.image.frame.raw()), 1);
                assert_eq!((*e).turn, 2);
                assert!((*e).target.is_none());
            }
        }
    })
}
#[test]
fn saw_collision_splinters_and_cloud_damage_depends_on_ship_flag() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_KOHR_AH_ID, 0);
            let h = ship.weapon(1)[0];
            let saw = ship.weapon_record(h);
            let h = saw.cast();
            let mut target_fixture = Object::new(&_battle);
            let target = target_fixture.element_mut() as *mut ElementRecord;
            (*target).playerNr = 1;
            (*target).life = 2;
            let mut p = POINT { x: 100, y: 200 };
            hooks::invoke_collision_pointer(
                (*saw).collision_func.unwrap(),
                saw,
                &mut p,
                target,
                &mut p,
                hooks::CollisionCall::Hook,
            );
            assert_eq!((*saw).life, 5);
            assert_eq!(frame_index((*saw).next.image.frame.raw()), 2);
            assert_eq!((*saw).state_flags & DISAPPEARING as u16, 0);
            assert_ne!((*saw).state_flags & NONSOLID as u16, 0);
            battle_context::dispose(h);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            let cloud = battle::element(battle::tail());
            let collide = (*cloud).collision_func.unwrap();
            hooks::invoke_collision_pointer(
                collide,
                cloud,
                &mut p,
                target,
                &mut p,
                hooks::CollisionCall::Hook,
            );
            assert_eq!((*cloud).mass, 50);
            (*target).state_flags = PLAYER_SHIP as u16;
            // Already collided: weapon_collision returns without needing target parent.
            hooks::invoke_collision_pointer(
                collide,
                cloud,
                &mut p,
                target,
                &mut p,
                hooks::CollisionCall::Hook,
            );
            assert_eq!((*cloud).mass, 3);
        }
    })
}
#[test]
fn ai_crew_threat_boundaries_and_no_target_do_not_draw_rng() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_KOHR_AH_ID, 0);
            let mut threat_fixture = Object::new(&_battle);
            let threat = threat_fixture.element_mut() as *mut ElementRecord;
            (*threat).state_flags = CREW_OBJECT as u16;
            for (turn, pursue) in [(8, true), (9, false)] {
                let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
                    ObjectPtr: threat,
                    facing: 0,
                    which_turn: turn,
                    MoveState: MOVEMENT_STATE_ENTICE,
                };
                with_rng(4, 0, || {
                    ship.intelligence(&mut *ship.ptr(), &mut concerns, 0)
                });
                assert_eq!(
                    concerns[ENEMY_WEAPON_INDEX as usize].MoveState,
                    if pursue {
                        MOVEMENT_STATE_PURSUE
                    } else {
                        MOVEMENT_STATE_ENTICE
                    }
                );
                assert_eq!(
                    ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                    pursue
                );
            }
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            with_rng(4, 0, || {
                ship.intelligence(&mut *ship.ptr(), &mut concerns, 0)
            });
            assert_eq!(ship.ship_mut().ship_input_state & SPECIAL as u8, 0);
            let out = ship.weapon(1);
            battle_context::dispose(ship.weapon_record(out[0]).cast());
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
