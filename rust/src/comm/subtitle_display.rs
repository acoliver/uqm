//! Subtitle display state with change detection.
//!
//! Tracks the currently visible subtitle text and detects changes that
//! require the subtitle display to be cleared or redrawn. Actual rendering
//! is delegated to C bridge functions.
//!
//! @plan PLAN-20260314-COMM.P10

/// Subtitle display state with change detection.
///
/// Rust owns the current text and dirty flags; all pixel rendering is
/// performed by C through the bridge functions below.
#[derive(Debug, Default)]
pub struct SubtitleDisplay {
    /// The subtitle text currently on-screen, if any.
    current_text: Option<String>,
    /// Whether the display needs to be cleared on the next render pass.
    needs_clear: bool,
    /// Y baseline of the last rendered subtitle (for C-side layout).
    last_baseline_y: i32,
}

impl SubtitleDisplay {
    /// Create a new, empty subtitle display state.
    pub fn new() -> Self {
        Self {
            current_text: None,
            needs_clear: false,
            last_baseline_y: 0,
        }
    }

    /// Check whether the subtitle has changed.
    ///
    /// Returns `true` if `new_text` differs from the current text.  Updates
    /// `current_text` and marks `needs_clear` when a change is detected.
    pub fn check_subtitle(&mut self, new_text: Option<&str>) -> bool {
        let changed = match (&self.current_text, new_text) {
            (None, None) => false,
            (Some(cur), Some(new)) => cur.as_str() != new,
            _ => true,
        };

        if changed {
            self.current_text = new_text.map(|s| s.to_owned());
            self.needs_clear = true;
        }

        changed
    }

    /// Mark the display as needing to be cleared and remove current text.
    pub fn clear(&mut self) {
        self.needs_clear = true;
        self.current_text = None;
    }

    /// Redraw the current subtitle via C bridge.
    ///
    /// Does nothing if there is no current text. In test mode, no C calls are
    /// made.
    pub fn redraw(&self) {
        if self.current_text.is_none() {
            return;
        }

        #[cfg(not(test))]
        unsafe {
            c_bridge::c_RedrawSubtitles();
        }
    }

    /// Whether the display needs to be cleared before the next draw.
    pub fn needs_clear(&self) -> bool {
        self.needs_clear
    }

    /// The subtitle text currently tracked by this display, if any.
    pub fn current_text(&self) -> Option<&str> {
        self.current_text.as_deref()
    }

    /// Acknowledge that the clear has been performed.
    pub fn clear_acknowledged(&mut self) {
        self.needs_clear = false;
    }

    /// Set the last baseline Y position (used by C-side layout).
    pub fn set_last_baseline_y(&mut self, y: i32) {
        self.last_baseline_y = y;
    }

    /// Last baseline Y position of the rendered subtitle.
    pub fn last_baseline_y(&self) -> i32 {
        self.last_baseline_y
    }
}

// ============================================================================
// C bridge
// ============================================================================

#[cfg(not(test))]
mod c_bridge {
    extern "C" {
        /// Clear the subtitle display area.
        pub fn c_ClearSubtitles();
        /// Check subtitle timing and update display if needed.
        pub fn c_CheckSubtitles();
        /// Redraw the current subtitle text.
        pub fn c_RedrawSubtitles();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_detection_same_text() {
        let mut d = SubtitleDisplay::new();
        d.check_subtitle(Some("Hello"));
        // Reset needs_clear so we can distinguish a second call.
        d.needs_clear = false;

        let changed = d.check_subtitle(Some("Hello"));
        assert!(!changed, "same text should not be detected as changed");
        assert!(!d.needs_clear());
    }

    #[test]
    fn test_change_detection_new_text() {
        let mut d = SubtitleDisplay::new();
        let changed = d.check_subtitle(Some("Hello"));
        assert!(changed, "None → Some should be detected as changed");
        assert!(d.needs_clear());
        assert_eq!(d.current_text(), Some("Hello"));
    }

    #[test]
    fn test_clear_sets_needs_clear() {
        let mut d = SubtitleDisplay::new();
        d.check_subtitle(Some("Hello"));
        d.needs_clear = false; // pretend we just rendered

        d.clear();

        assert!(d.needs_clear());
        assert!(d.current_text().is_none());
    }

    #[test]
    fn test_check_from_none_to_some() {
        let mut d = SubtitleDisplay::new();
        assert!(d.current_text().is_none());

        let changed = d.check_subtitle(Some("First line"));
        assert!(changed);
        assert_eq!(d.current_text(), Some("First line"));
    }

    #[test]
    fn test_check_from_some_to_none() {
        let mut d = SubtitleDisplay::new();
        d.check_subtitle(Some("Hello"));
        d.needs_clear = false;

        let changed = d.check_subtitle(None);
        assert!(changed, "Some → None should be detected as changed");
        assert!(d.current_text().is_none());
        assert!(d.needs_clear());
    }
}
