//! Fixed-capacity element ownership and queue order on the game thread.

use crate::collections::pool::{Id, OrderedPool};
use crate::engine::battle::record::ElementRecord;
use std::ffi::c_void;
use std::ops::{Index, IndexMut};

const CAPACITY: usize = 150;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElementId {
    pub(super) slot: u8,
    pub(super) gen: u32,
    pub(super) owner: u64,
    token: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShipId {
    pub(super) side: u8,
    pub(super) slot: u8,
    pub(super) gen: u32,
    pub(super) owner: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parent {
    None,
    Ship(ShipId),
    IpGroup(super::link_queue::LinkId<crate::engine::world::queues::Group>),
}

pub struct ElementTable {
    records: Records,
}

impl ElementTable {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            records: Records {
                pool: OrderedPool::with_capacity(capacity),
            },
        }
    }

    fn identify(&self, id: Id<ElementRecord>) -> ElementId {
        ElementId {
            token: id.token(),
            slot: id.index() as u8,
            gen: id.generation(),
            owner: self.records.pool.storage_identity(),
        }
    }

    fn key(&self, id: ElementId) -> Id<ElementRecord> {
        let key = self.records.pool.id_at(usize::from(id.slot));
        assert!(self.contains(id), "stale element id {id:?}");
        key.unwrap_or_else(|| unreachable!("checked element allocation"))
    }

    pub fn id_at(&self, slot: usize) -> Option<ElementId> {
        self.records.pool.id_at(slot).map(|id| self.identify(id))
    }

    pub(super) fn pair_mut(
        &mut self,
        left: ElementId,
        right: ElementId,
    ) -> (&mut ElementRecord, &mut ElementRecord) {
        let left = self.key(left);
        let right = self.key(right);
        self.records.pool.get_pair_mut(left, right)
    }

    pub fn contains(&self, id: ElementId) -> bool {
        id.owner == self.records.pool.storage_identity()
            && self
                .records
                .pool
                .id_at(usize::from(id.slot))
                .is_some_and(|key| key.generation() == id.gen)
    }

    pub fn raw(&self, id: ElementId) -> *mut c_void {
        std::ptr::from_ref(self.records.pool.get(self.key(id)))
            .cast_mut()
            .cast()
    }

    pub fn id(&self, address: *mut c_void) -> Option<ElementId> {
        if address.is_null() {
            return None;
        }
        let id = self.allocated_id(address);
        assert!(
            id.is_some(),
            "element address is not allocated: {address:p}"
        );
        id
    }

    pub(crate) fn allocated_id(&self, address: *mut c_void) -> Option<ElementId> {
        (0..CAPACITY).find_map(|index| {
            let key = self.records.pool.id_at(index)?;
            (std::ptr::from_ref(self.records.pool.get(key)).cast::<()>() == address.cast::<()>())
                .then_some(self.identify(key))
        })
    }

    pub(crate) fn journal_address(&self, id: ElementId) -> usize {
        assert_eq!(
            id.owner,
            self.records.pool.storage_identity(),
            "foreign element owner"
        );
        self.records.pool.retired_address(usize::from(id.slot))
    }

    pub fn head(&self) -> Option<ElementId> {
        self.records.pool.head().map(|id| self.identify(id))
    }
    pub fn tail(&self) -> Option<ElementId> {
        self.records.pool.tail().map(|id| self.identify(id))
    }
    pub fn succ(&self, id: ElementId) -> Option<ElementId> {
        self.records
            .pool
            .succ(self.key(id))
            .map(|id| self.identify(id))
    }
    pub fn pred(&self, id: ElementId) -> Option<ElementId> {
        self.records
            .pool
            .pred(self.key(id))
            .map(|id| self.identify(id))
    }
    pub fn linked(&self, id: ElementId) -> bool {
        self.records.pool.linked(self.key(id))
    }
    pub fn alloc(&mut self) -> Option<ElementId> {
        let key = self.records.pool.alloc(ElementRecord::default())?;
        let id = self.identify(key);
        Some(id)
    }
    pub fn free(&mut self, id: ElementId) {
        self.records.pool.free(self.key(id));
    }
    pub fn put(&mut self, id: ElementId) {
        self.insert_before(id, None);
    }
    pub fn insert_before(&mut self, id: ElementId, before: Option<ElementId>) {
        let key = self.key(id);
        let before = before.map(|other| self.key(other));
        self.records.pool.insert_before(key, before);
    }
    pub fn remove(&mut self, id: ElementId) {
        self.records.pool.remove(self.key(id));
    }
    pub fn reset_capacity(&mut self, capacity: usize) {
        self.records.pool.reset_capacity(capacity);
    }
    pub fn reinit(&mut self) {
        self.records.pool.reinit();
    }
}

impl Index<ElementId> for ElementTable {
    type Output = ElementRecord;
    fn index(&self, id: ElementId) -> &ElementRecord {
        self.records.pool.get(self.key(id))
    }
}
pub struct Records {
    pool: OrderedPool<ElementRecord, CAPACITY>,
}
impl Index<ElementId> for Records {
    type Output = ElementRecord;
    fn index(&self, id: ElementId) -> &ElementRecord {
        self.pool.get(self.key(id))
    }
}
impl Records {
    fn key(&self, id: ElementId) -> Id<ElementRecord> {
        assert_eq!(
            id.owner,
            self.pool.storage_identity(),
            "element belongs to another pool"
        );
        let key = self
            .pool
            .id_at(usize::from(id.slot))
            .unwrap_or_else(|| panic!("free element {id:?}"));
        assert_eq!(id.gen, key.generation(), "stale element {id:?}");
        key
    }
    #[cfg(not(test))]
    pub(crate) fn reinit(&mut self) {
        self.pool.reinit();
    }
}
impl IndexMut<ElementId> for Records {
    fn index_mut(&mut self, id: ElementId) -> &mut ElementRecord {
        let key = self.key(id);
        self.pool.get_mut(key)
    }
}
impl ElementTable {
    #[cfg(not(test))]
    pub(super) fn records_mut(&mut self) -> &mut Records {
        &mut self.records
    }
    pub(super) fn record_mut(&mut self, id: ElementId) -> &mut ElementRecord {
        &mut self.records[id]
    }
}
impl ElementId {
    pub fn slot(self) -> u8 {
        self.slot
    }
    pub fn generation(self) -> u32 {
        self.gen
    }
}
impl ShipId {
    pub fn side(self) -> u8 {
        self.side
    }
    pub fn slot(self) -> u8 {
        self.slot
    }
    pub fn generation(self) -> u32 {
        self.gen
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::battle::ElementTable;

    #[test]
    fn paired_elements_mutate_the_checked_owner_in_argument_order() {
        let mut elements = ElementTable::with_capacity(2);
        let a = elements.alloc().unwrap();
        let b = elements.alloc().unwrap();
        let (right, left) = elements.pair_mut(b, a);
        right.hits = 7;
        left.hits = 9;
        assert_eq!((elements[a].hits, elements[b].hits), (9, 7));
    }

    #[test]
    fn element_order_keeps_slot_then_generation_after_reuse() {
        let mut elements = ElementTable::with_capacity(2);
        let first = elements.alloc().unwrap();
        let second = elements.alloc().unwrap();
        elements.free(first);
        let replacement = elements.alloc().unwrap();
        assert!(first < replacement);
        assert!(replacement < second);
    }

    #[test]
    fn retired_observation_addresses_survive_retirement_and_slot_reuse() {
        let mut elements = ElementTable::with_capacity(1);
        let retired = elements.alloc().unwrap();
        let address = elements.raw(retired) as usize;
        elements.free(retired);
        assert_eq!(elements.journal_address(retired), address);
        let replacement = elements.alloc().unwrap();
        assert_ne!(retired, replacement);
        assert_eq!(elements.journal_address(retired), address);
        assert_eq!(elements.journal_address(replacement), address);
        elements.reinit();
        assert_eq!(elements.journal_address(replacement), address);
    }
}

impl ElementId {
    pub(crate) fn sound_token(self) -> usize {
        usize::try_from(self.token)
            .unwrap_or_else(|_| panic!("element token exceeds native handle"))
    }
    #[cfg(test)]
    pub(crate) fn fixture(slot: u8, gen: u32) -> Self {
        Self {
            slot,
            gen,
            owner: 0,
            token: (u64::from(gen) << 8 | u64::from(slot)) + 1,
        }
    }
}
impl ElementTable {
    pub(crate) fn sound_source(&self, token: usize) -> Option<ElementId> {
        (0..CAPACITY)
            .filter_map(|slot| self.id_at(slot))
            .find(|id| id.sound_token() == token)
    }
    #[cfg(test)]
    pub(crate) fn fixture_record_mut(&mut self, id: ElementId) -> &mut ElementRecord {
        self.record_mut(id)
    }
}
impl ShipId {
    #[cfg(test)]
    pub(crate) fn fixture(side: u8, slot: u8, gen: u32) -> Self {
        Self {
            side,
            slot,
            gen,
            owner: 0,
        }
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;
    #[test]
    fn foreign_element_tables_never_accept_matching_slot_generations() {
        let mut a = ElementTable::with_capacity(1);
        let mut b = ElementTable::with_capacity(1);
        let foreign = a.alloc().unwrap();
        let local = b.alloc().unwrap();
        assert_eq!(foreign.slot(), local.slot());
        assert_eq!(foreign.generation(), local.generation());
        assert_ne!(foreign, local);
        assert!(!b.contains(foreign));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| &b[foreign])).is_err());
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || b.journal_address(foreign)
        ))
        .is_err());
    }
}
