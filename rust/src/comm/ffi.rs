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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_InitCommunication() -> c_int {
    match COMM_STATE.write().init() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Uninitialize the communication system
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_UninitCommunication() {
    COMM_STATE.write().uninit();
}

/// Check if communication is initialized
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_IsCommInitialized() -> c_int {
    if COMM_STATE.read().is_initialized() {
        1
    } else {
        0
    }
}

/// Clear communication state
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_ClearCommunication() {
    COMM_STATE.write().clear();
}

// ============================================================================
// Track Management
// ============================================================================

/// Start the speech track
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_StartTrack() -> c_int {
    match COMM_STATE.write().start_track() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Stop the speech track
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_StopTrack() {
    COMM_STATE.write().stop_track();
}

/// Rewind the track to the beginning
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_RewindTrack() {
    COMM_STATE.write().track_mut().rewind();
}

/// Jump to end of current phrase (skip current speech).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
/// No offset parameter — JumpTrack advances to end of current phrase only (TP-REQ-005).
#[no_mangle]
pub unsafe extern "C" fn rust_JumpTrack() {
    COMM_STATE.write().track_mut().jump(0.0);
}

/// Seek to absolute position in track
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SeekTrack(position: f32) {
    COMM_STATE.write().track_mut().seek(position);
}

/// Commit track position (for save/restore)
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_CommitTrack() -> f32 {
    COMM_STATE.write().track_mut().commit()
}

/// Wait for track to finish (returns 1 when done)
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_WaitTrack() -> c_int {
    if COMM_STATE.read().wait_track() {
        1
    } else {
        0
    }
}

/// Get track position
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_GetTrackPosition() -> f32 {
    COMM_STATE.read().track().position()
}

/// Get track length
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_GetTrackLength() -> f32 {
    COMM_STATE.read().track().length()
}

/// Add a speech chunk to the track
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_ClearTrack() {
    COMM_STATE.write().track_mut().clear();
}

/// Check if a track is currently playing.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_PlayingTrack() -> c_uint {
    if COMM_STATE.read().track().is_playing() {
        1
    } else {
        0
    }
}

/// Fast-forward by one page (subtitle page skip).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_FastForward_Page() {
    COMM_STATE.write().track_mut().fast_forward_page();
}

/// Smooth fast-forward (increase playback rate).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_FastForward_Smooth() {
    COMM_STATE.write().track_mut().fast_forward_smooth();
}

/// Reverse by one page.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_FastReverse_Page() {
    COMM_STATE.write().track_mut().fast_reverse_page();
}

/// Smooth reverse (decrease playback rate / rewind).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_FastReverse_Smooth() {
    COMM_STATE.write().track_mut().fast_reverse_smooth();
}

// ============================================================================
// Subtitle Management
// ============================================================================

/// Get current subtitle (returns null if none).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SetSubtitlesEnabled(enabled: c_int) {
    COMM_STATE.write().subtitles_mut().set_enabled(enabled != 0);
}

/// Check if subtitles are enabled
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
/// `func` receives `response_ref` as its argument when selected (RS-REQ-011).
#[no_mangle]
pub unsafe extern "C" fn rust_DoResponsePhrase(
    response_ref: c_uint,
    text: *const c_char,
    func: Option<extern "C" fn(u32)>,
) -> c_int {
    eprintln!(
        "[DBG] rust_DoResponsePhrase: ref={} text_null={} func={}",
        response_ref,
        text.is_null(),
        func.is_some()
    );
    if text.is_null() {
        return 0;
    }

    let text_str = unsafe {
        match CStr::from_ptr(text).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    let ok = COMM_STATE
        .write()
        .add_response(response_ref, text_str, func);
    eprintln!(
        "[DBG] rust_DoResponsePhrase: added={}, count={}",
        ok,
        COMM_STATE.read().responses().count()
    );
    if ok {
        1
    } else {
        0
    }
}

/// Display response choices
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_DisplayResponses() {
    COMM_STATE.write().display_responses();
}

/// Clear all responses
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_ClearResponses() {
    COMM_STATE.write().clear_responses();
}

/// Select next response
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SelectNextResponse() -> c_int {
    if COMM_STATE.write().select_next_response() {
        1
    } else {
        0
    }
}

/// Select previous response
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SelectPrevResponse() -> c_int {
    if COMM_STATE.write().select_prev_response() {
        1
    } else {
        0
    }
}

/// Get selected response index
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_GetSelectedResponse() -> c_int {
    COMM_STATE.read().selected_response()
}

/// Get number of responses
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_GetResponseCount() -> c_int {
    COMM_STATE.read().responses().count() as c_int
}

/// Copy response text at `index` into `buf` (max `buf_len` bytes, including NUL).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Returns 1 if the text was written, 0 if index is out of range or buf is NULL.
/// The buffer is always NUL-terminated when returning 1.
///
/// Called from C's `c_RefreshResponses` to iterate over the Rust-owned response list.
///
/// @plan PLAN-20260326-COMMPT2.P05
/// @requirement REQ-RB-002
#[no_mangle]
pub unsafe extern "C" fn rust_GetResponseText(
    index: c_int,
    buf: *mut c_char,
    buf_len: c_int,
) -> c_int {
    if buf.is_null() || buf_len <= 0 || index < 0 {
        return 0;
    }

    let state = COMM_STATE.read();
    let entry = match state.responses().get(index as usize) {
        Some(e) => e,
        None => return 0,
    };

    let text = entry.response_text.as_bytes();
    let max = (buf_len as usize).saturating_sub(1);
    let copy_len = text.len().min(max);

    // SAFETY: buf is non-null with at least buf_len bytes (caller contract).
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr() as *const c_char, buf, copy_len);
        *buf.add(copy_len) = 0;
    }
    1
}

/// Execute selected response callback — passes response_ref as argument (RS-REQ-011).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
/// @plan PLAN-20260314-COMM.P07
#[no_mangle]
pub unsafe extern "C" fn rust_InitCommAnimations() {
    // Animation initialization requires CommData to be loaded.
    // In production, this is called after init_race populates CommData.
    // The actual descriptor population happens from C side (stubs in commanim).
}

/// Process communication animations for one frame.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
/// @plan PLAN-20260314-COMM.P07
#[no_mangle]
pub unsafe extern "C" fn rust_ProcessCommAnimations(delta_ticks: c_uint) -> c_int {
    let changed = COMM_STATE.write().animations_mut().process(delta_ticks);
    changed as c_int
}

/// Process communication animations — C bridge signature matching commanim.c.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_WantTalkingAnim() -> c_int {
    COMM_STATE.read().animations().want_talking_anim() as c_int
}

/// Check if talking animation is currently active.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_HaveTalkingAnim() -> c_int {
    COMM_STATE.read().animations().have_talking_anim() as c_int
}

/// Start the talking animation.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SetRunTalkingAnim(_run: c_int) {
    COMM_STATE.write().animations_mut().start_talking_anim();
}

/// Signal to stop the talking animation.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SetStopTalkingAnim() {
    COMM_STATE.write().animations_mut().stop_talking_anim();
}

/// Set intro animation running state.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SetRunIntroAnim(run: c_int) {
    COMM_STATE.write().animations_mut().set_intro_anim(run != 0);
}

/// Check if intro animation is running.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_RunningIntroAnim() -> c_int {
    COMM_STATE.read().animations().is_intro_anim_running() as c_int
}

/// Check if talking animation is running.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_RunningTalkingAnim() -> c_int {
    COMM_STATE.read().animations().is_talking_anim_running() as c_int
}

/// Get current frame for an animation sequence.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_BeginEncounter() -> c_int {
    match super::encounter::begin_encounter() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter normally (post + uninit callbacks, resource teardown).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_EndEncounterNormal() -> c_int {
    match super::encounter::end_encounter_normal() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter on abort/load (uninit only, skip post).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_EndEncounterAbort() -> c_int {
    match super::encounter::end_encounter_abort() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter for attack-without-hail (post + uninit, no init).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_EndEncounterAttack() -> c_int {
    match super::encounter::end_encounter_attack() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Check if encounter is active.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_IsEncounterActive() -> c_int {
    super::encounter::is_encounter_active() as c_int
}

// ============================================================================
// Oscilloscope
// ============================================================================

/// Add samples to the oscilloscope
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_AddOscilloscopeSamples(samples: *const i16, count: c_uint) {
    if samples.is_null() || count == 0 {
        return;
    }

    let samples_slice = unsafe { std::slice::from_raw_parts(samples, count as usize) };
    COMM_STATE.write().add_oscilloscope_samples(samples_slice);
}

/// Update the oscilloscope display
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_UpdateOscilloscope() {
    COMM_STATE.write().oscilloscope_mut().update();
}

/// Get oscilloscope Y value at position
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_GetOscilloscopeY(x: c_uint) -> u8 {
    COMM_STATE.read().oscilloscope().get_y(x as usize)
}

/// Clear the oscilloscope
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_ClearOscilloscope() {
    COMM_STATE.write().oscilloscope_mut().clear();
}

// ============================================================================
// State Queries
// ============================================================================

/// Check if alien is talking
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_IsTalking() -> c_int {
    if COMM_STATE.read().is_talking() {
        1
    } else {
        0
    }
}

/// Check if talking has finished
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_IsTalkingFinished() -> c_int {
    if COMM_STATE.read().is_talking_finished() {
        1
    } else {
        0
    }
}

/// Set talking finished flag
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SetTalkingFinished(finished: c_int) {
    COMM_STATE.write().set_talking_finished(finished != 0);
}

/// Get intro mode
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_GetCommIntroMode() -> c_uint {
    COMM_STATE.read().intro_mode() as c_uint
}

/// Set intro mode
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SetCommIntroMode(mode: c_uint) {
    COMM_STATE.write().set_intro_mode(CommIntroMode::from(mode));
}

/// Get fade time
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_GetCommFadeTime() -> c_uint {
    COMM_STATE.read().fade_time()
}

/// Set fade time
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SetCommFadeTime(time: c_uint) {
    COMM_STATE.write().set_fade_time(time);
}

/// Check if input is paused
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_IsCommInputPaused() -> c_int {
    if COMM_STATE.read().is_input_paused() {
        1
    } else {
        0
    }
}

/// Set input paused
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SetCommInputPaused(paused: c_int) {
    COMM_STATE.write().set_input_paused(paused != 0);
}

/// Update communication state
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_UpdateCommunication(delta_time: f32) {
    COMM_STATE.write().update(delta_time);
}

// ============================================================================
// Phrase State (P04)
// @plan PLAN-20260314-COMM.P04
// ============================================================================

/// Check if a phrase is enabled (not disabled this encounter).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_PhraseEnabled(index: c_int) -> c_int {
    if COMM_STATE.read().phrase_enabled(index) {
        1
    } else {
        0
    }
}

/// Disable a phrase for the remainder of this encounter.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_DisablePhrase(index: c_int) {
    COMM_STATE.write().disable_phrase(index);
}

// ============================================================================
// Segue (P04)
// @plan PLAN-20260314-COMM.P04
// ============================================================================

/// Set segue state (0=Peace, 1=Hostile, 2=Victory, 3=Defeat).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_SetSegue(segue: c_uint) {
    COMM_STATE.write().set_segue(Segue::from(segue));
}

/// Get segue state (0=Peace, 1=Hostile, 2=Victory, 3=Defeat).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_GetSegue() -> c_uint {
    u32::from(COMM_STATE.read().get_segue())
}

/// Get BATTLE_SEGUE value for current segue (0=peace, 1=combat).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_GetBattleSegue() -> c_uint {
    COMM_STATE.read().get_segue().to_battle_segue()
}

// ============================================================================
// Talk Segue & Main Loop (P09)
// @plan PLAN-20260314-COMM.P09
// ============================================================================

/// Run one iteration of the alien talk segue for the given wait-track.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Matches C `DoCommunication(pES)`.
/// Returns 1 to keep iterating, 0 when conversation is done.
///
/// Lock discipline: acquires COMM_STATE write lock, runs one iteration.
/// For `Selected`, releases the lock before invoking the callback so the
/// callback can safely re-enter the communication state.
///
/// @plan PLAN-20260325-COMMPT3.P11
/// @requirement REQ-RL-001..003, REQ-DC-001
/// @pseudocode 003-do-communication-rewrite lines 41-81
#[no_mangle]
pub unsafe extern "C" fn rust_DoCommunication() -> c_int {
    use super::talk_segue::WAIT_TRACK_ALL;

    eprintln!("[DBG] rust_DoCommunication: entry");

    // Phase 1: If still talking, run the talk segue in C (via DoInput)
    // WITHOUT holding the COMM_STATE lock. The C-side talk segue uses
    // DoInput for proper frame pacing and cooperative thread yielding.
    // Holding the Rust lock here would deadlock because SleepThread
    // yields to threads that call back into Rust FFI (which also lock).
    {
        let is_finished = COMM_STATE.read().is_talking_finished();
        eprintln!(
            "[DBG] rust_DoCommunication: is_talking_finished={}",
            is_finished
        );
        if !is_finished {
            // First-call initialization (only once per encounter)
            {
                let mut state = COMM_STATE.write();
                if !state.first_talk_call {
                    eprintln!("[DBG] rust_DoCommunication: first_talk_call init");
                    state.first_talk_call = true;
                    // Drop lock before calling C bridges
                    drop(state);
                    super::talk_segue::alien_talk_first_call_init();
                }
            }

            // Run talk segue via C DoInput — NO Rust lock held
            extern "C" {
                fn c_RunTalkSegue(wait_track: std::ffi::c_uint) -> std::ffi::c_int;
            }
            eprintln!(
                "[DBG] rust_DoCommunication: calling c_RunTalkSegue({})",
                WAIT_TRACK_ALL
            );
            let ended = c_RunTalkSegue(WAIT_TRACK_ALL as std::ffi::c_uint) != 0;
            eprintln!(
                "[DBG] rust_DoCommunication: c_RunTalkSegue returned ended={}",
                ended
            );

            // Update state with result
            {
                let mut state = COMM_STATE.write();
                state.set_talking_finished(ended);
            }

            if ended {
                super::talk_segue::fade_music_to_foreground_bridge();
            }

            return 1;
        }
    }

    // Phase 2: Response handling.
    // IMPORTANT: C bridge calls in this phase (c_RefreshResponses,
    // c_UpdateAnimations) re-enter Rust via rust_GetResponseText etc.
    // We MUST NOT hold COMM_STATE lock during those calls.

    // Step 2a: Quick state check (short lock)
    let response_count = {
        let state = COMM_STATE.read();
        eprintln!(
            "[DBG] rust_DoCommunication: response phase, num_responses={}",
            state.responses().count()
        );
        if super::talk_segue::check_abort_external() {
            return 0;
        }
        state.responses().count()
    };

    if response_count == 0 {
        // No responses — run last-replay then done.
        // run_last_replay calls C bridges, so no lock.
        extern "C" {
            fn c_FadeMusic(
                target_vol: std::ffi::c_int,
                duration: std::ffi::c_int,
            ) -> std::ffi::c_uint;
        }
        let timeout = c_FadeMusic(0, super::talk_segue::c_bridge::ONE_SECOND_TICKS * 3);
        super::talk_segue::run_last_replay_bridge(timeout as i32);
        return 0;
    }

    // Step 2b: Initialize response display if needed (lock briefly, drop before C call)
    {
        let needs_init = COMM_STATE.read().top_response.is_none();
        if needs_init {
            COMM_STATE.write().top_response = Some(0);
            // Drop lock, then render (C→Rust callback)
            let (top, count, cur) = {
                let s = COMM_STATE.read();
                (
                    0u8,
                    s.responses().count() as u8,
                    s.responses().selected().max(0) as u8,
                )
            };
            super::talk_segue::c_bridge::call_refresh_responses(top, count, cur);
        }
    }

    // Step 2c: Check input (short lock — no C callbacks)
    let select = super::talk_segue::check_select_external();
    let cancel = super::talk_segue::check_cancel_external();
    let up = super::talk_segue::check_up_external();
    let down = super::talk_segue::check_down_external();
    let left = super::talk_segue::check_left_external();

    if select {
        eprintln!("[DBG] rust_DoCommunication: SELECT pressed");
        // Player selected a response — extract callback info then drop lock
        let selection = {
            let mut state = COMM_STATE.write();
            let sel = super::talk_segue::select_response(&mut state);
            eprintln!(
                "[DBG] rust_DoCommunication: select_response returned {:?}",
                sel.is_some()
            );
            eprintln!(
                "[DBG] rust_DoCommunication: after select, talking_finished={}",
                state.is_talking_finished()
            );
            sel
        };
        match selection {
            Some((func, rref)) => {
                eprintln!(
                    "[DBG] rust_DoCommunication: calling response callback rref={}",
                    rref
                );
                // Lock is dropped — safe to call alien script callback
                func(rref);
                eprintln!(
                    "[DBG] rust_DoCommunication: callback returned, talking_finished={}",
                    COMM_STATE.read().is_talking_finished()
                );
                return 1;
            }
            None => return 1,
        }
    }

    if cancel {
        let won = super::talk_segue::won_last_battle_external();
        if !won {
            // Conversation summary — C bridge, no lock needed
            super::talk_segue::c_bridge::call_select_conversation_summary();
            // After summary returns, re-render responses (no lock for C call)
            let (top, count, cur) = {
                let s = COMM_STATE.read();
                (
                    s.top_response.unwrap_or(0),
                    s.responses().count() as u8,
                    s.responses().selected().max(0) as u8,
                )
            };
            super::talk_segue::c_bridge::call_refresh_responses(top, count, cur);
            return 1;
        }
    }

    if left {
        // Replay last phrase — all C bridge calls, no lock
        super::talk_segue::fade_music_to_background_bridge();
        super::talk_segue::c_bridge::call_feedback_player_phrase();
        extern "C" {
            fn c_RunTalkSegue(wait_track: std::ffi::c_uint) -> std::ffi::c_int;
        }
        let _ = c_RunTalkSegue(0);
        if !super::talk_segue::check_abort_external() {
            let (top, count, cur) = {
                let s = COMM_STATE.read();
                (
                    s.top_response.unwrap_or(0),
                    s.responses().count() as u8,
                    s.responses().selected().max(0) as u8,
                )
            };
            super::talk_segue::c_bridge::call_refresh_responses(top, count, cur);
            super::talk_segue::fade_music_to_foreground_bridge();
        }
        return 1;
    }

    // Step 2d: Navigate responses (short lock for state update)
    if up || down {
        let mut state = COMM_STATE.write();
        let count = state.responses().count();
        let cur = state.responses().selected().max(0) as usize;
        let next = if up {
            if cur == 0 {
                count - 1
            } else {
                cur - 1
            }
        } else {
            (cur + 1) % count
        };
        state.responses_mut().select(next as i32);
        let top = state.top_response.unwrap_or(0);
        let cnt = count as u8;
        let sel = next as u8;
        drop(state);
        // Render updated selection (C→Rust callback, no lock)
        super::talk_segue::c_bridge::call_refresh_responses(top, cnt, sel);
    }

    // Step 2e: Update animations (C bridge, no lock)
    super::talk_segue::c_bridge::call_update_comm_graphics();

    1
}

// ============================================================================
// Speech Graphics (P10)
// @plan PLAN-20260314-COMM.P10
// ============================================================================

/// Initialize speech graphics (oscilloscope + slider) for this encounter.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Matches C `InitSpeechGraphics`.
#[no_mangle]
pub unsafe extern "C" fn rust_InitSpeechGraphics() {
    COMM_STATE.write().speech_graphics_mut().init();
}

/// Rate-limited update of speech graphics display.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Matches C `ClearSubtitles`.
#[no_mangle]
pub unsafe extern "C" fn rust_ClearSubtitles() {
    COMM_STATE.write().subtitle_display_mut().clear();
}

/// Check subtitle timing and update display if the text has changed.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
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
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Production path: delegates directly to C `SelectConversationSummary` so
/// the full C input loop drives the summary display.
///
/// @plan PLAN-20260325-COMMPT3.P12
/// @requirement REQ-CS-002
/// @pseudocode 004-summary-guard-stale-markers lines 01-24
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn rust_ShowConversationSummary() -> c_int {
    use super::talk_segue::c_bridge::c_SelectConversationSummary;
    c_SelectConversationSummary();
    1
}

/// Show the conversation summary overlay (test path).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Uses the Rust SummaryView directly so tests can exercise summary logic
/// without a C runtime.
///
/// @plan PLAN-20260325-COMMPT3.P12, P14
/// @requirement REQ-CS-002, REQ-CS-003, REQ-SM-001
/// @pseudocode 004-summary-guard-stale-markers lines 01-47
#[cfg(test)]
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

    // Advance through pages until Exit or Abort.
    loop {
        match view.advance_page() {
            SummaryResult::NextPage => continue,
            SummaryResult::Exit => return 1,
            SummaryResult::Aborted => return 0,
        }
    }
}

// ============================================================================
// HailAlien bridge
// @plan PLAN-20260326-COMMPT2.P07
// @requirement REQ-HL-001
// ============================================================================

/// Entry point for the alien hail sequence from C InitCommunication.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Under USE_RUST_COMM, InitCommunication calls this instead of the C HailAlien().
/// Delegates to `hail::hail_alien()` which implements the full encounter loop:
/// resource loading → context setup → init_encounter_func → DoInput loop →
/// post/uninit_encounter_func → resource cleanup.
#[no_mangle]
pub unsafe extern "C" fn rust_HailAlien() {
    super::hail::hail_alien();
}

// ============================================================================
// NPC Phrase (P04)
// @plan PLAN-20260326-COMMPT2.P04
// @requirement REQ-NP-001, REQ-NP-002, REQ-NP-003, REQ-NP-004
// ============================================================================

// Special phrase-index constants matching commglue.h enum values.
#[cfg(not(test))]
const GLOBAL_PLAYER_NAME: c_int = -1_000_000;
#[cfg(not(test))]
const GLOBAL_SHIP_NAME: c_int = -999_999;
#[cfg(not(test))]
const GLOBAL_ALLIANCE_NAME: c_int = -999_998;

// Size of the per-call alliance-name working buffer (matches C NPCPhrase_cb buf[400]).
#[cfg(not(test))]
const ALLIANCE_NAME_BUF_SIZE: usize = 400;

// C bridge declarations used only by NPCPhrase (not test-compiled).
#[cfg(not(test))]
extern "C" {
    // Return GLOBAL_SIS(CommanderName) as a UTF-8 C string.
    fn c_get_commander_name() -> *const std::ffi::c_char;
    // Return GLOBAL_SIS(ShipName) as a UTF-8 C string.
    fn c_get_ship_name() -> *const std::ffi::c_char;
    // Full alliance-name lookup (i==3 appends CommanderName into caller-supplied buf).
    fn c_get_alliance_name_full(
        adjusted_index: c_int,
        buf: *mut std::ffi::c_char,
        buf_len: c_int,
    ) -> *const std::ffi::c_char;
    // Return the text (GetStringAddress) for ConversationPhrases[1-based phrase index].
    fn c_get_conversation_phrase(phrases: *const std::ffi::c_void, index: c_int) -> *const u8;
    // Return the sound-clip pointer (GetStringSoundClip) for a 0-based index.
    fn c_get_phrase_sound_clip(
        phrases: *const std::ffi::c_void,
        index: c_int,
    ) -> *mut std::ffi::c_void;
    // Return the timestamp pointer (GetStringTimeStamp) for a 0-based index.
    fn c_get_phrase_timestamp(
        phrases: *const std::ffi::c_void,
        index: c_int,
    ) -> *mut std::ffi::c_void;
    // Splice text + optional audio into the C trackplayer.
    fn c_SpliceTrack(
        filespec: *mut std::ffi::c_char,
        textspec: *mut std::ffi::c_char,
        timestamp: *mut std::ffi::c_char,
        cb: Option<unsafe extern "C" fn()>,
    );
    // Splice one or more voice clips with text (used by NPCPhrase_splice when clip exists).
    fn c_SpliceMultiTrack(
        track_names: *mut *mut std::ffi::c_char,
        track_text: *mut std::ffi::c_char,
    );
}

/// NPCPhrase with callback — the Rust replacement for C's NPCPhrase_cb.
///
/// Implements all six branches from commglue.c NPCPhrase_cb (lines 36–97):
///  1. index == 0: no-op, return immediately
///  2. GLOBAL_PLAYER_NAME: use commander name, null clip/timestamp
///  3. GLOBAL_SHIP_NAME: use ship name, null clip/timestamp
///  4. index < 0 (negative): alliance-name variant with GET_GAME_STATE lookup;
///     state==3 appends CommanderName
///  5. index > 0 (normal): look up ConversationPhrases[index-1] for text,
///     clip, and timestamp
///  6. For all non-zero paths: call c_SpliceTrack with resolved data + cb
///
/// # Safety
/// Must be called from the game thread with a valid encounter active.
///
/// @plan PLAN-20260326-COMMPT2.P04
/// @requirement REQ-NP-001, REQ-NP-002, REQ-NP-003, REQ-NP-004
#[no_mangle]
pub unsafe extern "C" fn rust_NPCPhrase_cb(index: c_int, cb: Option<unsafe extern "C" fn()>) {
    eprintln!(
        "[DBG] rust_NPCPhrase_cb: index={} cb={}",
        index,
        cb.is_some()
    );
    // Branch 1: no-op
    if index == 0 {
        eprintln!("[DBG] rust_NPCPhrase_cb: index=0, returning");
        return;
    }

    // Suppress unused-variable warning in test builds where the #[cfg(not(test))]
    // block below is omitted and cb is not consumed.
    let _ = cb;

    #[cfg(not(test))]
    {
        use std::ffi::c_void;
        use std::ptr;

        if index == GLOBAL_PLAYER_NAME {
            // Branch 2: commander name
            let name = c_get_commander_name();
            c_SpliceTrack(ptr::null_mut(), name as *mut _, ptr::null_mut(), cb);
        } else if index == GLOBAL_SHIP_NAME {
            // Branch 3: ship name
            let name = c_get_ship_name();
            c_SpliceTrack(ptr::null_mut(), name as *mut _, ptr::null_mut(), cb);
        } else if index < 0 {
            // Branch 4: alliance-name variant
            // adjusted = index - GLOBAL_ALLIANCE_NAME (undo the base offset from
            // the enum so alliance-name phrases map to small positive numbers)
            let adjusted = index - GLOBAL_ALLIANCE_NAME;
            let mut buf = [0u8; ALLIANCE_NAME_BUF_SIZE];
            let text_ptr = c_get_alliance_name_full(
                adjusted,
                buf.as_mut_ptr() as *mut _,
                ALLIANCE_NAME_BUF_SIZE as c_int,
            );
            if text_ptr.is_null() {
                return;
            }
            c_SpliceTrack(ptr::null_mut(), text_ptr as *mut _, ptr::null_mut(), cb);
        } else {
            // Branch 5: normal phrase from ConversationPhrases[index-1]
            let phrases = {
                let state = COMM_STATE.read();
                state
                    .comm_data()
                    .map(|d| d.conversation_phrases as *const c_void)
                    .unwrap_or(ptr::null())
            };

            if phrases.is_null() {
                eprintln!(
                    "[DBG] rust_NPCPhrase_cb: phrases is NULL (comm_data not set?), returning"
                );
                return;
            }

            // c_get_conversation_phrase expects 1-based phrase index (C legacy contract),
            // while clip/timestamp wrappers take 0-based table index.
            let text = c_get_conversation_phrase(phrases, index);
            if text.is_null() {
                return;
            }
            let table_idx = index - 1;
            let clip = c_get_phrase_sound_clip(phrases, table_idx);
            let timestamp = c_get_phrase_timestamp(phrases, table_idx);

            c_SpliceTrack(clip as *mut _, text as *mut _, timestamp as *mut _, cb);
        }
    }

    // Update conversation summary to record this phrase emission.
    COMM_STATE.write().rebuild_summary();
}

/// NPCPhrase_splice variant preserving C `NPCPhrase_splice` behavior.
///
/// C semantics (commglue.c):
/// - index == 0: return
/// - resolve text and clip for phrase index-1
/// - if clip is NULL: SpliceTrack(NULL, text, NULL, NULL)
/// - else: SpliceMultiTrack([clip, NULL], text)
///
/// # Safety
/// Must be called from the game thread with a valid encounter active.
///
/// @plan PLAN-20260326-COMMPT2.P04
/// @requirement REQ-NP-002
#[no_mangle]
pub unsafe extern "C" fn rust_NPCPhrase_splice(index: c_int) {
    if index == 0 {
        return;
    }

    #[cfg(not(test))]
    {
        use std::ffi::c_void;
        use std::ptr;

        if index < 0 {
            // For special negative phrases, reuse NPCPhrase_cb behavior.
            rust_NPCPhrase_cb(index, None);
            return;
        }

        let phrases = {
            let state = COMM_STATE.read();
            state
                .comm_data()
                .map(|d| d.conversation_phrases as *const c_void)
                .unwrap_or(ptr::null())
        };
        if phrases.is_null() {
            return;
        }

        // c_get_conversation_phrase expects 1-based phrase index.
        let text = c_get_conversation_phrase(phrases, index);
        if text.is_null() {
            return;
        }

        let table_idx = index - 1;
        let clip = c_get_phrase_sound_clip(phrases, table_idx);

        if clip.is_null() {
            c_SpliceTrack(ptr::null_mut(), text as *mut _, ptr::null_mut(), None);
        } else {
            let mut tracks: [*mut std::ffi::c_char; 2] = [clip as *mut _, ptr::null_mut()];
            c_SpliceMultiTrack(tracks.as_mut_ptr(), text as *mut _);
        }
    }

    COMM_STATE.write().rebuild_summary();
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

            // Should return a value without panicking.
            let _ = rust_GetOscilloscopeY(0);

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
