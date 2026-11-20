//! Generational slot storage for graphics objects. A handle names one live
//! value; using it after the value is removed is a bug and panics.
use std::marker::PhantomData;

/// A copyable reference to a value in an [`Arena`].
pub struct Handle<T> {
    index: u32,
    generation: u32,
    marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    /// A stable number for diagnostics and replay journals, unique among
    /// live and retired handles of the same arena.
    pub fn key(self) -> u64 {
        u64::from(self.generation) << 32 | u64::from(self.index)
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Handle<T> {}
impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}
impl<T> Eq for Handle<T> {}
impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}
impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}
impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}.{}", self.index, self.generation)
    }
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// Values addressed by [`Handle`]s. Freed slots are reused with a new
/// generation, so a stale handle never reaches the new value.
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    pub fn insert(&mut self, value: T) -> Handle<T> {
        let index = if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.generation = slot.generation.wrapping_add(1);
            slot.value = Some(value);
            index
        } else {
            self.slots.push(Slot {
                generation: 0,
                value: Some(value),
            });
            u32::try_from(self.slots.len() - 1).unwrap_or_else(|_| panic!("arena is full"))
        };
        Handle {
            index,
            generation: self.slots[index as usize].generation,
            marker: PhantomData,
        }
    }

    pub fn remove(&mut self, handle: Handle<T>) -> T {
        let value = self
            .slot_mut(handle)
            .take()
            .unwrap_or_else(|| panic!("graphics handle {handle:?} was already released"));
        self.free.push(handle.index);
        value
    }

    pub fn contains(&self, handle: Handle<T>) -> bool {
        self.slots
            .get(handle.index as usize)
            .is_some_and(|slot| slot.generation == handle.generation && slot.value.is_some())
    }

    pub fn get(&self, handle: Handle<T>) -> &T {
        let slot = self
            .slots
            .get(handle.index as usize)
            .filter(|slot| slot.generation == handle.generation);
        slot.and_then(|slot| slot.value.as_ref())
            .unwrap_or_else(|| panic!("stale graphics handle {handle:?}"))
    }

    pub fn get_mut(&mut self, handle: Handle<T>) -> &mut T {
        self.slot_mut(handle)
            .as_mut()
            .unwrap_or_else(|| panic!("stale graphics handle {handle:?}"))
    }

    fn slot_mut(&mut self, handle: Handle<T>) -> &mut Option<T> {
        let slot = self
            .slots
            .get_mut(handle.index as usize)
            .filter(|slot| slot.generation == handle.generation)
            .unwrap_or_else(|| panic!("stale graphics handle {handle:?}"));
        &mut slot.value
    }

    /// Two distinct values at once.
    pub fn pair_mut(&mut self, a: Handle<T>, b: Handle<T>) -> (&mut T, &mut T) {
        assert_ne!(a, b, "graphics handle {a:?} borrowed twice");
        let (low, high, swapped) = if a.index < b.index {
            (a, b, false)
        } else {
            (b, a, true)
        };
        let (head, tail) = self.slots.split_at_mut(high.index as usize);
        let first = &mut head[low.index as usize];
        let second = &mut tail[0];
        assert!(
            first.generation == low.generation && second.generation == high.generation,
            "stale graphics handle"
        );
        let first = first
            .value
            .as_mut()
            .unwrap_or_else(|| panic!("stale graphics handle {low:?}"));
        let second = second
            .value
            .as_mut()
            .unwrap_or_else(|| panic!("stale graphics handle {high:?}"));
        if swapped {
            (second, first)
        } else {
            (first, second)
        }
    }

    pub fn len(&self) -> usize {
        self.slots.len() - self.free.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.value.as_ref().map(|value| {
                (
                    Handle {
                        index: index as u32,
                        generation: slot.generation,
                        marker: PhantomData,
                    },
                    value,
                )
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reused_slots_reject_stale_handles() {
        let mut arena = Arena::new();
        let first = arena.insert(1);
        assert_eq!(arena.remove(first), 1);
        let second = arena.insert(2);
        assert!(!arena.contains(first));
        assert_eq!(*arena.get(second), 2);
        assert_ne!(first.key(), second.key());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| *arena.get(first)));
        assert!(result.is_err());
    }

    #[test]
    fn pairs_borrow_two_values_in_argument_order() {
        let mut arena = Arena::new();
        let a = arena.insert(1);
        let b = arena.insert(2);
        let (y, x) = arena.pair_mut(b, a);
        *x += 10;
        *y += 20;
        assert_eq!((*arena.get(a), *arena.get(b)), (11, 22));
        assert_eq!(arena.len(), 2);
    }
}
