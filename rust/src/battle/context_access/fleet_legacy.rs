use crate::ships::native::abi::*;
#[cfg(not(test))]
use std::ptr;
pub(crate) unsafe fn ship_from_index(queue: *mut QUEUE, mut index: COUNT) -> HLINK {
    // SAFETY: the caller owns the initialized queue and its live links for this operation.

    if usize::from((*queue).object_size) == crate::engine::battle::link_queue::ship_size() {
        let ships = crate::engine::battle::link_queue::LinkQueue::<
            crate::engine::battle::link_queue::Starship,
        >::new(&mut *queue);
        let mut link = ships.head();
        while let Some(id) = link.filter(|_| index > 0) {
            link = ships.succ(id);
            index -= 1;
        }
        return link.map_or(std::ptr::null_mut(), |id| ships.raw(id));
    }
    let mut handle = (*queue).head;
    while index > 0 && !handle.is_null() {
        handle = (*handle.cast::<LINK>()).succ;
        index -= 1;
    }
    handle
}

/// # Safety
/// Queue must be initialized and use STARSHIP or SHIP_FRAGMENT links.
#[cfg(not(test))]
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn Build(queue: *mut QUEUE, species: SPECIES_ID) -> HLINK {
    // SAFETY: the ABI contract retains the queue and its live links.
    build(queue, species)
}

/// # Safety
/// Queue and handle must refer to the same live linked record.
#[cfg(not(test))]
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn GetIndexFromStarShip(queue: *mut QUEUE, handle: HLINK) -> COUNT {
    // SAFETY: the ABI contract retains the queue and its live links.
    index_from_ship(queue, handle)
}

/// # Safety
/// Queue must be initialized and links must contain SHIP_BASE records.
#[cfg(not(test))]
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn NameCaptain(queue: *mut QUEUE, species: SPECIES_ID) -> BYTE {
    // SAFETY: the caller owns the initialized queue and its live links for this operation.

    assert!(
        matches!((*queue).object_size as usize, size if size == crate::engine::battle::link_queue::ship_size() || size == std::mem::size_of::<SHIP_FRAGMENT>())
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

#[cfg(not(test))]
unsafe fn captain_in_use(queue: *mut QUEUE, species: SPECIES_ID, name: BYTE) -> bool {
    // SAFETY: NameCaptain retains the initialized queue and its links.

    if usize::from((*queue).object_size) == crate::engine::battle::link_queue::ship_size() {
        let ships = crate::engine::battle::link_queue::LinkQueue::<
            crate::engine::battle::link_queue::Starship,
        >::new(&mut *queue);
        let mut link = ships.head();
        let mut found = false;
        while let Some(id) = link {
            let ship = ships.get(id);
            if ship.species == species && ship.captain == name {
                found = true;
                break;
            }
            link = ships.succ(id);
        }
        found
    } else {
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

    if queue.is_null() {
        return ptr::null_mut();
    }
    if usize::from((*queue).object_size) == crate::engine::battle::link_queue::ship_size() {
        let mut ships = crate::engine::battle::link_queue::LinkQueue::<
            crate::engine::battle::link_queue::Starship,
        >::new(&mut *queue);
        ships
            .build(species)
            .map_or(ptr::null_mut(), |id| ships.raw(id))
    } else {
        crate::ships::ffi::build_ship(&mut *queue.cast(), species as i32)
    }
}
#[cfg(not(test))]
unsafe fn index_from_ship(queue: *mut QUEUE, mut handle: HLINK) -> COUNT {
    // SAFETY: the caller owns the initialized queue and its live links for this operation.

    let mut index: COUNT = 0;
    if usize::from((*queue).object_size) == crate::engine::battle::link_queue::ship_size() {
        let ships = crate::engine::battle::link_queue::LinkQueue::<
            crate::engine::battle::link_queue::Starship,
        >::new(&mut *queue);
        let mut link = ships.id(handle);
        while link != ships.head() {
            link = ships.pred(link.unwrap_or_else(|| panic!("ship is not linked")));
            index = index.wrapping_add(1);
        }
        return index;
    }
    while handle != (*queue).head {
        let link = handle.cast::<LINK>();
        handle = (*link).pred;
        index = index.wrapping_add(1);
    }
    index
}
