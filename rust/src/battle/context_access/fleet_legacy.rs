use crate::ships::native::abi::*;
#[cfg(not(test))]
use std::ptr;
pub(crate) unsafe fn ship_from_index(queue: *mut QUEUE, mut index: COUNT) -> HLINK {
    // SAFETY: the caller owns the initialized queue and its live links for this operation.
    unsafe {
        let mut handle = (*queue).head;
        while index > 0 && !handle.is_null() {
            handle = (*handle.cast::<LINK>()).succ;
            index -= 1;
        }
        handle
    }
}

/// # Safety
/// Queue must be initialized and use STARSHIP or SHIP_FRAGMENT links.
#[cfg(not(test))]
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn Build(queue: *mut QUEUE, species: SPECIES_ID) -> HLINK {
    // SAFETY: the ABI contract retains the queue and its live links.
    unsafe { build(queue, species) }
}

/// # Safety
/// Queue and handle must refer to the same live linked record.
#[cfg(not(test))]
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn GetIndexFromStarShip(queue: *mut QUEUE, handle: HLINK) -> COUNT {
    // SAFETY: the ABI contract retains the queue and its live links.
    unsafe { index_from_ship(queue, handle) }
}

/// # Safety
/// Queue must be initialized and links must contain SHIP_BASE records.
#[cfg(not(test))]
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn NameCaptain(queue: *mut QUEUE, species: SPECIES_ID) -> BYTE {
    // SAFETY: the caller owns the initialized queue and its live links for this operation.
    unsafe {
        assert!(
            matches!((*queue).object_size as usize, size if size == std::mem::size_of::<SHIP_FRAGMENT>())
        );
        loop {
            let name = (((TFB_Random() as COUNT) & (NUM_CAPTAINS_NAMES - 1) as COUNT)
                + NAME_OFFSET as COUNT) as BYTE;
            let matched = captain_in_use(queue, species, name);
            if !matched {
                return name;
            }
        }
    }
}

#[cfg(not(test))]
unsafe fn captain_in_use(queue: *mut QUEUE, species: SPECIES_ID, name: BYTE) -> bool {
    // SAFETY: NameCaptain retains the initialized queue and its links.
    unsafe {
        let mut handle = (*queue).head;
        let mut matched = false;
        while !handle.is_null() {
            let ship = handle.cast::<SHIP_BASE>();
            let next = (*ship).succ;
            let test_name = if (*ship).SpeciesID == species {
                (*ship).captains_name_index
            } else {
                BYTE::MAX
            };
            if test_name == name {
                matched = true;
                break;
            }
            handle = next;
        }
        matched
    }
}

#[cfg(not(test))]
unsafe fn build(queue: *mut QUEUE, species: SPECIES_ID) -> HLINK {
    // SAFETY: the caller owns the initialized queue and its live links for this operation.
    unsafe {
        if queue.is_null() {
            return ptr::null_mut();
        }
        crate::ships::ffi::build_ship(&mut *queue.cast(), species as i32)
    }
}

#[cfg(not(test))]
unsafe fn index_from_ship(queue: *mut QUEUE, mut handle: HLINK) -> COUNT {
    // SAFETY: the caller owns the initialized queue and its live links for this operation.
    unsafe {
        let mut index: COUNT = 0;
        while handle != (*queue).head {
            let link = handle.cast::<LINK>();
            handle = (*link).pred;
            index = index.wrapping_add(1);
        }
        index
    }
}
