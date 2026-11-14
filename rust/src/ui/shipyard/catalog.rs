//! Available escort roster order used by the shipyard.

use crate::engine::{world::queues::FleetInfo, World};

pub(super) fn get_available_race_count(world: &World) -> u16 {
    let queue = world.avail_races_ref();
    std::iter::successors(queue.head(), |&id| queue.succ(id))
        .filter(|&id| queue.get(id).allied_state == 1)
        .count() as u16
}

pub(super) fn get_available_race_from_index(world: &World, index: u8) -> Option<(u16, FleetInfo)> {
    let queue = world.avail_races_ref();
    std::iter::successors(queue.head(), |&id| queue.succ(id))
        .enumerate()
        .filter_map(|(offset, id)| {
            let fleet = queue.get(id);
            (fleet.allied_state == 1).then_some((offset as u16, *fleet))
        })
        .nth(usize::from(index))
}
