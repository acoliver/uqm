use uqm_rust::ships::races::init_sis;
#[path = "support/battle_context.rs"]
mod battle_context;

mod native_ship_common;
use native_ship_common::*;
use uqm_rust::{
    ships::native::{abi::*, element},
    state::game_state_keys::{get_game_state, set_game_state},
};
unsafe fn modules() {
    uqm_rust::state::ffi::rust_init_game_state();
    GlobData.SIS_state.ModuleSlots = [255; 16];
    GlobData.SIS_state.DriveSlots = [255; 11];
    GlobData.SIS_state.JetSlots = [255; 8];
    GlobData.SIS_state.CrewEnlisted = 9;
    GlobData.Game_state.CurrentActivity = IN_ENCOUNTER as u16;
    set_game_state("CHMMR_BOMB_STATE", 0);
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
                GlobData.SIS_state.ModuleSlots[i] = m;
            }
            GlobData.SIS_state.DriveSlots = [1; 11];
            GlobData.SIS_state.JetSlots = [2; 8];
            let mut ship = Ship::new(init_sis, RPG_PLAYER_NUM as SIZE);
            ship.weapon_frames = FrameSet::new(32);
            ship.desc.ship_data.weapon = [ship.weapon_frames.frame(0); 3];
            assert_eq!(
                (ship.desc.ship_info.max_crew, ship.desc.ship_info.crew_level),
                (101, 10)
            );
            let c = &ship.desc.characteristics;
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
            let out = ship.weapon(6);
            for (h, damage) in out.iter().zip([2, 4, 4, 6, 6, 2]) {
                assert_eq!((*element(*h)).__bindgen_anon_3.mass_points, damage);
                assert_eq!((*element(*h)).__bindgen_anon_4.turn_wait, 17);
                FreeElement(*h);
            }
            ship.desc.ship_info.crew_level = 4;
            (ship.desc.uninit_func.unwrap())(&raw mut *ship.desc);
            assert_eq!({ GlobData.SIS_state.CrewEnlisted }, 3);
            assert!(ship.desc.data.is_null());
        }
    })
}
#[test]
fn no_weapons_disable_inputs_and_bomb_state_marks_carrier() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            modules();
            set_game_state("CHMMR_BOMB_STATE", 3);
            set_game_state("BOMB_CARRIER", 0);
            let mut ship = Ship::new(init_sis, 0);
            assert_eq!(get_game_state("BOMB_CARRIER"), 1);
            ship.ss.cur_status_flags = (SPECIAL | WEAPON) as u16;
            (ship.desc.preprocess_func.unwrap())(ship.ptr());
            assert_eq!(ship.ss.cur_status_flags & (SPECIAL | WEAPON) as u16, 0);
            assert_eq!((ship.ss.weapon_counter, ship.ss.special_counter), (2, 2));
            (ship.desc.uninit_func.unwrap())(&raw mut *ship.desc);
            set_game_state("BOMB_CARRIER", 0);
            set_game_state("CHMMR_BOMB_STATE", 0);
        }
    })
}
#[test]
fn hyperspace_fuel_brakes_and_quasispace_allows_thrust_without_fuel() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            modules();
            GlobData.Game_state.CurrentActivity = IN_HYPERSPACE as u16;
            GlobData.SIS_state.FuelOnBoard = 0;
            GlobData.Game_state.autopilot = POINT { x: -1, y: -1 };
            let mut ship = Ship::new(init_sis, 0);
            assert!(ship.desc.init_weapon_func.is_none());
            assert_eq!(ship.desc.characteristics.max_thrust, 8);
            assert!(ship.desc.ship_info.icons_rsc.is_null());
            SetVelocityComponents(&raw mut (*ship.ptr()).velocity, 256, 0);
            ship.ss.cur_status_flags = THRUST as u16;
            set_game_state("ARILOU_SPACE_SIDE", 0);
            (ship.desc.preprocess_func.unwrap())(ship.ptr());
            let mut x = 0;
            let mut y = 0;
            GetCurrentVelocityComponents(&raw mut (*ship.ptr()).velocity, &mut x, &mut y);
            assert_eq!((x, y), (128, 0));
            assert_eq!(ship.ss.cur_status_flags & THRUST as u16, 0);
            assert_eq!(ship.ss.weapon_counter, 1);
            set_game_state("ARILOU_SPACE_SIDE", 2);
            ship.ss.cur_status_flags = THRUST as u16;
            (*ship.ptr()).__bindgen_anon_5.thrust_wait = 0;
            (ship.desc.preprocess_func.unwrap())(ship.ptr());
            assert_ne!(ship.ss.cur_status_flags & THRUST as u16, 0);
            (ship.desc.postprocess_func.unwrap())(ship.ptr());
            assert_eq!(
                velocity(&{ GlobData.Game_state.velocity }),
                velocity(&(*ship.ptr()).velocity)
            );
            (ship.desc.uninit_func.unwrap())(&raw mut *ship.desc);
            set_game_state("ARILOU_SPACE_SIDE", 0);
        }
    })
}
#[test]
fn autopilot_accelerates_toward_target_and_cancels_on_manual_turn() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            modules();
            GlobData.Game_state.CurrentActivity = IN_HYPERSPACE as u16;
            GlobData.SIS_state.FuelOnBoard = 100;
            GlobData.SIS_state.log_x = 0;
            GlobData.SIS_state.log_y = 0;
            GlobData.Game_state.autopilot = POINT { x: 100, y: 9999 };
            set_game_state("ARILOU_SPACE_SIDE", 0);
            let mut ship = Ship::new(init_sis, 0);
            ship.ss.ShipFacing = 4;
            (ship.desc.preprocess_func.unwrap())(ship.ptr());
            let mut x = 0;
            let mut y = 0;
            GetCurrentVelocityComponents(&raw mut (*ship.ptr()).velocity, &mut x, &mut y);
            assert_eq!((x, y), (128, 0));
            assert_ne!(ship.ss.cur_status_flags & SHIP_AT_MAX_SPEED as u16, 0);
            ship.ss.cur_status_flags = LEFT as u16;
            (ship.desc.preprocess_func.unwrap())(ship.ptr());
            assert_eq!(
                (
                    GlobData.Game_state.autopilot.x,
                    GlobData.Game_state.autopilot.y
                ),
                (-1, -1)
            );
            (ship.desc.uninit_func.unwrap())(&raw mut *ship.desc);
        }
    })
}
