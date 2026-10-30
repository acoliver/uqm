//! Save-facing projection over checked typed ship links and element ids.
use crate::comm::locdata::CQueue;
use crate::engine::battle::{
    link_queue::{LinkQueue, Starship},
    record::{ELEMENT, STARSHIP},
};
use crate::savegame::runtime::{
    hyper_save::HyperSaveError,
    queues::{walk, QueueElement, QueueKind},
};
use std::ffi::c_void;
impl QueueElement for STARSHIP {
    fn pred(&self) -> *mut c_void {
        self.pred
    }
    fn succ(&self) -> *mut c_void {
        self.succ
    }
}

impl QueueElement for ELEMENT {
    fn pred(&self) -> *mut c_void {
        self.pred
    }
    fn succ(&self) -> *mut c_void {
        self.succ
    }
}

/// # Safety
/// Both queues and their pools are live, immutable game-thread authorities.
pub(crate) unsafe fn read_flagship_facing(
    player_ships: &CQueue,
    display_list: &CQueue,
) -> Result<Option<u16>, HyperSaveError> {
    let linked_ships = walk::<STARSHIP>(player_ships, QueueKind::PlayerShips)?;
    if linked_ships.is_empty() {
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
    let linked_elements = walk::<ELEMENT>(display_list, QueueKind::DisplayList)?;
    if !linked_elements
        .iter()
        .any(|pointer| pointer.cast::<c_void>() == handle)
    {
        return Err(HyperSaveError::ElementNotInDisplayList);
    }
    // SAFETY: the validated display list contains this exact live record; no callback runs here.
    let element = unsafe { &*handle.cast::<ELEMENT>() };
    // SAFETY: the validator retained the display pool and checked this element address.
    let state = unsafe { crate::engine::battle::access::observe_element_record(element) };
    if state.flags & crate::ships::native::abi::PLAYER_SHIP as u16 == 0 {
        return Err(HyperSaveError::ElementIsNotAPlayerShip);
    }
    let owner = element.pParent;
    if owner.is_null() {
        return Err(HyperSaveError::NoOwner);
    }
    if !linked_ships
        .iter()
        .any(|pointer| pointer.cast::<c_void>() == owner)
    {
        return Err(HyperSaveError::OwnerNotInPlayerQueue);
    }
    Ok(Some(ships.facing(
        ships.id(owner).unwrap_or_else(|| panic!("validated owner")),
    )))
}
