//! Starting flagship equipment and prices, shared with the live world initializer.

use super::abi::*;
use crate::engine::battle::schema::GAME_STATE;

pub fn equipment(sis: &mut SIS_STATE) {
    sis.drive_slots.fill(NUM_MODULES as u8);
    sis.drive_slots[5] = FUSION_THRUSTER as u8;
    sis.drive_slots[6] = FUSION_THRUSTER as u8;
    sis.jet_slots.fill((NUM_MODULES + 1) as u8);
    sis.jet_slots[0] = TURNING_JETS as u8;
    sis.jet_slots[6] = TURNING_JETS as u8;
    sis.module_slots.fill((NUM_MODULES + 2) as u8);
    sis.module_slots[15] = GUN_WEAPON as u8;
    sis.module_slots[2] = CREW_POD as u8;
    sis.crew_enlisted = CREW_POD_CAPACITY as u16;
    sis.module_slots[8] = STORAGE_BAY as u8;
    sis.module_slots[1] = FUEL_TANK as u8;
    sis.fuel_on_board = 10 * FUEL_TANK_SCALE;
}

pub fn prices(sis: &mut SIS_STATE, state: &mut GAME_STATE) {
    sis.res_units = 0;
    state.crew_cost = 3;
    state.fuel_cost = 20;
    for (module, cost) in [
        (PLANET_LANDER, 500),
        (FUSION_THRUSTER, 500),
        (TURNING_JETS, 500),
        (CREW_POD, 2000),
        (STORAGE_BAY, 750),
        (FUEL_TANK, 500),
        (DYNAMO_UNIT, 2000),
        (GUN_WEAPON, 2000),
    ] {
        state.module_cost[module as usize] = (cost / MODULE_COST_SCALE) as u8;
    }
    sis.num_landers = 1;
}
