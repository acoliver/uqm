#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use std::mem;
use uqm_rust::ships::native::{abi::*, angle_to_facing, element, frame_index, normalize_facing};
#[test]
fn descriptor_laser_and_limpet_launch() {
    battle_context::run(|| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_VUX_ID, 0);
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags,
                (FIRES_FORE | SEEKING_SPECIAL | IMMEDIATE_WEAPON) as u16
            );
            assert_eq!(
                (
                    ship.descriptor_mut().ship_info.crew_level,
                    ship.descriptor_mut().ship_info.energy_level
                ),
                (20, 40)
            );
            assert_eq!(
                (
                    ship.descriptor_mut().fleet.known_loc.x,
                    ship.descriptor_mut().fleet.known_loc.y
                ),
                (4412, 1558)
            );
            ship.ship_mut().ShipFacing = 4;
            let h = with_rng(11, 0, || ship.weapon(1))[0];
            let laser = ship.weapon_record(h);
            assert_eq!((*prim(laser)).Type, gfx_object_LINE_PRIM as GRAPHICS_PRIM);
            assert_eq!((*laser).__bindgen_anon_3.mass_points, 1);
            assert_eq!((*laser).current.location.x, 4000 + 48);
            let (mut dx, mut dy) = (0, 0);
            GetCurrentVelocityComponents(&raw mut (*laser).velocity, &mut dx, &mut dy);
            assert_eq!((dx, dy), (600 << VELOCITY_SHIFT, 0));
            FreeElement(laser.cast());
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            with_rng(11, 0, || ship.postprocess());
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 38);
            assert_eq!(ship.ship_mut().special_counter, 7);
            let e = element(disp_q.tail);
            assert_eq!(
                (
                    (*e).__bindgen_anon_1.life_span,
                    (*e).__bindgen_anon_2.hit_points,
                    (*e).__bindgen_anon_3.mass_points
                ),
                (80, 1, 0)
            );
            // HALF_CIRCLE is 32, added directly to facing 4 in the C code.
            assert_eq!(velocity(&(*e).velocity), toward(25, 36));
            assert_eq!((*e).pParent, (&raw mut *ship.ship_mut()).cast());
            (*e).next = (*e).current;
            with_rng(11, 0, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*e).preprocess_func.unwrap(),
                    e,
                )
            });
            assert_eq!(frame_index((*e).next.image.frame), 1);
        }
    })
}
#[test]
fn limpets_saturate_waits_and_reduce_thrust_without_rng_for_nonships() {
    battle_context::run(|| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_VUX_ID, 0);
            let mut enemy = Ship::new(&_battle, SPECIES_ID_VUX_ID, 1);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            let e = element(disp_q.tail);
            for (increment, max, expected_inc, expected_max) in
                [(7, 21, 6, 18), (5, 25, 4, 20), (4, 20, 4, 8), (1, 10, 1, 2)]
            {
                enemy.descriptor_mut().characteristics.turn_wait = 255;
                enemy.descriptor_mut().characteristics.thrust_wait = 254;
                enemy.descriptor_mut().characteristics.thrust_increment = increment;
                enemy.descriptor_mut().characteristics.max_thrust = max;
                enemy.descriptor_mut().cyborg_control.ManeuverabilityIndex = 99;
                // Null icons make ModifySilhouette return before its placement RNG.
                with_rng(12, 1, || {
                    call_collision(
                        "ships::races::vux_native::limpet_collision",
                        id(e),
                        &mut POINT { x: 0, y: 0 },
                        id(enemy.ptr()),
                        &mut POINT { x: 0, y: 0 },
                    )
                });
                assert_eq!(enemy.descriptor_mut().characteristics.turn_wait, 255);
                assert_eq!(enemy.descriptor_mut().characteristics.thrust_wait, 255);
                assert_eq!(
                    enemy.descriptor_mut().characteristics.thrust_increment,
                    expected_inc
                );
                assert_eq!(
                    enemy.descriptor_mut().characteristics.max_thrust,
                    expected_max
                );
                assert_eq!(
                    enemy.descriptor_mut().cyborg_control.ManeuverabilityIndex,
                    0
                );
                assert_eq!((*e).__bindgen_anon_1.life_span, 0);
                assert_eq!((*e).__bindgen_anon_2.hit_points, 0);
                assert_eq!(
                    (*e).state_flags & (COLLISION | DISAPPEARING) as u16,
                    (COLLISION | DISAPPEARING) as u16
                );
            }
            let mut object = Object::new(&_battle);
            with_rng(12, 0, || {
                call_collision(
                    "ships::races::vux_native::limpet_collision",
                    id(e),
                    &mut POINT { x: 0, y: 0 },
                    id(object.ptr()),
                    &mut POINT { x: 0, y: 0 },
                )
            });
        }
    })
}
#[test]
fn warp_in_is_disabled_in_encounters_and_one_shot_in_melee() {
    battle_context::run(|| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_VUX_ID, 0);
            let enemy = Ship::new(&_battle, SPECIES_ID_VUX_ID, 1);
            (*enemy.ptr()).current.location = POINT { x: 5000, y: 5000 };
            (*enemy.ptr()).next = (*enemy.ptr()).current;
            (*ship.ptr()).state_flags |= APPEARING as u16;
            let previous = GlobData.Game_state.CurrentActivity;
            GlobData.Game_state.CurrentActivity = IN_ENCOUNTER as _;
            with_rng(42, 0, || ship.preprocess());
            assert_eq!((*ship.ptr()).current.location.x, 4000);
            assert_eq!(ship.descriptor_mut().callbacks()[1], 0);
            ship.descriptor_mut().race_callbacks.pre = uqm_rust::engine::battle::Selection::Default;
            GlobData.Game_state.CurrentActivity = 0;
            TFB_SeedRandom(42);
            let raw_x = 5000 - 832 + (TFB_Random() as COUNT) % 1664;
            let raw_y = 5000 - 832 + (TFB_Random() as COUNT) % 1664;
            let x = raw_x & !3;
            let y = raw_y & !3;
            with_rng(42, 2, || ship.preprocess());
            assert_eq!(
                (
                    (*ship.ptr()).current.location.x,
                    (*ship.ptr()).current.location.y
                ),
                (x as i16, y as i16)
            );
            let face = normalize_facing(angle_to_facing(i32::from(ARCTAN(
                (5000 - i32::from(raw_x)) as SIZE,
                (5000 - i32::from(raw_y)) as SIZE,
            ))));
            assert_eq!(ship.ship_mut().ShipFacing, face);
            assert_eq!(frame_index((*ship.ptr()).current.image.frame), face);
            assert!((*ship.ptr()).hTarget.is_null());
            assert_eq!(ship.descriptor_mut().callbacks()[1], 0);
            assert_eq!(
                (*ship.ptr()).IntersectControl.EndPoint.x,
                (x >> ONE_SHIFT) as i16
            );
            GlobData.Game_state.CurrentActivity = previous;
        }
    })
}
#[test]
fn ai_changes_threat_state_and_uses_turning_energy_threshold() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_VUX_ID, 0);
            let enemy = Ship::new(&_battle, SPECIES_ID_VUX_ID, 1);
            let mut threat = Object::new(&_battle);
            ship.ship_mut().weapon_counter = 1;
            for flags in [0, FINITE_LIFE, FINITE_LIFE | CREW_OBJECT] {
                for energy in [19, 20] {
                    threat.element_mut().state_flags = flags as u16;
                    ship.descriptor_mut().ship_info.energy_level = energy;
                    ship.ship_mut().ship_input_state = LEFT as u8;
                    let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                    concerns[ENEMY_SHIP_INDEX as usize] = EVALUATE_DESC {
                        ObjectPtr: enemy.ptr(),
                        facing: 0,
                        which_turn: 12,
                        MoveState: MOVEMENT_STATE_ENTICE,
                    };
                    concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
                        ObjectPtr: threat.ptr(),
                        facing: 0,
                        which_turn: 3,
                        MoveState: MOVEMENT_STATE_ENTICE,
                    };
                    with_rng(8, 0, || {
                        ship.intelligence(&mut *ship.ptr(), &mut concerns, 0)
                    });
                    assert_eq!(
                        concerns[ENEMY_WEAPON_INDEX as usize].MoveState,
                        if flags == FINITE_LIFE {
                            MOVEMENT_STATE_AVOID
                        } else {
                            MOVEMENT_STATE_PURSUE
                        }
                    );
                    assert_eq!(
                        ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                        energy >= 20
                    );
                }
            }
        }
    })
}
