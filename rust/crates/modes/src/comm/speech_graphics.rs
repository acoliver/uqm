//! Speech graphics state — oscilloscope and slider rendering state.
//!
//! Tracks display state for the oscilloscope waveform visualizer and playback
//! slider using retained conversation frames and supplied drawing contexts.
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
/// Tracks widget state and rate limits rendering on the conversation contexts.
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

    /// Initialize the retained oscilloscope and slider with the play image.
    pub fn init(comm: &mut Comm) {
        comm.speech_graphics_mut().initialized = true;
        comm.speech_graphics_mut().slider_state = SliderState::Play;
        comm.speech_graphics_mut().last_update_ms = 0;

        let activity =
            crate::graphics::drawable::ContextFrame::retain(comm.presentation.activity_frame);
        super::oscill_abi::runtime::init_oscilloscope(comm, activity.wrapped_frame(9));
        super::oscill_abi::runtime::init_slider(
            comm,
            0,
            107,
            242,
            activity.wrapped_frame(5),
            activity.wrapped_frame(2),
        );
    }

    /// Select the retained image for the next slider redraw.
    pub fn set_slider_state(comm: &mut Comm, state: SliderState) {
        comm.speech_graphics_mut().slider_state = state;

        let frame =
            crate::graphics::drawable::ContextFrame::retain(comm.presentation.activity_frame)
                .wrapped_frame(state.frame_index() as u16);
        super::oscill_abi::runtime::set_slider_image(comm, frame);
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

        comm.draw_speech_widgets();
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
        uqm_engine::host::install_test(true);
        uqm_engine::host::advance_to(42);
        assert_eq!(update_timestamp(0), 50);
        assert_eq!(update_timestamp(1_000_000), 50);
        uqm_engine::host::advance_to(84);
        assert_eq!(update_timestamp(1_000_000), 100);
        Ok(())
    }

    #[test]
    fn test_not_initialized_by_default() {
        let sg = SpeechGraphics::new();
        assert!(!sg.is_initialized());
    }

    #[test]
    #[serial_test::serial]
    fn test_init_sets_play_state() {
        super::super::sis_graphics_font_tests::with_display(
            "comm::speech_graphics::tests::test_init_sets_play_state",
            |comm, _| {
                SpeechGraphics::init(comm);
                assert!(comm.speech_graphics().is_initialized());
                assert_eq!(comm.speech_graphics().slider_state(), SliderState::Play);
            },
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_slider_state_changes() {
        super::super::sis_graphics_font_tests::with_display(
            "comm::speech_graphics::tests::test_slider_state_changes",
            |comm, _| {
                SpeechGraphics::init(comm);

                SpeechGraphics::set_slider_state(comm, SliderState::Forward);
                assert_eq!(comm.speech_graphics().slider_state(), SliderState::Forward);

                SpeechGraphics::set_slider_state(comm, SliderState::Reverse);
                assert_eq!(comm.speech_graphics().slider_state(), SliderState::Reverse);

                SpeechGraphics::set_slider_state(comm, SliderState::Stop);
                assert_eq!(comm.speech_graphics().slider_state(), SliderState::Stop);

                SpeechGraphics::set_slider_state(comm, SliderState::Play);
                assert_eq!(comm.speech_graphics().slider_state(), SliderState::Play);
            },
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_update_rate_limited() {
        super::super::sis_graphics_font_tests::with_display(
            "comm::speech_graphics::tests::test_update_rate_limited",
            |comm, _| {
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
            },
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_clear_resets() {
        super::super::sis_graphics_font_tests::with_display(
            "comm::speech_graphics::tests::test_clear_resets",
            |comm, _| {
                SpeechGraphics::init(comm);
                SpeechGraphics::set_slider_state(comm, SliderState::Stop);

                comm.speech_graphics_mut().clear();

                assert!(!comm.speech_graphics().is_initialized());
                assert_eq!(comm.speech_graphics().slider_state(), SliderState::Play);
                assert_eq!(comm.speech_graphics().last_update_ms, 0);
            },
        );
    }

    #[test]
    fn test_update_noop_when_not_initialized() {
        let comm = &mut Comm::new();

        // Should not update last_update_ms if not initialized.
        SpeechGraphics::update(comm, 999);
        assert_eq!(comm.speech_graphics().last_update_ms, 0);
    }
}
