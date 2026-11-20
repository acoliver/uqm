#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
#[path = "support/special_hooks.rs"]
mod special_hooks;
use battle::*;
use uqm_rust::collections::observation;
use uqm_rust::engine::battle::ai_constants::ENEMY_WEAPON_INDEX;
use uqm_rust::engine::battle::ai_constants::FIRES_FORE;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_ENTICE;
use uqm_rust::engine::battle::ai_constants::POINT_DEFENSE;
use uqm_rust::engine::battle::ai_constants::SHIELD_DEFENSE;
use uqm_rust::engine::battle::ai_constants::SPECIAL;
use uqm_rust::engine::battle::ai_constants::WEAPON;
use uqm_rust::engine::battle::flags::APPEARING;
use uqm_rust::engine::battle::flags::FINITE_LIFE;
use uqm_rust::engine::battle::flags::STAMPFILL_PRIMITIVE;
use uqm_rust::engine::battle::Concern;
use uqm_rust::engine::battle::ElementId;
use uqm_rust::engine::battle::STAMP_PRIMITIVE;
use uqm_rust::engine::species::species_id::SPECIES_ID_UTWIG_ID;
use uqm_rust::graphics::geometry::Point;
use uqm_rust::math::battle_types::ONE_SHIFT;
use uqm_rust::math::battle_types::QUADRANT;
use uqm_rust::math::random;
use uqm_rust::math::seed_random;
use uqm_rust::math::velocity::VELOCITY_SHIFT;
use uqm_rust::ships::native::{cosine, facing_to_angle, sine};

#[test]
fn descriptor_and_six_lance_order_and_coordinates() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UTWIG_ID, 0);
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags,
                (FIRES_FORE | POINT_DEFENSE | SHIELD_DEFENSE) as u16
            );
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (
                        descriptor.ship_info.crew_level,
                        descriptor.ship_info.energy_level,
                        descriptor.ship_info.max_energy
                    ),
                    (20, 10, 20)
                );
            }
            assert_eq!(ship.descriptor_mut().characteristics.energy_wait, 255);
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (descriptor.fleet.known_loc.x, descriptor.fleet.known_loc.y),
                    (8534, 8797)
                );
            }
            for facing in [0, 4, 7, 15] {
                ship.ship_mut().ship_facing = facing;
                let handles = with_rng(19, 0, || {
                    ship.weapon(6)
                        .into_iter()
                        .map(|id| id.expect("allocated weapon"))
                        .collect::<Vec<ElementId>>()
                });
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
                        let e = handles[i * 2 + side];
                        let dx = sign * i32::from(cosine(angle, x << ONE_SHIFT))
                            - i32::from(sine(angle, y << ONE_SHIFT));
                        let dy = sign * i32::from(sine(angle, x << ONE_SHIFT))
                            + i32::from(cosine(angle, y << ONE_SHIFT));
                        assert_eq!(
                            (
                                battle::get(e).current.location.x,
                                battle::get(e).current.location.y
                            ),
                            (
                                (4000 + dx - i32::from(vx)) as i16,
                                (4000 + dy - i32::from(vy)) as i16
                            )
                        );
                        assert_eq!(
                            (
                                battle::get(e).life,
                                battle::get(e).hits,
                                battle::get(e).mass
                            ),
                            (10, 1, 1)
                        );
                        assert_eq!((battle::get(e).current.image.frame.raw()).index(), facing);
                        assert_eq!(
                            velocity(&battle::get(e).velocity),
                            toward(30 << ONE_SHIFT, facing)
                        );
                        battle_context::dispose(handles[i * 2 + side]);
                    }
                }
            }
        }
    })
}
#[test]
fn shield_absorbs_damage_before_collision_and_recharges_on_next_preprocess() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UTWIG_ID, 0);
            let e = ship.id;
            battle::get_mut(e).state_flags |= APPEARING as u16;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            observation::enable();
            observation::drain();
            with_rng(1, 0, || ship.preprocess());
            assert_eq!(special_hooks::count(&observation::drain(), "utwig"), 1);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 9);
            assert_eq!(ship.ship_mut().special_counter, 12);
            assert_eq!(battle::get(e).life, 2);
            assert_eq!(battle::get(e).color_cycle_index, 1);
            assert_eq!(battle::prim(e).kind, STAMPFILL_PRIMITIVE);
            ship.ship_mut().weapon_counter = 1;
            let projectile_fixture = Object::new(&_battle);
            let projectile = projectile_fixture.id;
            let mut pa = Point { x: 0, y: 0 };
            let mut pb = Point { x: 0, y: 0 };
            battle::get_mut(projectile).state_flags = FINITE_LIFE as u16;
            battle::get_mut(projectile).mass = 6;
            with_rng(1, 0, || {
                hooks::invoke_collision(
                    battle::get(e).collision_func.unwrap(),
                    e,
                    &mut pa,
                    projectile,
                    &mut pb,
                )
            });
            assert_eq!(battle::get(e).life, 8);
            assert_eq!(ship.descriptor_mut().ship_info.crew_level, 20);
            ship.ship_mut().special_counter = 11;
            ship.preprocess();
            // The shield is already running, so its pulse does not start again.
            assert_eq!(special_hooks::count(&observation::drain(), "utwig"), 0);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 15);
            assert_eq!(battle::get(e).life, 2);
            for index in [3, 4, 5, 6, 7, 1] {
                ship.preprocess();
                assert_eq!(battle::get(e).color_cycle_index, index);
            }
            ship.ship_mut().cur_status_flags = 0;
            ship.preprocess();
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(battle::get(e).life, 1);
            assert_eq!(battle::get(e).color_cycle_index, 0);
            assert_eq!(battle::prim(e).kind, STAMP_PRIMITIVE);
        }
    })
}
#[test]
fn failed_shield_removes_defense_flags_and_half_period_drains_energy() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UTWIG_ID, 0);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.descriptor_mut().ship_info.energy_level = 0;
            observation::enable();
            observation::drain();
            ship.preprocess();
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags,
                FIRES_FORE as u16
            );
            assert_eq!(ship.ship_mut().special_counter, 0);
            ship.descriptor_mut().ship_info.energy_level = 3;
            ship.ship_mut().special_counter = 6;
            ship.preprocess();
            // Neither a failed payment nor a half-period drain activates the shield.
            assert_eq!(special_hooks::count(&observation::drain(), "utwig"), 0);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 2);
            assert_eq!(ship.ship_mut().special_counter, 6);
            assert_eq!(ship.ship_mut().weapon_counter, 1);
        }
    })
}
#[test]
fn ai_halves_finite_threat_turns_and_consumes_rng_only_at_shield_boundary() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UTWIG_ID, 0);
            ship.ship_mut().weapon_counter = 1;
            let projectile_fixture = Object::new(&_battle);
            let projectile = projectile_fixture.id;
            battle::get_mut(projectile).state_flags = FINITE_LIFE as u16;
            battle::get_mut(projectile).mass = 1;
            for turn in [0, 1, 2, 4, 5, 6] {
                for seed in 1..9 {
                    let mut concerns: [Concern; 4] = [Concern::default(); 4];
                    concerns[ENEMY_WEAPON_INDEX as usize] = Concern {
                        object: Some(projectile),
                        facing: 0,
                        which_turn: turn,
                        movement: MOVEMENT_STATE_ENTICE,
                    };
                    seed_random(seed);
                    let shield = random() & 3 != 0;
                    let transformed = (turn >> 1).max(1);
                    ship.ship_mut().ship_input_state = WEAPON as u8;
                    with_rng(seed, usize::from(transformed <= 2), || {
                        ship.think(ship.id, &mut concerns, 0)
                    });
                    assert!(concerns[ENEMY_WEAPON_INDEX as usize].object.is_none());
                    assert_eq!(
                        concerns[ENEMY_WEAPON_INDEX as usize].which_turn,
                        transformed
                    );
                    assert_eq!(
                        ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                        transformed <= 2 && shield
                    );
                    assert_eq!(
                        ship.ship_mut().ship_input_state & WEAPON as u8 != 0,
                        !(transformed <= 2 && shield)
                    );
                }
            }
            ship.descriptor_mut().ship_info.energy_level = 0;
            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            with_rng(2, 0, || ship.think(ship.id, &mut concerns, 0));
            assert_eq!(ship.ship_mut().ship_input_state & SPECIAL as u8, 0);
        }
    })
}
#[test]
fn lance_allocation_failure_still_returns_six_slots() {
    battle_context::run(|_| {
        let _battle = Battle::new(3);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UTWIG_ID, 0);
            let out = ship.weapon(6);
            assert!(out[0].is_some() && out[1].is_some());
            assert!(out[2..].iter().all(|h| h.is_none()));
            battle_context::dispose(out[0].unwrap());
            battle_context::dispose(out[1].unwrap());
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
