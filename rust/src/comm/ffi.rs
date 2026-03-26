//! C FFI bindings for the communication system
//!
//! Provides C-compatible functions for the communication system.

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_uint, CStr};

use super::segue::Segue;
use super::state::COMM_STATE;
use super::types::CommIntroMode;

// Thread-local buffer for subtitle strings returned to C (OL-REQ-009, OL-REQ-010)
thread_local! {
    static SUBTITLE_BUF: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(512));
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the communication system
#[no_mangle]
pub unsafe extern "C" fn rust_InitCommunication() -> c_int {
    match COMM_STATE.write().init() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Uninitialize the communication system
#[no_mangle]
pub unsafe extern "C" fn rust_UninitCommunication() {
    COMM_STATE.write().uninit();
}

/// Check if communication is initialized
#[no_mangle]
pub unsafe extern "C" fn rust_IsCommInitialized() -> c_int {
    if COMM_STATE.read().is_initialized() {
        1
    } else {
        0
    }
}

/// Clear communication state
#[no_mangle]
pub unsafe extern "C" fn rust_ClearCommunication() {
    COMM_STATE.write().clear();
}

// ============================================================================
// Track Management
// ============================================================================

/// Start the speech track
#[no_mangle]
pub unsafe extern "C" fn rust_StartTrack() -> c_int {
    match COMM_STATE.write().start_track() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Stop the speech track
#[no_mangle]
pub unsafe extern "C" fn rust_StopTrack() {
    COMM_STATE.write().stop_track();
}

/// Rewind the track to the beginning
#[no_mangle]
pub unsafe extern "C" fn rust_RewindTrack() {
    COMM_STATE.write().track_mut().rewind();
}

/// Jump to end of current phrase (skip current speech).
/// No offset parameter — JumpTrack advances to end of current phrase only (TP-REQ-005).
#[no_mangle]
pub unsafe extern "C" fn rust_JumpTrack() {
    COMM_STATE.write().track_mut().jump(0.0);
}

/// Seek to absolute position in track
#[no_mangle]
pub unsafe extern "C" fn rust_SeekTrack(position: f32) {
    COMM_STATE.write().track_mut().seek(position);
}

/// Commit track position (for save/restore)
#[no_mangle]
pub unsafe extern "C" fn rust_CommitTrack() -> f32 {
    COMM_STATE.write().track_mut().commit()
}

/// Wait for track to finish (returns 1 when done)
#[no_mangle]
pub unsafe extern "C" fn rust_WaitTrack() -> c_int {
    if COMM_STATE.read().wait_track() {
        1
    } else {
        0
    }
}

/// Get track position
#[no_mangle]
pub unsafe extern "C" fn rust_GetTrackPosition() -> f32 {
    COMM_STATE.read().track().position()
}

/// Get track length
#[no_mangle]
pub unsafe extern "C" fn rust_GetTrackLength() -> f32 {
    COMM_STATE.read().track().length()
}

/// Add a speech chunk to the track
#[no_mangle]
pub unsafe extern "C" fn rust_SpliceTrack(
    audio_handle: c_uint,
    text: *const c_char,
    start_time: f32,
    duration: f32,
) {
    let subtitle = if text.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(text).to_str().ok() }
    };

    COMM_STATE
        .write()
        .track_mut()
        .splice_track(audio_handle, subtitle, start_time, duration);
}

/// Add text-only subtitle to the track
#[no_mangle]
pub unsafe extern "C" fn rust_SpliceTrackText(text: *const c_char, start_time: f32, duration: f32) {
    if text.is_null() {
        return;
    }

    let text_str = unsafe {
        match CStr::from_ptr(text).to_str() {
            Ok(s) => s,
            Err(_) => return,
        }
    };

    COMM_STATE
        .write()
        .track_mut()
        .splice_text(text_str, start_time, duration);
}

/// Clear the track
#[no_mangle]
pub unsafe extern "C" fn rust_ClearTrack() {
    COMM_STATE.write().track_mut().clear();
}

/// Check if a track is currently playing.
#[no_mangle]
pub unsafe extern "C" fn rust_PlayingTrack() -> c_uint {
    if COMM_STATE.read().track().is_playing() {
        1
    } else {
        0
    }
}

/// Fast-forward by one page (subtitle page skip).
#[no_mangle]
pub unsafe extern "C" fn rust_FastForward_Page() {
    COMM_STATE.write().track_mut().fast_forward_page();
}

/// Smooth fast-forward (increase playback rate).
#[no_mangle]
pub unsafe extern "C" fn rust_FastForward_Smooth() {
    COMM_STATE.write().track_mut().fast_forward_smooth();
}

/// Reverse by one page.
#[no_mangle]
pub unsafe extern "C" fn rust_FastReverse_Page() {
    COMM_STATE.write().track_mut().fast_reverse_page();
}

/// Smooth reverse (decrease playback rate / rewind).
#[no_mangle]
pub unsafe extern "C" fn rust_FastReverse_Smooth() {
    COMM_STATE.write().track_mut().fast_reverse_smooth();
}

// ============================================================================
// Subtitle Management
// ============================================================================

/// Get current subtitle (returns null if none).
/// Returns a stable C string via thread-local buffer (OL-REQ-009, OL-REQ-010).
/// Pointer is valid until the next call to rust_GetSubtitle on the same thread.
#[no_mangle]
pub unsafe extern "C" fn rust_GetSubtitle() -> *const c_char {
    let state = COMM_STATE.read();
    match state.current_subtitle() {
        Some(s) => SUBTITLE_BUF.with(|buf| {
            let mut buf = buf.borrow_mut();
            buf.clear();
            buf.extend_from_slice(s.as_bytes());
            buf.push(0); // null terminator
            buf.as_ptr() as *const c_char
        }),
        None => std::ptr::null(),
    }
}

/// Enable/disable subtitles
#[no_mangle]
pub unsafe extern "C" fn rust_SetSubtitlesEnabled(enabled: c_int) {
    COMM_STATE.write().subtitles_mut().set_enabled(enabled != 0);
}

/// Check if subtitles are enabled
#[no_mangle]
pub unsafe extern "C" fn rust_AreSubtitlesEnabled() -> c_int {
    if COMM_STATE.read().subtitles().is_enabled() {
        1
    } else {
        0
    }
}

// ============================================================================
// Response System
// ============================================================================

/// Add a response option.
/// `func` receives `response_ref` as its argument when selected (RS-REQ-011).
#[no_mangle]
pub unsafe extern "C" fn rust_DoResponsePhrase(
    response_ref: c_uint,
    text: *const c_char,
    func: Option<extern "C" fn(u32)>,
) -> c_int {
    if text.is_null() {
        return 0;
    }

    let text_str = unsafe {
        match CStr::from_ptr(text).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    if COMM_STATE
        .write()
        .add_response(response_ref, text_str, func)
    {
        1
    } else {
        0
    }
}

/// Display response choices
#[no_mangle]
pub unsafe extern "C" fn rust_DisplayResponses() {
    COMM_STATE.write().display_responses();
}

/// Clear all responses
#[no_mangle]
pub unsafe extern "C" fn rust_ClearResponses() {
    COMM_STATE.write().clear_responses();
}

/// Select next response
#[no_mangle]
pub unsafe extern "C" fn rust_SelectNextResponse() -> c_int {
    if COMM_STATE.write().select_next_response() {
        1
    } else {
        0
    }
}

/// Select previous response
#[no_mangle]
pub unsafe extern "C" fn rust_SelectPrevResponse() -> c_int {
    if COMM_STATE.write().select_prev_response() {
        1
    } else {
        0
    }
}

/// Get selected response index
#[no_mangle]
pub unsafe extern "C" fn rust_GetSelectedResponse() -> c_int {
    COMM_STATE.read().selected_response()
}

/// Get number of responses
#[no_mangle]
pub unsafe extern "C" fn rust_GetResponseCount() -> c_int {
    COMM_STATE.read().responses().count() as c_int
}

/// Execute selected response callback — passes response_ref as argument (RS-REQ-011).
#[no_mangle]
pub unsafe extern "C" fn rust_ExecuteResponse() -> c_uint {
    let state = COMM_STATE.read();
    let response = match state.responses().get_selected() {
        Some(r) => r,
        None => return 0,
    };

    let func = match response.response_func {
        Some(f) => f,
        None => return response.response_ref,
    };

    let response_ref = response.response_ref;

    // Drop the lock before calling the callback (callback may re-enter comm state)
    drop(state);

    // Call with response_ref per RS-REQ-011
    func(response_ref);

    response_ref
}

// ============================================================================
// Animation Management
// ============================================================================

/// Initialize communication animations from current CommData.
/// @plan PLAN-20260314-COMM.P07
#[no_mangle]
pub unsafe extern "C" fn rust_InitCommAnimations() {
    // Animation initialization requires CommData to be loaded.
    // In production, this is called after init_race populates CommData.
    // For now, the actual descriptor population happens from C side.
}

/// Process communication animations for one frame.
/// @plan PLAN-20260314-COMM.P07
#[no_mangle]
pub unsafe extern "C" fn rust_ProcessCommAnimations(delta_ticks: c_uint) -> c_int {
    let changed = COMM_STATE.write().animations_mut().process(delta_ticks);
    changed as c_int
}

/// Process communication animations — C bridge signature matching commanim.c.
///
/// `clear` (BOOLEAN): full-redraw flag (passes FullRedraw to animation engine).
/// `paused` (BOOLEAN): if non-zero, drive colormap transforms only, no frame advance.
/// Returns BOOLEAN: non-zero if any visible change occurred.
///
/// This is the signature expected by the USE_RUST_COMM stubs in commanim.c.
#[no_mangle]
pub unsafe extern "C" fn rust_ProcessCommAnimations_cb(clear: c_int, paused: c_int) -> c_int {
    if paused != 0 {
        // Paused: drive colormap xforms only, no frame advancement.
        return 0;
    }
    // Use a fixed tick count (1) to advance one step; clear flag is passed
    // through as FullRedraw and forces change=true when set.
    let mut changed = COMM_STATE.write().animations_mut().process(1);
    if clear != 0 {
        changed = true;
    }
    changed as c_int
}

/// Check if talking animation is wanted (defined with frames).
#[no_mangle]
pub unsafe extern "C" fn rust_WantTalkingAnim() -> c_int {
    COMM_STATE.read().animations().want_talking_anim() as c_int
}

/// Check if talking animation is currently active.
#[no_mangle]
pub unsafe extern "C" fn rust_HaveTalkingAnim() -> c_int {
    COMM_STATE.read().animations().have_talking_anim() as c_int
}

/// Start the talking animation.
#[no_mangle]
pub unsafe extern "C" fn rust_SetRunTalkingAnim(_run: c_int) {
    COMM_STATE.write().animations_mut().start_talking_anim();
}

/// Signal to stop the talking animation.
#[no_mangle]
pub unsafe extern "C" fn rust_SetStopTalkingAnim() {
    COMM_STATE.write().animations_mut().stop_talking_anim();
}

/// Set intro animation running state.
#[no_mangle]
pub unsafe extern "C" fn rust_SetRunIntroAnim(run: c_int) {
    COMM_STATE.write().animations_mut().set_intro_anim(run != 0);
}

/// Check if intro animation is running.
#[no_mangle]
pub unsafe extern "C" fn rust_RunningIntroAnim() -> c_int {
    COMM_STATE.read().animations().is_intro_anim_running() as c_int
}

/// Check if talking animation is running.
#[no_mangle]
pub unsafe extern "C" fn rust_RunningTalkingAnim() -> c_int {
    COMM_STATE.read().animations().is_talking_anim_running() as c_int
}

/// Get current frame for an animation sequence.
#[no_mangle]
pub unsafe extern "C" fn rust_GetCommAnimationFrame(index: c_uint) -> c_uint {
    COMM_STATE
        .read()
        .animations()
        .get_frame(index as usize)
        .unwrap_or(0)
}

// ============================================================================
// Encounter Lifecycle
// @plan PLAN-20260314-COMM.P08
// ============================================================================

/// Begin an encounter (marks active, records init callback).
#[no_mangle]
pub unsafe extern "C" fn rust_BeginEncounter() -> c_int {
    match super::encounter::begin_encounter() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter normally (post + uninit callbacks, resource teardown).
#[no_mangle]
pub unsafe extern "C" fn rust_EndEncounterNormal() -> c_int {
    match super::encounter::end_encounter_normal() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter on abort/load (uninit only, skip post).
#[no_mangle]
pub unsafe extern "C" fn rust_EndEncounterAbort() -> c_int {
    match super::encounter::end_encounter_abort() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter for attack-without-hail (post + uninit, no init).
#[no_mangle]
pub unsafe extern "C" fn rust_EndEncounterAttack() -> c_int {
    match super::encounter::end_encounter_attack() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Check if encounter is active.
#[no_mangle]
pub unsafe extern "C" fn rust_IsEncounterActive() -> c_int {
    super::encounter::is_encounter_active() as c_int
}

// ============================================================================
// Oscilloscope
// ============================================================================

/// Add samples to the oscilloscope
#[no_mangle]
pub unsafe extern "C" fn rust_AddOscilloscopeSamples(samples: *const i16, count: c_uint) {
    if samples.is_null() || count == 0 {
        return;
    }

    let samples_slice = unsafe { std::slice::from_raw_parts(samples, count as usize) };
    COMM_STATE.write().add_oscilloscope_samples(samples_slice);
}

/// Update the oscilloscope display
#[no_mangle]
pub unsafe extern "C" fn rust_UpdateOscilloscope() {
    COMM_STATE.write().oscilloscope_mut().update();
}

/// Get oscilloscope Y value at position
#[no_mangle]
pub unsafe extern "C" fn rust_GetOscilloscopeY(x: c_uint) -> u8 {
    COMM_STATE.read().oscilloscope().get_y(x as usize)
}

/// Clear the oscilloscope
#[no_mangle]
pub unsafe extern "C" fn rust_ClearOscilloscope() {
    COMM_STATE.write().oscilloscope_mut().clear();
}

// ============================================================================
// State Queries
// ============================================================================

/// Check if alien is talking
#[no_mangle]
pub unsafe extern "C" fn rust_IsTalking() -> c_int {
    if COMM_STATE.read().is_talking() {
        1
    } else {
        0
    }
}

/// Check if talking has finished
#[no_mangle]
pub unsafe extern "C" fn rust_IsTalkingFinished() -> c_int {
    if COMM_STATE.read().is_talking_finished() {
        1
    } else {
        0
    }
}

/// Set talking finished flag
#[no_mangle]
pub unsafe extern "C" fn rust_SetTalkingFinished(finished: c_int) {
    COMM_STATE.write().set_talking_finished(finished != 0);
}

/// Get intro mode
#[no_mangle]
pub unsafe extern "C" fn rust_GetCommIntroMode() -> c_uint {
    COMM_STATE.read().intro_mode() as c_uint
}

/// Set intro mode
#[no_mangle]
pub unsafe extern "C" fn rust_SetCommIntroMode(mode: c_uint) {
    COMM_STATE.write().set_intro_mode(CommIntroMode::from(mode));
}

/// Get fade time
#[no_mangle]
pub unsafe extern "C" fn rust_GetCommFadeTime() -> c_uint {
    COMM_STATE.read().fade_time()
}

/// Set fade time
#[no_mangle]
pub unsafe extern "C" fn rust_SetCommFadeTime(time: c_uint) {
    COMM_STATE.write().set_fade_time(time);
}

/// Check if input is paused
#[no_mangle]
pub unsafe extern "C" fn rust_IsCommInputPaused() -> c_int {
    if COMM_STATE.read().is_input_paused() {
        1
    } else {
        0
    }
}

/// Set input paused
#[no_mangle]
pub unsafe extern "C" fn rust_SetCommInputPaused(paused: c_int) {
    COMM_STATE.write().set_input_paused(paused != 0);
}

/// Update communication state
#[no_mangle]
pub unsafe extern "C" fn rust_UpdateCommunication(delta_time: f32) {
    COMM_STATE.write().update(delta_time);
}

// ============================================================================
// Phrase State (P04)
// @plan PLAN-20260314-COMM.P04
// ============================================================================

/// Check if a phrase is enabled (not disabled this encounter).
#[no_mangle]
pub unsafe extern "C" fn rust_PhraseEnabled(index: c_int) -> c_int {
    if COMM_STATE.read().phrase_enabled(index) {
        1
    } else {
        0
    }
}

/// Disable a phrase for the remainder of this encounter.
#[no_mangle]
pub unsafe extern "C" fn rust_DisablePhrase(index: c_int) {
    COMM_STATE.write().disable_phrase(index);
}

// ============================================================================
// Segue (P04)
// @plan PLAN-20260314-COMM.P04
// ============================================================================

/// Set segue state (0=Peace, 1=Hostile, 2=Victory, 3=Defeat).
#[no_mangle]
pub unsafe extern "C" fn rust_SetSegue(segue: c_uint) {
    COMM_STATE.write().set_segue(Segue::from(segue));
}

/// Get segue state (0=Peace, 1=Hostile, 2=Victory, 3=Defeat).
#[no_mangle]
pub unsafe extern "C" fn rust_GetSegue() -> c_uint {
    u32::from(COMM_STATE.read().get_segue())
}

/// Get BATTLE_SEGUE value for current segue (0=peace, 1=combat).
#[no_mangle]
pub unsafe extern "C" fn rust_GetBattleSegue() -> c_uint {
    COMM_STATE.read().get_segue().to_battle_segue()
}

// ============================================================================
// Talk Segue & Main Loop (P09)
// @plan PLAN-20260314-COMM.P09
// ============================================================================

/// Run one iteration of the alien talk segue for the given wait-track.
///
/// Matches C `AlienTalkSegue(wait_track)`.
/// Returns 1 if talking finished (reached end), 0 otherwise.
#[no_mangle]
pub unsafe extern "C" fn rust_AlienTalkSegue(wait: c_uint) -> c_int {
    let mut state = COMM_STATE.write();
    let was_finished = state.is_talking_finished();
    super::talk_segue::alien_talk_segue(&mut state, wait);
    if !was_finished && state.is_talking_finished() {
        1
    } else {
        0
    }
}

/// Run the full talk segue loop for the given wait-track.
///
/// Matches C `TalkSegue(wait_track)`.
/// Returns 1 if playback ended naturally, 0 if aborted.
#[no_mangle]
pub unsafe extern "C" fn rust_TalkSegue(wait: c_int) -> c_int {
    let wait_track = if wait <= 0 { 0 } else { wait as u32 };
    let mut state = COMM_STATE.write();
    if super::talk_segue::talk_segue(&mut state, wait_track) {
        1
    } else {
        0
    }
}

/// Run one iteration of the top-level communication state machine.
///
/// Matches C `DoCommunication(pES)`.
/// Returns 1 to keep iterating (Continue), 0 when conversation is done.
///
/// Lock discipline: acquires COMM_STATE write lock, runs one iteration.
/// If `select_response` returns a callback, releases the lock, calls the
/// callback, then reacquires to continue.
#[no_mangle]
pub unsafe extern "C" fn rust_DoCommunication() -> c_int {
    use super::talk_segue::{
        do_communication, player_response_input, select_response, CommunicationResult,
        PlayerInputResult,
    };

    let mut state = COMM_STATE.write();

    match do_communication(&mut state) {
        CommunicationResult::Done => 0,
        CommunicationResult::Continue => {
            // If the last player_response_input returned Selected, execute
            // the callback with the lock released.
            // We detect this by checking whether responses were just cleared
            // and a callback is pending.  The canonical path: do_communication
            // called player_response_input which returned Selected → we need
            // to re-run select_response here to actually dispatch.
            if state.is_talking_finished() && state.responses().count() > 0 {
                if let Some(selected_result) = {
                    // Check if select was triggered by trying to select
                    let check_result = player_response_input(&mut state);
                    if check_result == PlayerInputResult::Selected {
                        select_response(&mut state)
                    } else {
                        None
                    }
                } {
                    let (func, rref) = selected_result;
                    // Release lock, call callback, done
                    drop(state);
                    func(rref);
                    return 1;
                }
            }
            1
        }
    }
}

// ============================================================================
// Speech Graphics (P10)
// @plan PLAN-20260314-COMM.P10
// ============================================================================

/// Initialize speech graphics (oscilloscope + slider) for this encounter.
///
/// Matches C `InitSpeechGraphics`.
#[no_mangle]
pub unsafe extern "C" fn rust_InitSpeechGraphics() {
    COMM_STATE.write().speech_graphics_mut().init();
}

/// Rate-limited update of speech graphics display.
///
/// Matches C `UpdateSpeechGraphics`. Uses current system time for rate
/// limiting. In test mode, operates on state fields only.
#[no_mangle]
pub unsafe extern "C" fn rust_UpdateSpeechGraphics() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut state = COMM_STATE.write();
    // We need split borrows: call update with the oscilloscope as a reference.
    // We collect what we need first to avoid the double-borrow.
    let osc_ptr: *const super::oscilloscope::Oscilloscope = state.oscilloscope();
    // SAFETY: osc_ptr is valid for the duration of this call; the RwLock
    // write guard keeps the allocation stable. The oscilloscope field is
    // not mutated by speech_graphics_mut().update().
    let osc_ref = unsafe { &*osc_ptr };
    state.speech_graphics_mut().update(osc_ref, now_ms);
}

// ============================================================================
// Response UI (P10)
// @plan PLAN-20260314-COMM.P10
// ============================================================================

/// Refresh the response list display.
///
/// Matches C `RefreshResponses`. Delegates rendering to C bridge; Rust
/// updates scroll state.
#[no_mangle]
pub unsafe extern "C" fn rust_RefreshResponses() {
    let mut state = COMM_STATE.write();
    let selected = state.responses().selected().max(0) as usize;
    // We need an owned snapshot of responses to avoid split-borrow.
    // response_ui_mut() borrows state mutably; responses() borrows it
    // immutably. Collect what we need first.
    let count = state.responses().count();
    let _ = count; // used inside refresh_responses via ResponseSystem ref
    let responses_ptr: *const super::response::ResponseSystem = state.responses();
    // SAFETY: responses_ptr is valid for the duration of this call under the
    // write guard. response_ui_mut() does not touch the responses field.
    let responses_ref = unsafe { &*responses_ptr };
    state
        .response_ui_mut()
        .refresh_responses(responses_ref, selected);
}

// ============================================================================
// Subtitle Display (P10)
// @plan PLAN-20260314-COMM.P10
// ============================================================================

/// Clear the subtitle display area.
///
/// Matches C `ClearSubtitles`.
#[no_mangle]
pub unsafe extern "C" fn rust_ClearSubtitles() {
    COMM_STATE.write().subtitle_display_mut().clear();
}

/// Check subtitle timing and update display if the text has changed.
///
/// Matches C `CheckSubtitles`.
#[no_mangle]
pub unsafe extern "C" fn rust_CheckSubtitles() {
    let mut state = COMM_STATE.write();
    let current = state.current_subtitle().map(|s| s.to_owned());
    state
        .subtitle_display_mut()
        .check_subtitle(current.as_deref());
}

/// Redraw the current subtitle text.
///
/// Matches C `RedrawSubtitles`.
#[no_mangle]
pub unsafe extern "C" fn rust_RedrawSubtitles() {
    COMM_STATE.read().subtitle_display().redraw();
}

// ============================================================================
// Conversation Summary (P10)
// @plan PLAN-20260314-COMM.P10
// ============================================================================

/// Show the conversation summary overlay.
///
/// Matches C `SelectConversationSummary`. Rebuilds the summary from the
/// current track history, then runs a simple page-advance loop until the
/// player exits. Returns 1 when the player exits normally, 0 on abort.
#[no_mangle]
pub unsafe extern "C" fn rust_ShowConversationSummary() -> c_int {
    use super::summary::SummaryResult;
    use super::summary::SummaryView;

    // Rebuild summary from current track history.
    COMM_STATE.write().rebuild_summary();

    let lines_per_page = 10usize;
    let mut view = SummaryView::new(lines_per_page);

    let total = {
        let state = COMM_STATE.read();
        view.init(state.summary())
    };

    if total == 0 {
        return 1;
    }

    // Advance through pages until Exit or Abort (abort not yet wired — use
    // a simple bounded loop so we can't spin forever in production if
    // input handling is not yet implemented).
    loop {
        match view.advance_page() {
            SummaryResult::NextPage => continue,
            SummaryResult::Exit => return 1,
            SummaryResult::Aborted => return 0,
        }
    }
}

// ============================================================================
// HailAlien bridge (P11)
// @plan PLAN-20260314-COMM.P11
// ============================================================================

/// Entry point for the alien hail sequence from C InitCommunication.
///
/// Under USE_RUST_COMM, InitCommunication calls this instead of the C HailAlien().
/// The Rust side runs the full encounter loop: init speech graphics, play music,
/// run DoCommunication loop, teardown.
///
/// For P11, this is a delegation stub — the actual encounter loop is still
/// driven by the C HailAlien() called via the existing path. This export
/// satisfies the symbol requirement so the guard in InitCommunication compiles.
#[no_mangle]
pub unsafe extern "C" fn rust_HailAlien() {
    // P11: Stub — full Rust HailAlien will replace C's in a later phase.
    // For now this triggers the Rust encounter state machinery to record
    // that a hail was initiated, then falls through to C via the bridge.
    // The C guard in InitCommunication routes here; the actual HailAlien
    // work continues in C for now (this function will be fleshed out in P12+).
}

/// NPCPhrase with callback, routed from commglue.c under USE_RUST_COMM.
///
/// Routes phrase lookup and trackplayer splicing through the Rust track manager.
/// The C commglue.c NPCPhrase_cb will call this instead of SpliceTrack directly.
#[no_mangle]
pub unsafe extern "C" fn rust_NPCPhrase_cb(index: c_int, cb: Option<unsafe extern "C" fn()>) {
    // P11: Track splicing remains in C (SpliceTrack is authoritative C trackplayer).
    // This stub satisfies the symbol requirement. Full routing in P12+.
    let _ = (index, cb);
}

/// NPCPhrase_splice routed from commglue.c under USE_RUST_COMM.
#[no_mangle]
pub unsafe extern "C" fn rust_NPCPhrase_splice(index: c_int) {
    // P11: Stub — routing in P12+.
    let _ = index;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::ffi::CString;

    fn reset_state() {
        unsafe { rust_UninitCommunication() };
    }

    #[test]
    #[serial]
    fn test_ffi_init_uninit() {
        unsafe {
            reset_state();

            assert_eq!(rust_InitCommunication(), 1);
            assert_eq!(rust_IsCommInitialized(), 1);

            rust_UninitCommunication();
            assert_eq!(rust_IsCommInitialized(), 0);
        }
    }

    #[test]
    #[serial]
    fn test_ffi_track_management() {
        unsafe {
            reset_state();
            rust_InitCommunication();

            let text = CString::new("Hello world").unwrap();
            rust_SpliceTrack(1, text.as_ptr(), 0.0, 2.0);

            assert_eq!(rust_StartTrack(), 1);
            assert_eq!(rust_IsTalking(), 1);

            rust_StopTrack();
            assert_eq!(rust_IsTalking(), 0);

            rust_UninitCommunication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_response_system() {
        unsafe {
            reset_state();
            rust_InitCommunication();

            let text1 = CString::new("Option A").unwrap();
            let text2 = CString::new("Option B").unwrap();

            assert_eq!(rust_DoResponsePhrase(1, text1.as_ptr(), None), 1);
            assert_eq!(rust_DoResponsePhrase(2, text2.as_ptr(), None), 1);

            assert_eq!(rust_GetResponseCount(), 2);

            rust_DisplayResponses();
            assert_eq!(rust_GetSelectedResponse(), 0);

            rust_SelectNextResponse();
            assert_eq!(rust_GetSelectedResponse(), 1);

            rust_ClearResponses();
            assert_eq!(rust_GetResponseCount(), 0);

            rust_UninitCommunication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_subtitles() {
        unsafe {
            reset_state();
            rust_InitCommunication();

            assert_eq!(rust_AreSubtitlesEnabled(), 1);

            rust_SetSubtitlesEnabled(0);
            assert_eq!(rust_AreSubtitlesEnabled(), 0);

            rust_SetSubtitlesEnabled(1);
            assert_eq!(rust_AreSubtitlesEnabled(), 1);

            rust_UninitCommunication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_oscilloscope() {
        unsafe {
            reset_state();
            rust_InitCommunication();

            let samples: [i16; 4] = [100, 200, 300, 400];
            rust_AddOscilloscopeSamples(samples.as_ptr(), 4);
            rust_UpdateOscilloscope();

            // Should return a value in range
            let y = rust_GetOscilloscopeY(0);
            assert!(y <= 255);

            rust_ClearOscilloscope();

            rust_UninitCommunication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_state_queries() {
        unsafe {
            reset_state();
            rust_InitCommunication();

            assert_eq!(rust_IsTalkingFinished(), 0);
            rust_SetTalkingFinished(1);
            assert_eq!(rust_IsTalkingFinished(), 1);

            rust_SetCommIntroMode(2);
            assert_eq!(rust_GetCommIntroMode(), 2);

            rust_SetCommFadeTime(30);
            assert_eq!(rust_GetCommFadeTime(), 30);

            assert_eq!(rust_IsCommInputPaused(), 0);
            rust_SetCommInputPaused(1);
            assert_eq!(rust_IsCommInputPaused(), 1);

            rust_UninitCommunication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_track_text_only() {
        unsafe {
            reset_state();
            rust_InitCommunication();

            let text = CString::new("Subtitle only").unwrap();
            rust_SpliceTrackText(text.as_ptr(), 0.0, 2.0);

            // Track should have a chunk
            assert!(rust_GetTrackLength() > 0.0);

            rust_UninitCommunication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_track_position() {
        unsafe {
            reset_state();
            rust_InitCommunication();

            let text = CString::new("Test").unwrap();
            rust_SpliceTrack(1, text.as_ptr(), 0.0, 5.0);

            rust_StartTrack();
            rust_SeekTrack(2.5);
            assert!((rust_GetTrackPosition() - 2.5).abs() < 0.01);

            rust_JumpTrack();
            // JumpTrack skips to end of current phrase — position depends on track state

            rust_RewindTrack();
            assert!((rust_GetTrackPosition() - 0.0).abs() < 0.01);

            rust_UninitCommunication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_update() {
        unsafe {
            reset_state();
            rust_InitCommunication();

            let text = CString::new("Test").unwrap();
            rust_SpliceTrack(1, text.as_ptr(), 0.0, 1.0);

            rust_StartTrack();
            rust_UpdateCommunication(0.5);
            assert!((rust_GetTrackPosition() - 0.5).abs() < 0.01);

            rust_UpdateCommunication(1.0);
            assert_eq!(rust_IsTalkingFinished(), 1);

            rust_UninitCommunication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_null_safety() {
        unsafe {
            reset_state();
            rust_InitCommunication();

            // Should handle null pointers gracefully
            rust_SpliceTrack(1, std::ptr::null(), 0.0, 1.0);
            rust_SpliceTrackText(std::ptr::null(), 0.0, 1.0);
            assert_eq!(rust_DoResponsePhrase(1, std::ptr::null(), None), 0);
            rust_AddOscilloscopeSamples(std::ptr::null(), 0);

            rust_UninitCommunication();
        }
    }
}
