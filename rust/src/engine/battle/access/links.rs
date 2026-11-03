#[cfg(not(test))]
use super::link_queue::*;
#[cfg(not(test))]
use crate::engine::battle::ShipId;
#[cfg(not(test))]
use std::ptr;
/// # Safety
/// Only the game thread may take a view, and it must end it before a legacy call.
#[cfg(not(test))]
pub(crate) unsafe fn ships<'a>(side: usize) -> LinkQueue<'a, Starship> {
    assert!(side < 2, "invalid ship side {side}");
    // SAFETY: callers serialize the address-stable battle ship authority.
    unsafe { LinkQueue::new(&mut *crate::engine::battle::record::ship_queue(side).as_ptr()) }
}

/// # Safety
/// The address is a live ship in a game-thread-owned battle pool, without a retained queue borrow.
#[cfg(not(test))]
pub(crate) unsafe fn identify(address: usize) -> ShipId {
    for side in 0..2 {
        // SAFETY: callers retain the published game-thread Kernel throughout this lookup.
        let pool = unsafe { crate::engine::battle::record::ship_queue(side).as_ref() };
        for slot in 0..32 {
            if let Some(id) = pool.pool.id_at(slot) {
                if ptr::from_ref(pool.pool.get(id)) as usize == address {
                    return ShipId {
                        side: side as u8,
                        slot: slot as u8,
                        gen: id.generation(),
                        owner: pool.pool.storage_identity(),
                    };
                }
            }
        }
    }
    panic!("ship address {address:#x} is outside the ship pools")
}

#[cfg(not(test))]
impl LinkQueue<'_, Starship> {
    /// # Safety
    /// The fragment's string table and icon drawable stay loaded until this ship is cleared.
    pub(crate) unsafe fn set_picker_resources(
        &mut self,
        id: LinkId<Starship>,
        strings: crate::graphics::native_abi::STRING,
        icons: crate::graphics::native_abi::FRAME,
    ) {
        let ship = self.record_mut(id);
        ship.race_strings = strings;
        ship.icons = icons;
    }
}
