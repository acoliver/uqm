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
