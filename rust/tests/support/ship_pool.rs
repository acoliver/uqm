use std::ops::{Deref, DerefMut};
use uqm_rust::{
    engine::battle::link_queue::{LinkId, Starship},
    ships::native::abi::{SIZE, STARSHIP},
};

pub struct PoolShip {
    pointer: *mut STARSHIP,
    side: usize,
    link: LinkId<Starship>,
    pub ship_id: uqm_rust::engine::battle::ShipId,
}
impl PoolShip {
    /// # Safety
    /// The caller exclusively retains the serialized fixture pools throughout this borrow.
    pub unsafe fn new(player: SIZE) -> Self {
        let side = player as usize;
        // SAFETY: the constructor's contract retains the serialized published fixture.
        let (pointer, link, ship_id) = unsafe {
            uqm_rust::app::bridge::battle(|owner| {
                let mut ships = owner.ships(side);
                let link = ships
                    .alloc()
                    .unwrap_or_else(|| panic!("fixture ship pool exhausted"));
                ships.edit(link, |ship| ship.player = player);
                (
                    ships.raw(link).cast(),
                    link,
                    ships.ship_id(link, side as u8),
                )
            })
        };
        Self {
            pointer,
            side,
            link,
            ship_id,
        }
    }
}
impl Deref for PoolShip {
    type Target = STARSHIP;
    fn deref(&self) -> &STARSHIP {
        // SAFETY: the construction contract retains this owned fixture slot.
        unsafe { &*self.pointer }
    }
}
impl DerefMut for PoolShip {
    fn deref_mut(&mut self) -> &mut STARSHIP {
        // SAFETY: this exclusive borrow retains the fixture slot.
        unsafe { &mut *self.pointer }
    }
}
impl Drop for PoolShip {
    fn drop(&mut self) {
        // SAFETY: the fixture contract retains the published pool until this owner drops.
        unsafe {
            uqm_rust::app::bridge::battle(|owner| {
                let mut ships = owner.ships(self.side);
                if ships.contains(self.link) {
                    if ships.linked(self.link) {
                        ships.remove(self.link);
                    }
                    ships.free(self.link);
                }
            });
        }
    }
}
