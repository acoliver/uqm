//! Display primitive storage and frame-local drawing order.

use crate::collections::pool::{Id, OrderedPool};
mod primitive;
pub use primitive::DisplayPrimitive;

const CAPACITY: usize = 330;

pub struct DisplayPool {
    primitives: OrderedPool<DisplayPrimitive, CAPACITY>,
    // FreeLink used to retain every field except its links. Keep inactive values,
    // moving each one back into the pool when its slot is allocated again.
    retired: Box<[Option<DisplayPrimitive>; CAPACITY]>,
}
impl DisplayPool {
    pub fn new() -> Self {
        Self {
            primitives: OrderedPool::new(),
            retired: Box::new(std::array::from_fn(|_| None)),
        }
    }
    fn id(&self, index: u16) -> Id<DisplayPrimitive> {
        self.primitives
            .id_at(usize::from(index))
            .unwrap_or_else(|| panic!("unallocated display primitive {index}"))
    }
    pub fn get(&self, index: u16) -> &DisplayPrimitive {
        self.primitives.get(self.id(index))
    }
    pub fn get_mut(&mut self, index: u16) -> &mut DisplayPrimitive {
        let id = self.id(index);
        self.primitives.get_mut(id)
    }
    pub fn alloc(&mut self) -> Option<u16> {
        let id = self.primitives.alloc(DisplayPrimitive::default())?;
        if let Some(value) = self.retired[id.index()].take() {
            self.primitives[id] = value;
        }
        Some(id.index() as u16)
    }
    pub fn free(&mut self, index: u16) {
        let id = self.id(index);
        self.retired[usize::from(index)] = Some(self.primitives.free(id));
    }
    pub fn insert_before(&mut self, index: u16, before: Option<u16>) {
        let id = self.id(index);
        let before = before.map(|index| self.id(index));
        self.primitives.insert_before(id, before);
    }
    pub fn clear_order(&mut self) {
        while let Some(id) = self.primitives.head() {
            self.primitives.remove(id);
        }
    }
    pub fn reinit(&mut self) {
        self.clear_order();
        for index in 0..CAPACITY {
            if self.primitives.id_at(index).is_some() {
                self.free(index as u16);
            }
        }
        self.primitives.reinit();
    }
    pub fn order(&self) -> Vec<u16> {
        let mut order = Vec::with_capacity(self.primitives.count());
        let mut next = self.primitives.head();
        while let Some(id) = next {
            order.push(id.index() as u16);
            next = self.primitives.succ(id);
        }
        order
    }
    pub fn background_order(&mut self, count: u16) {
        for index in (0..count).rev() {
            self.insert_before(index, None);
        }
    }
    /// Serialize a transient drawing command batch without exposing pool storage.
    pub fn batch(&self) -> crate::graphics::native_batch::NativeBatch {
        crate::graphics::native_batch::NativeBatch::new(
            self.order()
                .into_iter()
                .map(|index| self.get(index).command())
                .collect(),
        )
    }
}

impl Default for DisplayPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_exhaustion_and_lifo_reuse_preserve_primitive_fields() {
        let mut display = DisplayPool::new();
        let ids: Vec<_> = (0..330).map(|_| display.alloc().unwrap()).collect();
        assert_eq!(ids, (0..330).collect::<Vec<_>>());
        assert!(display.alloc().is_none());
        display.get_mut(9).kind = 3;
        display.free(9);
        display.free(21);
        assert_eq!(display.alloc(), Some(21));
        assert_eq!(display.alloc(), Some(9));
        assert_eq!(display.get(9).kind, 3);
    }

    #[test]
    fn batch_serializes_background_and_element_order_without_mutating_storage() {
        let mut display = DisplayPool::new();
        for index in 0..181 {
            assert_eq!(display.alloc(), Some(index));
            display.get_mut(index).color.r = index as u8;
        }
        display.background_order(180);
        display.insert_before(180, None);
        let order_before = display.order();
        let retained = display.batch();
        let batch = retained.primitives();
        assert_eq!(batch.len(), 181);
        assert_eq!(batch[0].color.r, 179);
        assert_eq!(batch[179].color.r, 0);
        assert_eq!(batch[180].color.r, 180);
        assert_eq!(batch[0].Links, u32::from(u16::MAX) | (1 << 16));
        assert_eq!(batch[180].Links, 179 | (u32::from(u16::MAX) << 16));
        assert_eq!(display.order(), order_before);
        display.clear_order();
        assert!(display.batch().is_empty());
        display.background_order(180);
        assert_eq!(display.order(), (0..180).rev().collect::<Vec<_>>());
    }

    #[test]
    fn frame_order_can_be_rebuilt_without_freeing_primitives() {
        let mut display = DisplayPool::new();
        let a = display.alloc().unwrap();
        let b = display.alloc().unwrap();
        let c = display.alloc().unwrap();
        display.insert_before(b, None);
        display.insert_before(a, Some(b));
        display.insert_before(c, None);
        assert_eq!(display.order(), vec![a, b, c]);
        display.clear_order();
        display.insert_before(c, None);
        display.insert_before(a, None);
        assert_eq!(display.order(), vec![c, a]);
        assert_eq!(display.alloc(), Some(3));
        display.reinit();
        assert!(display.order().is_empty());
        assert_eq!(display.alloc(), Some(0));
    }

    #[test]
    fn freeing_a_linked_primitive_is_rejected() {
        let mut display = DisplayPool::new();
        let id = display.alloc().unwrap();
        display.insert_before(id, None);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| display.free(id))).is_err()
        );
        assert_eq!(display.order(), vec![id]);
    }
}

#[cfg(test)]
mod default_tests {
    use super::*;

    #[test]
    fn default_constructed_owner_has_initial_state() {
        let mut display = DisplayPool::default();
        assert!(display.order().is_empty());
        for index in 0..330 {
            assert_eq!(display.alloc(), Some(index));
        }
        assert!(display.alloc().is_none());
    }
}
