#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use uqm_rust::engine::battle::ai_constants::ENEMY_SHIP_INDEX;
use uqm_rust::engine::battle::ai_constants::FIRES_FORE;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_ENTICE;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_PURSUE;
use uqm_rust::engine::battle::ai_constants::SPECIAL;
use uqm_rust::engine::battle::ai_constants::WEAPON;
use uqm_rust::engine::battle::flags::NONSOLID;
use uqm_rust::engine::battle::Concern;
use uqm_rust::engine::battle::ElementId;
use uqm_rust::engine::species::species_id::SPECIES_ID_ZOQFOTPIK_ID;
use uqm_rust::graphics::geometry::Point;
use uqm_rust::math::battle_types::ONE_SHIFT;
use uqm_rust::math::random;
use uqm_rust::math::seed_random;
use uqm_rust::math::velocity::VelocityDesc;
use uqm_rust::math::velocity::VELOCITY_SHIFT;
use uqm_rust::ships::native::{cosine, sine};
#[test]
fn descriptor_and_spit_animation_preserve_jitter_rng_and_speed() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_ZOQFOTPIK_ID, 0);
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags,
                FIRES_FORE as u16
            );
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (
                        descriptor.ship_info.crew_level,
                        descriptor.ship_info.energy_level
                    ),
                    (10, 10)
                );
            }
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (descriptor.fleet.known_loc.x, descriptor.fleet.known_loc.y),
                    (3761, 5333)
                );
            }
            assert_eq!(ship.descriptor_mut().cyborg_control.weapon_range, 400);
            let h = with_rng(19, 0, || {
                ship.weapon(1)
                    .into_iter()
                    .map(|id| id.expect("allocated weapon"))
                    .collect::<Vec<ElementId>>()
            })[0];
            let e = h;
            assert_eq!(
                (
                    battle::get(e).life,
                    battle::get(e).hits,
                    battle::get(e).mass
                ),
                (10, 1, 1)
            );
            assert_eq!(
                velocity(&battle::get(e).velocity),
                toward(16 << ONE_SHIFT << 1, 0)
            );
            seed_random(19);
            let angle = ((random() as u16) % 3).wrapping_sub(1);
            with_rng(19, 1, || {
                hooks::invoke_element(battle::get(e).preprocess_func.unwrap(), e)
            });
            assert_eq!((battle::get(e).next.image.frame.raw()).index(), 1);
            assert_eq!(battle::get(e).turn, 2);
            let mut v: VelocityDesc = VelocityDesc::new();
            let speed = (15 << ONE_SHIFT) << 1 << VELOCITY_SHIFT;
            (v).set_components(
                i32::from(cosine(i32::from(angle), speed)),
                i32::from(sine(i32::from(angle), speed)),
            );
            assert_eq!(velocity(&battle::get(e).velocity), velocity(&v));
            for expected in [1, 0] {
                with_rng(19, 0, || {
                    hooks::invoke_element(battle::get(e).preprocess_func.unwrap(), e)
                });
                assert_eq!(battle::get(e).turn, expected);
                assert_eq!((battle::get(e).next.image.frame.raw()).index(), 1);
            }
            with_rng(19, 0, || {
                hooks::invoke_element(battle::get(e).preprocess_func.unwrap(), e)
            });
            assert_eq!((battle::get(e).next.image.frame.raw()).index(), 2);
            assert_eq!(battle::get(e).turn, 2);
            battle_context::dispose(h);
        }
    })
}
#[test]
fn tongue_chain_extends_in_facing_direction_and_stops_at_zero() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_ZOQFOTPIK_ID, 0);
            ship.ship_mut().ship_facing = 0;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            with_rng(3, 0, || ship.postprocess());
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 3);
            assert_eq!(ship.ship_mut().special_counter, 6);
            let mut e = battle::tail().unwrap();
            assert_eq!(battle::get(e).turn, 6);
            for remaining in (0..6).rev() {
                battle::get_mut(e).next = battle::get(e).current.clone();
                let y = battle::get(e).next.location.y;
                with_rng(3, 0, || {
                    hooks::invoke_element(battle::get(e).postprocess_func.unwrap(), e)
                });
                e = battle::tail().unwrap();
                assert_eq!(battle::get(e).turn, remaining);
                assert_eq!(battle::get(e).current.location.y, y - 8);
                assert_eq!((battle::get(e).life, battle::get(e).mass), (1, 12));
                assert_eq!(velocity(&battle::get(e).velocity), [0; 9]);
            }
            let all = queued();
            assert_eq!(all.len(), 8);
            hooks::invoke_element(battle::get(e).postprocess_func.unwrap(), e);
            assert_eq!(queued(), all);
        }
    })
}
#[test]
fn first_tongue_collision_damages_but_later_frames_only_cancel_extension() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_ZOQFOTPIK_ID, 0);
            let mut enemy = Ship::new(&_battle, SPECIES_ID_ZOQFOTPIK_ID, 1);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            let tongue = battle::tail().unwrap();
            let mut a = Point { x: 100, y: 100 };
            let mut b = Point { x: 110, y: 100 };
            with_rng(7, 0, || {
                hooks::invoke_collision(
                    battle::get(tongue).collision_func.unwrap(),
                    tongue,
                    &mut a,
                    enemy.id,
                    &mut b,
                )
            });
            assert_eq!(enemy.descriptor_mut().ship_info.crew_level, 0);
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(battle::get(tongue).turn, 0);
            assert_ne!(battle::get(tongue).state_flags & NONSOLID as u16, 0);
            // A later frame has no damage window even if it still has mass 12.
            ship.ship_mut().special_counter = 5;
            let target_fixture = Object::new(&_battle);
            let target = target_fixture.id;
            battle::get_mut(target).life = 1;
            battle::get_mut(tongue).turn = 4;
            let before = queued();
            hooks::invoke_collision(
                battle::get(tongue).collision_func.unwrap(),
                tongue,
                &mut a,
                target,
                &mut b,
            );
            assert_eq!(ship.ship_mut().special_counter, 1);
            assert_eq!(queued(), before);
        }
    })
}
#[test]
fn tongue_allocation_failure_still_spends_energy_and_sets_counter() {
    battle_context::run(|_| {
        let _battle = Battle::new(1);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_ZOQFOTPIK_ID, 0);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 3);
            assert_eq!(ship.ship_mut().special_counter, 6);
            assert_eq!(queued(), vec![ship.id]);
        }
    })
}
#[test]
fn ai_tongue_range_and_facing_suppress_spit_without_energy_gate() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_ZOQFOTPIK_ID, 0);
            let enemy = Ship::new(&_battle, SPECIES_ID_ZOQFOTPIK_ID, 1);
            battle::get_mut(enemy.id).next.location = Point { x: 4000, y: 3900 };
            ship.descriptor_mut().ship_info.energy_level = 0;
            for (turn, face, tongue) in [(4, 0, true), (4, 1, false), (5, 0, false)] {
                let mut concerns: [Concern; 4] = [Concern::default(); 4];
                concerns[ENEMY_SHIP_INDEX as usize] = Concern {
                    object: Some(enemy.id),
                    facing: 0,
                    which_turn: turn,
                    movement: MOVEMENT_STATE_ENTICE,
                };
                ship.ship_mut().ship_facing = face;
                ship.ship_mut().ship_input_state = WEAPON as u8;
                with_rng(5, 0, || ship.think(ship.id, &mut concerns, 0));
                assert_eq!(
                    ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                    tongue
                );
                assert_eq!(ship.ship_mut().ship_input_state & WEAPON as u8, 0);
                assert_eq!(ship.ship_mut().weapon_counter, 0);
                assert_eq!(
                    concerns[ENEMY_SHIP_INDEX as usize].movement,
                    if turn <= 4 {
                        MOVEMENT_STATE_PURSUE
                    } else {
                        MOVEMENT_STATE_ENTICE
                    }
                );
            }
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
