//! The communication facade driven the way the C conversation code drives it:
//! tracks spliced and played, responses offered and chosen, animation and
//! oscilloscope state, and the encounter lifecycle, all through the exported
//! `rust_*` entry points.

use std::ffi::{c_char, c_int, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use serial_test::serial;

use super::ffi::*;

/// An initialised communication system that is torn down again on drop.
struct Comm;

impl Comm {
    fn new() -> Self {
        unsafe {
            rust_UninitCommunication();
            assert_eq!(rust_InitCommunication(), 1);
        }
        Comm
    }
}

impl Drop for Comm {
    fn drop(&mut self) {
        unsafe {
            rust_ClearResponses();
            rust_UninitCommunication();
        }
    }
}

fn offer(text: &str, reference: u32, func: Option<crate::comm::response::ResponseFunc>) -> c_int {
    let text = CString::new(text).unwrap();
    do_response_phrase(reference, Some(text.as_c_str()), func)
}

fn response_text(index: c_int, capacity: c_int) -> (c_int, String) {
    let mut buf = vec![0x7fu8; 64];
    let ok = unsafe { rust_GetResponseText(index, buf.as_mut_ptr() as *mut c_char, capacity) };
    let text = CStr::from_bytes_until_nul(&buf)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    (ok, text)
}

static CHOSEN: AtomicU32 = AtomicU32::new(0);

fn remember_choice(reference: u32) {
    CHOSEN.store(reference, Ordering::SeqCst);
}

#[test]
#[serial]
fn the_system_is_initialised_cleared_and_uninitialised() {
    unsafe {
        rust_UninitCommunication();
        assert_eq!(rust_IsCommInitialized(), 0);
        assert_eq!(rust_InitCommunication(), 1);
        assert_eq!(rust_IsCommInitialized(), 1);
        offer("kept?", 1, None);
        rust_ClearCommunication();
        assert_eq!(
            rust_GetResponseCount(),
            0,
            "clearing drops the conversation"
        );
        rust_UninitCommunication();
        assert_eq!(rust_IsCommInitialized(), 0);
    }
}

#[test]
#[serial]
fn a_spliced_track_has_a_length_and_plays_from_the_start() {
    let _comm = Comm::new();
    let line = CString::new("Greetings, Captain.").unwrap();
    unsafe {
        assert_eq!(rust_PlayingTrack(), 0);
        rust_SpliceTrack(7, line.as_ptr(), 0.0, 2.0);
        rust_SpliceTrack(8, ptr::null(), 2.0, 1.5);
        assert!(
            rust_GetTrackLength() >= 3.5 - 1e-3,
            "length {}",
            rust_GetTrackLength()
        );
        assert_eq!(rust_GetTrackPosition(), 0.0);

        assert_eq!(rust_StartTrack(), 1);
        assert_eq!(rust_PlayingTrack(), 1);
        assert_eq!(rust_IsTalking(), 1);
        rust_StopTrack();
        assert_eq!(rust_PlayingTrack(), 0);
        assert_eq!(rust_IsTalking(), 0);
    }
}

#[test]
#[serial]
fn splicing_text_without_audio_extends_the_track() {
    let _comm = Comm::new();
    let line = CString::new("Silent line").unwrap();
    let bad = [0xffu8, 0xfe, 0];
    unsafe {
        let before = rust_GetTrackLength();
        rust_SpliceTrackText(ptr::null(), 0.0, 1.0);
        rust_SpliceTrackText(bad.as_ptr() as *const c_char, 0.0, 1.0);
        assert_eq!(
            rust_GetTrackLength(),
            before,
            "null and undecodable text add nothing"
        );
        rust_SpliceTrackText(line.as_ptr(), 0.0, 2.0);
        assert!(rust_GetTrackLength() > before);
    }
}

#[test]
#[serial]
fn an_undecodable_subtitle_is_spliced_as_no_subtitle() {
    let _comm = Comm::new();
    let bad = [0xffu8, 0xfe, 0];
    unsafe {
        rust_SpliceTrack(3, bad.as_ptr() as *const c_char, 0.0, 1.0);
        assert!(rust_GetTrackLength() >= 1.0 - 1e-3);
        assert_eq!(rust_StartTrack(), 1);
        assert!(rust_GetSubtitle().is_null(), "there is nothing to display");
    }
}

#[test]
#[serial]
fn seeking_rewinding_and_clearing_move_the_play_position() {
    let _comm = Comm::new();
    let line = CString::new("A long speech").unwrap();
    unsafe {
        rust_SpliceTrack(1, line.as_ptr(), 0.0, 10.0);
        assert_eq!(rust_StartTrack(), 1);
        rust_SeekTrack(4.0);
        assert!(
            (rust_GetTrackPosition() - 4.0).abs() < 1e-3,
            "{}",
            rust_GetTrackPosition()
        );
        rust_RewindTrack();
        assert_eq!(rust_GetTrackPosition(), 0.0);
        rust_SeekTrack(6.5);
        assert!(
            (rust_CommitTrack() - 6.5).abs() < 1e-3,
            "commit reports where the track is"
        );
        rust_JumpTrack();
        assert!((0.0..=10.0).contains(&rust_GetTrackPosition()));
        rust_ClearTrack();
        assert_eq!(rust_GetTrackLength(), 0.0);
        assert_eq!(rust_PlayingTrack(), 0);
    }
}

#[test]
#[serial]
fn paging_moves_between_spliced_lines_and_smooth_scrubbing_moves_half_a_second() {
    let _comm = Comm::new();
    let line = CString::new("One line of speech").unwrap();
    unsafe {
        for (handle, start) in [(1, 0.0), (2, 10.0), (3, 20.0)] {
            rust_SpliceTrack(handle, line.as_ptr(), start, 10.0);
        }
        assert_eq!(rust_StartTrack(), 1);
        assert!((rust_GetTrackLength() - 30.0).abs() < 1e-3);

        rust_FastForward_Page();
        assert!((rust_GetTrackPosition() - 10.0).abs() < 1e-3);
        rust_FastForward_Page();
        assert!((rust_GetTrackPosition() - 20.0).abs() < 1e-3);
        rust_FastForward_Page();
        assert!(
            (rust_GetTrackPosition() - 30.0).abs() < 1e-3,
            "past the last line is the end"
        );

        rust_SeekTrack(25.0);
        rust_FastReverse_Page();
        assert!(
            (rust_GetTrackPosition() - 10.0).abs() < 1e-3,
            "back to the start of the previous line"
        );
        rust_FastReverse_Page();
        assert!(rust_GetTrackPosition().abs() < 1e-3);
        rust_FastReverse_Page();
        assert!(
            rust_GetTrackPosition().abs() < 1e-3,
            "nothing before the first line"
        );

        rust_SeekTrack(5.0);
        rust_FastForward_Smooth();
        assert!((rust_GetTrackPosition() - 5.5).abs() < 1e-3);
        rust_FastReverse_Smooth();
        rust_FastReverse_Smooth();
        assert!((rust_GetTrackPosition() - 4.5).abs() < 1e-3);
        rust_SeekTrack(0.2);
        rust_FastReverse_Smooth();
        assert!(
            rust_GetTrackPosition().abs() < 1e-3,
            "scrubbing stops at the start"
        );
    }
}

#[test]
#[serial]
fn waiting_on_a_track_is_over_only_once_it_has_stopped() {
    let _comm = Comm::new();
    let line = CString::new("Speech").unwrap();
    unsafe {
        rust_SpliceTrack(1, line.as_ptr(), 0.0, 5.0);
        assert_eq!(rust_StartTrack(), 1);
        assert_eq!(rust_WaitTrack(), 0, "still talking");
        rust_StopTrack();
        assert_eq!(rust_WaitTrack(), 1);
    }
}

#[test]
#[serial]
fn subtitles_can_be_switched_off_and_on() {
    let _comm = Comm::new();
    unsafe {
        assert_eq!(rust_AreSubtitlesEnabled(), 1);
        rust_SetSubtitlesEnabled(0);
        assert_eq!(rust_AreSubtitlesEnabled(), 0);
        rust_SetSubtitlesEnabled(5);
        assert_eq!(
            rust_AreSubtitlesEnabled(),
            1,
            "any non-zero value turns them on"
        );
    }
}

#[test]
#[serial]
fn the_current_subtitle_is_handed_to_c_as_a_terminated_string() {
    let _comm = Comm::new();
    let line = CString::new("We come in peace.").unwrap();
    unsafe {
        assert!(rust_GetSubtitle().is_null());
        rust_SpliceTrack(1, line.as_ptr(), 0.0, 3.0);
        assert_eq!(rust_StartTrack(), 1);
        let ptr = rust_GetSubtitle();
        assert!(!ptr.is_null());
        assert_eq!(CStr::from_ptr(ptr).to_str().unwrap(), "We come in peace.");
    }
}

#[test]
#[serial]
fn responses_are_offered_displayed_and_selected_with_wraparound() {
    let _comm = Comm::new();
    assert_eq!(offer("Who are you?", 10, None), 1);
    assert_eq!(offer("Goodbye.", 20, None), 1);
    assert_eq!(offer("Give me your ship.", 30, None), 1);
    unsafe {
        assert_eq!(rust_GetResponseCount(), 3);
        rust_DisplayResponses();
        assert_eq!(rust_GetSelectedResponse(), 0);

        assert_eq!(rust_SelectNextResponse(), 1);
        assert_eq!(rust_GetSelectedResponse(), 1);
        assert_eq!(rust_SelectNextResponse(), 1);
        assert_eq!(rust_SelectNextResponse(), 1);
        assert_eq!(rust_GetSelectedResponse(), 0, "next wraps to the first");
        assert_eq!(rust_SelectPrevResponse(), 1);
        assert_eq!(rust_GetSelectedResponse(), 2, "previous wraps to the last");

        assert_eq!(rust_select_response_index(1), 1);
        assert_eq!(rust_GetSelectedResponse(), 1);
        assert_eq!(rust_select_response_index(3), 0);
        assert_eq!(rust_select_response_index(-1), 0);
        assert_eq!(rust_GetSelectedResponse(), 1, "a bad index changes nothing");
    }
}

#[test]
#[serial]
fn there_is_nothing_to_select_without_responses() {
    let _comm = Comm::new();
    unsafe {
        assert_eq!(rust_SelectNextResponse(), 0);
        assert_eq!(rust_SelectPrevResponse(), 0);
        assert_eq!(rust_select_response_index(0), 0);
        assert_eq!(rust_ExecuteResponse(), 0);
    }
}

#[test]
#[serial]
fn a_response_with_a_handler_calls_it_with_its_reference() {
    let _comm = Comm::new();
    CHOSEN.store(0, Ordering::SeqCst);
    offer("First", 11, None);
    offer("Second", 22, Some(remember_choice));
    unsafe {
        assert_eq!(rust_select_response_index(1), 1);
        assert_eq!(rust_ExecuteResponse(), 22);
    }
    assert_eq!(CHOSEN.load(Ordering::SeqCst), 22);
}

#[test]
#[serial]
fn a_response_without_a_handler_just_reports_its_reference() {
    let _comm = Comm::new();
    CHOSEN.store(0, Ordering::SeqCst);
    offer("Plain", 33, None);
    unsafe {
        rust_DisplayResponses();
        assert_eq!(rust_ExecuteResponse(), 33);
    }
    assert_eq!(CHOSEN.load(Ordering::SeqCst), 0);
}

#[test]
#[serial]
fn offering_a_response_needs_readable_text() {
    let _comm = Comm::new();
    let bad = [0xffu8, 0xfe, 0];
    unsafe {
        assert_eq!(do_response_phrase(1, None, None), 0);
        assert_eq!(
            do_response_phrase(1, Some(CStr::from_bytes_with_nul(&bad).unwrap()), None),
            0
        );
        assert_eq!(rust_GetResponseCount(), 0);
    }
}

#[test]
#[serial]
fn response_text_is_copied_out_and_truncated_to_the_buffer() {
    let _comm = Comm::new();
    offer("Tell me about your homeworld.", 5, None);
    assert_eq!(
        response_text(0, 64),
        (1, "Tell me about your homeworld.".to_owned())
    );
    assert_eq!(
        response_text(0, 8),
        (1, "Tell me".to_owned()),
        "room for seven characters and the terminator"
    );
    assert_eq!(response_text(0, 1), (1, String::new()));
}

#[test]
#[serial]
fn response_text_refuses_bad_requests() {
    let _comm = Comm::new();
    offer("Only one", 5, None);
    let mut buf = [0x55u8; 8];
    unsafe {
        assert_eq!(rust_GetResponseText(0, ptr::null_mut(), 8), 0);
        assert_eq!(
            rust_GetResponseText(0, buf.as_mut_ptr() as *mut c_char, 0),
            0
        );
        assert_eq!(
            rust_GetResponseText(-1, buf.as_mut_ptr() as *mut c_char, 8),
            0
        );
        assert_eq!(
            rust_GetResponseText(1, buf.as_mut_ptr() as *mut c_char, 8),
            0
        );
    }
    assert_eq!(buf, [0x55; 8], "the caller's buffer is untouched");
}

#[test]
#[serial]
fn clearing_responses_empties_the_list() {
    let _comm = Comm::new();
    offer("a", 1, None);
    offer("b", 2, None);
    unsafe {
        rust_ClearResponses();
        assert_eq!(rust_GetResponseCount(), 0);
        assert_eq!(rust_ExecuteResponse(), 0);
    }
}

#[test]
#[serial]
fn intro_and_talking_animation_flags_follow_their_setters() {
    let _comm = Comm::new();
    unsafe {
        rust_init_comm_animations();
        assert_eq!(rust_RunningIntroAnim(), 0);
        rust_SetRunIntroAnim(1);
        assert_eq!(rust_RunningIntroAnim(), 1);
        rust_SetRunIntroAnim(0);
        assert_eq!(rust_RunningIntroAnim(), 0);

        assert_eq!(rust_RunningTalkingAnim(), 0);
        rust_SetRunTalkingAnim(1);
        assert_eq!(rust_RunningTalkingAnim(), 1);
        rust_SetStopTalkingAnim();
        assert_eq!(
            rust_RunningTalkingAnim(),
            1,
            "stopping is a request the animation loop honours on its next step"
        );
        assert_eq!(rust_WantTalkingAnim(), 0, "no talking sequence is defined");
        assert_eq!(rust_HaveTalkingAnim(), 0, "so none can be active");
        assert_eq!(rust_GetCommAnimationFrame(99), 0, "no such animation");
    }
}

#[test]
#[serial]
fn animation_callbacks_report_a_change_on_redraw_and_none_when_paused() {
    let _comm = Comm::new();
    unsafe {
        assert_eq!(
            rust_ProcessCommAnimations_cb(0, 1),
            0,
            "paused advances nothing"
        );
        assert_eq!(
            rust_ProcessCommAnimations_cb(1, 1),
            0,
            "paused even on a full redraw"
        );
        assert_eq!(
            rust_ProcessCommAnimations_cb(1, 0),
            1,
            "a full redraw always reports a change"
        );
        assert_eq!(
            rust_ProcessCommAnimations_cb(0, 0),
            0,
            "no animations means no change"
        );
        assert_eq!(rust_process_comm_animations(4), 0);
    }
}

#[test]
#[serial]
fn the_encounter_lifecycle_begins_and_ends_in_order() {
    let _comm = Comm::new();
    unsafe {
        if rust_is_encounter_active() == 1 {
            rust_end_encounter_abort();
        }
        assert_eq!(rust_is_encounter_active(), 0);
        assert_eq!(rust_end_encounter_normal(), 0, "nothing to end");
        assert_eq!(rust_end_encounter_abort(), 0);

        assert_eq!(rust_begin_encounter(), 1);
        assert_eq!(rust_is_encounter_active(), 1);
        assert_eq!(rust_begin_encounter(), 0, "one encounter at a time");
        assert_eq!(rust_end_encounter_normal(), 1);
        assert_eq!(rust_is_encounter_active(), 0);

        assert_eq!(rust_begin_encounter(), 1);
        assert_eq!(rust_end_encounter_abort(), 1);
        assert_eq!(rust_is_encounter_active(), 0);

        assert_eq!(
            rust_end_encounter_attack(),
            1,
            "attacking without hailing still ends cleanly"
        );
        assert_eq!(rust_is_encounter_active(), 0);
    }
}

#[test]
#[serial]
fn the_oscilloscope_shows_samples_and_clears_back_to_the_centre_line() {
    let _comm = Comm::new();
    let loud: Vec<i16> = vec![20_000; 2048];
    unsafe {
        assert_eq!(rust_GetOscilloscopeY(10), 128);
        rust_AddOscilloscopeSamples(ptr::null(), 4);
        rust_AddOscilloscopeSamples(loud.as_ptr(), 0);
        rust_AddOscilloscopeSamples(loud.as_ptr(), loud.len() as u32);
        rust_UpdateOscilloscope();
        let drawn: Vec<u8> = (0..128).map(|x| rust_GetOscilloscopeY(x)).collect();
        assert!(
            drawn.iter().any(|&y| y != 128),
            "the waveform moves off the centre line"
        );
        assert_eq!(
            rust_GetOscilloscopeY(100_000),
            128,
            "off the display is the centre line"
        );
        rust_ClearOscilloscope();
        assert!((0..128).all(|x| rust_GetOscilloscopeY(x) == 128));
    }
}

#[test]
#[serial]
fn talking_state_intro_mode_fade_and_input_pause_are_plain_settings() {
    let _comm = Comm::new();
    unsafe {
        assert_eq!(rust_IsTalkingFinished(), 0);
        rust_SetTalkingFinished(1);
        assert_eq!(rust_IsTalkingFinished(), 1);
        rust_SetTalkingFinished(0);
        assert_eq!(rust_IsTalkingFinished(), 0);

        for mode in 0..4 {
            rust_SetCommIntroMode(mode);
            assert_eq!(rust_GetCommIntroMode(), mode);
        }
        rust_SetCommIntroMode(77);
        assert_eq!(
            rust_GetCommIntroMode(),
            0,
            "unknown modes fall back to the default"
        );

        rust_SetCommFadeTime(250);
        assert_eq!(rust_GetCommFadeTime(), 250);

        assert_eq!(rust_IsCommInputPaused(), 0);
        rust_SetCommInputPaused(1);
        assert_eq!(rust_IsCommInputPaused(), 1);
        rust_SetCommInputPaused(0);
        assert_eq!(rust_IsCommInputPaused(), 0);
    }
}

#[test]
#[serial]
fn phrases_can_be_disabled_once_and_stay_disabled() {
    let _comm = Comm::new();
    unsafe {
        assert_eq!(rust_PhraseEnabled(5), 1);
        rust_DisablePhrase(5);
        assert_eq!(rust_PhraseEnabled(5), 0);
        assert_eq!(rust_PhraseEnabled(6), 1, "other phrases are unaffected");
    }
}

#[test]
#[serial]
fn the_segue_round_trips_and_maps_to_a_battle_outcome() {
    let _comm = Comm::new();
    unsafe {
        for (segue, battle) in [(0, 0), (1, 1), (2, 1), (3, 1)] {
            rust_SetSegue(segue);
            assert_eq!(rust_GetSegue(), segue);
            assert_eq!(rust_GetBattleSegue(), battle, "segue {segue}");
        }
        rust_SetSegue(99);
        assert_eq!(rust_GetSegue(), 0, "unknown segues mean peace");
    }
}

#[test]
#[serial]
fn updating_with_time_does_not_disturb_an_idle_conversation() {
    let _comm = Comm::new();
    unsafe {
        rust_UpdateCommunication(0.25);
        assert_eq!(rust_IsTalking(), 0);
        assert_eq!(rust_GetResponseCount(), 0);
    }
}

#[test]
#[serial]
fn the_speech_graphics_and_subtitle_display_refresh_without_a_screen() {
    let _comm = Comm::new();
    let line = CString::new("Subtitle text").unwrap();
    unsafe {
        rust_InitSpeechGraphics();
        rust_UpdateSpeechGraphics();
        rust_SpliceTrack(1, line.as_ptr(), 0.0, 3.0);
        assert_eq!(rust_StartTrack(), 1);
        rust_CheckSubtitles();
        rust_RedrawSubtitles();
        rust_ClearSubtitles();
        rust_RedrawSubtitles();
        offer("Choice", 1, None);
        rust_refresh_responses();
        assert_eq!(
            rust_GetResponseCount(),
            1,
            "refreshing the list leaves the responses alone"
        );
    }
}

#[test]
#[serial]
fn the_conversation_summary_pages_through_what_was_said() {
    let _comm = Comm::new();
    let line = CString::new("Something the alien said").unwrap();
    unsafe {
        assert_eq!(
            rust_show_conversation_summary(),
            1,
            "an empty history exits at once"
        );
        for i in 0..30 {
            rust_SpliceTrack(i + 1, line.as_ptr(), i as f32, 1.0);
        }
        assert_eq!(rust_show_conversation_summary(), 1);
    }
}
