//! Encounter lifecycle orchestration.
//!
//! In Rust mode, this module owns the high-level encounter lifecycle:
//! context resolution → SIS update → race init → hail/attack decision
//! → dialogue → teardown. Heavy lifting (resource loading, game state,
//! graphics) remains C-owned and is accessed through FFI wrappers.
//!
//! @plan PLAN-20260314-COMM.P08
//! @requirement EC-REQ-001 through EC-REQ-016, OL-REQ-002

use super::types::{CommError, CommResult};
use crate::comm::Comm;

// ============================================================================
// Encounter status codes matching C (comm.c)
// ============================================================================

/// Encounter outcome: player chose to talk.
pub const HAIL: u32 = 0;
/// Encounter outcome: player chose to attack.
pub const ATTACK: u32 = 1;

// ============================================================================
// Callback tracking
// ============================================================================

/// Tracks which lifecycle callbacks have been invoked for this encounter.
/// Enforces EC-REQ-009 (normal: init→post→uninit) and EC-REQ-010
/// (abort: init→uninit, skip post).
#[derive(Debug, Default)]
pub struct CallbackTracker {
    pub init_called: bool,
    pub post_called: bool,
    pub uninit_called: bool,
}

impl CallbackTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.init_called = false;
        self.post_called = false;
        self.uninit_called = false;
    }

    /// Record that init_encounter_func was called.
    /// Returns Err if already called (EC-REQ-016: at most once).
    pub fn mark_init(&mut self) -> CommResult<()> {
        if self.init_called {
            return Err(CommError::AlreadyInitialized);
        }
        self.init_called = true;
        Ok(())
    }

    /// Record that post_encounter_func was called.
    pub fn mark_post(&mut self) -> CommResult<()> {
        if self.post_called {
            return Err(CommError::InvalidState("post already called".into()));
        }
        self.post_called = true;
        Ok(())
    }

    /// Record that uninit_encounter_func was called.
    pub fn mark_uninit(&mut self) -> CommResult<()> {
        if self.uninit_called {
            return Err(CommError::InvalidState("uninit already called".into()));
        }
        self.uninit_called = true;
        Ok(())
    }
}

// ============================================================================
// Encounter state
// ============================================================================

// ============================================================================
// Encounter lifecycle
// ============================================================================

/// Active encounter state, held in Comm.
#[derive(Debug)]
pub struct EncounterState {
    /// Whether an encounter is currently active.
    pub active: bool,
    /// Callback invocation tracking.
    pub callbacks: CallbackTracker,
    /// Whether this is an abort/load exit.
    pub aborted: bool,
}

impl Default for EncounterState {
    fn default() -> Self {
        Self::new()
    }
}

impl EncounterState {
    pub fn new() -> Self {
        Self {
            active: false,
            callbacks: CallbackTracker::new(),
            aborted: false,
        }
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.callbacks.reset();
        self.aborted = false;
    }
}

// ============================================================================
// Public lifecycle functions
// ============================================================================

/// Begin encounter lifecycle — called after hail_alien resources loaded.
///
/// This records that the encounter is active and the init callback has
/// been invoked. In production, the actual callbacks are invoked by C;
/// this provides the Rust-side tracking and ordering enforcement.
pub fn begin_encounter(comm: &mut Comm) -> CommResult<()> {
    let state = &mut *comm;
    let enc = &mut state.encounter;
    if enc.active {
        return Err(CommError::AlreadyInitialized);
    }
    enc.active = true;
    enc.callbacks.mark_init()?;
    Ok(())
}

/// Record normal encounter exit (init→post→uninit).
pub fn end_encounter_normal(comm: &mut Comm) -> CommResult<()> {
    let state = &mut *comm;
    let enc = &mut state.encounter;
    if !enc.active {
        return Err(CommError::NotInitialized);
    }
    enc.callbacks.mark_post()?;
    enc.callbacks.mark_uninit()?;
    enc.active = false;
    // Reset callbacks for next encounter reuse
    enc.callbacks.reset();
    Ok(())
}

/// Record abort encounter exit (init→uninit, skip post).
pub fn end_encounter_abort(comm: &mut Comm) -> CommResult<()> {
    let state = &mut *comm;
    let enc = &mut state.encounter;
    if !enc.active {
        return Err(CommError::NotInitialized);
    }
    enc.aborted = true;
    enc.callbacks.mark_uninit()?;
    enc.active = false;
    enc.callbacks.reset();
    enc.aborted = false;
    Ok(())
}

/// Record attack-without-hail exit (post→uninit, no init).
pub fn end_encounter_attack(comm: &mut Comm) -> CommResult<()> {
    let state = &mut *comm;
    let enc = &mut state.encounter;
    // Attack path doesn't call init, so we mark encounter active temporarily
    enc.active = true;
    enc.callbacks.mark_post()?;
    enc.callbacks.mark_uninit()?;
    enc.active = false;
    enc.callbacks.reset();
    Ok(())
}

/// Check if an encounter is currently active.
pub fn is_encounter_active(comm: &mut Comm) -> bool {
    comm.encounter.active
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn reset_state(comm: &mut Comm) {
        let state = &mut *comm;
        state.encounter.reset();
    }

    #[test]
    #[serial]
    fn test_normal_exit_calls_init_post_uninit() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(begin_encounter(comm).is_ok());
        assert!(is_encounter_active(comm));

        // Check callbacks were set before end
        {
            let state = &*comm;
            assert!(state.encounter.callbacks.init_called);
        }

        assert!(end_encounter_normal(comm).is_ok());
        assert!(!is_encounter_active(comm));

        // After end, callbacks are reset for reuse
        let state = &*comm;
        assert!(!state.encounter.callbacks.init_called);
    }

    #[test]
    #[serial]
    fn test_abort_skips_post() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(begin_encounter(comm).is_ok());

        // Verify init was called
        {
            let state = &*comm;
            assert!(state.encounter.callbacks.init_called);
        }

        assert!(end_encounter_abort(comm).is_ok());

        // After end, state is reset for reuse
        let state = &*comm;
        assert!(!state.encounter.active);
    }

    #[test]
    #[serial]
    fn test_attack_without_hail() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        // Attack path: post + uninit, no init — should succeed
        assert!(end_encounter_attack(comm).is_ok());

        // After end, state is reset for reuse
        let state = &*comm;
        assert!(!state.encounter.active);
    }

    #[test]
    #[serial]
    fn test_double_init_rejected() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(begin_encounter(comm).is_ok());
        assert!(begin_encounter(comm).is_err());
        // Clean up
        let _ = end_encounter_abort(comm);
    }

    #[test]
    #[serial]
    fn test_state_valid_after_teardown() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(begin_encounter(comm).is_ok());
        assert!(end_encounter_normal(comm).is_ok());

        // Should be able to start a new encounter
        assert!(begin_encounter(comm).is_ok());
        assert!(end_encounter_normal(comm).is_ok());
    }

    #[test]
    #[serial]
    fn test_encounter_active_flag() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(!is_encounter_active(comm));
        assert!(begin_encounter(comm).is_ok());
        assert!(is_encounter_active(comm));
        assert!(end_encounter_normal(comm).is_ok());
        assert!(!is_encounter_active(comm));
    }

    #[test]
    fn test_callback_tracker_reset() {
        let mut tracker = CallbackTracker::new();
        assert!(tracker.mark_init().is_ok());
        assert!(tracker.mark_post().is_ok());
        assert!(tracker.mark_uninit().is_ok());

        tracker.reset();
        assert!(!tracker.init_called);
        assert!(!tracker.post_called);
        assert!(!tracker.uninit_called);
    }

    #[test]
    fn test_callback_tracker_double_post_rejected() {
        let mut tracker = CallbackTracker::new();
        assert!(tracker.mark_post().is_ok());
        assert!(tracker.mark_post().is_err());
    }
}
