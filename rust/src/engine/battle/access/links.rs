use super::super::link_queue::*;
#[cfg(not(test))]
use super::super::ShipId;
use crate::collections::queue::HLink;
use std::{marker::PhantomData, ptr};
impl<'a, T: QueueRecord> LinkQueue<'a, T> {
    pub fn new(queue: &'a mut crate::collections::pool::OrderedPool<T::Storage, 32>) -> Self {
        Self {
            queue: QueueBorrow::Exclusive(queue),
            marker: PhantomData,
        }
    }
    pub fn borrow(queue: &'a crate::collections::pool::OrderedPool<T::Storage, 32>) -> Self {
        Self {
            queue: QueueBorrow::Shared(queue),
            marker: PhantomData,
        }
    }
    pub fn raw(&self, id: LinkId<T>) -> HLink {
        ptr::from_ref(self.record(id)).cast_mut().cast()
    }
    pub(crate) fn record(&self, id: LinkId<T>) -> &T::Storage {
        self.queue.get(self.key(id))
    }
    pub(crate) fn record_mut(&mut self, id: LinkId<T>) -> &mut T::Storage {
        let key = self.key(id);
        self.queue.get_mut(key)
    }
    pub fn alloc(&mut self) -> Option<LinkId<T>> {
        let key = self.queue.alloc(T::Storage::default())?;
        Some(self.identify(key))
    }
    pub fn put(&mut self, id: LinkId<T>) {
        let key = self.key(id);
        self.queue.put(key);
    }
    pub fn remove(&mut self, id: LinkId<T>) {
        let key = self.key(id);
        self.queue.remove(key);
    }
    pub fn free(&mut self, id: LinkId<T>) {
        let key = self.key(id);
        self.queue.free(key);
    }
    pub fn reinit(&mut self) {
        self.queue.reinit();
    }
}
/// # Safety
/// Only the game thread may take a view, and it must end it before a legacy call.
#[cfg(not(test))]
pub(crate) unsafe fn ships<'a>(side: usize) -> LinkQueue<'a, Starship> {
    assert!(side < 2, "invalid ship side {side}");
    // SAFETY: callers serialize the address-stable battle ship authority.
    unsafe { LinkQueue::new(&mut *super::super::record::ship_queue(side).as_ptr()) }
}

/// # Safety
/// The address is a live ship in a game-thread-owned battle pool, without a retained queue borrow.
#[cfg(not(test))]
pub(crate) unsafe fn identify(address: usize) -> ShipId {
    for side in 0..2 {
        // SAFETY: callers retain the published game-thread Kernel throughout this lookup.
        let pool = unsafe { super::super::record::ship_queue(side).as_ref() };
        for slot in 0..32 {
            if let Some(id) = pool.id_at(slot) {
                if ptr::from_ref(pool.get(id)) as usize == address {
                    return ShipId {
                        side: side as u8,
                        slot: slot as u8,
                        gen: id.generation(),
                    };
                }
            }
        }
    }
    panic!("ship address {address:#x} is outside the ship pools")
}

impl LinkQueue<'_, Starship> {
    pub fn facing(&self, id: LinkId<Starship>) -> u16 {
        self.record(id).ShipFacing
    }
    pub(crate) fn element_handle(&self, id: LinkId<Starship>) -> Option<super::super::ElementId> {
        self.record(id).hShip
    }
}
