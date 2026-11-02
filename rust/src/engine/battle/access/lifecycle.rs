//! Element and display storage lifecycle on the owning game thread.
#![cfg(not(test))]
use super::super::record::Kernel;

/// # Safety
/// Run on the game thread before the first element allocation.
pub(crate) unsafe fn init_elements(capacity: u16) -> bool {
    // SAFETY: the caller retains the published Kernel during initialization.
    unsafe { Kernel::current() }
        .elements
        .reset_capacity(usize::from(capacity));
    true
}

/// # Safety
/// Run after the last use of the element pool on the game thread.
pub(crate) unsafe fn uninit_elements() {
    // SAFETY: the caller retains the published Kernel during cleanup.
    unsafe { Kernel::current() }.elements.reinit();
}

pub(crate) fn bind_display_array(game: &mut super::schema::GAME_STATE) {
    game._display_array = std::ptr::null_mut();
}
