//! Fixed-capacity element ownership and queue order on the game thread.

use super::record::ELEMENT;
use crate::collections::pool::{Id, OrderedPool};
use crate::collections::queue::HLink;
use std::ops::{Index, IndexMut};

const CAPACITY: usize = 150;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElementId {
    pub slot: u8,
    pub gen: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShipId {
    pub side: u8,
    pub slot: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parent {
    None,
    Ship(ShipId),
    IpGroup(u8),
}

pub struct ElementTable {
    pool: OrderedPool<ELEMENT, CAPACITY>,
    // The journal can name a retired allocation without borrowing its former value.
    journal_addresses: [usize; CAPACITY],
}

impl ElementTable {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            pool: OrderedPool::with_capacity(capacity),
            journal_addresses: [0; CAPACITY],
        }
    }

    fn identify(id: Id<ELEMENT>) -> ElementId {
        ElementId {
            slot: id.index() as u8,
            gen: id.generation(),
        }
    }

    fn key(&self, id: ElementId) -> Id<ELEMENT> {
        let key = self.pool.id_at(usize::from(id.slot));
        assert!(self.contains(id), "stale element id {id:?}");
        key.unwrap_or_else(|| unreachable!("checked element allocation"))
    }

    pub fn id_at(&self, slot: usize) -> Option<ElementId> {
        self.pool.id_at(slot).map(Self::identify)
    }

    pub fn contains(&self, id: ElementId) -> bool {
        self.pool
            .id_at(usize::from(id.slot))
            .is_some_and(|key| key.generation() == id.gen)
    }

    pub fn raw(&self, id: ElementId) -> HLink {
        std::ptr::from_ref(self.pool.get(self.key(id)))
            .cast_mut()
            .cast()
    }

    pub fn id(&self, address: HLink) -> Option<ElementId> {
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

    pub(crate) fn allocated_id(&self, address: HLink) -> Option<ElementId> {
        (0..CAPACITY).find_map(|index| {
            let key = self.pool.id_at(index)?;
            (std::ptr::from_ref(self.pool.get(key)).cast::<()>() == address.cast::<()>())
                .then_some(Self::identify(key))
        })
    }

    pub(crate) fn journal_address(&self, id: ElementId) -> usize {
        let address = self.journal_addresses[usize::from(id.slot)];
        assert_ne!(address, 0, "element slot has never been allocated: {id:?}");
        address
    }

    pub fn head(&self) -> Option<ElementId> {
        self.pool.head().map(Self::identify)
    }
    pub fn tail(&self) -> Option<ElementId> {
        self.pool.tail().map(Self::identify)
    }
    pub fn succ(&self, id: ElementId) -> Option<ElementId> {
        self.pool.succ(self.key(id)).map(Self::identify)
    }
    pub fn pred(&self, id: ElementId) -> Option<ElementId> {
        self.pool.pred(self.key(id)).map(Self::identify)
    }
    pub fn linked(&self, id: ElementId) -> bool {
        self.pool.linked(self.key(id))
    }
    pub fn alloc(&mut self) -> Option<ElementId> {
        let key = self.pool.alloc(ELEMENT::default())?;
        let id = Self::identify(key);
        self.journal_addresses[key.index()] = self.raw(id) as usize;
        Some(id)
    }
    pub fn free(&mut self, id: ElementId) {
        self.pool.free(self.key(id));
    }
    pub fn put(&mut self, id: ElementId) {
        self.insert_before(id, None);
    }
    pub fn insert_before(&mut self, id: ElementId, before: Option<ElementId>) {
        let key = self.key(id);
        let before = before.map(|other| self.key(other));
        self.pool.insert_before(key, before);
    }
    pub fn remove(&mut self, id: ElementId) {
        self.pool.remove(self.key(id));
    }
    pub fn reset_capacity(&mut self, capacity: usize) {
        self.pool.reset_capacity(capacity);
    }
    pub fn reinit(&mut self) {
        self.pool.reinit();
    }
}

impl Index<ElementId> for ElementTable {
    type Output = ELEMENT;
    fn index(&self, id: ElementId) -> &ELEMENT {
        self.pool.get(self.key(id))
    }
}
impl IndexMut<ElementId> for ElementTable {
    fn index_mut(&mut self, id: ElementId) -> &mut ELEMENT {
        let key = self.key(id);
        self.pool.get_mut(key)
    }
}

#[cfg(test)]
mod tests {
    use super::ElementTable;

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
