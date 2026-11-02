//! Save-facing projection over typed element allocations and ship links.
use crate::comm::locdata::CQueue;
use crate::engine::battle::link_queue::{LinkQueue, Starship};
use crate::engine::battle::{ElementTable, Parent};
use crate::savegame::runtime::{
    hyper_save::HyperSaveError,
    queues::{walk_layout, QueueKind},
};

/// # Safety
/// The ship pool and its resources remain live and immutable for the call.
pub(crate) unsafe fn read_flagship_facing(
    player_ships: &CQueue,
    elements: &ElementTable,
) -> Result<Option<u16>, HyperSaveError> {
    walk_layout(
        player_ships,
        QueueKind::PlayerShips,
        crate::engine::battle::access::observe::SHIP_LINKS,
    )?;
    // SAFETY: validation checked the retained ship pool and its links.
    let ships = unsafe { LinkQueue::<Starship>::borrow(&*std::ptr::from_ref(player_ships).cast()) };
    let Some(flagship) = ships.head() else {
        return Ok(None);
    };
    let handle = ships.element_handle(flagship);
    if handle.is_null() {
        return Ok(None);
    }
    let Some(element) = elements
        .allocated_id(handle)
        .filter(|&id| elements.linked(id))
    else {
        return Err(HyperSaveError::ElementNotInDisplayList);
    };
    let record = &elements[element];
    if record.state_flags & crate::ships::native::abi::PLAYER_SHIP as u16 == 0 {
        return Err(HyperSaveError::ElementIsNotAPlayerShip);
    }
    if record.parent == Parent::None {
        return Err(HyperSaveError::NoOwner);
    }
    let mut next = ships.head();
    while let Some(id) = next {
        if record.parent
            == Parent::Ship(ships.ship_id(id, crate::ships::native::abi::RPG_PLAYER_NUM as u8))
        {
            return Ok(Some(ships.facing(id)));
        }
        next = ships.succ(id);
    }
    Err(HyperSaveError::OwnerNotInPlayerQueue)
}
