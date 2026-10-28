//! Behavioural tests for the track player: assembling a conversation from
//! spliced tracks, playing and seeking through it, and subtitle navigation.
//!
//! The track player and the stream engine share process-wide state, so every
//! test runs inside an `Engine` guard that serializes access, gives the speech
//! source a mixer handle, and clears the track list again on exit.

use super::*;
use crate::sound::mixer::source as mixer_source;
use crate::sound::test_support::{ScriptedDecoder, ENGINE_TEST_LOCK};

type Calls = Arc<Mutex<Vec<i32>>>;

struct Engine {
    _guard: parking_lot::MutexGuard<'static, ()>,
}

fn engine() -> Engine {
    let guard = ENGINE_TEST_LOCK.lock();
    let _ = stop_track();
    let handle = mixer_source::mixer_gen_sources(1).unwrap()[0];
    stream::with_source(SPEECH_SOURCE, |source| {
        *source = SoundSource::new();
        source.handle = handle;
    });
    Engine { _guard: guard }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = stop_track();
        stream::with_source(SPEECH_SOURCE, |source| *source = SoundSource::new());
    }
}

/// A decoder with `pcm_chunks` blocks of audio that lasts `secs` seconds.
fn voice(pcm_chunks: usize, secs: f32) -> Box<dyn SoundDecoder> {
    let chunks = (0..pcm_chunks).map(|i| vec![i as u8; 400]).collect();
    Box::new(ScriptedDecoder::new(chunks).with_length(secs))
}

/// More audio than the eight prefill buffers can hold, so the first chunk's
/// decoder is still the active one right after `play_track`.
fn long_voice(secs: f32) -> Box<dyn SoundDecoder> {
    voice(20, secs)
}

fn recording_callback(calls: &Calls) -> Option<Box<dyn Fn(i32) + Send>> {
    let calls = Arc::clone(calls);
    Some(Box::new(move |value| calls.lock().push(value)))
}

/// Two one-second pages: "First line..." at 0 ms and "..Second line" at 1000 ms.
fn splice_two_pages(calls: Option<&Calls>) {
    splice_track(
        Some("conversation"),
        Some("First line\r\nSecond line"),
        None,
        calls.and_then(recording_callback),
        vec![long_voice(1.0), long_voice(1.0)],
    )
    .unwrap();
}

fn chunk_at(index: usize) -> NonNull<SoundChunk> {
    let state = TRACK_STATE.lock();
    let mut chunk = state.chunks_head.as_deref().expect("a chunk list");
    for _ in 0..index {
        chunk = chunk.next.as_deref().expect("enough chunks");
    }
    NonNull::from(chunk)
}

fn chunk_count() -> usize {
    let state = TRACK_STATE.lock();
    let mut count = 0;
    let mut cur = state.chunks_head.as_deref();
    while let Some(chunk) = cur {
        count += 1;
        cur = chunk.next.as_deref();
    }
    count
}

fn texts() -> Vec<Option<String>> {
    let state = TRACK_STATE.lock();
    let mut out = Vec::new();
    let mut cur = state.chunks_head.as_deref();
    while let Some(chunk) = cur {
        out.push(chunk.text.clone());
        cur = chunk.next.as_deref();
    }
    out
}

fn cur_chunk() -> Option<NonNull<SoundChunk>> {
    TRACK_STATE.lock().cur_chunk
}

fn cur_sub_chunk() -> Option<NonNull<SoundChunk>> {
    TRACK_STATE.lock().cur_sub_chunk
}

fn speech_start_time() -> i32 {
    stream::with_source(SPEECH_SOURCE, |source| source.start_time).unwrap()
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

#[test]
fn speech_callback_advances_while_the_comm_slider_waits_for_the_source() {
    use std::sync::mpsc;
    use std::time::Duration;

    let _engine = engine();
    splice_two_pages(None);
    play_track(false).unwrap();
    let sample_arc = Arc::clone(TRACK_STATE.lock().sound_sample.as_ref().unwrap());
    let (reading, reader_started) = mpsc::channel();

    let (reader, callback_was_blocked) = stream::with_source(SPEECH_SOURCE, |_| {
        let reader = std::thread::spawn(move || {
            track_position(100, || {
                reading.send(()).unwrap();
                stream::get_stream_position_ticks(SPEECH_SOURCE)
            })
        });
        reader_started.recv_timeout(Duration::from_secs(5)).unwrap();
        // The decoder owns source -> sample before entering TrackCallbacks.
        // The slider has reached its source read and cannot finish until we
        // release the source. Acquiring track state here must remain possible.
        let mut sample = sample_arc.lock();
        let state = TRACK_STATE.try_lock_for(Duration::from_secs(1));
        let blocked = state.is_none();
        drop(state);
        if !blocked {
            assert!(TrackCallbacks.on_end_chunk(&mut sample, 0));
        }
        (reader, blocked)
    })
    .unwrap();
    reader.join().unwrap();
    assert!(
        !callback_was_blocked,
        "comm holds TRACK_STATE waiting for the source; decoder holds the source waiting for TRACK_STATE"
    );
    assert_eq!(cur_chunk(), Some(chunk_at(1)));
}

#[test]
fn track_mutations_leave_state_available_to_decoder_callbacks() {
    use crate::sound::test_support::Probe;
    use std::time::Duration;

    type Mutation = fn() -> AudioResult<()>;
    let mutations: [(&str, Mutation); 4] = [
        ("stop", stop_track),
        ("seek", || seek_to_position(ms_to_ticks(500))),
        ("splice", || {
            splice_track(
                Some("next"),
                Some("Next line"),
                None,
                None,
                vec![long_voice(1.0)],
            )
        }),
        ("number splice", || {
            splice_multi_track(vec![long_voice(1.0)], Some("42"))
        }),
    ];
    for (name, mutation) in mutations {
        let _engine = engine();
        splice_two_pages(None);
        play_track(false).unwrap();
        let sample_arc = Arc::clone(TRACK_STATE.lock().sound_sample.as_ref().unwrap());
        let (worker, blocked) = stream::with_source(SPEECH_SOURCE, |_| {
            let mut sample = sample_arc.lock();
            let worker = Probe::spawn(move || mutation().unwrap());
            assert!(
                !worker.finishes_within_grace(),
                "{name} must wait for the source/sample"
            );
            let state = TRACK_STATE.try_lock_for(Duration::from_secs(1));
            let blocked = state.is_none();
            drop(state);
            if !blocked {
                assert!(TrackCallbacks.on_end_chunk(&mut sample, 0));
            }
            (worker, blocked)
        })
        .unwrap();
        worker.join().unwrap();
        assert!(
            !blocked,
            "{name} holds track state while waiting for the decoder's source/sample"
        );
    }
}

#[test]
fn splicing_a_track_builds_one_tagged_chunk_per_subtitle_page() {
    let _engine = engine();

    splice_two_pages(None);

    assert_eq!(
        texts(),
        [
            Some("First line...".to_string()),
            Some("..Second line".to_string())
        ]
    );
    let state = TRACK_STATE.lock();
    let first = state.chunks_head.as_deref().unwrap();
    let second = first.next.as_deref().unwrap();
    assert_eq!((first.start_time, second.start_time), (0.0, 1000.0));
    assert!(first.tag_me && second.tag_me);
    assert_eq!((first.track_num, second.track_num), (0, 0));
    assert_eq!(
        first.run_time,
        ms_to_ticks(1040) as i32,
        "13 characters at 80 ms each"
    );
    assert_eq!(state.track_count, 1);
    assert_eq!(state.dec_offset, 2000.0);
    assert_eq!(state.last_sub, state.chunks_tail);
    assert_eq!(state.last_track_name, "conversation");
    let sample = state.sound_sample.as_ref().unwrap().lock();
    assert_eq!(sample.length, 2.0);
    assert_eq!(sample.num_buffers, 8);
}

#[test]
fn explicit_timestamps_override_the_reading_time_of_each_page() {
    let _engine = engine();

    splice_track(
        Some("conversation"),
        Some("one\ntwo"),
        Some("1500,2500"),
        None,
        vec![voice(1, 1.0), voice(1, 1.0)],
    )
    .unwrap();

    let state = TRACK_STATE.lock();
    let first = state.chunks_head.as_deref().unwrap();
    let second = first.next.as_deref().unwrap();
    assert_eq!(first.run_time, ms_to_ticks(1500) as i32);
    assert_eq!(second.run_time, ms_to_ticks(2500) as i32);
}

#[test]
fn a_splice_without_text_adds_nothing_and_a_subtitle_needs_a_track_to_join() {
    let _engine = engine();

    splice_track(Some("silent"), None, None, None, vec![voice(1, 1.0)]).unwrap();
    splice_track(None, Some("orphan"), None, None, Vec::new()).unwrap();

    assert_eq!(chunk_count(), 0);
    assert_eq!(TRACK_STATE.lock().track_count, 0);
    assert!(TRACK_STATE.lock().sound_sample.is_none());
}

#[test]
fn only_the_first_page_carries_the_completion_callback() {
    let _engine = engine();
    let calls: Calls = Arc::default();
    splice_two_pages(Some(&calls));

    let state = TRACK_STATE.lock();
    let first = state.chunks_head.as_deref().unwrap();
    assert!(first.callback.is_some());
    assert!(first.next.as_deref().unwrap().callback.is_none());
}

#[test]
fn a_later_subtitle_joins_the_last_page_and_extra_pages_become_audio_less_chunks() {
    let _engine = engine();
    splice_two_pages(None);

    splice_track(None, Some("more\nthird"), None, None, Vec::new()).unwrap();

    assert_eq!(
        texts(),
        [
            Some("First line...".to_string()),
            Some("..Second linemore...".to_string()),
            Some("..third".to_string()),
        ]
    );
    let state = TRACK_STATE.lock();
    let third = state
        .chunks_head
        .as_deref()
        .unwrap()
        .next
        .as_deref()
        .unwrap()
        .next
        .as_deref()
        .unwrap();
    assert!(third.decoder.is_none());
    assert!(third.tag_me);
    assert_eq!(third.start_time, 2000.0, "it starts where the audio ended");
    assert_eq!(third.track_num, 0);
    assert_eq!(state.track_count, 1, "a subtitle is not a new track");
}

#[test]
fn successive_subtitles_keep_growing_the_newest_page() {
    let _engine = engine();
    splice_two_pages(None);
    splice_track(None, Some("more\nthird"), None, None, Vec::new()).unwrap();

    splice_track(None, Some(" and more"), None, None, Vec::new()).unwrap();

    assert_eq!(
        texts()[2],
        Some("..third and more".to_string()),
        "the append lands on the newest page, not an older one"
    );
    assert_eq!(texts()[1], Some("..Second linemore...".to_string()));
}

#[test]
fn a_second_track_continues_the_numbering_and_the_decoder_offset() {
    let _engine = engine();
    splice_two_pages(None);

    splice_track(
        Some("reply"),
        Some("Answer"),
        None,
        None,
        vec![voice(1, 0.5)],
    )
    .unwrap();

    let state = TRACK_STATE.lock();
    let mut chunk = state.chunks_head.as_deref().unwrap();
    while let Some(next) = chunk.next.as_deref() {
        chunk = next;
    }
    assert_eq!(chunk.track_num, 1);
    assert_eq!(chunk.start_time, 2000.0);
    assert_eq!(state.track_count, 2);
    assert_eq!(state.dec_offset, 2500.0);
    assert_eq!(state.sound_sample.as_ref().unwrap().lock().length, 2.5);
}

#[test]
fn numbered_sale_speech_has_decoders_for_the_number_and_continuation() {
    let _engine = engine();
    unsafe {
        crate::sound::heart_ffi::splice_track(
            c"comm/melnorme/prefix.ogg".as_ptr(),
            c"You have ".as_ptr(),
            std::ptr::null(),
            None,
        );
        let numbers = [c"comm/melnorme/seven.ogg".as_ptr(), std::ptr::null()];
        crate::sound::heart_ffi::splice_multi_track(numbers.as_ptr(), c"7".as_ptr());
        crate::sound::heart_ffi::splice_track(
            c"comm/melnorme/suffix.ogg".as_ptr(),
            c" credits".as_ptr(),
            std::ptr::null(),
            None,
        );
    }
    assert_eq!(chunk_count(), 3);
    for i in 0..3 {
        assert!(
            unsafe { chunk_at(i).as_ref() }.decoder.is_some(),
            "chunk {i} has no speech decoder"
        );
    }
    assert_eq!(
        texts(),
        [Some("You have 7 credits".to_string()), None, None]
    );
    play_track(false).unwrap();
    assert!(playing_track());
    let sample = playing_sample();
    while TrackCallbacks.on_end_chunk(&mut sample.lock(), 123) {
        assert!(sample.lock().decoder.is_some());
    }
    TrackCallbacks.on_end_stream(&mut sample.lock());
    assert_eq!(playing_track_num(), 0);
}

#[test]
fn a_multi_track_splice_needs_an_existing_track_and_asks_the_next_one_to_continue_the_page() {
    let _engine = engine();
    assert!(matches!(
        splice_multi_track(vec![voice(1, 0.24)], Some("7")),
        Err(AudioError::InvalidSample)
    ));

    splice_two_pages(None);
    splice_multi_track(vec![voice(1, 0.24), voice(1, 0.5)], Some(" tail")).unwrap();

    assert_eq!(chunk_count(), 4, "one audio chunk per decoded number clip");
    let state = TRACK_STATE.lock();
    assert_eq!(state.dec_offset, 2740.0);
    assert!((state.sound_sample.as_ref().unwrap().lock().length - 2.74).abs() < 0.001);
    drop(state);
    assert_eq!(texts()[1], Some("..Second line tail".to_string()));
    assert!(TRACK_STATE.lock().no_page_break);
    assert!(splice_multi_track(Vec::new(), Some("")).is_ok());
}

#[test]
fn a_track_spliced_without_a_page_break_extends_the_previous_subtitle() {
    let _engine = engine();
    splice_two_pages(None);
    splice_multi_track(Vec::new(), Some("")).unwrap();
    TRACK_STATE.lock().no_page_break = true;

    splice_track(
        Some("continuation"),
        Some(" and on"),
        None,
        None,
        vec![voice(1, 0.5)],
    )
    .unwrap();

    assert_eq!(texts()[1], Some("..Second line and on".to_string()));
    let state = TRACK_STATE.lock();
    assert_eq!(state.track_count, 1, "it is part of the same track");
    assert!(!state.no_page_break, "the flag is consumed");
    let mut chunk = state.chunks_head.as_deref().unwrap();
    while let Some(next) = chunk.next.as_deref() {
        chunk = next;
    }
    assert!(
        !chunk.tag_me,
        "the continued audio is not a page of its own"
    );
    assert!(chunk.text.is_none());
}

#[test]
fn pages_and_timestamps_are_parsed_the_way_the_conversation_scripts_write_them() {
    let pages = split_sub_pages("Hello there\nfriend.\r\n\r\nBye");
    let texts: Vec<_> = pages.iter().map(|p| p.text.as_str()).collect();
    assert_eq!(texts, ["Hello there...", "..friend.", "..Bye"]);
    assert!(pages.iter().all(|p| p.timestamp >= 1000.0));
    assert!(split_sub_pages("").is_empty());
    assert_eq!(get_time_stamps("100, 0, abc,\n250\r-5"), [100.0, 250.0]);
}

// ---------------------------------------------------------------------------
// Playback
// ---------------------------------------------------------------------------

#[test]
fn playing_a_track_starts_speech_at_the_first_page_and_reports_the_total_length() {
    let _engine = engine();
    let calls: Calls = Arc::default();
    splice_two_pages(Some(&calls));

    play_track(false).unwrap();

    assert!(stream::playing_stream(SPEECH_SOURCE));
    assert_eq!(
        TRACK_STATE.lock().tracks_length.load(Ordering::Acquire),
        ms_to_ticks(2000)
    );
    assert_eq!(cur_chunk(), Some(chunk_at(0)));
    assert_eq!(
        cur_sub_chunk(),
        Some(chunk_at(0)),
        "the first page is shown at once"
    );
    assert_eq!(calls.lock().as_slice(), [0], "its callback fired once");
    assert_eq!(playing_track_num(), 1);
    assert!(playing_track());
    assert_eq!(get_track_subtitle(), Some("First line...".to_string()));
    let sample = TRACK_STATE.lock().sound_sample.clone().unwrap();
    assert!(
        sample.lock().decoder.is_some(),
        "the chunk's decoder moved to the sample"
    );
    assert_eq!(sample.lock().offset, 0);
}

#[test]
fn playing_with_no_track_assembled_does_nothing() {
    let _engine = engine();
    play_track(true).unwrap();
    assert!(!stream::playing_stream(SPEECH_SOURCE));
    assert!(!playing_track());
    assert_eq!(playing_track_num(), 0);
}

#[test]
fn stopping_a_track_silences_speech_and_forgets_the_whole_conversation() {
    let _engine = engine();
    splice_two_pages(None);
    play_track(false).unwrap();

    stop_track().unwrap();

    assert!(!stream::playing_stream(SPEECH_SOURCE));
    let state = TRACK_STATE.lock();
    assert!(state.sound_sample.is_none());
    assert!(state.chunks_head.is_none());
    assert!(state.chunks_tail.is_null() && state.last_sub.is_null());
    assert_eq!((state.track_count, state.dec_offset), (0, 0.0));
    assert_eq!(state.tracks_length.load(Ordering::Acquire), 0);
    assert!(state.cur_chunk.is_none() && state.cur_sub_chunk.is_none());
}

#[test]
fn jumping_past_the_end_stops_speech_but_keeps_the_conversation_for_replay() {
    let _engine = engine();
    assert!(jump_track(0).is_ok(), "nothing to jump in");
    splice_two_pages(None);
    play_track(false).unwrap();

    jump_track(9).unwrap();

    assert!(!stream::playing_stream(SPEECH_SOURCE));
    assert!(cur_chunk().is_none() && cur_sub_chunk().is_none());
    assert_eq!(chunk_count(), 2);
    assert_eq!(playing_track_num(), 0);
}

#[test]
fn pausing_and_resuming_follow_the_speech_source_once_a_chunk_is_current() {
    let _engine = engine();
    resume_track().unwrap();
    assert!(
        !stream::playing_stream(SPEECH_SOURCE),
        "no current chunk, nothing to resume"
    );

    splice_two_pages(None);
    play_track(false).unwrap();
    pause_track().unwrap();
    assert!(!stream::playing_stream(SPEECH_SOURCE));
    resume_track().unwrap();
    assert!(stream::playing_stream(SPEECH_SOURCE));
}

#[test]
fn track_position_is_the_stream_clock_clamped_to_the_conversation_and_scalable() {
    let _engine = engine();
    assert_eq!(get_track_position(100), 0, "no conversation");
    splice_two_pages(None);
    play_track(false).unwrap();
    let length = ms_to_ticks(2000);

    stream::with_source(SPEECH_SOURCE, |s| s.start_time = -(length as i32 / 2));
    assert_eq!(get_track_position(0), length / 2);
    assert_eq!(get_track_position(100), 50);

    stream::with_source(SPEECH_SOURCE, |s| s.start_time = -(length as i32 * 3));
    assert_eq!(get_track_position(0), length, "clamped to the end");
    assert_eq!(get_track_position(100), 100);
}

// ---------------------------------------------------------------------------
// Chunk transitions driven by the stream engine
// ---------------------------------------------------------------------------

fn playing_sample() -> Arc<Mutex<SoundSample>> {
    TRACK_STATE.lock().sound_sample.clone().unwrap()
}

#[test]
fn reaching_the_end_of_a_chunk_hands_the_stream_to_the_next_one_and_tags_the_buffer() {
    let _engine = engine();
    let second_voice = ScriptedDecoder::new(vec![vec![5; 400]; 3]).with_length(1.0);
    let rewinds = Arc::clone(&second_voice.seeks);
    splice_track(
        Some("conversation"),
        Some("First line\r\nSecond line"),
        None,
        None,
        vec![long_voice(1.0), Box::new(second_voice)],
    )
    .unwrap();
    play_track(false).unwrap();
    let sample = playing_sample();
    let second = chunk_at(1);
    let buffer = 123;

    let switched = TrackCallbacks.on_end_chunk(&mut sample.lock(), buffer);

    assert!(switched);
    assert_eq!(cur_chunk(), Some(second));
    let first_has_decoder = unsafe { chunk_at(0).as_ref() }.decoder.is_some();
    assert!(
        first_has_decoder,
        "the finished chunk gets its decoder back"
    );
    assert_eq!(
        rewinds.lock().as_slice(),
        [0],
        "the next chunk starts from its beginning"
    );
    let sample = sample.lock();
    assert!(sample.decoder.is_some());
    let tag = stream::find_tagged_buffer(&sample, buffer).expect("the buffer is tagged");
    assert_eq!(tag.data, second.as_ptr() as usize);
}

#[test]
fn the_last_chunk_has_no_successor() {
    let _engine = engine();
    splice_two_pages(None);
    play_track(false).unwrap();
    let sample = playing_sample();
    assert!(TrackCallbacks.on_end_chunk(&mut sample.lock(), 1));

    assert!(!TrackCallbacks.on_end_chunk(&mut sample.lock(), 2));
    assert_eq!(cur_chunk(), Some(chunk_at(1)));
}

#[test]
fn a_played_tagged_buffer_shows_its_page_once_and_releases_the_tag() {
    let _engine = engine();
    let calls: Calls = Arc::default();
    splice_track(
        Some("conversation"),
        Some("First line\r\nSecond line"),
        None,
        None,
        vec![long_voice(1.0), long_voice(1.0)],
    )
    .unwrap();
    {
        // Give the second page a callback so its display is observable.
        let second = unsafe { &mut *chunk_at(1).as_ptr() };
        second.callback = recording_callback(&calls);
    }
    play_track(false).unwrap();
    let sample = playing_sample();
    TrackCallbacks.on_end_chunk(&mut sample.lock(), 55);
    let tag = {
        let sample = sample.lock();
        let found = stream::find_tagged_buffer(&sample, 55).unwrap();
        SoundTag {
            buf_handle: found.buf_handle,
            data: found.data,
        }
    };

    TrackCallbacks.on_tagged_buffer(&mut sample.lock(), &tag);

    assert_eq!(cur_sub_chunk(), Some(chunk_at(1)));
    assert_eq!(calls.lock().as_slice(), [0]);
    assert!(stream::find_tagged_buffer(&sample.lock(), 55).is_none());
    assert_eq!(get_track_subtitle(), Some("..Second line".to_string()));
}

#[test]
fn a_tag_that_no_longer_points_into_the_conversation_is_ignored() {
    let _engine = engine();
    splice_two_pages(None);
    play_track(false).unwrap();
    let sample = playing_sample();
    let before = cur_sub_chunk();
    let stale_chunk = Box::new(SoundChunk {
        decoder: None,
        length_ms: 0,
        start_time: 0.0,
        run_time: 0,
        tag_me: true,
        track_num: 7,
        text: Some("gone".into()),
        text_cstr: None,
        callback: None,
        next: None,
    });
    let tag = SoundTag {
        buf_handle: 9,
        data: stale_chunk.as_ref() as *const SoundChunk as usize,
    };

    TrackCallbacks.on_tagged_buffer(&mut sample.lock(), &tag);

    assert_eq!(cur_sub_chunk(), before);
}

#[test]
fn ending_the_stream_forgets_the_current_chunk_and_page() {
    let _engine = engine();
    splice_two_pages(None);
    play_track(false).unwrap();
    let sample = playing_sample();

    TrackCallbacks.on_end_stream(&mut sample.lock());

    assert!(cur_chunk().is_none() && cur_sub_chunk().is_none());
    assert_eq!(chunk_count(), 2, "the conversation itself stays");
}

#[test]
fn starting_without_a_current_chunk_declines() {
    let _engine = engine();
    splice_two_pages(None);
    let sample = playing_sample();
    assert!(!TrackCallbacks.on_start_stream(&mut sample.lock()));
}

// ---------------------------------------------------------------------------
// Seeking
// ---------------------------------------------------------------------------

#[test]
fn seeking_inside_the_chunk_that_is_playing_stays_in_that_chunk() {
    let _engine = engine();
    splice_two_pages(None);
    play_track(false).unwrap();
    let half_a_second = ms_to_ticks(500);

    seek_to_position(half_a_second).unwrap();

    assert_eq!(cur_chunk(), Some(chunk_at(0)));
    assert_eq!(cur_sub_chunk(), Some(chunk_at(0)));
    assert_eq!(speech_start_time(), -(half_a_second as i32));
    let sample = playing_sample();
    let sample = sample.lock();
    assert!(sample.decoder.is_some());
    assert_eq!(
        sample.offset, 0,
        "the sample is positioned at its chunk's start"
    );
}

#[test]
fn seeking_into_a_later_chunk_resolves_the_page_and_the_offset_within_it() {
    let _engine = engine();
    splice_two_pages(None);
    play_track(false).unwrap();
    let target = ms_to_ticks(1500);

    seek_to_position(target).unwrap();

    assert_eq!(cur_chunk(), Some(chunk_at(1)));
    assert_eq!(cur_sub_chunk(), Some(chunk_at(1)));
    assert_eq!(playing_sample().lock().offset, ms_to_ticks(1000) as i32);
    assert_eq!(speech_start_time(), -(target as i32));
}

#[test]
fn seeking_beyond_the_end_stops_speech_and_clears_the_position() {
    let _engine = engine();
    splice_two_pages(None);
    play_track(false).unwrap();

    seek_to_position(ms_to_ticks(2000) + 500).unwrap();

    assert!(!stream::playing_stream(SPEECH_SOURCE));
    assert!(cur_chunk().is_none() && cur_sub_chunk().is_none());
    let end = ms_to_ticks(2000);
    assert_eq!(
        speech_start_time(),
        -(end as i32 + 1),
        "clamped to one tick past the end"
    );
}

#[test]
fn smooth_scrolling_moves_the_position_by_a_fixed_step_from_the_stream_clock() {
    let _engine = engine();
    fast_forward_smooth().unwrap();
    fast_reverse_smooth().unwrap();
    splice_two_pages(None);
    play_track(false).unwrap();
    let step = ACCEL_SCROLL_SPEED as u32;
    stream::with_source(SPEECH_SOURCE, |s| s.start_time = -(step as i32 * 2));

    fast_forward_smooth().unwrap();
    assert_eq!(speech_start_time(), -(step as i32 * 3));

    fast_reverse_smooth().unwrap();
    assert_eq!(speech_start_time(), -(step as i32 * 2));

    stream::with_source(SPEECH_SOURCE, |s| s.start_time = 0);
    fast_reverse_smooth().unwrap();
    assert_eq!(
        speech_start_time(),
        0,
        "rewinding at the start stays at the start"
    );
}

#[test]
fn the_conversation_length_counts_a_chunk_whose_decoder_is_lent_to_the_stream() {
    let _engine = engine();
    splice_track(
        Some("solo"),
        Some("Only page"),
        None,
        None,
        vec![long_voice(1.0)],
    )
    .unwrap();
    play_track(false).unwrap();
    assert!(
        unsafe { chunk_at(0).as_ref() }.decoder.is_none(),
        "the only chunk's decoder is now with the stream"
    );

    let state = TRACK_STATE.lock();
    assert_eq!(tracks_end_time_inner(&state), ms_to_ticks(1000));
}

// ---------------------------------------------------------------------------
// Page navigation and subtitle access
// ---------------------------------------------------------------------------

#[test]
fn paging_forward_restarts_speech_at_the_next_page_and_paging_back_returns() {
    let _engine = engine();
    splice_two_pages(None);
    play_track(false).unwrap();

    fast_forward_page().unwrap();
    assert_eq!(cur_sub_chunk(), Some(chunk_at(1)));
    assert_eq!(cur_chunk(), Some(chunk_at(1)));
    assert_eq!(speech_start_time(), -(ms_to_ticks(1000) as i32));
    assert_eq!(get_track_subtitle(), Some("..Second line".to_string()));

    fast_reverse_page().unwrap();
    assert_eq!(cur_sub_chunk(), Some(chunk_at(0)));
    assert_eq!(speech_start_time(), 0);
}

#[test]
fn paging_forward_from_the_last_page_seeks_past_the_end() {
    let _engine = engine();
    splice_two_pages(None);
    play_track(false).unwrap();
    fast_forward_page().unwrap();

    fast_forward_page().unwrap();

    assert!(!stream::playing_stream(SPEECH_SOURCE));
    assert!(cur_chunk().is_none());
}

#[test]
fn paging_does_nothing_without_a_conversation() {
    let _engine = engine();
    fast_forward_page().unwrap();
    fast_reverse_page().unwrap();
    assert!(!stream::playing_stream(SPEECH_SOURCE));
}

#[test]
fn subtitles_can_be_walked_from_the_first_page_to_the_last() {
    let _engine = engine();
    assert!(get_first_track_subtitle().is_none());
    assert!(get_next_track_subtitle().is_none());
    splice_two_pages(None);
    play_track(false).unwrap();

    let first = get_first_track_subtitle().unwrap();
    assert_eq!(get_track_subtitle_text(&first), Some("First line..."));
    assert_eq!(first.track_num, 0);
    let next = get_next_track_subtitle().unwrap();
    assert_eq!(next.text, "..Second line");
    fast_forward_page().unwrap();
    assert!(
        get_next_track_subtitle().is_none(),
        "nothing after the last page"
    );
}

#[test]
fn chunk_pointers_walk_the_tagged_pages_and_refuse_foreign_pointers() {
    let _engine = engine();
    assert!(get_first_chunk_ptr().is_null());
    splice_two_pages(None);

    let first = get_first_chunk_ptr();
    assert_eq!(get_chunk_text(first), Some("First line...".to_string()));
    let second = get_next_chunk_ptr(first);
    assert_eq!(get_chunk_text(second), Some("..Second line".to_string()));
    assert!(get_next_chunk_ptr(second).is_null());
    assert!(get_next_chunk_ptr(std::ptr::null()).is_null());

    let foreign = 0x10 as *const u8;
    assert!(get_next_chunk_ptr(foreign).is_null());
    assert_eq!(get_chunk_text(foreign), None);
    assert_eq!(get_chunk_text(std::ptr::null()), None);
    assert!(get_chunk_text_cstr(foreign).is_null());
    assert!(get_chunk_text_cstr(std::ptr::null()).is_null());
}

#[test]
fn the_c_string_for_a_page_is_stable_so_callers_can_compare_pointers() {
    let _engine = engine();
    assert!(get_track_subtitle_cstr().is_null());
    splice_two_pages(None);
    play_track(false).unwrap();

    let once = get_track_subtitle_cstr();
    let twice = get_track_subtitle_cstr();
    assert!(!once.is_null());
    assert_eq!(once, twice);
    let text = unsafe { std::ffi::CStr::from_ptr(once) }.to_str().unwrap();
    assert_eq!(text, "First line...");

    let by_chunk = get_chunk_text_cstr(get_first_chunk_ptr());
    assert_eq!(by_chunk, once, "the same cached string serves both lookups");
}

#[test]
fn finding_pages_skips_untagged_audio() {
    let _engine = engine();
    splice_two_pages(None);
    splice_multi_track(vec![voice(1, 0.24)], Some("")).unwrap();
    TRACK_STATE.lock().no_page_break = false;
    splice_track(Some("next"), Some("Third"), None, None, vec![voice(1, 1.0)]).unwrap();
    let state = TRACK_STATE.lock();

    let after_second = find_next_page_inner(Some(chunk_at_locked(&state, 1))).unwrap();
    assert_eq!(
        unsafe { after_second.as_ref() }.text.as_deref(),
        Some("Third")
    );
    let before_third = find_prev_page_inner(&state.chunks_head, Some(after_second)).unwrap();
    assert_eq!(
        unsafe { before_third.as_ref() }.text.as_deref(),
        Some("..Second line")
    );
    let when_unknown = find_prev_page_inner(&state.chunks_head, None).unwrap();
    assert_eq!(
        unsafe { when_unknown.as_ref() }.text.as_deref(),
        Some("Third")
    );
}

fn chunk_at_locked(state: &TrackPlayerState, index: usize) -> NonNull<SoundChunk> {
    let mut chunk = state.chunks_head.as_deref().unwrap();
    for _ in 0..index {
        chunk = chunk.next.as_deref().unwrap();
    }
    NonNull::from(chunk)
}

#[test]
fn a_chunk_caches_one_nul_terminated_copy_of_its_text() {
    let mut chunk = SoundChunk {
        decoder: None,
        length_ms: 0,
        start_time: 0.0,
        run_time: 0,
        tag_me: false,
        track_num: 0,
        text: Some("hi".into()),
        text_cstr: None,
        callback: None,
        next: None,
    };
    let first = chunk.text_cstr_ptr();
    assert_eq!(first, chunk.text_cstr_ptr());
    assert_eq!(unsafe { std::ffi::CStr::from_ptr(first) }.to_bytes(), b"hi");

    chunk.text = None;
    chunk.text_cstr = None;
    assert!(chunk.text_cstr_ptr().is_null());
}
