//! Speech graphics state — oscilloscope and slider rendering state.
//!
//! Tracks display state for the oscilloscope waveform visualizer and playback
//! slider. Actual pixel rendering is delegated to C bridge functions; Rust
//! manages the state and rate-limiting logic only.
//!
//! @plan PLAN-20260314-COMM.P10

use super::oscilloscope::Oscilloscope;

/// Update rate for oscilloscope: 1/32 second in milliseconds.
const OSCILLOSCOPE_RATE_MS: u64 = 1000 / 32;

/// Slider display states matching C ActivityFrame indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderState {
    /// Frame index 2 — normal playback.
    Play,
    /// Frame index 3 — fast-forward.
    Forward,
    /// Frame index 4 — rewind/reverse.
    Reverse,
    /// Frame index 8 — stopped.
    Stop,
}

impl SliderState {
    /// C ActivityFrame index for this slider state.
    pub fn frame_index(self) -> u32 {
        match self {
            SliderState::Play => 2,
            SliderState::Forward => 3,
            SliderState::Reverse => 4,
            SliderState::Stop => 8,
        }
    }
}

/// Speech graphics state (oscilloscope + slider).
///
/// Tracks what the speech graphics subsystem is currently showing. All actual
/// drawing is delegated to C bridge functions via FFI.
#[derive(Debug)]
pub struct SpeechGraphics {
    initialized: bool,
    slider_state: SliderState,
    /// Last update timestamp in milliseconds (for rate limiting).
    last_update_ms: u64,
}

impl Default for SpeechGraphics {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeechGraphics {
    /// Create a new, uninitialized speech graphics state.
    pub fn new() -> Self {
        Self {
            initialized: false,
            slider_state: SliderState::Play,
            last_update_ms: 0,
        }
    }

    /// Initialize speech graphics. Sets slider to Play and calls C bridge
    /// to initialize oscilloscope and slider widgets.
    pub fn init(&mut self) {
        self.initialized = true;
        self.slider_state = SliderState::Play;
        self.last_update_ms = 0;

        #[cfg(not(test))]
        unsafe {
            // Frame index 9 is the oscilloscope background frame.
            c_bridge::c_InitOscilloscope(9);
            // Slider defaults: position 0,0, full width, background frame 0,
            // cursor frame 1. Production code fills in real values from CommData.
            c_bridge::c_InitSlider(0, 0, 0, 0, 1);
        }
    }

    /// Update slider state and notify C bridge to redraw the slider image.
    pub fn set_slider_state(&mut self, state: SliderState) {
        self.slider_state = state;

        #[cfg(not(test))]
        unsafe {
            c_bridge::c_SetSliderImage(state.frame_index());
        }
    }

    /// Rate-limited update: redraws oscilloscope and slider if enough time has
    /// passed since the last update. `now_ms` is the current timestamp in
    /// milliseconds (caller provides to avoid platform dependencies here).
    pub fn update(&mut self, osc: &Oscilloscope, now_ms: u64) {
        if !self.initialized {
            return;
        }

        if now_ms.saturating_sub(self.last_update_ms) < OSCILLOSCOPE_RATE_MS {
            return;
        }

        self.last_update_ms = now_ms;

        // osc is passed for potential Rust-side logic; actual drawing is C.
        let _ = osc;

        #[cfg(not(test))]
        unsafe {
            c_bridge::c_DrawOscilloscope();
            c_bridge::c_DrawSlider();
        }
    }

    /// Reset to uninitialized state.
    pub fn clear(&mut self) {
        self.initialized = false;
        self.slider_state = SliderState::Play;
        self.last_update_ms = 0;
    }

    /// Whether speech graphics have been initialized this encounter.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Current slider display state.
    pub fn slider_state(&self) -> SliderState {
        self.slider_state
    }
}

// ============================================================================
// C bridge
// ============================================================================

#[cfg(not(test))]
mod c_bridge {
    extern "C" {
        /// Initialize oscilloscope display. `frame` is the background frame index.
        pub fn c_InitOscilloscope(frame: u32);
        /// Initialize slider widget.
        pub fn c_InitSlider(x: i32, y: i32, w: i32, bg_frame: u32, cursor_frame: u32);
        /// Set slider display image by ActivityFrame index.
        pub fn c_SetSliderImage(frame: u32);
        /// Redraw oscilloscope waveform.
        pub fn c_DrawOscilloscope();
        /// Redraw slider widget.
        pub fn c_DrawSlider();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_initialized_by_default() {
        let sg = SpeechGraphics::new();
        assert!(!sg.is_initialized());
    }

    #[test]
    fn test_init_sets_play_state() {
        let mut sg = SpeechGraphics::new();
        sg.init();
        assert!(sg.is_initialized());
        assert_eq!(sg.slider_state(), SliderState::Play);
    }

    #[test]
    fn test_slider_state_changes() {
        let mut sg = SpeechGraphics::new();
        sg.init();

        sg.set_slider_state(SliderState::Forward);
        assert_eq!(sg.slider_state(), SliderState::Forward);

        sg.set_slider_state(SliderState::Reverse);
        assert_eq!(sg.slider_state(), SliderState::Reverse);

        sg.set_slider_state(SliderState::Stop);
        assert_eq!(sg.slider_state(), SliderState::Stop);

        sg.set_slider_state(SliderState::Play);
        assert_eq!(sg.slider_state(), SliderState::Play);
    }

    #[test]
    fn test_update_rate_limited() {
        let mut sg = SpeechGraphics::new();
        sg.init();
        let osc = Oscilloscope::new();

        // First update at t=0 — should proceed (last_update_ms starts at 0).
        sg.update(&osc, 100);
        assert_eq!(sg.last_update_ms, 100);

        // Update 1ms later — less than OSCILLOSCOPE_RATE_MS — should be skipped.
        sg.update(&osc, 101);
        assert_eq!(sg.last_update_ms, 100, "update should be rate-limited");

        // Update after enough time has passed.
        sg.update(&osc, 100 + OSCILLOSCOPE_RATE_MS);
        assert_eq!(sg.last_update_ms, 100 + OSCILLOSCOPE_RATE_MS);
    }

    #[test]
    fn test_clear_resets() {
        let mut sg = SpeechGraphics::new();
        sg.init();
        sg.set_slider_state(SliderState::Stop);

        sg.clear();

        assert!(!sg.is_initialized());
        assert_eq!(sg.slider_state(), SliderState::Play);
        assert_eq!(sg.last_update_ms, 0);
    }

    #[test]
    fn test_update_noop_when_not_initialized() {
        let mut sg = SpeechGraphics::new();
        let osc = Oscilloscope::new();

        // Should not update last_update_ms if not initialized.
        sg.update(&osc, 999);
        assert_eq!(sg.last_update_ms, 0);
    }
}
