//! Keyboard binding management
//!
//! Handles keyboard key bindings to control state variables.

use std::collections::HashMap;

/// A keyboard key binding that maps a keycode to a control state variable.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// Target control state variable address (as usize for FFI)
    pub target: usize,
    /// SDL keycode
    pub keycode: i32,
}

impl KeyBinding {
    /// Create a new key binding
    pub fn new(keycode: i32, target: usize) -> Self {
        Self { keycode, target }
    }
}

/// Number of buckets for keyboard binding hash table
pub const KEYBOARD_INPUT_BUCKETS: usize = 512;

/// Keyboard binding manager
#[derive(Debug)]
pub struct KeyboardBindings {
    /// Hash buckets: keycode → list of bindings
    buckets: [Vec<KeyBinding>; KEYBOARD_INPUT_BUCKETS],
    /// Fast lookup: keycode → bucket index
    keycode_to_bucket: HashMap<i32, usize>,
}

impl Default for KeyboardBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardBindings {
    /// Create a new empty keyboard binding manager
    pub fn new() -> Self {
        // Initialize empty buckets
        const EMPTY_VEC: Vec<KeyBinding> = Vec::new();
        Self {
            buckets: [EMPTY_VEC; KEYBOARD_INPUT_BUCKETS],
            keycode_to_bucket: HashMap::new(),
        }
    }

    /// Compute bucket index for a keycode
    fn bucket_index(keycode: i32) -> usize {
        (keycode as usize) % KEYBOARD_INPUT_BUCKETS
    }

    /// Add a key binding
    pub fn add_binding(&mut self, keycode: i32, target: usize) -> bool {
        let bucket_idx = Self::bucket_index(keycode);
        let bucket = &mut self.buckets[bucket_idx];

        // Check if this exact binding already exists
        for binding in bucket.iter() {
            if binding.keycode == keycode && binding.target == target {
                return false; // Already bound
            }
        }

        bucket.push(KeyBinding::new(keycode, target));
        self.keycode_to_bucket.insert(keycode, bucket_idx);
        true
    }

    /// Remove a key binding
    pub fn remove_binding(&mut self, keycode: i32, target: usize) -> bool {
        let bucket_idx = Self::bucket_index(keycode);
        let bucket = &mut self.buckets[bucket_idx];

        let original_len = bucket.len();
        bucket.retain(|b| !(b.keycode == keycode && b.target == target));

        if bucket.len() != original_len {
            // Check if there are any remaining bindings for this keycode
            if !bucket.iter().any(|b| b.keycode == keycode) {
                self.keycode_to_bucket.remove(&keycode);
            }
            true
        } else {
            false
        }
    }

    /// Remove all bindings for a keycode
    pub fn remove_all_for_key(&mut self, keycode: i32) -> usize {
        let bucket_idx = Self::bucket_index(keycode);
        let bucket = &mut self.buckets[bucket_idx];

        let original_len = bucket.len();
        bucket.retain(|b| b.keycode != keycode);
        let removed = original_len - bucket.len();

        if removed > 0 {
            self.keycode_to_bucket.remove(&keycode);
        }

        removed
    }

    /// Clear all bindings
    pub fn clear(&mut self) {
        for bucket in self.buckets.iter_mut() {
            bucket.clear();
        }
        self.keycode_to_bucket.clear();
    }

    /// Get all bindings for a keycode
    pub fn get_bindings(&self, keycode: i32) -> impl Iterator<Item = &KeyBinding> {
        let bucket_idx = Self::bucket_index(keycode);
        self.buckets[bucket_idx]
            .iter()
            .filter(move |b| b.keycode == keycode)
    }

    /// Check if a keycode has any bindings
    pub fn has_bindings(&self, keycode: i32) -> bool {
        self.keycode_to_bucket.contains_key(&keycode)
    }

    /// Get total number of bindings
    pub fn binding_count(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    /// Handle key down event - increments target and sets STARTBIT
    ///
    /// This matches the C behavior:
    /// `*(target) = (*(target)+1) | VCONTROL_STARTBIT`
    ///
    /// The STARTBIT (0x100) indicates this key was just pressed this frame.
    /// The lower bits (VCONTROL_MASK = 0xFF) count how many times key is pressed
    /// (for multiple bindings to the same key).
    ///
    /// # Safety
    /// Caller must ensure target addresses are valid writable i32 pointers
    pub unsafe fn handle_key_down(&self, keycode: i32) {
        const VCONTROL_STARTBIT: i32 = 0x100;
        for binding in self.get_bindings(keycode) {
            let target_ptr = binding.target as *mut i32;
            if !target_ptr.is_null() {
                *target_ptr = (*target_ptr + 1) | VCONTROL_STARTBIT;
            }
        }
    }

    /// Handle key up event - decrements target (keeping STARTBIT if present)
    ///
    /// This matches the C behavior:
    /// ```c
    /// int v = *(target) & VCONTROL_MASK;
    /// if (v > 0) *(target) = (v-1) | (*(target) & VCONTROL_STARTBIT);
    /// ```
    ///
    /// # Safety
    /// Caller must ensure target addresses are valid writable i32 pointers
    pub unsafe fn handle_key_up(&self, keycode: i32) {
        const VCONTROL_STARTBIT: i32 = 0x100;
        const VCONTROL_MASK: i32 = 0xFF;
        for binding in self.get_bindings(keycode) {
            let target_ptr = binding.target as *mut i32;
            if !target_ptr.is_null() {
                let v = *target_ptr & VCONTROL_MASK;
                if v > 0 {
                    *target_ptr = (v - 1) | (*target_ptr & VCONTROL_STARTBIT);
                }
            }
        }
    }

    /// Reset all bound control states to 0
    ///
    /// # Safety
    /// Caller must ensure target addresses are valid writable i32 pointers
    pub unsafe fn reset_all_states(&self) {
        for bucket in self.buckets.iter() {
            for binding in bucket.iter() {
                let target_ptr = binding.target as *mut i32;
                if !target_ptr.is_null() {
                    *target_ptr = 0;
                }
            }
        }
    }

    /// Begin a new input frame - clear start bits from all bound targets
    ///
    /// # Safety
    /// Caller must ensure target addresses are valid writable i32 pointers
    pub unsafe fn begin_frame(&self) {
        // Clear the VCONTROL_STARTBIT from all bound targets
        // VCONTROL_STARTBIT = 0x100, VCONTROL_MASK = 0xFF
        const VCONTROL_MASK: i32 = 0xFF;
        for bucket in self.buckets.iter() {
            for binding in bucket.iter() {
                let target_ptr = binding.target as *mut i32;
                if !target_ptr.is_null() {
                    *target_ptr &= VCONTROL_MASK;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_bindings() {
        let bindings = KeyboardBindings::new();
        assert_eq!(bindings.binding_count(), 0);
    }

    #[test]
    fn test_add_binding() {
        let mut bindings = KeyboardBindings::new();
        assert!(bindings.add_binding(32, 0x1000)); // Space key
        assert_eq!(bindings.binding_count(), 1);
        assert!(bindings.has_bindings(32));
    }

    #[test]
    fn test_add_duplicate_binding() {
        let mut bindings = KeyboardBindings::new();
        assert!(bindings.add_binding(32, 0x1000));
        assert!(!bindings.add_binding(32, 0x1000)); // Duplicate
        assert_eq!(bindings.binding_count(), 1);
    }

    #[test]
    fn test_multiple_bindings_same_key() {
        let mut bindings = KeyboardBindings::new();
        assert!(bindings.add_binding(32, 0x1000));
        assert!(bindings.add_binding(32, 0x2000)); // Different target
        assert_eq!(bindings.binding_count(), 2);

        let count = bindings.get_bindings(32).count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_remove_binding() {
        let mut bindings = KeyboardBindings::new();
        bindings.add_binding(32, 0x1000);
        bindings.add_binding(32, 0x2000);

        assert!(bindings.remove_binding(32, 0x1000));
        assert_eq!(bindings.binding_count(), 1);
        assert!(bindings.has_bindings(32)); // Still has one binding
    }

    #[test]
    fn test_remove_nonexistent_binding() {
        let mut bindings = KeyboardBindings::new();
        assert!(!bindings.remove_binding(32, 0x1000));
    }

    #[test]
    fn test_remove_all_for_key() {
        let mut bindings = KeyboardBindings::new();
        bindings.add_binding(32, 0x1000);
        bindings.add_binding(32, 0x2000);
        bindings.add_binding(65, 0x3000); // 'a' key

        let removed = bindings.remove_all_for_key(32);
        assert_eq!(removed, 2);
        assert!(!bindings.has_bindings(32));
        assert!(bindings.has_bindings(65));
        assert_eq!(bindings.binding_count(), 1);
    }

    #[test]
    fn test_clear() {
        let mut bindings = KeyboardBindings::new();
        bindings.add_binding(32, 0x1000);
        bindings.add_binding(65, 0x2000);
        bindings.add_binding(66, 0x3000);

        bindings.clear();
        assert_eq!(bindings.binding_count(), 0);
        assert!(!bindings.has_bindings(32));
    }

    #[test]
    fn test_key_down_up() {
        let mut bindings = KeyboardBindings::new();
        let mut target: i32 = 0;
        let target_addr = &mut target as *mut i32 as usize;

        bindings.add_binding(32, target_addr);

        // VCONTROL_STARTBIT (0x100) is set on key down, plus the count (1)
        // So key down sets target to 0x101 (257)
        // Key up decrements the count but preserves STARTBIT, so 0x100 (256)
        const VCONTROL_STARTBIT: i32 = 0x100;
        const VCONTROL_MASK: i32 = 0xFF;

        unsafe {
            bindings.handle_key_down(32);
            assert_eq!(target & VCONTROL_MASK, 1); // Count is 1
            assert_ne!(target & VCONTROL_STARTBIT, 0); // STARTBIT is set

            bindings.handle_key_up(32);
            assert_eq!(target & VCONTROL_MASK, 0); // Count is 0
        }
    }

    #[test]
    fn test_multiple_targets_same_key() {
        let mut bindings = KeyboardBindings::new();
        let mut target1: i32 = 0;
        let mut target2: i32 = 0;

        bindings.add_binding(32, &mut target1 as *mut i32 as usize);
        bindings.add_binding(32, &mut target2 as *mut i32 as usize);

        const VCONTROL_MASK: i32 = 0xFF;

        unsafe {
            bindings.handle_key_down(32);
            assert_eq!(target1 & VCONTROL_MASK, 1);
            assert_eq!(target2 & VCONTROL_MASK, 1);

            bindings.handle_key_up(32);
            assert_eq!(target1 & VCONTROL_MASK, 0);
            assert_eq!(target2 & VCONTROL_MASK, 0);
        }
    }

    #[test]
    fn test_reset_all_states() {
        let mut bindings = KeyboardBindings::new();
        let mut target1: i32 = 5;
        let mut target2: i32 = 10;

        bindings.add_binding(32, &mut target1 as *mut i32 as usize);
        bindings.add_binding(65, &mut target2 as *mut i32 as usize);

        unsafe {
            bindings.reset_all_states();
            assert_eq!(target1, 0);
            assert_eq!(target2, 0);
        }
    }

    #[test]
    fn test_bucket_distribution() {
        let mut bindings = KeyboardBindings::new();

        // Add keys that should hash to different buckets
        for i in 0..100 {
            bindings.add_binding(i, 0x1000 + i as usize);
        }

        assert_eq!(bindings.binding_count(), 100);
    }
}
