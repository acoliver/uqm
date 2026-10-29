#[cfg(not(test))]
use crate::engine::battle::{link_queue::*, ShipId};
#[cfg(not(test))]
use std::ffi::c_void;

#[cfg(not(test))]
fn with_fleet<R>(side: usize, read: impl FnOnce(LinkQueue<'_, Starship>) -> R) -> R {
    // SAFETY: these picker accessors run on the game thread, and return only
    // copied values or ABI handles. The view ends before any legacy callback.
    unsafe { read(ships(side)) }
}

#[cfg(not(test))]
impl LinkQueue<'_, Starship> {
    pub(crate) fn tail_of(side: usize) -> *mut c_void {
        with_fleet(side, |queue| {
            queue
                .tail()
                .map_or(std::ptr::null_mut(), |id| queue.raw(id))
        })
    }
    pub(crate) fn ship_value(id: ShipId) -> PickShip {
        with_fleet(id.side as usize, |queue| queue.get(queue.link(id)))
    }
    pub(crate) fn identify_ship(side: usize, raw: *mut c_void) -> Option<ShipId> {
        with_fleet(side, |queue| {
            queue.id(raw).map(|id| queue.ship_id(id, side as u8))
        })
    }
    pub(crate) fn ship_raw(id: Option<ShipId>) -> *mut c_void {
        id.map_or(std::ptr::null_mut(), |id| {
            with_fleet(id.side as usize, |queue| queue.raw(queue.link(id)))
        })
    }
    pub(crate) fn first_ship(side: usize) -> Option<ShipId> {
        with_fleet(side, |queue| {
            queue.head().map(|id| queue.ship_id(id, side as u8))
        })
    }
    pub(crate) fn all_links() -> Vec<ShipId> {
        with_fleet(0, |queue| {
            let mut found = Vec::new();
            let mut link = queue.head();
            while let Some(id) = link {
                found.push(queue.ship_id(id, 0));
                link = queue.succ(id);
            }
            found
        })
    }
}
