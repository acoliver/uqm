//! Engine master queue and ownership of its loaded icons and race strings.

use super::native::abi::*;

/// Queue-table links are their record addresses.
///
/// # Safety
/// `head` and every successor must be live master queue entries.
unsafe fn find(mut head: HMASTERSHIP, species: SPECIES_ID) -> (HMASTERSHIP, i32) {
    let mut index = 0;
    while !head.is_null() {
        let entry = &*head.cast::<MASTER_SHIP_INFO>();
        if entry.SpeciesID == species {
            return (head, index);
        }
        head = entry.succ;
        index += 1;
    }
    (head, -1)
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
#[allow(non_snake_case)]
mod engine {
    use super::*;
    use std::{mem, ptr};

    extern "C" {
        fn AllocLink(queue: *mut std::ffi::c_void) -> HLINK;
        fn InsertQueue(queue: *mut std::ffi::c_void, handle: HLINK, before: HLINK);
        fn c_load_ship(species: SPECIES_ID, battle: BOOLEAN) -> *mut RACE_DESC;
        fn c_free_ship(desc: *mut RACE_DESC, icons: BOOLEAN, battle: BOOLEAN);
    }

    #[no_mangle]
    pub static mut master_q: QUEUE = unsafe { mem::zeroed() };

    /// # Safety
    /// Called once during engine startup on the game thread, with loaded resources.
    #[no_mangle]
    pub unsafe extern "C" fn LoadMasterShipList(yield_processing: Option<unsafe extern "C" fn()>) {
        crate::ships::ffi::rust_ships_load_catalog();
        let mut species = SPECIES_ID_ARILOU_ID;
        let count = SPECIES_ID_LAST_MELEE_ID - SPECIES_ID_ARILOU_ID + 1;
        InitQueue(
            &raw mut master_q,
            count as u16,
            mem::size_of::<MASTER_SHIP_INFO>() as u16,
        );
        for _ in 0..count {
            let built = AllocLink((&raw mut master_q).cast());
            if built.is_null() {
                continue;
            }
            if let Some(yield_processing) = yield_processing {
                yield_processing();
            }
            let entry = built.cast::<MASTER_SHIP_INFO>();
            (*entry).SpeciesID = species;
            species += 1;
            let desc = c_load_ship((*entry).SpeciesID, 0);
            if desc.is_null() {
                continue;
            }
            (*entry).ShipInfo = (*desc).ship_info;
            (*entry).Fleet = (*desc).fleet;
            c_free_ship(desc, 0, 0);
            let name = GetStringAddress(SetAbsStringTableIndex((*entry).ShipInfo.race_strings, 2));
            let mut current = master_q.head;
            while !current.is_null() {
                let other = current.cast::<MASTER_SHIP_INFO>();
                let next = (*other).succ;
                let other_name =
                    GetStringAddress(SetAbsStringTableIndex((*other).ShipInfo.race_strings, 2));
                if libc::strcmp(name, other_name) < 0 {
                    break;
                }
                current = next;
            }
            InsertQueue((&raw mut master_q).cast(), built, current);
        }
    }

    /// # Safety
    /// Called after users of master-queue resources have released their references.
    #[no_mangle]
    pub unsafe extern "C" fn FreeMasterShipList() {
        crate::ships::ffi::rust_ships_free_catalog();
        let mut current = master_q.head;
        while !current.is_null() {
            let entry = current.cast::<MASTER_SHIP_INFO>();
            current = (*entry).succ;
            DestroyDrawable(ReleaseDrawable((*entry).ShipInfo.melee_icon));
            DestroyDrawable(ReleaseDrawable((*entry).ShipInfo.icons));
            DestroyStringTable(ReleaseStringTable((*entry).ShipInfo.race_strings));
        }
        UninitQueue(&raw mut master_q);
    }

    /// # Safety
    /// The master queue must be initialized and accessed on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn FindMasterShip(species: SPECIES_ID) -> HMASTERSHIP {
        find(master_q.head, species).0
    }

    /// # Safety
    /// The master queue must be initialized and accessed on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn FindMasterShipIndex(species: SPECIES_ID) -> i32 {
        find(master_q.head, species).1
    }

    /// # Safety
    /// The Rust ship catalog must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn GetShipCostFromIndex(index: u32) -> u16 {
        crate::ships::ffi::rust_ships_get_cost_by_index(index as u16)
    }

    unsafe fn icons(index: u32, melee: bool) -> FRAME {
        let mut current = master_q.head;
        let mut remaining = index as u16;
        while !current.is_null() && remaining != 0 {
            current = (*current.cast::<MASTER_SHIP_INFO>()).succ;
            remaining -= 1;
        }
        if current.is_null() {
            return ptr::null_mut();
        }
        let info = &(*current.cast::<MASTER_SHIP_INFO>()).ShipInfo;
        if melee {
            info.melee_icon
        } else {
            info.icons
        }
    }

    /// # Safety
    /// The master queue and its resources must be live on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn GetShipIconsFromIndex(index: u32) -> FRAME {
        icons(index, false)
    }

    /// # Safety
    /// The master queue and its resources must be live on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn GetShipMeleeIconsFromIndex(index: u32) -> FRAME {
        icons(index, true)
    }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use engine::{
    master_q, FindMasterShip, FindMasterShipIndex, FreeMasterShipList, GetShipCostFromIndex,
    GetShipIconsFromIndex, GetShipMeleeIconsFromIndex, LoadMasterShipList,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::{mem, ptr};

    #[test]
    fn lookup_uses_queue_order_and_reports_absent_species() {
        unsafe {
            let mut entries: [MASTER_SHIP_INFO; 3] = mem::zeroed();
            for (i, entry) in entries.iter_mut().enumerate() {
                entry.SpeciesID = [23, 1, 2][i];
            }
            entries[0].succ = (&raw mut entries[1]).cast();
            entries[1].succ = (&raw mut entries[2]).cast();
            let head = (&raw mut entries[0]).cast();
            assert_eq!(find(head, 1), ((&raw mut entries[1]).cast(), 1));
            assert_eq!(find(head, 2), ((&raw mut entries[2]).cast(), 2));
            assert_eq!(find(head, 99), (ptr::null_mut(), -1));
            assert_eq!(find(ptr::null_mut(), 1), (ptr::null_mut(), -1));
        }
    }
}
