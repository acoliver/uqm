//! Element and display storage lifecycle on the owning game thread.
#![cfg(not(test))]
use super::super::record::Kernel;

pub(crate) fn init_elements(kernel: &mut Kernel, capacity: u16) -> bool {
    kernel.elements.reset_capacity(usize::from(capacity));
    true
}

pub(crate) fn uninit_elements(kernel: &mut Kernel) {
    kernel.elements.reinit();
}

pub(crate) fn bind_display_array(game: &mut super::schema::GAME_STATE) {
    game._display_array = std::ptr::null_mut();
}
