//! Save the hyperspace flagship's facing from its owned ship and element pools.

use super::super::battle::link_queue::{LinkQueue, RacePool, Starship};
use super::super::battle::schema::GAME_STATE;
use super::super::battle::{ElementTable, Parent};
use super::super::{Kernel, World};
use std::fmt;

const PLAYER_SHIP: u16 = 1 << 2;

/// Why the flagship's facing could not be prepared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperSaveError {
    ElementNotInDisplayList,
    ElementIsNotAPlayerShip,
    NoOwner,
    OwnerNotInPlayerQueue,
    FacingOverflow,
}

impl fmt::Display for HyperSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for HyperSaveError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperFlagship {
    Absent,
    Facing(u16),
}

/// Read the element owner's facing, which need not be the queue head's facing.
pub fn read_flagship_facing(
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
    if record.state_flags & PLAYER_SHIP == 0 {
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

/// Store facing plus one for solar reentry, leaving all other fields unchanged.
/// Validation completes before the single write, including overflow refusal.
pub fn save_hyper_facing(
    game: &mut GAME_STATE,
    player_ships: &RacePool,
    display_list: &ElementTable,
) -> Result<HyperFlagship, HyperSaveError> {
    let Some(facing) = read_flagship_facing(player_ships, display_list)? else {
        return Ok(HyperFlagship::Absent);
    };
    let facing = facing
        .checked_add(1)
        .ok_or(HyperSaveError::FacingOverflow)?;
    game.ship_facing = facing;
    Ok(HyperFlagship::Facing(facing))
}

/// The human player's pool is side zero in the campaign.
pub fn prepare_hyper_state(
    world: &mut World,
    kernel: &Kernel,
) -> Result<HyperFlagship, HyperSaveError> {
    save_hyper_facing(&mut world.data.game, &kernel.races[0], &kernel.elements)
}
