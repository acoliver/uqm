//! Temporary flagship and escort state while rendering a saved-game summary.

use crate::collections::pool::OrderedPool;
use crate::engine::world::{queues::ShipFragment, SisState};
use crate::engine::World;

pub(super) struct SavedState {
    flagship: SisState,
    ships: OrderedPool<ShipFragment, 12>,
}

impl SavedState {
    pub(super) fn capture(world: &mut World) -> Self {
        Self {
            flagship: world.data.flagship,
            ships: std::mem::take(&mut world.queues.built_ships),
        }
    }

    pub(super) fn restore(self, world: &mut World) {
        world.data.flagship = self.flagship;
        world.queues.built_ships = self.ships;
    }
}

/// # Safety
/// Race templates must be live and SavedState must hold the campaign escorts.
#[cfg(not(test))]
pub(super) unsafe fn install(world: &mut World, summary: &crate::navigation::abi::SUMMARY_DESC) {
    world.data.flagship = summary.SS;
    for &species in &summary.ShipList[..usize::from(summary.NumShips)] {
        let fleets = world.avail_races_ref();
        let Some(id) = fleets.at(usize::from(species)) else {
            continue;
        };
        let template = *fleets.get(id);
        world.built_ships().clone_fragment(
            &template,
            species,
            0,
            crate::comm::dispatch::production_captain_name,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_reinstates_the_flagship_and_escorts_without_rewinding_other_state() {
        let mut world = crate::engine::World::new();
        world.data.flagship.crew_enlisted = 20;
        let mut ships = world.built_ships();
        let first = ships.alloc().unwrap();
        ships.get_mut(first).crew_level = 9;
        ships.put(first);
        let second = ships.alloc().unwrap();
        ships.put(second);
        let saved = SavedState::capture(&mut world);
        assert_eq!(world.built_ships_ref().count(), 0);
        assert!(!world.built_ships_ref().contains(first));

        world.data.flagship.crew_enlisted = 5;
        let temporary = world.built_ships().alloc().unwrap();
        world.built_ships().put(temporary);
        world.data.game.glob_flags = 7;
        saved.restore(&mut world);

        assert_eq!(world.data.flagship.crew_enlisted, 20);
        let ships = world.built_ships_ref();
        assert_eq!(ships.count(), 2);
        assert_eq!(ships.head(), Some(first));
        assert_eq!(ships.succ(first), Some(second));
        assert_eq!(ships.get(first).crew_level, 9);
        assert!(!ships.contains(temporary));
        assert_eq!(world.data.game.glob_flags, 7);
    }
}
