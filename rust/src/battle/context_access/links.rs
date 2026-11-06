#[cfg(not(test))]
use crate::engine::battle::{link_queue::*, ShipId};

#[cfg(not(test))]
fn with_fleet<R>(side: usize, read: impl FnOnce(LinkQueue<'_, Starship>) -> R) -> R {
    // SAFETY: these picker accessors run on the game thread, and return only
    // copied values or ABI handles. The view ends before any legacy callback.
    unsafe { read(ships(side)) }
}

#[cfg(not(test))]
impl LinkQueue<'_, Starship> {
    pub(crate) fn last_ship(side: usize) -> Option<ShipId> {
        with_fleet(side, |queue| queue.tail().map(|id| queue.ship_id(id)))
    }
    pub(crate) fn ship_value(id: ShipId) -> PickShip {
        with_fleet(id.side() as usize, |queue| queue.get(queue.link(id)))
    }
    pub(crate) fn all_links() -> Vec<ShipId> {
        with_fleet(0, |queue| {
            let mut found = Vec::new();
            let mut link = queue.head();
            while let Some(id) = link {
                found.push(queue.ship_id(id));
                link = queue.succ(id);
            }
            found
        })
    }
}

#[cfg(not(test))]
impl LinkQueue<'_, Starship> {
    pub(crate) fn with_ship_resources<R>(
        ship: ShipId,
        read: impl FnOnce(PickerResources<'_>) -> R,
    ) -> R {
        with_fleet(ship.side() as usize, |queue| {
            read(queue.resources(queue.link(ship)))
        })
    }
}
