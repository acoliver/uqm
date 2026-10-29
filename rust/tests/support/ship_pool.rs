use std::{
    mem,
    ops::{Deref, DerefMut},
};
use uqm_rust::{
    collections::queue::{AllocLink, FreeLink},
    ships::native::abi::{race_q, SIZE, STARSHIP},
};

pub struct PoolShip {
    pointer: *mut STARSHIP,
    side: usize,
}
impl PoolShip {
    /// # Safety
    /// The caller initialized and exclusively owns the fixture pools, and retains
    /// them until this ship and all references to it have dropped.
    pub unsafe fn new(player: SIZE) -> Self {
        let side = player as usize;
        assert!(side < 2);
        // SAFETY: the fixture owns the initialized queue and the newly allocated slot.
        unsafe {
            let pointer = AllocLink(&raw mut race_q[side]).cast::<STARSHIP>();
            assert!(!pointer.is_null(), "fixture ship pool exhausted");
            pointer.write(mem::zeroed());
            (*pointer).playerNr = player;
            Self { pointer, side }
        }
    }
}
impl Deref for PoolShip {
    type Target = STARSHIP;
    fn deref(&self) -> &STARSHIP {
        // SAFETY: construction retains this owned slot in the fixture's live pool.
        unsafe { &*self.pointer }
    }
}
impl DerefMut for PoolShip {
    fn deref_mut(&mut self) -> &mut STARSHIP {
        // SAFETY: this exclusive borrow is the only projection of the owned slot.
        unsafe { &mut *self.pointer }
    }
}
impl Drop for PoolShip {
    fn drop(&mut self) {
        // SAFETY: the construction contract retains the pool until this slot is released.
        unsafe {
            FreeLink(&raw mut race_q[self.side], self.pointer.cast());
        }
    }
}
