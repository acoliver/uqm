//! Rust communication facade for state, playback and dialogue orchestration.

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
pub fn rust_init_communication() -> c_int {
    match COMM_STATE.write().init() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Uninitialize the communication system
pub fn rust_uninit_communication() {
    COMM_STATE.write().uninit();
}

/// Check if communication is initialized
pub fn rust_is_comm_initialized() -> c_int {
    if COMM_STATE.read().is_initialized() {
        1
    } else {
        0
    }
}

/// Clear communication state
pub fn rust_clear_communication() {
    COMM_STATE.write().clear();
}

// ============================================================================
// Track Management
// ============================================================================

/// Start the speech track
pub fn rust_start_track() -> c_int {
    match COMM_STATE.write().start_track() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Stop the speech track
pub fn rust_stop_track() {
    COMM_STATE.write().stop_track();
}

/// Rewind the track to the beginning
pub fn rust_rewind_track() {
    COMM_STATE.write().track_mut().rewind();
}

/// Jump to end of current phrase (skip current speech).
/// No offset parameter — JumpTrack advances to end of current phrase only (TP-REQ-005).
pub fn rust_jump_track() {
    COMM_STATE.write().track_mut().jump(0.0);
}

/// Seek to absolute position in track
pub fn rust_seek_track(position: f32) {
    COMM_STATE.write().track_mut().seek(position);
}

/// Commit track position (for save/restore)
pub fn rust_commit_track() -> f32 {
    COMM_STATE.write().track_mut().commit()
}

/// Wait for track to finish (returns 1 when done)
pub fn rust_wait_track() -> c_int {
    if COMM_STATE.read().wait_track() {
        1
    } else {
        0
    }
}

/// Get track position
pub fn rust_get_track_position() -> f32 {
    COMM_STATE.read().track().position()
}

/// Get track length
pub fn rust_get_track_length() -> f32 {
    COMM_STATE.read().track().length()
}

/// Add a speech chunk to the track
/// # Safety
///
/// `text` must be null or point to a valid NUL-terminated string.
pub unsafe fn rust_splice_track(
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
/// `text` must be null or point to a valid NUL-terminated string.
pub unsafe fn rust_splice_track_text(text: *const c_char, start_time: f32, duration: f32) {
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
pub fn rust_clear_track() {
    COMM_STATE.write().track_mut().clear();
}

/// Check if a track is currently playing.
pub fn rust_playing_track() -> c_uint {
    if COMM_STATE.read().track().is_playing() {
        1
    } else {
        0
    }
}

/// Fast-forward by one page (subtitle page skip).
pub fn rust_fast_forward_page() {
    COMM_STATE.write().track_mut().fast_forward_page();
}

/// Smooth fast-forward (increase playback rate).
pub fn rust_fast_forward_smooth() {
    COMM_STATE.write().track_mut().fast_forward_smooth();
}

/// Reverse by one page.
pub fn rust_fast_reverse_page() {
    COMM_STATE.write().track_mut().fast_reverse_page();
}

/// Smooth reverse (decrease playback rate / rewind).
pub fn rust_fast_reverse_smooth() {
    COMM_STATE.write().track_mut().fast_reverse_smooth();
}

// ============================================================================
// Subtitle Management
// ============================================================================

/// Get current subtitle (returns null if none).
/// Returns a stable C string via thread-local buffer (OL-REQ-009, OL-REQ-010).
/// Pointer is valid until the next call to rust_get_subtitle on the same thread.
pub fn rust_get_subtitle() -> *const c_char {
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
pub fn rust_set_subtitles_enabled(enabled: c_int) {
    COMM_STATE.write().subtitles_mut().set_enabled(enabled != 0);
}

/// Check if subtitles are enabled
pub fn rust_are_subtitles_enabled() -> c_int {
    if COMM_STATE.read().subtitles().is_enabled() {
        1
    } else {
        0
    }
}

// ============================================================================
// Response System
// ============================================================================

/// Add a response option, rejecting absent or non-UTF-8 resource text.
/// `func` receives `response_ref` when the response is selected.
pub fn do_response_phrase(
    response_ref: c_uint,
    text: Option<&CStr>,
    func: Option<super::response::ResponseFunc>,
) -> c_int {
    eprintln!(
        "[DBG] rust_DoResponsePhrase: ref={} text_null={} func={}",
        response_ref,
        text.is_none(),
        func.is_some()
    );
    let Some(text) = text else {
        return 0;
    };
    let Ok(text_str) = text.to_str() else {
        return 0;
    };

    let ok = COMM_STATE
        .write()
        .add_response(response_ref, text_str, func);
    let response_count = COMM_STATE.read().responses().count();
    if ok {
        crate::automation::ui_observation::observe_communication_responses(response_count);
    }
    eprintln!(
        "[DBG] rust_DoResponsePhrase: added={}, count={}",
        ok, response_count
    );
    if ok {
        1
    } else {
        0
    }
}

/// Display response choices
pub fn rust_display_responses() {
    COMM_STATE.write().display_responses();
}

/// Clear all responses
pub fn rust_clear_responses() {
    COMM_STATE.write().clear_responses();
    crate::automation::ui_observation::observe_communication_responses(0);
    crate::automation::ui_observation::set_communication_responses_ready(false);
}

/// Select next response
pub fn rust_select_next_response() -> c_int {
    if COMM_STATE.write().select_next_response() {
        1
    } else {
        0
    }
}

/// Select previous response
pub fn rust_select_prev_response() -> c_int {
    if COMM_STATE.write().select_prev_response() {
        1
    } else {
        0
    }
}

/// Get selected response index
pub fn rust_get_selected_response() -> c_int {
    COMM_STATE.read().selected_response()
}

/// Select an exact response index for semantic automation.
///
/// Returns 1 only when the index is valid for the current response list.
pub fn rust_select_response_index(index: c_int) -> c_int {
    if COMM_STATE.write().responses_mut().select(index) {
        1
    } else {
        0
    }
}

/// Resolve a phrase name against the offered list on the game thread.
pub fn offered_response_index(phrase: &str) -> Result<usize, String> {
    #[cfg(not(test))]
    {
        use crate::strings::string_table::{
            GetStringName, GetStringTableCount, SetAbsStringTableIndex,
        };
        // The current conversation owns this table until communication teardown.
        let table = unsafe { super::locdata::COMM_DATA.conversation_phrases.cast() };
        COMM_STATE
            .read()
            .responses()
            .offered_index(phrase, |reference| unsafe {
                if reference == 0 || reference > u32::from(GetStringTableCount(table)) {
                    return None;
                }
                let entry = SetAbsStringTableIndex(table, (reference - 1) as u16);
                let name = GetStringName(entry);
                (!name.is_null()).then(|| CStr::from_ptr(name).to_string_lossy().into_owned())
            })
    }
    #[cfg(test)]
    {
        Err(format!("response_not_offered:phrase={phrase}"))
    }
}

/// Get number of responses
pub fn rust_get_response_count() -> c_int {
    COMM_STATE.read().responses().count() as c_int
}

/// Copy response text at `index` into `buf` (max `buf_len` bytes, including NUL).
/// # Safety
///
/// A non-null `buf` must be writable for `buf_len` bytes when `buf_len` is positive.
///
/// Returns 1 if the text was written, 0 if index is out of range or buf is NULL.
/// The buffer is always NUL-terminated when returning 1.
///
/// Called from C's `c_RefreshResponses` to iterate over the Rust-owned response list.
///
/// @plan PLAN-20260326-COMMPT2.P05
/// @requirement REQ-RB-002
pub unsafe fn rust_get_response_text(index: c_int, buf: *mut c_char, buf_len: c_int) -> c_int {
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
pub fn rust_execute_response() -> c_uint {
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
pub fn rust_init_comm_animations() {
    // Animation initialization requires CommData to be loaded.
    // In production, this is called after init_race populates CommData.
    // The actual descriptor population happens from C side (stubs in commanim).
}

/// Process communication animations for one frame.
/// @plan PLAN-20260314-COMM.P07
pub fn rust_process_comm_animations(delta_ticks: c_uint) -> c_int {
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
pub fn rust_process_comm_animations_cb(clear: c_int, paused: c_int) -> c_int {
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
pub fn rust_want_talking_anim() -> c_int {
    COMM_STATE.read().animations().want_talking_anim() as c_int
}

/// Check if talking animation is currently active.
pub fn rust_have_talking_anim() -> c_int {
    COMM_STATE.read().animations().have_talking_anim() as c_int
}

/// Start the talking animation.
pub fn rust_set_run_talking_anim(_run: c_int) {
    COMM_STATE.write().animations_mut().start_talking_anim();
}

/// Signal to stop the talking animation.
pub fn rust_set_stop_talking_anim() {
    COMM_STATE.write().animations_mut().stop_talking_anim();
}

/// Set intro animation running state.
pub fn rust_set_run_intro_anim(run: c_int) {
    COMM_STATE.write().animations_mut().set_intro_anim(run != 0);
}

/// Check if intro animation is running.
pub fn rust_running_intro_anim() -> c_int {
    COMM_STATE.read().animations().is_intro_anim_running() as c_int
}

/// Check if talking animation is running.
pub fn rust_running_talking_anim() -> c_int {
    COMM_STATE.read().animations().is_talking_anim_running() as c_int
}

/// Get current frame for an animation sequence.
pub fn rust_get_comm_animation_frame(index: c_uint) -> c_uint {
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
pub fn rust_begin_encounter() -> c_int {
    match super::encounter::begin_encounter() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter normally (post + uninit callbacks, resource teardown).
pub fn rust_end_encounter_normal() -> c_int {
    match super::encounter::end_encounter_normal() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter on abort/load (uninit only, skip post).
pub fn rust_end_encounter_abort() -> c_int {
    match super::encounter::end_encounter_abort() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter for attack-without-hail (post + uninit, no init).
pub fn rust_end_encounter_attack() -> c_int {
    match super::encounter::end_encounter_attack() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Check if encounter is active.
pub fn rust_is_encounter_active() -> c_int {
    super::encounter::is_encounter_active() as c_int
}

// ============================================================================
// Oscilloscope
// ============================================================================

/// Add samples to the oscilloscope
/// # Safety
///
/// A non-null `samples` must be readable for `count` samples.
pub unsafe fn rust_add_oscilloscope_samples(samples: *const i16, count: c_uint) {
    if samples.is_null() || count == 0 {
        return;
    }

    let samples_slice = unsafe { std::slice::from_raw_parts(samples, count as usize) };
    COMM_STATE.write().add_oscilloscope_samples(samples_slice);
}

/// Update the oscilloscope display
pub fn rust_update_oscilloscope() {
    COMM_STATE.write().oscilloscope_mut().update();
}

/// Get oscilloscope Y value at position
pub fn rust_get_oscilloscope_y(x: c_uint) -> u8 {
    COMM_STATE.read().oscilloscope().get_y(x as usize)
}

/// Clear the oscilloscope
pub fn rust_clear_oscilloscope() {
    COMM_STATE.write().oscilloscope_mut().clear();
}

// ============================================================================
// State Queries
// ============================================================================

/// Check if alien is talking
pub fn rust_is_talking() -> c_int {
    if COMM_STATE.read().is_talking() {
        1
    } else {
        0
    }
}

/// Check if talking has finished
pub fn rust_is_talking_finished() -> c_int {
    if COMM_STATE.read().is_talking_finished() {
        1
    } else {
        0
    }
}

/// Set talking finished flag
pub fn rust_set_talking_finished(finished: c_int) {
    COMM_STATE.write().set_talking_finished(finished != 0);
}

/// Get intro mode
pub fn rust_get_comm_intro_mode() -> c_uint {
    COMM_STATE.read().intro_mode() as c_uint
}

/// Set intro mode
pub fn rust_set_comm_intro_mode(mode: c_uint) {
    COMM_STATE.write().set_intro_mode(CommIntroMode::from(mode));
}

/// C `SetCommIntroMode (mode, howLong)` (comm.c:1357): the mode and fade time
/// that the talk segue's intro transition reads. Callers that run in Rust set
/// them here, not through the C function, whose statics Rust never reads.
pub fn set_comm_intro_mode(mode: u32, how_long: u32) {
    let mut state = COMM_STATE.write();
    state.set_intro_mode(CommIntroMode::from(mode));
    state.set_fade_time(how_long);
}

/// Get fade time
pub fn rust_get_comm_fade_time() -> c_uint {
    COMM_STATE.read().fade_time()
}

/// Set fade time
pub fn rust_set_comm_fade_time(time: c_uint) {
    COMM_STATE.write().set_fade_time(time);
}

/// Check if input is paused
pub fn rust_is_comm_input_paused() -> c_int {
    if COMM_STATE.read().is_input_paused() {
        1
    } else {
        0
    }
}

/// Set input paused
pub fn rust_set_comm_input_paused(paused: c_int) {
    COMM_STATE.write().set_input_paused(paused != 0);
}

/// Update communication state
pub fn rust_update_communication(delta_time: f32) {
    COMM_STATE.write().update(delta_time);
}

// ============================================================================
// Phrase State (P04)
// @plan PLAN-20260314-COMM.P04
// ============================================================================

/// Check if a phrase is enabled (not disabled this encounter).
pub fn rust_phrase_enabled(index: c_int) -> c_int {
    if COMM_STATE.read().phrase_enabled(index) {
        1
    } else {
        0
    }
}

/// Disable a phrase for the remainder of this encounter.
pub fn rust_disable_phrase(index: c_int) {
    COMM_STATE.write().disable_phrase(index);
}

// ============================================================================
// Segue (P04)
// @plan PLAN-20260314-COMM.P04
// ============================================================================

/// Set segue state (0=Peace, 1=Hostile, 2=Victory, 3=Defeat).
pub fn rust_set_segue(segue: c_uint) {
    COMM_STATE.write().set_segue(Segue::from(segue));
}

/// Get segue state (0=Peace, 1=Hostile, 2=Victory, 3=Defeat).
pub fn rust_get_segue() -> c_uint {
    u32::from(COMM_STATE.read().get_segue())
}

/// Get BATTLE_SEGUE value for current segue (0=peace, 1=combat).
pub fn rust_get_battle_segue() -> c_uint {
    COMM_STATE.read().get_segue().to_battle_segue()
}

// ============================================================================
// Talk Segue & Main Loop (P09)
// @plan PLAN-20260314-COMM.P09
// ============================================================================

/// Run one iteration of the alien talk segue for the given wait-track.
/// # Safety
///
/// Must run on the game thread with an active communication context.
///
/// Matches C `AlienTalkSegue(wait_track)`.
/// Returns 1 if talking finished (reached end), 0 otherwise.
pub unsafe fn rust_alien_talk_segue(wait: c_uint) -> c_int {
    #[cfg(not(test))]
    {
        run_alien_talk_unlocked(
            wait,
            || super::talk_segue::alien_talk_first_call_init(),
            super::talk_segue::dinput::run_talk_segue_dinput,
            || super::talk_segue::fade_music_to_foreground_bridge(),
        )
    }
    #[cfg(test)]
    {
        let mut state = COMM_STATE.write();
        let was_finished = state.is_talking_finished();
        super::talk_segue::alien_talk_segue(&mut state, wait);
        if !was_finished && state.is_talking_finished() {
            1
        } else {
            0
        }
    }
}

fn run_alien_talk_unlocked(
    wait: u32,
    initialize: impl FnOnce(),
    playback: impl FnOnce(u32) -> bool,
    fade: impl FnOnce(),
) -> c_int {
    let initialize_needed = {
        let mut state = COMM_STATE.write();
        if super::talk_segue::check_abort_external() || state.is_talking_finished() {
            return 0;
        }
        let needed = !state.first_talk_call;
        state.first_talk_call = true;
        needed
    };
    if initialize_needed {
        initialize();
    }
    let finished = playback(wait);
    COMM_STATE.write().set_talking_finished(finished);
    if finished {
        fade();
    }
    i32::from(finished)
}

/// Run the full talk segue loop for the given wait-track.
/// # Safety
///
/// Must run on the game thread with an active communication context.
///
/// Matches C `TalkSegue(wait_track)`.
/// Returns 1 if playback ended naturally, 0 if aborted.
pub unsafe fn rust_talk_segue(wait: c_int) -> c_int {
    let wait_track = if wait <= 0 { 0 } else { wait as u32 };
    #[cfg(not(test))]
    {
        super::talk_segue::dinput::run_talk_segue_dinput(wait_track) as c_int
    }
    #[cfg(test)]
    {
        super::talk_segue::talk_segue(&mut COMM_STATE.write(), wait_track) as c_int
    }
}

/// Run one iteration of the top-level communication state machine.
/// # Safety
///
/// Must run on the game thread with an active communication context.
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
pub unsafe fn rust_do_communication() -> c_int {
    #[cfg(not(test))]
    use super::talk_segue::WAIT_TRACK_ALL;

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
                    state.first_talk_call = true;
                    // Drop lock before calling C bridges
                    drop(state);
                    super::talk_segue::alien_talk_first_call_init();
                }
            }

            // Responses may already have been generated by the callback, but
            // they are not selectable until this speech segment finishes.
            crate::automation::ui_observation::set_communication_responses_ready(false);

            // Advance the active wait and inspect the resulting action as one
            // coordinator operation, avoiding an ambiguous split boundary.
            let response_count = COMM_STATE.read().responses().count();
            let semantic_skip = crate::automation::coordinator::Coordinator::service_and_should_skip_pending_communication_speech()
                || crate::automation::coordinator::Coordinator::pending_communication_response_action(response_count)
                || crate::automation::coordinator::Coordinator::should_skip_communication_closing_speech();
            if semantic_skip {
                let _ = crate::sound::trackplayer::jump_track(0);
            }

            // Run talk segue via Rust DoInput — NO Rust lock held
            #[cfg(not(test))]
            let ended = if semantic_skip {
                true
            } else {
                super::talk_segue::dinput::run_talk_segue_dinput(WAIT_TRACK_ALL)
            };
            #[cfg(test)]
            let ended = true;

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
    // c_UpdateAnimations) re-enter Rust via rust_get_response_text etc.
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

    crate::automation::ui_observation::observe_communication_responses(response_count);
    crate::automation::ui_observation::set_communication_responses_ready(true);

    if response_count == 0 {
        // No responses — run last-replay then done.
        // run_last_replay calls C bridges, so no lock.
        use crate::sound::heart_ffi::fade_music as FadeMusic;
        let timeout = FadeMusic(
            0u8,
            (super::talk_segue::c_bridge::ONE_SECOND_TICKS * 3) as i16,
        );
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
        #[cfg(not(test))]
        let _ = super::talk_segue::dinput::run_talk_segue_dinput(0);
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
///
/// Matches C `InitSpeechGraphics`.
pub fn rust_init_speech_graphics() {
    COMM_STATE.write().speech_graphics_mut().init();
}

/// Rate-limited update of speech graphics display.
///
/// Matches C `UpdateSpeechGraphics`. Uses current system time for rate
/// limiting. In test mode, operates on state fields only.
pub fn rust_update_speech_graphics() {
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
pub fn rust_refresh_responses() {
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
pub fn rust_clear_subtitles() {
    COMM_STATE.write().subtitle_display_mut().clear();
}

/// Check subtitle timing and update display if the text has changed.
///
/// Matches C `CheckSubtitles`.
pub fn rust_check_subtitles() {
    let mut state = COMM_STATE.write();
    let current = state.current_subtitle().map(|s| s.to_owned());
    state
        .subtitle_display_mut()
        .check_subtitle(current.as_deref());
}

/// Redraw the current subtitle text.
///
/// Matches C `RedrawSubtitles`.
pub fn rust_redraw_subtitles() {
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
pub unsafe fn rust_show_conversation_summary() -> c_int {
    super::response_ui::select_conversation_summary_production();
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
pub unsafe fn rust_show_conversation_summary() -> c_int {
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
// hail_alien bridge
// @plan PLAN-20260326-COMMPT2.P07
// @requirement REQ-HL-001
// ============================================================================

/// Entry point for the alien hail sequence from C InitCommunication.
/// # Safety
///
/// Must run on the game thread with an active communication context.
///
/// Under USE_RUST_COMM, InitCommunication calls this instead of the C hail_alien().
/// Delegates to `hail::hail_alien()` which implements the full encounter loop:
/// resource loading → context setup → init_encounter_func → DoInput loop →
/// post/uninit_encounter_func → resource cleanup.
pub unsafe fn rust_hail_alien() {
    super::hail::hail_alien();
}

// ============================================================================
// NPC Phrase (P04)
// @plan PLAN-20260326-COMMPT2.P04
// @requirement REQ-NP-001, REQ-NP-002, REQ-NP-003, REQ-NP-004
// ============================================================================

#[cfg(not(test))]
use super::locdata::COMM_DATA as CommData;

// Special phrase-index constants matching commglue.h enum values.
#[cfg(not(test))]
const GLOBAL_PLAYER_NAME: c_int = -1_000_000;
#[cfg(not(test))]
const GLOBAL_SHIP_NAME: c_int = -999_999;
#[cfg(not(test))]
const GLOBAL_ALLIANCE_NAME: c_int = -999_998;

// Size of the per-call alliance-name working buffer (matches C npcphrase_cb buf[400]).
#[cfg(not(test))]
const ALLIANCE_NAME_BUF_SIZE: usize = 400;

// C bridge declarations used only by NPCPhrase (not test-compiled).
#[cfg(not(test))]
use crate::sound::heart_ffi::splice_track as SpliceTrack;
#[cfg(not(test))]
use crate::strings::string_table::GetStringAddress;
#[cfg(not(test))]
use crate::strings::string_table::GetStringSoundClip;
#[cfg(not(test))]
use crate::strings::string_table::GetStringTimeStamp;
#[cfg(not(test))]
use crate::strings::string_table::SetAbsStringTableIndex;
#[cfg(not(test))]
extern "C" {
    pub static mut GlobData: crate::ships::native::abi::GLOBDATA;
}

// Splice one or more voice clips with text (used by npcphrase_splice when clip exists).
#[cfg(not(test))]
use crate::sound::heart_ffi::splice_multi_track as SpliceMultiTrack;

/// Ported from C's c_get_alliance_name: returns alliance name phrase text.
/// Uses CommData.ConversationPhrases + GET_GAME_STATE(NEW_ALLIANCE_NAME).
#[cfg(not(test))]
pub(crate) unsafe fn get_alliance_name(index: c_int) -> *const u8 {
    let i = crate::state::game_state_keys::get_game_state("NEW_ALLIANCE_NAME") as c_int;
    let phrases = std::ptr::addr_of_mut!(CommData.conversation_phrases) as *mut std::ffi::c_void;
    if phrases.is_null() {
        return std::ptr::null();
    }
    let s = SetAbsStringTableIndex((phrases).cast(), (index + i) as u16);
    GetStringAddress(s).cast::<u8>()
}

/// Ported from C's c_get_alliance_name_full: full alliance name with CommanderName append.
#[cfg(not(test))]
unsafe fn get_alliance_name_full(
    adjusted_index: c_int,
    buf: *mut std::ffi::c_char,
    buf_len: c_int,
) -> *const std::ffi::c_char {
    use std::ffi::CStr;

    let phrases = std::ptr::addr_of_mut!(CommData.conversation_phrases) as *mut std::ffi::c_void;
    if phrases.is_null() || buf.is_null() || buf_len <= 0 {
        return std::ptr::null();
    }

    let i = crate::state::game_state_keys::get_game_state("NEW_ALLIANCE_NAME") as c_int;
    let s = SetAbsStringTableIndex((phrases).cast(), ((adjusted_index - 1) + i) as u16);
    let src = GetStringAddress(s);
    if src.is_null() {
        return std::ptr::null();
    }

    // Copy src into buf (strncpy equivalent)
    let src_cstr = CStr::from_ptr(src);
    let src_bytes = src_cstr.to_bytes();
    let copy_len = (buf_len as usize - 1).min(src_bytes.len());
    std::ptr::copy_nonoverlapping(src_bytes.as_ptr(), buf.cast::<u8>(), copy_len);
    *buf.add(copy_len) = 0;

    // If state==3, append CommanderName
    if i == 3 {
        let cname_ptr =
            std::ptr::addr_of_mut!(GlobData.SIS_state.CommanderName) as *const std::ffi::c_char;
        if !cname_ptr.is_null() {
            let cname = CStr::from_ptr(cname_ptr);
            let cname_bytes = cname.to_bytes();
            let used = libc_strlen(buf);
            let remaining = (buf_len as usize).saturating_sub(used + 1);
            let append_len = remaining.min(cname_bytes.len());
            if append_len > 0 {
                std::ptr::copy_nonoverlapping(
                    cname_bytes.as_ptr(),
                    buf.add(used).cast::<u8>(),
                    append_len,
                );
                *buf.add(used + append_len) = 0;
            }
        }
    }

    buf as *const std::ffi::c_char
}

/// Minimal strlen for C strings (avoids linking libc's strlen).
#[cfg(not(test))]
unsafe fn libc_strlen(s: *const std::ffi::c_char) -> usize {
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}

/// NPCPhrase with callback — the Rust replacement for C's npcphrase_cb.
///
/// Implements all six branches from commglue.c npcphrase_cb (lines 36–97):
///  1. index == 0: no-op, return immediately
///  2. GLOBAL_PLAYER_NAME: use commander name, null clip/timestamp
///  3. GLOBAL_SHIP_NAME: use ship name, null clip/timestamp
///  4. index < 0 (negative): alliance-name variant with GET_GAME_STATE lookup;
///     state==3 appends CommanderName
///  5. index > 0 (normal): look up ConversationPhrases[index-1] for text,
///     clip, and timestamp
///  6. For all non-zero paths: call SpliceTrack with resolved data + cb
///
/// # Safety
/// Must be called from the game thread with a valid encounter active.
///
/// @plan PLAN-20260326-COMMPT2.P04
/// @requirement REQ-NP-001, REQ-NP-002, REQ-NP-003, REQ-NP-004
pub unsafe fn rust_npc_phrase_cb(index: c_int, cb: super::glue::PhraseCallback) {
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
            // Branch 2: commander name — direct GlobData.SIS_state access
            let name =
                std::ptr::addr_of_mut!(GlobData.SIS_state.CommanderName) as *mut std::ffi::c_char;
            SpliceTrack(ptr::null_mut(), name, ptr::null_mut(), cb);
        } else if index == GLOBAL_SHIP_NAME {
            // Branch 3: ship name — direct GlobData.SIS_state access
            let name = std::ptr::addr_of_mut!(GlobData.SIS_state.ShipName) as *mut std::ffi::c_char;
            SpliceTrack(ptr::null_mut(), name, ptr::null_mut(), cb);
        } else if index < 0 {
            // Branch 4: alliance-name variant
            let adjusted = index - GLOBAL_ALLIANCE_NAME;
            let mut buf = [0u8; ALLIANCE_NAME_BUF_SIZE];
            let text_ptr = get_alliance_name_full(
                adjusted,
                buf.as_mut_ptr() as *mut _,
                ALLIANCE_NAME_BUF_SIZE as c_int,
            );
            if text_ptr.is_null() {
                return;
            }
            SpliceTrack(ptr::null_mut(), text_ptr as *mut _, ptr::null_mut(), cb);
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

            // Direct SetAbsStringTableIndex + GetStringAddress (1-based phrase index)
            let s = SetAbsStringTableIndex(phrases as *mut _, (index - 1) as u16);
            let text = GetStringAddress(s);
            if text.is_null() {
                return;
            }
            let table_idx = (index - 1) as u16;
            let s2 = SetAbsStringTableIndex(phrases as *mut _, table_idx);
            let clip = GetStringSoundClip(s2);
            let s3 = SetAbsStringTableIndex(phrases as *mut _, table_idx);
            let timestamp = GetStringTimeStamp(s3);

            SpliceTrack(clip as *mut _, text as *mut _, timestamp as *mut _, cb);
        }
    }

    // Update conversation summary to record this phrase emission.
    COMM_STATE.write().rebuild_summary();
}

/// npcphrase_splice variant preserving C `npcphrase_splice` behavior.
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
pub unsafe fn rust_npc_phrase_splice(index: c_int) {
    if index == 0 {
        return;
    }

    #[cfg(not(test))]
    {
        use std::ffi::c_void;
        use std::ptr;

        if index < 0 {
            // For special negative phrases, reuse npcphrase_cb behavior.
            rust_npc_phrase_cb(index, None);
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

        // Direct SetAbsStringTableIndex + GetStringAddress (1-based phrase index)
        let s = SetAbsStringTableIndex(phrases as *mut _, (index - 1) as u16);
        let text = GetStringAddress(s);
        if text.is_null() {
            return;
        }

        let table_idx = (index - 1) as u16;
        let s2 = SetAbsStringTableIndex(phrases as *mut _, table_idx);
        let clip = GetStringSoundClip(s2);

        if clip.is_null() {
            SpliceTrack(ptr::null_mut(), text as *mut _, ptr::null_mut(), None);
        } else {
            let tracks: [*const std::ffi::c_char; 2] = [clip.cast(), ptr::null()];
            SpliceMultiTrack(tracks.as_ptr(), text);
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
        rust_uninit_communication();
    }

    #[test]
    #[serial]
    fn test_ffi_init_uninit() {
        reset_state();

        assert_eq!(rust_init_communication(), 1);
        assert_eq!(rust_is_comm_initialized(), 1);

        rust_uninit_communication();
        assert_eq!(rust_is_comm_initialized(), 0);
    }

    #[test]
    #[serial]
    fn test_ffi_track_management() {
        unsafe {
            reset_state();
            rust_init_communication();

            let text = CString::new("Hello world").unwrap();
            rust_splice_track(1, text.as_ptr(), 0.0, 2.0);

            assert_eq!(rust_start_track(), 1);
            assert_eq!(rust_is_talking(), 1);

            rust_stop_track();
            assert_eq!(rust_is_talking(), 0);

            rust_uninit_communication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_response_system() {
        reset_state();
        rust_init_communication();

        let text1 = CString::new("Option A").unwrap();
        let text2 = CString::new("Option B").unwrap();

        assert_eq!(do_response_phrase(1, Some(text1.as_c_str()), None), 1);
        assert_eq!(do_response_phrase(2, Some(text2.as_c_str()), None), 1);

        assert_eq!(rust_get_response_count(), 2);

        rust_display_responses();
        assert_eq!(rust_get_selected_response(), 0);

        rust_select_next_response();
        assert_eq!(rust_get_selected_response(), 1);

        rust_clear_responses();
        assert_eq!(rust_get_response_count(), 0);

        rust_uninit_communication();
    }

    #[test]
    #[serial]
    fn test_ffi_subtitles() {
        reset_state();
        rust_init_communication();

        assert_eq!(rust_are_subtitles_enabled(), 1);

        rust_set_subtitles_enabled(0);
        assert_eq!(rust_are_subtitles_enabled(), 0);

        rust_set_subtitles_enabled(1);
        assert_eq!(rust_are_subtitles_enabled(), 1);

        rust_uninit_communication();
    }

    #[test]
    #[serial]
    fn test_ffi_oscilloscope() {
        unsafe {
            reset_state();
            rust_init_communication();

            let samples: [i16; 4] = [100, 200, 300, 400];
            rust_add_oscilloscope_samples(samples.as_ptr(), 4);
            rust_update_oscilloscope();

            // Should return a value without panicking.
            let _ = rust_get_oscilloscope_y(0);

            rust_clear_oscilloscope();

            rust_uninit_communication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_state_queries() {
        reset_state();
        rust_init_communication();

        assert_eq!(rust_is_talking_finished(), 0);
        rust_set_talking_finished(1);
        assert_eq!(rust_is_talking_finished(), 1);

        rust_set_comm_intro_mode(2);
        assert_eq!(rust_get_comm_intro_mode(), 2);

        rust_set_comm_fade_time(30);
        assert_eq!(rust_get_comm_fade_time(), 30);

        assert_eq!(rust_is_comm_input_paused(), 0);
        rust_set_comm_input_paused(1);
        assert_eq!(rust_is_comm_input_paused(), 1);

        rust_uninit_communication();
    }

    #[test]
    #[serial]
    fn test_ffi_track_text_only() {
        unsafe {
            reset_state();
            rust_init_communication();

            let text = CString::new("Subtitle only").unwrap();
            rust_splice_track_text(text.as_ptr(), 0.0, 2.0);

            // Track should have a chunk
            assert!(rust_get_track_length() > 0.0);

            rust_uninit_communication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_track_position() {
        unsafe {
            reset_state();
            rust_init_communication();

            let text = CString::new("Test").unwrap();
            rust_splice_track(1, text.as_ptr(), 0.0, 5.0);

            rust_start_track();
            rust_seek_track(2.5);
            assert!((rust_get_track_position() - 2.5).abs() < 0.01);

            rust_jump_track();
            // JumpTrack skips to end of current phrase — position depends on track state

            rust_rewind_track();
            assert!((rust_get_track_position() - 0.0).abs() < 0.01);

            rust_uninit_communication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_update() {
        unsafe {
            reset_state();
            rust_init_communication();

            let text = CString::new("Test").unwrap();
            rust_splice_track(1, text.as_ptr(), 0.0, 1.0);

            rust_start_track();
            rust_update_communication(0.5);
            assert!((rust_get_track_position() - 0.5).abs() < 0.01);

            rust_update_communication(1.0);
            assert_eq!(rust_is_talking_finished(), 1);

            rust_uninit_communication();
        }
    }

    #[test]
    #[serial]
    fn test_ffi_null_safety() {
        unsafe {
            reset_state();
            rust_init_communication();

            // Should handle null pointers gracefully
            rust_splice_track(1, std::ptr::null(), 0.0, 1.0);
            rust_splice_track_text(std::ptr::null(), 0.0, 1.0);
            assert_eq!(do_response_phrase(1, None, None), 0);
            rust_add_oscilloscope_samples(std::ptr::null(), 0);

            rust_uninit_communication();
        }
    }

    /// The Rust-ported `VisitStarBase` sets the intro mode for the Commander
    /// conversation that follows its time passage. That mode must be the one
    /// the talk segue reads, and it must survive the hail's state reset.
    #[test]
    #[serial]
    fn set_comm_intro_mode_feeds_the_mode_the_talk_segue_reads() {
        reset_state();
        rust_init_communication();

        set_comm_intro_mode(3, 1680);
        assert_eq!(rust_get_comm_intro_mode(), 3);
        assert_eq!(rust_get_comm_fade_time(), 1680);

        COMM_STATE.write().reset_for_encounter();
        assert_eq!(rust_get_comm_intro_mode(), 3);
        assert_eq!(rust_get_comm_fade_time(), 1680);

        rust_uninit_communication();
    }
}

#[cfg(test)]
#[test]
#[serial_test::serial]
fn alien_talk_callbacks_run_without_the_global_state_lock() {
    COMM_STATE.write().reset_for_encounter();
    let finished = run_alien_talk_unlocked(
        1,
        || assert!(COMM_STATE.try_write().is_some()),
        |wait| {
            assert_eq!(wait, 1);
            assert!(COMM_STATE.try_write().is_some());
            true
        },
        || assert!(COMM_STATE.try_write().is_some()),
    );
    assert_eq!(finished, 1);
    assert!(COMM_STATE.read().is_talking_finished());
}
