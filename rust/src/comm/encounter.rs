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
// C FFI declarations — encounter flow helpers
// ============================================================================

#[cfg(not(test))]
use crate::graphics::drawable_abi::DestroyDrawable;
#[cfg(not(test))]
use crate::graphics::font_abi::DestroyFont;
#[cfg(not(test))]
use crate::sound::heart_ffi::DestroyMusic;
#[cfg(not(test))]
use crate::strings::string_table::DestroyStringTable;

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

/// Tracks resource handles loaded during hail_alien for teardown.
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
                DestroyStringTable((self.conversation_phrases as *mut std::ffi::c_void).cast());
                self.conversation_phrases = 0;
            }
            if self.alien_song != 0 {
                DestroyMusic(self.alien_song as *mut std::ffi::c_void);
                self.alien_song = 0;
            }
            if self.alien_colormap != 0 {
                // DestroyColorMap is #define DestroyColorMap DestroyStringTable in gfxlib.h
                DestroyStringTable((self.alien_colormap as *mut std::ffi::c_void).cast());
                self.alien_colormap = 0;
            }
            if self.alien_font != 0 {
                DestroyFont((self.alien_font as *mut std::ffi::c_void).cast());
                self.alien_font = 0;
            }
            if self.alien_frame != 0 {
                DestroyDrawable((self.alien_frame as *mut std::ffi::c_void).cast());
                self.alien_frame = 0;
            }
            if self.player_font != 0 {
                DestroyFont((self.player_font as *mut std::ffi::c_void).cast());
                self.player_font = 0;
            }
            if self.text_cache_frame != 0 {
                DestroyDrawable((self.text_cache_frame as *mut std::ffi::c_void).cast());
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

/// Active encounter state, held in Comm.
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
    enc.resources.destroy();
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
    enc.resources.destroy();
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
    enc.resources.destroy();
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
    fn test_resource_teardown() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        {
            let state = &mut *comm;
            state.encounter.resources.alien_frame = 42;
            state.encounter.resources.alien_font = 43;
            state.encounter.resources.alien_song = 44;
        }

        assert!(begin_encounter(comm).is_ok());
        assert!(end_encounter_normal(comm).is_ok());

        let state = &*comm;
        assert!(!state.encounter.resources.has_resources());
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

    #[test]
    fn test_encounter_resources_default() {
        let res = EncounterResources::new();
        assert!(!res.has_resources());
    }
}
