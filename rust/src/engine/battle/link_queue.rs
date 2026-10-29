//! Typed link order over the transitional battle ship pool.

use super::ShipId;
use crate::collections::queue::{HLink, Queue};
/// Storage mapping for typed views over the transitional intrusive queues.
mod queue_record {
    pub trait Sealed {}
    impl Sealed for super::Starship {}
}
pub trait QueueRecord: queue_record::Sealed + Eq {
    type Storage;
    fn links(record: &Self::Storage) -> (HLink, HLink);
}

use std::{marker::PhantomData, ptr};

#[derive(Debug, PartialEq, Eq)]
pub enum Starship {}

#[derive(Debug, PartialEq, Eq)]
pub struct LinkId<T> {
    pub(crate) slot: u8,
    pub(crate) pool: usize,
    pub(crate) marker: PhantomData<T>,
}
impl<T> Copy for LinkId<T> {}
impl<T> Clone for LinkId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub struct LinkQueue<'a, T> {
    pub(crate) queue: QueueBorrow<'a>,
    pub(crate) marker: PhantomData<T>,
}

pub(crate) enum QueueBorrow<'a> {
    Shared(&'a Queue),
    Exclusive(&'a mut Queue),
}
impl std::ops::Deref for QueueBorrow<'_> {
    type Target = Queue;
    fn deref(&self) -> &Queue {
        match self {
            Self::Shared(q) => q,
            Self::Exclusive(q) => q,
        }
    }
}
impl std::ops::DerefMut for QueueBorrow<'_> {
    fn deref_mut(&mut self) -> &mut Queue {
        match self {
            Self::Exclusive(q) => q,
            Self::Shared(_) => panic!("cannot mutate a read-only queue view"),
        }
    }
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

impl<'a, T: QueueRecord> LinkQueue<'a, T> {
    pub fn id(&self, raw: HLink) -> Option<LinkId<T>> {
        if raw.is_null() {
            return None;
        }
        let stride = size_of::<T::Storage>();
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

    pub fn head(&self) -> Option<LinkId<T>> {
        self.id(self.queue.head)
    }
    pub fn tail(&self) -> Option<LinkId<T>> {
        self.id(self.queue.tail)
    }
    pub fn succ(&self, id: LinkId<T>) -> Option<LinkId<T>> {
        self.id(T::links(self.record(id)).1)
    }
    pub fn pred(&self, id: LinkId<T>) -> Option<LinkId<T>> {
        self.id(T::links(self.record(id)).0)
    }
    pub(crate) fn linked(&self, needle: LinkId<T>) -> bool {
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
}

impl LinkQueue<'_, Starship> {
    pub fn by_index(&self, side: u8, mut index: u16, used: bool) -> Option<ShipId> {
        let mut link = self.head();
        while let Some(id) = link {
            let ship = self.get(id);
            if used {
                if ship.species != 0 {
                    if index == 0 {
                        return Some(self.ship_id(id, side));
                    }
                    index = index.wrapping_sub(1);
                }
            } else if ship.index == index {
                return if ship.species == 0 {
                    None
                } else {
                    Some(self.ship_id(id, side))
                };
            }
            link = self.succ(id);
        }
        None
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
    pub fn edit(&mut self, id: LinkId<Starship>, edit: impl FnOnce(&mut PickShip)) {
        let mut value = self.get(id);
        edit(&mut value);
        self.set(id, value);
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

pub(crate) use super::access::links::ship_size;
#[cfg(not(test))]
pub(crate) use super::access::links::{identify, ships};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::queue;
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
                queue::FreeLink(&mut *ships.queue, raw);
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
