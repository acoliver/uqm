//! Storage lifecycle for the transitional element and ship pools.
#![cfg(not(test))]
use crate::collections::queue::{InitQueue, UninitQueue};
use crate::engine::battle::record;

/// # Safety
/// Run on the game thread before the first ship allocation.
pub(crate) unsafe fn init_ships() {
    for side in 0..2 {
        // SAFETY: the caller guarantees exclusive access at this pool lifecycle boundary.
        unsafe {
            InitQueue(
                record::ship_queue(side).as_ptr(),
                crate::ships::native::abi::MAX_SHIPS_PER_SIDE as u16,
                crate::engine::battle::link_queue::ship_size() as u16,
            );
        }
    }
}
/// # Safety
/// Run after all ship resources and element references have been retired.
pub(crate) unsafe fn uninit_ships() {
    for side in 0..2 {
        // SAFETY: the caller guarantees exclusive access at this pool lifecycle boundary.
        unsafe {
            UninitQueue(record::ship_queue(side).as_ptr());
        }
    }
}
