// Phrase enable/disable tracking — encounter-local state
// @plan PLAN-20260314-COMM.P04
// @requirement PS-REQ-001, PS-REQ-002, PS-REQ-003, PS-REQ-004, PS-REQ-005, PS-REQ-006, PS-REQ-007

use std::collections::HashSet;

/// Encounter-local phrase enable/disable state.
///
/// All phrases start enabled. `disable()` marks a phrase disabled for the
/// remainder of the current encounter. There is no re-enable path within
/// an encounter. `reset()` clears all disable state for the next encounter.
#[derive(Debug, Clone)]
pub struct PhraseState {
    disabled: HashSet<i32>,
}

impl Default for PhraseState {
    fn default() -> Self {
        Self::new()
    }
}

impl PhraseState {
    pub fn new() -> Self {
        Self {
            disabled: HashSet::new(),
        }
    }

    /// Check if a phrase is enabled (not yet disabled this encounter).
    pub fn is_enabled(&self, index: i32) -> bool {
        !self.disabled.contains(&index)
    }

    /// Disable a phrase for the remainder of this encounter.
    /// Disabling an already-disabled phrase is a no-op.
    pub fn disable(&mut self, index: i32) {
        self.disabled.insert(index);
    }

    /// Reset all phrase state (called at encounter teardown/start).
    pub fn reset(&mut self) {
        self.disabled.clear();
    }

    /// Number of disabled phrases (for diagnostics/testing).
    pub fn disabled_count(&self) -> usize {
        self.disabled.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_state_all_enabled() {
        let state = PhraseState::new();
        assert!(state.is_enabled(1));
        assert!(state.is_enabled(100));
        assert!(state.is_enabled(-1));
        assert_eq!(state.disabled_count(), 0);
    }

    #[test]
    fn disable_makes_phrase_not_enabled() {
        let mut state = PhraseState::new();
        state.disable(5);
        assert!(!state.is_enabled(5));
        assert!(state.is_enabled(3));
    }

    #[test]
    fn disable_is_idempotent() {
        let mut state = PhraseState::new();
        state.disable(5);
        state.disable(5);
        assert_eq!(state.disabled_count(), 1);
        assert!(!state.is_enabled(5));
    }

    #[test]
    fn no_re_enable_within_encounter() {
        let mut state = PhraseState::new();
        state.disable(7);
        // No re-enable API exists — phrase stays disabled
        assert!(!state.is_enabled(7));
    }

    #[test]
    fn reset_clears_all_disabled() {
        let mut state = PhraseState::new();
        state.disable(1);
        state.disable(2);
        state.disable(3);
        assert_eq!(state.disabled_count(), 3);

        state.reset();
        assert_eq!(state.disabled_count(), 0);
        assert!(state.is_enabled(1));
        assert!(state.is_enabled(2));
        assert!(state.is_enabled(3));
    }

    #[test]
    fn encounter_isolation() {
        let mut state = PhraseState::new();
        // Encounter 1: disable phrase 5
        state.disable(5);
        assert!(!state.is_enabled(5));

        // End encounter 1
        state.reset();

        // Encounter 2: phrase 5 should be enabled again
        assert!(state.is_enabled(5));
    }

    #[test]
    fn negative_and_zero_indices() {
        let mut state = PhraseState::new();
        state.disable(0);
        state.disable(-3);
        assert!(!state.is_enabled(0));
        assert!(!state.is_enabled(-3));
        assert!(state.is_enabled(1));
    }
}
