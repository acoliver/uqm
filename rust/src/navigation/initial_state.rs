//! Starting flagship equipment and prices, shared with the live world initializer.

use super::abi::*;

pub fn equipment(sis: &mut SIS_STATE) {
    sis.DriveSlots.fill(NUM_MODULES as u8);
    sis.DriveSlots[5] = FUSION_THRUSTER as u8;
    sis.DriveSlots[6] = FUSION_THRUSTER as u8;
    sis.JetSlots.fill((NUM_MODULES + 1) as u8);
    sis.JetSlots[0] = TURNING_JETS as u8;
    sis.JetSlots[6] = TURNING_JETS as u8;
    sis.ModuleSlots.fill((NUM_MODULES + 2) as u8);
    sis.ModuleSlots[15] = GUN_WEAPON as u8;
    sis.ModuleSlots[2] = CREW_POD as u8;
    sis.CrewEnlisted = CREW_POD_CAPACITY as u16;
    sis.ModuleSlots[8] = STORAGE_BAY as u8;
    sis.ModuleSlots[1] = FUEL_TANK as u8;
    sis.FuelOnBoard = 10 * FUEL_TANK_SCALE;
}

pub fn prices(sis: &mut SIS_STATE, state: &mut GAME_STATE) {
    sis.ResUnits = 0;
    state.CrewCost = 3;
    state.FuelCost = 20;
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
        state.ModuleCost[module as usize] = (cost / MODULE_COST_SCALE) as u8;
    }
    sis.NumLanders = 1;
}
