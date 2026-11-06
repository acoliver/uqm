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
    pub(crate) fn first_ship(side: usize) -> Option<ShipId> {
        with_fleet(side, |queue| queue.head().map(|id| queue.ship_id(id)))
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
    pub(crate) fn mark_dead(ship: ShipId) -> PickShip {
        with_fleet(ship.side() as usize, |mut queue| {
            let id = queue.link(ship);
            queue.edit(id, |value| value.species = 0);
            queue.get(id)
        })
    }
    pub(crate) fn next_ship(ship: ShipId) -> Option<ShipId> {
        with_fleet(ship.side() as usize, |queue| {
            queue.succ(queue.link(ship)).map(|id| queue.ship_id(id))
        })
    }
    pub(crate) fn restore_ship(ship: ShipId, player: i16, captain: u8) {
        with_fleet(ship.side() as usize, |mut queue| {
            queue.edit(queue.link(ship), |value| {
                value.crew = value.max_crew;
                value.player = player;
                value.captain = captain;
            });
        });
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
