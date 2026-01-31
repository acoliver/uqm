//! Communication state management
//!
//! Global state for the communication system.

use parking_lot::RwLock;
use std::sync::LazyLock;

use super::animation::AnimContext;
use super::oscilloscope::Oscilloscope;
use super::response::ResponseSystem;
use super::subtitle::SubtitleTracker;
use super::track::TrackManager;
use super::types::{CommData, CommError, CommIntroMode, CommResult};

/// Global communication state
pub static COMM_STATE: LazyLock<RwLock<CommState>> =
    LazyLock::new(|| RwLock::new(CommState::new()));

/// Communication state
#[derive(Debug)]
pub struct CommState {
    /// Whether communication is initialized
    initialized: bool,

    /// Current alien data
    comm_data: Option<CommData>,

    /// Track manager for speech
    track: TrackManager,

    /// Subtitle tracker
    subtitles: SubtitleTracker,

    /// Response system
    responses: ResponseSystem,

    /// Animation context
    animations: AnimContext,

    /// Oscilloscope display
    oscilloscope: Oscilloscope,

    /// Whether alien is currently talking
    talking: bool,

    /// Whether talking has finished
    talking_finished: bool,

    /// Intro mode
    intro_mode: CommIntroMode,

    /// Fade time in ticks
    fade_time: u32,

    /// Whether input is paused
    input_paused: bool,

    /// Last input time (for timeout)
    last_input_time: u64,
}

impl Default for CommState {
    fn default() -> Self {
        Self::new()
    }
}

impl CommState {
    /// Create a new communication state
    pub fn new() -> Self {
        Self {
            initialized: false,
            comm_data: None,
            track: TrackManager::new(),
            subtitles: SubtitleTracker::new(),
            responses: ResponseSystem::new(),
            animations: AnimContext::new(),
            oscilloscope: Oscilloscope::new(),
            talking: false,
            talking_finished: false,
            intro_mode: CommIntroMode::Default,
            fade_time: 0,
            input_paused: false,
            last_input_time: 0,
        }
    }

    /// Initialize communication
    pub fn init(&mut self) -> CommResult<()> {
        if self.initialized {
            return Err(CommError::AlreadyInitialized);
        }

        self.initialized = true;
        self.oscilloscope.activate();
        Ok(())
    }

    /// Uninitialize communication
    pub fn uninit(&mut self) {
        self.clear();
        self.initialized = false;
        self.oscilloscope.deactivate();
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Clear all state (but stay initialized)
    pub fn clear(&mut self) {
        self.comm_data = None;
        self.track.clear();
        self.subtitles.clear();
        self.responses.clear();
        self.animations.clear();
        self.oscilloscope.clear();
        self.talking = false;
        self.talking_finished = false;
        self.intro_mode = CommIntroMode::Default;
        self.fade_time = 0;
        self.input_paused = false;
    }

    /// Set communication data for current encounter
    pub fn set_comm_data(&mut self, data: CommData) {
        self.comm_data = Some(data);
    }

    /// Get communication data
    pub fn comm_data(&self) -> Option<&CommData> {
        self.comm_data.as_ref()
    }

    /// Get mutable communication data
    pub fn comm_data_mut(&mut self) -> Option<&mut CommData> {
        self.comm_data.as_mut()
    }

    // Track management

    /// Get the track manager
    pub fn track(&self) -> &TrackManager {
        &self.track
    }

    /// Get mutable track manager
    pub fn track_mut(&mut self) -> &mut TrackManager {
        &mut self.track
    }

    /// Start playing the track
    pub fn start_track(&mut self) -> CommResult<()> {
        if !self.initialized {
            return Err(CommError::NotInitialized);
        }
        self.track.start();
        self.talking = true;
        self.talking_finished = false;
        Ok(())
    }

    /// Stop the track
    pub fn stop_track(&mut self) {
        self.track.stop();
        self.talking = false;
    }

    /// Wait for track to finish
    pub fn wait_track(&self) -> bool {
        self.track.wait()
    }

    // Subtitle management

    /// Get subtitle tracker
    pub fn subtitles(&self) -> &SubtitleTracker {
        &self.subtitles
    }

    /// Get mutable subtitle tracker
    pub fn subtitles_mut(&mut self) -> &mut SubtitleTracker {
        &mut self.subtitles
    }

    /// Get current subtitle
    pub fn current_subtitle(&self) -> Option<&str> {
        // Prefer track's subtitle, fall back to subtitle tracker
        self.track
            .current_subtitle()
            .or_else(|| self.subtitles.current_subtitle())
    }

    // Response management

    /// Get response system
    pub fn responses(&self) -> &ResponseSystem {
        &self.responses
    }

    /// Get mutable response system
    pub fn responses_mut(&mut self) -> &mut ResponseSystem {
        &mut self.responses
    }

    /// Add a response option
    pub fn add_response(&mut self, response_ref: u32, text: &str, func: Option<usize>) -> bool {
        self.responses.do_response_phrase(response_ref, text, func)
    }

    /// Display responses
    pub fn display_responses(&mut self) {
        self.responses.start_display();
    }

    /// Clear responses
    pub fn clear_responses(&mut self) {
        self.responses.clear();
    }

    /// Select next response
    pub fn select_next_response(&mut self) -> bool {
        self.responses.select_next()
    }

    /// Select previous response
    pub fn select_prev_response(&mut self) -> bool {
        self.responses.select_prev()
    }

    /// Get selected response index
    pub fn selected_response(&self) -> i32 {
        self.responses.selected()
    }

    // Animation management

    /// Get animation context
    pub fn animations(&self) -> &AnimContext {
        &self.animations
    }

    /// Get mutable animation context
    pub fn animations_mut(&mut self) -> &mut AnimContext {
        &mut self.animations
    }

    // Oscilloscope

    /// Get oscilloscope
    pub fn oscilloscope(&self) -> &Oscilloscope {
        &self.oscilloscope
    }

    /// Get mutable oscilloscope
    pub fn oscilloscope_mut(&mut self) -> &mut Oscilloscope {
        &mut self.oscilloscope
    }

    /// Add samples to oscilloscope
    pub fn add_oscilloscope_samples(&mut self, samples: &[i16]) {
        self.oscilloscope.add_samples(samples);
    }

    // State queries

    /// Check if alien is talking
    pub fn is_talking(&self) -> bool {
        self.talking
    }

    /// Check if talking has finished
    pub fn is_talking_finished(&self) -> bool {
        self.talking_finished
    }

    /// Set talking finished
    pub fn set_talking_finished(&mut self, finished: bool) {
        self.talking_finished = finished;
        if finished {
            self.talking = false;
        }
    }

    /// Get intro mode
    pub fn intro_mode(&self) -> CommIntroMode {
        self.intro_mode
    }

    /// Set intro mode
    pub fn set_intro_mode(&mut self, mode: CommIntroMode) {
        self.intro_mode = mode;
    }

    /// Get fade time
    pub fn fade_time(&self) -> u32 {
        self.fade_time
    }

    /// Set fade time
    pub fn set_fade_time(&mut self, time: u32) {
        self.fade_time = time;
    }

    /// Check if input is paused
    pub fn is_input_paused(&self) -> bool {
        self.input_paused
    }

    /// Set input paused
    pub fn set_input_paused(&mut self, paused: bool) {
        self.input_paused = paused;
    }

    /// Update communication state (call each frame)
    pub fn update(&mut self, delta_time: f32) {
        if !self.initialized {
            return;
        }

        // Update track
        self.track.update(delta_time);

        // Update subtitles
        self.subtitles.update(self.track.position());

        // Update animations
        let delta_duration = std::time::Duration::from_secs_f32(delta_time);
        self.animations.update(delta_duration);

        // Update oscilloscope
        self.oscilloscope.update();

        // Check if track finished
        if self.track.is_finished() && self.talking {
            self.talking = false;
            self.talking_finished = true;
        }
    }
}

// Global access functions

/// Initialize the global communication system
pub fn init_communication() -> CommResult<()> {
    COMM_STATE.write().init()
}

/// Uninitialize the global communication system
pub fn uninit_communication() {
    COMM_STATE.write().uninit();
}

/// Check if communication is initialized
pub fn is_initialized() -> bool {
    COMM_STATE.read().is_initialized()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // Helper to reset state between tests
    fn reset_state() {
        let mut state = COMM_STATE.write();
        state.uninit();
    }

    #[test]
    #[serial]
    fn test_comm_state_new() {
        let state = CommState::new();
        assert!(!state.is_initialized());
        assert!(!state.is_talking());
    }

    #[test]
    #[serial]
    fn test_comm_init_uninit() {
        let mut state = CommState::new();

        assert!(state.init().is_ok());
        assert!(state.is_initialized());

        // Double init should fail
        assert!(matches!(state.init(), Err(CommError::AlreadyInitialized)));

        state.uninit();
        assert!(!state.is_initialized());
    }

    #[test]
    #[serial]
    fn test_global_init_uninit() {
        reset_state();

        assert!(init_communication().is_ok());
        assert!(is_initialized());

        uninit_communication();
        assert!(!is_initialized());
    }

    #[test]
    #[serial]
    fn test_comm_data() {
        let mut state = CommState::new();

        let data = CommData::new();
        state.set_comm_data(data);

        assert!(state.comm_data().is_some());
    }

    #[test]
    #[serial]
    fn test_track_management() {
        let mut state = CommState::new();
        state.init().unwrap();

        state
            .track_mut()
            .splice_track(1, Some("Hello"), 0.0, 2.0);

        assert!(state.start_track().is_ok());
        assert!(state.is_talking());

        state.stop_track();
        assert!(!state.is_talking());
    }

    #[test]
    #[serial]
    fn test_track_not_initialized() {
        let mut state = CommState::new();

        assert!(matches!(
            state.start_track(),
            Err(CommError::NotInitialized)
        ));
    }

    #[test]
    #[serial]
    fn test_response_management() {
        let mut state = CommState::new();

        assert!(state.add_response(1, "Option A", None));
        assert!(state.add_response(2, "Option B", None));

        assert_eq!(state.responses().count(), 2);

        state.display_responses();
        assert!(state.responses().is_displaying());
        assert_eq!(state.selected_response(), 0);

        assert!(state.select_next_response());
        assert_eq!(state.selected_response(), 1);

        state.clear_responses();
        assert!(state.responses().is_empty());
    }

    #[test]
    #[serial]
    fn test_subtitle_tracking() {
        let mut state = CommState::new();
        state.init().unwrap();

        state.subtitles_mut().add_subtitle(0.0, "First");
        state.subtitles_mut().add_subtitle(2.0, "Second");

        state.subtitles_mut().update(0.5);
        assert_eq!(state.current_subtitle(), Some("First"));

        state.subtitles_mut().update(2.5);
        assert_eq!(state.current_subtitle(), Some("Second"));
    }

    #[test]
    #[serial]
    fn test_oscilloscope() {
        let mut state = CommState::new();
        state.init().unwrap();

        state.add_oscilloscope_samples(&[100, 200, 300]);

        assert!(state.oscilloscope().is_active());
        assert!(state.oscilloscope().peak() > 1);
    }

    #[test]
    #[serial]
    fn test_talking_state() {
        let mut state = CommState::new();

        assert!(!state.is_talking_finished());

        state.set_talking_finished(true);
        assert!(state.is_talking_finished());
        assert!(!state.is_talking());
    }

    #[test]
    #[serial]
    fn test_intro_mode() {
        let mut state = CommState::new();

        assert_eq!(state.intro_mode(), CommIntroMode::Default);

        state.set_intro_mode(CommIntroMode::FadeIn);
        assert_eq!(state.intro_mode(), CommIntroMode::FadeIn);
    }

    #[test]
    #[serial]
    fn test_fade_time() {
        let mut state = CommState::new();

        state.set_fade_time(30);
        assert_eq!(state.fade_time(), 30);
    }

    #[test]
    #[serial]
    fn test_input_paused() {
        let mut state = CommState::new();

        assert!(!state.is_input_paused());

        state.set_input_paused(true);
        assert!(state.is_input_paused());
    }

    #[test]
    #[serial]
    fn test_update() {
        let mut state = CommState::new();
        state.init().unwrap();

        state
            .track_mut()
            .splice_track(1, Some("Test"), 0.0, 1.0);
        state.start_track().unwrap();

        state.update(0.5);
        assert!(state.is_talking());
        assert_eq!(state.track().position(), 0.5);

        state.update(1.0);
        assert!(state.is_talking_finished());
    }

    #[test]
    #[serial]
    fn test_clear() {
        let mut state = CommState::new();
        state.init().unwrap();

        state.set_comm_data(CommData::new());
        state.add_response(1, "Test", None);
        state.set_fade_time(30);

        state.clear();

        assert!(state.comm_data().is_none());
        assert!(state.responses().is_empty());
        assert_eq!(state.fade_time(), 0);
        // Should still be initialized
        assert!(state.is_initialized());
    }
}
