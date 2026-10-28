use uqm_rust::ships::races::init_utwig;
#[path = "support/battle_context.rs"]
mod battle_context;

mod native_ship_common;
use native_ship_common::*;
use std::mem;
use uqm_rust::ships::native::{abi::*, cosine, element, facing_to_angle, frame_index, sine};

#[test]
fn descriptor_and_six_lance_order_and_coordinates() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(init_utwig, 0);
            assert_eq!(
                ship.desc.ship_info.ship_flags,
                (FIRES_FORE | POINT_DEFENSE | SHIELD_DEFENSE) as u16
            );
            assert_eq!(
                (
                    ship.desc.ship_info.crew_level,
                    ship.desc.ship_info.energy_level,
                    ship.desc.ship_info.max_energy
                ),
                (20, 10, 20)
            );
            assert_eq!(ship.desc.characteristics.energy_wait, 255);
            assert_eq!(
                (ship.desc.fleet.known_loc.x, ship.desc.fleet.known_loc.y),
                (8534, 8797)
            );
            for facing in [0, 4, 7, 15] {
                ship.ss.ShipFacing = facing;
                let handles = with_rng(19, 0, || ship.weapon(6));
                let angle = facing_to_angle(i32::from(facing)) + QUADRANT as i32;
                let vx = cosine(
                    facing_to_angle(i32::from(facing)),
                    (30 << ONE_SHIFT) << VELOCITY_SHIFT,
                ) >> VELOCITY_SHIFT;
                let vy = sine(
                    facing_to_angle(i32::from(facing)),
                    (30 << ONE_SHIFT) << VELOCITY_SHIFT,
                ) >> VELOCITY_SHIFT;
                for (i, (x, y)) in [(5, -18), (13, -9), (17, -4)].into_iter().enumerate() {
                    for (side, sign) in [(0, 1), (1, -1)] {
                        let e = element(handles[i * 2 + side]);
                        let dx = sign * i32::from(cosine(angle, x << ONE_SHIFT))
                            - i32::from(sine(angle, y << ONE_SHIFT));
                        let dy = sign * i32::from(sine(angle, x << ONE_SHIFT))
                            + i32::from(cosine(angle, y << ONE_SHIFT));
                        assert_eq!(
                            ((*e).current.location.x, (*e).current.location.y),
                            (
                                (4000 + dx - i32::from(vx)) as i16,
                                (4000 + dy - i32::from(vy)) as i16
                            )
                        );
                        assert_eq!(
                            (
                                (*e).__bindgen_anon_1.life_span,
                                (*e).__bindgen_anon_2.hit_points,
                                (*e).__bindgen_anon_3.mass_points
                            ),
                            (10, 1, 1)
                        );
                        assert_eq!(frame_index((*e).current.image.frame), facing);
                        assert_eq!(velocity(&(*e).velocity), toward(30 << ONE_SHIFT, facing));
                        FreeElement(handles[i * 2 + side]);
                    }
                }
            }
        }
    })
}
#[test]
fn shield_absorbs_damage_before_collision_and_recharges_on_next_preprocess() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(init_utwig, 0);
            let e = ship.ptr();
            (*e).state_flags |= APPEARING as u16;
            ship.ss.cur_status_flags = SPECIAL as u16;
            with_rng(1, 0, || (ship.desc.preprocess_func.unwrap())(e));
            assert_eq!(ship.desc.ship_info.energy_level, 9);
            assert_eq!(ship.ss.special_counter, 12);
            assert_eq!((*e).__bindgen_anon_1.life_span, 2);
            assert_eq!((*e).colorCycleIndex, 1);
            assert_eq!((*prim(e)).Type, gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM);
            ship.ss.weapon_counter = 1;
            let handle = AllocElement();
            assert!(!handle.is_null());
            let projectile = &mut *element(handle);
            let mut pa = POINT { x: 0, y: 0 };
            let mut pb = POINT { x: 0, y: 0 };
            projectile.state_flags = FINITE_LIFE as u16;
            projectile.__bindgen_anon_3.mass_points = 6;
            with_rng(1, 0, || {
                uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                    (*e).collision_func.unwrap(),
                    e,
                    &mut pa,
                    projectile,
                    &mut pb,
                    uqm_rust::battle::process_native::invoke_collision::CollisionCall::Hook,
                )
            });
            FreeElement(handle);
            assert_eq!((*e).__bindgen_anon_1.life_span, 8);
            assert_eq!(ship.desc.ship_info.crew_level, 20);
            ship.ss.special_counter = 11;
            (ship.desc.preprocess_func.unwrap())(e);
            assert_eq!(ship.desc.ship_info.energy_level, 15);
            assert_eq!((*e).__bindgen_anon_1.life_span, 2);
            for index in [3, 4, 5, 6, 7, 1] {
                (ship.desc.preprocess_func.unwrap())(e);
                assert_eq!((*e).colorCycleIndex, index);
            }
            ship.ss.cur_status_flags = 0;
            (ship.desc.preprocess_func.unwrap())(e);
            assert_eq!(ship.ss.special_counter, 0);
            assert_eq!((*e).__bindgen_anon_1.life_span, 1);
            assert_eq!((*e).colorCycleIndex, 0);
            assert_eq!((*prim(e)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
        }
    })
}
#[test]
fn failed_shield_removes_defense_flags_and_half_period_drains_energy() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(init_utwig, 0);
            ship.ss.cur_status_flags = SPECIAL as u16;
            ship.desc.ship_info.energy_level = 0;
            (ship.desc.preprocess_func.unwrap())(ship.ptr());
            assert_eq!(ship.desc.ship_info.ship_flags, FIRES_FORE as u16);
            assert_eq!(ship.ss.special_counter, 0);
            ship.desc.ship_info.energy_level = 3;
            ship.ss.special_counter = 6;
            (ship.desc.preprocess_func.unwrap())(ship.ptr());
            assert_eq!(ship.desc.ship_info.energy_level, 2);
            assert_eq!(ship.ss.special_counter, 6);
            assert_eq!(ship.ss.weapon_counter, 1);
        }
    })
}
#[test]
fn ai_halves_finite_threat_turns_and_consumes_rng_only_at_shield_boundary() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(init_utwig, 0);
            ship.ss.weapon_counter = 1;
            let mut projectile: ELEMENT = mem::zeroed();
            projectile.state_flags = FINITE_LIFE as u16;
            projectile.__bindgen_anon_3.mass_points = 1;
            for turn in [0, 1, 2, 4, 5, 6] {
                for seed in 1..9 {
                    let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                    concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
                        ObjectPtr: &mut projectile,
                        facing: 0,
                        which_turn: turn,
                        MoveState: MOVEMENT_STATE_ENTICE,
                    };
                    TFB_SeedRandom(seed);
                    let shield = TFB_Random() & 3 != 0;
                    let transformed = (turn >> 1).max(1);
                    ship.ss.ship_input_state = WEAPON as u8;
                    with_rng(seed, usize::from(transformed <= 2), || {
                        ship.intelligence(&mut concerns)
                    });
                    assert!(concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr.is_null());
                    assert_eq!(
                        concerns[ENEMY_WEAPON_INDEX as usize].which_turn,
                        transformed
                    );
                    assert_eq!(
                        ship.ss.ship_input_state & SPECIAL as u8 != 0,
                        transformed <= 2 && shield
                    );
                    assert_eq!(
                        ship.ss.ship_input_state & WEAPON as u8 != 0,
                        !(transformed <= 2 && shield)
                    );
                }
            }
            ship.desc.ship_info.energy_level = 0;
            let mut concerns = mem::zeroed();
            with_rng(2, 0, || ship.intelligence(&mut concerns));
            assert_eq!(ship.ss.ship_input_state & SPECIAL as u8, 0);
        }
    })
}
#[test]
fn lance_allocation_failure_still_returns_six_slots() {
    battle_context::run(|| {
        let _battle = Battle::new(3);
        unsafe {
            let mut ship = Ship::new(init_utwig, 0);
            let out = ship.weapon(6);
            assert!(!out[0].is_null() && !out[1].is_null());
            assert!(out[2..].iter().all(|h| h.is_null()));
            FreeElement(out[0]);
            FreeElement(out[1]);
        }
    })
}
