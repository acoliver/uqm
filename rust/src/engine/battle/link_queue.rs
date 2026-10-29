//! Typed link order over the transitional battle ship pool.

use super::{record::STARSHIP, ShipId};
use crate::collections::queue::{self, HLink, Queue};
use std::{marker::PhantomData, ptr};

#[derive(Debug, PartialEq, Eq)]
pub enum Starship {}

#[derive(Debug, PartialEq, Eq)]
pub struct LinkId<T> {
    slot: u8,
    pool: usize,
    marker: PhantomData<T>,
}
impl<T> Copy for LinkId<T> {}
impl<T> Clone for LinkId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub struct LinkQueue<'a, T> {
    queue: &'a mut Queue,
    marker: PhantomData<T>,
}

/// Picker and fleet fields, independent of descriptor and element lifetimes.
#[derive(Clone, Copy, Debug)]
pub struct PickShip {
    pub species: u32,
    pub captain: u8,
    pub crew: u16,
    pub max_crew: u16,
    pub cost: u8,
    pub index: u16,
    pub strings: usize,
    pub icons: usize,
    pub energy_counter: u8,
    pub player: i16,
    pub control: u8,
}

impl<'a> LinkQueue<'a, Starship> {
    /// # Safety
    /// The initialized queue must contain ship records and be exclusively borrowed
    /// on the owning game thread. End the view before a legacy call can use it.
    pub unsafe fn new(queue: &'a mut Queue) -> Self {
        assert_eq!(usize::from(queue.object_size), size_of::<STARSHIP>());
        Self {
            queue,
            marker: PhantomData,
        }
    }

    pub fn id(&self, raw: HLink) -> Option<LinkId<Starship>> {
        if raw.is_null() {
            return None;
        }
        let stride = size_of::<STARSHIP>();
        let offset = (raw as usize)
            .checked_sub(self.queue.pq_tab as usize)
            .unwrap_or_else(|| panic!("ship link precedes pool"));
        assert!(
            offset.is_multiple_of(stride) && offset / stride < usize::from(self.queue.num_objects),
            "ship link outside pool"
        );
        let id = LinkId {
            slot: (offset / stride) as u8,
            pool: self.queue.pq_tab as usize,
            marker: PhantomData,
        };
        self.raw(id);
        Some(id)
    }

    pub fn raw(&self, id: LinkId<Starship>) -> HLink {
        assert_eq!(
            id.pool, self.queue.pq_tab as usize,
            "ship link belongs to another pool"
        );
        assert!(id.slot < self.queue.num_objects, "ship link outside pool");
        let raw = self
            .queue
            .pq_tab
            .wrapping_add(usize::from(id.slot) * size_of::<STARSHIP>())
            .cast();
        let mut free = self.queue.free_list;
        while !free.is_null() {
            assert_ne!(raw, free, "ship link addresses free storage");
            // SAFETY: queue initialization writes every free-list link before a view is created.
            free = unsafe { (*free.cast::<queue::Link>()).succ };
        }
        raw
    }
    fn record(&self, id: LinkId<Starship>) -> &STARSHIP {
        // SAFETY: construction guarantees ship storage; raw checks pool membership.
        unsafe { &*self.raw(id).cast::<STARSHIP>() }
    }
    fn record_mut(&mut self, id: LinkId<Starship>) -> &mut STARSHIP {
        // SAFETY: this exclusive view scopes the checked one-record borrow.
        unsafe { &mut *self.raw(id).cast::<STARSHIP>() }
    }
    pub fn head(&self) -> Option<LinkId<Starship>> {
        self.id(self.queue.head)
    }
    pub fn tail(&self) -> Option<LinkId<Starship>> {
        self.id(self.queue.tail)
    }
    pub fn succ(&self, id: LinkId<Starship>) -> Option<LinkId<Starship>> {
        self.id(self.record(id).succ)
    }
    pub fn pred(&self, id: LinkId<Starship>) -> Option<LinkId<Starship>> {
        self.id(self.record(id).pred)
    }
    fn linked(&self, needle: LinkId<Starship>) -> bool {
        let mut link = self.head();
        while let Some(id) = link {
            if id == needle {
                return true;
            }
            link = self.succ(id);
        }
        false
    }
    pub fn count(&self) -> usize {
        let mut count = 0;
        let mut link = self.head();
        while let Some(id) = link {
            count += 1;
            link = self.succ(id);
        }
        count
    }
    pub fn alloc(&mut self) -> Option<LinkId<Starship>> {
        // SAFETY: the view exclusively owns this queue's initialized free list.
        let raw = unsafe { queue::AllocLink(self.queue) };
        let id = self.id(raw)?;
        // SAFETY: all fields of a newly allocated legacy ship admit zero.
        unsafe {
            ptr::write_bytes(raw.cast::<STARSHIP>(), 0, 1);
        }
        Some(id)
    }
    pub fn put(&mut self, id: LinkId<Starship>) {
        assert!(!self.linked(id), "ship is already linked");
        let raw = self.raw(id);
        // SAFETY: callers append allocated unlinked records to this pool.
        unsafe {
            queue::PutQueue(self.queue, raw);
        }
    }
    pub fn remove(&mut self, id: LinkId<Starship>) {
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
    pub fn build(&mut self, species: u32) -> Option<LinkId<Starship>> {
        let id = self.alloc()?;
        self.record_mut(id).SpeciesID = species;
        self.put(id);
        Some(id)
    }
    pub fn get(&self, id: LinkId<Starship>) -> PickShip {
        let s = self.record(id);
        PickShip {
            species: s.SpeciesID,
            captain: s.captains_name_index,
            crew: s.crew_level,
            max_crew: s.max_crew,
            cost: s.ship_cost,
            index: s.index,
            strings: s.race_strings as usize,
            icons: s.icons as usize,
            energy_counter: s.energy_counter,
            player: s.playerNr,
            control: s.control,
        }
    }
    pub fn set(&mut self, id: LinkId<Starship>, value: PickShip) {
        let s = self.record_mut(id);
        s.SpeciesID = value.species;
        s.captains_name_index = value.captain;
        s.crew_level = value.crew;
        s.max_crew = value.max_crew;
        s.ship_cost = value.cost;
        s.index = value.index;
        s.race_strings = value.strings as _;
        s.icons = value.icons as _;
        s.energy_counter = value.energy_counter;
        s.playerNr = value.player;
        s.control = value.control;
    }
    pub fn clear_descriptor(&mut self, id: LinkId<Starship>) {
        self.record_mut(id).RaceDescPtr = ptr::null_mut();
    }
    pub fn ship_id(&self, id: LinkId<Starship>, side: u8) -> ShipId {
        self.raw(id);
        ShipId {
            side,
            slot: id.slot,
        }
    }
    pub fn link(&self, id: ShipId) -> LinkId<Starship> {
        let link = LinkId {
            slot: id.slot,
            pool: self.queue.pq_tab as usize,
            marker: PhantomData,
        };
        self.raw(link);
        link
    }
}

/// # Safety
/// Only the game thread may take a view, and it must end it before a legacy call.
#[cfg(not(test))]
pub(crate) unsafe fn ships<'a>(side: usize) -> LinkQueue<'a, Starship> {
    assert!(side < 2, "invalid ship side {side}");
    // SAFETY: callers serialize the address-stable battle ship authority.
    unsafe { LinkQueue::new(&mut *super::record::ship_queue(side).as_ptr()) }
}

pub(crate) const fn ship_size() -> usize {
    size_of::<STARSHIP>()
}

/// # Safety
/// The address is a live ship in a game-thread-owned battle pool, without a retained queue borrow.
#[cfg(not(test))]
pub(crate) unsafe fn identify(address: usize) -> ShipId {
    for side in 0..2 {
        // SAFETY: the queue headers outlive all battle-side records.
        let queue = unsafe { &*super::record::ship_queue(side).as_ptr() };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::queue::{InitQueue, Queue, UninitQueue};

    #[test]
    fn links_preserve_append_remove_reverse_order_and_exhaustion() {
        let mut queue = Queue::default();
        // SAFETY: the test exclusively owns this queue and its records.
        unsafe {
            assert!(InitQueue(
                &mut queue,
                3,
                size_of::<super::super::record::STARSHIP>() as u16
            ));
            {
                let mut ships = LinkQueue::<Starship>::new(&mut queue);
                assert_eq!(ships.count(), 0);
                let a = ships.alloc().unwrap();
                let b = ships.alloc().unwrap();
                let c = ships.alloc().unwrap();
                assert!(ships.alloc().is_none());
                ships.put(b);
                ships.put(a);
                ships.put(c);
                assert_eq!(ships.count(), 3);
                assert_eq!(ships.head(), Some(b));
                assert_eq!(ships.succ(b), Some(a));
                assert_eq!(ships.pred(c), Some(a));
                assert_eq!(ships.tail(), Some(c));
                ships.remove(a);
                assert_eq!(ships.succ(b), Some(c));
                assert_eq!(ships.pred(c), Some(b));
                assert_eq!(ships.count(), 2);
                ships.put(a);
                assert_eq!(ships.tail(), Some(a));
                assert_eq!(ships.id(ships.raw(a)), Some(a));
                ships.reinit();
                assert_eq!(ships.count(), 0);
                assert!(ships.head().is_none());
                assert!(ships.tail().is_none());
            }
            UninitQueue(&mut queue);
        }
    }

    #[test]
    fn build_sets_species_and_edits_picker_fields_without_touching_links() {
        let mut queue = Queue::default();
        // SAFETY: the test exclusively owns this queue and its records.
        unsafe {
            assert!(InitQueue(
                &mut queue,
                2,
                size_of::<super::super::record::STARSHIP>() as u16
            ));
            {
                let mut ships = LinkQueue::<Starship>::new(&mut queue);
                let a = ships.build(11).unwrap();
                let b = ships.build(2).unwrap();
                let mut ship = ships.get(a);
                assert_eq!(ship.species, 11);
                ship.index = 9;
                ship.crew = 7;
                ships.set(a, ship);
                assert_eq!(ships.get(a).index, 9);
                assert_eq!(ships.get(a).crew, 7);
                assert_eq!(ships.succ(a), Some(b));
                assert_eq!(ships.pred(b), Some(a));
            }
            UninitQueue(&mut queue);
        }
    }
    #[test]
    fn invalid_lifecycle_operations_fail_before_touching_links() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let mut queue = Queue::default();
        // SAFETY: this test owns the initialized pool and releases it after all views end.
        unsafe {
            assert!(InitQueue(&raw mut queue, 2, ship_size() as u16));
            {
                let mut ships = LinkQueue::<Starship>::new(&mut queue);
                let id = ships.alloc().unwrap();
                assert!(catch_unwind(AssertUnwindSafe(|| ships.remove(id))).is_err());
                ships.put(id);
                assert!(catch_unwind(AssertUnwindSafe(|| ships.put(id))).is_err());
                assert_eq!(ships.count(), 1);
                ships.remove(id);
                let raw = ships.raw(id);
                queue::FreeLink(ships.queue, raw);
                assert!(catch_unwind(AssertUnwindSafe(|| ships.get(id))).is_err());
                assert!(catch_unwind(AssertUnwindSafe(|| ships.id(raw))).is_err());
                assert!(ships.id(std::ptr::null_mut()).is_none());
                let replacement = ships.alloc().unwrap();
                assert_eq!(ships.get(replacement).species, 0);
                ships.clear_descriptor(replacement);
                let ship_id = ships.ship_id(replacement, 1);
                assert_eq!(ships.link(ship_id), replacement);
                ships.reinit();
                assert!(catch_unwind(AssertUnwindSafe(|| ships.raw(replacement))).is_err());
            }
            UninitQueue(&raw mut queue);
        }
    }
}
