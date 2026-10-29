use super::super::link_queue::*;
#[cfg(not(test))]
use super::super::ShipId;
use crate::collections::queue::{self, HLink, Queue};
use std::{marker::PhantomData, ptr};
impl<'a, T: QueueRecord> LinkQueue<'a, T> {
    /// # Safety
    /// The initialized queue must contain ship records and be exclusively borrowed
    /// on the owning game thread. End the view before a legacy call can use it.
    pub unsafe fn new(queue: &'a mut Queue) -> Self {
        assert_eq!(usize::from(queue.object_size), size_of::<T::Storage>());
        Self {
            queue,
            marker: PhantomData,
        }
    }
    pub fn raw(&self, id: LinkId<T>) -> HLink {
        assert_eq!(
            id.pool, self.queue.pq_tab as usize,
            "ship link belongs to another pool"
        );
        assert!(id.slot < self.queue.num_objects, "ship link outside pool");
        let raw = self
            .queue
            .pq_tab
            .wrapping_add(usize::from(id.slot) * size_of::<T::Storage>())
            .cast();
        let mut free = self.queue.free_list;
        while !free.is_null() {
            assert_ne!(raw, free, "ship link addresses free storage");
            // SAFETY: queue initialization writes every free-list link before a view is created.
            free = unsafe { (*free.cast::<queue::Link>()).succ };
        }
        raw
    }
    pub(crate) fn record(&self, id: LinkId<T>) -> &T::Storage {
        // SAFETY: construction guarantees ship storage; raw checks pool membership.
        unsafe { &*self.raw(id).cast::<T::Storage>() }
    }
    pub(crate) fn record_mut(&mut self, id: LinkId<T>) -> &mut T::Storage {
        // SAFETY: this exclusive view scopes the checked one-record borrow.
        unsafe { &mut *self.raw(id).cast::<T::Storage>() }
    }
    pub fn alloc(&mut self) -> Option<LinkId<T>> {
        // SAFETY: the view exclusively owns this queue's initialized free list.
        let raw = unsafe { queue::AllocLink(self.queue) };
        let id = self.id(raw)?;
        // SAFETY: all fields of a newly allocated legacy ship admit zero.
        unsafe {
            ptr::write_bytes(raw.cast::<T::Storage>(), 0, 1);
        }
        Some(id)
    }
    pub fn put(&mut self, id: LinkId<T>) {
        assert!(!self.linked(id), "ship is already linked");
        let raw = self.raw(id);
        // SAFETY: callers append allocated unlinked records to this pool.
        unsafe {
            queue::PutQueue(self.queue, raw);
        }
    }
    pub fn remove(&mut self, id: LinkId<T>) {
        assert!(self.linked(id), "ship is not linked");
        let raw = self.raw(id);
        // SAFETY: callers unlink a currently linked record from this pool.
        unsafe {
            queue::RemoveQueue(self.queue, raw);
        }
    }
    pub fn reinit(&mut self) {
        // SAFETY: no record borrow outlives this exclusive view's operation.
        unsafe {
            queue::ReinitQueue(self.queue);
        }
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

pub(crate) const fn ship_size() -> usize {
    size_of::<<Starship as QueueRecord>::Storage>()
}

/// # Safety
/// The address is a live ship in a game-thread-owned battle pool, without a retained queue borrow.
#[cfg(not(test))]
pub(crate) unsafe fn identify(address: usize) -> ShipId {
    for side in 0..2 {
        // SAFETY: the queue headers outlive all battle-side records.
        let queue = unsafe { &*super::super::record::ship_queue(side).as_ptr() };
        if let Some(offset) = address.checked_sub(queue.pq_tab as usize) {
            let stride = usize::from(queue.object_size);
            if stride == ship_size()
                && offset.is_multiple_of(stride)
                && offset / stride < usize::from(queue.num_objects)
            {
                return ShipId {
                    side: side as u8,
                    slot: (offset / stride) as u8,
                };
            }
        }
    }
    panic!("ship address {address:#x} is outside the ship pools")
}
