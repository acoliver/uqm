//! The communication session driven the way the conversation code drives it:
//! tracks spliced and played, responses offered and chosen, animation and
//! oscilloscope state, and the encounter lifecycle.

use std::ffi::{c_int, c_uint, CStr, CString};
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

static CHOSEN: AtomicU32 = AtomicU32::new(0);

fn remember_choice(_comm: &mut Comm, _cx: &mut crate::engine::battle::Cx<'_>, reference: u32) {
    CHOSEN.store(reference, Ordering::SeqCst);
}

#[test]
#[serial]
fn the_system_is_initialised_cleared_and_uninitialised() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    comm.uninit();
    assert!(!comm.is_initialized());
    assert!(comm.init().is_ok());
    assert!(comm.is_initialized());
    offer(comm, "kept?", 1, None);
    comm.clear();
    assert_eq!(
        comm.responses().count() as c_int,
        0,
        "clearing drops the conversation"
    );
    comm.uninit();
    assert!(!comm.is_initialized());
}

#[test]
#[serial]
fn a_spliced_track_has_a_length_and_plays_from_the_start() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
    let line = CString::new("Greetings, Captain.").unwrap();
    assert!(!comm.track().is_playing());
    comm.track_mut()
        .splice_track(7, Some(line.to_str().unwrap()), 0.0, 2.0);
    comm.track_mut().splice_track(8, None, 2.0, 1.5);
    assert!(
        comm.track().length() >= 3.5 - 1e-3,
        "length {}",
        comm.track().length()
    );
    assert_eq!(comm.track().position(), 0.0);

    assert!(comm.start_track().is_ok());
    assert!(comm.track().is_playing());
    assert!(comm.is_talking());
    comm.stop_track();
    assert!(!comm.track().is_playing());
    assert!(!comm.is_talking());
}

#[test]
#[serial]
fn splicing_text_without_audio_extends_the_track() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
    let line = CString::new("Silent line").unwrap();
    let _: fn(&mut super::track::TrackManager, &str, f32, f32) =
        super::track::TrackManager::splice_text;
    let before = comm.track().length();
    assert_eq!(before, 0.0);
    assert!(comm.current_subtitle().is_none());
    comm.track_mut()
        .splice_text(line.to_str().unwrap(), 0.0, 2.0);
    assert!(comm.track().length() > before);
}

#[test]
#[serial]
fn an_audio_chunk_can_omit_its_subtitle() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
    comm.track_mut().splice_track(3, None, 0.0, 1.0);
    assert!(comm.track().length() >= 1.0 - 1e-3);
    assert!(comm.start_track().is_ok());
    assert!(
        comm.current_subtitle().is_none(),
        "there is nothing to display"
    );
}

#[test]
#[serial]
fn seeking_rewinding_and_clearing_move_the_play_position() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
    let line = CString::new("A long speech").unwrap();
    comm.track_mut()
        .splice_track(1, Some(line.to_str().unwrap()), 0.0, 10.0);
    assert!(comm.start_track().is_ok());
    comm.track_mut().seek(4.0);
    assert!(
        (comm.track().position() - 4.0).abs() < 1e-3,
        "{}",
        comm.track().position()
    );
    comm.track_mut().rewind();
    assert_eq!(comm.track().position(), 0.0);
    comm.track_mut().seek(6.5);
    assert!(
        (comm.track_mut().commit() - 6.5).abs() < 1e-3,
        "commit reports where the track is"
    );
    comm.track_mut().jump(0.0);
    assert!((0.0..=10.0).contains(&comm.track().position()));
    comm.track_mut().clear();
    assert_eq!(comm.track().length(), 0.0);
    assert!(!comm.track().is_playing());
}

#[test]
#[serial]
fn paging_moves_between_spliced_lines_and_smooth_scrubbing_moves_half_a_second() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
    let line = CString::new("One line of speech").unwrap();
    for (handle, start) in [(1, 0.0), (2, 10.0), (3, 20.0)] {
        comm.track_mut()
            .splice_track(handle, Some(line.to_str().unwrap()), start, 10.0);
    }
    assert!(comm.start_track().is_ok());
    assert!((comm.track().length() - 30.0).abs() < 1e-3);

    comm.track_mut().fast_forward_page();
    assert!((comm.track().position() - 10.0).abs() < 1e-3);
    comm.track_mut().fast_forward_page();
    assert!((comm.track().position() - 20.0).abs() < 1e-3);
    comm.track_mut().fast_forward_page();
    assert!(
        (comm.track().position() - 30.0).abs() < 1e-3,
        "past the last line is the end"
    );

    comm.track_mut().seek(25.0);
    comm.track_mut().fast_reverse_page();
    assert!(
        (comm.track().position() - 10.0).abs() < 1e-3,
        "back to the start of the previous line"
    );
    comm.track_mut().fast_reverse_page();
    assert!(comm.track().position().abs() < 1e-3);
    comm.track_mut().fast_reverse_page();
    assert!(
        comm.track().position().abs() < 1e-3,
        "nothing before the first line"
    );

    comm.track_mut().seek(5.0);
    comm.track_mut().fast_forward_smooth();
    assert!((comm.track().position() - 5.5).abs() < 1e-3);
    comm.track_mut().fast_reverse_smooth();
    comm.track_mut().fast_reverse_smooth();
    assert!((comm.track().position() - 4.5).abs() < 1e-3);
    comm.track_mut().seek(0.2);
    comm.track_mut().fast_reverse_smooth();
    assert!(
        comm.track().position().abs() < 1e-3,
        "scrubbing stops at the start"
    );
}

#[test]
#[serial]
fn waiting_on_a_track_is_over_only_once_it_has_stopped() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
    let line = CString::new("Speech").unwrap();
    comm.track_mut()
        .splice_track(1, Some(line.to_str().unwrap()), 0.0, 5.0);
    assert!(comm.start_track().is_ok());
    assert_eq!(c_int::from(comm.wait_track()), 0, "still talking");
    comm.stop_track();
    assert!(comm.wait_track());
}

#[test]
#[serial]
fn subtitles_can_be_switched_off_and_on() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());

    assert!(comm.subtitles().is_enabled());
    comm.subtitles_mut().set_enabled(0 != 0);
    assert!(!comm.subtitles().is_enabled());
    comm.subtitles_mut().set_enabled(5 != 0);
    assert!(
        comm.subtitles().is_enabled(),
        "any non-zero value turns them on"
    );
}

#[test]
#[serial]
fn the_current_subtitle_is_borrowed_from_its_conversation() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
    let line = CString::new("We come in peace.").unwrap();
    assert!(comm.current_subtitle().is_none());
    comm.track_mut()
        .splice_track(1, Some(line.to_str().unwrap()), 0.0, 3.0);
    assert!(comm.start_track().is_ok());
    assert_eq!(comm.current_subtitle(), Some("We come in peace."));
}

#[test]
#[serial]
fn responses_are_offered_displayed_and_selected_with_wraparound() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
    assert_eq!(offer(comm, "Who are you?", 10, None), 1);
    assert_eq!(offer(comm, "Goodbye.", 20, None), 1);
    assert_eq!(offer(comm, "Give me your ship.", 30, None), 1);

    assert_eq!(comm.responses().count() as c_int, 3);
    comm.display_responses();
    assert_eq!(comm.selected_response(), 0);

    assert!(comm.select_next_response());
    assert_eq!(comm.selected_response(), 1);
    assert!(comm.select_next_response());
    assert!(comm.select_next_response());
    assert_eq!(comm.selected_response(), 0, "next wraps to the first");
    assert!(comm.select_prev_response());
    assert_eq!(comm.selected_response(), 2, "previous wraps to the last");

    assert!(comm.responses_mut().select(1));
    assert_eq!(comm.selected_response(), 1);
    assert!(!comm.responses_mut().select(3));
    assert!(!comm.responses_mut().select(-1));
    assert_eq!(comm.selected_response(), 1, "a bad index changes nothing");
}

#[test]
#[serial]
fn there_is_nothing_to_select_without_responses() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());

    assert!(!comm.select_next_response());
    assert!(!comm.select_prev_response());
    assert!(!comm.responses_mut().select(0));
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
    assert!(comm.init().is_ok());
    CHOSEN.store(0, Ordering::SeqCst);
    offer(comm, "First", 11, None);
    offer(comm, "Second", 22, Some(remember_choice));

    assert!(comm.responses_mut().select(1));
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
    assert!(comm.init().is_ok());
    CHOSEN.store(0, Ordering::SeqCst);
    offer(comm, "Plain", 33, None);

    comm.display_responses();
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
    assert!(comm.init().is_ok());
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
    assert_eq!(comm.responses().count() as c_int, 0);
}

#[test]
#[serial]
fn response_text_is_borrowed_without_a_caller_buffer() {
    let mut comm = Comm::new();
    assert!(comm.init().is_ok());
    offer(&mut comm, "Tell me about your homeworld.", 5, None);
    assert_eq!(
        comm.responses().get(0).unwrap().response_text,
        "Tell me about your homeworld."
    );
    assert!(comm.responses().get(1).is_none());
    assert!(comm.responses().get(usize::MAX).is_none());
}

#[test]
#[serial]
fn clearing_responses_empties_the_list() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
    offer(comm, "a", 1, None);
    offer(comm, "b", 2, None);

    rust_clear_responses(comm);
    assert_eq!(comm.responses().count() as c_int, 0);
    assert_eq!(
        crate::comm::with_test_context(|cx| rust_execute_response(comm, cx)),
        0
    );
}

#[test]
#[serial]
fn the_encounter_lifecycle_begins_and_ends_in_order() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());

    if crate::comm::encounter::is_encounter_active(comm) as c_int == 1 {
        assert!(crate::comm::encounter::end_encounter_abort(comm).is_ok());
    }
    assert_eq!(
        crate::comm::encounter::is_encounter_active(comm) as c_int,
        0
    );
    assert!(
        !crate::comm::encounter::end_encounter_normal(comm).is_ok(),
        "nothing to end"
    );
    assert!(crate::comm::encounter::end_encounter_abort(comm).is_err());

    assert!(crate::comm::encounter::begin_encounter(comm).is_ok());
    assert_eq!(
        crate::comm::encounter::is_encounter_active(comm) as c_int,
        1
    );
    assert!(
        !crate::comm::encounter::begin_encounter(comm).is_ok(),
        "one encounter at a time"
    );
    assert!(crate::comm::encounter::end_encounter_normal(comm).is_ok());
    assert_eq!(
        crate::comm::encounter::is_encounter_active(comm) as c_int,
        0
    );

    assert!(crate::comm::encounter::begin_encounter(comm).is_ok());
    assert!(crate::comm::encounter::end_encounter_abort(comm).is_ok());
    assert_eq!(
        crate::comm::encounter::is_encounter_active(comm) as c_int,
        0
    );

    assert!(
        crate::comm::encounter::end_encounter_attack(comm).is_ok(),
        "attacking without hailing still ends cleanly"
    );
    assert_eq!(
        crate::comm::encounter::is_encounter_active(comm) as c_int,
        0
    );
}

#[test]
#[serial]
fn the_oscilloscope_shows_samples_and_clears_back_to_the_centre_line() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
    let loud: Vec<i16> = vec![20_000; 2048];
    assert_eq!(comm.oscilloscope().get_y(10), 128);
    comm.add_oscilloscope_samples(&loud[..0]);
    comm.add_oscilloscope_samples(&loud);
    comm.oscilloscope_mut().update();
    let drawn: Vec<u8> = (0..128)
        .map(|x| comm.oscilloscope().get_y(x as usize))
        .collect();
    assert!(
        drawn.iter().any(|&y| y != 128),
        "the waveform moves off the centre line"
    );
    assert_eq!(
        comm.oscilloscope().get_y(100_000),
        128,
        "off the display is the centre line"
    );
    comm.oscilloscope_mut().clear();
    assert!((0..128).all(|x| comm.oscilloscope().get_y(x as usize) == 128));
}

#[test]
#[serial]
fn talking_state_intro_mode_fade_and_input_pause_are_plain_settings() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());

    assert!(!comm.is_talking_finished());
    comm.set_talking_finished(1 != 0);
    assert!(comm.is_talking_finished());
    comm.set_talking_finished(0 != 0);
    assert!(!comm.is_talking_finished());

    for mode in 0..4 {
        comm.set_intro_mode(super::CommIntroMode::from(mode));
        assert_eq!(comm.intro_mode() as u32, mode);
    }
    comm.set_intro_mode(super::CommIntroMode::from(77));
    assert_eq!(
        comm.intro_mode() as u32,
        0,
        "unknown modes fall back to the default"
    );

    comm.set_fade_time(250);
    assert_eq!(comm.fade_time(), 250);

    assert!(!comm.is_input_paused());
    comm.set_input_paused(1 != 0);
    assert!(comm.is_input_paused());
    comm.set_input_paused(0 != 0);
    assert!(!comm.is_input_paused());
}

#[test]
#[serial]
fn phrases_can_be_disabled_once_and_stay_disabled() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());

    assert_eq!(i32::from(comm.phrase_enabled(5)), 1);
    comm.disable_phrase(5);
    assert_eq!(i32::from(comm.phrase_enabled(5)), 0);
    assert_eq!(
        i32::from(comm.phrase_enabled(6)),
        1,
        "other phrases are unaffected"
    );
}

#[test]
#[serial]
fn the_segue_round_trips_and_maps_to_a_battle_outcome() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());

    for (segue, battle) in [(0, 0), (1, 1), (2, 1), (3, 1)] {
        rust_set_segue(comm, segue);
        assert_eq!(u32::from(comm.get_segue()), segue);
        assert_eq!(comm.get_segue().to_battle_segue(), battle, "segue {segue}");
    }
    rust_set_segue(comm, 99);
    assert_eq!(u32::from(comm.get_segue()), 0, "unknown segues mean peace");
}

#[test]
#[serial]
fn updating_with_time_does_not_disturb_an_idle_conversation() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());

    comm.update(0.25);
    assert!(!comm.is_talking());
    assert_eq!(comm.responses().count() as c_int, 0);
}

#[test]
#[serial]
fn the_speech_graphics_and_subtitle_display_refresh_on_the_real_renderer() {
    super::sis_graphics_font_tests::with_display("comm::ffi_behavior_tests::the_speech_graphics_and_subtitle_display_refresh_on_the_real_renderer", |comm, _| {
    let line = CString::new("Subtitle text").unwrap();
    rust_init_speech_graphics(comm);
    rust_update_speech_graphics(comm);
    comm.track_mut()
        .splice_track(1, Some(line.to_str().unwrap()), 0.0, 3.0);
    assert!(comm.start_track().is_ok());
    let current = comm.current_subtitle().map(|s| s.to_owned());
    comm.subtitle_display_mut().check_subtitle(current.as_deref());
    crate::comm::subtitle_display::SubtitleDisplay::redraw(
        comm,
        crate::engine::input::InputSession::default().preferences().subtitles != 0,
    );
    rust_clear_subtitles(comm);
    crate::comm::subtitle_display::SubtitleDisplay::redraw(
        comm,
        crate::engine::input::InputSession::default().preferences().subtitles != 0,
    );
    offer(comm, "Choice", 1, None);
    comm.refresh_response_ui(crate::engine::World::new().activity());
    assert_eq!(
        comm.responses().count() as c_int,
        1,
        "refreshing the list leaves the responses alone"
    );    });
}

#[test]
#[serial]
fn the_conversation_summary_pages_through_what_was_said() {
    let mut owner = Comm::new();
    let comm = &mut owner;
    assert!(comm.init().is_ok());
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
    assert_eq!(
        rust_show_conversation_summary(comm, &mut cx, &mut io),
        1,
        "an empty history exits at once"
    );
    for i in 0..30 {
        comm.track_mut()
            .splice_track(i + 1, Some(line.to_str().unwrap()), i as f32, 1.0);
    }
    assert_eq!(rust_show_conversation_summary(comm, &mut cx, &mut io), 1);
}

#[test]
fn state_accessors_use_safe_rust_signatures() {
    let _: fn(&Comm) -> Option<&str> = Comm::current_subtitle;
    let _: fn(&mut Comm, &mut crate::engine::battle::Cx<'_>) -> c_uint = rust_execute_response;
    let _: fn(&mut Comm) = rust_init_speech_graphics;
    let _: fn(&mut Comm) = rust_update_speech_graphics;
    let _: fn(&mut Comm) = rust_clear_subtitles;

    let _: fn(&mut Comm) = rust_clear_responses;
    let _: fn(&Comm) -> super::CommIntroMode = Comm::intro_mode;
    let _: fn(&mut Comm, super::CommIntroMode) = Comm::set_intro_mode;
    let _: fn(&Comm) -> u32 = Comm::fade_time;
    let _: fn(&mut Comm, u32) = Comm::set_fade_time;
    let _: fn(&Comm, i32) -> bool = Comm::phrase_enabled;
    let _: fn(&mut Comm, i32) = Comm::disable_phrase;
    let _: fn(&mut Comm, segue: c_uint) = rust_set_segue;
}
