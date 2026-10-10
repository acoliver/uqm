//! Behavioural tests for the streaming pipeline: starting, pausing, seeking
//! and refilling a stream, the oscilloscope read-out, and music fades.
//!
//! The engine keeps its sources in process-wide statics, so every test takes
//! `ENGINE_TEST_LOCK` and works on a source whose mixer handle it created.
//! No decoder thread runs while the lock is held, which lets the tests call
//! `process_source_stream` directly and observe the result deterministically.

use super::*;
use crate::sound::mixer::ffi::playback_tests as mixer_playback;
use crate::sound::mixer::types::BufferState;
use crate::sound::test_support::{Probe, ScriptedDecoder, ENGINE_TEST_LOCK};

const CHUNK: usize = 400;
const TEST_SOURCE: usize = LAST_SFX_SOURCE;

type Log = Arc<Mutex<Vec<String>>>;

/// Records every callback the engine makes, in order.
struct Recorder {
    log: Log,
    accept_start: bool,
    replacement: Option<Vec<Vec<u8>>>,
}

impl Recorder {
    fn new(log: &Log) -> Self {
        Recorder {
            log: Arc::clone(log),
            accept_start: true,
            replacement: None,
        }
    }
}

impl StreamCallbacks for Recorder {
    fn on_start_stream(&mut self, _sample: &mut SoundSample) -> bool {
        self.log.lock().push("start".into());
        self.accept_start
    }
    fn on_end_chunk(&mut self, sample: &mut SoundSample, buffer: usize) -> bool {
        self.log.lock().push(format!("end_chunk {buffer}"));
        match self.replacement.take() {
            Some(chunks) => {
                sample.decoder = Some(Box::new(ScriptedDecoder::new(chunks)));
                true
            }
            None => false,
        }
    }
    fn on_end_stream(&mut self, _sample: &mut SoundSample) {
        self.log.lock().push("end_stream".into());
    }
    fn on_tagged_buffer(&mut self, _sample: &mut SoundSample, tag: &SoundTag) {
        self.log.lock().push(format!("tagged {}", tag.data));
    }
    fn on_queue_buffer(&mut self, _sample: &mut SoundSample, buffer: usize) {
        self.log.lock().push(format!("queue {buffer}"));
    }
}

fn pcm(fill: u8) -> Vec<u8> {
    vec![fill; CHUNK]
}

fn chunks(count: usize) -> Vec<Vec<u8>> {
    (0..count).map(|i| pcm(i as u8 + 1)).collect()
}

/// `ENGINE_TEST_LOCK` together with the mixer source a test plays on. The
/// mixer renders every playing source in the process-wide pool, so the source
/// is deleted before the lock is released and a finished test leaves nothing
/// in the pool for the mixer tests to hear.
struct TestSourceLease {
    handle: usize,
    _guard: parking_lot::MutexGuard<'static, ()>,
}

impl Drop for TestSourceLease {
    fn drop(&mut self) {
        let deleted = mixer_source::mixer_delete_sources(&[self.handle]);
        assert!(
            deleted.is_ok() || std::thread::panicking(),
            "the test's mixer source was already gone"
        );
    }
}

fn lock_test() -> TestSourceLease {
    let guard = ENGINE_TEST_LOCK.lock();
    let handle = mixer_source::mixer_gen_sources(1).unwrap()[0];
    let mut source = get_source(TEST_SOURCE).unwrap();
    *source = SoundSource::new();
    source.handle = handle;
    TestSourceLease {
        handle,
        _guard: guard,
    }
}

fn sample_with(
    decoder: Option<ScriptedDecoder>,
    buffers: u32,
    callbacks: Option<Recorder>,
) -> Arc<Mutex<SoundSample>> {
    let decoder = decoder.map(|d| Box::new(d) as Box<dyn SoundDecoder>);
    let callbacks = callbacks.map(|c| Box::new(c) as Box<dyn StreamCallbacks + Send>);
    Arc::new(Mutex::new(
        create_sound_sample(decoder, buffers, callbacks).unwrap(),
    ))
}

fn mixer_int(prop: SourceProp) -> i32 {
    let handle = get_source(TEST_SOURCE).unwrap().handle;
    mixer_source::mixer_get_source_i(handle, prop).unwrap()
}

fn log_of(log: &Log) -> Vec<String> {
    log.lock().clone()
}

/// What the mixer does when it reaches the end of the buffer at the front of
/// the queue: mark it processed so the engine can recycle it.
fn finish_front_buffer() -> usize {
    let handle = get_source(TEST_SOURCE).unwrap().handle;
    let source = mixer_source::mixer_get_source(handle).unwrap();
    let mut source = source.lock();
    let front = source.first_queued.expect("a buffer is queued");
    crate::sound::mixer::buffer::mixer_get_buffer(front)
        .unwrap()
        .lock()
        .state = BufferState::Processed as u32;
    source.processed_count += 1;
    front
}

// ---------------------------------------------------------------------------
// Starting a stream
// ---------------------------------------------------------------------------

#[test]
fn starting_a_stream_prefills_one_buffer_per_decoded_chunk_then_plays() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let sample = sample_with(
        Some(ScriptedDecoder::new(chunks(2))),
        3,
        Some(Recorder::new(&log)),
    );
    let buffers = sample.lock().buffers.clone();

    play_stream(Arc::clone(&sample), TEST_SOURCE, false, false, true).unwrap();

    assert_eq!(mixer_int(SourceProp::BuffersQueued), 2);
    assert_eq!(
        mixer_int(SourceProp::SourceState),
        SourceState::Playing as i32
    );
    let source = get_source(TEST_SOURCE).unwrap();
    assert!(source.stream_should_be_playing);
    assert_eq!(source.queued_buf_sizes, [CHUNK, CHUNK]);
    drop(source);
    assert_eq!(
        log_of(&log),
        [
            "start".to_string(),
            format!("queue {}", buffers[0]),
            format!("queue {}", buffers[1]),
            format!("end_chunk {}", buffers[2]),
        ],
        "the third buffer found the decoder at EOF and asked for a successor"
    );
    assert!(playing_stream(TEST_SOURCE));
}

#[test]
fn a_start_callback_that_declines_aborts_before_any_audio_is_queued() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let mut recorder = Recorder::new(&log);
    recorder.accept_start = false;
    let sample = sample_with(Some(ScriptedDecoder::new(chunks(2))), 2, Some(recorder));

    let result = play_stream(sample, TEST_SOURCE, false, false, true);

    assert!(matches!(result, Err(AudioError::EndOfStream)));
    assert_eq!(mixer_int(SourceProp::BuffersQueued), 0);
    assert!(!playing_stream(TEST_SOURCE));
    assert_eq!(log_of(&log), ["start"]);
}

#[test]
fn a_stream_cannot_start_on_a_missing_source_or_without_a_decoder() {
    let _guard = lock_test();
    let with_decoder = sample_with(Some(ScriptedDecoder::new(chunks(1))), 1, None);
    assert!(matches!(
        play_stream(with_decoder, NUM_SOUNDSOURCES, false, false, true),
        Err(AudioError::InvalidSource(n)) if n == NUM_SOUNDSOURCES
    ));

    let without_decoder = sample_with(None, 1, None);
    assert!(matches!(
        play_stream(without_decoder, TEST_SOURCE, false, false, true),
        Err(AudioError::InvalidDecoder)
    ));
    assert!(!playing_stream(TEST_SOURCE));
}

#[test]
fn a_scoped_stream_keeps_the_pcm_it_queued_for_the_oscilloscope() {
    let _guard = lock_test();
    let sample = sample_with(Some(ScriptedDecoder::new(chunks(2))), 2, None);

    play_stream(sample, TEST_SOURCE, false, true, true).unwrap();

    let source = get_source(TEST_SOURCE).unwrap();
    assert_eq!(source.sbuf_size as usize, 2 * BUFFER_SIZE + PAD_SCOPE_BYTES);
    assert_eq!(source.sbuf_tail as usize, 2 * CHUNK);
    let scope = source.sbuffer.as_ref().unwrap();
    assert!(scope[..CHUNK].iter().all(|&b| b == 1));
    assert!(scope[CHUNK..2 * CHUNK].iter().all(|&b| b == 2));
    assert!(scope[2 * CHUNK..].iter().all(|&b| b == 0));
}

#[test]
fn the_start_offset_is_either_rewound_overridden_or_taken_from_the_decoder_position() {
    let _guard = lock_test();
    let decoder = ScriptedDecoder::new(chunks(3));
    let seeks = Arc::clone(&decoder.seeks);
    let sample = sample_with(Some(decoder), 1, None);
    sample.lock().offset = 100;

    play_stream(Arc::clone(&sample), TEST_SOURCE, false, false, true).unwrap();
    assert_eq!(
        seeks.lock().as_slice(),
        [0],
        "rewind goes back to the start"
    );
    assert_eq!(get_source(TEST_SOURCE).unwrap().start_time, -100);
    assert_eq!(get_stream_position_ticks(TEST_SOURCE), 100);

    play_stream_with_offset_override(
        Arc::clone(&sample),
        TEST_SOURCE,
        false,
        false,
        false,
        Some(840),
    )
    .unwrap();
    assert_eq!(
        seeks.lock().len(),
        1,
        "an override without rewind does not seek"
    );
    assert_eq!(get_source(TEST_SOURCE).unwrap().start_time, -840);
    assert_eq!(get_stream_position_ticks(TEST_SOURCE), 840);

    // Without rewind the position continues from the decoder's own progress:
    // two chunks (one second) were decoded by the previous two starts.
    sample.lock().offset = 7;
    play_stream(sample, TEST_SOURCE, true, false, false).unwrap();
    assert_eq!(
        get_source(TEST_SOURCE).unwrap().start_time,
        -(7 + ONE_SECOND as i32)
    );
}

#[test]
fn looping_is_recorded_on_the_sample_but_never_on_the_mixer_source() {
    let _guard = lock_test();
    let sample = sample_with(Some(ScriptedDecoder::new(chunks(1))), 1, None);

    play_stream(Arc::clone(&sample), TEST_SOURCE, true, false, true).unwrap();

    assert!(sample.lock().looping);
    assert_eq!(mixer_int(SourceProp::Looping), 0);
}

#[test]
fn starting_a_second_stream_replaces_the_first_on_the_same_source() {
    let _guard = lock_test();
    let first = sample_with(Some(ScriptedDecoder::new(chunks(2))), 2, None);
    let second = sample_with(Some(ScriptedDecoder::new(chunks(1))), 2, None);

    play_stream(Arc::clone(&first), TEST_SOURCE, false, false, true).unwrap();
    play_stream(Arc::clone(&second), TEST_SOURCE, false, false, true).unwrap();

    let source = get_source(TEST_SOURCE).unwrap();
    assert!(Arc::ptr_eq(source.sample.as_ref().unwrap(), &second));
    assert_eq!(source.queued_buf_sizes, [CHUNK]);
    drop(source);
    assert_eq!(mixer_int(SourceProp::BuffersQueued), 1);
}

// ---------------------------------------------------------------------------
// Stop, pause, resume, seek
// ---------------------------------------------------------------------------

#[test]
fn stopping_a_stream_clears_the_queue_the_scope_and_the_sample() {
    let _guard = lock_test();
    let sample = sample_with(Some(ScriptedDecoder::new(chunks(2))), 2, None);
    play_stream(sample, TEST_SOURCE, false, true, true).unwrap();

    stop_stream(TEST_SOURCE).unwrap();

    let source = get_source(TEST_SOURCE).unwrap();
    assert!(!source.stream_should_be_playing);
    assert!(source.sample.is_none());
    assert!(source.sbuffer.is_none());
    assert_eq!(
        (source.sbuf_size, source.sbuf_tail, source.sbuf_head),
        (0, 0, 0)
    );
    assert!(source.queued_buf_sizes.is_empty());
    drop(source);
    assert_eq!(mixer_int(SourceProp::BuffersQueued), 0);
    assert_eq!(
        mixer_int(SourceProp::SourceState),
        SourceState::Stopped as i32
    );
    assert!(matches!(
        stop_stream(NUM_SOUNDSOURCES),
        Err(AudioError::InvalidSource(_))
    ));
}

#[test]
fn pausing_halts_the_mixer_source_and_resuming_restarts_it() {
    let _guard = lock_test();
    let sample = sample_with(Some(ScriptedDecoder::new(chunks(2))), 2, None);
    play_stream(sample, TEST_SOURCE, false, false, true).unwrap();

    pause_stream(TEST_SOURCE).unwrap();
    assert!(!playing_stream(TEST_SOURCE));
    assert_eq!(
        mixer_int(SourceProp::SourceState),
        SourceState::Paused as i32
    );

    resume_stream(TEST_SOURCE).unwrap();
    assert!(playing_stream(TEST_SOURCE));
    assert_eq!(
        mixer_int(SourceProp::SourceState),
        SourceState::Playing as i32
    );
    assert_eq!(get_source(TEST_SOURCE).unwrap().pause_time, 0);
}

#[test]
fn resuming_shifts_the_start_time_by_the_time_spent_paused() {
    let _guard = lock_test();
    let sample = sample_with(Some(ScriptedDecoder::new(chunks(1))), 1, None);
    play_stream(sample, TEST_SOURCE, false, false, true).unwrap();
    {
        let mut source = get_source(TEST_SOURCE).unwrap();
        source.start_time = 1000;
        source.pause_time = 40; // the clock reads 0 in tests, so 40 ticks "ago" is in the future
    }

    resume_stream(TEST_SOURCE).unwrap();

    let source = get_source(TEST_SOURCE).unwrap();
    assert_eq!(source.start_time, 1000 + (0 - 40));
    assert_eq!(source.pause_time, 0);
}

#[test]
fn seeking_converts_milliseconds_to_pcm_frames_and_restarts_with_the_scope_kept() {
    let _guard = lock_test();
    let decoder = ScriptedDecoder::new(chunks(2));
    let seeks = Arc::clone(&decoder.seeks);
    let sample = sample_with(Some(decoder), 2, None);
    play_stream(Arc::clone(&sample), TEST_SOURCE, true, true, true).unwrap();
    seeks.lock().clear();

    seek_stream(TEST_SOURCE, 500).unwrap();

    assert_eq!(seeks.lock().as_slice(), [22050]);
    let source = get_source(TEST_SOURCE).unwrap();
    assert!(source.stream_should_be_playing);
    assert!(source.sbuffer.is_some(), "scope mode survives a seek");
    drop(source);
    assert!(sample.lock().looping, "loop mode survives a seek");
    assert_eq!(mixer_int(SourceProp::BuffersQueued), 2);
}

#[test]
fn seeking_a_source_with_no_stream_is_refused() {
    let _guard = lock_test();
    assert!(matches!(
        seek_stream(TEST_SOURCE, 10),
        Err(AudioError::InvalidSample)
    ));
    assert!(matches!(
        seek_stream(NUM_SOUNDSOURCES, 10),
        Err(AudioError::InvalidSource(_))
    ));
}

#[test]
fn stream_position_is_zero_without_a_stream_and_for_a_missing_source() {
    let _guard = lock_test();
    assert_eq!(get_stream_position_ticks(TEST_SOURCE), 0);
    assert_eq!(get_stream_position_ticks(NUM_SOUNDSOURCES), 0);
    assert!(!playing_stream(NUM_SOUNDSOURCES));
}

// ---------------------------------------------------------------------------
// Refilling from the decoder thread
// ---------------------------------------------------------------------------

#[test]
fn a_finished_buffer_is_recycled_with_the_next_chunk_and_reported() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let sample = sample_with(
        Some(ScriptedDecoder::new(chunks(4))),
        2,
        Some(Recorder::new(&log)),
    );
    play_stream(Arc::clone(&sample), TEST_SOURCE, false, true, true).unwrap();
    let finished = finish_front_buffer();
    log.lock().clear();

    process_source_stream(TEST_SOURCE, &sample);

    assert_eq!(log_of(&log), [format!("queue {finished}")]);
    let source = get_source(TEST_SOURCE).unwrap();
    assert_eq!(source.last_q_buf, finished);
    assert_eq!(source.queued_buf_sizes, [CHUNK, CHUNK]);
    assert_eq!(
        source.sbuf_head as usize, CHUNK,
        "scope drops the played chunk"
    );
    assert_eq!(
        source.sbuf_tail as usize,
        3 * CHUNK,
        "and gains the new one"
    );
    drop(source);
    assert_eq!(mixer_int(SourceProp::BuffersQueued), 2);
    assert_eq!(mixer_int(SourceProp::BuffersProcessed), 0);
}

#[test]
fn a_tagged_buffer_reports_its_tag_when_it_finishes_playing() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let sample = sample_with(
        Some(ScriptedDecoder::new(chunks(4))),
        2,
        Some(Recorder::new(&log)),
    );
    play_stream(Arc::clone(&sample), TEST_SOURCE, false, false, true).unwrap();
    let front = {
        let mut sample = sample.lock();
        let front = sample.buffers[0];
        assert!(tag_buffer(&mut sample, front, 77));
        front
    };
    let finished = finish_front_buffer();
    assert_eq!(finished, front);
    log.lock().clear();

    process_source_stream(TEST_SOURCE, &sample);

    assert_eq!(
        log_of(&log),
        ["tagged 77".to_string(), format!("queue {front}")]
    );
}

#[test]
fn a_decoder_at_eof_asks_for_a_successor_and_keeps_streaming_from_it() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let mut recorder = Recorder::new(&log);
    recorder.replacement = Some(vec![pcm(9)]);
    let sample = sample_with(Some(ScriptedDecoder::new(chunks(2))), 2, Some(recorder));
    play_stream(Arc::clone(&sample), TEST_SOURCE, false, false, true).unwrap();
    let last_queued = get_source(TEST_SOURCE).unwrap().last_q_buf;
    finish_front_buffer();
    log.lock().clear();

    process_source_stream(TEST_SOURCE, &sample);

    let events = log_of(&log);
    assert_eq!(events.len(), 2, "{events:?}");
    assert_eq!(events[0], format!("end_chunk {last_queued}"));
    assert!(events[1].starts_with("queue "));
    assert!(!get_source(TEST_SOURCE).unwrap().end_chunk_failed);
    assert_eq!(mixer_int(SourceProp::BuffersQueued), 2);
}

#[test]
fn when_no_successor_exists_the_failure_is_remembered_and_not_retried() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let sample = sample_with(
        Some(ScriptedDecoder::new(chunks(2))),
        2,
        Some(Recorder::new(&log)),
    );
    play_stream(Arc::clone(&sample), TEST_SOURCE, false, false, true).unwrap();
    finish_front_buffer();
    log.lock().clear();

    process_source_stream(TEST_SOURCE, &sample);
    assert!(get_source(TEST_SOURCE).unwrap().end_chunk_failed);
    let after_first = log_of(&log).len();
    assert_eq!(after_first, 1, "{:?}", log_of(&log));
    assert_eq!(
        mixer_int(SourceProp::BuffersQueued),
        1,
        "no buffer was refilled"
    );

    finish_front_buffer();
    process_source_stream(TEST_SOURCE, &sample);
    assert_eq!(
        log_of(&log).len(),
        after_first,
        "on_end_chunk is not called again"
    );
}

#[test]
fn a_decode_failure_stops_the_stream_without_ending_it_normally() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let mut decoder = ScriptedDecoder::new(chunks(4));
    decoder.fail_after = Some(2);
    let sample = sample_with(Some(decoder), 2, Some(Recorder::new(&log)));
    play_stream(Arc::clone(&sample), TEST_SOURCE, false, false, true).unwrap();
    finish_front_buffer();

    process_source_stream(TEST_SOURCE, &sample);

    assert!(!playing_stream(TEST_SOURCE));
    assert!(!log_of(&log).contains(&"end_stream".to_string()));
}

#[test]
fn an_idle_source_with_nothing_left_to_decode_ends_the_stream_once() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let sample = sample_with(
        Some(ScriptedDecoder::new(Vec::new())),
        2,
        Some(Recorder::new(&log)),
    );
    {
        let mut source = get_source(TEST_SOURCE).unwrap();
        source.sample = Some(Arc::clone(&sample));
        source.stream_should_be_playing = true;
    }

    process_source_stream(TEST_SOURCE, &sample);

    assert!(!playing_stream(TEST_SOURCE));
    assert_eq!(
        log_of(&log),
        [
            format!("end_chunk {}", sample.lock().buffers[0]),
            format!("end_chunk {}", sample.lock().buffers[1]),
            "end_stream".to_string()
        ]
    );
}

#[test]
fn an_idle_source_restarts_when_the_decoder_still_has_audio() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let sample = sample_with(
        Some(ScriptedDecoder::new(chunks(2))),
        2,
        Some(Recorder::new(&log)),
    );
    {
        let mut source = get_source(TEST_SOURCE).unwrap();
        source.sample = Some(Arc::clone(&sample));
        source.stream_should_be_playing = true;
    }

    process_source_stream(TEST_SOURCE, &sample);

    assert!(playing_stream(TEST_SOURCE));
    assert_eq!(mixer_int(SourceProp::BuffersQueued), 2);
    assert_eq!(
        mixer_int(SourceProp::SourceState),
        SourceState::Playing as i32
    );
    assert!(!log_of(&log).contains(&"end_stream".to_string()));
}

#[test]
fn callbacks_for_a_stream_that_was_replaced_are_dropped() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let stale = sample_with(None, 1, Some(Recorder::new(&log)));
    let current = sample_with(None, 1, None);
    get_source(TEST_SOURCE).unwrap().sample = Some(current);

    execute_deferred_callbacks(
        vec![
            DeferredCallback::EndStream,
            DeferredCallback::QueueBuffer { buf_handle: 3 },
        ],
        &stale,
        TEST_SOURCE,
    );

    assert!(log_of(&log).is_empty());
}

#[test]
fn deferred_callbacks_run_in_order_for_the_current_stream() {
    let _guard = lock_test();
    let log: Log = Arc::default();
    let sample = sample_with(None, 1, Some(Recorder::new(&log)));
    get_source(TEST_SOURCE).unwrap().sample = Some(Arc::clone(&sample));

    execute_deferred_callbacks(
        vec![
            DeferredCallback::QueueBuffer { buf_handle: 3 },
            DeferredCallback::TaggedBuffer {
                buf_handle: 3,
                tag_data: 5,
            },
            DeferredCallback::EndChunk { buf_handle: 3 },
            DeferredCallback::EndStream,
        ],
        &sample,
        TEST_SOURCE,
    );

    assert_eq!(
        log_of(&log),
        ["queue 3", "tagged 5", "end_chunk 3", "end_stream"]
    );
    assert!(sample.lock().callbacks.is_some(), "callbacks are put back");
}

// ---------------------------------------------------------------------------
// Oscilloscope read-out
// ---------------------------------------------------------------------------

fn reset_agc() {
    *AGC.lock() = AgcState {
        page_sum: DEF_PAGE_MAX * AGC_PAGE_COUNT as i32,
        pages: [DEF_PAGE_MAX; AGC_PAGE_COUNT],
        page_head: 0,
        frame_sum: 0,
        frames: 0,
        avg_amp: DEF_PAGE_MAX,
    };
}

/// Attach a playing 16-bit mono stream to the music source whose scope ring
/// holds `samples`.
fn scoped_music(samples: &[i16]) -> Arc<Mutex<SoundSample>> {
    let sample = sample_with(Some(ScriptedDecoder::new(Vec::new())), 1, None);
    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let mut source = get_source(MUSIC_SOURCE).unwrap();
    source.sample = Some(Arc::clone(&sample));
    source.stream_should_be_playing = true;
    source.sbuf_size = bytes.len() as u32;
    source.sbuffer = Some(bytes);
    source.sbuf_head = 0;
    source.sbuf_lasttime = get_time_counter();
    sample
}

fn release_music() {
    *get_source(MUSIC_SOURCE).unwrap() = SoundSource::new();
}

#[test]
fn the_scope_reports_nothing_for_a_source_that_is_not_streaming() {
    let _guard = lock_test();
    release_music();
    let mut line = [7u8; 8];

    assert_eq!(graph_foreground_stream(&mut line, 8, 64, false), 0);

    assert_eq!(line, [7; 8], "the caller's line is left untouched");
}

#[test]
fn a_silent_stream_draws_a_flat_line_through_the_middle() {
    let _guard = lock_test();
    reset_agc();
    let _sample = scoped_music(&[0; 64]);
    let mut line = [0u8; 16];

    let drawn = graph_foreground_stream(&mut line, 16, 64, false);
    release_music();

    assert_eq!(drawn, 1);
    assert_eq!(line, [32; 16]);
}

#[test]
fn loud_samples_are_scaled_around_the_middle_and_clamped_to_the_height() {
    let _guard = lock_test();
    reset_agc();
    // Music steps four 11025 Hz units per point, i.e. 16 frames at 44100 Hz.
    let mut wave = vec![0i16; 16 * 6];
    wave[0] = 28000; // point 0
    wave[16] = -28000; // point 1
    wave[32] = i16::MAX; // point 2
    let _sample = scoped_music(&wave);
    let mut line = [0u8; 4];

    graph_foreground_stream(&mut line, 4, 64, false);
    release_music();

    // avg_amp 28000 over a target amplitude of 16 gives a scale of 1750.
    assert_eq!(line, [32 + 16, 32 - 16, 32 + 18, 32]);
}

#[test]
fn eight_bit_scope_data_is_centred_on_128() {
    assert_eq!(read_scope_sample(&[128, 255, 0], 0, 3, 1), 0);
    assert_eq!(read_scope_sample(&[128, 255, 0], 1, 3, 1), 127 << 8);
    assert_eq!(read_scope_sample(&[128, 255, 0], 2, 3, 1), -128 << 8);
}

#[test]
fn sixteen_bit_scope_data_is_little_endian_and_wraps_around_the_ring() {
    assert_eq!(read_scope_sample(&[0x34, 0x12], 0, 2, 2), 0x1234);
    assert_eq!(read_scope_sample(&[0x34, 0x12], 1, 2, 2), 0x3412);
    assert_eq!(read_scope_sample(&[], 0, 0, 2), 0);
}

#[test]
fn removing_scope_data_resets_the_read_clock() {
    let mut source = SoundSource::new();
    source.sbuf_size = 10;
    source.sbuf_lasttime = 99;
    remove_scope_data(&mut source, 4);
    assert_eq!(source.sbuf_head, 4);
    assert_eq!(source.sbuf_lasttime, get_time_counter());

    let mut empty = SoundSource::new();
    remove_scope_data(&mut empty, 4);
    assert_eq!(empty.sbuf_head, 0);
}

#[test]
fn adding_scope_data_without_a_ring_buffer_is_a_no_op() {
    let mut source = SoundSource::new();
    add_scope_data(&mut source, &[1, 2, 3]);
    assert_eq!(source.sbuf_tail, 0);
    source.sbuffer = Some(Vec::new());
    add_scope_data(&mut source, &[1, 2, 3]);
    assert_eq!(source.sbuf_tail, 0);
}

// ---------------------------------------------------------------------------
// Music fade and source access
// ---------------------------------------------------------------------------

#[test]
fn a_zero_length_fade_is_refused_and_leaves_the_fade_state_alone() {
    let _guard = lock_test();
    ENGINE.fade.lock().interval = 0;
    assert!(!set_music_stream_fade(0, 50));
    assert_eq!(ENGINE.fade.lock().interval, 0);
}

#[test]
fn a_fade_records_the_distance_to_the_target_and_applies_the_starting_volume() {
    let _guard = lock_test();
    music::set_music_volume(40);

    assert!(set_music_stream_fade(120, 100));
    {
        let fade = ENGINE.fade.lock();
        assert_eq!(fade.interval, 120);
        assert_eq!(fade.start_volume, 40);
        assert_eq!(fade.delta, 60);
    }
    process_music_fade();

    assert_eq!(music::current_music_volume(), 40, "no time has passed");
    assert_eq!(
        ENGINE.fade.lock().interval,
        120,
        "the fade is still running"
    );
    ENGINE.fade.lock().interval = 0;
    process_music_fade();
    assert_eq!(
        music::current_music_volume(),
        40,
        "an idle fade changes nothing"
    );
}

#[test]
fn a_fade_whose_interval_has_elapsed_is_retired() {
    let _guard = lock_test();
    music::set_music_volume(10);
    // The test clock is frozen at zero; a start time of u32::MAX is exactly one
    // tick before it once the subtraction wraps, so the one-tick fade is over.
    *ENGINE.fade.lock() = FadeState {
        start_time: u32::MAX,
        interval: 1,
        start_volume: 10,
        delta: 30,
    };

    process_music_fade();

    assert_eq!(ENGINE.fade.lock().interval, 0, "the fade completes");
}

#[test]
fn source_accessors_reject_indexes_past_the_last_source() {
    let _guard = lock_test();
    assert!(with_source(NUM_SOUNDSOURCES, |_| ()).is_none());
    assert!(with_source(TEST_SOURCE, |s| s.handle).is_some());
    assert!(matches!(
        stop_source(NUM_SOUNDSOURCES),
        Err(AudioError::InvalidChannel(_))
    ));
}

#[test]
fn stopping_a_plain_source_detaches_its_sample() {
    let _guard = lock_test();
    let sample = sample_with(Some(ScriptedDecoder::new(chunks(1))), 1, None);
    play_stream(sample, TEST_SOURCE, false, false, true).unwrap();

    stop_source(TEST_SOURCE).unwrap();

    let source = get_source(TEST_SOURCE).unwrap();
    assert!(source.sample.is_none());
    assert!(!source.stream_should_be_playing);
}

#[test]
fn a_stereo_eight_bit_stream_at_a_foreign_rate_is_queued_like_any_other() {
    let _guard = lock_test();
    let decoder = ScriptedDecoder::new(chunks(1)).with_format(22050, AudioFormat::Stereo8);
    let sample = sample_with(Some(decoder), 1, None);
    let buffer = sample.lock().buffers[0];

    play_stream(sample, TEST_SOURCE, false, false, true).unwrap();

    let buffer = crate::sound::mixer::buffer::mixer_get_buffer(buffer).unwrap();
    assert_eq!(buffer.lock().state, BufferState::Queued as u32);
    assert_eq!(mixer_int(SourceProp::BuffersQueued), 1);
}

// ---------------------------------------------------------------------------
// Sharing the mixer's source pool with the mixer tests
// ---------------------------------------------------------------------------

#[test]
fn a_finished_stream_test_leaves_nothing_for_the_mixer_tests_to_hear() {
    starting_a_stream_prefills_one_buffer_per_decoded_chunk_then_plays();
    mixer_playback::a_queued_source_plays_its_samples_then_stops_with_the_buffer_processed();
}

#[test]
fn a_mixer_test_waits_while_a_stream_test_is_playing_on_the_pool() {
    let lease = lock_test();
    let sample = sample_with(Some(ScriptedDecoder::new(chunks(2))), 3, None);
    play_stream(sample, TEST_SOURCE, false, false, true).unwrap();
    assert_eq!(
        mixer_int(SourceProp::SourceState),
        SourceState::Playing as i32
    );

    let mixer_test = Probe::spawn(
        mixer_playback::a_queued_source_plays_its_samples_then_stops_with_the_buffer_processed,
    );
    let rendered_early = mixer_test.finishes_within_grace();
    drop(lease);

    assert!(
        !rendered_early,
        "the mixer test rendered while a stream test held the pool"
    );
    assert!(
        mixer_test.join().is_ok(),
        "the mixer test heard a source it did not play"
    );
}
