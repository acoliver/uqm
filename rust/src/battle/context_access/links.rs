//! Picker reads borrow the caller's retained fleet authority.
#[cfg(not(test))]
use crate::engine::{
    battle::{link_queue::*, ShipId},
    Kernel,
};

#[cfg(not(test))]
pub(crate) fn last_ship(kernel: &Kernel, side: usize) -> Option<ShipId> {
    let queue = kernel.ships_ref(side);
    queue.tail().map(|id| queue.ship_id(id))
}

#[cfg(not(test))]
pub(crate) fn ship_value(kernel: &Kernel, id: ShipId) -> PickShip {
    let queue = kernel.ships_ref(id.side() as usize);
    queue.get(queue.link(id))
}

#[cfg(not(test))]
pub(crate) fn all_links(kernel: &Kernel) -> Vec<ShipId> {
    let queue = kernel.ships_ref(0);
    let mut found = Vec::new();
    let mut link = queue.head();
    while let Some(id) = link {
        found.push(queue.ship_id(id));
        link = queue.succ(id);
    }
    found
}

#[cfg(not(test))]
pub(crate) fn with_ship_resources<R>(
    kernel: &Kernel,
    ship: ShipId,
    read: impl FnOnce(PickerResources<'_>) -> R,
) -> R {
    let queue = kernel.ships_ref(ship.side() as usize);
    read(queue.resources(queue.link(ship)))
}
