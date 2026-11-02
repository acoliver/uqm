//! Generational identities over the one, address-stable legacy element pool.

use super::record::{ELEMENT, QUEUE};
use crate::collections::queue::{AllocLink, FreeLink, HLink, ReinitQueue};
use std::{
    ops::{Index, IndexMut},
    ptr::{self, NonNull},
};

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
    queue: NonNull<QUEUE>,
    generations: [u32; 256],
    allocated: [bool; 256],
}

impl ElementTable {
    /// # Safety
    /// The queue remains live on the owning game thread. Initialize its fixed pool
    /// before record access, and finish all access before cleaning the pool up.
    /// No record reference may cross a call into legacy code.
    pub unsafe fn from_queue(queue: *mut QUEUE) -> Self {
        Self {
            queue: NonNull::new(queue).unwrap_or_else(|| panic!("null element queue")),
            generations: [0; 256],
            allocated: [false; 256],
        }
    }

    pub(super) fn queue(&self) -> *mut QUEUE {
        self.queue.as_ptr()
    }

    #[inline]
    fn slot(&self, handle: HLink) -> usize {
        // SAFETY: the construction contract keeps the queue and backing pool live.
        let queue = unsafe { &*self.queue() };
        assert!(!queue.pq_tab.is_null(), "element pool is not initialized");
        let size = usize::from(queue.object_size);
        assert_eq!(size, size_of::<ELEMENT>(), "wrong element record stride");
        let offset = (handle as usize)
            .checked_sub(queue.pq_tab as usize)
            .unwrap_or_else(|| panic!("element handle precedes pool"));
        assert!(
            offset.is_multiple_of(size) && offset / size < usize::from(queue.num_objects),
            "element handle outside pool"
        );
        offset / size
    }

    #[inline]
    pub fn id(&self, handle: HLink) -> Option<ElementId> {
        if handle.is_null() {
            return None;
        }
        let slot = self.slot(handle);
        assert!(
            self.allocated[slot],
            "element handle addresses free slot {slot}"
        );
        Some(ElementId {
            slot: slot as u8,
            gen: self.generations[slot],
        })
    }

    /// Legacy bookkeeping can retain an address after its allocation retires.
    /// Foreign addresses still fail validation; a retired slot has no current id.
    pub(crate) fn allocated_id(&self, handle: HLink) -> Option<ElementId> {
        if handle.is_null() {
            return None;
        }
        let slot = self.slot(handle);
        self.allocated[slot].then_some(ElementId {
            slot: slot as u8,
            gen: self.generations[slot],
        })
    }

    pub fn contains(&self, id: ElementId) -> bool {
        let slot = usize::from(id.slot);
        self.allocated[slot] && self.generations[slot] == id.gen
    }

    #[inline]
    pub fn raw(&self, id: ElementId) -> HLink {
        let slot = usize::from(id.slot);
        assert!(
            self.allocated[slot] && self.generations[slot] == id.gen,
            "stale element id {id:?}"
        );
        // SAFETY: slot membership was checked when the id was created. This pointer
        // derives from the pool address, never from a reference to a record.
        unsafe {
            (*self.queue())
                .pq_tab
                .add(slot * usize::from((*self.queue()).object_size))
                .cast()
        }
    }

    #[inline]
    pub fn head(&self) -> Option<ElementId> {
        // SAFETY: queue access is game-thread exclusive.
        self.id(unsafe { (*self.queue()).head })
    }

    #[inline]
    pub fn succ(&self, id: ElementId) -> Option<ElementId> {
        self.id(self[id].succ)
    }

    pub fn alloc(&mut self) -> Option<ElementId> {
        // SAFETY: this table owns allocation of the legacy queue.
        let handle = unsafe { AllocLink(self.queue()) };
        if handle.is_null() {
            return None;
        }
        let slot = self.slot(handle);
        assert!(!self.allocated[slot], "allocated slot was already live");
        self.allocated[slot] = true;
        // SAFETY: ELEMENT's nullable hooks, pointers, integers and unions admit zero.
        unsafe {
            ptr::write_bytes(handle.cast::<ELEMENT>(), 0, 1);
        }
        self.id(handle)
    }

    pub(crate) fn free(&mut self, id: ElementId) {
        let handle = self.raw(id);
        let slot = usize::from(id.slot);
        self.allocated[slot] = false;
        self.bump(slot);
        // SAFETY: the caller unlinked this allocated element before freeing it.
        unsafe {
            FreeLink(self.queue(), handle);
        }
    }

    fn bump(&mut self, slot: usize) {
        self.generations[slot] = self.generations[slot]
            .checked_add(1)
            .unwrap_or_else(|| panic!("element generation exhausted at slot {slot}"));
    }

    pub fn reinit(&mut self) {
        for slot in 0..self.generations.len() {
            self.bump(slot);
        }
        self.allocated.fill(false);
        // SAFETY: reset is called at the existing mode-entry queue reset point.
        unsafe {
            ReinitQueue(self.queue());
        }
    }
}

impl Index<ElementId> for ElementTable {
    type Output = ELEMENT;
    #[inline]
    fn index(&self, id: ElementId) -> &ELEMENT {
        // SAFETY: raw validates generation and membership; reference is scoped to this borrow.
        unsafe { &*self.raw(id).cast::<ELEMENT>() }
    }
}
impl IndexMut<ElementId> for ElementTable {
    #[inline]
    fn index_mut(&mut self, id: ElementId) -> &mut ELEMENT {
        // SAFETY: the exclusive table borrow scopes this one-record reference.
        unsafe { &mut *self.raw(id).cast::<ELEMENT>() }
    }
}

#[cfg(test)]
mod tests {
    use super::super::record::{ELEMENT, QUEUE};
    use super::*;
    use crate::collections::queue::{InitQueue, PutQueue, UninitQueue};

    #[test]
    fn allocation_order_reuse_and_reset_invalidate_old_ids() {
        let mut queue = QUEUE::default();
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        unsafe {
            assert!(InitQueue(&mut queue, 3, size_of::<ELEMENT>() as u16));
            let mut elements = ElementTable::from_queue(&raw mut queue);
            let a = elements.alloc().unwrap();
            let b = elements.alloc().unwrap();
            let c = elements.alloc().unwrap();
            assert_eq!([a.slot, b.slot, c.slot], [0, 1, 2]);
            assert!(elements.alloc().is_none());
            assert_eq!(elements.id(elements.raw(b)), Some(b));
            let retained = elements.raw(b);
            assert_eq!(elements.allocated_id(retained), Some(b));
            elements.free(b);
            assert_eq!(elements.allocated_id(retained), None);
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || elements.id(retained)
            ))
            .is_err());
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || elements.allocated_id(std::ptr::dangling_mut())
            ))
            .is_err());
            let replacement = elements.alloc().unwrap();
            assert_eq!(elements.allocated_id(retained), Some(replacement));
            assert_eq!(b.slot, replacement.slot);
            assert_ne!(b, replacement);
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || elements[b].state_flags
            ))
            .is_err());
            elements.reinit();
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || elements[a].state_flags
            ))
            .is_err());
            assert_ne!(elements.alloc().unwrap(), a);
            UninitQueue(&mut queue);
        }
    }

    #[test]
    fn typed_parents_preserve_ship_and_group_identity_without_decoding_addresses() {
        let mut queue = QUEUE::default();
        // SAFETY: the test exclusively retains the initialized element authority.
        unsafe {
            assert!(InitQueue(&mut queue, 1, size_of::<ELEMENT>() as u16));
            let mut elements = ElementTable::from_queue(&raw mut queue);
            let id = elements.alloc().unwrap();
            assert_eq!(elements[id].parent, Parent::None);
            let ship = ShipId { side: 1, slot: 1 };
            elements[id].parent = Parent::Ship(ship);
            assert_eq!(elements[id].parent, Parent::Ship(ship));
            elements[id].parent = Parent::IpGroup(0);
            assert_eq!(elements[id].parent, Parent::IpGroup(0));
            UninitQueue(&mut queue);
        }
    }

    #[test]
    fn successor_reads_see_elements_appended_during_the_pass() {
        let mut queue = QUEUE::default();
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        unsafe {
            assert!(InitQueue(&mut queue, 3, size_of::<ELEMENT>() as u16));
            let mut elements = ElementTable::from_queue(&raw mut queue);
            let a = elements.alloc().unwrap();
            PutQueue(&mut queue, elements.raw(a));
            assert_eq!(elements.head(), Some(a));
            assert_eq!(elements.succ(a), None);
            let b = elements.alloc().unwrap();
            PutQueue(&mut queue, elements.raw(b));
            assert_eq!(elements.succ(a), Some(b));
            elements[b].state_flags = 123;
            assert_eq!((*elements.raw(b).cast::<ELEMENT>()).state_flags, 123);
            UninitQueue(&mut queue);
        }
    }
}
