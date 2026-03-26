//! Response list rendering state.
//!
//! Tracks scrolling state for the player response list and delegates actual
//! rendering to C bridge functions. Rust manages visible range, scroll
//! position, and refresh state; C renders the actual text.
//!
//! @plan PLAN-20260314-COMM.P10

use super::response::ResponseSystem;

/// Visible range for the response list, with scroll indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleRange {
    /// Index of the first visible response.
    pub top: usize,
    /// Index one past the last visible response.
    pub bottom: usize,
    /// Whether a scroll-up indicator should be shown.
    pub has_up_indicator: bool,
    /// Whether a scroll-down indicator should be shown.
    pub has_down_indicator: bool,
}

/// Response UI rendering state.
///
/// Tracks the scroll position and refresh flag for the response list.
/// All actual pixel rendering goes through C bridge functions.
#[derive(Debug)]
pub struct ResponseUI {
    /// Index of the topmost visible response in the scrolled list.
    top_response: usize,
    /// Whether the response list needs to be redrawn.
    needs_refresh: bool,
}

impl Default for ResponseUI {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseUI {
    /// Create a new response UI state. Starts needing a refresh.
    pub fn new() -> Self {
        Self {
            top_response: 0,
            needs_refresh: true,
        }
    }

    /// Compute the visible range for a scrolled response list.
    ///
    /// Keeps the selected response visible. Returns up/down indicators when
    /// the list is taller than `max_visible`.
    pub fn calculate_visible_range(
        response_count: usize,
        selected: usize,
        max_visible: usize,
    ) -> VisibleRange {
        if response_count == 0 || max_visible == 0 {
            return VisibleRange {
                top: 0,
                bottom: 0,
                has_up_indicator: false,
                has_down_indicator: false,
            };
        }

        let max_vis = max_visible.min(response_count);

        // Clamp selected to valid range.
        let sel = selected.min(response_count.saturating_sub(1));

        // Compute top so that sel is visible.
        let top = if sel < max_vis { 0 } else { sel - max_vis + 1 };

        let bottom = (top + max_vis).min(response_count);

        VisibleRange {
            top,
            bottom,
            has_up_indicator: top > 0,
            has_down_indicator: bottom < response_count,
        }
    }

    /// Refresh the displayed response list.
    ///
    /// Updates `top_response` to keep `selected` visible, then calls C bridge
    /// to re-render. In test mode, only updates state fields.
    pub fn refresh_responses(&mut self, responses: &ResponseSystem, selected: usize) {
        let count = responses.count();
        // Default max_visible: in production this comes from SIS window height.
        // We use a sentinel of 4 here to keep scroll state consistent with C.
        const MAX_VISIBLE: usize = 4;

        let range = Self::calculate_visible_range(count, selected, MAX_VISIBLE);
        self.top_response = range.top;
        self.needs_refresh = false;

        #[cfg(not(test))]
        unsafe {
            c_bridge::c_RefreshResponses(range.top as u8, count as u8, selected as u8);
        }
    }

    /// Display feedback text for the selected player phrase.
    ///
    /// In production calls C bridge to render the text in the SIS comms
    /// window. In test mode, no-op.
    pub fn draw_feedback(&self, text: &str) {
        #[cfg(not(test))]
        unsafe {
            use std::ffi::CString;
            if let Ok(cs) = CString::new(text) {
                c_bridge::c_FeedbackPlayerPhrase(cs.as_ptr());
            }
        }
        #[cfg(test)]
        {
            let _ = text;
        }
    }

    /// Set the top response index directly.
    pub fn set_top_response(&mut self, top: usize) {
        self.top_response = top;
    }

    /// Index of the topmost visible response.
    pub fn top_response(&self) -> usize {
        self.top_response
    }

    /// Mark the response list as needing a redraw.
    pub fn mark_needs_refresh(&mut self) {
        self.needs_refresh = true;
    }

    /// Whether the response list needs to be redrawn.
    pub fn needs_refresh(&self) -> bool {
        self.needs_refresh
    }

    /// Reset to initial state.
    pub fn clear(&mut self) {
        self.top_response = 0;
        self.needs_refresh = true;
    }
}

// ============================================================================
// C bridge
// ============================================================================

#[cfg(not(test))]
mod c_bridge {
    use std::ffi::c_char;

    extern "C" {
        /// Refresh the response list display.
        /// `top` = topmost visible index, `num_responses` = total count,
        /// `cur_response` = currently selected index.
        pub fn c_RefreshResponses(top: u8, num_responses: u8, cur_response: u8);
        /// Render player feedback text in the SIS comms window.
        pub fn c_FeedbackPlayerPhrase(text: *const c_char);
        /// Redraw the SIS comms window background.
        pub fn c_DrawSISComWindow();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- calculate_visible_range -------------------------------------------

    #[test]
    fn test_visible_range_all_fit() {
        // 3 responses, max_visible 5 — all fit, no indicators.
        let r = ResponseUI::calculate_visible_range(3, 0, 5);
        assert_eq!(r.top, 0);
        assert_eq!(r.bottom, 3);
        assert!(!r.has_up_indicator);
        assert!(!r.has_down_indicator);
    }

    #[test]
    fn test_visible_range_overflow_down() {
        // 6 responses, max_visible 4, selected = 1 — near start, down indicator.
        let r = ResponseUI::calculate_visible_range(6, 1, 4);
        assert_eq!(r.top, 0);
        assert_eq!(r.bottom, 4);
        assert!(!r.has_up_indicator);
        assert!(r.has_down_indicator);
    }

    #[test]
    fn test_visible_range_overflow_up() {
        // 6 responses, max_visible 4, selected = 5 — near end, up indicator.
        let r = ResponseUI::calculate_visible_range(6, 5, 4);
        assert_eq!(r.top, 2);
        assert_eq!(r.bottom, 6);
        assert!(r.has_up_indicator);
        assert!(!r.has_down_indicator);
    }

    #[test]
    fn test_visible_range_both_indicators() {
        // 8 responses, max_visible 3, selected = 4 — middle, both indicators.
        let r = ResponseUI::calculate_visible_range(8, 4, 3);
        // top = 4 - 3 + 1 = 2, bottom = 5
        assert_eq!(r.top, 2);
        assert_eq!(r.bottom, 5);
        assert!(r.has_up_indicator);
        assert!(r.has_down_indicator);
    }

    // ---- ResponseUI state --------------------------------------------------

    #[test]
    fn test_needs_refresh_default() {
        let ui = ResponseUI::new();
        assert!(ui.needs_refresh(), "should start needing refresh");
    }

    #[test]
    fn test_clear_resets() {
        let mut ui = ResponseUI::new();
        ui.set_top_response(3);
        ui.needs_refresh = false;

        ui.clear();

        assert_eq!(ui.top_response(), 0);
        assert!(ui.needs_refresh());
    }
}
