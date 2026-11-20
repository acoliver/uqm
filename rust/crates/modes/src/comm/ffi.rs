//! Rust communication facade for state, playback and dialogue orchestration.

use crate::comm::Comm;
use crate::engine::observe::CheckpointView;
use crate::engine::StoryKey;
use std::ffi::{c_int, c_uint};

use super::segue::Segue;

mod response_commands;
pub(crate) use response_commands::observe_responses;
mod speech;
pub use response_commands::{do_response_phrase, rust_clear_responses};

// ============================================================================
// Response System
// ============================================================================

/// Resolve a phrase name against the offered list on the game thread.
pub fn response_choices(comm: &Comm) -> crate::engine::observe::ResponseChoices {
    comm.responses().choices(|reference| {
        let table = comm.display.phrases.as_ref()?;
        if reference == 0 || reference as usize > table.len() {
            return None;
        }
        table
            .phrase(reference - 1)?
            .name
            .map(|name| name.to_string_lossy().into_owned())
    })
}

/// Execute selected response callback — passes response_ref as argument (RS-REQ-011).
pub fn rust_execute_response(comm: &mut Comm, cx: &mut crate::engine::battle::Cx<'_>) -> u16 {
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
// Segue (P04)
// @plan PLAN-20260314-COMM.P04
// ============================================================================

/// Set segue state (0=Peace, 1=Hostile, 2=Victory, 3=Defeat).
pub fn rust_set_segue(comm: &mut Comm, segue: c_uint) {
    comm.set_segue(Segue::from(segue));
}

// ============================================================================
// Talk Segue & Main Loop (P09)
// @plan PLAN-20260314-COMM.P09
// ============================================================================

/// Run one iteration of the alien talk segue for the given wait-track.
///
/// Matches C `AlienTalkSegue(comm, wait_track)`.
/// Returns 1 if talking finished (reached end), 0 otherwise.
pub fn rust_alien_talk_segue(
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
            |_, _| super::talk_segue::fade_music_to_foreground_bridge(),
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
        super::talk_segue::fade_music_to_background_bridge();
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
            super::talk_segue::fade_music_to_foreground_bridge();
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
    super::talk_segue::c_bridge::call_update_comm_graphics(comm, input);

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
    let now_ms = crate::platform::sys::unix_millis()
        .map(super::speech_graphics::update_timestamp)
        .unwrap_or(0);
    crate::comm::speech_graphics::SpeechGraphics::update(comm, now_ms);
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
        let resources = comm.conversation_resources();
        let name = if index == GLOBAL_PLAYER_NAME {
            resources.commander_name.as_c_str()
        } else {
            resources.ship_name.as_c_str()
        };
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
        let text = std::ffi::CString::from(
            buffer
                .into_iter()
                .map_while(std::num::NonZeroU8::new)
                .collect::<Vec<_>>(),
        );
        crate::sound::heart_ffi::splice_track(None, Some(&text), None, callback);
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
        comm.uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_init_uninit() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);

        assert!(comm.init().is_ok());
        assert!(comm.is_initialized());

        comm.uninit();
        assert!(!comm.is_initialized());
    }

    #[test]
    #[serial]
    fn test_ffi_track_management() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(comm.init().is_ok());

        let text = CString::new("Hello world").unwrap();
        comm.track_mut()
            .splice_track(1, Some(text.to_str().unwrap()), 0.0, 2.0);

        assert!(comm.start_track().is_ok());
        assert!(comm.is_talking());

        comm.stop_track();
        assert!(!comm.is_talking());

        comm.uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_response_system() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(comm.init().is_ok());

        let text1 = CString::new("Option A").unwrap();
        let text2 = CString::new("Option B").unwrap();

        assert_eq!(do_response_phrase(comm, 1, Some(text1.as_c_str()), None), 1);
        assert_eq!(do_response_phrase(comm, 2, Some(text2.as_c_str()), None), 1);

        assert_eq!(comm.responses().count() as c_int, 2);

        comm.display_responses();
        assert_eq!(comm.selected_response(), 0);

        comm.select_next_response();
        assert_eq!(comm.selected_response(), 1);

        rust_clear_responses(comm);
        assert_eq!(comm.responses().count() as c_int, 0);

        comm.uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_subtitles() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(comm.init().is_ok());

        assert!(comm.subtitles().is_enabled());

        comm.subtitles_mut().set_enabled(0 != 0);
        assert!(!comm.subtitles().is_enabled());

        comm.subtitles_mut().set_enabled(1 != 0);
        assert!(comm.subtitles().is_enabled());

        comm.uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_oscilloscope() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(comm.init().is_ok());

        let samples: [i16; 4] = [100, 200, 300, 400];
        comm.add_oscilloscope_samples(&samples);
        comm.oscilloscope_mut().update();

        // Should return a value without panicking.
        let _ = comm.oscilloscope().get_y(0);

        comm.oscilloscope_mut().clear();

        comm.uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_state_queries() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(comm.init().is_ok());

        assert!(!comm.is_talking_finished());
        comm.set_talking_finished(1 != 0);
        assert!(comm.is_talking_finished());

        comm.set_intro_mode(super::super::CommIntroMode::from(2));
        assert_eq!(comm.intro_mode() as u32, 2);

        comm.set_fade_time(30);
        assert_eq!(comm.fade_time(), 30);

        assert!(!comm.is_input_paused());
        comm.set_input_paused(1 != 0);
        assert!(comm.is_input_paused());

        comm.uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_track_text_only() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(comm.init().is_ok());

        let text = CString::new("Subtitle only").unwrap();
        comm.track_mut()
            .splice_text(text.to_str().unwrap(), 0.0, 2.0);

        // Track should have a chunk
        assert!(comm.track().length() > 0.0);

        comm.uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_track_position() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(comm.init().is_ok());

        let text = CString::new("Test").unwrap();
        comm.track_mut()
            .splice_track(1, Some(text.to_str().unwrap()), 0.0, 5.0);

        assert!(comm.start_track().is_ok());
        comm.track_mut().seek(2.5);
        assert!((comm.track().position() - 2.5).abs() < 0.01);

        comm.track_mut().jump(0.0);
        // JumpTrack skips to end of current phrase — position depends on track state

        comm.track_mut().rewind();
        assert!((comm.track().position() - 0.0).abs() < 0.01);

        comm.uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_update() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(comm.init().is_ok());

        let text = CString::new("Test").unwrap();
        comm.track_mut()
            .splice_track(1, Some(text.to_str().unwrap()), 0.0, 1.0);

        assert!(comm.start_track().is_ok());
        comm.update(0.5);
        assert!((comm.track().position() - 0.5).abs() < 0.01);

        comm.update(1.0);
        assert!(comm.is_talking_finished());

        comm.uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_null_safety() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(comm.init().is_ok());

        // Should handle null pointers gracefully
        comm.track_mut().splice_track(1, None, 0.0, 1.0);
        assert!(comm.current_subtitle().is_none());
        assert_eq!(do_response_phrase(comm, 1, None, None), 0);

        comm.uninit();
    }

    /// The Rust-ported `VisitStarBase` sets the intro mode for the Commander
    /// conversation that follows its time passage. That mode must be the one
    /// the talk segue reads, and it must survive the hail's state reset.
    #[test]
    #[serial]
    fn supplied_intro_transition_survives_the_encounter_reset() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        reset_state(comm);
        assert!(comm.init().is_ok());

        comm.set_intro_transition(super::super::CommIntroMode::from(3), 1680);
        assert_eq!(comm.intro_mode() as u32, 3);
        assert_eq!(comm.fade_time(), 1680);

        comm.reset_for_encounter();
        assert_eq!(comm.intro_mode() as u32, 3);
        assert_eq!(comm.fade_time(), 1680);

        comm.uninit();
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
