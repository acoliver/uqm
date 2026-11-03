//! Lifecycle of the Kernel-owned battle ship pools.
#![cfg(not(test))]
use crate::engine::battle::record::Kernel;

pub(crate) fn init_ships(kernel: &mut Kernel) {
    for pool in &mut kernel.races {
        pool.reset_capacity(crate::ships::native::abi::MAX_SHIPS_PER_SIDE as usize);
    }
}

pub(crate) fn uninit_ships(kernel: &mut Kernel) {
    for pool in &mut kernel.races {
        pool.reset_capacity(0);
    }
}
