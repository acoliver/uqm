#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use std::mem;
use uqm_rust::{
    ships::native::abi::*,
    state::game_state_keys::{get_game_state, set_game_state},
};
unsafe fn modules() {
    crate::battle_context::with(|cx| uqm_rust::state::ffi::rust_init_game_state(cx.world));
    crate::battle_context::with(|cx| cx.world.data.flagship.module_slots = [255; 16]);
    crate::battle_context::with(|cx| cx.world.data.flagship.drive_slots = [255; 11]);
    crate::battle_context::with(|cx| cx.world.data.flagship.jet_slots = [255; 8]);
    crate::battle_context::with(|cx| cx.world.data.flagship.crew_enlisted = 9);
    crate::battle_context::with(|cx| cx.world.set_activity(IN_ENCOUNTER as u16));
    crate::battle_context::with(|cx| {
        set_game_state(cx.world, uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, 0)
    });
}
/// Guns fore, aft and to both sides, three trackers, and two antimissile modules.
fn full_loadout() {
    for (i, m) in [
        (15, 9),
        (14, 10),
        (13, 11),
        (0, 9),
        (1, 3),
        (2, 3),
        (3, 12),
        (4, 12),
        (5, 12),
        (6, 12),
        (7, 13),
        (8, 13),
        (9, 8),
        (10, 7),
        (11, 7),
        (12, 7),
    ] {
        crate::battle_context::with(|cx| cx.world.data.flagship.module_slots[i] = m);
    }
}
fn target(origin: *mut ElementRecord, dx: i16, dy: i16) -> PoolElement {
    // SAFETY: the caller's serialized battle owns `origin` and the pool slot allocated here.
    unsafe {
        let mut fixture = PoolElement::new();
        let e = &raw mut *fixture;
        (*e).playerNr = 1;
        (*e).hits = 1;
        (*e).mass = 1;
        (*e).next.location = POINT {
            x: (*origin).next.location.x.wrapping_add(dx << ONE_SHIFT),
            y: (*origin).next.location.y.wrapping_add(dy << ONE_SHIFT),
        };
        (*prim(e)).kind = gfx_object_STAMP_PRIM as u8;
        battle::put(e.cast());
        fixture
    }
}
#[test]
fn module_configuration_preserves_weapon_order_energy_caps_and_drive_rounding() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            modules();
            full_loadout();
            crate::battle_context::with(|cx| cx.world.data.flagship.drive_slots = [1; 11]);
            crate::battle_context::with(|cx| cx.world.data.flagship.jet_slots = [2; 8]);
            let mut ship = Ship::with_frames(
                &_battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as SIZE,
                [16, 32, 16],
                [4, 4],
            );
            assert_eq!(
                (
                    ship.descriptor_mut().ship_info.max_crew,
                    ship.descriptor_mut().ship_info.crew_level
                ),
                (101, 10)
            );
            let c = ship.descriptor_mut().characteristics;
            assert_eq!((c.max_thrust, c.thrust_wait, c.turn_wait), (36, 1, 1));
            assert_eq!(
                (
                    c.energy_regeneration,
                    c.energy_wait,
                    c.special_energy_cost,
                    c.weapon_energy_cost
                ),
                (2, 4, 2, 24)
            );
            let out = ship.weapon_handles(6);
            for (h, damage) in out.iter().zip([2, 4, 4, 6, 6, 2]) {
                assert_eq!((*battle::element(*h)).mass, damage);
                assert_eq!((*battle::element(*h)).turn, 17);
                battle_context::dispose(*h);
            }
            ship.descriptor_mut().ship_info.crew_level = 4;
            ship.uninit();
            assert_eq!(
                { crate::battle_context::with(|cx| cx.world.data.flagship.crew_enlisted) },
                3
            );
        }
    })
}
#[test]
fn no_weapons_disable_inputs_and_bomb_state_marks_carrier() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        unsafe {
            modules();
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, 3)
            });
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::BOMB_CARRIER, 0)
            });
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_SIS_SHIP_ID, 0, [16, 32, 16], [4, 4]);
            assert_eq!(
                crate::battle_context::with(|cx| get_game_state(
                    cx.world,
                    uqm_rust::engine::StoryKey::BOMB_CARRIER
                )),
                1
            );
            ship.ship_mut().cur_status_flags = (SPECIAL | WEAPON) as u16;
            ship.preprocess();
            assert_eq!(
                ship.ship_mut().cur_status_flags & (SPECIAL | WEAPON) as u16,
                0
            );
            assert_eq!(
                (
                    ship.ship_mut().weapon_counter,
                    ship.ship_mut().special_counter
                ),
                (2, 2)
            );
            ship.uninit();
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::BOMB_CARRIER, 0)
            });
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, 0)
            });
        }
    })
}
#[test]
fn hyperspace_fuel_brakes_and_quasispace_allows_thrust_without_fuel() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        unsafe {
            modules();
            crate::battle_context::with(|cx| cx.world.set_activity(IN_HYPERSPACE as u16));
            crate::battle_context::with(|cx| cx.world.data.flagship.fuel_on_board = 0);
            crate::battle_context::with(|cx| cx.world.data.game.autopilot = POINT { x: -1, y: -1 });
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_SIS_SHIP_ID, 0, [16, 32, 16], [4, 4]);
            assert!(ship.descriptor_mut().callbacks()[3] == 0);
            assert_eq!(ship.descriptor_mut().characteristics.max_thrust, 8);
            assert!(ship.descriptor_mut().ship_info.icons_rsc.is_null());
            ((*ship.ptr()).velocity).set_components(256, 0);
            ship.ship_mut().cur_status_flags = THRUST as u16;
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 0)
            });
            ship.preprocess();
            let (velocity_x, velocity_y) = ((*ship.ptr()).velocity).get_current_components();
            let (x, y) = (velocity_x as i16, velocity_y as i16);
            assert_eq!((x, y), (128, 0));
            assert_eq!(ship.ship_mut().cur_status_flags & THRUST as u16, 0);
            assert_eq!(ship.ship_mut().weapon_counter, 1);
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 2)
            });
            ship.ship_mut().cur_status_flags = THRUST as u16;
            (*ship.ptr()).thrust = 0;
            ship.preprocess();
            assert_ne!(ship.ship_mut().cur_status_flags & THRUST as u16, 0);
            ship.postprocess();
            assert_eq!(
                velocity(&crate::battle_context::with(|cx| cx
                    .world
                    .data
                    .game
                    .velocity)),
                velocity(&(*ship.ptr()).velocity)
            );
            ship.uninit();
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 0)
            });
        }
    })
}
#[test]
fn autopilot_accelerates_toward_target_and_cancels_on_manual_turn() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        unsafe {
            modules();
            crate::battle_context::with(|cx| cx.world.set_activity(IN_HYPERSPACE as u16));
            crate::battle_context::with(|cx| cx.world.data.flagship.fuel_on_board = 100);
            crate::battle_context::with(|cx| cx.world.data.flagship.log_x = 0);
            crate::battle_context::with(|cx| cx.world.data.flagship.log_y = 0);
            crate::battle_context::with(|cx| {
                cx.world.data.game.autopilot = POINT { x: 100, y: 9999 }
            });
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 0)
            });
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_SIS_SHIP_ID, 0, [16, 32, 16], [4, 4]);
            ship.ship_mut().ship_facing = 4;
            ship.preprocess();
            let (velocity_x, velocity_y) = ((*ship.ptr()).velocity).get_current_components();
            let (x, y) = (velocity_x as i16, velocity_y as i16);
            assert_eq!((x, y), (128, 0));
            assert_ne!(
                ship.ship_mut().cur_status_flags & SHIP_AT_MAX_SPEED as u16,
                0
            );
            ship.ship_mut().cur_status_flags = LEFT as u16;
            ship.preprocess();
            assert_eq!(
                crate::battle_context::with(|cx| (
                    cx.world.data.game.autopilot.x,
                    cx.world.data.game.autopilot.y
                )),
                (-1, -1)
            );
            ship.uninit();
        }
    })
}

#[test]
fn point_defense_zaps_each_nearby_target_and_pays_energy_once() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        // SAFETY: the serialized battle owns every element and ship used below.
        unsafe {
            modules();
            full_loadout();
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as SIZE,
                [16, 32, 16],
                [4, 4],
            );
            assert_ne!(
                ship.descriptor_mut().ship_info.ship_flags & POINT_DEFENSE as u16,
                0
            );
            ship.element_mut().next = ship.element_mut().current.clone();
            let origin = ship.ptr();
            let near = target(origin, 30, 40);
            let edge = target(origin, -100, 0);
            let beyond = target(origin, 101, 0);
            let mut cloaked = target(origin, 10, 0);
            (*prim(&raw mut *cloaked)).kind = gfx_object_NUM_PRIMS as u8;
            let mut solid_not = target(origin, 0, 10);
            solid_not.state_flags = NONSOLID as u16;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            let defense = battle::element(battle::tail());
            assert_ne!(defense, origin);
            let energy = ship.descriptor_mut().ship_info.energy_level;
            let before = queued().len();
            call_element("ships::races::sis_native::spawn_point_defense", id(defense));
            assert_eq!(queued().len(), before + 2);
            assert_eq!(
                ship.descriptor_mut().ship_info.energy_level,
                energy - (2 << 2)
            );
            let special_wait = ship.descriptor_mut().characteristics.special_wait;
            assert_eq!(ship.ship_mut().special_counter, special_wait);
            for beam in &queued()[before..] {
                let beam = battle::element(*beam);
                assert_eq!((*beam).mass, 2);
                assert_eq!((*beam).parent, (*origin).parent);
            }
            drop((near, edge, beyond, cloaked, solid_not));
        }
    })
}

#[test]
fn point_defense_fires_nothing_when_the_flagship_cannot_pay() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        // SAFETY: the serialized battle owns every element and ship used below.
        unsafe {
            modules();
            full_loadout();
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as SIZE,
                [16, 32, 16],
                [4, 4],
            );
            ship.element_mut().next = ship.element_mut().current.clone();
            let origin = ship.ptr();
            let near = target(origin, 30, 40);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            let defense = battle::element(battle::tail());
            ship.descriptor_mut().ship_info.energy_level = 4;
            let before = queued().len();
            call_element("ships::races::sis_native::spawn_point_defense", id(defense));
            assert_eq!(queued().len(), before);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 4);
            assert_eq!(ship.ship_mut().special_counter, 0);
            drop(near);
        }
    })
}

#[test]
fn blasters_animate_retarget_and_spend_themselves_on_impact() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        // SAFETY: the serialized battle owns every element and ship used below.
        unsafe {
            modules();
            full_loadout();
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as SIZE,
                [16, 32, 16],
                [4, 4],
            );
            let enemy = Ship::with_frames(&battle, SPECIES_ID_SIS_SHIP_ID, 1, [16, 32, 16], [4, 4]);
            let blasters = ship.weapon_handles(6);
            for (handle, mass) in blasters.iter().zip([2u8, 4, 4, 6, 6, 2]) {
                let element = battle::element(*handle);
                assert_eq!((*element).mass, mass);
                (*element).hits = 3;
                call_element("ships::races::sis_native::blaster_preprocess", id(element));
                assert_eq!((*element).turn, 16, "countdown to the next retarget");
                call_element("ships::races::sis_native::blaster_preprocess", id(element));
                assert_eq!((*element).turn, 17, "retarget restarts the countdown");
                let mut at = POINT { x: 10, y: 10 };
                let mut on = POINT { x: 10, y: 10 };
                call_collision(
                    "ships::races::sis_native::blaster_collision",
                    id(element),
                    &mut at,
                    enemy.id,
                    &mut on,
                );
                assert_ne!(
                    (*element).state_flags & COLLISION as u16,
                    0,
                    "a blaster is consumed by what it hits"
                );
                let blast = battle::element(battle::tail());
                assert_eq!(
                    (*blast).life,
                    match mass {
                        2 => 2,
                        4 => 6,
                        _ => 7,
                    }
                );
            }
            for handle in blasters {
                battle_context::dispose(handle);
            }
        }
    })
}

unsafe fn think(
    ship: &mut Ship<'_>,
    threat: *mut ElementRecord,
    enemy: *mut ElementRecord,
    which_turn: (u16, u16),
    movement: u32,
) -> [EVALUATE_DESC; 4] {
    // SAFETY: the callers keep `threat`, `enemy` and the ship in the serialized battle.
    unsafe {
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr = threat;
        concerns[ENEMY_WEAPON_INDEX as usize].which_turn = which_turn.0;
        concerns[ENEMY_WEAPON_INDEX as usize].MoveState = movement;
        concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy;
        concerns[ENEMY_SHIP_INDEX as usize].which_turn = which_turn.1;
        ship.intelligence(&mut *ship.ptr(), &mut concerns, 0);
        concerns
    }
}

#[test]
fn ai_with_point_defense_fires_it_only_for_imminent_threats() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        // SAFETY: the serialized battle owns every element and ship used below.
        unsafe {
            modules();
            full_loadout();
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as SIZE,
                [16, 32, 16],
                [4, 4],
            );
            let enemy = Ship::with_frames(&battle, SPECIES_ID_SIS_SHIP_ID, 1, [16, 32, 16], [4, 4]);
            let mut missile = PoolElement::new();
            let threat = &raw mut *missile;
            for (counter, soon, enemy_turn, expected) in [
                (0, 2, 99, true),
                (0, 3, 4, true),
                (0, 3, 5, false),
                (1, 1, 1, false),
            ] {
                ship.ship_mut().special_counter = counter;
                ship.ship_mut().ship_input_state = SPECIAL as u8;
                let concerns = think(&mut ship, threat, enemy.ptr(), (soon, enemy_turn), 0);
                assert_eq!(
                    ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                    expected,
                    "{counter} {soon} {enemy_turn}"
                );
                assert!(concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr.is_null());
            }
        }
    })
}

#[test]
fn ai_without_point_defense_pursues_weapons_that_entice_it() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        // SAFETY: the serialized battle owns every element and ship used below.
        unsafe {
            modules();
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as SIZE,
                [16, 32, 16],
                [4, 4],
            );
            let enemy = Ship::with_frames(&battle, SPECIES_ID_SIS_SHIP_ID, 1, [16, 32, 16], [4, 4]);
            let mut missile = PoolElement::new();
            let threat = &raw mut *missile;
            missile.state_flags = 0;
            let concerns = think(
                &mut ship,
                threat,
                enemy.ptr(),
                (9, 99),
                MOVEMENT_STATE_ENTICE,
            );
            assert_eq!(
                concerns[ENEMY_WEAPON_INDEX as usize].MoveState,
                MOVEMENT_STATE_PURSUE
            );
            missile.state_flags = FINITE_LIFE as u16;
            missile.mass = 3;
            let concerns = think(
                &mut ship,
                threat,
                enemy.ptr(),
                (2, 99),
                MOVEMENT_STATE_ENTICE,
            );
            assert_eq!(
                concerns[ENEMY_WEAPON_INDEX as usize].MoveState,
                MOVEMENT_STATE_ENTICE
            );
            missile.mass = 4;
            let concerns = think(
                &mut ship,
                threat,
                enemy.ptr(),
                (2, 99),
                MOVEMENT_STATE_ENTICE,
            );
            assert_eq!(
                concerns[ENEMY_WEAPON_INDEX as usize].MoveState,
                MOVEMENT_STATE_PURSUE
            );
        }
    })
}

#[test]
fn ai_with_trackers_fires_blasters_that_bear_on_the_enemy() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        // SAFETY: the serialized battle owns every element and ship used below.
        unsafe {
            modules();
            for (slot, module) in [(15, 9), (3, 12)] {
                crate::battle_context::with(|cx| {
                    cx.world.data.flagship.module_slots[slot] = module
                });
            }
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as SIZE,
                [16, 32, 16],
                [4, 4],
            );
            let mut enemy =
                Ship::with_frames(&battle, SPECIES_ID_SIS_SHIP_ID, 1, [16, 32, 16], [4, 4]);
            ship.element_mut().current.location = POINT { x: 4000, y: 4000 };
            for (enemy_y, expected) in [(3000, true), (5000, false)] {
                enemy.element_mut().current.location = POINT {
                    x: 4000,
                    y: enemy_y,
                };
                ship.ship_mut().weapon_counter = 0;
                ship.ship_mut().ship_input_state = 0;
                think(&mut ship, std::ptr::null_mut(), enemy.ptr(), (0, 10), 0);
                assert_eq!(
                    ship.ship_mut().ship_input_state & WEAPON as u8 != 0,
                    expected,
                    "enemy at y={enemy_y}"
                );
            }
            ship.ship_mut().weapon_counter = 3;
            ship.ship_mut().ship_input_state = 0;
            enemy.element_mut().current.location = POINT { x: 4000, y: 3000 };
            think(&mut ship, std::ptr::null_mut(), enemy.ptr(), (0, 10), 0);
            assert_eq!(ship.ship_mut().ship_input_state & WEAPON as u8, 0);
        }
    })
}
