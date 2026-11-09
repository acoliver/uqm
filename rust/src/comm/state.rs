//! Communication state management
//!
//! State retained by one synchronous conversation.

use super::animation::AnimContext;
use super::encounter::EncounterState;
use super::oscilloscope::Oscilloscope;
use super::phrase_state::PhraseState;
use super::response::ResponseSystem;
use super::response_ui::ResponseUI;
use super::segue::Segue;
use super::speech_graphics::SpeechGraphics;
use super::subtitle::SubtitleTracker;
use super::subtitle_display::SubtitleDisplay;
use super::summary::ConversationSummary;
use super::track::TrackManager;
use super::types::{CommData, CommError, CommIntroMode, CommResult};

type PhraseEvents = std::sync::Arc<parking_lot::Mutex<Vec<fn(&mut Comm)>>>;

/// Communication state
pub struct Comm {
    pub(crate) presentation: super::presentation::Presentation,
    phrase_events: PhraseEvents,
    pub(crate) race_scratch: super::races::RuntimeScratch,
    pub(crate) data: super::locdata::CLocData,
    pub options: super::CommOptions,
    pub(crate) scope: super::oscill_abi::runtime::RuntimeState,
    pub(crate) display: super::comm_abi::RuntimeState,
    pub(crate) animation_runtime: super::commanim_abi::runtime::RuntimeState,
    pub(crate) response_display: [u8; 3],
    pub(crate) subtitle_buf: Vec<u8>,
    pub(crate) player_font: super::player_font::ConversationFont,
    pub(crate) next_speech_time: u32,
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

    /// Phrase enable/disable state (encounter-local)
    phrase_state: PhraseState,

    /// Segue state (controls encounter exit behavior)
    segue: Segue,

    /// Conversation summary — derived cache over trackplayer subtitle history.
    /// @plan PLAN-20260314-COMM.P06
    summary: ConversationSummary,

    /// Encounter lifecycle state (callback tracking, resource handles).
    /// @plan PLAN-20260314-COMM.P08
    pub encounter: EncounterState,

    /// Whether alien_talk_segue has made its first call this encounter.
    /// Tracks the `pCurInputState->Initialized` flag from C AlienTalkSegue.
    /// @plan PLAN-20260314-COMM.P09
    pub first_talk_call: bool,

    /// Index of the topmost visible response in the scrolled response list.
    /// Matches C ENCOUNTER_STATE.top_response (~0 = not yet initialized).
    /// @plan PLAN-20260314-COMM.P09
    pub top_response: Option<u8>,

    /// Speech graphics state (oscilloscope + slider).
    /// @plan PLAN-20260314-COMM.P10
    speech_graphics: SpeechGraphics,

    /// Response list UI rendering state.
    /// @plan PLAN-20260314-COMM.P10
    response_ui: ResponseUI,

    /// Subtitle display state with change detection.
    /// @plan PLAN-20260314-COMM.P10
    subtitle_display: SubtitleDisplay,

    /// Previous subtitle text for change detection in the talk loop.
    /// @plan PLAN-20260314-COMM.P10
    prev_subtitle: Option<String>,
}

impl Default for Comm {
    fn default() -> Self {
        Self::new()
    }
}

impl Comm {
    /// Create a new communication state
    pub fn new() -> Self {
        Self {
            presentation: Default::default(),
            phrase_events: Default::default(),
            race_scratch: Default::default(),
            data: Default::default(),
            options: Default::default(),
            scope: Default::default(),
            display: Default::default(),
            animation_runtime: Default::default(),
            response_display: [0; 3],
            subtitle_buf: Vec::with_capacity(512),
            player_font: super::player_font::ConversationFont::new(),
            next_speech_time: 0,
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
            phrase_state: PhraseState::new(),
            segue: Segue::default(),
            summary: ConversationSummary::new(10),
            encounter: EncounterState::new(),
            first_talk_call: false,
            top_response: None,
            speech_graphics: SpeechGraphics::new(),
            response_ui: ResponseUI::new(),
            subtitle_display: SubtitleDisplay::new(),
            prev_subtitle: None,
        }
    }

    pub(crate) fn queue_phrase_callback(
        &self,
        callback: super::glue::PhraseCallback,
    ) -> Option<Box<dyn Fn(i32) + Send>> {
        callback.map(|callback| {
            let events = self.phrase_events.clone();
            Box::new(move |_| events.lock().push(callback)) as Box<dyn Fn(i32) + Send>
        })
    }

    pub(crate) fn apply_phrase_events(&mut self) {
        let events = std::mem::take(&mut *self.phrase_events.lock());
        for event in events {
            event(self);
        }
    }

    pub fn with_options(options: super::CommOptions) -> Self {
        let mut comm = Self::new();
        if let Some(window) = &options.window {
            comm.display.window = window.get();
        }
        comm.set_intro_mode(CommIntroMode::from(options.intro.0));
        comm.set_fade_time(options.intro.1);
        comm.options = options;
        comm.initialized = true;
        comm.oscilloscope.activate();
        comm
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

    /// Reset for a new encounter's hail, keeping the intro transition that
    /// the caller set before `InitCommunication`. In the C game
    /// `SetCommIntroMode` precedes the hail and only `CommIntroTransition`
    /// resets the mode (comm.c:730-767, 1357-1362).
    pub fn reset_for_encounter(&mut self) {
        let (intro_mode, fade_time) = (self.intro_mode, self.fade_time);
        self.clear();
        self.intro_mode = intro_mode;
        self.fade_time = fade_time;
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
        self.phrase_state.reset();
        self.segue = Segue::default();
        self.summary.clear();
        self.encounter.reset();
        self.first_talk_call = false;
        self.top_response = None;
        self.speech_graphics.clear();
        self.response_ui.clear();
        self.subtitle_display.clear();
        self.prev_subtitle = None;
    }

    /// Install the descriptor and its typed conversation view together.
    ///
    /// # Safety
    /// Descriptor resources must remain live through session teardown.
    pub unsafe fn install_descriptor(&mut self, data: super::locdata::CLocData) {
        self.set_comm_data(super::locdata::read_locdata(&data));
        self.data = data;
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

    /// Clear communication data (encounter teardown — resources already freed)
    pub fn clear_comm_data(&mut self) {
        self.comm_data = None;
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
    pub fn add_response(
        &mut self,
        response_ref: u32,
        text: &str,
        func: Option<super::response::ResponseFunc>,
    ) -> bool {
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

    // Phrase state management

    /// Check if a phrase is enabled (not disabled this encounter).
    pub fn phrase_enabled(&self, index: i32) -> bool {
        self.phrase_state.is_enabled(index)
    }

    /// Disable a phrase for the remainder of this encounter.
    pub fn disable_phrase(&mut self, index: i32) {
        self.phrase_state.disable(index);
    }

    /// Get phrase state (for direct access in glue functions).
    pub fn phrase_state(&self) -> &PhraseState {
        &self.phrase_state
    }

    // Segue management

    /// Get current segue.
    pub fn get_segue(&self) -> Segue {
        self.segue
    }

    /// Set segue for encounter exit.
    pub fn set_segue(&mut self, segue: Segue) {
        self.segue = segue;
    }

    /// Get conversation summary (read-only).
    pub fn summary(&self) -> &ConversationSummary {
        &self.summary
    }

    /// Get conversation summary (mutable).
    pub fn summary_mut(&mut self) -> &mut ConversationSummary {
        &mut self.summary
    }

    // Speech graphics

    /// Get speech graphics state.
    pub fn speech_graphics(&self) -> &SpeechGraphics {
        &self.speech_graphics
    }

    /// Get mutable speech graphics state.
    pub fn speech_graphics_mut(&mut self) -> &mut SpeechGraphics {
        &mut self.speech_graphics
    }

    // Response UI

    /// Get response UI state.
    pub fn response_ui(&self) -> &ResponseUI {
        &self.response_ui
    }

    /// Get mutable response UI state.
    pub fn response_ui_mut(&mut self) -> &mut ResponseUI {
        &mut self.response_ui
    }

    pub fn refresh_response_ui(&mut self, activity: u16) {
        let selected = self.responses.selected().max(0) as usize;
        self.response_ui
            .refresh_responses(activity, &self.responses, selected);
        #[cfg(not(test))]
        super::response_ui::refresh_responses_production(
            self,
            activity,
            self.response_ui.top_response() as u8,
            self.responses.count() as u8,
            selected as u8,
        );
    }

    // Subtitle display

    /// Get subtitle display state.
    pub fn subtitle_display(&self) -> &SubtitleDisplay {
        &self.subtitle_display
    }

    /// Get mutable subtitle display state.
    pub fn subtitle_display_mut(&mut self) -> &mut SubtitleDisplay {
        &mut self.subtitle_display
    }

    /// Get previous subtitle text (for change detection).
    pub fn prev_subtitle(&self) -> Option<&str> {
        self.prev_subtitle.as_deref()
    }

    /// Set previous subtitle text.
    pub fn set_prev_subtitle(&mut self, text: Option<String>) {
        self.prev_subtitle = text;
    }

    /// Rebuild the summary from current track subtitle history.
    /// In test mode, reads from TrackManager chunks.
    pub fn rebuild_summary(&mut self) {
        let entries: Vec<String> = self
            .track
            .chunks()
            .iter()
            .filter_map(|c| c.subtitle.clone())
            .collect();
        self.summary.rebuild(entries);
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

        // Update animations (convert seconds to ticks, ~60fps = 1 tick per frame)
        let delta_ticks = (delta_time * 60.0) as u32;
        self.animations.process(delta_ticks);

        // Update oscilloscope
        self.oscilloscope.update();

        // Check if track finished
        if self.track.is_finished() && self.talking {
            self.talking = false;
            self.talking_finished = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn conversation_state_is_independent_and_persists_between_frames() {
        let mut first = Comm::new();
        let mut second = Comm::new();
        first.init().unwrap();
        second.init().unwrap();
        first.add_response(17, "First conversation", None);
        first.disable_phrase(9);
        first.top_response = Some(0);
        first.set_intro_mode(CommIntroMode::from(3));
        first.set_fade_time(1680);
        second.add_response(23, "Second conversation", None);
        second.clear_responses();
        assert_eq!(first.responses().count(), 1);
        assert_eq!(first.responses().get(0).unwrap().response_ref, 17);
        assert!(!first.phrase_enabled(9));
        assert!(second.phrase_enabled(9));
        assert_eq!(first.top_response, Some(0));
        assert_eq!(first.fade_time(), 1680);
        assert_eq!(second.fade_time(), 0);
        first.response_display = [7, 8, 9];
        first.next_speech_time = 1680;
        first.race_scratch.slyland_choices[0] = 17;
        first.race_scratch.last_alien = 5;
        assert_eq!(second.response_display, [0; 3]);
        assert_eq!(second.next_speech_time, 0);
        assert_eq!(second.race_scratch.slyland_choices, [0; 5]);
        assert_eq!(second.race_scratch.last_alien, 0);
        assert!(second.race_scratch.druuge.is_none());
        assert!(second.race_scratch.mycon.is_none());
        assert!(second.race_scratch.melnorm.is_none());
        assert!(second.race_scratch.spathi.is_none());
        assert!(second.race_scratch.starbas.is_none());
        assert_eq!(
            first.player_font.install(std::ptr::null_mut()),
            Err(super::super::player_font::ConversationFontError::LoadFailed)
        );
        first.reset_for_encounter();
        assert_eq!(first.responses().count(), 0);
        assert!(first.phrase_enabled(9));
        assert_eq!(first.intro_mode(), CommIntroMode::from(3));
        assert_eq!(first.fade_time(), 1680);
        assert!(second.is_initialized());
    }

    #[test]
    #[serial]
    fn test_comm_state_new() {
        let state = Comm::new();
        assert!(!state.is_initialized());
        assert!(!state.is_talking());
    }

    #[test]
    #[serial]
    fn test_comm_init_uninit() {
        let mut state = Comm::new();

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
        let mut comm = Comm::new();

        assert!(comm.init().is_ok());
        assert!(comm.is_initialized());

        comm.uninit();
        assert!(!comm.is_initialized());
    }

    #[test]
    #[serial]
    fn test_comm_data() {
        let mut state = Comm::new();

        let data = CommData::new();
        state.set_comm_data(data);

        assert!(state.comm_data().is_some());
    }

    #[test]
    #[serial]
    fn test_track_management() {
        let mut state = Comm::new();
        state.init().unwrap();

        state.track_mut().splice_track(1, Some("Hello"), 0.0, 2.0);

        assert!(state.start_track().is_ok());
        assert!(state.is_talking());

        state.stop_track();
        assert!(!state.is_talking());
    }

    #[test]
    #[serial]
    fn test_track_not_initialized() {
        let mut state = Comm::new();

        assert!(matches!(
            state.start_track(),
            Err(CommError::NotInitialized)
        ));
    }

    #[test]
    #[serial]
    fn test_response_management() {
        let mut state = Comm::new();

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
        let mut state = Comm::new();
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
        let mut state = Comm::new();
        state.init().unwrap();

        state.add_oscilloscope_samples(&[100, 200, 300]);

        assert!(state.oscilloscope().is_active());
        assert!(state.oscilloscope().peak() > 1);
    }

    #[test]
    #[serial]
    fn test_talking_state() {
        let mut state = Comm::new();

        assert!(!state.is_talking_finished());

        state.set_talking_finished(true);
        assert!(state.is_talking_finished());
        assert!(!state.is_talking());
    }

    #[test]
    #[serial]
    fn test_intro_mode() {
        let mut state = Comm::new();

        assert_eq!(state.intro_mode(), CommIntroMode::Default);

        state.set_intro_mode(CommIntroMode::FadeIn);
        assert_eq!(state.intro_mode(), CommIntroMode::FadeIn);
    }

    #[test]
    #[serial]
    fn test_fade_time() {
        let mut state = Comm::new();

        state.set_fade_time(30);
        assert_eq!(state.fade_time(), 30);
    }

    #[test]
    #[serial]
    fn test_input_paused() {
        let mut state = Comm::new();

        assert!(!state.is_input_paused());

        state.set_input_paused(true);
        assert!(state.is_input_paused());
    }

    #[test]
    #[serial]
    fn test_update() {
        let mut state = Comm::new();
        state.init().unwrap();

        state.track_mut().splice_track(1, Some("Test"), 0.0, 1.0);
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
        let mut state = Comm::new();
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

    #[test]
    #[serial]
    fn reset_for_encounter_keeps_the_intro_transition_set_before_the_hail() {
        // `SetCommIntroMode (CIM_FADE_IN_SCREEN, ...)` runs before
        // `InitCommunication`, and only `CommIntroTransition` consumes it
        // (comm.c:730-767), so the hail's own state reset must not drop it.
        let mut state = Comm::new();
        state.init().unwrap();
        state.set_comm_data(CommData::new());
        state.add_response(1, "Test", None);
        state.set_intro_mode(CommIntroMode::from(3));
        state.set_fade_time(1680);

        state.reset_for_encounter();

        assert!(state.comm_data().is_none());
        assert!(state.responses().is_empty());
        assert_eq!(state.intro_mode(), CommIntroMode::from(3));
        assert_eq!(state.fade_time(), 1680);
        assert!(state.is_initialized());
    }
}

#[cfg(test)]
mod session_owner_tests {
    use super::*;

    #[test]
    fn playback_callbacks_follow_the_owner_after_a_move_and_repeat_on_seek() {
        fn speaker(comm: &mut Comm) {
            comm.data.alien_text_baseline.x += 1;
        }
        let first = Comm::new();
        let second = Comm::new();
        assert!(first.queue_phrase_callback(None).is_none());
        let callback = first.queue_phrase_callback(Some(speaker)).unwrap();
        callback(0);
        callback(17);
        assert_eq!(first.data.alien_text_baseline.x, 0);
        let mut moved = Box::new(first);
        moved.apply_phrase_events();
        assert_eq!(moved.data.alien_text_baseline.x, 2);
        assert_eq!(second.data.alien_text_baseline.x, 0);
        moved.apply_phrase_events();
        assert_eq!(moved.data.alien_text_baseline.x, 2);
        callback(0);
        moved.apply_phrase_events();
        assert_eq!(moved.data.alien_text_baseline.x, 3);
        drop(moved);
        callback(17);
        assert_eq!(second.data.alien_text_baseline.x, 0);
    }

    #[test]
    #[serial_test::serial]
    fn credits_options_preserve_the_window_and_skip_both_native_widgets() {
        let window = super::super::locdata::CRect {
            corner: super::super::locdata::CPoint { x: 31, y: 47 },
            width: 212,
            height: 109,
        };
        let mut comm = Comm::with_options(super::super::CommOptions {
            oscilloscope: false,
            slider: false,
            window: Some(std::rc::Rc::new(std::cell::Cell::new(window))),
            intro: (2, 840),
        });
        assert!(comm.is_initialized());
        // SAFETY: disabled widgets return before using any native surface or resource handle.
        unsafe {
            super::super::oscill_abi::runtime::draw_oscilloscope(&mut comm);
            super::super::oscill_abi::runtime::draw_slider(&mut comm);
        }
        assert!(!comm.scope.scope_init);
        assert!(comm.scope.scope_work.is_null());
        assert_eq!(comm.scope.last_offset, -1);
        assert_eq!(
            (comm.display.window.corner.x, comm.display.window.corner.y),
            (31, 47)
        );
        assert_eq!(
            (comm.display.window.width, comm.display.window.height),
            (212, 109)
        );
        assert_eq!(comm.intro_mode() as u32, 2);
        assert_eq!(comm.fade_time(), 840);
    }
}
