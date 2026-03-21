// Display List, Pool & Callback Registry
// @plan PLAN-20260320-BATTLE.P06
// @requirement REQ-BAT-043 through REQ-BAT-058 — Display list pool allocator

use super::element::Element;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum number of display elements (from init.c)
pub const MAX_DISPLAY_ELEMENTS: usize = 150;

// ---------------------------------------------------------------------------
// Generational Handle
// ---------------------------------------------------------------------------

/// A generational handle that includes a generation counter to detect use-after-free.
/// Each allocation increments the generation for that slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementHandle {
    /// Index into the pool array
    index: usize,
    /// Generation counter for this slot
    generation: u32,
}

impl ElementHandle {
    /// Creates a new handle with the given index and generation
    const fn new(index: usize, generation: u32) -> Self {
        ElementHandle { index, generation }
    }

    /// Returns the index component of this handle
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the generation component of this handle
    pub const fn generation(&self) -> u32 {
        self.generation
    }
}

// ---------------------------------------------------------------------------
// Pool Node
// ---------------------------------------------------------------------------

/// Internal pool node that wraps an Element with generation tracking
/// and doubly-linked list pointers
#[derive(Debug)]
struct PoolNode {
    /// The actual element data
    element: Element,
    /// Generation counter for this slot
    generation: u32,
    /// Is this node allocated (in active list)?
    allocated: bool,
    /// Previous node in the active list (None if not in list)
    prev: Option<usize>,
    /// Next node in the active list (None if not in list)
    next: Option<usize>,
}

impl PoolNode {
    fn new() -> Self {
        PoolNode {
            element: Element::new(),
            generation: 0,
            allocated: false,
            prev: None,
            next: None,
        }
    }
}

// ---------------------------------------------------------------------------
// DisplayList
// ---------------------------------------------------------------------------

/// A pool-based doubly-linked list allocator for Elements.
/// Uses a flat array with embedded prev/next links, matching C's displist.c
pub struct DisplayList {
    /// Fixed-size pool of nodes
    pool: Vec<PoolNode>,
    /// Index of the first node in the active list
    head: Option<usize>,
    /// Index of the last node in the active list
    tail: Option<usize>,
    /// Index of the first node in the free list
    free_head: Option<usize>,
    /// Number of currently active elements
    count: usize,
}

impl DisplayList {
    /// Creates a new display list with the given capacity
    pub fn new(capacity: usize) -> Self {
        let mut pool = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            pool.push(PoolNode::new());
        }

        // Build the free list (all nodes initially)
        let free_head = if capacity > 0 { Some(0) } else { None };
        for i in 0..capacity {
            pool[i].next = if i + 1 < capacity { Some(i + 1) } else { None };
        }

        DisplayList {
            pool,
            head: None,
            tail: None,
            free_head,
            count: 0,
        }
    }

    /// Creates a new display list with the default capacity (150)
    pub fn with_default_capacity() -> Self {
        Self::new(MAX_DISPLAY_ELEMENTS)
    }

    /// Allocates an element from the free list
    /// Returns None if the pool is exhausted
    pub fn alloc(&mut self) -> Option<ElementHandle> {
        let index = self.free_head?;

        // Remove from free list
        let node = &mut self.pool[index];
        self.free_head = node.next;

        // Initialize the node
        node.allocated = true;
        node.prev = None;
        node.next = None;
        node.generation = node.generation.wrapping_add(1);

        Some(ElementHandle::new(index, node.generation))
    }

    /// Frees an element, returning it to the free list
    /// The element is removed from the active list if it's in it
    pub fn free(&mut self, handle: ElementHandle) -> bool {
        if !self.is_valid_handle(handle) {
            return false;
        }

        let index = handle.index;

        // Remove from active list if present
        self.remove_internal(index);

        // Add to free list
        let node = &mut self.pool[index];
        node.allocated = false;
        node.next = self.free_head;
        self.free_head = Some(index);

        true
    }

    /// Appends an element to the tail of the active list
    pub fn push_back(&mut self, handle: ElementHandle) -> bool {
        if !self.is_valid_handle(handle) {
            return false;
        }

        let index = handle.index;

        // Remove from current position if already in list
        if self.pool[index].prev.is_some()
            || self.pool[index].next.is_some()
            || self.head == Some(index)
        {
            self.remove_internal(index);
        }

        // Add to tail
        if let Some(old_tail) = self.tail {
            self.pool[old_tail].next = Some(index);
            self.pool[index].prev = Some(old_tail);
        } else {
            // List was empty
            self.head = Some(index);
            self.pool[index].prev = None;
        }

        self.pool[index].next = None;
        self.tail = Some(index);
        self.count += 1;

        true
    }

    /// Inserts an element before the given reference element
    /// If before is None, appends to tail (same as push_back)
    pub fn insert_before(&mut self, handle: ElementHandle, before: Option<ElementHandle>) -> bool {
        if !self.is_valid_handle(handle) {
            return false;
        }

        // If no before reference, just append
        let Some(before_handle) = before else {
            return self.push_back(handle);
        };

        if !self.is_valid_handle(before_handle) {
            return false;
        }

        let index = handle.index;
        let before_index = before_handle.index;

        // Remove from current position if already in list
        if self.pool[index].prev.is_some()
            || self.pool[index].next.is_some()
            || self.head == Some(index)
        {
            self.remove_internal(index);
        }

        // Insert before the reference node
        let prev_index = self.pool[before_index].prev;

        self.pool[index].prev = prev_index;
        self.pool[index].next = Some(before_index);

        if let Some(prev_idx) = prev_index {
            self.pool[prev_idx].next = Some(index);
        } else {
            // Inserting at head
            self.head = Some(index);
        }

        self.pool[before_index].prev = Some(index);
        self.count += 1;

        true
    }

    /// Removes an element from the active list
    /// The element is NOT freed (still allocated)
    pub fn remove(&mut self, handle: ElementHandle) -> bool {
        if !self.is_valid_handle(handle) {
            return false;
        }

        self.remove_internal(handle.index);
        true
    }

    /// Internal remove that doesn't check handle validity
    fn remove_internal(&mut self, index: usize) {
        let node = &self.pool[index];

        // If not in list, nothing to do
        if node.prev.is_none() && node.next.is_none() && self.head != Some(index) {
            return;
        }

        let prev = node.prev;
        let next = node.next;

        // Update previous node's next pointer
        if let Some(prev_idx) = prev {
            self.pool[prev_idx].next = next;
        } else {
            // Removing head
            self.head = next;
        }

        // Update next node's prev pointer
        if let Some(next_idx) = next {
            self.pool[next_idx].prev = prev;
        } else {
            // Removing tail
            self.tail = prev;
        }

        // Clear this node's links
        self.pool[index].prev = None;
        self.pool[index].next = None;

        self.count = self.count.saturating_sub(1);
    }

    /// Returns the number of active elements
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the handle of the first active element
    pub fn head(&self) -> Option<ElementHandle> {
        self.head
            .map(|index| ElementHandle::new(index, self.pool[index].generation))
    }

    /// Returns the handle of the last active element
    pub fn tail(&self) -> Option<ElementHandle> {
        self.tail
            .map(|index| ElementHandle::new(index, self.pool[index].generation))
    }

    /// Returns the next element in the active list
    pub fn next(&self, handle: ElementHandle) -> Option<ElementHandle> {
        if !self.is_valid_handle(handle) {
            return None;
        }

        self.pool[handle.index]
            .next
            .map(|index| ElementHandle::new(index, self.pool[index].generation))
    }

    /// Returns the previous element in the active list
    pub fn prev(&self, handle: ElementHandle) -> Option<ElementHandle> {
        if !self.is_valid_handle(handle) {
            return None;
        }

        self.pool[handle.index]
            .prev
            .map(|index| ElementHandle::new(index, self.pool[index].generation))
    }

    /// Borrows an element by handle
    /// Returns None if the handle is invalid or stale
    pub fn get(&self, handle: ElementHandle) -> Option<&Element> {
        if !self.is_valid_handle(handle) {
            return None;
        }

        Some(&self.pool[handle.index].element)
    }

    /// Mutably borrows an element by handle
    /// Returns None if the handle is invalid or stale
    pub fn get_mut(&mut self, handle: ElementHandle) -> Option<&mut Element> {
        if !self.is_valid_handle(handle) {
            return None;
        }

        Some(&mut self.pool[handle.index].element)
    }

    /// Checks if a handle is valid (correct generation, within bounds, allocated)
    fn is_valid_handle(&self, handle: ElementHandle) -> bool {
        if handle.index >= self.pool.len() {
            return false;
        }

        let node = &self.pool[handle.index];
        node.allocated && node.generation == handle.generation
    }

    /// Returns an iterator over all active elements
    pub fn iter(&self) -> DisplayListIter<'_> {
        DisplayListIter {
            list: self,
            current: self.head,
        }
    }
}

// ---------------------------------------------------------------------------
// Iterator
// ---------------------------------------------------------------------------

/// Iterator over active elements in the display list
pub struct DisplayListIter<'a> {
    list: &'a DisplayList,
    current: Option<usize>,
}

impl<'a> Iterator for DisplayListIter<'a> {
    type Item = (ElementHandle, &'a Element);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.current?;
        let node = &self.list.pool[index];
        let handle = ElementHandle::new(index, node.generation);
        let element = &node.element;

        self.current = node.next;

        Some((handle, element))
    }
}

// ---------------------------------------------------------------------------
// Callback Registry
// ---------------------------------------------------------------------------

/// Callback types for element processing
/// Phase 1 only defines the types; actual dispatch is Phase 2+
pub type PreprocessCallback = Box<dyn FnMut(&mut Element)>;
pub type PostprocessCallback = Box<dyn FnMut(&mut Element)>;
pub type CollisionCallback = Box<dyn FnMut(&mut Element, &mut Element)>;
pub type DeathCallback = Box<dyn FnMut(&mut Element)>;

/// Registry entry for element callbacks
struct CallbackEntry {
    generation: u32,
    preprocess: Option<PreprocessCallback>,
    postprocess: Option<PostprocessCallback>,
    collision: Option<CollisionCallback>,
    death: Option<DeathCallback>,
}

/// Callback registry that maps element handles to Rust closures
/// Uses generational handles to prevent stale dispatch after element pool reuse
pub struct CallbackRegistry {
    entries: Vec<Option<CallbackEntry>>,
}

impl CallbackRegistry {
    /// Creates a new callback registry with the given capacity
    pub fn new(capacity: usize) -> Self {
        let mut entries = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            entries.push(None);
        }

        CallbackRegistry { entries }
    }

    /// Registers callbacks for an element handle
    pub fn register(&mut self, handle: ElementHandle) {
        if handle.index >= self.entries.len() {
            return;
        }

        self.entries[handle.index] = Some(CallbackEntry {
            generation: handle.generation,
            preprocess: None,
            postprocess: None,
            collision: None,
            death: None,
        });
    }

    /// Sets the preprocess callback for an element
    pub fn set_preprocess<F>(&mut self, handle: ElementHandle, callback: F)
    where
        F: FnMut(&mut Element) + 'static,
    {
        if let Some(Some(entry)) = self.entries.get_mut(handle.index) {
            if entry.generation == handle.generation {
                entry.preprocess = Some(Box::new(callback));
            }
        }
    }

    /// Sets the postprocess callback for an element
    pub fn set_postprocess<F>(&mut self, handle: ElementHandle, callback: F)
    where
        F: FnMut(&mut Element) + 'static,
    {
        if let Some(Some(entry)) = self.entries.get_mut(handle.index) {
            if entry.generation == handle.generation {
                entry.postprocess = Some(Box::new(callback));
            }
        }
    }

    /// Sets the collision callback for an element
    pub fn set_collision<F>(&mut self, handle: ElementHandle, callback: F)
    where
        F: FnMut(&mut Element, &mut Element) + 'static,
    {
        if let Some(Some(entry)) = self.entries.get_mut(handle.index) {
            if entry.generation == handle.generation {
                entry.collision = Some(Box::new(callback));
            }
        }
    }

    /// Sets the death callback for an element
    pub fn set_death<F>(&mut self, handle: ElementHandle, callback: F)
    where
        F: FnMut(&mut Element) + 'static,
    {
        if let Some(Some(entry)) = self.entries.get_mut(handle.index) {
            if entry.generation == handle.generation {
                entry.death = Some(Box::new(callback));
            }
        }
    }

    /// Unregisters all callbacks for an element handle
    pub fn unregister(&mut self, handle: ElementHandle) {
        if handle.index < self.entries.len() {
            self.entries[handle.index] = None;
        }
    }

    /// Gets a reference to the callback entry for an element (if valid)
    fn get_entry(&self, handle: ElementHandle) -> Option<&CallbackEntry> {
        self.entries
            .get(handle.index)?
            .as_ref()
            .filter(|entry| entry.generation == handle.generation)
    }

    /// Gets a mutable reference to the callback entry for an element (if valid)
    fn get_entry_mut(&mut self, handle: ElementHandle) -> Option<&mut CallbackEntry> {
        self.entries
            .get_mut(handle.index)?
            .as_mut()
            .filter(|entry| entry.generation == handle.generation)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Pool Allocation/Deallocation --

    #[test]
    fn test_alloc_returns_valid_handle() {
        let mut list = DisplayList::new(150);
        let handle = list.alloc();
        assert!(handle.is_some());

        let handle = handle.unwrap();
        assert_eq!(handle.index(), 0);
        assert_eq!(handle.generation(), 1); // First allocation increments to 1
    }

    #[test]
    fn test_alloc_pool_exhaustion() {
        let mut list = DisplayList::new(2);

        let h1 = list.alloc();
        assert!(h1.is_some());

        let h2 = list.alloc();
        assert!(h2.is_some());

        // 3rd allocation should fail (pool exhausted)
        let h3 = list.alloc();
        assert!(h3.is_none());
    }

    #[test]
    fn test_alloc_free_alloc_reuses_slot() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        assert_eq!(h1.index(), 0);
        assert_eq!(h1.generation(), 1);

        list.free(h1);

        let h2 = list.alloc().unwrap();
        assert_eq!(h2.index(), 0); // Same slot
        assert_eq!(h2.generation(), 2); // Incremented generation
    }

    #[test]
    fn test_get_with_stale_handle_returns_none() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        list.free(h1);
        let _h2 = list.alloc().unwrap(); // Increments generation

        // h1 is now stale (wrong generation)
        assert!(list.get(h1).is_none());
    }

    // -- List Operations --

    #[test]
    fn test_push_back_maintains_count() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        let h2 = list.alloc().unwrap();
        let h3 = list.alloc().unwrap();

        assert_eq!(list.count(), 0);

        list.push_back(h1);
        assert_eq!(list.count(), 1);

        list.push_back(h2);
        assert_eq!(list.count(), 2);

        list.push_back(h3);
        assert_eq!(list.count(), 3);
    }

    #[test]
    fn test_push_back_maintains_order() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        let h2 = list.alloc().unwrap();
        let h3 = list.alloc().unwrap();

        list.push_back(h1);
        list.push_back(h2);
        list.push_back(h3);

        // Check head → tail order
        assert_eq!(list.head().unwrap().index(), h1.index());
        assert_eq!(list.tail().unwrap().index(), h3.index());

        // Check iteration order
        let mut iter = list.iter();
        assert_eq!(iter.next().unwrap().0.index(), h1.index());
        assert_eq!(iter.next().unwrap().0.index(), h2.index());
        assert_eq!(iter.next().unwrap().0.index(), h3.index());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_remove_updates_count() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        let h2 = list.alloc().unwrap();

        list.push_back(h1);
        list.push_back(h2);
        assert_eq!(list.count(), 2);

        list.remove(h1);
        assert_eq!(list.count(), 1);

        list.remove(h2);
        assert_eq!(list.count(), 0);
    }

    #[test]
    fn test_remove_from_middle() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        let h2 = list.alloc().unwrap();
        let h3 = list.alloc().unwrap();

        list.push_back(h1);
        list.push_back(h2);
        list.push_back(h3);

        list.remove(h2);

        // h1 → h3, h2 is gone
        let mut iter = list.iter();
        assert_eq!(iter.next().unwrap().0.index(), h1.index());
        assert_eq!(iter.next().unwrap().0.index(), h3.index());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_insert_before_places_correctly() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        let h2 = list.alloc().unwrap();
        let h3 = list.alloc().unwrap();

        list.push_back(h1);
        list.push_back(h3);

        // Insert h2 before h3
        list.insert_before(h2, Some(h3));

        // Expected order: h1 → h2 → h3
        let mut iter = list.iter();
        assert_eq!(iter.next().unwrap().0.index(), h1.index());
        assert_eq!(iter.next().unwrap().0.index(), h2.index());
        assert_eq!(iter.next().unwrap().0.index(), h3.index());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_insert_before_at_head() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        let h2 = list.alloc().unwrap();

        list.push_back(h1);

        // Insert h2 before h1 (at head)
        list.insert_before(h2, Some(h1));

        // Expected order: h2 → h1
        assert_eq!(list.head().unwrap().index(), h2.index());
        assert_eq!(list.tail().unwrap().index(), h1.index());
    }

    #[test]
    fn test_insert_before_none_appends() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        let h2 = list.alloc().unwrap();

        list.push_back(h1);

        // Insert h2 with before=None (should append)
        list.insert_before(h2, None);

        // Expected order: h1 → h2
        let mut iter = list.iter();
        assert_eq!(iter.next().unwrap().0.index(), h1.index());
        assert_eq!(iter.next().unwrap().0.index(), h2.index());
        assert!(iter.next().is_none());
    }

    // -- Navigation --

    #[test]
    fn test_head_tail_next_prev() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        let h2 = list.alloc().unwrap();
        let h3 = list.alloc().unwrap();

        list.push_back(h1);
        list.push_back(h2);
        list.push_back(h3);

        // Head/tail
        assert_eq!(list.head().unwrap().index(), h1.index());
        assert_eq!(list.tail().unwrap().index(), h3.index());

        // Next navigation
        assert_eq!(list.next(h1).unwrap().index(), h2.index());
        assert_eq!(list.next(h2).unwrap().index(), h3.index());
        assert!(list.next(h3).is_none());

        // Prev navigation
        assert!(list.prev(h1).is_none());
        assert_eq!(list.prev(h2).unwrap().index(), h1.index());
        assert_eq!(list.prev(h3).unwrap().index(), h2.index());
    }

    // -- Generational Handle Safety --

    #[test]
    fn test_stale_handle_get_returns_none() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        list.push_back(h1);

        list.free(h1); // Free and return to pool
        let _h2 = list.alloc().unwrap(); // Reuse slot, increment generation

        // h1 is now stale
        assert!(list.get(h1).is_none());
        assert!(list.get_mut(h1).is_none());
    }

    #[test]
    fn test_get_mut_allows_modification() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        list.push_back(h1);

        {
            let elem = list.get_mut(h1).unwrap();
            elem.mass_points = 42;
        }

        let elem = list.get(h1).unwrap();
        assert_eq!(elem.mass_points, 42);
    }

    // -- Edge Cases --

    #[test]
    fn test_empty_list_operations() {
        let list = DisplayList::new(150);

        assert_eq!(list.count(), 0);
        assert!(list.head().is_none());
        assert!(list.tail().is_none());

        let mut iter = list.iter();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_single_element_list() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        list.push_back(h1);

        assert_eq!(list.count(), 1);
        assert_eq!(list.head().unwrap().index(), h1.index());
        assert_eq!(list.tail().unwrap().index(), h1.index());
        assert!(list.next(h1).is_none());
        assert!(list.prev(h1).is_none());
    }

    #[test]
    fn test_remove_all_elements() {
        let mut list = DisplayList::new(150);

        let h1 = list.alloc().unwrap();
        let h2 = list.alloc().unwrap();

        list.push_back(h1);
        list.push_back(h2);

        list.remove(h1);
        list.remove(h2);

        assert_eq!(list.count(), 0);
        assert!(list.head().is_none());
        assert!(list.tail().is_none());
    }

    // -- Callback Registry --

    #[test]
    fn test_callback_registry_register_unregister() {
        let mut list = DisplayList::new(150);
        let mut registry = CallbackRegistry::new(150);

        let h1 = list.alloc().unwrap();
        registry.register(h1);

        // Should be able to set callbacks
        registry.set_preprocess(h1, |_elem| {});
        registry.set_postprocess(h1, |_elem| {});

        // Unregister
        registry.unregister(h1);

        // Entry should be gone
        assert!(registry.get_entry(h1).is_none());
    }

    #[test]
    fn test_callback_registry_generation_check() {
        let mut list = DisplayList::new(150);
        let mut registry = CallbackRegistry::new(150);

        let h1 = list.alloc().unwrap();
        registry.register(h1);

        list.free(h1);
        let h2 = list.alloc().unwrap(); // Same index, different generation

        // h1 is stale (old generation), setting callback should fail (no-op)
        // The old entry still exists with generation 1, but h1 has generation 1
        // and the callback set will fail because generation mismatch after h2 allocation
        registry.set_preprocess(h1, |_elem| {});

        // The entry still exists from h1, but has wrong generation for h2
        // h2 has generation 2, entry has generation 1
        assert!(registry.get_entry(h2).is_none());

        // Manually unregister old entry
        registry.unregister(h1);

        // Now register h2 with correct generation
        registry.register(h2);
        registry.set_preprocess(h2, |_elem| {});
        assert!(registry.get_entry(h2).is_some());
    }

    #[test]
    fn test_default_capacity() {
        let list = DisplayList::with_default_capacity();
        assert_eq!(list.pool.len(), MAX_DISPLAY_ELEMENTS);
    }

    // -- 151st allocation test --

    #[test]
    fn test_151st_allocation_fails() {
        let mut list = DisplayList::new(150);

        // Allocate 150 elements
        for _ in 0..150 {
            assert!(list.alloc().is_some());
        }

        // 151st allocation should fail
        assert!(list.alloc().is_none());
    }
}
