//! Master metadata owns its descriptors in the game lifecycle's typed ship queue.

use crate::engine::battle::link_queue::{LinkQueue, Starship};

fn find(queue: &LinkQueue<'_, Starship>, species: u32) -> i32 {
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
    let queue = LinkQueue::<Starship>::borrow(
        &crate::engine::battle::record::Kernel::current().master_queue,
    );
    cost_in(&queue, index as u16)
}

fn cost_in(queue: &LinkQueue<'_, Starship>, index: u16) -> u16 {
    let mut next = queue.head();
    for _ in 0..index {
        let Some(id) = next else { return 0 };
        next = queue.succ(id);
    }
    next.map_or(0, |id| {
        u16::from(
            queue
                .record(id)
                .descriptor
                .as_deref()
                .unwrap_or_else(|| panic!("master descriptor"))
                .ship_info
                .ship_cost,
        )
    })
}

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::{
        link_queue::{LinkQueue, Starship},
        record::{FLEET_STUFF, SHIP_INFO, STARSHIP},
    };
    use crate::ships::native::abi::*;
    use std::{ffi::CStr, ptr};

    #[derive(Clone, Copy)]
    pub struct MasterShip {
        pub species: u32,
        pub info: SHIP_INFO,
        pub fleet: FLEET_STUFF,
    }

    unsafe fn with_catalog<R>(call: impl FnOnce(*mut QUEUE) -> R) -> R {
        crate::app::bridge::enter(|cx| call(cx.master_queue()))
    }

    fn entry(
        queue: &LinkQueue<'_, Starship>,
        id: crate::engine::battle::link_queue::LinkId<Starship>,
    ) -> MasterShip {
        let ship = queue.record(id);
        let descriptor = ship
            .descriptor
            .as_deref()
            .unwrap_or_else(|| panic!("master ship descriptor"));
        MasterShip {
            species: ship.SpeciesID,
            info: descriptor.ship_info,
            fleet: descriptor.fleet,
        }
    }

    /// # Safety
    /// The game thread owns the resource system and calls this once before using the catalog.
    #[no_mangle]
    pub unsafe extern "C" fn LoadMasterShipList(yield_processing: Option<unsafe extern "C" fn()>) {
        with_catalog(|raw| {
            assert_ne!(InitQueue(raw, 25, size_of::<STARSHIP>() as u16), 0);
            for species in SPECIES_ID_ARILOU_ID..=SPECIES_ID_LAST_MELEE_ID {
                if let Some(yield_processing) = yield_processing {
                    yield_processing();
                }
                let descriptor = crate::ships::load_ship(species, 0)
                    .unwrap_or_else(|| panic!("master species {species}"));
                let name = CStr::from_ptr(GetStringAddress(SetAbsStringTableIndex(
                    descriptor.ship_info.race_strings,
                    2,
                )));
                let mut queue = LinkQueue::<Starship>::new(&mut *raw);
                let id = queue
                    .alloc()
                    .unwrap_or_else(|| panic!("master catalog capacity"));
                queue.record_mut(id).SpeciesID = species;
                queue.record_mut(id).descriptor = Some(descriptor);
                let mut before = queue.head();
                while let Some(other) = before {
                    let info = entry(&queue, other).info;
                    let other_name = CStr::from_ptr(GetStringAddress(SetAbsStringTableIndex(
                        info.race_strings,
                        2,
                    )));
                    if name < other_name {
                        break;
                    }
                    before = queue.succ(other);
                }
                let raw_id = queue.raw(id);
                let raw_before = before.map_or(ptr::null_mut(), |id| queue.raw(id));
                InsertQueue(raw, raw_id, raw_before);
            }
        });
    }

    /// # Safety
    /// Called after all catalog resource users have released their references.
    #[no_mangle]
    pub unsafe extern "C" fn FreeMasterShipList() {
        with_catalog(|raw| {
            let mut queue = LinkQueue::<Starship>::new(&mut *raw);
            let mut next = queue.head();
            while let Some(id) = next {
                next = queue.succ(id);
                let descriptor = queue
                    .record_mut(id)
                    .descriptor
                    .take()
                    .unwrap_or_else(|| panic!("master descriptor"));
                crate::ships::unload_ship(descriptor, 1, 0);
            }
            UninitQueue(raw);
        });
    }

    /// # Safety
    /// The initialized master catalog is owned by the calling game thread.
    pub unsafe fn master(index: u16) -> Option<MasterShip> {
        with_catalog(|raw| {
            let queue = LinkQueue::<Starship>::borrow(&*raw);
            let mut next = queue.head();
            for _ in 0..index {
                next = queue.succ(next?);
            }
            next.map(|id| entry(&queue, id))
        })
    }

    /// # Safety
    /// The initialized master catalog is owned by the calling game thread.
    pub unsafe fn find_master_ship(species: u32) -> Option<MasterShip> {
        let index = FindMasterShipIndex(species);
        (index >= 0).then(|| master(index as u16).unwrap_or_else(|| panic!("master index")))
    }

    /// # Safety
    /// The initialized master catalog is owned by the calling game thread.
    #[no_mangle]
    pub unsafe extern "C" fn FindMasterShipIndex(species: u32) -> i32 {
        with_catalog(|raw| {
            let queue = LinkQueue::<Starship>::borrow(&*raw);
            super::find(&queue, species)
        })
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
    use crate::collections::queue::{InitQueue, Queue, UninitQueue};
    use crate::engine::battle::record::STARSHIP;

    #[test]
    fn cost_uses_the_owned_catalog_in_queue_order() {
        let mut raw = Queue::default();
        // SAFETY: this test exclusively owns the queue and its descriptor allocations.
        unsafe {
            assert!(InitQueue(&mut raw, 2, size_of::<STARSHIP>() as u16));
            {
                let mut ships = LinkQueue::<Starship>::new(&mut raw);
                assert_eq!(cost_in(&ships, 0), 0);
                for (species, cost) in [(23, 23), (1, 16)] {
                    let id = ships.build(species).unwrap();
                    let mut descriptor = Box::new(crate::engine::battle::record::RACE_DESC::EMPTY);
                    descriptor.ship_info.ship_cost = cost;
                    ships.record_mut(id).descriptor = Some(descriptor);
                }
                assert_eq!(cost_in(&ships, 0), 23);
                assert_eq!(cost_in(&ships, 1), 16);
                assert_eq!(cost_in(&ships, 2), 0);
                assert_eq!(cost_in(&ships, u16::MAX), 0);
                let mut next = ships.head();
                while let Some(id) = next {
                    next = ships.succ(id);
                    ships.record_mut(id).descriptor.take();
                }
            }
            UninitQueue(&mut raw);
        }
    }

    #[test]
    fn lookup_uses_queue_order_and_reports_absent_species() {
        let mut raw = Queue::default();
        // SAFETY: this test exclusively owns the queue and every record in it.
        unsafe {
            assert!(InitQueue(&mut raw, 3, size_of::<STARSHIP>() as u16));
            {
                let mut ships = LinkQueue::<Starship>::new(&mut raw);
                assert_eq!(find(&ships, 1), -1);
                for species in [23, 1, 2] {
                    ships.build(species).unwrap();
                }
                assert_eq!(find(&ships, 1), 1);
                assert_eq!(find(&ships, 2), 2);
                assert_eq!(find(&ships, 99), -1);
            }
            UninitQueue(&mut raw);
        }
    }
}
