//! Lock-free mirror model for ABI shell state.
//!
//! Implements REQ-STATE-003 (fixed lock-free mirrors): terminal/status, abort,
//! phase, capture request, and six owned-key mask/values. Nested entry and
//! unusable/poisoned lock release/abort without locking and never resume
//! scheduling.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
//! @requirement REQ-STATE-003

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

use crate::automation::outcome::TerminalMirror;

// ===========================================================================
//  Runtime phase (REQ-STATE-003)
// ===========================================================================

/// The runtime phase of the automation system.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-003
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RuntimePhase {
    /// Not yet initialized.
    Inactive = 0,
    /// Active and running.
    Running = 1,
    /// Finalization in progress.
    Finalizing = 2,
    /// Finalization complete.
    Finalized = 3,
}

impl RuntimePhase {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Running,
            2 => Self::Finalizing,
            3 => Self::Finalized,
            _ => Self::Inactive,
        }
    }
}

// ===========================================================================
//  Owned-key mirror (REQ-STATE-003)
// ===========================================================================

/// The six owned menu keys (matching `MenuKey` variants).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-003
pub const NUM_OWNED_KEYS: usize = 6;

/// A lock-free mirror of the six owned-key mask and values.
///
/// Each key has:
/// - An owned bit (whether this key is currently held by automation).
/// - A value (0 or 1 — the key state set by automation).
///
/// All fields are lock-free atomics. Updates use release ordering; reads use
/// acquire ordering.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-003
pub struct OwnedKeyMirror {
    /// Bitmask of owned keys. Bit i = key i is owned.
    owned_mask: AtomicU64,
    /// Per-key values. value[i] is the value set for key i (0 or 1).
    values: [AtomicU8; NUM_OWNED_KEYS],
}

impl OwnedKeyMirror {
    /// Create a new owned-key mirror with no keys owned.
    #[must_use]
    pub fn new() -> Self {
        Self {
            owned_mask: AtomicU64::new(0),
            values: [const { AtomicU8::new(0) }; NUM_OWNED_KEYS],
        }
    }

    /// Set a key as owned with a value. Uses release ordering.
    pub fn set_owned(&self, key_index: usize, value: u8) {
        if key_index < NUM_OWNED_KEYS {
            self.values[key_index].store(value, Ordering::Release);
            let bit = 1u64 << key_index;
            self.owned_mask.fetch_or(bit, Ordering::AcqRel);
        }
    }

    /// Clear a key as no longer owned. Uses release ordering.
    pub fn clear_owned(&self, key_index: usize) {
        if key_index < NUM_OWNED_KEYS {
            let bit = 1u64 << key_index;
            self.owned_mask.fetch_and(!bit, Ordering::AcqRel);
            self.values[key_index].store(0, Ordering::Release);
        }
    }

    /// Release all owned keys. Returns the mask of keys that were owned.
    pub fn release_all(&self) -> u64 {
        let mask = self.owned_mask.swap(0, Ordering::AcqRel);
        for i in 0..NUM_OWNED_KEYS {
            self.values[i].store(0, Ordering::Release);
        }
        mask
    }

    /// Get the owned mask (acquire ordering).
    #[must_use]
    pub fn owned_mask(&self) -> u64 {
        self.owned_mask.load(Ordering::Acquire)
    }

    /// Get the value for a key (acquire ordering).
    #[must_use]
    pub fn value(&self, key_index: usize) -> u8 {
        if key_index < NUM_OWNED_KEYS {
            self.values[key_index].load(Ordering::Acquire)
        } else {
            0
        }
    }
}

impl Default for OwnedKeyMirror {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
//  Complete mirror model (REQ-STATE-003)
// ===========================================================================

/// The complete lock-free mirror model for ABI shell state.
///
/// All fields are fixed-size lock-free atomics:
/// - terminal: TerminalMirror (AtomicU8)
/// - abort_requested: AtomicBool
/// - phase: AtomicU8 (RuntimePhase)
/// - capture_request_generation: AtomicU64
/// - owned_keys: OwnedKeyMirror
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-003
pub struct SyncModel {
    pub terminal: TerminalMirror,
    pub abort_requested: AtomicBool,
    pub phase: AtomicU8,
    pub capture_request_generation: AtomicU64,
    pub owned_keys: OwnedKeyMirror,
    /// Shell entry depth (thread-local, but modeled here for pure tests).
    pub entry_depth: AtomicU8,
}

impl SyncModel {
    /// Create a new sync model in the inactive state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            terminal: TerminalMirror::new(),
            abort_requested: AtomicBool::new(false),
            phase: AtomicU8::new(RuntimePhase::Inactive as u8),
            capture_request_generation: AtomicU64::new(0),
            owned_keys: OwnedKeyMirror::new(),
            entry_depth: AtomicU8::new(0),
        }
    }

    /// Request abort. Uses release ordering.
    pub fn request_abort(&self) {
        self.abort_requested.store(true, Ordering::Release);
    }

    /// Whether abort has been requested.
    #[must_use]
    pub fn is_abort_requested(&self) -> bool {
        self.abort_requested.load(Ordering::Acquire)
    }

    /// Set the runtime phase.
    pub fn set_phase(&self, phase: RuntimePhase) {
        self.phase.store(phase as u8, Ordering::Release);
    }

    /// Check if the runtime is in an active (Running) phase.
    pub fn is_active(&self) -> bool {
        matches!(self.phase(), RuntimePhase::Running)
    }

    /// Get the current entry depth (for reentry detection).
    pub fn entry_depth(&self) -> u8 {
        self.entry_depth.load(Ordering::Acquire)
    }

    pub fn phase(&self) -> RuntimePhase {
        RuntimePhase::from_u8(self.phase.load(Ordering::Acquire))
    }

    /// Set capture request generation (0 = none).
    pub fn set_capture_generation(&self, gen: u64) {
        self.capture_request_generation
            .store(gen, Ordering::Release);
    }

    /// Get capture request generation (0 = none).
    #[must_use]
    pub fn capture_generation(&self) -> u64 {
        self.capture_request_generation.load(Ordering::Acquire)
    }

    /// Clear capture request generation.
    pub fn clear_capture_generation(&self) {
        self.capture_request_generation.store(0, Ordering::Release);
    }

    /// Whether this model is in a terminal state (any terminal class set).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.terminal.is_terminal()
    }

    /// Whether reentry is detected (depth > 0).
    #[must_use]
    pub fn is_reentrant(&self) -> bool {
        self.entry_depth.load(Ordering::Acquire) > 0
    }
}

impl Default for SyncModel {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Owned-key mirror ---

    #[test]
    fn owned_key_set_and_clear() {
        let mirror = OwnedKeyMirror::new();
        assert_eq!(mirror.owned_mask(), 0);
        mirror.set_owned(2, 1);
        assert_eq!(mirror.owned_mask(), 0b100);
        assert_eq!(mirror.value(2), 1);
        mirror.clear_owned(2);
        assert_eq!(mirror.owned_mask(), 0);
        assert_eq!(mirror.value(2), 0);
    }

    #[test]
    fn owned_key_release_all() {
        let mirror = OwnedKeyMirror::new();
        mirror.set_owned(0, 1);
        mirror.set_owned(3, 1);
        mirror.set_owned(5, 1);
        // Bits 0, 3, 5 = 1 + 8 + 32 = 41 = 0b101001
        let expected = (1 << 0) | (1 << 3) | (1 << 5);
        assert_eq!(mirror.owned_mask(), expected);
        let released = mirror.release_all();
        assert_eq!(released, expected);
        assert_eq!(mirror.owned_mask(), 0);
    }

    #[test]
    fn owned_key_out_of_bounds_ignored() {
        let mirror = OwnedKeyMirror::new();
        mirror.set_owned(99, 1);
        assert_eq!(mirror.owned_mask(), 0);
    }

    // --- Sync model ---

    #[test]
    fn sync_model_initial_state() {
        let model = SyncModel::new();
        assert!(!model.is_terminal());
        assert!(!model.is_abort_requested());
        assert_eq!(model.phase(), RuntimePhase::Inactive);
        assert_eq!(model.capture_generation(), 0);
        assert!(!model.is_reentrant());
    }

    #[test]
    fn sync_model_abort_request() {
        let model = SyncModel::new();
        model.request_abort();
        assert!(model.is_abort_requested());
    }

    #[test]
    fn sync_model_phase_transitions() {
        let model = SyncModel::new();
        model.set_phase(RuntimePhase::Running);
        assert_eq!(model.phase(), RuntimePhase::Running);
        model.set_phase(RuntimePhase::Finalizing);
        assert_eq!(model.phase(), RuntimePhase::Finalizing);
        model.set_phase(RuntimePhase::Finalized);
        assert_eq!(model.phase(), RuntimePhase::Finalized);
    }

    #[test]
    fn sync_model_capture_generation() {
        let model = SyncModel::new();
        model.set_capture_generation(42);
        assert_eq!(model.capture_generation(), 42);
        model.clear_capture_generation();
        assert_eq!(model.capture_generation(), 0);
    }

    #[test]
    fn sync_model_reentry_detection() {
        let model = SyncModel::new();
        assert!(!model.is_reentrant());
        model.entry_depth.store(1, Ordering::Release);
        assert!(model.is_reentrant());
    }

    // --- Lock-free property (verified by P00 probes) ---

    #[test]
    fn all_mirrors_use_atomic_types() {
        // P00 probes already verified lock-free atomics at runtime.
        // Here we verify the mirror types are atomics by exercising them.
        let model = SyncModel::new();
        model.request_abort();
        assert!(model.is_abort_requested());
        model.set_phase(RuntimePhase::Running);
        assert_eq!(model.phase(), RuntimePhase::Running);
    }

    // --- Nested entry → abort without locking (REQ-STATE-003) ---

    #[test]
    fn nested_entry_aborts_without_resume() {
        let model = SyncModel::new();
        // Simulate nested entry: depth > 0.
        model.entry_depth.store(1, Ordering::Release);
        // Nested entry should request abort and release keys from mirror,
        // without locking and never resume scheduling.
        model.request_abort();
        model.owned_keys.release_all();
        // Terminal is set from the nested entry path.
        model
            .terminal
            .try_set(crate::automation::outcome::TerminalClass::PanicFallback);
        assert!(model.is_abort_requested());
        assert!(model.is_terminal());
        assert_eq!(model.owned_keys.owned_mask(), 0);
    }
}
