#![forbid(unsafe_code)]
#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use uqm_engine::battle::ai_constants::ENEMY_SHIP_INDEX;
use uqm_engine::battle::ai_constants::FIRES_FORE;
use uqm_engine::battle::ai_constants::IGNORE_SIMILAR;
use uqm_engine::battle::ai_constants::LEFT;
use uqm_engine::battle::ai_constants::MOVEMENT_STATE_PURSUE;
use uqm_engine::battle::ai_constants::POINT_DEFENSE;
use uqm_engine::battle::ai_constants::RIGHT;
use uqm_engine::battle::ai_constants::SEEKING_SPECIAL;
use uqm_engine::battle::ai_constants::SPECIAL;
use uqm_engine::battle::flags::APPEARING;
use uqm_engine::battle::flags::CHANGING;
use uqm_engine::battle::flags::CREW_OBJECT;
use uqm_engine::battle::flags::DISAPPEARING;
use uqm_engine::battle::flags::FINITE_LIFE;
use uqm_engine::battle::flags::NONSOLID;
use uqm_engine::battle::Concern;
use uqm_engine::battle::ElementId;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::math::battle_types::ONE_SHIFT;
use uqm_engine::species::species_id::SPECIES_ID_UR_QUAN_ID;

#[test]
fn descriptor_and_fusion_match_c() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags,
                (FIRES_FORE | SEEKING_SPECIAL) as u16
            );
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (
                        descriptor.ship_info.crew_level,
                        descriptor.ship_info.energy_level
                    ),
                    (42, 42)
                );
            }
            assert_eq!(ship.descriptor_mut().cyborg_control.weapon_range, 1600);
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (descriptor.fleet.known_loc.x, descriptor.fleet.known_loc.y),
                    (5750, 6000)
                );
            }
            ship.ship_mut().ship_facing = 11;
            let h = with_rng(8, 0, || {
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
                (20, 10, 6)
            );
            assert_eq!(battle::get(e).thrust, 8);
            assert_eq!((battle::get(e).current.image.frame.raw()).index(), 11);
            assert_eq!(
                velocity(&battle::get(e).velocity),
                toward(20 << ONE_SHIFT, 11)
            );
            assert!(battle::get(e).preprocess_func.is_none());
            battle_context::dispose(h);
        }
    })
}
#[test]
fn fighters_launch_left_then_right_and_return_crew() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            with_rng(7, 0, || ship.postprocess());
            assert_eq!(ship.descriptor_mut().ship_info.crew_level, 40);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 34);
            assert_eq!(ship.ship_mut().special_counter, 9);
            let all = queued();
            assert_eq!(all.len(), 3);
            for (i, &h) in all[1..].iter().enumerate() {
                let e = h;
                let face = if i == 0 { 10 } else { 6 };
                assert_eq!(
                    battle::get(e).state_flags,
                    (APPEARING | FINITE_LIFE | CREW_OBJECT | IGNORE_SIMILAR) as u16
                );
                assert_eq!(
                    (
                        battle::get(e).life,
                        battle::get(e).hits,
                        battle::get(e).mass
                    ),
                    (400, 1, 0)
                );
                assert_eq!(battle::get(e).thrust, 7);
                assert_eq!(
                    battle::get(e).turn,
                    if i == 0 { LEFT as u8 } else { RIGHT as u8 }
                );
                assert_eq!((battle::get(e).current.image.frame.raw()).index(), face);
                assert_eq!(
                    velocity(&battle::get(e).velocity),
                    toward(8 << ONE_SHIFT, face)
                );
                battle::get_mut(e).next = battle::get(e).current.clone();
                with_rng(7, 0, || {
                    hooks::invoke_collision(
                        battle::get(e).collision_func.unwrap(),
                        e,
                        &mut Point { x: 0, y: 0 },
                        ship.id,
                        &mut Point { x: 0, y: 0 },
                    )
                });
                assert_eq!(battle::get(e).life, 0);
                assert_eq!(battle::get(e).hits, 0);
                assert_ne!(battle::get(e).state_flags & NONSOLID as u16, 0);
                assert_eq!(battle::get(e).state_flags & DISAPPEARING as u16, 0);
            }
            assert_eq!(ship.descriptor_mut().ship_info.crew_level, 42);
        }
    })
}
#[test]
fn two_crew_launches_one_right_fighter_and_allocation_failure_keeps_crew() {
    battle_context::run(|_| {
        for capacity in [1, 2] {
            let _battle = Battle::new(capacity);
            {
                let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
                ship.ship_mut().cur_status_flags = SPECIAL as u16;
                ship.descriptor_mut().ship_info.crew_level = 2;
                battle::get_mut(ship.id).hits = 2;
                ship.postprocess();
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 34);
                assert_eq!(ship.ship_mut().special_counter, 9);
                assert_eq!(
                    ship.descriptor_mut().ship_info.crew_level,
                    if capacity == 1 { 2 } else { 1 }
                );
                if capacity == 2 {
                    assert_eq!(battle::get(battle::tail().unwrap()).turn, RIGHT as u8);
                }
            }
        }
    })
}
#[test]
fn fighter_tracking_delay_then_return_and_fire_use_exact_ranges() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
            let enemy = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 1);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            let fighter = battle::tail().unwrap();
            battle::get_mut(fighter).next = battle::get(fighter).current.clone();
            let initial = velocity(&battle::get(fighter).velocity);
            battle::get_mut(fighter).life = 394;
            with_rng(17, 0, || {
                hooks::invoke_element(battle::get(fighter).preprocess_func.unwrap(), fighter)
            });
            assert_eq!(velocity(&battle::get(fighter).velocity), initial);
            assert_eq!(battle::get(fighter).thrust, 7);
            battle::get_mut(fighter).life = 392;
            battle::get_mut(fighter).target = Some(enemy.id);
            battle::get_mut(fighter).current.location = Point { x: 4000, y: 4100 };
            battle::get_mut(fighter).thrust = 1;
            with_rng(17, 0, || {
                hooks::invoke_element(battle::get(fighter).preprocess_func.unwrap(), fighter)
            });
            assert!(battle::get(fighter).postprocess_func.is_some());
            assert_eq!(battle::get(fighter).thrust, 0);
            hooks::invoke_element(battle::get(fighter).postprocess_func.unwrap(), fighter);
            assert_eq!(battle::get(fighter).thrust, 8);
            assert!(battle::get(fighter).postprocess_func.is_none());
            let laser = battle::tail().unwrap();
            assert_eq!(
                battle::get(laser).parent,
                uqm_engine::battle::Parent::Ship(ship.ship_id)
            );
            assert_eq!(battle::get(laser).mass, 1);
            // On the return leg the ship becomes collidable with its own fighters.
            battle::get_mut(fighter).state_flags &= !(CHANGING as u16);
            battle::get_mut(fighter).life = 124;
            hooks::invoke_element(battle::get(fighter).preprocess_func.unwrap(), fighter);
            assert_eq!(battle::get(fighter).state_flags & IGNORE_SIMILAR as u16, 0);
        }
    })
}
#[test]
fn fighter_planet_collision_clones_and_deflects_without_rng() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            let e = battle::tail().unwrap();
            battle::get_mut(e).next = battle::get(e).current.clone();
            uqm_modes::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(e).velocity,
                8 << ONE_SHIFT,
                0,
            );
            let planet_fixture = Object::new(&_battle);
            let planet = planet_fixture.id;
            battle::get_mut(planet).mass = 101;
            let mut pa = Point { x: 0, y: 0 };
            let mut pb = Point { x: 0, y: -10 };
            with_rng(18, 0, || {
                hooks::invoke_collision(
                    battle::get(e).collision_func.unwrap(),
                    e,
                    &mut pa,
                    planet,
                    &mut pb,
                )
            });
            let clone = battle::tail().unwrap();
            assert_ne!(clone, e);
            assert_eq!(battle::get(e).life, 0);
            assert_eq!((battle::get(clone).next.image.frame.raw()).index(), 4);
            assert_eq!(
                velocity(&battle::get(clone).velocity),
                toward(8 << ONE_SHIFT, 4)
            );
            assert_ne!(battle::get(clone).state_flags & CHANGING as u16, 0);
        }
    })
}
#[test]
fn ai_fighter_budget_point_defense_and_crew_boundaries() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 0);
            let mut enemy = Ship::new(&_battle, SPECIES_ID_UR_QUAN_ID, 1);
            ship.ship_mut().weapon_counter = 1;
            enemy.descriptor_mut().cyborg_control.maneuverability_index = 100;
            for crew in [10, 11] {
                for fighters in [5, 6] {
                    for defense in [false, true] {
                        ship.descriptor_mut().ship_info.crew_level = crew;
                        ship.descriptor_mut().characteristics.special_wait = fighters;
                        enemy.descriptor_mut().ship_info.ship_flags =
                            if defense { POINT_DEFENSE as u16 } else { 0 };
                        let mut concerns: [Concern; 4] = [Concern::default(); 4];
                        concerns[ENEMY_SHIP_INDEX as usize].object = Some(enemy.id);
                        concerns[ENEMY_SHIP_INDEX as usize].which_turn = 20;
                        with_rng(12, 0, || ship.think(ship.id, &mut concerns, 0));
                        assert_eq!(
                            ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                            crew > 10 && fighters < 6 && !defense
                        );
                        assert_eq!(ship.descriptor_mut().characteristics.special_wait, 0);
                        assert_eq!(
                            concerns[ENEMY_SHIP_INDEX as usize].movement,
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
