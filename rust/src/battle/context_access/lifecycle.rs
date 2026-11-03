//! Lifecycle of the Kernel-owned battle ship pools.
#![cfg(not(test))]
use crate::engine::battle::record::Kernel;

pub(crate) fn init_ships(kernel: &mut Kernel) {
    kernel.reset_race_capacities(crate::ships::native::abi::MAX_SHIPS_PER_SIDE as usize);
}

pub(crate) fn uninit_ships(kernel: &mut Kernel) {
    kernel.reset_race_capacities(0);
}
