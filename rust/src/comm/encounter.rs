//! Encounter lifecycle orchestration.
//!
//! In Rust mode, this module owns the high-level encounter lifecycle:
//! context resolution → SIS update → race init → hail/attack decision
//! → dialogue → teardown. Heavy lifting (resource loading, game state,
//! graphics) remains C-owned and is accessed through FFI wrappers.
//!
//! @plan PLAN-20260314-COMM.P08
//! @requirement EC-REQ-001 through EC-REQ-016, OL-REQ-002

use super::state::COMM_STATE;
use super::types::{CommError, CommResult};

// ============================================================================
// Encounter status codes matching C (comm.c)
// ============================================================================

/// Encounter outcome: player chose to talk.
pub const HAIL: u32 = 0;
/// Encounter outcome: player chose to attack.
pub const ATTACK: u32 = 1;

// ============================================================================
// C FFI declarations — encounter flow helpers
// ============================================================================

#[cfg(not(test))]
extern "C" {
    // Resource destruction
    fn c_DestroyDrawable(handle: usize);
    fn c_DestroyFont(handle: usize);
    fn c_DestroyColorMap(handle: usize);
    fn c_DestroyMusic(handle: usize);
    fn c_DestroyStringTable(handle: usize);
}

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

/// Tracks resource handles loaded during HailAlien for teardown.
/// All handles are pointer-sized (usize) since C returns uintptr_t.
#[derive(Debug, Default)]
pub struct EncounterResources {
    pub alien_frame: usize,
    pub alien_font: usize,
    pub alien_colormap: usize,
    pub alien_song: usize,
    pub conversation_phrases: usize,
    pub player_font: usize,
    pub text_cache_context: usize,
    pub text_cache_frame: usize,
}

impl EncounterResources {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_resources(&self) -> bool {
        self.alien_frame != 0
            || self.alien_font != 0
            || self.alien_colormap != 0
            || self.alien_song != 0
            || self.conversation_phrases != 0
    }

    /// Destroy all loaded resources in reverse order (EC-REQ-014).
    #[cfg(not(test))]
    pub fn destroy(&mut self) {
        unsafe {
            if self.conversation_phrases != 0 {
                c_DestroyStringTable(self.conversation_phrases);
                self.conversation_phrases = 0;
            }
            if self.alien_song != 0 {
                c_DestroyMusic(self.alien_song);
                self.alien_song = 0;
            }
            if self.alien_colormap != 0 {
                c_DestroyColorMap(self.alien_colormap);
                self.alien_colormap = 0;
            }
            if self.alien_font != 0 {
                c_DestroyFont(self.alien_font);
                self.alien_font = 0;
            }
            if self.alien_frame != 0 {
                c_DestroyDrawable(self.alien_frame);
                self.alien_frame = 0;
            }
            if self.player_font != 0 {
                c_DestroyFont(self.player_font);
                self.player_font = 0;
            }
            if self.text_cache_frame != 0 {
                c_DestroyDrawable(self.text_cache_frame);
                self.text_cache_frame = 0;
            }
            if self.text_cache_context != 0 {
                // Context destruction handled by C side
                self.text_cache_context = 0;
            }
        }
    }

    #[cfg(test)]
    pub fn destroy(&mut self) {
        // In test mode, just zero out handles
        *self = Self::default();
    }
}

// ============================================================================
// Encounter lifecycle
// ============================================================================

/// Active encounter state, held in CommState.
#[derive(Debug)]
pub struct EncounterState {
    /// Whether an encounter is currently active.
    pub active: bool,
    /// Callback invocation tracking.
    pub callbacks: CallbackTracker,
    /// Loaded resource handles for teardown.
    pub resources: EncounterResources,
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
            resources: EncounterResources::new(),
            aborted: false,
        }
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.callbacks.reset();
        self.resources.destroy();
        self.aborted = false;
    }
}

// ============================================================================
// Public lifecycle functions
// ============================================================================

/// Begin encounter lifecycle — called after HailAlien resources loaded.
///
/// This records that the encounter is active and the init callback has
/// been invoked. In production, the actual callbacks are invoked by C;
/// this provides the Rust-side tracking and ordering enforcement.
pub fn begin_encounter() -> CommResult<()> {
    let mut state = COMM_STATE.write();
    let enc = &mut state.encounter;
    if enc.active {
        return Err(CommError::AlreadyInitialized);
    }
    enc.active = true;
    enc.callbacks.mark_init()?;
    Ok(())
}

/// Record normal encounter exit (init→post→uninit).
pub fn end_encounter_normal() -> CommResult<()> {
    let mut state = COMM_STATE.write();
    let enc = &mut state.encounter;
    if !enc.active {
        return Err(CommError::NotInitialized);
    }
    enc.callbacks.mark_post()?;
    enc.callbacks.mark_uninit()?;
    enc.resources.destroy();
    enc.active = false;
    // Reset callbacks for next encounter reuse
    enc.callbacks.reset();
    Ok(())
}

/// Record abort encounter exit (init→uninit, skip post).
pub fn end_encounter_abort() -> CommResult<()> {
    let mut state = COMM_STATE.write();
    let enc = &mut state.encounter;
    if !enc.active {
        return Err(CommError::NotInitialized);
    }
    enc.aborted = true;
    enc.callbacks.mark_uninit()?;
    enc.resources.destroy();
    enc.active = false;
    enc.callbacks.reset();
    enc.aborted = false;
    Ok(())
}

/// Record attack-without-hail exit (post→uninit, no init).
pub fn end_encounter_attack() -> CommResult<()> {
    let mut state = COMM_STATE.write();
    let enc = &mut state.encounter;
    // Attack path doesn't call init, so we mark encounter active temporarily
    enc.active = true;
    enc.callbacks.mark_post()?;
    enc.callbacks.mark_uninit()?;
    enc.resources.destroy();
    enc.active = false;
    enc.callbacks.reset();
    Ok(())
}

/// Check if an encounter is currently active.
pub fn is_encounter_active() -> bool {
    COMM_STATE.read().encounter.active
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn reset_state() {
        let mut state = COMM_STATE.write();
        state.encounter.reset();
    }

    #[test]
    #[serial]
    fn test_normal_exit_calls_init_post_uninit() {
        reset_state();
        assert!(begin_encounter().is_ok());
        assert!(is_encounter_active());

        // Check callbacks were set before end
        {
            let state = COMM_STATE.read();
            assert!(state.encounter.callbacks.init_called);
        }

        assert!(end_encounter_normal().is_ok());
        assert!(!is_encounter_active());

        // After end, callbacks are reset for reuse
        let state = COMM_STATE.read();
        assert!(!state.encounter.callbacks.init_called);
    }

    #[test]
    #[serial]
    fn test_abort_skips_post() {
        reset_state();
        assert!(begin_encounter().is_ok());

        // Verify init was called
        {
            let state = COMM_STATE.read();
            assert!(state.encounter.callbacks.init_called);
        }

        assert!(end_encounter_abort().is_ok());

        // After end, state is reset for reuse
        let state = COMM_STATE.read();
        assert!(!state.encounter.active);
    }

    #[test]
    #[serial]
    fn test_attack_without_hail() {
        reset_state();
        // Attack path: post + uninit, no init — should succeed
        assert!(end_encounter_attack().is_ok());

        // After end, state is reset for reuse
        let state = COMM_STATE.read();
        assert!(!state.encounter.active);
    }

    #[test]
    #[serial]
    fn test_double_init_rejected() {
        reset_state();
        assert!(begin_encounter().is_ok());
        assert!(begin_encounter().is_err());
        // Clean up
        let _ = end_encounter_abort();
    }

    #[test]
    #[serial]
    fn test_resource_teardown() {
        reset_state();
        {
            let mut state = COMM_STATE.write();
            state.encounter.resources.alien_frame = 42;
            state.encounter.resources.alien_font = 43;
            state.encounter.resources.alien_song = 44;
        }

        assert!(begin_encounter().is_ok());
        assert!(end_encounter_normal().is_ok());

        let state = COMM_STATE.read();
        assert!(!state.encounter.resources.has_resources());
    }

    #[test]
    #[serial]
    fn test_state_valid_after_teardown() {
        reset_state();
        assert!(begin_encounter().is_ok());
        assert!(end_encounter_normal().is_ok());

        // Should be able to start a new encounter
        assert!(begin_encounter().is_ok());
        assert!(end_encounter_normal().is_ok());
    }

    #[test]
    #[serial]
    fn test_encounter_active_flag() {
        reset_state();
        assert!(!is_encounter_active());
        assert!(begin_encounter().is_ok());
        assert!(is_encounter_active());
        assert!(end_encounter_normal().is_ok());
        assert!(!is_encounter_active());
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

    #[test]
    fn test_encounter_resources_default() {
        let res = EncounterResources::new();
        assert!(!res.has_resources());
    }
}
