#![forbid(unsafe_code)]
#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use uqm_rust::engine::battle::ai_constants::ENEMY_SHIP_INDEX;
use uqm_rust::engine::battle::ai_constants::ENEMY_WEAPON_INDEX;
use uqm_rust::engine::battle::ai_constants::FIRES_FORE;
use uqm_rust::engine::battle::ai_constants::IMMEDIATE_WEAPON;
use uqm_rust::engine::battle::ai_constants::LEFT;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_AVOID;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_ENTICE;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_PURSUE;
use uqm_rust::engine::battle::ai_constants::SEEKING_SPECIAL;
use uqm_rust::engine::battle::ai_constants::SPECIAL;
use uqm_rust::engine::battle::flags::APPEARING;
use uqm_rust::engine::battle::flags::COLLISION;
use uqm_rust::engine::battle::flags::CREW_OBJECT;
use uqm_rust::engine::battle::flags::DISAPPEARING;
use uqm_rust::engine::battle::flags::FINITE_LIFE;
use uqm_rust::engine::battle::Concern;
use uqm_rust::engine::battle::LINE_PRIMITIVE;
use uqm_rust::engine::species::species_id::SPECIES_ID_VUX_ID;
use uqm_rust::engine::world::activity::activity_kinds::IN_ENCOUNTER;
use uqm_rust::graphics::geometry::Point;
use uqm_rust::math::arctan;
use uqm_rust::math::battle_types::ONE_SHIFT;
use uqm_rust::math::random;
use uqm_rust::math::seed_random;
use uqm_rust::math::velocity::VELOCITY_SHIFT;
use uqm_rust::ships::native::{angle_to_facing, normalize_facing};
#[test]
fn descriptor_laser_and_limpet_launch() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_VUX_ID, 0);
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags,
                (FIRES_FORE | SEEKING_SPECIAL | IMMEDIATE_WEAPON) as u16
            );
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (
                        descriptor.ship_info.crew_level,
                        descriptor.ship_info.energy_level
                    ),
                    (20, 40)
                );
            }
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (descriptor.fleet.known_loc.x, descriptor.fleet.known_loc.y),
                    (4412, 1558)
                );
            }
            ship.ship_mut().ship_facing = 4;
            let h = with_rng(11, 0, || ship.weapon(1))[0];
            let laser = h.expect("allocated weapon");
            assert_eq!(battle::prim(laser).kind, LINE_PRIMITIVE);
            assert_eq!(battle::get(laser).mass, 1);
            assert_eq!(battle::get(laser).current.location.x, 4000 + 48);
            let (velocity_x, velocity_y) = (battle::get(laser).velocity).get_current_components();
            let (dx, dy) = (velocity_x as i16, velocity_y as i16);
            assert_eq!((dx, dy), (600 << VELOCITY_SHIFT, 0));
            battle_context::dispose(laser);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            with_rng(11, 0, || ship.postprocess());
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 38);
            assert_eq!(ship.ship_mut().special_counter, 7);
            let e = battle::tail().unwrap();
            assert_eq!(
                (
                    battle::get(e).life,
                    battle::get(e).hits,
                    battle::get(e).mass
                ),
                (80, 1, 0)
            );
            // HALF_CIRCLE is 32, added directly to facing 4 in the C code.
            assert_eq!(velocity(&battle::get(e).velocity), toward(25, 36));
            assert_eq!(
                battle::get(e).parent,
                uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
            );
            battle::get_mut(e).next = battle::get(e).current.clone();
            with_rng(11, 0, || {
                hooks::invoke_element(battle::get(e).preprocess_func.unwrap(), e)
            });
            assert_eq!((battle::get(e).next.image.frame.raw()).index(), 1);
        }
    })
}
#[test]
fn limpets_saturate_waits_and_reduce_thrust_without_rng_for_nonships() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_VUX_ID, 0);
            let mut enemy = Ship::new(&_battle, SPECIES_ID_VUX_ID, 1);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            let e = battle::tail().unwrap();
            for (increment, max, expected_inc, expected_max) in
                [(7, 21, 6, 18), (5, 25, 4, 20), (4, 20, 4, 8), (1, 10, 1, 2)]
            {
                enemy.descriptor_mut().characteristics.turn_wait = 255;
                enemy.descriptor_mut().characteristics.thrust_wait = 254;
                enemy.descriptor_mut().characteristics.thrust_increment = increment;
                enemy.descriptor_mut().characteristics.max_thrust = max;
                enemy.descriptor_mut().cyborg_control.maneuverability_index = 99;
                // Null icons make ModifySilhouette return before its placement RNG.
                with_rng(12, 1, || {
                    call_collision(
                        "ships::races::vux_native::limpet_collision",
                        e,
                        &mut Point { x: 0, y: 0 },
                        enemy.id,
                        &mut Point { x: 0, y: 0 },
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
                    enemy.descriptor_mut().cyborg_control.maneuverability_index,
                    0
                );
                assert_eq!(battle::get(e).life, 0);
                assert_eq!(battle::get(e).hits, 0);
                assert_eq!(
                    battle::get(e).state_flags & (COLLISION | DISAPPEARING) as u16,
                    (COLLISION | DISAPPEARING) as u16
                );
            }
            let object = Object::new(&_battle);
            with_rng(12, 0, || {
                call_collision(
                    "ships::races::vux_native::limpet_collision",
                    e,
                    &mut Point { x: 0, y: 0 },
                    object.id,
                    &mut Point { x: 0, y: 0 },
                )
            });
        }
    })
}
#[test]
fn warp_in_is_disabled_in_encounters_and_one_shot_in_melee() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_VUX_ID, 0);
            let enemy = Ship::new(&_battle, SPECIES_ID_VUX_ID, 1);
            battle::get_mut(enemy.id).current.location = Point { x: 5000, y: 5000 };
            battle::get_mut(enemy.id).next = battle::get(enemy.id).current.clone();
            battle::get_mut(ship.id).state_flags |= APPEARING as u16;
            let previous = crate::battle_context::with(|cx| cx.world.activity());
            crate::battle_context::with(|cx| cx.world.set_activity(IN_ENCOUNTER as _));
            with_rng(42, 0, || ship.preprocess());
            assert_eq!(battle::get(ship.id).current.location.x, 4000);
            assert_eq!(ship.descriptor_mut().callbacks()[1], 0);
            ship.descriptor_mut().race_callbacks.pre = uqm_rust::engine::battle::Selection::Default;
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            seed_random(42);
            let raw_x = 5000 - 832 + (random() as u16) % 1664;
            let raw_y = 5000 - 832 + (random() as u16) % 1664;
            let x = raw_x & !3;
            let y = raw_y & !3;
            with_rng(42, 2, || ship.preprocess());
            assert_eq!(
                (
                    battle::get(ship.id).current.location.x,
                    battle::get(ship.id).current.location.y
                ),
                (x as i16, y as i16)
            );
            let face = normalize_facing(angle_to_facing(i32::from(arctan(
                (5000 - i32::from(raw_x)) as i16,
                (5000 - i32::from(raw_y)) as i16,
            ))));
            assert_eq!(ship.ship_mut().ship_facing, face);
            assert_eq!(
                (battle::get(ship.id).current.image.frame.raw()).index(),
                face
            );
            assert!(battle::get(ship.id).target.is_none());
            assert_eq!(ship.descriptor_mut().callbacks()[1], 0);
            assert_eq!(
                battle::get(ship.id).intersect_control.end_point.x,
                (x >> ONE_SHIFT) as i16
            );
            crate::battle_context::with(|cx| cx.world.set_activity(previous));
        }
    })
}
#[test]
fn ai_changes_threat_state_and_uses_turning_energy_threshold() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_VUX_ID, 0);
            let enemy = Ship::new(&_battle, SPECIES_ID_VUX_ID, 1);
            let mut threat = Object::new(&_battle);
            ship.ship_mut().weapon_counter = 1;
            for flags in [0, FINITE_LIFE, FINITE_LIFE | CREW_OBJECT] {
                for energy in [19, 20] {
                    threat.element_mut().state_flags = flags as u16;
                    ship.descriptor_mut().ship_info.energy_level = energy;
                    ship.ship_mut().ship_input_state = LEFT as u8;
                    let mut concerns: [Concern; 4] = [Concern::default(); 4];
                    concerns[ENEMY_SHIP_INDEX as usize] = Concern {
                        object: Some(enemy.id),
                        facing: 0,
                        which_turn: 12,
                        movement: MOVEMENT_STATE_ENTICE,
                    };
                    concerns[ENEMY_WEAPON_INDEX as usize] = Concern {
                        object: Some(threat.id),
                        facing: 0,
                        which_turn: 3,
                        movement: MOVEMENT_STATE_ENTICE,
                    };
                    with_rng(8, 0, || ship.think(ship.id, &mut concerns, 0));
                    assert_eq!(
                        concerns[ENEMY_WEAPON_INDEX as usize].movement,
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

#[path = "support/hooks.rs"]
mod hooks;
