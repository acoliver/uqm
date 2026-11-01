//! Storage lifecycle for the transitional element and ship pools.
#![cfg(not(test))]
use crate::collections::queue::{InitQueue, UninitQueue};
use crate::engine::battle::record;

/// # Safety
/// Run on the game thread before the first element allocation.
pub(crate) unsafe fn init_elements() -> bool {
    // SAFETY: the caller guarantees exclusive access at this pool lifecycle boundary.
    unsafe {
        InitQueue(
            &raw mut record::disp_q,
            crate::ships::native::abi::MAX_DISPLAY_ELEMENTS as u16,
            size_of::<record::ELEMENT>() as u16,
        )
    }
}
/// # Safety
/// Run after the last use of the element pool on the game thread.
pub(crate) unsafe fn uninit_elements() {
    // SAFETY: the caller guarantees exclusive access at this pool lifecycle boundary.
    unsafe {
        UninitQueue(&raw mut record::disp_q);
    }
}
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

/// # Safety
/// Run while initializing the game-thread-owned world.
#[cfg(not(test))]
pub(crate) unsafe fn bind_display_array() {
    // SAFETY: the caller guarantees exclusive access at this pool lifecycle boundary.
    unsafe {
        crate::navigation::globals::GlobData.Game_state.DisplayArray =
            (&raw mut record::DisplayArray).cast();
    }
}
