//! Fixed-capacity storage with the allocation and visit order of QUEUE_TABLE.
use std::{
    fmt,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

type SlotIx = u16;

pub struct Id<T> {
    ix: SlotIx,
    generation: u32,
    marker: PhantomData<fn() -> T>,
}
impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.ix, self.generation) == (other.ix, other.generation)
    }
}
impl<T> Eq for Id<T> {}
impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.ix, self.generation).cmp(&(other.ix, other.generation))
    }
}
impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ix.hash(state);
        self.generation.hash(state);
    }
}
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("ix", &self.ix)
            .field("gen", &self.generation)
            .finish()
    }
}
impl<T> Id<T> {
    pub fn index(self) -> usize {
        usize::from(self.ix)
    }
}

struct Slot<T> {
    pred: Option<SlotIx>,
    succ: Option<SlotIx>,
    generation: u32,
    value: Option<T>,
    linked: bool,
}

pub struct OrderedPool<T, const N: usize> {
    slots: Box<[Slot<T>; N]>,
    head: Option<SlotIx>,
    tail: Option<SlotIx>,
    free: Option<SlotIx>,
    count: usize,
}
impl<T, const N: usize> Default for OrderedPool<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T, const N: usize> OrderedPool<T, N> {
    pub fn new() -> Self {
        assert!(
            N <= usize::from(SlotIx::MAX),
            "pool capacity exceeds slot index"
        );
        let slots = (0..N)
            .map(|ix| Slot {
                pred: None,
                succ: (ix + 1 < N).then_some((ix + 1) as SlotIx),
                generation: 0,
                value: None,
                linked: false,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| unreachable!("pool capacity"));
        Self {
            slots,
            head: None,
            tail: None,
            free: (N != 0).then_some(0),
            count: 0,
        }
    }

    fn id(&self, ix: SlotIx) -> Id<T> {
        Id {
            ix,
            generation: self.slots[usize::from(ix)].generation,
            marker: PhantomData,
        }
    }
    fn slot(&self, id: Id<T>) -> &Slot<T> {
        assert!(self.contains(id), "stale pool id {id:?}");
        &self.slots[id.index()]
    }
    fn slot_mut(&mut self, id: Id<T>) -> &mut Slot<T> {
        assert!(self.contains(id), "stale pool id {id:?}");
        &mut self.slots[id.index()]
    }
    pub fn contains(&self, id: Id<T>) -> bool {
        self.slots
            .get(id.index())
            .is_some_and(|s| s.generation == id.generation && s.value.is_some())
    }
    pub fn get(&self, id: Id<T>) -> &T {
        self.slot(id)
            .value
            .as_ref()
            .unwrap_or_else(|| unreachable!("allocated pool slot"))
    }
    pub fn get_mut(&mut self, id: Id<T>) -> &mut T {
        self.slot_mut(id)
            .value
            .as_mut()
            .unwrap_or_else(|| unreachable!("allocated pool slot"))
    }
    pub fn alloc(&mut self, value: T) -> Option<Id<T>> {
        let ix = self.free?;
        let slot = &mut self.slots[usize::from(ix)];
        self.free = slot.succ;
        slot.pred = None;
        slot.succ = None;
        slot.value = Some(value);
        Some(self.id(ix))
    }
    pub fn free(&mut self, id: Id<T>) -> T {
        assert!(!self.slot(id).linked, "cannot free linked slot {id:?}");
        let free = self.free;
        let slot = self.slot_mut(id);
        slot.generation = slot
            .generation
            .checked_add(1)
            .unwrap_or_else(|| panic!("pool generation exhausted"));
        slot.succ = free;
        let value = slot
            .value
            .take()
            .unwrap_or_else(|| unreachable!("allocated pool slot"));
        self.free = Some(id.ix);
        value
    }
    pub fn head(&self) -> Option<Id<T>> {
        self.head.map(|ix| self.id(ix))
    }
    pub fn tail(&self) -> Option<Id<T>> {
        self.tail.map(|ix| self.id(ix))
    }
    pub fn succ(&self, id: Id<T>) -> Option<Id<T>> {
        self.slot(id).succ.map(|ix| self.id(ix))
    }
    pub fn pred(&self, id: Id<T>) -> Option<Id<T>> {
        self.slot(id).pred.map(|ix| self.id(ix))
    }
    pub fn linked(&self, id: Id<T>) -> bool {
        self.slot(id).linked
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn put(&mut self, id: Id<T>) {
        self.insert_before(id, None);
    }
    pub fn insert_before(&mut self, id: Id<T>, before: Option<Id<T>>) {
        assert!(!self.slot(id).linked, "slot already linked {id:?}");
        let pred = match before {
            Some(other) => {
                let slot = self.slot(other);
                assert!(slot.linked, "insertion reference is not linked {other:?}");
                slot.pred
            }
            None => self.tail,
        };
        if let Some(ix) = pred {
            self.slots[usize::from(ix)].succ = Some(id.ix);
        } else {
            self.head = Some(id.ix);
        }
        if let Some(other) = before {
            self.slots[other.index()].pred = Some(id.ix);
        } else {
            self.tail = Some(id.ix);
        }
        let slot = self.slot_mut(id);
        slot.pred = pred;
        slot.succ = before.map(|other| other.ix);
        slot.linked = true;
        self.count += 1;
    }
    pub fn remove(&mut self, id: Id<T>) {
        let slot = self.slot(id);
        assert!(slot.linked, "slot is not linked {id:?}");
        let (pred, succ) = (slot.pred, slot.succ);
        if let Some(ix) = pred {
            self.slots[usize::from(ix)].succ = succ;
        } else {
            self.head = succ;
        }
        if let Some(ix) = succ {
            self.slots[usize::from(ix)].pred = pred;
        } else {
            self.tail = pred;
        }
        // RemoveQueue retains these links; callers may still read the successor.
        self.slots[id.index()].linked = false;
        self.count -= 1;
    }
    pub fn reinit(&mut self) {
        for (ix, slot) in self.slots.iter_mut().enumerate() {
            slot.generation = slot
                .generation
                .checked_add(1)
                .unwrap_or_else(|| panic!("pool generation exhausted"));
            slot.value = None;
            slot.pred = None;
            slot.succ = (ix + 1 < N).then_some((ix + 1) as SlotIx);
            slot.linked = false;
        }
        self.head = None;
        self.tail = None;
        self.free = (N != 0).then_some(0);
        self.count = 0;
    }
}
impl<T, const N: usize> Index<Id<T>> for OrderedPool<T, N> {
    type Output = T;
    fn index(&self, id: Id<T>) -> &T {
        self.get(id)
    }
}
impl<T, const N: usize> IndexMut<Id<T>> for OrderedPool<T, N> {
    fn index_mut(&mut self, id: Id<T>) -> &mut T {
        self.get_mut(id)
    }
}
