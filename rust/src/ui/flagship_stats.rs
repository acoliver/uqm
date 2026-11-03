//! Flagship statistics computed from owned equipment, without graphics access.

use crate::engine::world::SisState;
use crate::navigation::abi::{
    DYNAMO_UNIT, FUEL_TANK, FUEL_TANK_CAPACITY, FUEL_TANK_SCALE, FUSION_THRUSTER,
    HEFUEL_TANK_CAPACITY, HIGHEFF_FUELSYS, SHIVA_FURNACE, TURNING_JETS,
};

#[derive(Debug, PartialEq)]
pub(super) struct Stats {
    pub energy_regen: u8,
    pub energy_wait: u8,
    pub thrust: u16,
    pub turn_wait: u8,
    pub fuel: u32,
}

pub(super) fn calculate(sis: &SisState) -> Stats {
    let mut energy_regen: u8 = 1;
    let mut energy_wait: u8 = 10;
    let mut thrust: u16 = 10;
    let mut turn_wait: u8 = 17;
    let mut fuel: u32 = 10 * FUEL_TANK_SCALE;
    for &m in &sis.module_slots {
        match u32::from(m) {
            FUEL_TANK => fuel += FUEL_TANK_CAPACITY,
            HIGHEFF_FUELSYS => fuel += HEFUEL_TANK_CAPACITY,
            DYNAMO_UNIT => energy_wait = ((i32::from(energy_wait) - 2) as u8).max(4),
            SHIVA_FURNACE => energy_regen += 1,
            _ => {}
        }
    }
    for &d in &sis.drive_slots {
        if u32::from(d) == FUSION_THRUSTER {
            thrust += 2;
        }
    }
    for &j in &sis.jet_slots {
        if u32::from(j) == TURNING_JETS {
            turn_wait -= 2;
        }
    }
    Stats {
        energy_regen,
        energy_wait,
        thrust,
        turn_wait,
        fuel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sis() -> SisState {
        let mut sis = crate::engine::World::new().data.flagship;
        sis.module_slots
            .fill(crate::navigation::abi::NUM_MODULES as u8);
        sis
    }

    #[test]
    fn empty_equipment_uses_the_original_base_statistics() {
        assert_eq!(
            calculate(&sis()),
            Stats {
                energy_regen: 1,
                energy_wait: 10,
                thrust: 10,
                turn_wait: 17,
                fuel: 10 * FUEL_TANK_SCALE
            }
        );
    }

    #[test]
    fn equipment_counts_preserve_dynamo_floor_and_capacity_units() {
        let mut sis = sis();
        sis.drive_slots.fill(FUSION_THRUSTER as u8);
        sis.jet_slots.fill(TURNING_JETS as u8);
        sis.module_slots[..5].fill(DYNAMO_UNIT as u8);
        sis.module_slots[5..9].fill(SHIVA_FURNACE as u8);
        sis.module_slots[9] = FUEL_TANK as u8;
        sis.module_slots[10] = HIGHEFF_FUELSYS as u8;
        assert_eq!(
            calculate(&sis),
            Stats {
                energy_regen: 5,
                energy_wait: 4,
                thrust: 32,
                turn_wait: 1,
                fuel: 10 * FUEL_TANK_SCALE + FUEL_TANK_CAPACITY + HEFUEL_TANK_CAPACITY
            }
        );
    }
}
