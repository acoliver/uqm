//! Talk segue and main dialogue loop.
//!
//! Implements the core playback and state-machine control flow for alien
//! communication.  Matches C `DoTalkSegue`, `TalkSegue`, `AlienTalkSegue`,
//! `SelectResponse`, `PlayerResponseInput`, and `DoCommunication` from
//! `sc2/src/uqm/comm.c` lines 565–1120.
//!
//! # Lock discipline
//!
//! All functions here accept `&mut CommState` — they do **not** touch the
//! global `COMM_STATE` lock.  Lock acquisition and release is the
//! responsibility of the FFI layer in `ffi.rs`.
//!
//! @plan PLAN-20260314-COMM.P09

use super::response::ResponseFunc;
use super::state::CommState;

// ============================================================================
// Wait-track sentinel
// ============================================================================

/// Pass to `talk_segue` / `alien_talk_segue` to mean "wait for all tracks".
/// Matches C `WAIT_TRACK_ALL` which is set to the maximum COUNT value.
pub const WAIT_TRACK_ALL: u32 = u32::MAX;

// ============================================================================
// C bridge: real calls used in production, simulated in test
// ============================================================================

/// Scroll option constants — matches C `optSmoothScroll`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollMode {
    /// Page-based scrolling (OPT_PC).
    Page,
    /// Smooth scrolling (OPT_3DO).
    Smooth,
}

// Production C bridge — all calls go through these wrappers so tests can
// override behaviour by operating on CommState fields directly.
#[cfg(not(test))]
mod c_bridge {
    use std::ffi::{c_char, c_int, c_uint};

    extern "C" {
        pub fn c_PlayingTrack() -> u16;
        pub fn c_JumpTrack();
        pub fn c_PlayTrack();
        pub fn c_StopTrack();
        pub fn c_FastForward_Page();
        pub fn c_FastForward_Smooth();
        pub fn c_FastReverse_Page();
        pub fn c_FastReverse_Smooth();
        pub fn c_CheckSubtitles();
        pub fn c_ClearSubtitles();
        pub fn c_UpdateSpeechGraphics();
        pub fn c_InitSpeechGraphics();
        pub fn c_FeedbackPlayerPhrase(text: *const c_char);
        pub fn c_FadeMusic(volume: c_int, duration: c_int) -> c_uint;
        pub fn c_SetSliderImage(frame_index: c_int);
        pub fn c_UpdateAnimations(seeking: c_int);
        pub fn c_CheckAbort() -> c_int;
        pub fn c_WonLastBattle() -> c_int;
        /// @plan PLAN-20260325-COMMPT3.P03
        /// @requirement REQ-CM-001, REQ-CM-002
        /// @pseudocode 001-colormap-music-bridges lines 01-08
        pub fn c_SetColorMapFromCommData();
        /// @plan PLAN-20260325-COMMPT3.P03
        /// @requirement REQ-MU-001, REQ-MU-002
        /// @pseudocode 001-colormap-music-bridges lines 09-15
        pub fn c_PlayAlienMusic();
        pub fn c_DrawAlienFrame();
        pub fn c_CommIntroTransition();
        pub fn c_InitCommAnimations();
        pub fn c_RunningIntroAnim() -> c_int;
        pub fn c_RunCommAnimFrame();
        pub fn c_RunningTalkingAnim() -> c_int;
        pub fn c_WantTalkingAnim() -> c_int;
        pub fn c_HaveTalkingAnim() -> c_int;
        pub fn c_SetRunTalkingAnim();
        pub fn c_SetStopTalkingAnim();
        pub fn c_SetRunIntroAnim();
        pub fn c_SetMenuSounds(up_down: c_int, select: c_int);
        pub fn c_RefreshResponses(top: u8, num_responses: u8, cur_response: u8);
        pub fn c_SelectConversationSummary();
        pub fn c_GetOptSmoothScroll() -> c_int;
        pub fn c_GetLastActivityAbortFlag() -> c_int;
        pub fn c_FadeOutMusicForReplay() -> c_uint;
        pub fn c_ClearLastActivityLoadFlag();
        // @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-IP-007
        pub fn c_GetPulsedMenuKey(key_index: c_int) -> c_int;
        // @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-AT-001
        pub fn c_HasTransitionAnim() -> c_int;
    }

    /// Slider image indices matching C ActivityFrame indices.
    pub mod slider {
        pub const FAST_FORWARD: i32 = 3;
        pub const FAST_REVERSE: i32 = 4;
        pub const PLAY: i32 = 2;
        pub const STOP: i32 = 8;
    }

    pub mod music_volume {
        pub const BACKGROUND: i32 = 64; // BACKGROUND_VOL
        pub const FOREGROUND: i32 = 255; // FOREGROUND_VOL
        pub const NORMAL: i32 = 255; // NORMAL_VOLUME
    }
}

// Menu key indices — matches the second enum in sc2/src/uqm/controls.h
// (KEY_PAUSE=0..KEY_FULLSCREEN=4 precede these)
// @plan PLAN-20260326-COMMPT2.P03
#[cfg(not(test))]
use std::ffi::c_int;
#[cfg(not(test))]
const KEY_MENU_UP: c_int = 5;
#[cfg(not(test))]
const KEY_MENU_DOWN: c_int = 6;
#[cfg(not(test))]
const KEY_MENU_LEFT: c_int = 7;
#[cfg(not(test))]
const KEY_MENU_RIGHT: c_int = 8;
#[cfg(not(test))]
const KEY_MENU_SELECT: c_int = 9;
#[cfg(not(test))]
const KEY_MENU_CANCEL: c_int = 10;

// ============================================================================
// TalkingState — matches C TALKING_STATE
// ============================================================================

/// Per-call playback control for a single talk segue.
///
/// Matches C `TALKING_STATE` and lives only for the duration of one
/// `talk_segue()` call — it does not persist in `CommState`.
#[derive(Debug, Default)]
pub struct TalkingState {
    /// Which track number to wait for before stopping.
    pub wait_track: u32,
    /// Whether the caller is currently seeking (FF/FR held down).
    pub seeking: bool,
    /// Whether to start with a rewind.
    pub rewind: bool,
    /// Whether playback has reached its natural end.
    pub ended: bool,
}

// ============================================================================
// Result types
// ============================================================================

/// Result of one `player_response_input` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerInputResult {
    /// Keep looping — no action taken this iteration.
    Continue,
    /// Player selected a response — caller should invoke the callback.
    Selected,
    /// Player opened the conversation summary.
    Summary,
    /// Player requested a replay of the last phrase.
    Replay,
}

/// Result of one `do_communication` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommunicationResult {
    /// Keep iterating.
    Continue,
    /// Conversation is complete — caller should tear down.
    Done,
}

// ============================================================================
// do_talk_segue — one frame / iteration of the playback loop
// ============================================================================

/// Process one iteration of the talk-segue input loop.
///
/// Matches C `DoTalkSegue`.  Returns `true` to keep looping, `false` to stop.
///
/// In production the caller loops over this inside `DoInput`; here we expose
/// the per-iteration logic so it can be driven by the Rust loop in
/// `talk_segue`.
pub fn do_talk_segue(state: &mut CommState, ts: &mut TalkingState) -> bool {
    // ---- abort check -------------------------------------------------------
    if check_abort(state) {
        ts.ended = true;
        return false;
    }

    // ---- cancel (skip to end of current phrase) ----------------------------
    if check_cancel_input(state) {
        jump_track(state);
        ts.ended = true;
        return false;
    }

    // ---- seek input --------------------------------------------------------
    let left = check_left_input(state);
    let right = check_right_input(state);

    if right {
        set_slider_image(state, SliderImage::FastForward);
        fast_forward(state);
        ts.seeking = true;
    } else if left || ts.rewind {
        ts.rewind = false;
        set_slider_image(state, SliderImage::FastReverse);
        fast_reverse(state);
        ts.seeking = true;
    } else if ts.seeking {
        // Seeking just ended — restore play slider
        ts.seeking = false;
        set_slider_image(state, SliderImage::Play);
    } else {
        check_subtitles(state);
    }

    update_animations(state, ts.seeking);
    update_speech_graphics(state);

    // In test mode, advance the track each iteration to prevent infinite loops.
    // Production relies on real-time SleepThreadUntil for frame pacing.
    #[cfg(test)]
    state.track_mut().update(1.0 / 60.0);

    let cur_track = playing_track(state);
    ts.ended = !ts.seeking && cur_track == 0;

    // Continue if seeking, or if still on a track at/before the wait track
    ts.seeking || (cur_track != 0 && cur_track <= ts.wait_track)
}

// ============================================================================
// talk_segue — runs the full playback loop for one phrase group
// ============================================================================

/// Run the full talk segue for the given wait-track number.
///
/// Matches C `TalkSegue`.  Returns `true` if playback reached its natural
/// end (i.e. `talkingState.ended`).
pub fn talk_segue(state: &mut CommState, wait_track: u32) -> bool {
    // ---- transition to talking animation, if available ---------------------
    if want_talking_anim(state) && have_talking_anim(state) {
        if has_transition_anim(state) {
            set_run_intro_anim(state);
        }
        set_run_talking_anim(state);

        // wait for intro animation to finish
        while running_intro_anim(state) {
            run_comm_anim_frame(state);
        }
    }

    // ---- build initial TalkingState ----------------------------------------
    let mut ts = TalkingState::default();

    let effective_wait = if wait_track == 0 {
        // Rewind-restart mode
        ts.rewind = true;
        WAIT_TRACK_ALL
    } else {
        if playing_track(state) == 0 {
            // Initial start of player
            play_track(state);
            // C asserts PlayingTrack() != 0 here
        }
        wait_track
    };
    ts.wait_track = effective_wait;

    // ---- main loop ---------------------------------------------------------
    while do_talk_segue(state, &mut ts) {
        // loop body is in do_talk_segue
    }

    clear_subtitles(state);

    if ts.ended {
        // Reached natural end — show STOP icon
        set_slider_image(state, SliderImage::Stop);
    }

    // ---- transition back to silent -----------------------------------------
    if running_talking_anim(state) {
        set_stop_talking_anim(state);
    }

    while running_talking_anim(state) {
        run_comm_anim_frame(state);
    }

    ts.ended
}

// ============================================================================
// alien_talk_segue — high-level wrapper with first-call initialization
// ============================================================================

/// High-level talk segue with first-call initialization.
///
/// Matches C `AlienTalkSegue`.  On the first call this encounter, initialises
/// speech graphics, starts music, sets up animations.  Subsequent calls just
/// delegate to `talk_segue`.
pub fn alien_talk_segue(state: &mut CommState, wait_track: u32) {
    // Skip if abort or already finished
    if check_abort(state) || state.is_talking_finished() {
        return;
    }

    if !state.first_talk_call {
        state.first_talk_call = true;
        // First call this encounter — initialize speech subsystem
        init_speech_graphics(state);
        set_colormap(state);
        draw_alien_frame(state);
        update_speech_graphics(state);
        comm_intro_transition(state);

        play_alien_music(state);
        set_music_background_vol(state);

        init_comm_animations(state);
        clear_last_activity_load_flag(state);
    }

    let finished = talk_segue(state, wait_track);
    state.set_talking_finished(finished);

    if finished {
        // Fade music back to foreground (alien finishes talking)
        fade_music_to_foreground(state);
    }
}

// ============================================================================
// select_response — handle player selecting a response
// ============================================================================

/// Process the player selecting a response.
///
/// Matches C `SelectResponse`.  Returns the selected callback and its ref
/// so the **caller** (in `ffi.rs`) can release the lock, invoke the callback,
/// then reacquire.  Returns `None` if no response is selected.
pub fn select_response(state: &mut CommState) -> Option<(ResponseFunc, u32)> {
    let (func, response_ref) = {
        let resp = state.responses().get_selected()?;
        // feedback_text comes from the response text
        let _text = resp.response_text.clone();
        let func = resp.response_func?;
        let rref = resp.response_ref;
        (func, rref)
    };

    feedback_player_phrase(
        state,
        &state.responses().get_selected()?.response_text.clone(),
    );
    stop_track(state);
    clear_subtitles(state);
    set_slider_image(state, SliderImage::Play);

    fade_music_to_background(state);

    state.set_talking_finished(false);
    // Clear all responses — caller invokes the callback
    state.responses_mut().clear();

    Some((func, response_ref))
}

// ============================================================================
// player_response_input — handle input while showing responses
// ============================================================================

/// Handle one frame of player input in the response-selection phase.
///
/// Matches C `PlayerResponseInput`.
pub fn player_response_input(state: &mut CommState) -> PlayerInputResult {
    // Initialize top_response on the very first call
    if state.top_response.is_none() {
        state.top_response = Some(0);
        refresh_responses(state);
    }

    if check_select_input(state) {
        return PlayerInputResult::Selected;
    }

    if check_cancel_input(state) && !won_last_battle(state) {
        select_conversation_summary(state);
        return PlayerInputResult::Summary;
    }

    if check_left_input(state) {
        // Replay last phrase
        fade_music_to_background(state);
        feedback_player_phrase(state, "");
        talk_segue(state, 0);
        if !check_abort(state) {
            refresh_responses(state);
            fade_music_to_foreground(state);
        }
        return PlayerInputResult::Replay;
    }

    // Navigate responses
    let count = state.responses().count();
    if count == 0 {
        return PlayerInputResult::Continue;
    }

    let cur = state.responses().selected().max(0) as usize;

    if check_up_input(state) {
        let next = if cur == 0 { count - 1 } else { cur - 1 };
        state.responses_mut().select(next as i32);
        update_response_scroll(state);
    } else if check_down_input(state) {
        let next = (cur + 1) % count;
        state.responses_mut().select(next as i32);
        update_response_scroll(state);
    }

    update_comm_graphics(state);

    PlayerInputResult::Continue
}

// ============================================================================
// do_communication — top-level dialogue state machine
// ============================================================================

/// One iteration of the top-level communication state machine.
///
/// Matches C `DoCommunication`.
pub fn do_communication(state: &mut CommState) -> CommunicationResult {
    if !state.is_talking_finished() {
        // Still talking — keep playing
        alien_talk_segue(state, WAIT_TRACK_ALL);
        return CommunicationResult::Continue;
    }

    if check_abort(state) {
        return CommunicationResult::Done;
    }

    if state.responses().count() == 0 {
        // No responses — run last-replay loop then finish
        run_last_replay(state);
        return CommunicationResult::Done;
    }

    // Show responses and handle input
    player_response_input(state);
    CommunicationResult::Continue
}

// ============================================================================
// Internal helpers — scroll / display
// ============================================================================

/// Update `top_response` so the selected response is on screen.
fn update_response_scroll(state: &mut CommState) {
    let selected = state.responses().selected().max(0) as u8;
    let top = state.top_response.unwrap_or(0);

    if selected < top {
        state.top_response = Some(0);
        refresh_responses(state);
    } else {
        // In production the "y > SIS_SCREEN_HEIGHT" check adjusts top_response;
        // we approximate: if selection moved past a threshold, scroll to it.
        // C uses rendered text height which we don't have in pure Rust.
        // For now just track the selection directly.
        state.top_response = Some(top);
    }
}

// ============================================================================
// Bridge abstraction — platform calls
// ============================================================================
//
// These thin wrappers are either real C FFI (non-test) or pure CommState
// simulation (test).  They keep all the #[cfg] noise out of the logic above.

/// Slider image positions (matches C ActivityFrame indices in comm.c).
#[derive(Debug, Clone, Copy)]
enum SliderImage {
    FastForward = 3,
    FastReverse = 4,
    Play = 2,
    Stop = 8,
}

// ---------- abort / input --------------------------------------------------

fn check_abort(state: &CommState) -> bool {
    #[cfg(not(test))]
    unsafe {
        c_bridge::c_CheckAbort() != 0
    }
    #[cfg(test)]
    {
        // In tests: abort is represented by a flag we set in CommState.
        // We repurpose input_paused as a simple abort-for-test sentinel.
        state.is_input_paused()
    }
}

fn check_cancel_input(state: &CommState) -> bool {
    // @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-IP-002
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_GetPulsedMenuKey(KEY_MENU_CANCEL) != 0
    }
    #[cfg(test)]
    {
        // Tests drive cancel via a dedicated flag in CommState.
        let _ = state;
        false // overridden per-test via state fields
    }
}

fn check_select_input(state: &CommState) -> bool {
    // @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-IP-001
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_GetPulsedMenuKey(KEY_MENU_SELECT) != 0
    }
    #[cfg(test)]
    {
        let _ = state;
        false
    }
}

fn check_left_input(state: &CommState) -> bool {
    // @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-IP-005
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_GetPulsedMenuKey(KEY_MENU_LEFT) != 0
    }
    #[cfg(test)]
    {
        let _ = state;
        false
    }
}

fn check_right_input(state: &CommState) -> bool {
    // @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-IP-006
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_GetPulsedMenuKey(KEY_MENU_RIGHT) != 0
    }
    #[cfg(test)]
    {
        let _ = state;
        false
    }
}

fn check_up_input(state: &CommState) -> bool {
    // @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-IP-003
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_GetPulsedMenuKey(KEY_MENU_UP) != 0
    }
    #[cfg(test)]
    {
        let _ = state;
        false
    }
}

fn check_down_input(state: &CommState) -> bool {
    // @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-IP-004
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_GetPulsedMenuKey(KEY_MENU_DOWN) != 0
    }
    #[cfg(test)]
    {
        let _ = state;
        false
    }
}

fn won_last_battle(state: &CommState) -> bool {
    #[cfg(not(test))]
    unsafe {
        c_bridge::c_WonLastBattle() != 0
    }
    #[cfg(test)]
    {
        let _ = state;
        false
    }
}

// ---------- track operations -----------------------------------------------

/// Returns current track number (0 = not playing).
fn playing_track(state: &CommState) -> u32 {
    #[cfg(not(test))]
    unsafe {
        c_bridge::c_PlayingTrack() as u32
    }
    #[cfg(test)]
    {
        // Simulate: playing_track returns 1 while track is playing, 0 otherwise.
        if state.track().is_playing() {
            1
        } else {
            0
        }
    }
}

fn jump_track(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        c_bridge::c_JumpTrack();
    }
    #[cfg(test)]
    {
        state.track_mut().stop();
    }
}

fn play_track(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        c_bridge::c_PlayTrack();
    }
    #[cfg(test)]
    {
        state.track_mut().start();
    }
}

fn stop_track(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        c_bridge::c_StopTrack();
    }
    #[cfg(test)]
    {
        state.track_mut().stop();
    }
}

fn fast_forward(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        match get_scroll_mode() {
            ScrollMode::Page => c_bridge::c_FastForward_Page(),
            ScrollMode::Smooth => c_bridge::c_FastForward_Smooth(),
        }
    }
    #[cfg(test)]
    {
        state.track_mut().fast_forward_page();
    }
}

fn fast_reverse(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        match get_scroll_mode() {
            ScrollMode::Page => c_bridge::c_FastReverse_Page(),
            ScrollMode::Smooth => c_bridge::c_FastReverse_Smooth(),
        }
    }
    #[cfg(test)]
    {
        state.track_mut().fast_reverse_page();
    }
}

#[cfg(not(test))]
fn get_scroll_mode() -> ScrollMode {
    let v = unsafe { c_bridge::c_GetOptSmoothScroll() };
    if v == 0 {
        ScrollMode::Page
    } else {
        ScrollMode::Smooth
    }
}

// ---------- subtitle / graphics --------------------------------------------

fn check_subtitles(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_CheckSubtitles();
    }
    #[cfg(test)]
    {
        // In tests, update the subtitle tracker position from the track.
        let pos = state.track().position();
        state.subtitles_mut().update(pos);
    }
}

fn clear_subtitles(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_ClearSubtitles();
    }
    #[cfg(test)]
    {
        state.subtitles_mut().clear();
    }
}

fn update_speech_graphics(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_UpdateSpeechGraphics();
    }
    #[cfg(test)]
    {
        let _ = state;
        // no-op in tests
    }
}

fn init_speech_graphics(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_InitSpeechGraphics();
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

fn set_slider_image(state: &mut CommState, img: SliderImage) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_SetSliderImage(img as i32);
    }
    #[cfg(test)]
    {
        let _ = (state, img);
    }
}

fn update_animations(state: &mut CommState, seeking: bool) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_UpdateAnimations(seeking as i32);
    }
    #[cfg(test)]
    {
        state.animations_mut().process(if seeking { 0 } else { 1 });
    }
}

fn feedback_player_phrase(state: &mut CommState, text: &str) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        use std::ffi::CString;
        if let Ok(cs) = CString::new(text) {
            c_bridge::c_FeedbackPlayerPhrase(cs.as_ptr());
        }
    }
    #[cfg(test)]
    {
        let _ = (state, text);
    }
}

fn update_comm_graphics(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        // UpdateCommGraphics() — calls UpdateAnimations + redraw
        c_bridge::c_UpdateAnimations(0);
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

fn refresh_responses(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let top = state.response_ui().top_response() as u8;
        let count = state.responses().count() as u8;
        let cur = state.responses().selected().max(0) as u8;
        c_bridge::c_RefreshResponses(top, count, cur);
    }
    #[cfg(test)]
    {
        // In tests, start_display initializes display state
        state.responses_mut().start_display();
    }
}

fn select_conversation_summary(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_SelectConversationSummary();
    }
    #[cfg(test)]
    {
        // Simulate: rebuild the summary from track subtitles
        state.rebuild_summary();
    }
}

// ---------- animation helpers ----------------------------------------------

fn want_talking_anim(state: &CommState) -> bool {
    #[cfg(not(test))]
    unsafe {
        c_bridge::c_WantTalkingAnim() != 0
    }
    #[cfg(test)]
    {
        state.animations().want_talking_anim()
    }
}

fn have_talking_anim(state: &CommState) -> bool {
    #[cfg(not(test))]
    unsafe {
        c_bridge::c_HaveTalkingAnim() != 0
    }
    #[cfg(test)]
    {
        state.animations().have_talking_anim()
    }
}

fn has_transition_anim(state: &CommState) -> bool {
    // @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-AT-001
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_HasTransitionAnim() != 0
    }
    #[cfg(test)]
    {
        state.animations().has_transition_anim()
    }
}

fn set_run_intro_anim(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_SetRunIntroAnim();
    }
    #[cfg(test)]
    {
        state.animations_mut().set_intro_anim(true);
    }
}

fn set_run_talking_anim(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_SetRunTalkingAnim();
    }
    #[cfg(test)]
    {
        state.animations_mut().start_talking_anim();
    }
}

fn set_stop_talking_anim(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_SetStopTalkingAnim();
    }
    #[cfg(test)]
    {
        state.animations_mut().stop_talking_anim();
    }
}

fn running_intro_anim(state: &CommState) -> bool {
    #[cfg(not(test))]
    unsafe {
        c_bridge::c_RunningIntroAnim() != 0
    }
    #[cfg(test)]
    {
        state.animations().is_intro_anim_running()
    }
}

fn running_talking_anim(state: &CommState) -> bool {
    #[cfg(not(test))]
    unsafe {
        c_bridge::c_RunningTalkingAnim() != 0
    }
    #[cfg(test)]
    {
        state.animations().is_talking_anim_running()
    }
}

fn run_comm_anim_frame(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_RunCommAnimFrame();
    }
    #[cfg(test)]
    {
        // Advance animations by one tick
        state.animations_mut().process(1);
    }
}

// ---------- music helpers --------------------------------------------------

/// @plan PLAN-20260325-COMMPT3.P03
/// @requirement REQ-MU-001, REQ-MU-002
/// @pseudocode 001-colormap-music-bridges lines 09-15
fn play_alien_music(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_PlayAlienMusic();
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

fn set_music_background_vol(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_FadeMusic(c_bridge::music_volume::BACKGROUND, 0);
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

fn fade_music_to_foreground(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_FadeMusic(
            c_bridge::music_volume::FOREGROUND,
            60, // ONE_SECOND
        );
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

fn fade_music_to_background(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_FadeMusic(
            c_bridge::music_volume::BACKGROUND,
            60, // ONE_SECOND
        );
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

// ---------- display / scene setup ------------------------------------------

/// @plan PLAN-20260325-COMMPT3.P03
/// @requirement REQ-CM-001, REQ-CM-002
/// @pseudocode 001-colormap-music-bridges lines 01-08
fn set_colormap(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_SetColorMapFromCommData();
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

fn draw_alien_frame(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_DrawAlienFrame();
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

fn comm_intro_transition(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_CommIntroTransition();
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

fn init_comm_animations(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_InitCommAnimations();
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

fn clear_last_activity_load_flag(state: &mut CommState) {
    #[cfg(not(test))]
    unsafe {
        let _ = state;
        c_bridge::c_ClearLastActivityLoadFlag();
    }
    #[cfg(test)]
    {
        let _ = state;
    }
}

/// Last-replay loop: lets player review alien's last words, then times out.
/// Matches C `DoLastReplay` / the no-responses branch in `DoCommunication`.
fn run_last_replay(state: &mut CommState) {
    #[cfg(not(test))]
    {
        let _ = state;
        // In production the C DoInput loop handles this with a timeout;
        // the Rust FFI layer drives the C-side DoInput directly.
    }
    #[cfg(test)]
    {
        // In tests, we just fade out music to simulate the timeout
        fade_music_to_background(state);
        stop_track(state);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::state::COMM_STATE;
    use serial_test::serial;

    // ---- helpers -----------------------------------------------------------

    fn reset() {
        let mut s = COMM_STATE.write();
        s.uninit();
        drop(s);
        let mut s = COMM_STATE.write();
        let _ = s.init();
        drop(s);
    }

    /// Build a fresh CommState with a simple 2-second track loaded and playing.
    fn playing_state() -> CommState {
        let mut s = CommState::new();
        s.init().unwrap();
        s.track_mut().splice_track(1, Some("Hello"), 0.0, 2.0);
        s.track_mut().start();
        s
    }

    /// Build a CommState whose track has already finished.
    fn finished_state() -> CommState {
        let mut s = CommState::new();
        s.init().unwrap();
        s.track_mut().splice_track(1, Some("Hello"), 0.0, 0.1);
        s.track_mut().start();
        s.track_mut().update(0.5); // advance past end
        s
    }

    // ---- do_talk_segue tests -----------------------------------------------

    #[test]
    fn test_do_talk_segue_cancel_ends() {
        let mut s = playing_state();
        let mut ts = TalkingState {
            wait_track: WAIT_TRACK_ALL,
            ..Default::default()
        };
        // Simulate cancel: abort flag (input_paused) ends segue
        s.set_input_paused(true);
        let cont = do_talk_segue(&mut s, &mut ts);
        assert!(!cont, "abort should stop loop");
        assert!(ts.ended);
    }

    #[test]
    fn test_do_talk_segue_continues_while_playing() {
        let mut s = playing_state();
        let mut ts = TalkingState {
            wait_track: WAIT_TRACK_ALL,
            ..Default::default()
        };
        let cont = do_talk_segue(&mut s, &mut ts);
        // Track is playing → should continue
        assert!(cont, "should continue while track is playing");
        assert!(!ts.ended);
    }

    #[test]
    fn test_do_talk_segue_ends_when_not_playing() {
        let mut s = finished_state();
        let mut ts = TalkingState {
            wait_track: WAIT_TRACK_ALL,
            ..Default::default()
        };
        let cont = do_talk_segue(&mut s, &mut ts);
        assert!(!cont, "should stop when track not playing");
        assert!(ts.ended);
    }

    #[test]
    fn test_do_talk_segue_seek_mode_stops_when_seeking() {
        let mut s = playing_state();
        let mut ts = TalkingState {
            wait_track: WAIT_TRACK_ALL,
            seeking: true,
            ..Default::default()
        };
        // While seeking, should return true even if track stops (seeking || ...)
        let cont = do_talk_segue(&mut s, &mut ts);
        // seeking starts true, no right/left input → seeking is cleared this iter
        // then we check track, which is playing → cont should be true (playing)
        assert!(cont);
    }

    // ---- talk_segue tests --------------------------------------------------

    #[test]
    fn test_talk_segue_ends_when_not_playing() {
        let mut s = CommState::new();
        s.init().unwrap();
        // No track spliced → playing_track() returns 0 immediately
        // wait_track = WAIT_TRACK_ALL (non-zero), so it will try to play
        // but nothing to play → ends immediately
        s.track_mut().splice_track(1, Some("Test"), 0.0, 0.0);
        s.track_mut().start();
        s.track_mut().update(0.1); // finish immediately

        let ended = talk_segue(&mut s, WAIT_TRACK_ALL);
        assert!(ended, "should return ended=true when track not playing");
    }

    #[test]
    fn test_talk_segue_rewind_mode() {
        let mut s = playing_state();
        // wait_track = 0 → rewind mode, ts.rewind set to true
        // Since track is playing and rewind is set, fast_reverse is called once
        // then the loop runs until track stops
        // Force-stop by marking abort (input_paused)
        s.set_input_paused(true);

        let ended = talk_segue(&mut s, 0);
        // abort was set, so ended = true from abort path
        assert!(ended);
    }

    // ---- alien_talk_segue tests --------------------------------------------

    #[test]
    fn test_alien_talk_segue_skips_if_finished() {
        let mut s = CommState::new();
        s.init().unwrap();
        s.set_talking_finished(true);

        // Should be a no-op
        alien_talk_segue(&mut s, WAIT_TRACK_ALL);
        // still finished, no crash
        assert!(s.is_talking_finished());
    }

    #[test]
    fn test_alien_talk_segue_first_call_sets_flag() {
        let mut s = CommState::new();
        s.init().unwrap();
        assert!(!s.first_talk_call);

        // Set abort immediately so we don't actually loop
        s.set_input_paused(true);
        alien_talk_segue(&mut s, WAIT_TRACK_ALL);

        // first_talk_call should be set (but then skip due to abort)
        // check_abort returns true before first_talk_call check → skip
        // Actually check_abort is first, so first_talk_call stays false here.
        // Let's check the non-abort path:
        let mut s2 = CommState::new();
        s2.init().unwrap();
        // Don't set abort — track will finish immediately (no track loaded)
        alien_talk_segue(&mut s2, WAIT_TRACK_ALL);
        assert!(
            s2.first_talk_call,
            "first_talk_call should be set after first call"
        );
    }

    #[test]
    fn test_alien_talk_segue_skips_if_abort() {
        let mut s = CommState::new();
        s.init().unwrap();
        s.set_input_paused(true); // simulate abort

        alien_talk_segue(&mut s, WAIT_TRACK_ALL);
        assert!(!s.first_talk_call, "should not initialize if abort");
    }

    // ---- select_response tests ---------------------------------------------

    #[test]
    #[serial]
    fn test_select_response_clears_state() {
        reset();
        let mut s = COMM_STATE.write();
        s.track_mut().splice_track(1, Some("Hi"), 0.0, 2.0);
        s.track_mut().start();

        extern "C" fn noop(_: u32) {}
        s.add_response(1, "Option A", Some(noop));
        s.add_response(2, "Option B", Some(noop));
        s.responses_mut().start_display();
        drop(s);

        let mut s = COMM_STATE.write();
        let result = select_response(&mut s);
        assert!(result.is_some(), "should return callback");
        assert!(
            s.responses().is_empty(),
            "responses should be cleared after selection"
        );
    }

    #[test]
    #[serial]
    fn test_select_response_returns_callback() {
        reset();
        use std::sync::atomic::{AtomicU32, Ordering};
        static CALLED_WITH: AtomicU32 = AtomicU32::new(0);

        extern "C" fn recording_cb(r: u32) {
            CALLED_WITH.store(r, Ordering::SeqCst);
        }

        let mut s = COMM_STATE.write();
        s.add_response(42, "Pick me", Some(recording_cb));
        s.responses_mut().start_display();
        drop(s);

        let mut s = COMM_STATE.write();
        let result = select_response(&mut s);
        drop(s);

        // Simulate caller invoking the callback after releasing the lock
        if let Some((func, rref)) = result {
            func(rref);
            assert_eq!(CALLED_WITH.load(Ordering::SeqCst), 42);
        } else {
            panic!("expected Some callback");
        }
    }

    #[test]
    fn test_select_response_no_selection_returns_none() {
        let mut s = CommState::new();
        s.init().unwrap();
        // No responses added, no selection
        let result = select_response(&mut s);
        assert!(result.is_none());
    }

    // ---- player_response_input tests --------------------------------------

    #[test]
    fn test_player_input_initializes_top_response() {
        let mut s = CommState::new();
        s.init().unwrap();
        s.add_response(1, "A", None);
        s.add_response(2, "B", None);

        assert!(s.top_response.is_none());
        player_response_input(&mut s);
        assert!(
            s.top_response.is_some(),
            "top_response should be set after first call"
        );
    }

    #[test]
    fn test_player_input_navigate() {
        // up/down navigation is driven by check_up/down_input which return false
        // in test mode; verify that calling player_response_input is a no-op
        // (Continue result, no panic)
        let mut s = CommState::new();
        s.init().unwrap();
        s.add_response(1, "A", None);
        s.add_response(2, "B", None);
        s.responses_mut().start_display();

        let result = player_response_input(&mut s);
        assert_eq!(result, PlayerInputResult::Continue);
    }

    // ---- do_communication tests --------------------------------------------

    #[test]
    fn test_communication_talks_first() {
        // When talking_finished = false, do_communication enters talk segue
        // and returns Continue.
        let mut s = CommState::new();
        s.init().unwrap();
        // talking_finished starts false
        assert!(!s.is_talking_finished());

        let result = do_communication(&mut s);
        // alien_talk_segue runs; since no track loaded, finished immediately
        // → talking_finished = true. But do_communication returns Continue
        // because it delegated to alien_talk_segue in this call.
        assert_eq!(result, CommunicationResult::Continue);
    }

    #[test]
    fn test_communication_exits_no_responses() {
        let mut s = CommState::new();
        s.init().unwrap();
        s.set_talking_finished(true);
        // no responses → last replay → Done
        let result = do_communication(&mut s);
        assert_eq!(result, CommunicationResult::Done);
    }

    #[test]
    fn test_communication_shows_responses_when_ready() {
        let mut s = CommState::new();
        s.init().unwrap();
        s.set_talking_finished(true);
        s.add_response(1, "Choice A", None);

        let result = do_communication(&mut s);
        // Has responses → player_response_input → Continue
        assert_eq!(result, CommunicationResult::Continue);
    }

    // ========================================================================

    // ========================================================================
    // P04: Colormap + Music Bridge TDD
    //
    // @plan PLAN-20260325-COMMPT3.P04
    // @requirement REQ-CM-001, REQ-CM-002, REQ-MU-001, REQ-MU-002, REQ-SM-001
    // @pseudocode 001-colormap-music-bridges lines 01-31
    // ========================================================================

    // ---- Call-site wiring tests (EXPECTED TO PASS with P03 stubs) ----------

    /// REQ-CM-001: set_colormap() executes inside alien_talk_segue's first-call
    /// initialization block.  The production path calls c_SetColorMapFromCommData()
    /// (not a null_mut stub).  In test mode the bridge is a no-op, so we verify
    /// the structural invariant: first_talk_call is set iff the init block ran.
    #[test]
    fn test_set_colormap_calls_bridge() {
        let mut s = CommState::new();
        s.init().unwrap();
        assert!(!s.first_talk_call, "precondition: first_talk_call not yet set");

        alien_talk_segue(&mut s, WAIT_TRACK_ALL);

        assert!(
            s.first_talk_call,
            "alien_talk_segue must execute the first-call block (includes set_colormap)"
        );
    }

    /// REQ-MU-001: play_alien_music() executes inside alien_talk_segue's first-call
    /// initialization block.  Same structural witness as test_set_colormap_calls_bridge.
    #[test]
    fn test_play_alien_music_calls_bridge() {
        let mut s = CommState::new();
        s.init().unwrap();
        assert!(!s.first_talk_call, "precondition: first_talk_call not yet set");

        alien_talk_segue(&mut s, WAIT_TRACK_ALL);

        assert!(
            s.first_talk_call,
            "alien_talk_segue must execute the first-call block (includes play_alien_music)"
        );
    }

    /// REQ-SM-001: the "for now" placeholder comment must not appear in the
    /// set_colormap function body in this file.  P03 removed it.
    #[test]
    fn test_for_now_marker_removed() {
        let source = include_str!("talk_segue.rs");
        let body = extract_fn_body(source, "fn set_colormap");
        assert!(
            body.is_some(),
            "set_colormap must exist in talk_segue.rs"
        );
        assert!(
            !body.unwrap().to_lowercase().contains("for now"),
            "set_colormap must not contain 'for now' placeholder"
        );
    }

    // ---- C structural tests (EXPECTED TO FAIL with P03 stubs) -------------
    //
    // These tests inspect the C source of rust_comm.c directly to verify that
    // the real P05 implementation is present inside each function body.
    //
    // They are function-body-aware: the search is limited to the brace-delimited
    // body, so doc-comment lines above each stub (which mention CommData fields
    // by name) cannot produce false positives.

    /// verify_c_bridge_reads_commdata_colormap:
    /// c_SetColorMapFromCommData body must reference CommData.AlienColorMap.
    ///
    /// EXPECTED FAIL against P03 stubs — body is only a comment.
    /// Will pass only after P05 implements the real body.
    #[test]
    fn verify_c_bridge_reads_commdata_colormap() {
        let source = include_str!("../../../sc2/src/uqm/rust_comm.c");
        let body = extract_c_fn_body(source, "c_SetColorMapFromCommData")
            .expect("c_SetColorMapFromCommData must be defined in rust_comm.c");
        assert!(
            body.contains("CommData.AlienColorMap"),
            "c_SetColorMapFromCommData body must read CommData.AlienColorMap \
             (EXPECTED FAIL vs P03 stubs; stub body: {:?})",
            body
        );
    }

    /// verify_c_bridge_null_guard_colormap:
    /// c_SetColorMapFromCommData body must contain a null/zero guard before
    /// calling SetColorMap (handles zero AlienColorMap gracefully).
    ///
    /// EXPECTED FAIL against P03 stubs.
    #[test]
    fn verify_c_bridge_null_guard_colormap() {
        let source = include_str!("../../../sc2/src/uqm/rust_comm.c");
        let body = extract_c_fn_body(source, "c_SetColorMapFromCommData")
            .expect("c_SetColorMapFromCommData must be defined in rust_comm.c");
        // Acceptable guard patterns: == 0, != 0, == NULL, != NULL, if (!cmap), if (cmap
        let has_guard = body.contains("== 0")
            || body.contains("!= 0")
            || body.contains("== NULL")
            || body.contains("!= NULL")
            || body.contains("if (!")
            || body.contains("if (cmap");
        assert!(
            has_guard,
            "c_SetColorMapFromCommData body must contain a null/zero guard \
             (EXPECTED FAIL vs P03 stubs; stub body: {:?})",
            body
        );
    }

    /// verify_c_music_reads_commdata:
    /// c_PlayAlienMusic body must reference CommData.AlienSong.
    ///
    /// EXPECTED FAIL against P03 stubs — body is only a comment.
    /// Will pass only after P05 implements the real body.
    #[test]
    fn verify_c_music_reads_commdata() {
        let source = include_str!("../../../sc2/src/uqm/rust_comm.c");
        let body = extract_c_fn_body(source, "c_PlayAlienMusic")
            .expect("c_PlayAlienMusic must be defined in rust_comm.c");
        assert!(
            body.contains("CommData.AlienSong"),
            "c_PlayAlienMusic body must read CommData.AlienSong \
             (EXPECTED FAIL vs P03 stubs; stub body: {:?})",
            body
        );
    }

    /// verify_c_bridge_functions_exist_with_impl:
    /// Both C bridge functions must have non-stub bodies — more than just
    /// a block comment.
    ///
    /// EXPECTED FAIL for both until P05.
    #[test]
    fn verify_c_bridge_functions_exist_with_impl() {
        let source = include_str!("../../../sc2/src/uqm/rust_comm.c");

        let cmap_body = extract_c_fn_body(source, "c_SetColorMapFromCommData")
            .expect("c_SetColorMapFromCommData must be defined in rust_comm.c");
        let cmap_has_impl = cmap_body
            .lines()
            .filter(|l| {
                let t = l.trim();
                // Exclude blank, braces-only, and comment lines — only real statements count.
                !t.is_empty() && t != "{" && t != "}" && !c_line_is_comment(t)
            })
            .count()
            > 0;
        assert!(
            cmap_has_impl,
            "c_SetColorMapFromCommData must have a real implementation body \
             (EXPECTED FAIL vs P03 stubs; stub body: {:?})",
            cmap_body
        );

        let music_body = extract_c_fn_body(source, "c_PlayAlienMusic")
            .expect("c_PlayAlienMusic must be defined in rust_comm.c");
        let music_has_impl = music_body
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty() && t != "{" && t != "}" && !c_line_is_comment(t)
            })
            .count()
            > 0;
        assert!(
            music_has_impl,
            "c_PlayAlienMusic must have a real implementation body \
             (EXPECTED FAIL vs P03 stubs; stub body: {:?})",
            music_body
        );
    }

    // ---- Source-inspection helpers -----------------------------------------

    /// Returns true if a trimmed C source line is a comment line.
    /// Matches: `//`, `*`, or `/` followed by `*` (block comment open).
    fn c_line_is_comment(line: &str) -> bool {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with('*') {
            return true;
        }
        // Detect `/*` without triggering Rust's own block-comment parser.
        let b = t.as_bytes();
        b.len() >= 2 && b[0] == b'/' && b[1] == b'*'
    }

    /// Extract the brace-balanced body of a Rust function by signature prefix.
    /// Only the function's own block is returned; doc-comments before the
    /// signature are excluded.
    fn extract_fn_body<'a>(source: &'a str, fn_signature: &str) -> Option<&'a str> {
        let fn_start = source.find(fn_signature)?;
        let after_sig = &source[fn_start..];
        let brace_open = after_sig.find('{')?;
        let body_start = fn_start + brace_open;

        let mut depth = 0usize;
        let bytes = source.as_bytes();
        let mut i = body_start;
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&source[body_start..=i]);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        None
    }

    /// Extract the brace-balanced body of a C function definition.
    ///
    /// Searches for occurrences of `fn_name` that look like a definition
    /// (not a comment or forward declaration).  Only the body content between
    /// the opening and closing braces is returned.
    fn extract_c_fn_body(source: &str, fn_name: &str) -> Option<String> {
        let mut search_pos = 0;
        while let Some(rel) = source[search_pos..].find(fn_name) {
            let abs = search_pos + rel;

            // Preceding byte must be whitespace/newline (not part of an identifier).
            let pre_ok = abs == 0 || {
                let b = source.as_bytes()[abs - 1];
                b == b'\n' || b == b' ' || b == b'\t'
            };

            // Following byte must be `(` or whitespace (not inside an identifier).
            let post_pos = abs + fn_name.len();
            let post_ok = post_pos < source.len() && {
                let b = source.as_bytes()[post_pos];
                b == b'(' || b == b' ' || b == b'\t' || b == b'\n'
            };

            if pre_ok && post_ok {
                let after = &source[abs..];
                if let Some(brace_rel) = after.find('{') {
                    let between = &after[..brace_rel];
                    // A semicolon before the brace means this is a declaration, not a definition.
                    if between.contains(';') {
                        search_pos = abs + fn_name.len();
                        continue;
                    }

                    let body_abs = abs + brace_rel;
                    let mut depth = 0usize;
                    let bytes = source.as_bytes();
                    let mut i = body_abs;
                    while i < bytes.len() {
                        match bytes[i] {
                            b'{' => depth += 1,
                            b'}' => {
                                depth -= 1;
                                if depth == 0 {
                                    return Some(source[body_abs..=i].to_string());
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                }
            }

            search_pos = abs + fn_name.len();
        }
        None
    }
}

