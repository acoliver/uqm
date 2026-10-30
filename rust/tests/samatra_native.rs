#[test]
fn live_observation_projects_owned_events_generators_hazards_and_player() {
    battle_context::run(|| {
        let battle = Battle::new(64);
        // SAFETY: the serialized fixture publishes the live context for observation.
        unsafe {
            use uqm_rust::automation::samatra::observe;
            uqm_rust::state::ffi::rust_init_game_state();
            assert_eq!(observe().0.generation, 0);
            assert!(observe().1.is_none());
            let mut ship = boss(&battle);
            let mut player = Ship::with_frames(
                &battle,
                SPECIES_ID_CHENJESU_ID,
                RPG_PLAYER_NUM as SIZE,
                [16, 10, 6],
                [4, 4],
            );
            player.ship_mut().SpeciesID = SPECIES_ID_SIS_SHIP_ID;
            player.ship_mut().ShipFacing = 19;
            set_game_state("BOMB_CARRIER", 1);
            GlobData.Game_state.CurrentActivity = (IN_LAST_BATTLE | IN_BATTLE) as u16;
            let (events, snapshot) = observe();
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
            assert_eq!(observe().1.unwrap().hazards.len(), 2);
            let sentinel = queued()
                .into_iter()
                .find(|h| (*element(*h)).__bindgen_anon_2.hit_points == 10)
                .unwrap();
            (*element(sentinel)).state_flags |= DISAPPEARING as u16;
            assert_eq!(observe().1.unwrap().hazards.len(), 1);
            let state = owned_state(&mut ship);
            for generator in state.generators.into_iter().flatten() {
                call_element("ships::races::samatra::generator_death", generator);
            }
            let gate = queued()
                .into_iter()
                .find(|h| {
                    (*element(*h)).collision_func.is_some_and(|hook| {
                        uqm_rust::ships::hook_registry::name(
                            &uqm_rust::ships::hook_registry::hooks(),
                            hook.address(),
                        ) == "ships::races::samatra::gate_collision"
                    })
                })
                .unwrap();
            call_element("ships::races::samatra::gate_preprocess", id(gate.cast()));
            let (events, snapshot) = observe();
            assert_eq!(events.destroyed, 255);
            let snapshot = snapshot.unwrap();
            assert_eq!(snapshot.remaining, 0);
            assert!(snapshot.generators.is_empty());
            assert!(snapshot.gate_open);
            let mut pa = POINT { x: 0, y: 0 };
            let mut pb = pa;
            call_collision(
                "ships::races::samatra::gate_collision",
                id(gate.cast()),
                &mut pa,
                player.id,
                &mut pb,
            );
            let (events, snapshot) = observe();
            assert!(events.gate_entered);
            assert_eq!(events.destroyed, 255);
            assert!(snapshot.is_none());
            set_game_state("BOMB_CARRIER", 0);
        }
    });
}

use uqm_rust::engine::battle::{race::SaMatraState, RaceState};

unsafe fn owned_state(ship: &mut Ship<'_>) -> SaMatraState {
    // SAFETY: the fixture retains the boss descriptor on the serialized game thread.
    let RaceState::SaMatra(state) = (unsafe { ship.descriptor_mut() }).race_state else {
        panic!("boss owns Sa-Matra state")
    };
    state
}

#[test]
fn lifecycle_counts_each_identity_once_and_retains_gate_entry() {
    battle_context::run(|| {
        let battle = Battle::new(64);
        // SAFETY: the fixture serializes the live element and ship pools.
        unsafe {
            uqm_rust::state::ffi::rust_init_game_state();
            let mut ship = boss(&battle);
            let mut other = Ship::with_frames(
                &battle,
                SPECIES_ID_SA_MATRA_ID,
                RPG_PLAYER_NUM as SIZE,
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
                    (*element(*h)).collision_func.is_some_and(|hook| {
                        uqm_rust::ships::hook_registry::name(
                            &uqm_rust::ships::hook_registry::hooks(),
                            hook.address(),
                        ) == "ships::races::samatra::gate_collision"
                    })
                })
                .unwrap_or_else(|| panic!("boss gate"));
            call_element("ships::races::samatra::gate_preprocess", id(gate.cast()));
            GlobData.Game_state.CurrentActivity = IN_BATTLE as u16;
            set_game_state("BOMB_CARRIER", 1);
            let mut pa = POINT { x: 0, y: 0 };
            let mut pb = pa;
            call_collision(
                "ships::races::samatra::gate_collision",
                id(gate.cast()),
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
            set_game_state("BOMB_CARRIER", 0);
        }
    });
}

#[test]
fn identity_does_not_depend_on_scrolling_or_survivor_order() {
    battle_context::run(|| {
        let battle = Battle::new(64);
        // SAFETY: every operand is an allocated pool slot retained by this fixture.
        unsafe {
            let mut ship = boss(&battle);
            let generators = owned_state(&mut ship).generators;
            for (index, generator) in generators.into_iter().enumerate().rev() {
                let generator = generator.unwrap_or_else(|| panic!("eight allocated generators"));
                uqm_rust::app::bridge::enter(|cx| {
                    cx.element_mut(generator).current.location = POINT { x: 17, y: 31 };
                    cx.unlink(generator);
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
    battle_context::run(|| {
        let battle = Battle::new(64);
        // SAFETY: the ship and sentinel remain allocated through the callback.
        unsafe {
            let mut ship = boss(&battle);
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            let sentinel = queued()
                .into_iter()
                .find(|h| (*element(*h)).__bindgen_anon_2.hit_points == 10)
                .unwrap_or_else(|| panic!("sentinel"));
            (*element(sentinel)).__bindgen_anon_4.turn_wait = 0;
            ship.ship_mut().hShip = std::ptr::null_mut();
            uqm_rust::app::bridge::enter(|cx| {
                assert_eq!(cx.optional_ship_element(ship.ship_id), None);
            });
            with_rng(5, 0, || {
                call_element(
                    "ships::races::samatra::sentinel_preprocess",
                    id(sentinel.cast()),
                )
            });
            assert_eq!((*element(sentinel)).__bindgen_anon_4.turn_wait, 1);
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
use uqm_rust::{
    ships::native::{abi::*, element, frame_index},
    state::game_state_keys::set_game_state,
};

unsafe fn boss(battle: &Battle) -> Ship<'_> {
    let mut ship = Ship::with_frames(
        battle,
        SPECIES_ID_SA_MATRA_ID,
        NPC_PLAYER_NUM as SIZE,
        [32; 3],
        [4, 4],
    );
    (*ship.ptr()).state_flags |= APPEARING as u16;
    with_rng(7, 8, || ship.preprocess());
    ship
}
#[test]
fn generators_spawn_at_exact_offsets_and_decrement_once_before_gate_opens() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = boss(&_battle);
            assert_eq!(ship.descriptor_mut().characteristics.max_thrust, 8);
            assert_eq!(
                (
                    ship.ship_mut().weapon_counter,
                    ship.ship_mut().special_counter
                ),
                (120, 36)
            );
            let all = queued();
            assert_eq!(all.len(), 11);
            let generators: Vec<_> = all
                .iter()
                .copied()
                .filter(|h| (*element(*h)).death_func.is_some())
                .collect();
            let mut positions: Vec<_> = generators
                .iter()
                .map(|h| {
                    let e = element(*h);
                    assert_eq!((*e).__bindgen_anon_2.hit_points, 15);
                    ((*e).current.location.x, (*e).current.location.y)
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
            let gate = element(
                *all.iter()
                    .find(|h| (*element(**h)).state_flags & FINITE_LIFE as u16 != 0)
                    .unwrap(),
            );
            for (i, h) in generators.iter().enumerate() {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*gate).preprocess_func.unwrap(),
                    gate,
                );
                assert_eq!((*prim(gate)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
                let g = element(*h);
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*g).death_func.unwrap(),
                    g,
                );
                assert_eq!(
                    ship.descriptor_mut().characteristics.max_thrust,
                    7 - i as u16
                );
                assert_eq!(frame_index((*g).current.image.frame), 9);
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*g).death_func.unwrap(),
                    g,
                );
                assert_eq!(
                    ship.descriptor_mut().characteristics.max_thrust,
                    7 - i as u16
                );
            }
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*gate).preprocess_func.unwrap(),
                gate,
            );
            assert!((*gate).preprocess_func.is_none());
            assert_eq!((*gate).state_flags & FINITE_LIFE as u16, 0);
            assert_eq!(
                (*prim(gate)).Type,
                gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM
            );
            assert_eq!((*gate).__bindgen_anon_1.life_span, 2);
        }
    })
}
#[test]
fn only_player_ship_bomb_carrier_through_open_gate_finishes_battle() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            uqm_rust::state::ffi::rust_init_game_state();
            let mut ship = boss(&_battle);
            let gate = element(
                *queued()
                    .iter()
                    .find(|h| (*element(**h)).state_flags & FINITE_LIFE as u16 != 0)
                    .unwrap(),
            );
            let player = Ship::with_frames(
                &_battle,
                SPECIES_ID_SA_MATRA_ID,
                RPG_PLAYER_NUM as SIZE,
                [32; 3],
                [4, 4],
            );
            let mut point = POINT { x: 0, y: 0 };
            let mut other_point = point;
            let collide = (*gate).collision_func.unwrap();
            for generators in [8, 1, 0] {
                ship.descriptor_mut().characteristics.max_thrust = generators;
                if generators == 0 {
                    uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                        (*gate).preprocess_func.unwrap(),
                        gate,
                    );
                }
                for bomb in [0, 1] {
                    for is_ship in [false, true] {
                        for owner in [RPG_PLAYER_NUM, NPC_PLAYER_NUM] {
                            GlobData.Game_state.CurrentActivity = IN_BATTLE as u16;
                            set_game_state("BOMB_CARRIER", bomb);
                            (*player.ptr()).playerNr = owner as SIZE;
                            (*player.ptr()).state_flags = if is_ship {
                                PLAYER_SHIP as u16
                            } else {
                                FINITE_LIFE as u16
                            };
                            uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                            collide,
                            gate,
                            &mut point,
                            player.ptr(),
                            &mut other_point,
                         uqm_rust::battle::process_native::invoke_collision::CollisionCall::Hook);
                            assert_eq!(
                                GlobData.Game_state.CurrentActivity & IN_BATTLE as u16 == 0,
                                generators == 0 && bomb == 1 && is_ship && owner == RPG_PLAYER_NUM
                            );
                        }
                    }
                }
            }
            set_game_state("BOMB_CARRIER", 0);
        }
    })
}
#[test]
fn postprocess_spawns_comets_and_sentinels_only_while_generators_live() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = boss(&_battle);
            ship.ship_mut().weapon_counter = 0;
            ship.ship_mut().special_counter = 0;
            with_rng(5, 1, || ship.postprocess());
            assert_eq!(queued().len(), 13);
            assert_eq!(
                (
                    ship.ship_mut().weapon_counter,
                    ship.ship_mut().special_counter
                ),
                (240, 72)
            );
            let comet = element(
                *queued()
                    .iter()
                    .find(|h| frame_index((*element(**h)).current.image.frame) == 24)
                    .unwrap(),
            );
            (*comet).next = (*comet).current;
            with_rng(5, 0, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*comet).preprocess_func.unwrap(),
                    comet,
                )
            });
            assert_eq!(ship.descriptor_mut().characteristics.weapon_wait, 1);
            assert_eq!(queued().len(), 14);
            assert_ne!((*comet).state_flags & NONSOLID as u16, 0);
            let sentinel = element(
                *queued()
                    .iter()
                    .find(|h| (*element(**h)).__bindgen_anon_2.hit_points == 10)
                    .unwrap(),
            );
            (*sentinel).next = (*sentinel).current;
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*sentinel).preprocess_func.unwrap(),
                sentinel,
            );
            assert_eq!(ship.descriptor_mut().characteristics.special_wait, 1);
            assert_eq!((*sentinel).__bindgen_anon_1.life_span, 3);
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
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = boss(&_battle);
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            let sentinel = element(
                *queued()
                    .iter()
                    .find(|h| (*element(**h)).__bindgen_anon_2.hit_points == 10)
                    .unwrap(),
            );
            let mut player = Ship::with_frames(
                &_battle,
                SPECIES_ID_SA_MATRA_ID,
                RPG_PLAYER_NUM as SIZE,
                [32; 3],
                [4, 4],
            );
            player.descriptor_mut().ship_info.crew_level = 20;
            (*player.ptr()).__bindgen_anon_2.crew_level = 20;
            (*player.ptr()).__bindgen_anon_3.mass_points = 10;
            player.ship_mut().cur_status_flags = (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
            SetVelocityComponents(&raw mut (*player.ptr()).velocity, 5000, 0);
            SetVelocityVector(&raw mut (*sentinel).velocity, 32, 4);
            let mut p = POINT { x: 0, y: 0 };
            let mut q = p;
            uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                (*sentinel).collision_func.unwrap(),
                sentinel,
                &mut p,
                player.ptr(),
                &mut q,
                uqm_rust::battle::process_native::invoke_collision::CollisionCall::Hook,
            );
            assert_eq!((*sentinel).__bindgen_anon_2.hit_points, 10);
            assert_eq!((*sentinel).__bindgen_anon_1.life_span, 2);
            assert_eq!((*sentinel).__bindgen_anon_4.turn_wait, 12);
            assert_ne!((*sentinel).state_flags & DEFY_PHYSICS as u16, 0);
            assert_eq!((*sentinel).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(
                player.ship_mut().cur_status_flags
                    & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16,
                0
            );
            assert_eq!((*player.ptr()).__bindgen_anon_4.turn_wait, 1);
            assert_eq!((*player.ptr()).__bindgen_anon_5.thrust_wait, 3);
            let mut dx = 0;
            let mut dy = 0;
            GetCurrentVelocityComponents(&raw mut (*player.ptr()).velocity, &mut dx, &mut dy);
            assert_eq!((dx, dy), (5120, 0));
            assert_eq!(velocity(&(*sentinel).velocity), [0; 9]);
        }
    })
}
