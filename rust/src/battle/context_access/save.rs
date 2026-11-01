//! Save-facing projection over checked typed ship links and element ids.
use crate::comm::locdata::CQueue;
use crate::engine::battle::link_queue::{LinkQueue, Starship};
use crate::savegame::runtime::{
    hyper_save::HyperSaveError,
    queues::{walk_layout, QueueKind},
};
use std::ffi::c_void;
/// # Safety
/// Both queues and their pools are live, immutable game-thread authorities.
pub(crate) unsafe fn read_flagship_facing(
    player_ships: &CQueue,
    display_list: &CQueue,
) -> Result<Option<u16>, HyperSaveError> {
    walk_layout(
        player_ships,
        QueueKind::PlayerShips,
        crate::engine::battle::access::observe::SHIP_LINKS,
    )?;
    if player_ships.head.is_null() {
        return Ok(None);
    }
    // SAFETY: walk checked the live pool, record stride and link order.
    let ships = unsafe { LinkQueue::<Starship>::borrow(&*std::ptr::from_ref(player_ships).cast()) };
    let Some(flagship) = ships.head() else {
        return Ok(None);
    };
    let handle = ships.element_handle(flagship);
    if handle.is_null() {
        return Ok(None);
    }
    walk_layout(
        display_list,
        QueueKind::DisplayList,
        crate::engine::battle::access::observe::ELEMENT_LINKS,
    )?;
    // SAFETY: the caller retains the validated display pool for this lookup.
    let projection = unsafe { display_element(display_list, handle) };
    let Some((state, owner)) = projection else {
        return Err(HyperSaveError::ElementNotInDisplayList);
    };
    if state.flags & crate::ships::native::abi::PLAYER_SHIP as u16 == 0 {
        return Err(HyperSaveError::ElementIsNotAPlayerShip);
    }
    if owner.is_null() {
        return Err(HyperSaveError::NoOwner);
    }
    let mut next = ships.head();
    while let Some(id) = next {
        if ships.raw(id) == owner {
            return Ok(Some(ships.facing(id)));
        }
        next = ships.succ(id);
    }
    Err(HyperSaveError::OwnerNotInPlayerQueue)
}

#[cfg(not(test))]
unsafe fn display_element(
    _queue: &CQueue,
    handle: *mut c_void,
) -> Option<(crate::engine::battle::views::ElementState, *mut c_void)> {
    // SAFETY: the caller owns the published game-thread context.
    unsafe {
        crate::app::bridge::enter(|cx| {
            let mut next = cx.elements().head();
            while let Some(id) = next {
                if cx.elements().raw(id) == handle {
                    return Some((cx.element(id), cx.element_parent_address(id)));
                }
                next = cx.elements().succ(id);
            }
            None
        })
    }
}
#[cfg(test)]
unsafe fn display_element(
    queue: &CQueue,
    handle: *mut c_void,
) -> Option<(crate::engine::battle::views::ElementState, *mut c_void)> {
    use crate::engine::battle::record::ELEMENT;
    use crate::savegame::runtime::queues::walk_layout;
    // Unit fixtures supply independent immutable pools instead of the published game authority.
    let elements = walk_layout(
        queue,
        QueueKind::DisplayList,
        crate::engine::battle::access::observe::ELEMENT_LINKS,
    )
    .unwrap();
    elements
        .into_iter()
        .find(|p| p.cast::<c_void>() == handle)
        .map(|p| {
            // SAFETY: walk_layout validated this retained fixture record.
            unsafe {
                (
                    crate::engine::battle::access::observe_element_record(&*p.cast::<ELEMENT>()),
                    (*p.cast::<ELEMENT>()).pParent,
                )
            }
        })
}
