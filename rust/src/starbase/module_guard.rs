//! Module purchase and removal rules from `DoInstallModule`.
//!
//! Ports the select-time guards of `outfit.c:244-295` and the price
//! arithmetic used by the RU updates at `outfit.c:325-332`. The module
//! chooser screen itself (slot cursor, flash rectangles, drawing) is not
//! part of this slice. These functions answer only "may this slot change
//! happen, and what does it cost".
//!
//! A blocked change is a normal outcome (the C code plays
//! `MENU_SOUND_FAILURE` and stays in the screen), so it is a value, not an
//! error. Errors are reserved for malformed input.

use super::sis_modules::{ModuleSlot, ModuleSlots, ModuleType, NUM_MODULES};

/// `MODULE_COST_SCALE` (`sis.h:43`): RU per `ModuleCost` point.
pub const MODULE_COST_SCALE: u32 = 50;
/// `CREW_POD_CAPACITY` (`sis.h:38`).
pub const CREW_POD_CAPACITY: u32 = 50;
/// `STORAGE_BAY_CAPACITY` (`sis.h:39`).
pub const STORAGE_BAY_CAPACITY: u32 = 500;

/// `GLOBAL (ModuleCost[])`: per-module cost in `MODULE_COST_SCALE` units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleCosts([u8; NUM_MODULES]);

/// Raw cost table of the wrong length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected {NUM_MODULES} module costs, got {actual}")]
pub struct ModuleCostsError {
    pub actual: usize,
}

impl ModuleCosts {
    pub fn from_raw(raw: &[u8]) -> Result<Self, ModuleCostsError> {
        let table: [u8; NUM_MODULES] = raw
            .try_into()
            .map_err(|_| ModuleCostsError { actual: raw.len() })?;
        Ok(Self(table))
    }
}

/// RU price of one module: `ModuleCost[module] * MODULE_COST_SCALE`. The
/// same amount is charged on install and refunded on removal
/// (`outfit.c:325-332`).
#[must_use]
pub fn module_price(costs: &ModuleCosts, module: ModuleType) -> u32 {
    u32::from(costs.0[module as usize]) * MODULE_COST_SCALE
}

/// Why a slot change is refused. Each maps to a `MENU_SOUND_FAILURE`
/// (`outfit.c:254`, `265`, `282`, `291`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleBlock {
    /// Not enough RU to build the module.
    InsufficientResUnits,
    /// A crew pod is still needed for the crew on board.
    CrewPodStillNeeded,
    /// A fuel tank is still needed for the fuel on board.
    FuelTankStillNeeded,
    /// A storage bay is still needed for the cargo.
    StorageBayStillNeeded,
}

/// The answer to "may this change happen".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleCheck {
    Allowed,
    Blocked(ModuleBlock),
}

/// Ship contents that can pin a module in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipLoad {
    /// `SIS_STATE.CrewEnlisted`.
    pub crew_enlisted: u16,
    /// `SIS_STATE.TotalElementMass`.
    pub total_element_mass: u16,
    /// `SIS_STATE.FuelOnBoard`.
    pub fuel_on_board: u32,
}

/// Buying a module needs `ResUnits >= price` (`outfit.c:248-256`).
#[must_use]
pub fn check_module_purchase(res_units: u32, price: u32) -> ModuleCheck {
    if res_units < price {
        ModuleCheck::Blocked(ModuleBlock::InsufficientResUnits)
    } else {
        ModuleCheck::Allowed
    }
}

/// `slot_index` is not a module slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("module slot index {slot_index} is out of range")]
pub struct SlotIndexOutOfRange {
    pub slot_index: usize,
}

/// Removing the module in `slot_index` (`outfit.c:257-293`).
///
/// A crew pod, fuel tank or storage bay stays if what it holds would no
/// longer fit without it. An empty slot, or any other module, may go.
pub fn check_module_removal(
    slots: &ModuleSlots,
    slot_index: usize,
    load: ShipLoad,
) -> Result<ModuleCheck, SlotIndexOutOfRange> {
    let slot = slots
        .get(slot_index)
        .ok_or(SlotIndexOutOfRange { slot_index })?;
    let ModuleSlot::Installed(module) = slot else {
        return Ok(ModuleCheck::Allowed);
    };

    let block = match module {
        ModuleType::CrewPod => {
            let remaining = slots.count(ModuleType::CrewPod) - 1;
            (u32::from(load.crew_enlisted) > CREW_POD_CAPACITY * remaining)
                .then_some(ModuleBlock::CrewPodStillNeeded)
        }
        ModuleType::FuelTank | ModuleType::HighEffFuelSys => {
            // `volume + FUEL_RESERVE` in the C code is the capacity left
            // after this tank is gone.
            let remaining_capacity = slots.fuel_tank_capacity() - module.fuel_capacity();
            (load.fuel_on_board > remaining_capacity).then_some(ModuleBlock::FuelTankStillNeeded)
        }
        ModuleType::StorageBay => {
            let remaining = slots.count(ModuleType::StorageBay) - 1;
            (u32::from(load.total_element_mass) > STORAGE_BAY_CAPACITY * remaining)
                .then_some(ModuleBlock::StorageBayStillNeeded)
        }
        _ => None,
    };
    Ok(block.map_or(ModuleCheck::Allowed, ModuleCheck::Blocked))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starbase::sis_modules::{EMPTY_MODULE_SLOT, NUM_MODULE_SLOTS};

    fn slots_with(entries: &[(usize, ModuleType)]) -> ModuleSlots {
        let mut raw = [EMPTY_MODULE_SLOT; NUM_MODULE_SLOTS];
        for (index, module) in entries {
            raw[*index] = *module as u8;
        }
        ModuleSlots::from_raw(&raw).unwrap()
    }

    fn load(crew: u16, mass: u16, fuel: u32) -> ShipLoad {
        ShipLoad {
            crew_enlisted: crew,
            total_element_mass: mass,
            fuel_on_board: fuel,
        }
    }

    #[test]
    fn price_is_cost_times_fifty() {
        let mut raw = [0u8; NUM_MODULES];
        raw[ModuleType::FuelTank as usize] = 4;
        raw[ModuleType::CrewPod as usize] = 255;
        let costs = ModuleCosts::from_raw(&raw).unwrap();
        assert_eq!(module_price(&costs, ModuleType::FuelTank), 200);
        assert_eq!(module_price(&costs, ModuleType::CrewPod), 12_750);
        assert_eq!(module_price(&costs, ModuleType::StorageBay), 0);
    }

    #[test]
    fn cost_table_of_the_wrong_length_is_an_error() {
        assert_eq!(
            ModuleCosts::from_raw(&[1; 19]),
            Err(ModuleCostsError { actual: 19 })
        );
    }

    #[test]
    fn purchase_needs_the_full_price_in_ru() {
        assert_eq!(check_module_purchase(200, 200), ModuleCheck::Allowed);
        assert_eq!(
            check_module_purchase(199, 200),
            ModuleCheck::Blocked(ModuleBlock::InsufficientResUnits)
        );
        assert_eq!(check_module_purchase(0, 0), ModuleCheck::Allowed);
    }

    #[test]
    fn crew_pod_stays_while_its_crew_needs_it() {
        let slots = slots_with(&[(0, ModuleType::CrewPod), (1, ModuleType::CrewPod)]);
        // Two pods hold 100. Removing one leaves room for 50.
        assert_eq!(
            check_module_removal(&slots, 0, load(50, 0, 0)),
            Ok(ModuleCheck::Allowed)
        );
        assert_eq!(
            check_module_removal(&slots, 0, load(51, 0, 0)),
            Ok(ModuleCheck::Blocked(ModuleBlock::CrewPodStillNeeded))
        );
    }

    #[test]
    fn the_last_crew_pod_needs_an_empty_crew() {
        let slots = slots_with(&[(4, ModuleType::CrewPod)]);
        assert_eq!(
            check_module_removal(&slots, 4, load(0, 0, 0)),
            Ok(ModuleCheck::Allowed)
        );
        assert_eq!(
            check_module_removal(&slots, 4, load(1, 0, 0)),
            Ok(ModuleCheck::Blocked(ModuleBlock::CrewPodStillNeeded))
        );
    }

    #[test]
    fn fuel_tank_stays_while_its_fuel_needs_it() {
        let slots = slots_with(&[(0, ModuleType::FuelTank), (1, ModuleType::FuelTank)]);
        // Two tanks hold 11000. Removing one leaves 6000.
        assert_eq!(
            check_module_removal(&slots, 1, load(0, 0, 6000)),
            Ok(ModuleCheck::Allowed)
        );
        assert_eq!(
            check_module_removal(&slots, 1, load(0, 0, 6001)),
            Ok(ModuleCheck::Blocked(ModuleBlock::FuelTankStillNeeded))
        );
    }

    #[test]
    fn removing_the_only_tank_leaves_the_reserve() {
        let slots = slots_with(&[(0, ModuleType::FuelTank)]);
        assert_eq!(
            check_module_removal(&slots, 0, load(0, 0, 1000)),
            Ok(ModuleCheck::Allowed)
        );
        assert_eq!(
            check_module_removal(&slots, 0, load(0, 0, 1001)),
            Ok(ModuleCheck::Blocked(ModuleBlock::FuelTankStillNeeded))
        );
    }

    #[test]
    fn high_efficiency_tank_removal_uses_its_own_capacity() {
        let slots = slots_with(&[(0, ModuleType::HighEffFuelSys), (1, ModuleType::FuelTank)]);
        assert_eq!(
            check_module_removal(&slots, 0, load(0, 0, 6000)),
            Ok(ModuleCheck::Allowed)
        );
        assert_eq!(
            check_module_removal(&slots, 0, load(0, 0, 6001)),
            Ok(ModuleCheck::Blocked(ModuleBlock::FuelTankStillNeeded))
        );
        assert_eq!(
            check_module_removal(&slots, 1, load(0, 0, 11_000)),
            Ok(ModuleCheck::Allowed)
        );
        assert_eq!(
            check_module_removal(&slots, 1, load(0, 0, 11_001)),
            Ok(ModuleCheck::Blocked(ModuleBlock::FuelTankStillNeeded))
        );
    }

    #[test]
    fn storage_bay_stays_while_its_cargo_needs_it() {
        let slots = slots_with(&[(2, ModuleType::StorageBay), (3, ModuleType::StorageBay)]);
        assert_eq!(
            check_module_removal(&slots, 2, load(0, 500, 0)),
            Ok(ModuleCheck::Allowed)
        );
        assert_eq!(
            check_module_removal(&slots, 2, load(0, 501, 0)),
            Ok(ModuleCheck::Blocked(ModuleBlock::StorageBayStillNeeded))
        );
    }

    #[test]
    fn other_modules_and_empty_slots_may_always_go() {
        let slots = slots_with(&[(0, ModuleType::DynamoUnit), (1, ModuleType::GunWeapon)]);
        let full = load(u16::MAX, u16::MAX, u32::MAX);
        assert_eq!(
            check_module_removal(&slots, 0, full),
            Ok(ModuleCheck::Allowed)
        );
        assert_eq!(
            check_module_removal(&slots, 1, full),
            Ok(ModuleCheck::Allowed)
        );
        assert_eq!(
            check_module_removal(&slots, 7, full),
            Ok(ModuleCheck::Allowed)
        );
    }

    #[test]
    fn slot_index_past_the_slots_is_an_error() {
        let slots = ModuleSlots::empty();
        assert_eq!(
            check_module_removal(&slots, NUM_MODULE_SLOTS, load(0, 0, 0)),
            Err(SlotIndexOutOfRange {
                slot_index: NUM_MODULE_SLOTS
            })
        );
    }
}
