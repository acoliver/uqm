//! Lifecycle of the Kernel-owned battle ship pools.
#![cfg(not(test))]
use crate::engine::battle::record::Kernel;

/// # Safety
/// Run on the owning game thread before the first ship allocation.
pub(crate) unsafe fn init_ships() {
    // SAFETY: the lifecycle owner exclusively retains the published Kernel.
    for pool in &mut unsafe { Kernel::current() }.races {
        pool.reset_capacity(crate::ships::native::abi::MAX_SHIPS_PER_SIDE as usize);
    }
}

/// # Safety
/// Run after ship resources and element references have been retired.
pub(crate) unsafe fn uninit_ships() {
    // SAFETY: the lifecycle owner exclusively retains the published Kernel.
    for pool in &mut unsafe { Kernel::current() }.races {
        pool.reset_capacity(0);
    }
}
