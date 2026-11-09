//! Owned flagship transactions for the module-installation menu.
use crate::starbase::module_guard;
use crate::starbase::sis_modules as equipment;
use crate::starbase::sis_modules::ModuleType;

use crate::engine::World;

pub(super) const EMPTY_SLOT: u8 = equipment::NUM_MODULES as u8;

pub(super) fn can_install(world: &World, new_piece: u8, old_piece: u8) -> bool {
    let sis = &world.data.flagship;
    if new_piece < EMPTY_SLOT {
        return sis.res_units
            >= u32::from(world.data.game.module_cost[new_piece as usize])
                * module_guard::MODULE_COST_SCALE;
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
    match ModuleType::from_u8(old_piece) {
        Some(ModuleType::CrewPod) => {
            i32::from(sis.crew_enlisted)
                <= module_guard::CREW_POD_CAPACITY as i32 * (count(ModuleType::CrewPod as u32) - 1)
        }
        Some(ModuleType::FuelTank | ModuleType::HighEffFuelSys) => {
            let mut volume = count(ModuleType::FuelTank as u32) as u32
                * equipment::FUEL_TANK_CAPACITY
                + count(ModuleType::HighEffFuelSys as u32) as u32 * equipment::HEFUEL_TANK_CAPACITY;
            volume -= if u32::from(old_piece) == ModuleType::FuelTank as u32 {
                equipment::FUEL_TANK_CAPACITY
            } else {
                equipment::HEFUEL_TANK_CAPACITY
            };
            sis.fuel_on_board <= volume + equipment::FUEL_RESERVE
        }
        Some(ModuleType::StorageBay) => {
            i32::from(sis.total_element_mass)
                <= module_guard::STORAGE_BAY_CAPACITY as i32
                    * (count(ModuleType::StorageBay as u32) - 1)
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
        s if s == ModuleType::PlanetLander as u32 => sis.num_landers += 1,
        s if s == u32::from(EMPTY_SLOT + 3) => sis.num_landers -= 1,
        s if s == ModuleType::FusionThruster as u32 || s == u32::from(EMPTY_SLOT) => {
            sis.drive_slots[slot as usize] = new_piece
        }
        s if s == ModuleType::TurningJets as u32 || s == u32::from(EMPTY_SLOT + 1) => {
            sis.jet_slots[slot as usize] = new_piece
        }
        _ => sis.module_slots[slot as usize] = new_piece,
    }
    if new_piece < EMPTY_SLOT {
        -(i32::from(world.data.game.module_cost[new_piece as usize])
            * module_guard::MODULE_COST_SCALE as i32)
    } else {
        i32::from(world.data.game.module_cost[old_piece as usize])
            * module_guard::MODULE_COST_SCALE as i32
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
        world.data.game.module_cost[ModuleType::CrewPod as usize] = 3;
        world.data.flagship.res_units = 3 * module_guard::MODULE_COST_SCALE - 1;
        assert!(!can_install(
            &world,
            ModuleType::CrewPod as u8,
            EMPTY_SLOT + 2
        ));
        world.data.flagship.res_units += 1;
        assert!(can_install(
            &world,
            ModuleType::CrewPod as u8,
            EMPTY_SLOT + 2
        ));
    }

    #[test]
    fn removal_keeps_crew_fuel_and_cargo_within_remaining_capacity() {
        let mut world = world();
        let sis = &mut world.data.flagship;
        sis.module_slots[0] = ModuleType::CrewPod as u8;
        sis.crew_enlisted = 1;
        assert!(!can_install(
            &world,
            EMPTY_SLOT + 2,
            ModuleType::CrewPod as u8
        ));
        world.data.flagship.crew_enlisted = 0;
        assert!(can_install(
            &world,
            EMPTY_SLOT + 2,
            ModuleType::CrewPod as u8
        ));
        world.data.flagship.module_slots[1] = ModuleType::HighEffFuelSys as u8;
        world.data.flagship.fuel_on_board = equipment::FUEL_RESERVE + 1;
        assert!(!can_install(
            &world,
            EMPTY_SLOT + 2,
            ModuleType::HighEffFuelSys as u8
        ));
        world.data.flagship.fuel_on_board -= 1;
        assert!(can_install(
            &world,
            EMPTY_SLOT + 2,
            ModuleType::HighEffFuelSys as u8
        ));
        world.data.flagship.module_slots[2] = ModuleType::StorageBay as u8;
        world.data.flagship.total_element_mass = 1;
        assert!(!can_install(
            &world,
            EMPTY_SLOT + 2,
            ModuleType::StorageBay as u8
        ));
        world.data.flagship.total_element_mass = 0;
        assert!(can_install(
            &world,
            EMPTY_SLOT + 2,
            ModuleType::StorageBay as u8
        ));
    }

    #[test]
    fn installation_updates_only_the_selected_slot_and_returns_the_gauge_delta() {
        let mut world = world();
        world.data.game.module_cost[ModuleType::CrewPod as usize] = 3;
        let cost = apply_piece(
            &mut world,
            ModuleType::CrewPod as u8,
            7,
            ModuleType::CrewPod as u8,
            EMPTY_SLOT + 2,
        );
        assert_eq!(cost, -(3 * module_guard::MODULE_COST_SCALE as i32));
        assert_eq!(
            world.data.flagship.module_slots[7],
            ModuleType::CrewPod as u8
        );
        assert_eq!(world.data.flagship.module_slots[6], EMPTY_SLOT + 2);
        let refund = apply_piece(
            &mut world,
            EMPTY_SLOT + 2,
            7,
            EMPTY_SLOT + 2,
            ModuleType::CrewPod as u8,
        );
        assert_eq!(refund, 3 * module_guard::MODULE_COST_SCALE as i32);
        assert_eq!(world.data.flagship.module_slots[7], EMPTY_SLOT + 2);
        apply_piece(
            &mut world,
            ModuleType::PlanetLander as u8,
            0,
            ModuleType::PlanetLander as u8,
            EMPTY_SLOT + 3,
        );
        assert_eq!(world.data.flagship.num_landers, 1);
        apply_piece(
            &mut world,
            EMPTY_SLOT + 3,
            0,
            EMPTY_SLOT + 3,
            ModuleType::PlanetLander as u8,
        );
        assert_eq!(world.data.flagship.num_landers, 0);
    }
}
