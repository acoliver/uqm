#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use std::mem;
use uqm_rust::ships::native::{abi::*, frame_index};

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
            ship.ship_mut().ship_facing = 11;
            let h = with_rng(8, 0, || {
                ship.weapon(1)
                    .into_iter()
                    .map(|id| {
                        id.map_or(std::ptr::null_mut(), |id| {
                            ship.weapon_record(Some(id)).cast()
                        })
                    })
                    .collect::<Vec<*mut std::ffi::c_void>>()
            })[0];
            let e = battle::element(h);
            assert_eq!(((*e).life, (*e).hits, (*e).mass), (20, 10, 6));
            assert_eq!((*e).thrust, 8);
            assert_eq!(frame_index((*e).current.image.frame.raw()), 11);
            assert_eq!(velocity(&(*e).velocity), toward(20 << ONE_SHIFT, 11));
            assert!((*e).preprocess_func.is_none());
            battle_context::dispose(h);
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
                let e = battle::element(h);
                let face = if i == 0 { 10 } else { 6 };
                assert_eq!(
                    (*e).state_flags,
                    (APPEARING | FINITE_LIFE | CREW_OBJECT | IGNORE_SIMILAR) as u16
                );
                assert_eq!(((*e).life, (*e).hits, (*e).mass), (400, 1, 0));
                assert_eq!((*e).thrust, 7);
                assert_eq!((*e).turn, if i == 0 { LEFT as u8 } else { RIGHT as u8 });
                assert_eq!(frame_index((*e).current.image.frame.raw()), face);
                assert_eq!(velocity(&(*e).velocity), toward(8 << ONE_SHIFT, face));
                (*e).next = (*e).current.clone();
                with_rng(7, 0, || {
                    hooks::invoke_collision_pointer(
                        (*e).collision_func.unwrap(),
                        e,
                        &mut POINT { x: 0, y: 0 },
                        ship.ptr(),
                        &mut POINT { x: 0, y: 0 },
                        hooks::CollisionCall::Hook,
                    )
                });
                assert_eq!((*e).life, 0);
                assert_eq!((*e).hits, 0);
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
                (*ship.ptr()).hits = 2;
                ship.postprocess();
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 34);
                assert_eq!(ship.ship_mut().special_counter, 9);
                assert_eq!(
                    ship.descriptor_mut().ship_info.crew_level,
                    if capacity == 1 { 2 } else { 1 }
                );
                if capacity == 2 {
                    assert_eq!((*battle::element(battle::tail())).turn, RIGHT as u8);
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
            let fighter = battle::element(battle::tail());
            (*fighter).next = (*fighter).current.clone();
            let initial = velocity(&(*fighter).velocity);
            (*fighter).life = 394;
            with_rng(17, 0, || {
                hooks::invoke_element_pointer((*fighter).preprocess_func.unwrap(), fighter)
            });
            assert_eq!(velocity(&(*fighter).velocity), initial);
            assert_eq!((*fighter).thrust, 7);
            (*fighter).life = 392;
            (*fighter).target = Some(battle::id((enemy.handle).cast()));
            (*fighter).current.location = POINT { x: 4000, y: 4100 };
            (*fighter).thrust = 1;
            with_rng(17, 0, || {
                hooks::invoke_element_pointer((*fighter).preprocess_func.unwrap(), fighter)
            });
            assert!((*fighter).postprocess_func.is_some());
            assert_eq!((*fighter).thrust, 0);
            hooks::invoke_element_pointer((*fighter).postprocess_func.unwrap(), fighter);
            assert_eq!((*fighter).thrust, 8);
            assert!((*fighter).postprocess_func.is_none());
            let laser = battle::element(battle::tail());
            assert_eq!(
                (*laser).parent,
                uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
            );
            assert_eq!((*laser).mass, 1);
            // On the return leg the ship becomes collidable with its own fighters.
            (*fighter).state_flags &= !(CHANGING as u16);
            (*fighter).life = 124;
            hooks::invoke_element_pointer((*fighter).preprocess_func.unwrap(), fighter);
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
            let e = battle::element(battle::tail());
            (*e).next = (*e).current.clone();
            uqm_rust::battle::velocity::set_velocity_vector(&mut (*e).velocity, 8 << ONE_SHIFT, 0);
            let mut planet_fixture = Object::new(&_battle);
            let planet = planet_fixture.ptr();
            (*planet).mass = 101;
            let mut pa = POINT { x: 0, y: 0 };
            let mut pb = POINT { x: 0, y: -10 };
            with_rng(18, 0, || {
                hooks::invoke_collision_pointer(
                    (*e).collision_func.unwrap(),
                    e,
                    &mut pa,
                    planet,
                    &mut pb,
                    hooks::CollisionCall::Hook,
                )
            });
            let clone = battle::element(battle::tail());
            assert_ne!(clone, e);
            assert_eq!((*e).life, 0);
            assert_eq!(frame_index((*clone).next.image.frame.raw()), 4);
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

#[path = "support/hooks.rs"]
mod hooks;
