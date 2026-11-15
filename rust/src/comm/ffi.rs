//! Rust communication facade for state, playback and dialogue orchestration.

use crate::comm::Comm;
use crate::engine::observe::CheckpointView;
use crate::engine::StoryKey;
use std::ffi::{c_int, c_uint, CStr};

use super::segue::Segue;
use super::types::CommIntroMode;

mod response_commands;
pub(crate) use response_commands::observe_responses;
mod speech;
pub use response_commands::{do_response_phrase, rust_clear_responses};

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the communication system
pub fn rust_init_communication(comm: &mut Comm) -> c_int {
    match comm.init() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Uninitialize the communication system
pub fn rust_uninit_communication(comm: &mut Comm) {
    comm.uninit();
}

/// Check if communication is initialized
pub fn rust_is_comm_initialized(comm: &mut Comm) -> c_int {
    if comm.is_initialized() {
        1
    } else {
        0
    }
}

/// Clear communication state
pub fn rust_clear_communication(comm: &mut Comm) {
    comm.clear();
}

// ============================================================================
// Track Management
// ============================================================================

/// Start the speech track
pub fn rust_start_track(comm: &mut Comm) -> c_int {
    match comm.start_track() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Stop the speech track
pub fn rust_stop_track(comm: &mut Comm) {
    comm.stop_track();
}

/// Rewind the track to the beginning
pub fn rust_rewind_track(comm: &mut Comm) {
    comm.track_mut().rewind();
}

/// Jump to end of current phrase (skip current speech).
/// No offset parameter — JumpTrack advances to end of current phrase only (TP-REQ-005).
pub fn rust_jump_track(comm: &mut Comm) {
    comm.track_mut().jump(0.0);
}

/// Seek to absolute position in track
pub fn rust_seek_track(comm: &mut Comm, position: f32) {
    comm.track_mut().seek(position);
}

/// Commit track position (for save/restore)
pub fn rust_commit_track(comm: &mut Comm) -> f32 {
    comm.track_mut().commit()
}

/// Wait for track to finish (returns 1 when done)
pub fn rust_wait_track(comm: &mut Comm) -> c_int {
    if comm.wait_track() {
        1
    } else {
        0
    }
}

/// Get track position
pub fn rust_get_track_position(comm: &mut Comm) -> f32 {
    comm.track().position()
}

/// Get track length
pub fn rust_get_track_length(comm: &mut Comm) -> f32 {
    comm.track().length()
}

/// Clear the track
pub fn rust_clear_track(comm: &mut Comm) {
    comm.track_mut().clear();
}

/// Check if a track is currently playing.
pub fn rust_playing_track(comm: &mut Comm) -> c_uint {
    if comm.track().is_playing() {
        1
    } else {
        0
    }
}

/// Fast-forward by one page (subtitle page skip).
pub fn rust_fast_forward_page(comm: &mut Comm) {
    comm.track_mut().fast_forward_page();
}

/// Smooth fast-forward (increase playback rate).
pub fn rust_fast_forward_smooth(comm: &mut Comm) {
    comm.track_mut().fast_forward_smooth();
}

/// Reverse by one page.
pub fn rust_fast_reverse_page(comm: &mut Comm) {
    comm.track_mut().fast_reverse_page();
}

/// Smooth reverse (decrease playback rate / rewind).
pub fn rust_fast_reverse_smooth(comm: &mut Comm) {
    comm.track_mut().fast_reverse_smooth();
}

// ============================================================================
// Subtitle Management
// ============================================================================

/// Enable/disable subtitles
pub fn rust_set_subtitles_enabled(comm: &mut Comm, enabled: c_int) {
    comm.subtitles_mut().set_enabled(enabled != 0);
}

/// Check if subtitles are enabled
pub fn rust_are_subtitles_enabled(comm: &mut Comm) -> c_int {
    if comm.subtitles().is_enabled() {
        1
    } else {
        0
    }
}

// ============================================================================
// Response System
// ============================================================================

/// Display response choices
pub fn rust_display_responses(comm: &mut Comm) {
    comm.display_responses();
}

/// Select next response
pub fn rust_select_next_response(comm: &mut Comm) -> c_int {
    if comm.select_next_response() {
        1
    } else {
        0
    }
}

/// Select previous response
pub fn rust_select_prev_response(comm: &mut Comm) -> c_int {
    if comm.select_prev_response() {
        1
    } else {
        0
    }
}

/// Get selected response index
pub fn rust_get_selected_response(comm: &mut Comm) -> c_int {
    comm.selected_response()
}

/// Select an exact response index for semantic automation.
///
/// Returns 1 only when the index is valid for the current response list.
pub fn rust_select_response_index(comm: &mut Comm, index: c_int) -> c_int {
    if comm.responses_mut().select(index) {
        1
    } else {
        0
    }
}

/// Resolve a phrase name against the offered list on the game thread.
pub fn response_choices(comm: &Comm) -> crate::engine::observe::ResponseChoices {
    comm.responses().choices(|reference| {
        let table = comm.display.phrases.as_ref()?;
        if reference == 0 || reference as usize > table.len() {
            return None;
        }
        table
            .phrase((reference - 1) as u16)?
            .name
            .map(|name| name.to_string_lossy().into_owned())
    })
}

/// Get number of responses
pub fn rust_get_response_count(comm: &mut Comm) -> c_int {
    comm.responses().count() as c_int
}

/// Execute selected response callback — passes response_ref as argument (RS-REQ-011).
pub fn rust_execute_response(comm: &mut Comm, cx: &mut crate::engine::battle::Cx<'_>) -> c_uint {
    let state = &*comm;
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

    // Call with response_ref per RS-REQ-011
    func(comm, cx, response_ref);

    response_ref
}

// ============================================================================
// Animation Management
// ============================================================================

/// Initialize communication animations from current CommData.
/// @plan PLAN-20260314-COMM.P07
pub fn rust_init_comm_animations(_comm: &mut Comm) {
    // Animation initialization requires CommData to be loaded.
    // In production, this is called after init_race populates CommData.
    // The actual descriptor population happens from C side (stubs in commanim).
}

/// Process communication animations for one frame.
/// @plan PLAN-20260314-COMM.P07
pub fn rust_process_comm_animations(comm: &mut Comm, delta_ticks: c_uint) -> c_int {
    let changed = comm.animations_mut().process(delta_ticks);
    changed as c_int
}

/// Process communication animations — C bridge signature matching commanim.c.
///
/// `clear` (BOOLEAN): full-redraw flag (passes FullRedraw to animation engine).
/// `paused` (BOOLEAN): if non-zero, drive colormap transforms only, no frame advance.
/// Returns BOOLEAN: non-zero if any visible change occurred.
///
/// This is the signature expected by the USE_RUST_COMM stubs in commanim.c.
pub fn rust_process_comm_animations_cb(comm: &mut Comm, clear: c_int, paused: c_int) -> c_int {
    if paused != 0 {
        // Paused: drive colormap xforms only, no frame advancement.
        return 0;
    }
    // Use a fixed tick count (1) to advance one step; clear flag is passed
    // through as FullRedraw and forces change=true when set.
    let mut changed = comm.animations_mut().process(1);
    if clear != 0 {
        changed = true;
    }
    changed as c_int
}

/// Check if talking animation is wanted (defined with frames).
pub fn rust_want_talking_anim(comm: &mut Comm) -> c_int {
    comm.animations().want_talking_anim() as c_int
}

/// Check if talking animation is currently active.
pub fn rust_have_talking_anim(comm: &mut Comm) -> c_int {
    comm.animations().have_talking_anim() as c_int
}

/// Start the talking animation.
pub fn rust_set_run_talking_anim(comm: &mut Comm, _run: c_int) {
    comm.animations_mut().start_talking_anim();
}

/// Signal to stop the talking animation.
pub fn rust_set_stop_talking_anim(comm: &mut Comm) {
    comm.animations_mut().stop_talking_anim();
}

/// Set intro animation running state.
pub fn rust_set_run_intro_anim(comm: &mut Comm, run: c_int) {
    comm.animations_mut().set_intro_anim(run != 0);
}

/// Check if intro animation is running.
pub fn rust_running_intro_anim(comm: &mut Comm) -> c_int {
    comm.animations().is_intro_anim_running() as c_int
}

/// Check if talking animation is running.
pub fn rust_running_talking_anim(comm: &mut Comm) -> c_int {
    comm.animations().is_talking_anim_running() as c_int
}

/// Get current frame for an animation sequence.
pub fn rust_get_comm_animation_frame(comm: &mut Comm, index: c_uint) -> c_uint {
    comm.animations().get_frame(index as usize).unwrap_or(0)
}

// ============================================================================
// Encounter Lifecycle
// @plan PLAN-20260314-COMM.P08
// ============================================================================

/// Begin an encounter (marks active, records init callback).
pub fn rust_begin_encounter(comm: &mut Comm) -> c_int {
    match super::encounter::begin_encounter(comm) {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter normally (post + uninit callbacks, resource teardown).
pub fn rust_end_encounter_normal(comm: &mut Comm) -> c_int {
    match super::encounter::end_encounter_normal(comm) {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter on abort/load (uninit only, skip post).
pub fn rust_end_encounter_abort(comm: &mut Comm) -> c_int {
    match super::encounter::end_encounter_abort(comm) {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// End encounter for attack-without-hail (post + uninit, no init).
pub fn rust_end_encounter_attack(comm: &mut Comm) -> c_int {
    match super::encounter::end_encounter_attack(comm) {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Check if encounter is active.
pub fn rust_is_encounter_active(comm: &mut Comm) -> c_int {
    super::encounter::is_encounter_active(comm) as c_int
}

// ============================================================================
// Oscilloscope
// ============================================================================

/// Add samples to the oscilloscope
/// # Safety
///
/// A non-null `samples` must be readable for `count` samples.
pub unsafe fn rust_add_oscilloscope_samples(comm: &mut Comm, samples: *const i16, count: c_uint) {
    if samples.is_null() || count == 0 {
        return;
    }

    let samples_slice = unsafe { std::slice::from_raw_parts(samples, count as usize) };
    comm.add_oscilloscope_samples(samples_slice);
}

/// Update the oscilloscope display
pub fn rust_update_oscilloscope(comm: &mut Comm) {
    comm.oscilloscope_mut().update();
}

/// Get oscilloscope Y value at position
pub fn rust_get_oscilloscope_y(comm: &mut Comm, x: c_uint) -> u8 {
    comm.oscilloscope().get_y(x as usize)
}

/// Clear the oscilloscope
pub fn rust_clear_oscilloscope(comm: &mut Comm) {
    comm.oscilloscope_mut().clear();
}

// ============================================================================
// State Queries
// ============================================================================

/// Check if alien is talking
pub fn rust_is_talking(comm: &mut Comm) -> c_int {
    if comm.is_talking() {
        1
    } else {
        0
    }
}

/// Check if talking has finished
pub fn rust_is_talking_finished(comm: &mut Comm) -> c_int {
    if comm.is_talking_finished() {
        1
    } else {
        0
    }
}

/// Set talking finished flag
pub fn rust_set_talking_finished(comm: &mut Comm, finished: c_int) {
    comm.set_talking_finished(finished != 0);
}

/// Get intro mode
pub fn rust_get_comm_intro_mode(comm: &mut Comm) -> c_uint {
    comm.intro_mode() as c_uint
}

/// Set intro mode
pub fn rust_set_comm_intro_mode(comm: &mut Comm, mode: c_uint) {
    comm.set_intro_mode(CommIntroMode::from(mode));
}

/// C `SetCommIntroMode (mode, howLong)` (comm.c:1357): the mode and fade time
/// that the talk segue's intro transition reads. Callers that run in Rust set
/// them here, not through the C function, whose statics Rust never reads.
pub fn set_comm_intro_mode(comm: &mut Comm, mode: u32, how_long: u32) {
    let state = &mut *comm;
    state.set_intro_mode(CommIntroMode::from(mode));
    state.set_fade_time(how_long);
}

/// Get fade time
pub fn rust_get_comm_fade_time(comm: &mut Comm) -> c_uint {
    comm.fade_time()
}

/// Set fade time
pub fn rust_set_comm_fade_time(comm: &mut Comm, time: c_uint) {
    comm.set_fade_time(time);
}

/// Check if input is paused
pub fn rust_is_comm_input_paused(comm: &mut Comm) -> c_int {
    if comm.is_input_paused() {
        1
    } else {
        0
    }
}

/// Set input paused
pub fn rust_set_comm_input_paused(comm: &mut Comm, paused: c_int) {
    comm.set_input_paused(paused != 0);
}

/// Update communication state
pub fn rust_update_communication(comm: &mut Comm, delta_time: f32) {
    comm.update(delta_time);
}

// ============================================================================
// Phrase State (P04)
// @plan PLAN-20260314-COMM.P04
// ============================================================================

/// Check if a phrase is enabled (not disabled this encounter).
pub fn rust_phrase_enabled(comm: &mut Comm, index: c_int) -> c_int {
    if comm.phrase_enabled(index) {
        1
    } else {
        0
    }
}

/// Disable a phrase for the remainder of this encounter.
pub fn rust_disable_phrase(comm: &mut Comm, index: c_int) {
    comm.disable_phrase(index);
}

// ============================================================================
// Segue (P04)
// @plan PLAN-20260314-COMM.P04
// ============================================================================

/// Set segue state (0=Peace, 1=Hostile, 2=Victory, 3=Defeat).
pub fn rust_set_segue(comm: &mut Comm, segue: c_uint) {
    comm.set_segue(Segue::from(segue));
}

/// Get segue state (0=Peace, 1=Hostile, 2=Victory, 3=Defeat).
pub fn rust_get_segue(comm: &mut Comm) -> c_uint {
    u32::from(comm.get_segue())
}

/// Get BATTLE_SEGUE value for current segue (0=peace, 1=combat).
pub fn rust_get_battle_segue(comm: &mut Comm) -> c_uint {
    comm.get_segue().to_battle_segue()
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
/// Matches C `AlienTalkSegue(comm, wait_track)`.
/// Returns 1 if talking finished (reached end), 0 otherwise.
pub unsafe fn rust_alien_talk_segue(
    comm: &mut Comm,
    input: &crate::engine::input::InputSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    wait: c_uint,
) -> c_int {
    #[cfg(not(test))]
    {
        let aborted = super::talk_segue::check_abort_external(cx.world);
        run_alien_talk_unlocked(
            comm,
            wait,
            aborted,
            &mut (cx, io),
            |comm, (cx, _)| super::talk_segue::alien_talk_first_call_init(comm, cx),
            |comm, (cx, io), wait| {
                super::talk_segue::dinput::run_talk_segue_dinput(comm, input, cx, io, wait)
            },
            |_, (cx, _)| cx.with_legacy(super::talk_segue::fade_music_to_foreground_bridge),
        )
    }
    #[cfg(test)]
    {
        let _ = io;
        let state = &mut *comm;
        let was_finished = state.is_talking_finished();
        super::talk_segue::alien_talk_segue(input, cx, state, wait);
        if !was_finished && state.is_talking_finished() {
            1
        } else {
            0
        }
    }
}

fn run_alien_talk_unlocked<C>(
    comm: &mut Comm,
    wait: u32,
    aborted: bool,
    context: &mut C,
    initialize: impl FnOnce(&mut Comm, &mut C),
    playback: impl FnOnce(&mut Comm, &mut C, u32) -> bool,
    fade: impl FnOnce(&mut Comm, &mut C),
) -> c_int {
    let initialize_needed = {
        let state = &mut *comm;
        if aborted || state.is_talking_finished() {
            return 0;
        }
        let needed = !state.first_talk_call;
        state.first_talk_call = true;
        needed
    };
    if initialize_needed {
        initialize(comm, context);
    }
    let finished = playback(comm, context, wait);
    comm.set_talking_finished(finished);
    if finished {
        fade(comm, context);
    }
    i32::from(finished)
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
pub fn rust_do_communication(
    comm: &mut Comm,
    input: &crate::engine::input::InputSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) -> c_int {
    if speech::run_pending(comm, input, cx, io) {
        return 1;
    }

    // Phase 2: Response handling.

    // Step 2a: Quick state check (short lock)
    let response_count = {
        let state = &*comm;
        eprintln!(
            "[DBG] rust_DoCommunication: response phase, num_responses={}",
            state.responses().count()
        );
        if super::talk_segue::check_abort_external(cx.world) {
            return 0;
        }
        state.responses().count()
    };

    response_commands::observe_responses(comm, io);
    io.observer
        .checkpoint(&CheckpointView::ResponsesReady(true));

    if response_count == 0 {
        // No responses — run last-replay then done.
        // run_last_replay calls C bridges, so no lock.
        use crate::sound::heart_ffi::fade_music as FadeMusic;
        let timeout = FadeMusic(
            0u8,
            (super::talk_segue::c_bridge::ONE_SECOND_TICKS * 3) as i16,
        );
        super::talk_segue::run_last_replay_bridge(comm, cx, io, timeout as i32);
        return 0;
    }

    // Step 2b: Initialize response display if needed (lock briefly, drop before C call)
    {
        let needs_init = comm.top_response.is_none();
        if needs_init {
            comm.top_response = Some(0);
            // Drop lock, then render (C→Rust callback)
            let (top, count, cur) = {
                let s = &*comm;
                (
                    0u8,
                    s.responses().count() as u8,
                    s.responses().selected().max(0) as u8,
                )
            };
            super::talk_segue::c_bridge::call_refresh_responses(comm, cx, top, count, cur);
        }
    }

    // Step 2c: Check input (short lock — no C callbacks)
    let select = super::talk_segue::check_select_external(input);
    let cancel = super::talk_segue::check_cancel_external(input);
    let up = super::talk_segue::check_up_external(input);
    let down = super::talk_segue::check_down_external(input);
    let left = super::talk_segue::check_left_external(input);

    if select {
        response_commands::deliver_selected(comm, cx, io);
        return 1;
    }

    if cancel {
        let won = super::talk_segue::won_last_battle_external(cx.world);
        if !won {
            // Conversation summary — C bridge, no lock needed
            super::talk_segue::c_bridge::call_select_conversation_summary(comm, cx, io);
            // After summary returns, re-render responses (no lock for C call)
            let (top, count, cur) = {
                let s = &*comm;
                (
                    s.top_response.unwrap_or(0),
                    s.responses().count() as u8,
                    s.responses().selected().max(0) as u8,
                )
            };
            super::talk_segue::c_bridge::call_refresh_responses(comm, cx, top, count, cur);
            return 1;
        }
    }

    if left {
        // Replay last phrase — all C bridge calls, no lock
        cx.with_legacy(super::talk_segue::fade_music_to_background_bridge);
        super::talk_segue::c_bridge::call_feedback_player_phrase(comm, cx);
        #[cfg(not(test))]
        let _ = super::talk_segue::dinput::run_talk_segue_dinput(comm, input, cx, io, 0);
        if !super::talk_segue::check_abort_external(cx.world) {
            let (top, count, cur) = {
                let s = &*comm;
                (
                    s.top_response.unwrap_or(0),
                    s.responses().count() as u8,
                    s.responses().selected().max(0) as u8,
                )
            };
            super::talk_segue::c_bridge::call_refresh_responses(comm, cx, top, count, cur);
            cx.with_legacy(super::talk_segue::fade_music_to_foreground_bridge);
        }
        return 1;
    }

    // Step 2d: Navigate responses (short lock for state update)
    if up || down {
        let state = &mut *comm;
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

        // Render updated selection (C→Rust callback, no lock)
        super::talk_segue::c_bridge::call_refresh_responses(comm, cx, top, cnt, sel);
    }

    // Step 2e: Update animations (C bridge, no lock)
    cx.with_legacy(|| super::talk_segue::c_bridge::call_update_comm_graphics(comm, input));

    1
}

// ============================================================================
// Speech Graphics (P10)
// @plan PLAN-20260314-COMM.P10
// ============================================================================

/// Initialize speech graphics (oscilloscope + slider) for this encounter.
///
/// Matches C `InitSpeechGraphics`.
pub fn rust_init_speech_graphics(comm: &mut Comm) {
    crate::comm::speech_graphics::SpeechGraphics::init(comm);
}

/// Rate-limited update of speech graphics display.
///
/// Matches C `UpdateSpeechGraphics`. Uses current system time for rate
/// limiting. In test mode, operates on state fields only.
pub fn rust_update_speech_graphics(comm: &mut Comm) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| super::speech_graphics::update_timestamp(d.as_millis() as u64))
        .unwrap_or(0);
    crate::comm::speech_graphics::SpeechGraphics::update(comm, now_ms);
}

// ============================================================================
// Response UI (P10)
// @plan PLAN-20260314-COMM.P10
// ============================================================================

/// Refresh the response list display.
///
/// Matches C `RefreshResponses`. Delegates rendering to C bridge; Rust
/// updates scroll state.
pub fn rust_refresh_responses(comm: &mut Comm, world: &crate::engine::World) {
    comm.refresh_response_ui(world.activity());
}

// ============================================================================
// Subtitle Display (P10)
// @plan PLAN-20260314-COMM.P10
// ============================================================================

/// Clear the subtitle display area.
///
/// Matches C `ClearSubtitles`.
pub fn rust_clear_subtitles(comm: &mut Comm) {
    comm.subtitle_display_mut().clear();
}

/// Check subtitle timing and update display if the text has changed.
///
/// Matches C `CheckSubtitles`.
pub fn rust_check_subtitles(comm: &mut Comm) {
    let state = &mut *comm;
    let current = state.current_subtitle().map(|s| s.to_owned());
    state
        .subtitle_display_mut()
        .check_subtitle(current.as_deref());
}

/// Redraw the current subtitle text.
///
/// Matches C `RedrawSubtitles`.
pub fn rust_redraw_subtitles(comm: &mut Comm, input: &crate::engine::input::InputSession) {
    super::subtitle_display::SubtitleDisplay::redraw(comm, input.preferences().subtitles != 0);
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
pub fn rust_show_conversation_summary(
    comm: &mut Comm,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) -> c_int {
    super::sis_graphics::select_conversation_summary(comm, cx, io);
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
pub fn rust_show_conversation_summary(
    comm: &mut Comm,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) -> c_int {
    let _ = (cx, io);
    use super::summary::SummaryResult;
    use super::summary::SummaryView;

    // Rebuild summary from current track history.
    comm.rebuild_summary();

    let lines_per_page = 10usize;
    let mut view = SummaryView::new(lines_per_page);

    let total = {
        let state = &*comm;
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
// NPC Phrase (P04)
// @plan PLAN-20260326-COMMPT2.P04
// @requirement REQ-NP-001, REQ-NP-002, REQ-NP-003, REQ-NP-004
// ============================================================================

const GLOBAL_PLAYER_NAME: c_int = -1_000_000;
const GLOBAL_SHIP_NAME: c_int = -999_999;
const GLOBAL_ALLIANCE_NAME: c_int = -999_998;
const ALLIANCE_NAME_BUF_SIZE: usize = 400;

fn format_alliance_name(buffer: &mut [u8], phrase: &[u8], commander: Option<&[u8]>) {
    let mut used = phrase.len().min(buffer.len() - 1);
    buffer[..used].copy_from_slice(&phrase[..used]);
    if let Some(name) = commander {
        let end = name
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(name.len());
        let count = end.min(buffer.len() - used - 1);
        buffer[used..used + count].copy_from_slice(&name[..count]);
        used += count;
    }
    buffer[used] = 0;
}

pub fn npc_phrase(
    world: &crate::engine::World,
    comm: &mut Comm,
    index: c_int,
    callback: super::glue::PhraseCallback,
) {
    if index == 0 {
        return;
    }
    let callback = comm.queue_phrase_callback(callback);
    if index == GLOBAL_PLAYER_NAME || index == GLOBAL_SHIP_NAME {
        let bytes = if index == GLOBAL_PLAYER_NAME {
            &world.data.flagship.commander_name
        } else {
            &world.data.flagship.ship_name
        };
        let name = CStr::from_bytes_until_nul(bytes).expect("flagship name is terminated");
        crate::sound::heart_ffi::splice_track(None, Some(name), None, callback);
    } else if index < 0 {
        let variant = world.story_state(StoryKey::NEW_ALLIANCE_NAME) as c_int;
        let adjusted = index - GLOBAL_ALLIANCE_NAME;
        let Some(phrase) = comm
            .display
            .phrases
            .as_ref()
            .and_then(|table| table.phrase(((adjusted - 1) + variant) as u16))
            .and_then(|phrase| phrase.text)
        else {
            return;
        };
        let mut buffer = [0; ALLIANCE_NAME_BUF_SIZE];
        format_alliance_name(
            &mut buffer,
            phrase.to_bytes(),
            (variant == 3).then_some(world.data.flagship.commander_name.as_slice()),
        );
        let text = CStr::from_bytes_until_nul(&buffer).expect("alliance name is terminated");
        crate::sound::heart_ffi::splice_track(None, Some(text), None, callback);
    } else {
        let Some(phrase) = comm
            .display
            .phrases
            .as_ref()
            .and_then(|table| table.phrase((index - 1) as u16))
        else {
            return;
        };
        let Some(text) = phrase.text else { return };
        crate::sound::heart_ffi::splice_track(phrase.clip, Some(text), phrase.timestamp, callback);
    }
    comm.rebuild_summary();
}

pub fn splice_phrase(world: &crate::engine::World, comm: &mut Comm, index: c_int) {
    if index == 0 {
        return;
    }
    if index < 0 {
        npc_phrase(world, comm, index, None);
        return;
    }
    let Some(phrase) = comm
        .display
        .phrases
        .as_ref()
        .and_then(|table| table.phrase((index - 1) as u16))
    else {
        return;
    };
    let Some(text) = phrase.text else { return };
    if let Some(clip) = phrase.clip {
        crate::sound::heart_ffi::splice_multi_track(&[clip], Some(text));
    } else {
        crate::sound::heart_ffi::splice_track(None, Some(text), None, None);
    }
    comm.rebuild_summary();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::ffi::CString;

    #[test]
    fn alliance_name_truncates_and_terminates_each_buffer_size() {
        for size in 1..12 {
            let mut buffer = vec![255; size];
            format_alliance_name(&mut buffer, b"Alliance ", Some(b"Zelnik\0ignored"));
            let expected = b"Alliance Zelnik";
            let length = expected.len().min(size - 1);
            assert_eq!(&buffer[..length], &expected[..length]);
            assert_eq!(buffer[length], 0);
        }
        let mut buffer = [255; 32];
        format_alliance_name(&mut buffer, b"Alliance", None);
        assert_eq!(&buffer[..9], b"Alliance\0");
    }

    fn reset_state(comm: &mut Comm) {
        rust_uninit_communication(comm);
    }

    #[test]
    #[serial]
    fn test_ffi_init_uninit() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);

        assert_eq!(rust_init_communication(comm), 1);
        assert_eq!(rust_is_comm_initialized(comm), 1);

        rust_uninit_communication(comm);
        assert_eq!(rust_is_comm_initialized(comm), 0);
    }

    #[test]
    #[serial]
    fn test_ffi_track_management() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        rust_init_communication(comm);

        let text = CString::new("Hello world").unwrap();
        comm.track_mut()
            .splice_track(1, Some(text.to_str().unwrap()), 0.0, 2.0);

        assert_eq!(rust_start_track(comm), 1);
        assert_eq!(rust_is_talking(comm), 1);

        rust_stop_track(comm);
        assert_eq!(rust_is_talking(comm), 0);

        rust_uninit_communication(comm);
    }

    #[test]
    #[serial]
    fn test_ffi_response_system() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        rust_init_communication(comm);

        let text1 = CString::new("Option A").unwrap();
        let text2 = CString::new("Option B").unwrap();

        assert_eq!(do_response_phrase(comm, 1, Some(text1.as_c_str()), None), 1);
        assert_eq!(do_response_phrase(comm, 2, Some(text2.as_c_str()), None), 1);

        assert_eq!(rust_get_response_count(comm), 2);

        rust_display_responses(comm);
        assert_eq!(rust_get_selected_response(comm), 0);

        rust_select_next_response(comm);
        assert_eq!(rust_get_selected_response(comm), 1);

        rust_clear_responses(comm);
        assert_eq!(rust_get_response_count(comm), 0);

        rust_uninit_communication(comm);
    }

    #[test]
    #[serial]
    fn test_ffi_subtitles() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        rust_init_communication(comm);

        assert_eq!(rust_are_subtitles_enabled(comm), 1);

        rust_set_subtitles_enabled(comm, 0);
        assert_eq!(rust_are_subtitles_enabled(comm), 0);

        rust_set_subtitles_enabled(comm, 1);
        assert_eq!(rust_are_subtitles_enabled(comm), 1);

        rust_uninit_communication(comm);
    }

    #[test]
    #[serial]
    fn test_ffi_oscilloscope() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        unsafe {
            reset_state(comm);
            rust_init_communication(comm);

            let samples: [i16; 4] = [100, 200, 300, 400];
            rust_add_oscilloscope_samples(comm, samples.as_ptr(), 4);
            rust_update_oscilloscope(comm);

            // Should return a value without panicking.
            let _ = rust_get_oscilloscope_y(comm, 0);

            rust_clear_oscilloscope(comm);

            rust_uninit_communication(comm);
        }
    }

    #[test]
    #[serial]
    fn test_ffi_state_queries() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        rust_init_communication(comm);

        assert_eq!(rust_is_talking_finished(comm), 0);
        rust_set_talking_finished(comm, 1);
        assert_eq!(rust_is_talking_finished(comm), 1);

        rust_set_comm_intro_mode(comm, 2);
        assert_eq!(rust_get_comm_intro_mode(comm), 2);

        rust_set_comm_fade_time(comm, 30);
        assert_eq!(rust_get_comm_fade_time(comm), 30);

        assert_eq!(rust_is_comm_input_paused(comm), 0);
        rust_set_comm_input_paused(comm, 1);
        assert_eq!(rust_is_comm_input_paused(comm), 1);

        rust_uninit_communication(comm);
    }

    #[test]
    #[serial]
    fn test_ffi_track_text_only() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        rust_init_communication(comm);

        let text = CString::new("Subtitle only").unwrap();
        comm.track_mut()
            .splice_text(text.to_str().unwrap(), 0.0, 2.0);

        // Track should have a chunk
        assert!(rust_get_track_length(comm) > 0.0);

        rust_uninit_communication(comm);
    }

    #[test]
    #[serial]
    fn test_ffi_track_position() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        rust_init_communication(comm);

        let text = CString::new("Test").unwrap();
        comm.track_mut()
            .splice_track(1, Some(text.to_str().unwrap()), 0.0, 5.0);

        rust_start_track(comm);
        rust_seek_track(comm, 2.5);
        assert!((rust_get_track_position(comm) - 2.5).abs() < 0.01);

        rust_jump_track(comm);
        // JumpTrack skips to end of current phrase — position depends on track state

        rust_rewind_track(comm);
        assert!((rust_get_track_position(comm) - 0.0).abs() < 0.01);

        rust_uninit_communication(comm);
    }

    #[test]
    #[serial]
    fn test_ffi_update() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        rust_init_communication(comm);

        let text = CString::new("Test").unwrap();
        comm.track_mut()
            .splice_track(1, Some(text.to_str().unwrap()), 0.0, 1.0);

        rust_start_track(comm);
        rust_update_communication(comm, 0.5);
        assert!((rust_get_track_position(comm) - 0.5).abs() < 0.01);

        rust_update_communication(comm, 1.0);
        assert_eq!(rust_is_talking_finished(comm), 1);

        rust_uninit_communication(comm);
    }

    #[test]
    #[serial]
    fn test_ffi_null_safety() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        unsafe {
            reset_state(comm);
            rust_init_communication(comm);

            // Should handle null pointers gracefully
            comm.track_mut().splice_track(1, None, 0.0, 1.0);
            assert!(comm.current_subtitle().is_none());
            assert_eq!(do_response_phrase(comm, 1, None, None), 0);
            rust_add_oscilloscope_samples(comm, std::ptr::null(), 0);

            rust_uninit_communication(comm);
        }
    }

    /// The Rust-ported `VisitStarBase` sets the intro mode for the Commander
    /// conversation that follows its time passage. That mode must be the one
    /// the talk segue reads, and it must survive the hail's state reset.
    #[test]
    #[serial]
    fn set_comm_intro_mode_feeds_the_mode_the_talk_segue_reads() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        rust_init_communication(comm);

        set_comm_intro_mode(comm, 3, 1680);
        assert_eq!(rust_get_comm_intro_mode(comm), 3);
        assert_eq!(rust_get_comm_fade_time(comm), 1680);

        comm.reset_for_encounter();
        assert_eq!(rust_get_comm_intro_mode(comm), 3);
        assert_eq!(rust_get_comm_fade_time(comm), 1680);

        rust_uninit_communication(comm);
    }
}

#[cfg(test)]
#[test]
#[serial_test::serial]
fn alien_talk_callbacks_mutate_the_supplied_session_in_order() {
    let comm = &mut Comm::new();
    let mut calls = Vec::new();
    let finished = run_alien_talk_unlocked(
        comm,
        1,
        false,
        &mut calls,
        |comm, calls| {
            comm.disable_phrase(7);
            calls.push("initialize");
        },
        |comm, calls, wait| {
            assert_eq!(wait, 1);
            assert!(!comm.phrase_enabled(7));
            calls.push("playback");
            true
        },
        |comm, calls| {
            assert!(comm.is_talking_finished());
            calls.push("fade");
        },
    );
    assert_eq!(finished, 1);
    assert!(comm.is_talking_finished());
    assert_eq!(calls, ["initialize", "playback", "fade"]);
}
