//! Field-only lifecycle operations for Stage B storage.
#![cfg(not(test))]
use super::super::record;
use crate::collections::queue::{InitQueue, UninitQueue};
/// # Safety
/// Run on the game thread before the first element allocation.
pub(crate) unsafe fn init_elements(capacity: u16) -> bool {
    // SAFETY: the caller guarantees exclusive access at this pool lifecycle boundary.
    unsafe {
        InitQueue(
            &raw mut record::disp_q,
            capacity,
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

/// Bind the retained display storage while initializing the game-thread-owned world.
pub(crate) fn bind_display_array(game: &mut super::schema::GAME_STATE) {
    game._display_array = std::ptr::null_mut();
}
