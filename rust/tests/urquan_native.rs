#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use std::mem;
use uqm_rust::ships::native::{abi::*, element, frame_index};

#[test]
fn descriptor_and_fusion_match_c() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags,
                (FIRES_FORE | SEEKING_SPECIAL) as u16
            );
            assert_eq!(
                (
                    ship.descriptor_mut().ship_info.crew_level,
                    ship.descriptor_mut().ship_info.energy_level
                ),
                (42, 42)
            );
            assert_eq!(ship.descriptor_mut().cyborg_control.WeaponRange, 1600);
            assert_eq!(
                (
                    ship.descriptor_mut().fleet.known_loc.x,
                    ship.descriptor_mut().fleet.known_loc.y
                ),
                (5750, 6000)
            );
            ship.ship_mut().ShipFacing = 11;
            let h = with_rng(8, 0, || {
                ship.weapon(1)
                    .into_iter()
                    .map(|id| {
                        id.map_or(std::ptr::null_mut(), |id| {
                            ship.weapon_record(Some(id)).cast()
                        })
                    })
                    .collect::<Vec<HELEMENT>>()
            })[0];
            let e = element(h);
            assert_eq!(
                (
                    (*e).__bindgen_anon_1.life_span,
                    (*e).__bindgen_anon_2.hit_points,
                    (*e).__bindgen_anon_3.mass_points
                ),
                (20, 10, 6)
            );
            assert_eq!((*e).__bindgen_anon_5.blast_offset, 8);
            assert_eq!(frame_index((*e).current.image.frame), 11);
            assert_eq!(velocity(&(*e).velocity), toward(20 << ONE_SHIFT, 11));
            assert!((*e).preprocess_func.is_none());
            FreeElement(h);
        }
    })
}
#[test]
fn fighters_launch_left_then_right_and_return_crew() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            with_rng(7, 0, || ship.postprocess());
            assert_eq!(ship.descriptor_mut().ship_info.crew_level, 40);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 34);
            assert_eq!(ship.ship_mut().special_counter, 9);
            let all = queued();
            assert_eq!(all.len(), 3);
            for (i, &h) in all[1..].iter().enumerate() {
                let e = element(h);
                let face = if i == 0 { 10 } else { 6 };
                assert_eq!(
                    (*e).state_flags,
                    (APPEARING | FINITE_LIFE | CREW_OBJECT | IGNORE_SIMILAR) as u16
                );
                assert_eq!(
                    (
                        (*e).__bindgen_anon_1.life_span,
                        (*e).__bindgen_anon_2.hit_points,
                        (*e).__bindgen_anon_3.mass_points
                    ),
                    (400, 1, 0)
                );
                assert_eq!((*e).__bindgen_anon_5.thrust_wait, 7);
                assert_eq!(
                    (*e).__bindgen_anon_4.turn_wait,
                    if i == 0 { LEFT as u8 } else { RIGHT as u8 }
                );
                assert_eq!(frame_index((*e).current.image.frame), face);
                assert_eq!(velocity(&(*e).velocity), toward(8 << ONE_SHIFT, face));
                (*e).next = (*e).current;
                with_rng(7, 0, || {
                    uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                        (*e).collision_func.unwrap(),
                        e,
                        &mut POINT { x: 0, y: 0 },
                        ship.ptr(),
                        &mut POINT { x: 0, y: 0 },
                        uqm_rust::battle::process_native::invoke_collision::CollisionCall::Hook,
                    )
                });
                assert_eq!((*e).__bindgen_anon_1.life_span, 0);
                assert_eq!((*e).__bindgen_anon_2.hit_points, 0);
                assert_ne!((*e).state_flags & NONSOLID as u16, 0);
                assert_eq!((*e).state_flags & DISAPPEARING as u16, 0);
            }
            assert_eq!(ship.descriptor_mut().ship_info.crew_level, 42);
        }
    })
}
#[test]
fn two_crew_launches_one_right_fighter_and_allocation_failure_keeps_crew() {
    battle_context::run(|| {
        for capacity in [1, 2] {
            let _battle = Battle::new(capacity);
            unsafe {
                let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
                ship.ship_mut().cur_status_flags = SPECIAL as u16;
                ship.descriptor_mut().ship_info.crew_level = 2;
                (*ship.ptr()).__bindgen_anon_2.crew_level = 2;
                ship.postprocess();
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 34);
                assert_eq!(ship.ship_mut().special_counter, 9);
                assert_eq!(
                    ship.descriptor_mut().ship_info.crew_level,
                    if capacity == 1 { 2 } else { 1 }
                );
                if capacity == 2 {
                    assert_eq!(
                        (*element(disp_q.tail)).__bindgen_anon_4.turn_wait,
                        RIGHT as u8
                    );
                }
            }
        }
    })
}
#[test]
fn fighter_tracking_delay_then_return_and_fire_use_exact_ranges() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
            let enemy = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 1);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            let fighter = element(disp_q.tail);
            (*fighter).next = (*fighter).current;
            let initial = velocity(&(*fighter).velocity);
            (*fighter).__bindgen_anon_1.life_span = 394;
            with_rng(17, 0, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*fighter).preprocess_func.unwrap(),
                    fighter,
                )
            });
            assert_eq!(velocity(&(*fighter).velocity), initial);
            assert_eq!((*fighter).__bindgen_anon_5.thrust_wait, 7);
            (*fighter).__bindgen_anon_1.life_span = 392;
            (*fighter).hTarget = enemy.handle;
            (*fighter).current.location = POINT { x: 4000, y: 4100 };
            (*fighter).__bindgen_anon_5.thrust_wait = 1;
            with_rng(17, 0, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*fighter).preprocess_func.unwrap(),
                    fighter,
                )
            });
            assert!((*fighter).postprocess_func.is_some());
            assert_eq!((*fighter).__bindgen_anon_5.thrust_wait, 0);
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*fighter).postprocess_func.unwrap(),
                fighter,
            );
            assert_eq!((*fighter).__bindgen_anon_5.thrust_wait, 8);
            assert!((*fighter).postprocess_func.is_none());
            let laser = element(disp_q.tail);
            assert_eq!((*laser).pParent, (&raw mut *ship.ship_mut()).cast());
            assert_eq!((*laser).__bindgen_anon_3.mass_points, 1);
            // On the return leg the ship becomes collidable with its own fighters.
            (*fighter).state_flags &= !(CHANGING as u16);
            (*fighter).__bindgen_anon_1.life_span = 124;
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*fighter).preprocess_func.unwrap(),
                fighter,
            );
            assert_eq!((*fighter).state_flags & IGNORE_SIMILAR as u16, 0);
        }
    })
}
#[test]
fn fighter_planet_collision_clones_and_deflects_without_rng() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            let e = element(disp_q.tail);
            (*e).next = (*e).current;
            SetVelocityVector(&raw mut (*e).velocity, 8 << ONE_SHIFT, 0);
            let mut planet_fixture = Object::new(&_battle);
            let planet = planet_fixture.ptr();
            (*planet).__bindgen_anon_3.mass_points = 101;
            let mut pa = POINT { x: 0, y: 0 };
            let mut pb = POINT { x: 0, y: -10 };
            with_rng(18, 0, || {
                uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                    (*e).collision_func.unwrap(),
                    e,
                    &mut pa,
                    planet,
                    &mut pb,
                    uqm_rust::battle::process_native::invoke_collision::CollisionCall::Hook,
                )
            });
            let clone = element(disp_q.tail);
            assert_ne!(clone, e);
            assert_eq!((*e).__bindgen_anon_1.life_span, 0);
            assert_eq!(frame_index((*clone).next.image.frame), 4);
            assert_eq!(velocity(&(*clone).velocity), toward(8 << ONE_SHIFT, 4));
            assert_ne!((*clone).state_flags & CHANGING as u16, 0);
        }
    })
}
#[test]
fn ai_fighter_budget_point_defense_and_crew_boundaries() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
            let mut enemy = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 1);
            ship.ship_mut().weapon_counter = 1;
            enemy.descriptor_mut().cyborg_control.ManeuverabilityIndex = 100;
            for crew in [10, 11] {
                for fighters in [5, 6] {
                    for defense in [false, true] {
                        ship.descriptor_mut().ship_info.crew_level = crew;
                        ship.descriptor_mut().characteristics.special_wait = fighters;
                        enemy.descriptor_mut().ship_info.ship_flags =
                            if defense { POINT_DEFENSE as u16 } else { 0 };
                        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                        concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
                        concerns[ENEMY_SHIP_INDEX as usize].which_turn = 20;
                        with_rng(12, 0, || {
                            ship.intelligence(&mut *ship.ptr(), &mut concerns, 0)
                        });
                        assert_eq!(
                            ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                            crew > 10 && fighters < 6 && !defense
                        );
                        assert_eq!(ship.descriptor_mut().characteristics.special_wait, 0);
                        assert_eq!(
                            concerns[ENEMY_SHIP_INDEX as usize].MoveState,
                            MOVEMENT_STATE_PURSUE
                        );
                    }
                }
            }
        }
    })
}
