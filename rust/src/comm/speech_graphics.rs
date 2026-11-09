//! Speech graphics state — oscilloscope and slider rendering state.
//!
//! Tracks display state for the oscilloscope waveform visualizer and playback
//! slider. Actual pixel rendering is delegated to C bridge functions; Rust
//! manages the state and rate-limiting logic only.
//!
//! @plan PLAN-20260314-COMM.P10

use crate::comm::Comm;

// Host-time rate limiting otherwise changes whether a speech frame queues a
// drawing command, which changes replay presentation counts.
pub(super) fn update_timestamp(real_ms: u64) -> u64 {
    if super::talk_segue::c_bridge::clock::is_replay() {
        u64::from(super::talk_segue::c_bridge::clock::now_ms())
    } else {
        real_ms
    }
}

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
    pub fn init(comm: &mut Comm) {
        comm.speech_graphics_mut().initialized = true;
        comm.speech_graphics_mut().slider_state = SliderState::Play;
        comm.speech_graphics_mut().last_update_ms = 0;

        #[cfg(not(test))]
        unsafe {
            let oscilloscope = activity_frame(comm, 9);
            let slider = activity_frame(comm, 5);
            let play = activity_frame(comm, SliderState::Play.frame_index() as u16);
            c_bridge::init_oscilloscope(comm, (oscilloscope).cast());
            c_bridge::init_slider(comm, 0, 107, 242, slider.cast(), play.cast());
        }
    }

    /// Update slider state and notify C bridge to redraw the slider image.
    pub fn set_slider_state(comm: &mut Comm, state: SliderState) {
        comm.speech_graphics_mut().slider_state = state;

        #[cfg(not(test))]
        {
            // SAFETY: the conversation retains the initialized activity drawable on its graphics thread.
            let frame = unsafe { activity_frame(comm, state.frame_index() as u16) };
            c_bridge::set_slider_image(comm, frame.cast());
        }
    }

    /// Rate-limited update: redraws oscilloscope and slider if enough time has
    /// passed since the last update. `now_ms` is the current timestamp in
    /// milliseconds (caller provides to avoid platform dependencies here).
    pub fn update(comm: &mut Comm, now_ms: u64) {
        if !comm.speech_graphics_mut().initialized {
            return;
        }

        if now_ms.saturating_sub(comm.speech_graphics_mut().last_update_ms) < OSCILLOSCOPE_RATE_MS {
            return;
        }

        comm.speech_graphics_mut().last_update_ms = now_ms;

        #[cfg(not(test))]
        unsafe {
            let old_context = c_bridge::SetContext(comm.presentation.radar_context);
            c_bridge::draw_oscilloscope(comm);
            c_bridge::SetContext(comm.presentation.space_context);
            c_bridge::draw_slider(comm);
            c_bridge::SetContext(old_context);
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
unsafe fn activity_frame(comm: &Comm, index: u16) -> *mut std::ffi::c_void {
    (c_bridge::SetAbsFrameIndex((comm.presentation.activity_frame).cast(), index)).cast()
}

#[cfg(not(test))]
mod c_bridge {

    pub use crate::comm::oscill_abi::runtime::init_oscilloscope;

    pub use crate::comm::oscill_abi::runtime::init_slider;

    pub use crate::graphics::pixmap_abi::SetAbsFrameIndex;

    pub use crate::comm::oscill_abi::runtime::set_slider_image;

    pub use crate::graphics::context_abi::SetContext;

    pub use crate::comm::oscill_abi::runtime::draw_oscilloscope;

    pub use crate::comm::oscill_abi::runtime::draw_slider;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_speech_redraw_clock_ignores_host_timestamp() -> Result<(), Box<dyn std::error::Error>>
    {
        const CHILD: &str = "UQM_REPLAY_SPEECH_REDRAW_CLOCK";
        if std::env::var_os(CHILD).is_none() {
            assert_eq!(update_timestamp(1234), 1234);
            let status = std::process::Command::new(std::env::current_exe()?)
                .args(["--exact", "comm::speech_graphics::tests::replay_speech_redraw_clock_ignores_host_timestamp"])
                .env(CHILD, "1")
                .status()?;
            assert!(status.success());
            return Ok(());
        }
        crate::time::source::install(true);
        crate::time::source::advance_to(42);
        assert_eq!(update_timestamp(0), 50);
        assert_eq!(update_timestamp(1_000_000), 50);
        crate::time::source::advance_to(84);
        assert_eq!(update_timestamp(1_000_000), 100);
        Ok(())
    }

    #[test]
    fn test_not_initialized_by_default() {
        let sg = SpeechGraphics::new();
        assert!(!sg.is_initialized());
    }

    #[test]
    fn test_init_sets_play_state() {
        let comm = &mut Comm::new();
        SpeechGraphics::init(comm);
        assert!(comm.speech_graphics().is_initialized());
        assert_eq!(comm.speech_graphics().slider_state(), SliderState::Play);
    }

    #[test]
    fn test_slider_state_changes() {
        let comm = &mut Comm::new();
        SpeechGraphics::init(comm);

        SpeechGraphics::set_slider_state(comm, SliderState::Forward);
        assert_eq!(comm.speech_graphics().slider_state(), SliderState::Forward);

        SpeechGraphics::set_slider_state(comm, SliderState::Reverse);
        assert_eq!(comm.speech_graphics().slider_state(), SliderState::Reverse);

        SpeechGraphics::set_slider_state(comm, SliderState::Stop);
        assert_eq!(comm.speech_graphics().slider_state(), SliderState::Stop);

        SpeechGraphics::set_slider_state(comm, SliderState::Play);
        assert_eq!(comm.speech_graphics().slider_state(), SliderState::Play);
    }

    #[test]
    fn test_update_rate_limited() {
        let comm = &mut Comm::new();
        SpeechGraphics::init(comm);

        // First update at t=0 — should proceed (last_update_ms starts at 0).
        SpeechGraphics::update(comm, 100);
        assert_eq!(comm.speech_graphics().last_update_ms, 100);

        // Update 1ms later — less than OSCILLOSCOPE_RATE_MS — should be skipped.
        SpeechGraphics::update(comm, 101);
        assert_eq!(
            comm.speech_graphics().last_update_ms,
            100,
            "update should be rate-limited"
        );

        // Update after enough time has passed.
        SpeechGraphics::update(comm, 100 + OSCILLOSCOPE_RATE_MS);
        assert_eq!(
            comm.speech_graphics().last_update_ms,
            100 + OSCILLOSCOPE_RATE_MS
        );
    }

    #[test]
    fn test_clear_resets() {
        let comm = &mut Comm::new();
        SpeechGraphics::init(comm);
        SpeechGraphics::set_slider_state(comm, SliderState::Stop);

        comm.speech_graphics_mut().clear();

        assert!(!comm.speech_graphics().is_initialized());
        assert_eq!(comm.speech_graphics().slider_state(), SliderState::Play);
        assert_eq!(comm.speech_graphics().last_update_ms, 0);
    }

    #[test]
    fn test_update_noop_when_not_initialized() {
        let comm = &mut Comm::new();

        // Should not update last_update_ms if not initialized.
        SpeechGraphics::update(comm, 999);
        assert_eq!(comm.speech_graphics().last_update_ms, 0);
    }
}
