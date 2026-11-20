#![forbid(unsafe_code)]
#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use uqm_rust::engine::battle::ai_constants::ENEMY_WEAPON_INDEX;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_ENTICE;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_PURSUE;
use uqm_rust::engine::battle::ai_constants::SPECIAL;
use uqm_rust::engine::battle::ai_constants::WEAPON;
use uqm_rust::engine::battle::flags::APPEARING;
use uqm_rust::engine::battle::flags::CHANGING;
use uqm_rust::engine::battle::flags::CREW_OBJECT;
use uqm_rust::engine::battle::flags::DISAPPEARING;
use uqm_rust::engine::battle::flags::NONSOLID;
use uqm_rust::engine::battle::flags::PLAYER_SHIP;
use uqm_rust::engine::battle::flags::POST_PROCESS;
use uqm_rust::engine::battle::flags::PRE_PROCESS;
use uqm_rust::engine::battle::Concern;
use uqm_rust::engine::species::species_id::SPECIES_ID_KOHR_AH_ID;
use uqm_rust::graphics::geometry::Point;
use uqm_rust::math::battle_types::ONE_SHIFT;
use uqm_rust::math::velocity::VelocityDesc;

#[test]
fn descriptor_saw_launch_hold_release_decelerate_and_tracking() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_KOHR_AH_ID, 0);
            assert_eq!(ship.descriptor_mut().ship_info.ship_cost, 30);
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
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (descriptor.fleet.known_loc.x, descriptor.fleet.known_loc.y),
                    (6000, 6250)
                );
            }
            assert_eq!(ship.descriptor_mut().cyborg_control.weapon_range, 200);
            ship.ship_mut().ship_facing = 4;
            let h = with_rng(9, 0, || ship.weapon(1))[0];
            let saw = h.expect("allocated weapon");
            let h = saw;
            assert_eq!(
                (
                    battle::get(saw).life,
                    battle::get(saw).hits,
                    battle::get(saw).mass
                ),
                (64, 10, 4)
            );
            assert_eq!(velocity(&battle::get(saw).velocity), toward(64, 4));
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            ship.descriptor_mut().characteristics.special_wait = 0;
            with_rng(9, 0, || {
                hooks::invoke_element(battle::get(saw).preprocess_func.unwrap(), saw)
            });
            assert_eq!(battle::get(saw).life, 65);
            assert_eq!((battle::get(saw).next.image.frame.raw()).index(), 1);
            ship.ship_mut().cur_status_flags = 0;
            hooks::invoke_element(battle::get(saw).preprocess_func.unwrap(), saw);
            assert_eq!(battle::get(saw).life, 33);
            ship.descriptor_mut().characteristics.special_wait = 0;
            let decelerate = battle::get(saw).preprocess_func.unwrap();
            for speed in [1024, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1, 0] {
                hooks::invoke_element(decelerate, saw);
                let (velocity_x, velocity_y) = (battle::get(saw).velocity).get_current_components();
                let (dx, dy) = (velocity_x as i16, velocity_y as i16);
                assert_eq!((dx, dy), (speed, 0));
                ship.descriptor_mut().characteristics.special_wait = 0;
            }
            with_rng(9, 0, || {
                hooks::invoke_element(battle::get(saw).preprocess_func.unwrap(), saw)
            });
            assert_eq!(velocity(&battle::get(saw).velocity), [0; 9]);
            battle_context::dispose(h);
        }
    })
}
#[test]
fn ninth_saw_expires_and_postprocess_requeues_after_ship_with_intersection_points() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_KOHR_AH_ID, 0);
            let h = ship.weapon(1)[0];
            let saw = h.expect("allocated weapon");
            let h = saw;
            battle::put(h);
            battle::get_mut(saw).next.location = Point { x: 1235, y: 2347 };
            hooks::invoke_element(battle::get(saw).postprocess_func.unwrap(), saw);
            let all = queued();
            assert_eq!(all.len(), 3);
            let copy = all[1];
            assert_eq!(all[2], h);
            assert_eq!(battle::get(saw).life, 0);
            assert!(battle::get(copy).postprocess_func.is_none());
            assert_eq!(
                (
                    battle::get(copy).current.location.x,
                    battle::get(copy).current.location.y
                ),
                (1235, 2347)
            );
            assert_eq!(
                (
                    battle::get(copy).intersect_control.end_point.x,
                    battle::get(copy).intersect_control.end_point.y
                ),
                (1235 >> ONE_SHIFT, 2347 >> ONE_SHIFT)
            );
            assert_eq!(
                battle::get(copy).state_flags & (APPEARING | CHANGING | PRE_PROCESS) as u16,
                0
            );
            assert_ne!(battle::get(copy).state_flags & POST_PROCESS as u16, 0);
            ship.descriptor_mut().characteristics.special_wait = 8;
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            hooks::invoke_element(battle::get(copy).preprocess_func.unwrap(), copy);
            assert_eq!(battle::get(copy).life, 1);
            assert_ne!(battle::get(copy).state_flags & DISAPPEARING as u16, 0);
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
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_KOHR_AH_ID, 0);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            (battle::get_mut(ship.id).velocity).set_components(41, -21);
            with_rng(12, 0, || ship.postprocess());
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 21);
            assert_eq!(ship.ship_mut().special_counter, 9);
            let all = queued();
            assert_eq!(all.len(), 17);
            for (face, &h) in all[1..].iter().enumerate() {
                let e = h;
                assert_eq!(
                    (
                        battle::get(e).life,
                        battle::get(e).hits,
                        battle::get(e).mass
                    ),
                    (47, 100, 3)
                );
                assert_eq!(battle::get(e).turn, 1);
                let mut expected: VelocityDesc = VelocityDesc::new();
                uqm_rust::battle::velocity::set_velocity_vector(&mut expected, 16, face as u16);
                expected.delta_components(41, -21);
                assert_eq!(velocity(&battle::get(e).velocity), velocity(&expected));
                hooks::invoke_element(battle::get(e).preprocess_func.unwrap(), e);
                assert_eq!(battle::get(e).turn, 0);
                hooks::invoke_element(battle::get(e).preprocess_func.unwrap(), e);
                assert_eq!((battle::get(e).next.image.frame.raw()).index(), 1);
                assert_eq!(battle::get(e).turn, 2);
                assert!(battle::get(e).target.is_none());
            }
        }
    })
}
#[test]
fn saw_collision_splinters_and_cloud_damage_depends_on_ship_flag() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_KOHR_AH_ID, 0);
            let h = ship.weapon(1)[0];
            let saw = h.expect("allocated weapon");
            let h = saw;
            let target_fixture = Object::new(&_battle);
            let target = target_fixture.id;
            battle::get_mut(target).player_nr = 1;
            battle::get_mut(target).life = 2;
            let mut p = Point { x: 100, y: 200 };
            let mut q = p;
            hooks::invoke_collision(
                battle::get(saw).collision_func.unwrap(),
                saw,
                &mut p,
                target,
                &mut q,
            );
            assert_eq!(battle::get(saw).life, 5);
            assert_eq!((battle::get(saw).next.image.frame.raw()).index(), 2);
            assert_eq!(battle::get(saw).state_flags & DISAPPEARING as u16, 0);
            assert_ne!(battle::get(saw).state_flags & NONSOLID as u16, 0);
            battle_context::dispose(h);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            let cloud = battle::tail().unwrap();
            let collide = battle::get(cloud).collision_func.unwrap();
            hooks::invoke_collision(collide, cloud, &mut p, target, &mut q);
            assert_eq!(battle::get(cloud).mass, 50);
            battle::get_mut(target).state_flags = PLAYER_SHIP as u16;
            // Already collided: weapon_collision returns without needing target parent.
            hooks::invoke_collision(collide, cloud, &mut p, target, &mut q);
            assert_eq!(battle::get(cloud).mass, 3);
        }
    })
}
#[test]
fn ai_crew_threat_boundaries_and_no_target_do_not_draw_rng() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_KOHR_AH_ID, 0);
            let threat_fixture = Object::new(&_battle);
            let threat = threat_fixture.id;
            battle::get_mut(threat).state_flags = CREW_OBJECT as u16;
            for (turn, pursue) in [(8, true), (9, false)] {
                let mut concerns: [Concern; 4] = [Concern::default(); 4];
                concerns[ENEMY_WEAPON_INDEX as usize] = Concern {
                    object: Some(threat),
                    facing: 0,
                    which_turn: turn,
                    movement: MOVEMENT_STATE_ENTICE,
                };
                with_rng(4, 0, || ship.think(ship.id, &mut concerns, 0));
                assert_eq!(
                    concerns[ENEMY_WEAPON_INDEX as usize].movement,
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
            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            with_rng(4, 0, || ship.think(ship.id, &mut concerns, 0));
            assert_eq!(ship.ship_mut().ship_input_state & SPECIAL as u8, 0);
            let out = ship.weapon(1);
            battle_context::dispose(out[0].expect("allocated weapon"));
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
