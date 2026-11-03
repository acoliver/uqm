//! Master metadata owns its descriptors in the game lifecycle's typed ship queue.

use crate::engine::battle::kernel::MasterCatalog;
use crate::engine::Kernel as KernelOwner;

fn find(queue: &MasterCatalog, species: u32) -> i32 {
    let mut next = queue.head();
    let mut index = 0;
    while let Some(id) = next {
        if queue.get(id).species == species {
            return index;
        }
        next = queue.succ(id);
        index += 1;
    }
    -1
}

/// # Safety
/// The game thread owns the initialized catalog throughout this synchronous lookup.
pub(crate) unsafe fn cost(index: u32) -> u16 {
    let queue = &(**KernelOwner::current()).master_queue;
    cost_in(queue, index as u16)
}

fn cost_in(queue: &MasterCatalog, index: u16) -> u16 {
    let mut next = queue.head();
    for _ in 0..index {
        let Some(id) = next else { return 0 };
        next = queue.succ(id);
    }
    next.map_or(0, |id| u16::from(queue.get(id).descriptor.info().ship_cost))
}

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::{
        kernel::{CatalogEntry, MasterCatalog},
        record::{FLEET_STUFF, SHIP_INFO},
    };
    use crate::engine::{Kernel as KernelOwner, World as WorldOwner};
    use crate::ships::native::abi::*;
    use std::{ffi::CStr, ptr};

    #[derive(Clone, Copy)]
    pub struct MasterShip {
        pub species: u32,
        pub info: SHIP_INFO,
        pub fleet: FLEET_STUFF,
    }

    unsafe fn with_catalog<R>(call: impl FnOnce(&mut MasterCatalog) -> R) -> R {
        KernelOwner::current().write(|kernel| call(&mut kernel.master_queue))
    }

    fn entry(queue: &MasterCatalog, id: crate::collections::pool::Id<CatalogEntry>) -> MasterShip {
        MasterShip {
            species: queue.get(id).species,
            info: *queue.get(id).descriptor.info(),
            fleet: *queue.get(id).descriptor.fleet(),
        }
    }

    /// # Safety
    /// The game thread owns the resource system and calls this once before using the catalog.
    #[no_mangle]
    pub unsafe extern "C" fn LoadMasterShipList(yield_processing: Option<unsafe extern "C" fn()>) {
        with_catalog(|queue| assert_eq!(queue.count(), 0, "catalog already loaded"));
        for species in SPECIES_ID_ARILOU_ID..=SPECIES_ID_LAST_MELEE_ID {
            if let Some(yield_processing) = yield_processing {
                yield_processing();
            }
            let descriptor = WorldOwner::current()
                .write(|world| crate::ships::load_ship(world, species, 0))
                .unwrap_or_else(|| panic!("master species {species}"));
            let name = CStr::from_ptr(GetStringAddress(SetAbsStringTableIndex(
                descriptor.info().race_strings,
                2,
            )));
            with_catalog(|queue| {
                let id = queue
                    .alloc(CatalogEntry {
                        species,
                        descriptor,
                    })
                    .unwrap_or_else(|| panic!("master catalog capacity"));
                let mut before = queue.head();
                while let Some(other) = before {
                    let info = entry(queue, other).info;
                    let other_name = CStr::from_ptr(GetStringAddress(SetAbsStringTableIndex(
                        info.race_strings,
                        2,
                    )));
                    if name < other_name {
                        break;
                    }
                    before = queue.succ(other);
                }
                queue.insert_before(id, before);
            });
        }
    }

    /// # Safety
    /// Called after all catalog resource users have released their references.
    #[no_mangle]
    pub unsafe extern "C" fn FreeMasterShipList() {
        while let Some(descriptor) = with_catalog(|queue| {
            let id = queue.head()?;
            queue.remove(id);
            Some(queue.free(id).descriptor)
        }) {
            WorldOwner::current().write(|world| crate::ships::unload_ship(world, descriptor, 1, 0));
        }
        with_catalog(MasterCatalog::reinit);
    }

    /// # Safety
    /// The initialized master catalog is owned by the calling game thread.
    pub unsafe fn master(index: u16) -> Option<MasterShip> {
        with_catalog(|queue| {
            let mut next = queue.head();
            for _ in 0..index {
                next = queue.succ(next?);
            }
            next.map(|id| entry(queue, id))
        })
    }

    /// # Safety
    /// The initialized master catalog is owned by the calling game thread.
    pub unsafe fn find_master_ship(
        kernel: &crate::engine::Kernel,
        species: u32,
    ) -> Option<MasterShip> {
        let queue = &kernel.master_queue;
        let mut next = queue.head();
        while let Some(id) = next {
            if queue.get(id).species == species {
                return Some(entry(queue, id));
            }
            next = queue.succ(id);
        }
        None
    }

    /// # Safety
    /// The initialized master catalog is owned by the calling game thread.
    #[no_mangle]
    pub unsafe extern "C" fn FindMasterShipIndex(species: u32) -> i32 {
        with_catalog(|queue| super::find(queue, species))
    }

    /// # Safety
    /// The initialized master catalog is owned by the calling game thread.
    #[no_mangle]
    pub unsafe extern "C" fn GetShipCostFromIndex(index: u32) -> u16 {
        super::cost(index)
    }
    /// # Safety
    /// The master catalog's graphics remain loaded on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn GetShipIconsFromIndex(index: u32) -> FRAME {
        master(index as u16).map_or(ptr::null_mut(), |ship| ship.info.icons)
    }
    /// # Safety
    /// The master catalog's graphics remain loaded on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn GetShipMeleeIconsFromIndex(index: u32) -> FRAME {
        master(index as u16).map_or(ptr::null_mut(), |ship| ship.info.melee_icon)
    }
}

#[cfg(not(test))]
pub use engine::{
    find_master_ship, master, FindMasterShipIndex, FreeMasterShipList, GetShipCostFromIndex,
    GetShipIconsFromIndex, GetShipMeleeIconsFromIndex, LoadMasterShipList,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::battle::{kernel::CatalogEntry, record::RaceDescriptor, RaceDesc};

    fn push(queue: &mut MasterCatalog, species: u32, cost: u8) {
        let mut descriptor = RaceDescriptor::EMPTY;
        descriptor.ship_info.ship_cost = cost;
        let id = queue
            .alloc(CatalogEntry {
                species,
                descriptor: RaceDesc::fixture(Box::new(descriptor)),
            })
            .unwrap();
        queue.put(id);
    }

    #[test]
    fn cost_uses_the_owned_catalog_in_queue_order() {
        let mut ships = MasterCatalog::new();
        assert_eq!(cost_in(&ships, 0), 0);
        push(&mut ships, 23, 23);
        push(&mut ships, 1, 16);
        assert_eq!(cost_in(&ships, 0), 23);
        assert_eq!(cost_in(&ships, 1), 16);
        assert_eq!(cost_in(&ships, 2), 0);
        assert_eq!(cost_in(&ships, u16::MAX), 0);
        let id = ships.head().unwrap();
        ships.remove(id);
        let retired = ships.free(id);
        assert_eq!(retired.species, 23);
        assert_eq!(retired.descriptor.info().ship_cost, 23);
        assert_eq!(cost_in(&ships, 0), 16);
    }

    #[test]
    fn lookup_uses_queue_order_and_reports_absent_species() {
        let mut ships = MasterCatalog::new();
        assert_eq!(find(&ships, 1), -1);
        for species in [23, 1, 2] {
            push(&mut ships, species, 0);
        }
        assert_eq!(find(&ships, 1), 1);
        assert_eq!(find(&ships, 2), 2);
        assert_eq!(find(&ships, 99), -1);
        ships.reinit();
        assert_eq!(find(&ships, 1), -1);
        for species in 1..=25 {
            push(&mut ships, species, 0);
        }
        assert!(ships
            .alloc(CatalogEntry {
                species: 26,
                descriptor: RaceDesc::fixture(Box::new(RaceDescriptor::EMPTY)),
            })
            .is_none());
    }
}
