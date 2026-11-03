//! Temporary flagship and escort state while rendering a saved-game summary.

use crate::collections::queue::Queue;
use crate::engine::{world::SisState, WorldData};

pub(super) struct SavedState {
    flagship: SisState,
    ships: Queue,
}

impl SavedState {
    pub(super) fn capture(world: &WorldData) -> Self {
        Self {
            flagship: world.flagship,
            ships: world.game.built_ship_q,
        }
    }

    pub(super) fn restore(self, world: &mut WorldData) {
        world.flagship = self.flagship;
        world.game.built_ship_q = self.ships;
    }
}

/// # Safety
/// Race templates must be live. Release the temporary escort queue before restoring SavedState.
#[cfg(not(test))]
pub(super) unsafe fn install(
    world: &mut WorldData,
    summary: &crate::navigation::abi::SUMMARY_DESC,
) {
    use crate::navigation::abi::*;
    world.flagship = summary.SS;
    let queue = &raw mut world.game.built_ship_q;
    // SAFETY: the saved header is retained separately; this initializes the temporary queue.
    unsafe {
        InitQueue(
            queue,
            MAX_BUILT_SHIPS as COUNT,
            std::mem::size_of::<SHIP_FRAGMENT>() as OBJ_SIZE,
        );
    }
    for &species in &summary.ShipList[..usize::from(summary.NumShips)] {
        // SAFETY: live templates and the newly initialized destination are distinct queues.
        unsafe {
            CloneShipFragment(&mut world.game.avail_race_q, species as COUNT, queue, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_reinstates_the_flagship_and_escorts_without_rewinding_other_state() {
        let mut world = crate::engine::World::new();
        world.data.flagship.crew_enlisted = 20;
        world.data.game.built_ship_q.object_size = 36;
        let saved = SavedState::capture(&world.data);
        world.data.flagship.crew_enlisted = 5;
        world.data.game.built_ship_q.object_size = 40;
        world.data.game.glob_flags = 7;
        saved.restore(&mut world.data);
        assert_eq!(world.data.flagship.crew_enlisted, 20);
        assert_eq!(world.data.game.built_ship_q.object_size, 36);
        assert_eq!(world.data.game.glob_flags, 7);
    }
}
