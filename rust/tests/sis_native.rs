#![forbid(unsafe_code)]
#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use uqm_rust::engine::battle::ai_constants::{
    ENEMY_SHIP_INDEX, ENEMY_WEAPON_INDEX, LEFT, MOVEMENT_STATE_ENTICE, MOVEMENT_STATE_PURSUE,
    POINT_DEFENSE, SPECIAL, THRUST, WEAPON,
};
use uqm_rust::engine::battle::flags::{
    COLLISION, FINITE_LIFE, NONSOLID, RPG_PLAYER_NUM, SHIP_AT_MAX_SPEED,
};
use uqm_rust::engine::battle::Concern;
use uqm_rust::engine::battle::ElementId;
use uqm_rust::engine::battle::{EMPTY_PRIMITIVE, STAMP_PRIMITIVE};
use uqm_rust::engine::species::species_id::SPECIES_ID_SIS_SHIP_ID;
use uqm_rust::engine::world::activity::activity_kinds::IN_ENCOUNTER;
use uqm_rust::engine::world::IN_HYPERSPACE;
use uqm_rust::graphics::geometry::Point;
use uqm_rust::math::battle_types::ONE_SHIFT;
use uqm_rust::state::game_state_keys::{get_game_state, set_game_state};
fn modules() {
    crate::battle_context::with(|cx| uqm_rust::state::ffi::rust_init_game_state(cx.world));
    crate::battle_context::with(|cx| cx.world.data.flagship.module_slots = [255; 16]);
    crate::battle_context::with(|cx| cx.world.data.flagship.drive_slots = [255; 11]);
    crate::battle_context::with(|cx| cx.world.data.flagship.jet_slots = [255; 8]);
    crate::battle_context::with(|cx| cx.world.data.flagship.crew_enlisted = 9);
    crate::battle_context::with(|cx| cx.world.set_activity(IN_ENCOUNTER));
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
fn target(origin: ElementId, dx: i16, dy: i16) -> PoolElement {
    {
        let fixture = PoolElement::new();
        let e = fixture.id;
        battle::get_mut(e).player_nr = 1;
        battle::get_mut(e).hits = 1;
        battle::get_mut(e).mass = 1;
        battle::get_mut(e).next.location = Point {
            x: battle::get(origin)
                .next
                .location
                .x
                .wrapping_add(dx << ONE_SHIFT),
            y: battle::get(origin)
                .next
                .location
                .y
                .wrapping_add(dy << ONE_SHIFT),
        };
        battle::prim_mut(e).kind = STAMP_PRIMITIVE;
        battle::put(e);
        fixture
    }
}
#[test]
fn module_configuration_preserves_weapon_order_energy_caps_and_drive_rounding() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        {
            modules();
            full_loadout();
            crate::battle_context::with(|cx| cx.world.data.flagship.drive_slots = [1; 11]);
            crate::battle_context::with(|cx| cx.world.data.flagship.jet_slots = [2; 8]);
            let mut ship = Ship::with_frames(
                &_battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as i16,
                [16, 32, 16],
                [4, 4],
            );
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (
                        descriptor.ship_info.max_crew,
                        descriptor.ship_info.crew_level
                    ),
                    (101, 10)
                );
            }
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
                assert_eq!(battle::get(h.unwrap()).mass, damage);
                assert_eq!(battle::get(h.unwrap()).turn, 17);
                battle_context::dispose(h.unwrap());
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
        {
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
            {
                let record = ship.ship_mut();
                assert_eq!((record.weapon_counter, record.special_counter), (2, 2));
            }
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
        {
            modules();
            crate::battle_context::with(|cx| cx.world.set_activity(IN_HYPERSPACE as u16));
            crate::battle_context::with(|cx| cx.world.data.flagship.fuel_on_board = 0);
            crate::battle_context::with(|cx| cx.world.data.game.autopilot = Point { x: -1, y: -1 });
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_SIS_SHIP_ID, 0, [16, 32, 16], [4, 4]);
            assert!(ship.descriptor_mut().callbacks()[3] == 0);
            assert_eq!(ship.descriptor_mut().characteristics.max_thrust, 8);
            assert!(ship.descriptor_mut().ship_info.icons_rsc.is_none());
            (battle::get_mut(ship.id).velocity).set_components(256, 0);
            ship.ship_mut().cur_status_flags = THRUST as u16;
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 0)
            });
            ship.preprocess();
            let (velocity_x, velocity_y) = (battle::get(ship.id).velocity).get_current_components();
            let (x, y) = (velocity_x as i16, velocity_y as i16);
            assert_eq!((x, y), (128, 0));
            assert_eq!(ship.ship_mut().cur_status_flags & THRUST as u16, 0);
            assert_eq!(ship.ship_mut().weapon_counter, 1);
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 2)
            });
            ship.ship_mut().cur_status_flags = THRUST as u16;
            battle::get_mut(ship.id).thrust = 0;
            ship.preprocess();
            assert_ne!(ship.ship_mut().cur_status_flags & THRUST as u16, 0);
            ship.postprocess();
            assert_eq!(
                velocity(&crate::battle_context::with(|cx| cx
                    .world
                    .data
                    .game
                    .velocity)),
                velocity(&battle::get(ship.id).velocity)
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
        {
            modules();
            crate::battle_context::with(|cx| cx.world.set_activity(IN_HYPERSPACE as u16));
            crate::battle_context::with(|cx| cx.world.data.flagship.fuel_on_board = 100);
            crate::battle_context::with(|cx| cx.world.data.flagship.log_x = 0);
            crate::battle_context::with(|cx| cx.world.data.flagship.log_y = 0);
            crate::battle_context::with(|cx| {
                cx.world.data.game.autopilot = Point { x: 100, y: 9999 }
            });
            crate::battle_context::with(|cx| {
                set_game_state(cx.world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 0)
            });
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_SIS_SHIP_ID, 0, [16, 32, 16], [4, 4]);
            ship.ship_mut().ship_facing = 4;
            ship.preprocess();
            let (velocity_x, velocity_y) = (battle::get(ship.id).velocity).get_current_components();
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
        {
            modules();
            full_loadout();
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as i16,
                [16, 32, 16],
                [4, 4],
            );
            assert_ne!(
                ship.descriptor_mut().ship_info.ship_flags & POINT_DEFENSE as u16,
                0
            );
            {
                let mut element = ship.element_mut();
                element.next = element.current.clone();
            }
            let origin = ship.id;
            let near = target(origin, 30, 40);
            let edge = target(origin, -100, 0);
            let beyond = target(origin, 101, 0);
            let cloaked = target(origin, 10, 0);
            battle::prim_mut(cloaked.id).kind = EMPTY_PRIMITIVE;
            let solid_not = target(origin, 0, 10);
            battle::get_mut(solid_not.id).state_flags = NONSOLID as u16;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            let defense = battle::tail().unwrap();
            assert_ne!(defense, origin);
            let energy = ship.descriptor_mut().ship_info.energy_level;
            let before = queued().len();
            call_element("ships::races::sis_native::spawn_point_defense", defense);
            assert_eq!(queued().len(), before + 2);
            assert_eq!(
                ship.descriptor_mut().ship_info.energy_level,
                energy - (2 << 2)
            );
            let special_wait = ship.descriptor_mut().characteristics.special_wait;
            assert_eq!(ship.ship_mut().special_counter, special_wait);
            for beam in &queued()[before..] {
                let beam = *beam;
                assert_eq!(battle::get(beam).mass, 2);
                assert_eq!(battle::get(beam).parent, battle::get(origin).parent);
            }
            drop((near, edge, beyond, cloaked, solid_not));
        }
    })
}

#[test]
fn point_defense_fires_nothing_when_the_flagship_cannot_pay() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        {
            modules();
            full_loadout();
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as i16,
                [16, 32, 16],
                [4, 4],
            );
            {
                let mut element = ship.element_mut();
                element.next = element.current.clone();
            }
            let origin = ship.id;
            let near = target(origin, 30, 40);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            let defense = battle::tail().unwrap();
            ship.descriptor_mut().ship_info.energy_level = 4;
            let before = queued().len();
            call_element("ships::races::sis_native::spawn_point_defense", defense);
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
        {
            modules();
            full_loadout();
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as i16,
                [16, 32, 16],
                [4, 4],
            );
            let enemy = Ship::with_frames(&battle, SPECIES_ID_SIS_SHIP_ID, 1, [16, 32, 16], [4, 4]);
            let blasters = ship.weapon_handles(6);
            for (handle, mass) in blasters.iter().zip([2u8, 4, 4, 6, 6, 2]) {
                let element = handle.unwrap();
                assert_eq!(battle::get(element).mass, mass);
                battle::get_mut(element).hits = 3;
                call_element("ships::races::sis_native::blaster_preprocess", element);
                assert_eq!(
                    battle::get(element).turn,
                    16,
                    "countdown to the next retarget"
                );
                call_element("ships::races::sis_native::blaster_preprocess", element);
                assert_eq!(
                    battle::get(element).turn,
                    17,
                    "retarget restarts the countdown"
                );
                let mut at = Point { x: 10, y: 10 };
                let mut on = Point { x: 10, y: 10 };
                call_collision(
                    "ships::races::sis_native::blaster_collision",
                    element,
                    &mut at,
                    enemy.id,
                    &mut on,
                );
                assert_ne!(
                    battle::get(element).state_flags & COLLISION as u16,
                    0,
                    "a blaster is consumed by what it hits"
                );
                let blast = battle::tail().unwrap();
                assert_eq!(
                    battle::get(blast).life,
                    match mass {
                        2 => 2,
                        4 => 6,
                        _ => 7,
                    }
                );
            }
            for handle in blasters {
                battle_context::dispose(handle.unwrap());
            }
        }
    })
}

fn think(
    ship: &mut Ship<'_>,
    threat: Option<ElementId>,
    enemy: ElementId,
    which_turn: (u16, u16),
    movement: u32,
) -> [Concern; 4] {
    {
        let mut concerns: [Concern; 4] = [Concern::default(); 4];
        concerns[ENEMY_WEAPON_INDEX as usize].object = threat;
        concerns[ENEMY_WEAPON_INDEX as usize].which_turn = which_turn.0;
        concerns[ENEMY_WEAPON_INDEX as usize].movement = movement;
        concerns[ENEMY_SHIP_INDEX as usize].object = Some(enemy);
        concerns[ENEMY_SHIP_INDEX as usize].which_turn = which_turn.1;
        ship.think(ship.id, &mut concerns, 0);
        concerns
    }
}

#[test]
fn ai_with_point_defense_fires_it_only_for_imminent_threats() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        {
            modules();
            full_loadout();
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as i16,
                [16, 32, 16],
                [4, 4],
            );
            let enemy = Ship::with_frames(&battle, SPECIES_ID_SIS_SHIP_ID, 1, [16, 32, 16], [4, 4]);
            let missile = PoolElement::new();
            let threat = missile.id;
            for (counter, soon, enemy_turn, expected) in [
                (0, 2, 99, true),
                (0, 3, 4, true),
                (0, 3, 5, false),
                (1, 1, 1, false),
            ] {
                ship.ship_mut().special_counter = counter;
                ship.ship_mut().ship_input_state = SPECIAL as u8;
                let concerns = think(&mut ship, Some(threat), enemy.id, (soon, enemy_turn), 0);
                assert_eq!(
                    ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                    expected,
                    "{counter} {soon} {enemy_turn}"
                );
                assert!(concerns[ENEMY_WEAPON_INDEX as usize].object.is_none());
            }
        }
    })
}

#[test]
fn ai_without_point_defense_pursues_weapons_that_entice_it() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        {
            modules();
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as i16,
                [16, 32, 16],
                [4, 4],
            );
            let enemy = Ship::with_frames(&battle, SPECIES_ID_SIS_SHIP_ID, 1, [16, 32, 16], [4, 4]);
            let missile = PoolElement::new();
            let threat = missile.id;
            battle::get_mut(missile.id).state_flags = 0;
            let concerns = think(
                &mut ship,
                Some(threat),
                enemy.id,
                (9, 99),
                MOVEMENT_STATE_ENTICE,
            );
            assert_eq!(
                concerns[ENEMY_WEAPON_INDEX as usize].movement,
                MOVEMENT_STATE_PURSUE
            );
            battle::get_mut(missile.id).state_flags = FINITE_LIFE as u16;
            battle::get_mut(missile.id).mass = 3;
            let concerns = think(
                &mut ship,
                Some(threat),
                enemy.id,
                (2, 99),
                MOVEMENT_STATE_ENTICE,
            );
            assert_eq!(
                concerns[ENEMY_WEAPON_INDEX as usize].movement,
                MOVEMENT_STATE_ENTICE
            );
            battle::get_mut(missile.id).mass = 4;
            let concerns = think(
                &mut ship,
                Some(threat),
                enemy.id,
                (2, 99),
                MOVEMENT_STATE_ENTICE,
            );
            assert_eq!(
                concerns[ENEMY_WEAPON_INDEX as usize].movement,
                MOVEMENT_STATE_PURSUE
            );
        }
    })
}

#[test]
fn ai_with_trackers_fires_blasters_that_bear_on_the_enemy() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        {
            modules();
            for (slot, module) in [(15, 9), (3, 12)] {
                crate::battle_context::with(|cx| {
                    cx.world.data.flagship.module_slots[slot] = module
                });
            }
            let mut ship = Ship::with_frames(
                &battle,
                SPECIES_ID_SIS_SHIP_ID,
                RPG_PLAYER_NUM as i16,
                [16, 32, 16],
                [4, 4],
            );
            let mut enemy =
                Ship::with_frames(&battle, SPECIES_ID_SIS_SHIP_ID, 1, [16, 32, 16], [4, 4]);
            ship.element_mut().current.location = Point { x: 4000, y: 4000 };
            for (enemy_y, expected) in [(3000, true), (5000, false)] {
                enemy.element_mut().current.location = Point {
                    x: 4000,
                    y: enemy_y,
                };
                ship.ship_mut().weapon_counter = 0;
                ship.ship_mut().ship_input_state = 0;
                think(&mut ship, None, enemy.id, (0, 10), 0);
                assert_eq!(
                    ship.ship_mut().ship_input_state & WEAPON as u8 != 0,
                    expected,
                    "enemy at y={enemy_y}"
                );
            }
            ship.ship_mut().weapon_counter = 3;
            ship.ship_mut().ship_input_state = 0;
            enemy.element_mut().current.location = Point { x: 4000, y: 3000 };
            think(&mut ship, None, enemy.id, (0, 10), 0);
            assert_eq!(ship.ship_mut().ship_input_state & WEAPON as u8, 0);
        }
    })
}
