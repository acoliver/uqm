//! Owned flagship transactions for the module-installation menu.

use crate::engine::World;
use crate::navigation::abi::*;

pub(super) const EMPTY_SLOT: u8 = NUM_MODULES as u8;

pub(super) fn can_install(world: &World, new_piece: u8, old_piece: u8) -> bool {
    let sis = &world.data.flagship;
    if new_piece < EMPTY_SLOT {
        return sis.res_units
            >= u32::from(world.data.game.module_cost[new_piece as usize]) * MODULE_COST_SCALE;
    }
    if new_piece != EMPTY_SLOT + 2 {
        return true;
    }
    let count = |piece| {
        sis.module_slots
            .iter()
            .filter(|&&module| u32::from(module) == piece)
            .count() as i32
    };
    match u32::from(old_piece) {
        CREW_POD => {
            i32::from(sis.crew_enlisted) <= CREW_POD_CAPACITY as i32 * (count(CREW_POD) - 1)
        }
        FUEL_TANK | HIGHEFF_FUELSYS => {
            let mut volume = count(FUEL_TANK) as u32 * FUEL_TANK_CAPACITY
                + count(HIGHEFF_FUELSYS) as u32 * HEFUEL_TANK_CAPACITY;
            volume -= if u32::from(old_piece) == FUEL_TANK {
                FUEL_TANK_CAPACITY
            } else {
                HEFUEL_TANK_CAPACITY
            };
            sis.fuel_on_board <= volume + FUEL_RESERVE
        }
        STORAGE_BAY => {
            i32::from(sis.total_element_mass)
                <= STORAGE_BAY_CAPACITY as i32 * (count(STORAGE_BAY) - 1)
        }
        _ => true,
    }
}

pub(super) fn apply_piece(
    world: &mut World,
    state: u8,
    slot: i16,
    new_piece: u8,
    old_piece: u8,
) -> i32 {
    let sis = &mut world.data.flagship;
    match u32::from(state) {
        PLANET_LANDER => sis.num_landers += 1,
        s if s == u32::from(EMPTY_SLOT + 3) => sis.num_landers -= 1,
        s if s == FUSION_THRUSTER || s == u32::from(EMPTY_SLOT) => {
            sis.drive_slots[slot as usize] = new_piece
        }
        s if s == TURNING_JETS || s == u32::from(EMPTY_SLOT + 1) => {
            sis.jet_slots[slot as usize] = new_piece
        }
        _ => sis.module_slots[slot as usize] = new_piece,
    }
    if new_piece < EMPTY_SLOT {
        -(i32::from(world.data.game.module_cost[new_piece as usize]) * MODULE_COST_SCALE as i32)
    } else {
        i32::from(world.data.game.module_cost[old_piece as usize]) * MODULE_COST_SCALE as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let mut world = World::new();
        world.data.flagship.module_slots.fill(EMPTY_SLOT + 2);
        world
    }

    #[test]
    fn purchase_requires_the_exact_scaled_price() {
        let mut world = world();
        world.data.game.module_cost[CREW_POD as usize] = 3;
        world.data.flagship.res_units = 3 * MODULE_COST_SCALE - 1;
        assert!(!can_install(&world, CREW_POD as u8, EMPTY_SLOT + 2));
        world.data.flagship.res_units += 1;
        assert!(can_install(&world, CREW_POD as u8, EMPTY_SLOT + 2));
    }

    #[test]
    fn removal_keeps_crew_fuel_and_cargo_within_remaining_capacity() {
        let mut world = world();
        let sis = &mut world.data.flagship;
        sis.module_slots[0] = CREW_POD as u8;
        sis.crew_enlisted = 1;
        assert!(!can_install(&world, EMPTY_SLOT + 2, CREW_POD as u8));
        world.data.flagship.crew_enlisted = 0;
        assert!(can_install(&world, EMPTY_SLOT + 2, CREW_POD as u8));
        world.data.flagship.module_slots[1] = HIGHEFF_FUELSYS as u8;
        world.data.flagship.fuel_on_board = FUEL_RESERVE + 1;
        assert!(!can_install(&world, EMPTY_SLOT + 2, HIGHEFF_FUELSYS as u8));
        world.data.flagship.fuel_on_board -= 1;
        assert!(can_install(&world, EMPTY_SLOT + 2, HIGHEFF_FUELSYS as u8));
        world.data.flagship.module_slots[2] = STORAGE_BAY as u8;
        world.data.flagship.total_element_mass = 1;
        assert!(!can_install(&world, EMPTY_SLOT + 2, STORAGE_BAY as u8));
        world.data.flagship.total_element_mass = 0;
        assert!(can_install(&world, EMPTY_SLOT + 2, STORAGE_BAY as u8));
    }

    #[test]
    fn installation_updates_only_the_selected_slot_and_returns_the_gauge_delta() {
        let mut world = world();
        world.data.game.module_cost[CREW_POD as usize] = 3;
        let cost = apply_piece(
            &mut world,
            CREW_POD as u8,
            7,
            CREW_POD as u8,
            EMPTY_SLOT + 2,
        );
        assert_eq!(cost, -(3 * MODULE_COST_SCALE as i32));
        assert_eq!(world.data.flagship.module_slots[7], CREW_POD as u8);
        assert_eq!(world.data.flagship.module_slots[6], EMPTY_SLOT + 2);
        let refund = apply_piece(
            &mut world,
            EMPTY_SLOT + 2,
            7,
            EMPTY_SLOT + 2,
            CREW_POD as u8,
        );
        assert_eq!(refund, 3 * MODULE_COST_SCALE as i32);
        assert_eq!(world.data.flagship.module_slots[7], EMPTY_SLOT + 2);
        apply_piece(
            &mut world,
            PLANET_LANDER as u8,
            0,
            PLANET_LANDER as u8,
            EMPTY_SLOT + 3,
        );
        assert_eq!(world.data.flagship.num_landers, 1);
        apply_piece(
            &mut world,
            EMPTY_SLOT + 3,
            0,
            EMPTY_SLOT + 3,
            PLANET_LANDER as u8,
        );
        assert_eq!(world.data.flagship.num_landers, 0);
    }
}
