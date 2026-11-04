//! Save-facing projection over typed element allocations and ship links.
use crate::engine::battle::link_queue::{LinkQueue, RacePool, Starship};
use crate::engine::battle::{ElementTable, Parent};
use crate::savegame::runtime::hyper_save::HyperSaveError;

/// # Safety
/// The ship pool and its resources remain live and immutable for the call.
pub(crate) fn read_flagship_facing(
    player_ships: &RacePool,
    elements: &ElementTable,
) -> Result<Option<u16>, HyperSaveError> {
    let ships = LinkQueue::<Starship>::borrow(player_ships);
    let Some(flagship) = ships.head() else {
        return Ok(None);
    };
    let Some(element) = ships.element_handle(flagship) else {
        return Ok(None);
    };
    if !elements.contains(element) || !elements.linked(element) {
        return Err(HyperSaveError::ElementNotInDisplayList);
    }
    let record = &elements[element];
    if record.state_flags & crate::ships::native::abi::PLAYER_SHIP as u16 == 0 {
        return Err(HyperSaveError::ElementIsNotAPlayerShip);
    }
    if record.parent == Parent::None {
        return Err(HyperSaveError::NoOwner);
    }
    let mut next = ships.head();
    while let Some(id) = next {
        if record.parent == Parent::Ship(ships.ship_id(id)) {
            return Ok(Some(ships.facing(id)));
        }
        next = ships.succ(id);
    }
    Err(HyperSaveError::OwnerNotInPlayerQueue)
}
