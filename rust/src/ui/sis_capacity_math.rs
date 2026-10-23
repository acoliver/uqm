//! Slot searches retain the SIS crew and mineral boundary conventions.

use crate::starbase::sis_modules::ModuleType;

pub(crate) fn module_crew_capacity(module: u8) -> u16 {
    if module == ModuleType::CrewPod as u8 {
        50
    } else {
        0
    }
}
pub(crate) fn module_storage_capacity(module: u8) -> u16 {
    if module == ModuleType::StorageBay as u8 {
        500
    } else {
        0
    }
}

pub(crate) fn crew_pod_for(modules: &[u8], crew: u16) -> Option<(u16, u16)> {
    let mut capacity = 0;
    for (slot, &module) in modules.iter().enumerate().rev() {
        let module_capacity = module_crew_capacity(module);
        if crew < capacity + module_capacity {
            return Some((slot as u16, crew - capacity));
        }
        capacity += module_capacity;
    }
    None
}

pub(crate) fn storage_cell_for(modules: &[u8], unit: u16) -> Option<(u16, u16)> {
    let mut capacity = 0;
    for (slot, &module) in modules.iter().enumerate().rev() {
        let module_capacity = module_storage_capacity(module);
        if unit <= capacity + module_capacity {
            return Some((slot as u16, unit - capacity));
        }
        capacity += module_capacity;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starbase::sis_modules::EMPTY_MODULE_SLOT;

    #[test]
    fn crew_fills_pods_from_the_rear_and_excludes_full_endpoint() {
        let mut slots = [EMPTY_MODULE_SLOT; 16];
        slots[3] = ModuleType::CrewPod as u8;
        slots[11] = ModuleType::CrewPod as u8;
        assert_eq!(crew_pod_for(&slots, 0), Some((11, 0)));
        assert_eq!(crew_pod_for(&slots, 49), Some((11, 49)));
        assert_eq!(crew_pod_for(&slots, 50), Some((3, 0)));
        assert_eq!(crew_pod_for(&slots, 99), Some((3, 49)));
        assert_eq!(crew_pod_for(&slots, 100), None);
    }

    #[test]
    fn mineral_cell_search_includes_full_endpoint_and_zero_in_empty_slot() {
        let mut slots = [EMPTY_MODULE_SLOT; 16];
        assert_eq!(storage_cell_for(&slots, 0), Some((15, 0)));
        assert_eq!(storage_cell_for(&slots, 1), None);
        slots[3] = ModuleType::StorageBay as u8;
        slots[11] = ModuleType::StorageBay as u8;
        assert_eq!(storage_cell_for(&slots, 500), Some((11, 500)));
        assert_eq!(storage_cell_for(&slots, 501), Some((3, 1)));
        assert_eq!(storage_cell_for(&slots, 1000), Some((3, 500)));
        assert_eq!(storage_cell_for(&slots, 1001), None);
    }
}
