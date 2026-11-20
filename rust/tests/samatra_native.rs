#![forbid(unsafe_code)]
use uqm_rust::battle::ContextOps;
use uqm_rust::engine::battle::flags::APPEARING;
use uqm_rust::engine::battle::flags::DEFY_PHYSICS;
use uqm_rust::engine::battle::flags::DISAPPEARING;
use uqm_rust::engine::battle::flags::FINITE_LIFE;
use uqm_rust::engine::battle::flags::IN_LAST_BATTLE;
use uqm_rust::engine::battle::flags::NONSOLID;
use uqm_rust::engine::battle::flags::NPC_PLAYER_NUM;
use uqm_rust::engine::battle::flags::PLAYER_SHIP;
use uqm_rust::engine::battle::flags::RPG_PLAYER_NUM;
use uqm_rust::engine::battle::flags::SHIP_AT_MAX_SPEED;
use uqm_rust::engine::battle::flags::SHIP_BEYOND_MAX_SPEED;
use uqm_rust::engine::battle::flags::STAMPFILL_PRIMITIVE;
use uqm_rust::engine::battle::STAMP_PRIMITIVE;
use uqm_rust::engine::species::species_id::SPECIES_ID_CHENJESU_ID;
use uqm_rust::engine::species::species_id::SPECIES_ID_SA_MATRA_ID;
use uqm_rust::engine::species::species_id::SPECIES_ID_SIS_SHIP_ID;
use uqm_rust::engine::world::activity::activity_flags::IN_BATTLE;
use uqm_rust::engine::world::activity::activity_kinds::IN_ENCOUNTER;
use uqm_rust::engine::world::activity::activity_kinds::WON_LAST_BATTLE;
use uqm_rust::graphics::geometry::Point;
#[test]
fn terminal_outcome_survives_descriptor_teardown_until_the_next_battle() {
    battle_context::run(|_| {
        let battle = Battle::new(64);
        {
            use uqm_rust::automation::samatra::observe;
            crate::battle_context::with(|cx| uqm_rust::state::ffi::rust_init_game_state(cx.world));
            crate::battle_context::with(|cx| cx.world.set_activity(IN_LAST_BATTLE as u16));
            crate::battle_context::with(|cx| cx.b.initialize(cx.kernel));
            let mut ship = boss(&battle);
            let expected = uqm_engine::battle::observation::samatra::Events {
                generation: 1,
                destroyed: 255,
                gate_entered: true,
            };
            {
                let mut descriptor = ship.descriptor_mut();
                let RaceState::SaMatra(ref mut state) = descriptor.race_state else {
                    panic!("boss owns Sa-Matra state")
                };
                state.destroyed = expected.destroyed;
                state.gate_entered = expected.gate_entered;
            }
            crate::battle_context::with(|cx| cx.finish_battle());
            assert!(matches!(ship.descriptor_mut().race_state, RaceState::None));
            drop(ship);
            crate::battle_context::with(|cx| {
                let scaled = cx.kernel_mut().configuration.preferences().melee_scale == 0;
                uqm_rust::battle::process_native::init_display_list(cx, scaled);
            });
            crate::battle_context::with(|cx| cx.world.set_activity(WON_LAST_BATTLE));
            assert_eq!(
                crate::battle_context::with(|cx| observe(cx)),
                (expected, None)
            );
            // A non-boss battle must clear the result too, before any ship spawns.
            crate::battle_context::with(|cx| cx.world.set_activity(IN_ENCOUNTER | IN_BATTLE));
            crate::battle_context::with(|cx| cx.b.initialize(cx.kernel));
            crate::battle_context::with(|cx| assert!(cx.terminal_outcome().is_none()));
            crate::battle_context::with(|cx| cx.world.set_activity(IN_LAST_BATTLE as u16));
            assert_eq!(
                crate::battle_context::with(|cx| observe(cx)),
                (Default::default(), None)
            );
        }
    });
}

#[test]
fn live_observation_projects_owned_events_generators_hazards_and_player() {
    battle_context::run(|_| {
        let battle = Battle::new(64);
        {
            use uqm_rust::automation::samatra::observe;
            crate::battle_context::with(|cx| uqm_rust::state::ffi::rust_init_game_state(cx.world));
            assert_eq!(
                crate::battle_context::with(|cx| observe(cx)).0.generation,
                0
            );
            assert!(crate::battle_context::with(|cx| observe(cx)).1.is_none());
            let mut ship = boss(&battle);
            let mut player = Ship::with_frames(
                &battle,
                SPECIES_ID_CHENJESU_ID,
                RPG_PLAYER_NUM as i16,
                [16, 10, 6],
                [4, 4],
            );
            player.ship_mut().species_id = SPECIES_ID_SIS_SHIP_ID;
            player.ship_mut().ship_facing = 19;
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::BOMB_CARRIER, 1)
            });
            crate::battle_context::with(|cx| {
                cx.world.set_activity(IN_LAST_BATTLE as u16 | IN_BATTLE)
            });
            let (events, snapshot) = crate::battle_context::with(|cx| observe(cx));
            let snapshot = snapshot.unwrap_or_else(|| panic!("live final battle"));
            assert_eq!(events.generation, 1);
            assert_eq!(events.destroyed, 0);
            assert!(!events.gate_entered);
            assert_eq!(snapshot.remaining, 8);
            assert_eq!(
                snapshot.generators.iter().map(|g| g.id).collect::<Vec<_>>(),
                (0..8).rev().collect::<Vec<_>>()
            );
            assert!(snapshot.hazards.is_empty());
            assert!(!snapshot.gate_open);
            assert_eq!(snapshot.gate, (4096, 3840));
            assert!(snapshot.bomb_carrier);
            assert_eq!(snapshot.selected_species, Some(SPECIES_ID_SIS_SHIP_ID));
            assert_eq!(snapshot.player.unwrap().facing, 3);
            assert_eq!(snapshot.velocity, (0, 0));
            ship.ship_mut().weapon_counter = 0;
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            assert_eq!(
                crate::battle_context::with(|cx| observe(cx))
                    .1
                    .unwrap()
                    .hazards
                    .len(),
                2
            );
            let sentinel = queued()
                .into_iter()
                .find(|h| battle::get(*h).hits == 10)
                .unwrap();
            battle::get_mut(sentinel).state_flags |= DISAPPEARING as u16;
            assert_eq!(
                crate::battle_context::with(|cx| observe(cx))
                    .1
                    .unwrap()
                    .hazards
                    .len(),
                1
            );
            let state = owned_state(&mut ship);
            for generator in state.generators.into_iter().flatten() {
                call_element("ships::races::samatra::generator_death", generator);
            }
            let gate = queued()
                .into_iter()
                .find(|h| {
                    battle::get(*h).collision_func.is_some_and(|hook| {
                        uqm_rust::battle::hook_registry::name(
                            &uqm_rust::battle::hook_registry::hooks(),
                            hook.address(),
                        ) == "ships::races::samatra::gate_collision"
                    })
                })
                .unwrap();
            call_element("ships::races::samatra::gate_preprocess", gate);
            let (events, snapshot) = crate::battle_context::with(|cx| observe(cx));
            assert_eq!(events.destroyed, 255);
            let snapshot = snapshot.unwrap();
            assert_eq!(snapshot.remaining, 0);
            assert!(snapshot.generators.is_empty());
            assert!(snapshot.gate_open);
            let mut pa = Point { x: 0, y: 0 };
            let mut pb = pa;
            call_collision(
                "ships::races::samatra::gate_collision",
                gate,
                &mut pa,
                player.id,
                &mut pb,
            );
            let (events, snapshot) = crate::battle_context::with(|cx| observe(cx));
            assert!(events.gate_entered);
            assert_eq!(events.destroyed, 255);
            assert!(snapshot.is_none());
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::BOMB_CARRIER, 0)
            });
        }
    });
}

use uqm_rust::engine::battle::{race::SaMatraState, RaceState};

fn owned_state(ship: &mut Ship<'_>) -> SaMatraState {
    let RaceState::SaMatra(state) = ({ ship.descriptor_mut() }).race_state else {
        panic!("boss owns Sa-Matra state")
    };
    state
}

#[test]
fn lifecycle_counts_each_identity_once_and_retains_gate_entry() {
    battle_context::run(|_| {
        let battle = Battle::new(64);
        {
            crate::battle_context::with(|cx| uqm_rust::state::ffi::rust_init_game_state(cx.world));
            let mut ship = boss(&battle);
            let mut other = Ship::with_frames(
                &battle,
                SPECIES_ID_SA_MATRA_ID,
                RPG_PLAYER_NUM as i16,
                [32; 3],
                [4, 4],
            );
            let initial = owned_state(&mut ship);
            assert_eq!(initial.generation, 1);
            assert_eq!(owned_state(&mut other).generation, 0);
            for id in initial.generators.into_iter().flatten() {
                call_element("ships::races::samatra::generator_death", id);
                call_element("ships::races::samatra::generator_death", id);
            }
            assert_eq!(owned_state(&mut ship).destroyed, 255);
            assert_eq!(owned_state(&mut other).destroyed, 0);
            let gate = queued()
                .into_iter()
                .find(|h| {
                    battle::get(*h).collision_func.is_some_and(|hook| {
                        uqm_rust::battle::hook_registry::name(
                            &uqm_rust::battle::hook_registry::hooks(),
                            hook.address(),
                        ) == "ships::races::samatra::gate_collision"
                    })
                })
                .unwrap_or_else(|| panic!("boss gate"));
            call_element("ships::races::samatra::gate_preprocess", gate);
            crate::battle_context::with(|cx| cx.world.set_activity(IN_BATTLE));
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::BOMB_CARRIER, 1)
            });
            let mut pa = Point { x: 0, y: 0 };
            let mut pb = pa;
            call_collision(
                "ships::races::samatra::gate_collision",
                gate,
                &mut pa,
                other.id,
                &mut pb,
            );
            assert!(owned_state(&mut ship).gate_entered);
            assert!(!owned_state(&mut other).gate_entered);
            assert_eq!(owned_state(&mut ship).generation, initial.generation);
            ship.preprocess();
            let restarted = owned_state(&mut ship);
            assert_eq!(restarted.generation, initial.generation + 1);
            assert_eq!(restarted.destroyed, 0);
            assert!(!restarted.gate_entered);
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::BOMB_CARRIER, 0)
            });
        }
    });
}

#[test]
fn identity_does_not_depend_on_scrolling_or_survivor_order() {
    battle_context::run(|_| {
        let battle = Battle::new(64);
        {
            let mut ship = boss(&battle);
            let generators = owned_state(&mut ship).generators;
            for (index, generator) in generators.into_iter().enumerate().rev() {
                let generator = generator.unwrap_or_else(|| panic!("eight allocated generators"));
                crate::battle_context::with(|cx| {
                    cx.element_mut(generator).current.location = Point { x: 17, y: 31 };
                    uqm_rust::battle::sounds::engine::unlink(cx, generator);
                    cx.put(generator);
                });
                call_element("ships::races::samatra::generator_death", generator);
                let state = owned_state(&mut ship);
                assert_eq!(state.generators[index], Some(generator));
                assert_eq!(state.destroyed, 255u8 << index);
            }
        }
    });
}

#[test]
fn sentinel_tracking_preserves_absent_owner_element() {
    battle_context::run(|_| {
        let battle = Battle::new(64);
        {
            let mut ship = boss(&battle);
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            let sentinel = queued()
                .into_iter()
                .find(|h| battle::get(*h).hits == 10)
                .unwrap_or_else(|| panic!("sentinel"));
            battle::get_mut(sentinel).turn = 0;
            ship.ship_mut().ship_element = None;
            crate::battle_context::with(|cx| {
                assert_eq!(cx.optional_ship_element(ship.ship_id), None);
            });
            with_rng(5, 0, || {
                call_element("ships::races::samatra::sentinel_preprocess", sentinel)
            });
            assert_eq!(battle::get(sentinel).turn, 1);
        }
    });
}

#[test]
fn assigned_races_use_native_dispatch() {
    for species in [SPECIES_ID_CHENJESU_ID, SPECIES_ID_SA_MATRA_ID] {
        assert!(uqm_rust::ships::races::race(species).is_some());
    }
}

#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use uqm_rust::state::game_state_keys::set_game_state;

fn boss(battle: &Battle) -> Ship<'_> {
    let mut ship = Ship::with_frames(
        battle,
        SPECIES_ID_SA_MATRA_ID,
        NPC_PLAYER_NUM as i16,
        [32; 3],
        [4, 4],
    );
    battle::get_mut(ship.id).state_flags |= APPEARING as u16;
    with_rng(7, 8, || ship.preprocess());
    ship
}
#[test]
fn generators_spawn_at_exact_offsets_and_decrement_once_before_gate_opens() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = boss(&_battle);
            assert_eq!(ship.descriptor_mut().characteristics.max_thrust, 8);
            {
                let record = ship.ship_mut();
                assert_eq!((record.weapon_counter, record.special_counter), (120, 36));
            }
            let all = queued();
            assert_eq!(all.len(), 11);
            let generators: Vec<_> = all
                .iter()
                .copied()
                .filter(|h| battle::get(*h).death_func.is_some())
                .collect();
            let mut positions: Vec<_> = generators
                .iter()
                .map(|h| {
                    let e = *h;
                    assert_eq!(battle::get(e).hits, 15);
                    (
                        battle::get(e).current.location.x,
                        battle::get(e).current.location.y,
                    )
                })
                .collect();
            positions.sort();
            let mut expected: Vec<_> = [
                (-136, -35),
                (-47, -70),
                (35, -67),
                (118, -42),
                (115, 46),
                (64, 79),
                (-96, 76),
                (-145, 47),
            ]
            .iter()
            .map(|(x, y)| ((4096 + x * 4) & !15, (3840 + y * 4) & !15))
            .collect();
            expected.sort();
            assert_eq!(positions, expected);
            let gate = *all
                .iter()
                .find(|h| battle::get(**h).state_flags & FINITE_LIFE as u16 != 0)
                .unwrap();
            for (i, h) in generators.iter().enumerate() {
                hooks::invoke_element(battle::get(gate).preprocess_func.unwrap(), gate);
                assert_eq!(battle::prim(gate).kind, STAMP_PRIMITIVE);
                let g = *h;
                hooks::invoke_element(battle::get(g).death_func.unwrap(), g);
                assert_eq!(
                    ship.descriptor_mut().characteristics.max_thrust,
                    7 - i as u16
                );
                assert_eq!((battle::get(g).current.image.frame.raw()).index(), 9);
                hooks::invoke_element(battle::get(g).death_func.unwrap(), g);
                assert_eq!(
                    ship.descriptor_mut().characteristics.max_thrust,
                    7 - i as u16
                );
            }
            hooks::invoke_element(battle::get(gate).preprocess_func.unwrap(), gate);
            assert!(battle::get(gate).preprocess_func.is_none());
            assert_eq!(battle::get(gate).state_flags & FINITE_LIFE as u16, 0);
            assert_eq!(battle::prim(gate).kind, STAMPFILL_PRIMITIVE);
            assert_eq!(battle::get(gate).life, 2);
        }
    })
}
#[test]
fn only_player_ship_bomb_carrier_through_open_gate_finishes_battle() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            crate::battle_context::with(|cx| uqm_rust::state::ffi::rust_init_game_state(cx.world));
            let mut ship = boss(&_battle);
            let gate = *queued()
                .iter()
                .find(|h| battle::get(**h).state_flags & FINITE_LIFE as u16 != 0)
                .unwrap();
            let player = Ship::with_frames(
                &_battle,
                SPECIES_ID_SA_MATRA_ID,
                RPG_PLAYER_NUM as i16,
                [32; 3],
                [4, 4],
            );
            let mut point = Point { x: 0, y: 0 };
            let mut other_point = point;
            let collide = battle::get(gate).collision_func.unwrap();
            for generators in [8, 1, 0] {
                ship.descriptor_mut().characteristics.max_thrust = generators;
                if generators == 0 {
                    hooks::invoke_element(battle::get(gate).preprocess_func.unwrap(), gate);
                }
                for bomb in [0, 1] {
                    for is_ship in [false, true] {
                        for owner in [RPG_PLAYER_NUM, NPC_PLAYER_NUM] {
                            crate::battle_context::with(|cx| cx.world.set_activity(IN_BATTLE));
                            crate::battle_context::with(|cx| {
                                set_game_state(
                                    cx.world,
                                    uqm_rust::engine::StoryKey::BOMB_CARRIER,
                                    bomb,
                                )
                            });
                            battle::get_mut(player.id).player_nr = owner as i16;
                            battle::get_mut(player.id).state_flags = if is_ship {
                                PLAYER_SHIP as u16
                            } else {
                                FINITE_LIFE as u16
                            };
                            hooks::invoke_collision(
                                collide,
                                gate,
                                &mut point,
                                player.id,
                                &mut other_point,
                            );
                            assert_eq!(
                                crate::battle_context::with(|cx| cx.world.activity()) & IN_BATTLE
                                    == 0,
                                generators == 0 && bomb == 1 && is_ship && owner == RPG_PLAYER_NUM
                            );
                        }
                    }
                }
            }
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::BOMB_CARRIER, 0)
            });
        }
    })
}
#[test]
fn postprocess_spawns_comets_and_sentinels_only_while_generators_live() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = boss(&_battle);
            ship.ship_mut().weapon_counter = 0;
            ship.ship_mut().special_counter = 0;
            with_rng(5, 1, || ship.postprocess());
            assert_eq!(queued().len(), 13);
            {
                let record = ship.ship_mut();
                assert_eq!((record.weapon_counter, record.special_counter), (240, 72));
            }
            let comet = *queued()
                .iter()
                .find(|h| (battle::get(**h).current.image.frame.raw()).index() == 24)
                .unwrap();
            battle::get_mut(comet).next = battle::get(comet).current.clone();
            with_rng(5, 0, || {
                hooks::invoke_element(battle::get(comet).preprocess_func.unwrap(), comet)
            });
            assert_eq!(ship.descriptor_mut().characteristics.weapon_wait, 1);
            assert_eq!(queued().len(), 14);
            assert_ne!(battle::get(comet).state_flags & NONSOLID as u16, 0);
            crate::battle_context::with(|cx| {
                cx.world.set_activity(IN_LAST_BATTLE as u16 | IN_BATTLE)
            });
            let hazards =
                crate::battle_context::with(|cx| uqm_rust::automation::samatra::observe(cx))
                    .1
                    .unwrap()
                    .hazards;
            assert_eq!(hazards.len(), 3);
            assert_eq!(hazards.iter().filter(|h| h.solid).count(), 2);
            let sentinel = *queued()
                .iter()
                .find(|h| battle::get(**h).hits == 10)
                .unwrap();
            battle::get_mut(sentinel).next = battle::get(sentinel).current.clone();
            hooks::invoke_element(battle::get(sentinel).preprocess_func.unwrap(), sentinel);
            assert_eq!(ship.descriptor_mut().characteristics.special_wait, 1);
            assert_eq!(battle::get(sentinel).life, 3);
            ship.descriptor_mut().characteristics.max_thrust = 0;
            ship.ship_mut().weapon_counter = 0;
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            assert_eq!(queued().len(), 14);
        }
    })
}

#[test]
fn sentinel_recoil_stuns_live_player_caps_velocity_and_preserves_sentinel() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = boss(&_battle);
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            let sentinel = *queued()
                .iter()
                .find(|h| battle::get(**h).hits == 10)
                .unwrap();
            let mut player = Ship::with_frames(
                &_battle,
                SPECIES_ID_SA_MATRA_ID,
                RPG_PLAYER_NUM as i16,
                [32; 3],
                [4, 4],
            );
            player.descriptor_mut().ship_info.crew_level = 20;
            battle::get_mut(player.id).hits = 20;
            battle::get_mut(player.id).mass = 10;
            player.ship_mut().cur_status_flags = (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
            battle::get_mut(player.id).velocity.set_components(5000, 0);
            uqm_rust::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(sentinel).velocity,
                32,
                4,
            );
            let mut p = Point { x: 0, y: 0 };
            let mut q = p;
            hooks::invoke_collision(
                battle::get(sentinel).collision_func.unwrap(),
                sentinel,
                &mut p,
                player.id,
                &mut q,
            );
            assert_eq!(battle::get(sentinel).hits, 10);
            assert_eq!(battle::get(sentinel).life, 2);
            assert_eq!(battle::get(sentinel).turn, 12);
            assert_ne!(battle::get(sentinel).state_flags & DEFY_PHYSICS as u16, 0);
            assert_eq!(battle::get(sentinel).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(
                player.ship_mut().cur_status_flags
                    & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16,
                0
            );
            assert_eq!(battle::get(player.id).turn, 1);
            assert_eq!(battle::get(player.id).thrust, 3);
            let (velocity_x, velocity_y) =
                (battle::get(player.id).velocity).get_current_components();
            let (dx, dy) = (velocity_x as i16, velocity_y as i16);
            assert_eq!((dx, dy), (5120, 0));
            assert_eq!(velocity(&battle::get(sentinel).velocity), [0; 9]);
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
