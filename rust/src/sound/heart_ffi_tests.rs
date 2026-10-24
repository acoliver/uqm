//! The audio-heart C exports driven the way the C game code drives them:
//! sound samples created and tagged through opaque pointers, streams started
//! on mixer sources, C callback tables invoked, and the channel, music and
//! control calls that sit on top.
//!
//! Streams live in process-wide statics, so every test takes
//! `ENGINE_TEST_LOCK` and brings the stream engine up and down itself.

use std::ffi::{c_int, c_uint, c_void, CString};
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use super::decoder::SoundDecoder;
use super::heart_ffi::*;
use super::stream;
use super::test_support::{ScriptedDecoder, ENGINE_TEST_LOCK};
use super::types::{SoundPosition, MUSIC_SOURCE};

const SOURCE: u32 = MUSIC_SOURCE as u32;

/// A running stream engine, stopped again on drop.
struct Engine {
    _guard: parking_lot::MutexGuard<'static, ()>,
}

impl Engine {
    fn start() -> Self {
        let guard = ENGINE_TEST_LOCK.lock();

        uninit_stream_decoder();
        assert_eq!(init_stream_decoder(), 0);

        Engine { _guard: guard }
    }

    /// Stops the engine and retires any fade a test left pending, which nothing
    /// else would ever complete once the decoder thread is gone.
    fn shut_down(&self) {
        stop_stream(SOURCE);
        uninit_stream_decoder();

        stream::clear_music_fade();
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shut_down();
    }
}

fn chunks(count: usize) -> Vec<Vec<u8>> {
    (0..count).map(|i| vec![i as u8 + 1; 400]).collect()
}

/// The raw decoder handle `TFB_CreateSoundSample` expects: a boxed trait
/// object behind a thin pointer.
fn decoder_handle(count: usize) -> *mut c_void {
    let decoder: Box<dyn SoundDecoder> = Box::new(ScriptedDecoder::new(chunks(count)));
    Box::into_raw(Box::new(decoder)) as *mut c_void
}

fn sample(count: usize, buffers: c_uint, callbacks: *mut c_void) -> *mut c_void {
    unsafe { TFB_CreateSoundSample(decoder_handle(count), buffers, callbacks) }
}

static STARTS: AtomicU32 = AtomicU32::new(0);
static ENDS: AtomicU32 = AtomicU32::new(0);
static QUEUED: AtomicU32 = AtomicU32::new(0);
static REFUSE_START: AtomicU32 = AtomicU32::new(0);

unsafe extern "C" fn on_start(_sample: *mut c_void) -> c_int {
    STARTS.fetch_add(1, Ordering::SeqCst);
    (REFUSE_START.load(Ordering::SeqCst) == 0) as c_int
}

unsafe extern "C" fn on_end_chunk(_sample: *mut c_void, _buffer: c_uint) -> c_int {
    0
}

unsafe extern "C" fn on_end_stream(_sample: *mut c_void) {
    ENDS.fetch_add(1, Ordering::SeqCst);
}

unsafe extern "C" fn on_queue(_sample: *mut c_void, _buffer: c_uint) {
    QUEUED.fetch_add(1, Ordering::SeqCst);
}

fn reset_counters(refuse: bool) {
    STARTS.store(0, Ordering::SeqCst);
    ENDS.store(0, Ordering::SeqCst);
    QUEUED.store(0, Ordering::SeqCst);
    REFUSE_START.store(refuse as u32, Ordering::SeqCst);
}

fn callbacks_table() -> Box<CTfbSoundCallbacks> {
    Box::new(CTfbSoundCallbacks {
        on_start_stream: Some(on_start),
        on_end_chunk: Some(on_end_chunk),
        on_end_stream: Some(on_end_stream),
        on_tagged_buffer: None,
        on_queue_buffer: Some(on_queue),
    })
}

#[test]
fn the_stream_engine_starts_once_and_can_be_restarted() {
    let _guard = ENGINE_TEST_LOCK.lock();

    uninit_stream_decoder();
    assert_eq!(init_stream_decoder(), 0);
    assert_eq!(init_stream_decoder(), -1, "already running");
    uninit_stream_decoder();
    assert_eq!(init_stream_decoder(), 0, "a stopped engine starts again");
    uninit_stream_decoder();
}

#[test]
fn a_sound_sample_carries_opaque_user_data_and_is_destroyed_once() {
    let _engine = Engine::start();
    let sample = sample(2, 3, ptr::null_mut());
    assert!(!sample.is_null());
    unsafe {
        assert!(
            TFB_GetSoundSampleData(sample).is_null(),
            "nothing stored yet"
        );
        let marker = 0x1234usize as *mut c_void;
        TFB_SetSoundSampleData(sample, marker);
        assert_eq!(TFB_GetSoundSampleData(sample), marker);
        assert!(!TFB_GetSoundSampleDecoder(sample).is_null());
        TFB_DestroySoundSample(sample);
    }
}

#[test]
fn sample_calls_with_no_sample_do_nothing() {
    let _engine = Engine::start();
    unsafe {
        TFB_DestroySoundSample(ptr::null_mut());
        TFB_SetSoundSampleData(ptr::null_mut(), ptr::dangling_mut::<c_void>());
        assert!(TFB_GetSoundSampleData(ptr::null_mut()).is_null());
        TFB_SetSoundSampleCallbacks(ptr::null_mut(), ptr::null_mut());
        assert!(TFB_GetSoundSampleDecoder(ptr::null_mut()).is_null());
        assert!(TFB_FindTaggedBuffer(ptr::null_mut(), 0).is_null());
        assert_eq!(TFB_TagBuffer(ptr::null_mut(), 0, 7), 0);
        TFB_ClearBufferTag(ptr::null_mut());
        PlayStream(ptr::null_mut(), SOURCE, false, false, true);
        assert_eq!(playing_stream(SOURCE), 0);
    }
}

#[test]
fn a_sample_without_a_decoder_has_none_to_report() {
    let _engine = Engine::start();
    unsafe {
        let sample = TFB_CreateSoundSample(ptr::null_mut(), 2, ptr::null_mut());
        assert!(!sample.is_null());
        assert!(TFB_GetSoundSampleDecoder(sample).is_null());
        TFB_DestroySoundSample(sample);
    }
}

#[test]
fn buffers_are_tagged_found_and_untagged_up_to_the_sample_capacity() {
    let _engine = Engine::start();
    let sample = sample(1, 2, ptr::null_mut());
    unsafe {
        assert!(TFB_FindTaggedBuffer(sample, 5).is_null());
        assert_eq!(TFB_TagBuffer(sample, 5, 42), 1);
        assert_eq!(TFB_TagBuffer(sample, 6, 43), 1);
        assert_eq!(
            TFB_TagBuffer(sample, 7, 44),
            0,
            "a two-buffer sample has two tag slots"
        );
        assert!(!TFB_FindTaggedBuffer(sample, 5).is_null());
        assert!(!TFB_FindTaggedBuffer(sample, 6).is_null());
        assert!(TFB_FindTaggedBuffer(sample, 7).is_null());
        assert!(
            TFB_FindTaggedBuffer(sample, 0).is_null(),
            "handle zero is never a tag"
        );

        TFB_ClearBufferTag(TFB_FindTaggedBuffer(sample, 5));
        assert!(TFB_FindTaggedBuffer(sample, 5).is_null());
        assert_eq!(
            TFB_TagBuffer(sample, 7, 44),
            1,
            "the cleared slot is reused"
        );
        TFB_DestroySoundSample(sample);
    }
}

#[test]
fn a_started_stream_plays_until_stopped() {
    let _engine = Engine::start();
    let sample = sample(6, 4, ptr::null_mut());
    unsafe {
        assert_eq!(playing_stream(SOURCE), 0);
        PlayStream(sample, SOURCE, false, false, true);
        assert_eq!(playing_stream(SOURCE), 1);
        stop_stream(SOURCE);
        assert_eq!(playing_stream(SOURCE), 0);
        TFB_DestroySoundSample(sample);
    }
}

#[test]
fn pausing_halts_a_stream_and_resuming_restarts_it() {
    let _engine = Engine::start();
    let sample = sample(6, 4, ptr::null_mut());
    unsafe {
        PlayStream(sample, SOURCE, false, false, true);
        assert_eq!(playing_stream(SOURCE), 1);
        pause_stream(SOURCE);
        assert_eq!(playing_stream(SOURCE), 0, "paused");
        resume_stream(SOURCE);
        assert_eq!(playing_stream(SOURCE), 1);
        seek_stream(SOURCE, 0);
        assert_eq!(playing_stream(SOURCE), 1);
        stop_stream(SOURCE);
        TFB_DestroySoundSample(sample);
    }
}

#[test]
fn stream_controls_on_an_unknown_source_are_ignored() {
    let _engine = Engine::start();

    stop_stream(9999);
    pause_stream(9999);
    resume_stream(9999);
    seek_stream(9999, 5);
    assert_eq!(playing_stream(9999), 0);
}

#[test]
fn the_c_callbacks_are_told_when_a_stream_starts_and_queues_buffers() {
    let _engine = Engine::start();
    reset_counters(false);
    let mut table = callbacks_table();
    let table_ptr = &mut *table as *mut CTfbSoundCallbacks as *mut c_void;
    let sample = sample(3, 4, table_ptr);
    unsafe {
        PlayStream(sample, SOURCE, false, false, true);
        assert_eq!(STARTS.load(Ordering::SeqCst), 1);
        assert_eq!(playing_stream(SOURCE), 1);
        assert!(
            QUEUED.load(Ordering::SeqCst) >= 1,
            "prefilled buffers are announced"
        );
        stop_stream(SOURCE);
        TFB_DestroySoundSample(sample);
    }
}

#[test]
fn a_c_callback_can_refuse_to_start_a_stream() {
    let _engine = Engine::start();
    reset_counters(true);
    let mut table = callbacks_table();
    let table_ptr = &mut *table as *mut CTfbSoundCallbacks as *mut c_void;
    let sample = sample(3, 4, table_ptr);
    unsafe {
        PlayStream(sample, SOURCE, false, false, true);
        assert_eq!(STARTS.load(Ordering::SeqCst), 1);
        assert_eq!(playing_stream(SOURCE), 0, "refused");
        TFB_DestroySoundSample(sample);
    }
}

#[test]
fn callbacks_can_be_replaced_after_the_sample_exists() {
    let _engine = Engine::start();
    reset_counters(false);
    let mut table = callbacks_table();
    let table_ptr = &mut *table as *mut CTfbSoundCallbacks as *mut c_void;
    let sample = sample(3, 4, ptr::null_mut());
    unsafe {
        TFB_SetSoundSampleCallbacks(sample, table_ptr);
        PlayStream(sample, SOURCE, false, false, true);
        assert_eq!(STARTS.load(Ordering::SeqCst), 1);
        stop_stream(SOURCE);
        TFB_SetSoundSampleCallbacks(sample, ptr::null_mut());
        PlayStream(sample, SOURCE, false, false, true);
        assert_eq!(STARTS.load(Ordering::SeqCst), 1, "no callbacks any more");
        stop_stream(SOURCE);
        TFB_DestroySoundSample(sample);
    }
}

#[test]
fn the_oscilloscope_reads_nothing_without_arguments_or_a_stream() {
    let _engine = Engine::start();
    let mut data = vec![0u8; 64];
    unsafe {
        assert_eq!(GraphForegroundStream(ptr::null_mut(), 64, 32, 0), 0);
        assert_eq!(GraphForegroundStream(data.as_mut_ptr(), 0, 32, 0), 0);
        assert_eq!(GraphForegroundStream(data.as_mut_ptr(), 64, -1, 0), 0);
        assert_eq!(
            GraphForegroundStream(data.as_mut_ptr(), 64, 32, 0),
            0,
            "no foreground stream"
        );
    }
}

#[test]
fn music_fades_are_scheduled_only_when_they_take_time() {
    let _engine = Engine::start();

    assert!(
        !set_music_stream_fade(0, 10),
        "a zero-length fade is not scheduled"
    );
    assert!(set_music_stream_fade(100, 10));
    assert!(
        !set_music_stream_fade(-5, 10),
        "a negative length counts as zero"
    );
}

#[test]
fn shutting_the_engine_down_retires_the_fade_a_test_scheduled() {
    let engine = Engine::start();

    assert!(set_music_stream_fade(100, 10));

    assert_eq!(stream::music_fade_interval(), 100);

    engine.shut_down();

    assert_eq!(
        stream::music_fade_interval(),
        0,
        "a pending fade would be read by the next test as the engine's idle state"
    );
}

#[test]
fn fading_music_with_no_time_sets_the_volume_straight_away() {
    let _engine = Engine::start();

    let now = crate::sound::types::get_time_counter();
    let until = fade_music(40, 0);
    assert!(until >= now);
    assert_eq!(crate::sound::music::current_music_volume(), 40);
    let until = fade_music(60, -5);
    assert!(until >= now, "a negative duration is an immediate change");
    assert_eq!(crate::sound::music::current_music_volume(), 60);
    set_music_volume(25);
    assert_eq!(crate::sound::music::current_music_volume(), 25);
}

#[test]
fn music_calls_without_music_do_nothing() {
    let _engine = Engine::start();
    let sentinel = usize::MAX as *mut c_void;
    unsafe {
        PLRPlaySong(ptr::null_mut(), 0, 1);
        PLRPlaySong(sentinel, 0, 1);
        PLRStop(ptr::null_mut());
        PLRStop(sentinel);
        assert_eq!(PLRPlaying(ptr::null_mut()), 0);
        assert_eq!(PLRPlaying(sentinel), 0);
        PLRSeek(ptr::null_mut(), 5);
        PLRSeek(sentinel, 5);
        PLRPause(ptr::null_mut());
        PLRResume(ptr::null_mut());
        snd_PlaySpeech(ptr::null_mut());
        snd_PlaySpeech(sentinel);
        snd_stop_speech();
    }
}

#[test]
fn channel_calls_ignore_missing_sounds_and_idle_channels() {
    let _engine = Engine::start();
    let pos = SoundPosition {
        positional: false,
        x: 0,
        y: 0,
    };
    unsafe {
        PlayChannel(0, ptr::null_mut(), pos, ptr::null_mut(), 1);
        assert_eq!(channel_playing(0), 0);
        stop_channel(0, 1);
        set_channel_volume(0, 100, 1);
        UpdateSoundPosition(0, ptr::null());
    }
}

#[test]
fn a_channel_remembers_its_positional_object() {
    let _engine = Engine::start();
    unsafe {
        SetPositionalObject(1, 0xABCD);
        assert_eq!(GetPositionalObject(1), 0xABCD);
        SetPositionalObject(1, 0);
        assert_eq!(GetPositionalObject(1), 0);
        let pos = SoundPosition {
            positional: true,
            x: 3,
            y: -4,
        };
        UpdateSoundPosition(1, &pos);
    }
}

#[test]
fn stopping_all_sound_stops_effect_sources() {
    let _engine = Engine::start();
    let sample = sample(6, 4, ptr::null_mut());
    unsafe {
        set_sfx_volume(0.5);
        set_speech_volume(0.25);
        PlayStream(sample, 0, false, false, true);
        assert_eq!(playing_stream(0), 1);
        stop_sound();
        assert_eq!(
            playing_stream(0),
            0,
            "stopping all sound stops effect sources"
        );
        TFB_DestroySoundSample(sample);
    }
}

#[test]
fn the_track_player_exports_are_quiet_with_no_track() {
    let _engine = Engine::start();
    unsafe {
        stop_track();
        pause_track();
        resume_track();
        jump_track();
        fast_forward_smooth();
        fast_reverse_smooth();
        fast_forward_page();
        fast_reverse_page();
        assert_eq!(playing_track(), 0);
        assert_eq!(get_track_position(1), 0);
        assert!(get_first_track_subtitle().is_null());
        assert!(get_next_track_subtitle(ptr::null_mut()).is_null());
        assert!(get_track_subtitle_text(ptr::null_mut()).is_null());
        splice_multi_track(ptr::null(), ptr::null());
    }
}

#[test]
fn loading_files_that_are_not_there_gives_nothing() {
    let _engine = Engine::start();
    let missing = CString::new("no/such/bank.snd").unwrap();
    let bad = [0xffu8, 0xfe, 0];
    unsafe {
        assert!(LoadSoundFile(ptr::null()).is_null());
        assert!(LoadSoundFile(bad.as_ptr() as *const _).is_null());
        assert!(LoadSoundFile(missing.as_ptr()).is_null());
        assert!(LoadMusicFile(ptr::null()).is_null());
        assert!(LoadMusicFile(missing.as_ptr()).is_null());
        DestroySound(ptr::null_mut());
        DestroyMusic(ptr::null_mut());
    }
}

#[test]
fn speech_controls_have_safe_rust_signatures() {
    let _: fn() = play_track;
    let _: fn() = stop_track;
    let _: fn() = jump_track;
    let _: fn() = pause_track;
    let _: fn() = resume_track;
    let _: fn() -> u16 = playing_track;
    let _: fn() = fast_reverse_smooth;
    let _: fn() = fast_forward_smooth;
    let _: fn() = fast_reverse_page;
    let _: fn() = fast_forward_page;
    let _: fn(c_int) -> c_int = get_track_position;
    let _: fn() -> *const std::ffi::c_char = get_track_subtitle;
    let _: fn() -> *mut c_void = get_first_track_subtitle;
    let _: Option<unsafe fn()> = None::<super::types::PhraseCallback>;
}

#[test]
fn audio_controls_have_safe_rust_signatures() {
    let _: fn() -> c_int = init_stream_decoder;
    let _: fn() = uninit_stream_decoder;
    let _: fn(source: u32) = stop_stream;
    let _: fn(source: u32) = pause_stream;
    let _: fn(source: u32) = resume_stream;
    let _: fn(source: u32, pos: u32) = seek_stream;
    let _: fn(source: u32) -> u8 = playing_stream;
    let _: fn(how_long: i32, end_volume: c_int) -> bool = set_music_stream_fade;
    let _: fn() = snd_stop_speech;
    let _: fn(volume: c_int) = set_music_volume;
    let _: fn(end_vol: u8, how_long: i16) -> u32 = fade_music;
    let _: fn(channel: c_uint, priority: c_int) = stop_channel;
    let _: fn(channel: c_uint) -> c_int = channel_playing;
    let _: fn(channel: c_uint, volume: c_int, priority: c_int) = set_channel_volume;
    let _: fn() = uninit_sound;
    let _: fn() = stop_sound;
    let _: fn() -> c_int = sound_playing;
    let _: fn(channel: u16) = wait_for_sound_end;
    let _: fn(volume: f32) = set_sfx_volume;
    let _: fn(volume: f32) = set_speech_volume;
}
