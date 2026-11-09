//! The communication facade driven the way the C conversation code drives it:
//! tracks spliced and played, responses offered and chosen, animation and
//! oscilloscope state, and the encounter lifecycle, all through the exported
//! `rust_*` entry points.

use std::ffi::{c_char, c_int, c_uint, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use serial_test::serial;

use super::ffi::*;

use super::Comm;

fn offer(
    comm: &mut Comm,
    text: &str,
    reference: u32,
    func: Option<crate::comm::response::ResponseFunc>,
) -> c_int {
    let text = CString::new(text).unwrap();
    do_response_phrase(comm, reference, Some(text.as_c_str()), func)
}

fn response_text(comm: &mut Comm, index: c_int, capacity: c_int) -> (c_int, String) {
    let mut buf = vec![0x7fu8; 64];
    let ok =
        unsafe { rust_get_response_text(comm, index, buf.as_mut_ptr() as *mut c_char, capacity) };
    let text = CStr::from_bytes_until_nul(&buf)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    (ok, text)
}

static CHOSEN: AtomicU32 = AtomicU32::new(0);

fn remember_choice(_comm: &mut Comm, _cx: &mut crate::engine::battle::Cx<'_>, reference: u32) {
    CHOSEN.store(reference, Ordering::SeqCst);
}

#[test]
#[serial]
fn the_system_is_initialised_cleared_and_uninitialised() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    rust_uninit_communication(comm);
    assert_eq!(rust_is_comm_initialized(comm), 0);
    assert_eq!(rust_init_communication(comm), 1);
    assert_eq!(rust_is_comm_initialized(comm), 1);
    offer(comm, "kept?", 1, None);
    rust_clear_communication(comm);
    assert_eq!(
        rust_get_response_count(comm),
        0,
        "clearing drops the conversation"
    );
    rust_uninit_communication(comm);
    assert_eq!(rust_is_comm_initialized(comm), 0);
}

#[test]
#[serial]
fn a_spliced_track_has_a_length_and_plays_from_the_start() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let line = CString::new("Greetings, Captain.").unwrap();
    unsafe {
        assert_eq!(rust_playing_track(comm), 0);
        rust_splice_track(comm, 7, line.as_ptr(), 0.0, 2.0);
        rust_splice_track(comm, 8, ptr::null(), 2.0, 1.5);
        assert!(
            rust_get_track_length(comm) >= 3.5 - 1e-3,
            "length {}",
            rust_get_track_length(comm)
        );
        assert_eq!(rust_get_track_position(comm), 0.0);

        assert_eq!(rust_start_track(comm), 1);
        assert_eq!(rust_playing_track(comm), 1);
        assert_eq!(rust_is_talking(comm), 1);
        rust_stop_track(comm);
        assert_eq!(rust_playing_track(comm), 0);
        assert_eq!(rust_is_talking(comm), 0);
    }
}

#[test]
#[serial]
fn splicing_text_without_audio_extends_the_track() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let line = CString::new("Silent line").unwrap();
    let bad = [0xffu8, 0xfe, 0];
    unsafe {
        let before = rust_get_track_length(comm);
        rust_splice_track_text(comm, ptr::null(), 0.0, 1.0);
        rust_splice_track_text(comm, bad.as_ptr() as *const c_char, 0.0, 1.0);
        assert_eq!(
            rust_get_track_length(comm),
            before,
            "null and undecodable text add nothing"
        );
        rust_splice_track_text(comm, line.as_ptr(), 0.0, 2.0);
        assert!(rust_get_track_length(comm) > before);
    }
}

#[test]
#[serial]
fn an_undecodable_subtitle_is_spliced_as_no_subtitle() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let bad = [0xffu8, 0xfe, 0];
    unsafe {
        rust_splice_track(comm, 3, bad.as_ptr() as *const c_char, 0.0, 1.0);
        assert!(rust_get_track_length(comm) >= 1.0 - 1e-3);
        assert_eq!(rust_start_track(comm), 1);
        assert!(
            rust_get_subtitle(comm).is_null(),
            "there is nothing to display"
        );
    }
}

#[test]
#[serial]
fn seeking_rewinding_and_clearing_move_the_play_position() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let line = CString::new("A long speech").unwrap();
    unsafe {
        rust_splice_track(comm, 1, line.as_ptr(), 0.0, 10.0);
        assert_eq!(rust_start_track(comm), 1);
        rust_seek_track(comm, 4.0);
        assert!(
            (rust_get_track_position(comm) - 4.0).abs() < 1e-3,
            "{}",
            rust_get_track_position(comm)
        );
        rust_rewind_track(comm);
        assert_eq!(rust_get_track_position(comm), 0.0);
        rust_seek_track(comm, 6.5);
        assert!(
            (rust_commit_track(comm) - 6.5).abs() < 1e-3,
            "commit reports where the track is"
        );
        rust_jump_track(comm);
        assert!((0.0..=10.0).contains(&rust_get_track_position(comm)));
        rust_clear_track(comm);
        assert_eq!(rust_get_track_length(comm), 0.0);
        assert_eq!(rust_playing_track(comm), 0);
    }
}

#[test]
#[serial]
fn paging_moves_between_spliced_lines_and_smooth_scrubbing_moves_half_a_second() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let line = CString::new("One line of speech").unwrap();
    unsafe {
        for (handle, start) in [(1, 0.0), (2, 10.0), (3, 20.0)] {
            rust_splice_track(comm, handle, line.as_ptr(), start, 10.0);
        }
        assert_eq!(rust_start_track(comm), 1);
        assert!((rust_get_track_length(comm) - 30.0).abs() < 1e-3);

        rust_fast_forward_page(comm);
        assert!((rust_get_track_position(comm) - 10.0).abs() < 1e-3);
        rust_fast_forward_page(comm);
        assert!((rust_get_track_position(comm) - 20.0).abs() < 1e-3);
        rust_fast_forward_page(comm);
        assert!(
            (rust_get_track_position(comm) - 30.0).abs() < 1e-3,
            "past the last line is the end"
        );

        rust_seek_track(comm, 25.0);
        rust_fast_reverse_page(comm);
        assert!(
            (rust_get_track_position(comm) - 10.0).abs() < 1e-3,
            "back to the start of the previous line"
        );
        rust_fast_reverse_page(comm);
        assert!(rust_get_track_position(comm).abs() < 1e-3);
        rust_fast_reverse_page(comm);
        assert!(
            rust_get_track_position(comm).abs() < 1e-3,
            "nothing before the first line"
        );

        rust_seek_track(comm, 5.0);
        rust_fast_forward_smooth(comm);
        assert!((rust_get_track_position(comm) - 5.5).abs() < 1e-3);
        rust_fast_reverse_smooth(comm);
        rust_fast_reverse_smooth(comm);
        assert!((rust_get_track_position(comm) - 4.5).abs() < 1e-3);
        rust_seek_track(comm, 0.2);
        rust_fast_reverse_smooth(comm);
        assert!(
            rust_get_track_position(comm).abs() < 1e-3,
            "scrubbing stops at the start"
        );
    }
}

#[test]
#[serial]
fn waiting_on_a_track_is_over_only_once_it_has_stopped() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let line = CString::new("Speech").unwrap();
    unsafe {
        rust_splice_track(comm, 1, line.as_ptr(), 0.0, 5.0);
        assert_eq!(rust_start_track(comm), 1);
        assert_eq!(rust_wait_track(comm), 0, "still talking");
        rust_stop_track(comm);
        assert_eq!(rust_wait_track(comm), 1);
    }
}

#[test]
#[serial]
fn subtitles_can_be_switched_off_and_on() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);

    assert_eq!(rust_are_subtitles_enabled(comm), 1);
    rust_set_subtitles_enabled(comm, 0);
    assert_eq!(rust_are_subtitles_enabled(comm), 0);
    rust_set_subtitles_enabled(comm, 5);
    assert_eq!(
        rust_are_subtitles_enabled(comm),
        1,
        "any non-zero value turns them on"
    );
}

#[test]
#[serial]
fn the_current_subtitle_is_handed_to_c_as_a_terminated_string() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let line = CString::new("We come in peace.").unwrap();
    unsafe {
        assert!(rust_get_subtitle(comm).is_null());
        rust_splice_track(comm, 1, line.as_ptr(), 0.0, 3.0);
        assert_eq!(rust_start_track(comm), 1);
        let ptr = rust_get_subtitle(comm);
        assert!(!ptr.is_null());
        assert_eq!(CStr::from_ptr(ptr).to_str().unwrap(), "We come in peace.");
    }
}

#[test]
#[serial]
fn responses_are_offered_displayed_and_selected_with_wraparound() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    assert_eq!(offer(comm, "Who are you?", 10, None), 1);
    assert_eq!(offer(comm, "Goodbye.", 20, None), 1);
    assert_eq!(offer(comm, "Give me your ship.", 30, None), 1);

    assert_eq!(rust_get_response_count(comm), 3);
    rust_display_responses(comm);
    assert_eq!(rust_get_selected_response(comm), 0);

    assert_eq!(rust_select_next_response(comm), 1);
    assert_eq!(rust_get_selected_response(comm), 1);
    assert_eq!(rust_select_next_response(comm), 1);
    assert_eq!(rust_select_next_response(comm), 1);
    assert_eq!(
        rust_get_selected_response(comm),
        0,
        "next wraps to the first"
    );
    assert_eq!(rust_select_prev_response(comm), 1);
    assert_eq!(
        rust_get_selected_response(comm),
        2,
        "previous wraps to the last"
    );

    assert_eq!(rust_select_response_index(comm, 1), 1);
    assert_eq!(rust_get_selected_response(comm), 1);
    assert_eq!(rust_select_response_index(comm, 3), 0);
    assert_eq!(rust_select_response_index(comm, -1), 0);
    assert_eq!(
        rust_get_selected_response(comm),
        1,
        "a bad index changes nothing"
    );
}

#[test]
#[serial]
fn there_is_nothing_to_select_without_responses() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);

    assert_eq!(rust_select_next_response(comm), 0);
    assert_eq!(rust_select_prev_response(comm), 0);
    assert_eq!(rust_select_response_index(comm, 0), 0);
    assert_eq!(
        crate::comm::with_test_context(|cx| rust_execute_response(comm, cx)),
        0
    );
}

#[test]
#[serial]
fn a_response_with_a_handler_calls_it_with_its_reference() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    CHOSEN.store(0, Ordering::SeqCst);
    offer(comm, "First", 11, None);
    offer(comm, "Second", 22, Some(remember_choice));

    assert_eq!(rust_select_response_index(comm, 1), 1);
    assert_eq!(
        crate::comm::with_test_context(|cx| rust_execute_response(comm, cx)),
        22
    );

    assert_eq!(CHOSEN.load(Ordering::SeqCst), 22);
}

#[test]
#[serial]
fn a_response_without_a_handler_just_reports_its_reference() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    CHOSEN.store(0, Ordering::SeqCst);
    offer(comm, "Plain", 33, None);

    rust_display_responses(comm);
    assert_eq!(
        crate::comm::with_test_context(|cx| rust_execute_response(comm, cx)),
        33
    );

    assert_eq!(CHOSEN.load(Ordering::SeqCst), 0);
}

#[test]
#[serial]
fn offering_a_response_needs_readable_text() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let bad = [0xffu8, 0xfe, 0];

    assert_eq!(do_response_phrase(comm, 1, None, None), 0);
    assert_eq!(
        do_response_phrase(
            comm,
            1,
            Some(CStr::from_bytes_with_nul(&bad).unwrap()),
            None
        ),
        0
    );
    assert_eq!(rust_get_response_count(comm), 0);
}

#[test]
#[serial]
fn response_text_is_copied_out_and_truncated_to_the_buffer() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    offer(comm, "Tell me about your homeworld.", 5, None);
    assert_eq!(
        response_text(comm, 0, 64),
        (1, "Tell me about your homeworld.".to_owned())
    );
    assert_eq!(
        response_text(comm, 0, 8),
        (1, "Tell me".to_owned()),
        "room for seven characters and the terminator"
    );
    assert_eq!(response_text(comm, 0, 1), (1, String::new()));
}

#[test]
#[serial]
fn response_text_refuses_bad_requests() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    offer(comm, "Only one", 5, None);
    let mut buf = [0x55u8; 8];
    unsafe {
        assert_eq!(rust_get_response_text(comm, 0, ptr::null_mut(), 8), 0);
        assert_eq!(
            rust_get_response_text(comm, 0, buf.as_mut_ptr() as *mut c_char, 0),
            0
        );
        assert_eq!(
            rust_get_response_text(comm, -1, buf.as_mut_ptr() as *mut c_char, 8),
            0
        );
        assert_eq!(
            rust_get_response_text(comm, 1, buf.as_mut_ptr() as *mut c_char, 8),
            0
        );
    }
    assert_eq!(buf, [0x55; 8], "the caller's buffer is untouched");
}

#[test]
#[serial]
fn clearing_responses_empties_the_list() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    offer(comm, "a", 1, None);
    offer(comm, "b", 2, None);

    rust_clear_responses(comm);
    assert_eq!(rust_get_response_count(comm), 0);
    assert_eq!(
        crate::comm::with_test_context(|cx| rust_execute_response(comm, cx)),
        0
    );
}

#[test]
#[serial]
fn intro_and_talking_animation_flags_follow_their_setters() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);

    rust_init_comm_animations(comm);
    assert_eq!(rust_running_intro_anim(comm), 0);
    rust_set_run_intro_anim(comm, 1);
    assert_eq!(rust_running_intro_anim(comm), 1);
    rust_set_run_intro_anim(comm, 0);
    assert_eq!(rust_running_intro_anim(comm), 0);

    assert_eq!(rust_running_talking_anim(comm), 0);
    rust_set_run_talking_anim(comm, 1);
    assert_eq!(rust_running_talking_anim(comm), 1);
    rust_set_stop_talking_anim(comm);
    assert_eq!(
        rust_running_talking_anim(comm),
        1,
        "stopping is a request the animation loop honours on its next step"
    );
    assert_eq!(
        rust_want_talking_anim(comm),
        0,
        "no talking sequence is defined"
    );
    assert_eq!(rust_have_talking_anim(comm), 0, "so none can be active");
    assert_eq!(
        rust_get_comm_animation_frame(comm, 99),
        0,
        "no such animation"
    );
}

#[test]
#[serial]
fn animation_callbacks_report_a_change_on_redraw_and_none_when_paused() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);

    assert_eq!(
        rust_process_comm_animations_cb(comm, 0, 1),
        0,
        "paused advances nothing"
    );
    assert_eq!(
        rust_process_comm_animations_cb(comm, 1, 1),
        0,
        "paused even on a full redraw"
    );
    assert_eq!(
        rust_process_comm_animations_cb(comm, 1, 0),
        1,
        "a full redraw always reports a change"
    );
    assert_eq!(
        rust_process_comm_animations_cb(comm, 0, 0),
        0,
        "no animations means no change"
    );
    assert_eq!(rust_process_comm_animations(comm, 4), 0);
}

#[test]
#[serial]
fn the_encounter_lifecycle_begins_and_ends_in_order() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);

    if rust_is_encounter_active(comm) == 1 {
        rust_end_encounter_abort(comm);
    }
    assert_eq!(rust_is_encounter_active(comm), 0);
    assert_eq!(rust_end_encounter_normal(comm), 0, "nothing to end");
    assert_eq!(rust_end_encounter_abort(comm), 0);

    assert_eq!(rust_begin_encounter(comm), 1);
    assert_eq!(rust_is_encounter_active(comm), 1);
    assert_eq!(rust_begin_encounter(comm), 0, "one encounter at a time");
    assert_eq!(rust_end_encounter_normal(comm), 1);
    assert_eq!(rust_is_encounter_active(comm), 0);

    assert_eq!(rust_begin_encounter(comm), 1);
    assert_eq!(rust_end_encounter_abort(comm), 1);
    assert_eq!(rust_is_encounter_active(comm), 0);

    assert_eq!(
        rust_end_encounter_attack(comm),
        1,
        "attacking without hailing still ends cleanly"
    );
    assert_eq!(rust_is_encounter_active(comm), 0);
}

#[test]
#[serial]
fn the_oscilloscope_shows_samples_and_clears_back_to_the_centre_line() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let loud: Vec<i16> = vec![20_000; 2048];
    unsafe {
        assert_eq!(rust_get_oscilloscope_y(comm, 10), 128);
        rust_add_oscilloscope_samples(comm, ptr::null(), 4);
        rust_add_oscilloscope_samples(comm, loud.as_ptr(), 0);
        rust_add_oscilloscope_samples(comm, loud.as_ptr(), loud.len() as u32);
        rust_update_oscilloscope(comm);
        let drawn: Vec<u8> = (0..128).map(|x| rust_get_oscilloscope_y(comm, x)).collect();
        assert!(
            drawn.iter().any(|&y| y != 128),
            "the waveform moves off the centre line"
        );
        assert_eq!(
            rust_get_oscilloscope_y(comm, 100_000),
            128,
            "off the display is the centre line"
        );
        rust_clear_oscilloscope(comm);
        assert!((0..128).all(|x| rust_get_oscilloscope_y(comm, x) == 128));
    }
}

#[test]
#[serial]
fn talking_state_intro_mode_fade_and_input_pause_are_plain_settings() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);

    assert_eq!(rust_is_talking_finished(comm), 0);
    rust_set_talking_finished(comm, 1);
    assert_eq!(rust_is_talking_finished(comm), 1);
    rust_set_talking_finished(comm, 0);
    assert_eq!(rust_is_talking_finished(comm), 0);

    for mode in 0..4 {
        rust_set_comm_intro_mode(comm, mode);
        assert_eq!(rust_get_comm_intro_mode(comm), mode);
    }
    rust_set_comm_intro_mode(comm, 77);
    assert_eq!(
        rust_get_comm_intro_mode(comm),
        0,
        "unknown modes fall back to the default"
    );

    rust_set_comm_fade_time(comm, 250);
    assert_eq!(rust_get_comm_fade_time(comm), 250);

    assert_eq!(rust_is_comm_input_paused(comm), 0);
    rust_set_comm_input_paused(comm, 1);
    assert_eq!(rust_is_comm_input_paused(comm), 1);
    rust_set_comm_input_paused(comm, 0);
    assert_eq!(rust_is_comm_input_paused(comm), 0);
}

#[test]
#[serial]
fn phrases_can_be_disabled_once_and_stay_disabled() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);

    assert_eq!(rust_phrase_enabled(comm, 5), 1);
    rust_disable_phrase(comm, 5);
    assert_eq!(rust_phrase_enabled(comm, 5), 0);
    assert_eq!(
        rust_phrase_enabled(comm, 6),
        1,
        "other phrases are unaffected"
    );
}

#[test]
#[serial]
fn the_segue_round_trips_and_maps_to_a_battle_outcome() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);

    for (segue, battle) in [(0, 0), (1, 1), (2, 1), (3, 1)] {
        rust_set_segue(comm, segue);
        assert_eq!(rust_get_segue(comm), segue);
        assert_eq!(rust_get_battle_segue(comm), battle, "segue {segue}");
    }
    rust_set_segue(comm, 99);
    assert_eq!(rust_get_segue(comm), 0, "unknown segues mean peace");
}

#[test]
#[serial]
fn updating_with_time_does_not_disturb_an_idle_conversation() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);

    rust_update_communication(comm, 0.25);
    assert_eq!(rust_is_talking(comm), 0);
    assert_eq!(rust_get_response_count(comm), 0);
}

#[test]
#[serial]
fn the_speech_graphics_and_subtitle_display_refresh_without_a_screen() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let line = CString::new("Subtitle text").unwrap();
    unsafe {
        rust_init_speech_graphics(comm);
        rust_update_speech_graphics(comm);
        rust_splice_track(comm, 1, line.as_ptr(), 0.0, 3.0);
        assert_eq!(rust_start_track(comm), 1);
        rust_check_subtitles(comm);
        rust_redraw_subtitles(comm, &crate::engine::input::InputSession::default());
        rust_clear_subtitles(comm);
        rust_redraw_subtitles(comm, &crate::engine::input::InputSession::default());
        offer(comm, "Choice", 1, None);
        rust_refresh_responses(comm, &crate::engine::World::new());
        assert_eq!(
            rust_get_response_count(comm),
            1,
            "refreshing the list leaves the responses alone"
        );
    }
}

#[test]
#[serial]
fn the_conversation_summary_pages_through_what_was_said() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert_eq!(rust_init_communication(comm), 1);
    let line = CString::new("Something the alien said").unwrap();
    let mut game = crate::engine::Game::new();
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = crate::engine::battle::Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        crate::math::random,
    );
    let mut observer = crate::engine::observe::Unobserved;
    let mut io = crate::engine::observe::Io {
        observer: &mut observer,
    };
    unsafe {
        assert_eq!(
            rust_show_conversation_summary(comm, &mut cx, &mut io),
            1,
            "an empty history exits at once"
        );
        for i in 0..30 {
            rust_splice_track(comm, i + 1, line.as_ptr(), i as f32, 1.0);
        }
        assert_eq!(rust_show_conversation_summary(comm, &mut cx, &mut io), 1);
    }
}

#[test]
fn state_accessors_use_safe_rust_signatures() {
    let _: fn(&mut Comm) -> *const c_char = rust_get_subtitle;
    let _: fn(&mut Comm, &mut crate::engine::battle::Cx<'_>) -> c_uint = rust_execute_response;
    let _: fn(&mut Comm) = rust_init_speech_graphics;
    let _: fn(&mut Comm) = rust_update_speech_graphics;
    let _: fn(&mut Comm, &crate::engine::World) = rust_refresh_responses;
    let _: fn(&mut Comm) = rust_clear_subtitles;
    let _: fn(&mut Comm) = rust_check_subtitles;
    let _: fn(&mut Comm, &crate::engine::input::InputSession) = rust_redraw_subtitles;

    let _: fn(&mut Comm) -> c_int = rust_init_communication;
    let _: fn(&mut Comm) = rust_uninit_communication;
    let _: fn(&mut Comm) -> c_int = rust_is_comm_initialized;
    let _: fn(&mut Comm) = rust_clear_communication;
    let _: fn(&mut Comm) -> c_int = rust_start_track;
    let _: fn(&mut Comm) = rust_stop_track;
    let _: fn(&mut Comm) = rust_rewind_track;
    let _: fn(&mut Comm) = rust_jump_track;
    let _: fn(&mut Comm, position: f32) = rust_seek_track;
    let _: fn(&mut Comm) -> f32 = rust_commit_track;
    let _: fn(&mut Comm) -> c_int = rust_wait_track;
    let _: fn(&mut Comm) -> f32 = rust_get_track_position;
    let _: fn(&mut Comm) -> f32 = rust_get_track_length;
    let _: fn(&mut Comm) = rust_clear_track;
    let _: fn(&mut Comm) -> c_uint = rust_playing_track;
    let _: fn(&mut Comm) = rust_fast_forward_page;
    let _: fn(&mut Comm) = rust_fast_forward_smooth;
    let _: fn(&mut Comm) = rust_fast_reverse_page;
    let _: fn(&mut Comm) = rust_fast_reverse_smooth;
    let _: fn(&mut Comm, enabled: c_int) = rust_set_subtitles_enabled;
    let _: fn(&mut Comm) -> c_int = rust_are_subtitles_enabled;
    let _: fn(&mut Comm) = rust_display_responses;
    let _: fn(&mut Comm) = rust_clear_responses;
    let _: fn(&mut Comm) -> c_int = rust_select_next_response;
    let _: fn(&mut Comm) -> c_int = rust_select_prev_response;
    let _: fn(&mut Comm) -> c_int = rust_get_selected_response;
    let _: fn(&mut Comm, index: c_int) -> c_int = rust_select_response_index;
    let _: fn(&mut Comm) -> c_int = rust_get_response_count;
    let _: fn(&mut Comm) = rust_init_comm_animations;
    let _: fn(&mut Comm, delta_ticks: c_uint) -> c_int = rust_process_comm_animations;
    let _: fn(&mut Comm, clear: c_int, paused: c_int) -> c_int = rust_process_comm_animations_cb;
    let _: fn(&mut Comm) -> c_int = rust_want_talking_anim;
    let _: fn(&mut Comm) -> c_int = rust_have_talking_anim;
    let _: fn(&mut Comm, _run: c_int) = rust_set_run_talking_anim;
    let _: fn(&mut Comm) = rust_set_stop_talking_anim;
    let _: fn(&mut Comm, run: c_int) = rust_set_run_intro_anim;
    let _: fn(&mut Comm) -> c_int = rust_running_intro_anim;
    let _: fn(&mut Comm) -> c_int = rust_running_talking_anim;
    let _: fn(&mut Comm, index: c_uint) -> c_uint = rust_get_comm_animation_frame;
    let _: fn(&mut Comm) -> c_int = rust_begin_encounter;
    let _: fn(&mut Comm) -> c_int = rust_end_encounter_normal;
    let _: fn(&mut Comm) -> c_int = rust_end_encounter_abort;
    let _: fn(&mut Comm) -> c_int = rust_end_encounter_attack;
    let _: fn(&mut Comm) -> c_int = rust_is_encounter_active;
    let _: fn(&mut Comm) = rust_update_oscilloscope;
    let _: fn(&mut Comm, x: c_uint) -> u8 = rust_get_oscilloscope_y;
    let _: fn(&mut Comm) = rust_clear_oscilloscope;
    let _: fn(&mut Comm) -> c_int = rust_is_talking;
    let _: fn(&mut Comm) -> c_int = rust_is_talking_finished;
    let _: fn(&mut Comm, finished: c_int) = rust_set_talking_finished;
    let _: fn(&mut Comm) -> c_uint = rust_get_comm_intro_mode;
    let _: fn(&mut Comm, mode: c_uint) = rust_set_comm_intro_mode;
    let _: fn(&mut Comm) -> c_uint = rust_get_comm_fade_time;
    let _: fn(&mut Comm, time: c_uint) = rust_set_comm_fade_time;
    let _: fn(&mut Comm) -> c_int = rust_is_comm_input_paused;
    let _: fn(&mut Comm, paused: c_int) = rust_set_comm_input_paused;
    let _: fn(&mut Comm, delta_time: f32) = rust_update_communication;
    let _: fn(&mut Comm, index: c_int) -> c_int = rust_phrase_enabled;
    let _: fn(&mut Comm, index: c_int) = rust_disable_phrase;
    let _: fn(&mut Comm, segue: c_uint) = rust_set_segue;
    let _: fn(&mut Comm) -> c_uint = rust_get_segue;
    let _: fn(&mut Comm) -> c_uint = rust_get_battle_segue;
}
