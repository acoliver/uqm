//! The audio-heart facade driven the way the game modes drive it: samples
//! created and tagged, streams started on mixer sources, stream callbacks
//! invoked, and the channel, music and control calls that sit on top.
//!
//! Streams live in process-wide statics, so every test takes
//! `ENGINE_TEST_LOCK` and brings the stream engine up and down itself.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;

use super::heart_ffi::*;
use super::test_support::{ScriptedDecoder, ENGINE_TEST_LOCK};
use super::types::{SoundPosition, SoundSample, StreamCallbacks, MUSIC_SOURCE};
use super::{stream, trackplayer};

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

fn sample(
    count: usize,
    buffers: u32,
    callbacks: Option<Box<dyn StreamCallbacks + Send>>,
) -> Arc<Mutex<SoundSample>> {
    let decoder = Box::new(ScriptedDecoder::new(chunks(count)));
    Arc::new(Mutex::new(
        stream::create_sound_sample(Some(decoder), buffers, callbacks).unwrap(),
    ))
}

/// Start the stream; a callback that refuses leaves it stopped, which the tests observe.
fn play(sample: &Arc<Mutex<SoundSample>>) {
    let _ = stream::play_stream(sample.clone(), SOURCE as usize, false, false, true);
}

/// What the stream engine told the callbacks.
#[derive(Default)]
struct Counters {
    starts: AtomicU32,
    queued: AtomicU32,
}

struct Observer {
    counters: Arc<Counters>,
    refuse_start: bool,
}

impl StreamCallbacks for Observer {
    fn on_start_stream(&mut self, _sample: &mut SoundSample) -> bool {
        self.counters.starts.fetch_add(1, Ordering::SeqCst);
        !self.refuse_start
    }
    fn on_end_chunk(&mut self, _sample: &mut SoundSample, _buffer: usize) -> bool {
        false
    }
    fn on_queue_buffer(&mut self, _sample: &mut SoundSample, _buffer: usize) {
        self.counters.queued.fetch_add(1, Ordering::SeqCst);
    }
}

fn observer(refuse_start: bool) -> (Arc<Counters>, Box<dyn StreamCallbacks + Send>) {
    let counters = Arc::new(Counters::default());
    (
        counters.clone(),
        Box::new(Observer {
            counters,
            refuse_start,
        }),
    )
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
fn a_sound_sample_carries_user_data_and_reports_its_decoder() {
    let _engine = Engine::start();
    let sample = sample(2, 3, None);
    let mut sample = sample.lock();
    assert!(
        stream::get_sound_sample_data(&sample).is_none(),
        "nothing stored yet"
    );
    stream::set_sound_sample_data(&mut sample, Box::new(0x1234usize));
    let stored = stream::get_sound_sample_data(&sample).unwrap();
    assert_eq!(stored.downcast_ref::<usize>(), Some(&0x1234));
    assert!(stream::get_sound_sample_decoder(&sample).is_some());
    stream::destroy_sound_sample(&mut sample).unwrap();
}

#[test]
fn a_sample_without_a_decoder_has_none_to_report() {
    let _engine = Engine::start();
    let mut sample = stream::create_sound_sample(None, 2, None).unwrap();
    assert!(stream::get_sound_sample_decoder(&sample).is_none());
    stream::destroy_sound_sample(&mut sample).unwrap();
}

#[test]
fn buffers_are_tagged_found_and_untagged_up_to_the_sample_capacity() {
    let _engine = Engine::start();
    let sample = sample(1, 2, None);
    let mut sample = sample.lock();
    assert!(stream::find_tagged_buffer(&sample, 5).is_none());
    assert!(stream::tag_buffer(&mut sample, 5, 42));
    assert!(stream::tag_buffer(&mut sample, 6, 43));
    assert!(
        !stream::tag_buffer(&mut sample, 7, 44),
        "a two-buffer sample has two tag slots"
    );
    assert!(stream::find_tagged_buffer(&sample, 5).is_some());
    assert!(stream::find_tagged_buffer(&sample, 6).is_some());
    assert!(stream::find_tagged_buffer(&sample, 7).is_none());
    assert!(
        stream::find_tagged_buffer(&sample, 0).is_none(),
        "handle zero is never a tag"
    );

    let tag = sample
        .buffer_tags
        .iter_mut()
        .flatten()
        .find(|tag| tag.buf_handle == 5)
        .unwrap();
    stream::clear_buffer_tag(tag);
    assert!(stream::find_tagged_buffer(&sample, 5).is_none());
    assert!(
        stream::tag_buffer(&mut sample, 7, 44),
        "the cleared slot is reused"
    );
    stream::destroy_sound_sample(&mut sample).unwrap();
}

#[test]
fn a_started_stream_plays_until_stopped() {
    let _engine = Engine::start();
    let sample = sample(6, 4, None);
    assert_eq!(playing_stream(SOURCE), 0);
    play(&sample);
    assert_eq!(playing_stream(SOURCE), 1);
    stop_stream(SOURCE);
    assert_eq!(playing_stream(SOURCE), 0);
    stream::destroy_sound_sample(&mut sample.lock()).unwrap();
}

#[test]
fn pausing_halts_a_stream_and_resuming_restarts_it() {
    let _engine = Engine::start();
    let sample = sample(6, 4, None);
    play(&sample);
    assert_eq!(playing_stream(SOURCE), 1);
    pause_stream(SOURCE);
    assert_eq!(playing_stream(SOURCE), 0, "paused");
    resume_stream(SOURCE);
    assert_eq!(playing_stream(SOURCE), 1);
    seek_stream(SOURCE, 0);
    assert_eq!(playing_stream(SOURCE), 1);
    stop_stream(SOURCE);
    stream::destroy_sound_sample(&mut sample.lock()).unwrap();
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
fn the_callbacks_are_told_when_a_stream_starts_and_queues_buffers() {
    let _engine = Engine::start();
    let (counters, callbacks) = observer(false);
    let sample = sample(3, 4, Some(callbacks));
    play(&sample);
    assert_eq!(counters.starts.load(Ordering::SeqCst), 1);
    assert_eq!(playing_stream(SOURCE), 1);
    assert!(
        counters.queued.load(Ordering::SeqCst) >= 1,
        "prefilled buffers are announced"
    );
    stop_stream(SOURCE);
    stream::destroy_sound_sample(&mut sample.lock()).unwrap();
}

#[test]
fn a_callback_can_refuse_to_start_a_stream() {
    let _engine = Engine::start();
    let (counters, callbacks) = observer(true);
    let sample = sample(3, 4, Some(callbacks));
    play(&sample);
    assert_eq!(counters.starts.load(Ordering::SeqCst), 1);
    assert_eq!(playing_stream(SOURCE), 0, "refused");
    stream::destroy_sound_sample(&mut sample.lock()).unwrap();
}

#[test]
fn callbacks_can_be_replaced_after_the_sample_exists() {
    let _engine = Engine::start();
    let (counters, callbacks) = observer(false);
    let sample = sample(3, 4, None);
    stream::set_sound_sample_callbacks(&mut sample.lock(), Some(callbacks));
    play(&sample);
    assert_eq!(counters.starts.load(Ordering::SeqCst), 1);
    stop_stream(SOURCE);
    stream::set_sound_sample_callbacks(&mut sample.lock(), None);
    play(&sample);
    assert_eq!(
        counters.starts.load(Ordering::SeqCst),
        1,
        "no callbacks any more"
    );
    stop_stream(SOURCE);
    stream::destroy_sound_sample(&mut sample.lock()).unwrap();
}

#[test]
fn the_oscilloscope_reads_nothing_without_arguments_or_a_stream() {
    let _engine = Engine::start();
    let mut data = vec![0u8; 64];
    assert_eq!(stream::graph_foreground_stream(&mut data, 0, 32, false), 0);
    assert_eq!(stream::graph_foreground_stream(&mut data, 64, 0, false), 0);
    assert_eq!(
        stream::graph_foreground_stream(&mut data, 64, 32, false),
        0,
        "no foreground stream"
    );
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
fn speech_stop_without_speech_does_nothing() {
    let _engine = Engine::start();
    snd_stop_speech();
}

#[test]
fn channel_calls_ignore_idle_channels() {
    let _engine = Engine::start();
    assert_eq!(channel_playing(0), 0);
    stop_channel(0, 1);
    set_channel_volume(0, 100, 1);
}

#[test]
fn channel_playback_retains_the_full_object_address_on_the_selected_channel() {
    let _engine = Engine::start();
    let mut sample = stream::create_sound_sample(None, 1, None).unwrap();
    super::mixer::buffer::mixer_buffer_data(
        sample.buffers[0],
        super::mixer::MixerFormat::Mono16 as u32,
        &[0; 128],
        22050,
        22050,
        super::mixer::MixerFormat::Stereo16,
    )
    .unwrap();
    let object = usize::MAX - 0xABCD;
    set_positional_object(2, 42);
    super::sfx::play_sample(1, &sample, SoundPosition::non_positional(), object, 255).unwrap();
    assert_eq!(channel_playing(1), 1);
    assert_eq!(get_positional_object(1), object);
    assert_eq!(get_positional_object(2), 42);
    stop_channel(1, 0);
    assert_eq!(channel_playing(1), 0);
    set_positional_object(1, 0);
    set_positional_object(2, 0);
    stream::destroy_sound_sample(&mut sample).unwrap();
}

#[test]
fn finished_channel_cleanup_uses_the_mixer_stopped_state() {
    let _engine = Engine::start();
    let sample = std::sync::Arc::new(parking_lot::Mutex::new(
        stream::create_sound_sample(None, 0, None).unwrap(),
    ));
    let handle = stream::with_source(1, |source| {
        source.sample = Some(sample);
        source.handle
    })
    .unwrap();
    super::mixer::source::mixer_source_stop(handle).unwrap();
    super::sfx::check_finished_channels();
    assert!(stream::with_source(1, |source| source.sample.is_none()).unwrap());
}

#[test]
#[cfg(target_pointer_width = "64")]
fn positional_object_preserves_a_64_bit_element_address() {
    let _engine = Engine::start();
    let object = u32::MAX as usize + 0xABCD;
    set_positional_object(1, object);
    assert_eq!(get_positional_object(1), object);
    set_positional_object(1, 0);
    assert_eq!(get_positional_object(1), 0);
}

#[test]
fn positional_update_changes_the_selected_mixer_source() {
    use super::mixer::source::mixer_get_source_f;
    use super::mixer::SourceProp;

    let _engine = Engine::start();
    let handle = stream::with_source(1, |source| source.handle).unwrap();
    let other = stream::with_source(2, |source| source.handle).unwrap();
    let position = || {
        [
            SourceProp::PositionX,
            SourceProp::PositionY,
            SourceProp::PositionZ,
        ]
        .map(|prop| mixer_get_source_f(handle, prop).unwrap())
    };
    update_sound_position(
        1,
        SoundPosition {
            positional: true,
            x: 320,
            y: -160,
        },
    );
    assert_eq!(position(), [2.0, 0.0, -1.0]);
    assert_eq!(
        mixer_get_source_f(other, SourceProp::PositionX).unwrap(),
        0.0
    );

    update_sound_position(
        1,
        SoundPosition {
            positional: true,
            x: 48,
            y: 64,
        },
    );
    assert_eq!(position(), [0.3, 0.0, 0.4]);

    update_sound_position(1, SoundPosition::non_positional());
    assert_eq!(position(), [0.0, 0.0, -1.0]);
}

#[test]
fn stopping_all_sound_stops_effect_sources() {
    let _engine = Engine::start();
    let sample = sample(6, 4, None);
    set_sfx_volume(0.5);
    set_speech_volume(0.25);
    stream::play_stream(sample.clone(), 0, false, false, true).unwrap();
    assert_eq!(playing_stream(0), 1);
    stop_sound();
    assert_eq!(
        playing_stream(0),
        0,
        "stopping all sound stops effect sources"
    );
    stream::destroy_sound_sample(&mut sample.lock()).unwrap();
}

#[test]
fn the_track_player_exports_are_quiet_with_no_track() {
    let _engine = Engine::start();
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
    assert!(trackplayer::subtitle_snapshot().is_none());
    assert!(trackplayer::subtitle_history().is_empty());
    splice_multi_track(None, &[], None);
}

#[test]
fn loading_files_that_are_not_there_gives_nothing() {
    let mount = crate::io::fixture::TempMount::new();
    let files = mount.dir();
    let _engine = Engine::start();
    assert!(load_sound_bank(&files, "no/such/bank.snd").is_err());
    assert!(load_music(&files, "no/such/music.ogg").is_err());
    assert!(load_music(&files, "noextension").is_err());
}

#[test]
fn music_destruction_releases_the_sample_and_its_mixer_buffers() {
    let _engine = Engine::start();
    let sample = stream::create_sound_sample(None, 1, None).unwrap();
    let buffer = sample.buffers[0];
    let sample = Arc::new(Mutex::new(sample));
    let weak = Arc::downgrade(&sample);
    destroy_music(super::types::MusicRef(sample));
    assert!(weak.upgrade().is_none());
    assert!(!super::mixer::buffer::mixer_is_buffer(buffer));
}

#[test]
fn sound_bank_skips_failed_samples_without_losing_a_good_relative_sample() {
    let _engine = Engine::start();
    let mount = crate::io::fixture::TempMount::new();
    mount.write("sounds/bank.snd", b"missing.wav\nbad.wav\ngood.wav\n");
    mount.write("sounds/bad.wav", b"corrupt WAV");
    let mut wave = b"RIFF".to_vec();
    wave.extend_from_slice(&38u32.to_le_bytes());
    wave.extend_from_slice(b"WAVEfmt ");
    wave.extend_from_slice(&16u32.to_le_bytes());
    for word in [1u16, 1] {
        wave.extend_from_slice(&word.to_le_bytes());
    }
    for word in [22050u32, 22050] {
        wave.extend_from_slice(&word.to_le_bytes());
    }
    for word in [1u16, 8] {
        wave.extend_from_slice(&word.to_le_bytes());
    }
    wave.extend_from_slice(b"data");
    wave.extend_from_slice(&2u32.to_le_bytes());
    wave.extend_from_slice(&[128, 129]);
    mount.write("sounds/good.wav", &wave);
    let bank = load_sound_bank(&mount.dir(), "sounds/bank.snd").unwrap();
    assert_eq!(bank.len(), 1);
}
