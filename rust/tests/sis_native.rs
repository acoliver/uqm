#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use uqm_rust::{
    ships::native::abi::*,
    state::game_state_keys::{get_game_state, set_game_state},
};
unsafe fn modules() {
    uqm_rust::engine::World::current().write(uqm_rust::state::ffi::rust_init_game_state);
    (**uqm_rust::engine::WorldData::current())
        .flagship
        .module_slots = [255; 16];
    (**uqm_rust::engine::WorldData::current())
        .flagship
        .drive_slots = [255; 11];
    (**uqm_rust::engine::WorldData::current())
        .flagship
        .jet_slots = [255; 8];
    (**uqm_rust::engine::WorldData::current())
        .flagship
        .crew_enlisted = 9;
    (**uqm_rust::engine::WorldData::current())
        .game
        .current_activity = IN_ENCOUNTER as u16;
    uqm_rust::engine::World::current()
        .write(|world| set_game_state(world, uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, 0));
}
#[test]
fn module_configuration_preserves_weapon_order_energy_caps_and_drive_rounding() {
    battle_context::run(|| {
        let _battle = Battle::new(32);
        unsafe {
            modules();
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
                (**uqm_rust::engine::WorldData::current())
                    .flagship
                    .module_slots[i] = m;
            }
            (**uqm_rust::engine::WorldData::current())
                .flagship
                .drive_slots = [1; 11];
            (**uqm_rust::engine::WorldData::current())
                .flagship
                .jet_slots = [2; 8];
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
                {
                    (**uqm_rust::engine::WorldData::current())
                        .flagship
                        .crew_enlisted
                },
                3
            );
        }
    })
}
#[test]
fn no_weapons_disable_inputs_and_bomb_state_marks_carrier() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            modules();
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(world, uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, 3)
            });
            uqm_rust::engine::World::current()
                .write(|world| set_game_state(world, uqm_rust::engine::StoryKey::BOMB_CARRIER, 0));
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_SIS_SHIP_ID, 0, [16, 32, 16], [4, 4]);
            assert_eq!(
                uqm_rust::engine::World::current()
                    .read(|world| get_game_state(world, uqm_rust::engine::StoryKey::BOMB_CARRIER)),
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
            uqm_rust::engine::World::current()
                .write(|world| set_game_state(world, uqm_rust::engine::StoryKey::BOMB_CARRIER, 0));
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(world, uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, 0)
            });
        }
    })
}
#[test]
fn hyperspace_fuel_brakes_and_quasispace_allows_thrust_without_fuel() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            modules();
            (**uqm_rust::engine::WorldData::current())
                .game
                .current_activity = IN_HYPERSPACE as u16;
            (**uqm_rust::engine::WorldData::current())
                .flagship
                .fuel_on_board = 0;
            (**uqm_rust::engine::WorldData::current()).game.autopilot = POINT { x: -1, y: -1 };
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_SIS_SHIP_ID, 0, [16, 32, 16], [4, 4]);
            assert!(ship.descriptor_mut().callbacks()[3] == 0);
            assert_eq!(ship.descriptor_mut().characteristics.max_thrust, 8);
            assert!(ship.descriptor_mut().ship_info.icons_rsc.is_null());
            ((*ship.ptr()).velocity).set_components(256, 0);
            ship.ship_mut().cur_status_flags = THRUST as u16;
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 0)
            });
            ship.preprocess();
            let (velocity_x, velocity_y) = ((*ship.ptr()).velocity).get_current_components();
            let (x, y) = (velocity_x as i16, velocity_y as i16);
            assert_eq!((x, y), (128, 0));
            assert_eq!(ship.ship_mut().cur_status_flags & THRUST as u16, 0);
            assert_eq!(ship.ship_mut().weapon_counter, 1);
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 2)
            });
            ship.ship_mut().cur_status_flags = THRUST as u16;
            (*ship.ptr()).thrust = 0;
            ship.preprocess();
            assert_ne!(ship.ship_mut().cur_status_flags & THRUST as u16, 0);
            ship.postprocess();
            assert_eq!(
                velocity(&{ (**uqm_rust::engine::WorldData::current()).game.velocity }),
                velocity(&(*ship.ptr()).velocity)
            );
            ship.uninit();
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 0)
            });
        }
    })
}
#[test]
fn autopilot_accelerates_toward_target_and_cancels_on_manual_turn() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            modules();
            (**uqm_rust::engine::WorldData::current())
                .game
                .current_activity = IN_HYPERSPACE as u16;
            (**uqm_rust::engine::WorldData::current())
                .flagship
                .fuel_on_board = 100;
            (**uqm_rust::engine::WorldData::current()).flagship.log_x = 0;
            (**uqm_rust::engine::WorldData::current()).flagship.log_y = 0;
            (**uqm_rust::engine::WorldData::current()).game.autopilot = POINT { x: 100, y: 9999 };
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(world, uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 0)
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
                uqm_rust::engine::WorldData::current()
                    .read(|world| (world.game.autopilot.x, world.game.autopilot.y)),
                (-1, -1)
            );
            ship.uninit();
        }
    })
}
