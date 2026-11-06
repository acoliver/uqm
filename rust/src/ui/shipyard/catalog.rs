//! Available escort roster order used by the shipyard.

use super::*;

pub(super) fn get_available_race_count() -> COUNT {
    avail_race_queue(|queue| {
        std::iter::successors(queue.head(), |&id| queue.succ(id))
            .filter(|&id| queue.get(id).allied_state == GOOD_GUY as u16)
            .count() as COUNT
    })
}

pub(super) fn get_available_race_from_index(index: BYTE) -> Option<(COUNT, FleetInfo)> {
    avail_race_queue(|queue| {
        std::iter::successors(queue.head(), |&id| queue.succ(id))
            .enumerate()
            .filter_map(|(offset, id)| {
                let fleet = queue.get(id);
                (fleet.allied_state == GOOD_GUY as u16).then_some((offset as COUNT, *fleet))
            })
            .nth(usize::from(index))
    })
}
