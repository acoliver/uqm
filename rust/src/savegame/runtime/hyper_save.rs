//! `getSisElement` and `SaveSisHyperState` (hyper.c) in Rust: the part of a
//! save that records which way the flagship faces in hyperspace.
//!
//! `SaveSisHyperState` writes one field, `GLOBAL (ShipFacing)`, and it is
//! worked out in four steps, each a pointer the C code follows:
//!
//! 1. `getSisElement`: the head of `race_q[RPG_PLAYER_NUM]` is the flagship's
//!    `STARSHIP`; its `hShip` is the flagship's element. No `STARSHIP` in the
//!    queue, or none with an element, is the "saving from the hyperspace
//!    encounter screen" case: nothing is written and the save goes on.
//! 2. `hShip` is a link of `disp_q`, so it is an `ELEMENT`.
//! 3. `GetElementStarShip`: the element's `pParent` is the `STARSHIP` that owns
//!    it. This is read from the element, not assumed to be the head of step 1:
//!    the two are the same ship in a running game, but nothing in the queues
//!    forces it, and the facing that C stores is the owner's.
//! 4. `ShipFacing` becomes the owner's `ShipFacing` plus one. The reentry test
//!    of the solar system needs it non-zero.
//!
//! The queues are the game's own `QUEUE_TABLE` pools, borrowed as they are, and
//! the records are the generated `STARSHIP` and `ELEMENT`. Both queues are
//! walked against their pools (`queues::walk`), so a handle that is not a link
//! of its queue, a loop and a pool of the wrong element size are all errors
//! before a field is read.
//!
//! Everything is worked out before the one write, so a call that returns an
//! error has changed nothing.
//!
//! Where this differs from the C code on purpose:
//!
//! * C trusts `hShip`. The display queue is walked and an `hShip` that is not
//!   one of its links, such as a slot the game has since freed, is an error.
//! * C asserts `PLAYER_SHIP` on the element only in a `DEBUG` build and then
//!   dereferences `pParent` whatever it holds. Here an element without
//!   `PLAYER_SHIP` is always an error, a NULL `pParent` is an error, and so is
//!   an owner that is not a link of the same `race_q` side (the flagship's
//!   element belongs to the human player's `STARSHIP`).
//! * `ShipFacing` of 65535 would store 0, which the reentry test reads as "no
//!   facing", so it is an error and not a wrapped value.
//!
//! Nothing here moves the flagship: no field of `GAME_STATE` other than
//! `ShipFacing` is written.

use super::queues::QueueError;
use crate::comm::locdata::{CGameState, CQueue};
use std::fmt;
/// Why the flagship's facing could not be prepared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperSaveError {
    /// `race_q` or `disp_q` is not a `QUEUE_TABLE` pool of its record.
    Queue(QueueError),
    /// The flagship's `hShip` is not a link of `disp_q`.
    ElementNotInDisplayList,
    /// The flagship's element does not have `PLAYER_SHIP` set.
    ElementIsNotAPlayerShip,
    /// The flagship's element has no owning `STARSHIP`.
    NoOwner,
    /// The element's owner is not a link of the human player's `race_q`.
    OwnerNotInPlayerQueue,
    /// `ShipFacing` is 65535, so the saved facing would wrap to 0.
    FacingOverflow,
}

impl fmt::Display for HyperSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for HyperSaveError {}

impl From<QueueError> for HyperSaveError {
    fn from(error: QueueError) -> Self {
        Self::Queue(error)
    }
}

/// What `SaveSisHyperState` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperFlagship {
    /// There was no flagship element, so `ShipFacing` was left as it was.
    Absent,
    /// `ShipFacing` was set to this value: the flagship's facing plus one.
    Facing(u16),
}

/// The flagship's own `ShipFacing`, or `None` when there is no flagship
/// element (`getSisElement` returning NULL). Nothing is written.
///
/// # Safety
/// `player_ships` must be the `STARSHIP` queue of the human player and
/// `display_list` the `ELEMENT` queue; every handle either of them reaches is
/// examined only after the queue was walked against its pool, but the pools
/// themselves must be live memory that C is not changing during the call.
pub unsafe fn read_flagship_facing(
    player_ships: &CQueue,
    display_list: &CQueue,
) -> Result<Option<u16>, HyperSaveError> {
    crate::battle::context_access::save::read_flagship_facing(player_ships, display_list)
}

/// The flagship's facing plus one, or `None` when there is no flagship element
/// (`getSisElement` returning NULL).
///
/// # Safety
/// As [`read_flagship_facing`].
unsafe fn flagship_facing(
    player_ships: &CQueue,
    display_list: &CQueue,
) -> Result<Option<u16>, HyperSaveError> {
    match read_flagship_facing(player_ships, display_list)? {
        None => Ok(None),
        Some(facing) => facing
            .checked_add(1)
            .map(Some)
            .ok_or(HyperSaveError::FacingOverflow),
    }
}

/// `SaveSisHyperState ()` over the borrowed queues: store the flagship's
/// facing plus one in `game.ship_facing`.
///
/// # Safety
/// As [`flagship_facing`]: `player_ships` is `race_q[RPG_PLAYER_NUM]`,
/// `display_list` is `disp_q`, and neither is changed by anyone else while the
/// call runs.
pub unsafe fn save_hyper_facing(
    game: &mut CGameState,
    player_ships: &CQueue,
    display_list: &CQueue,
) -> Result<HyperFlagship, HyperSaveError> {
    match flagship_facing(player_ships, display_list)? {
        None => Ok(HyperFlagship::Absent),
        Some(facing) => {
            game.ship_facing = facing;
            Ok(HyperFlagship::Facing(facing))
        }
    }
}
