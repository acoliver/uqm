//! The mixer driven the way the sound layer drives it: initialise, fill
//! buffers, queue them on a source, play, and read back what the mixer
//! renders into the output stream.

use super::*;
use crate::sound::test_support::ENGINE_TEST_LOCK;

/// MIX_FORMAT_MAKE(bytes per channel, channels) as the C headers encode it.
const C_STEREO16: u32 = 0x0017_0202;
const C_MONO16: u32 = 0x0017_0102;
const RATE: u32 = 44_100;

thread_local! {
    static SOURCES: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
    static BUFFERS: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The mixer pools outlive a mixer instance, so each test removes the
/// objects it created before the next one starts. The mixer renders every
/// playing source in the process-wide pool, so the test holds
/// `ENGINE_TEST_LOCK`, the lock of everything else that plays on that pool,
/// from `start` until its objects are gone.
struct Mixer {
    _lease: parking_lot::MutexGuard<'static, ()>,
}

impl Mixer {
    fn start() -> Self {
        Self::start_with_flags(MixerFlags::None)
    }

    fn start_with_flags(flags: MixerFlags) -> Self {
        let lease = ENGINE_TEST_LOCK.lock();
        let _ = mixer_uninit();
        mixer_init(RATE, MixerFormat::Stereo16, MixerQuality::Medium, flags).unwrap();
        Mixer { _lease: lease }
    }
}

impl Drop for Mixer {
    fn drop(&mut self) {
        for source in SOURCES.take() {
            let _ = mixer_source_stop(source);
            let _ = mixer_delete_sources(&[source]);
        }
        for buffer in BUFFERS.take() {
            let _ = mixer_delete_buffers(&[buffer]);
        }
        let _ = mixer_uninit();
    }
}

fn pcm(samples: &[i16]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_le_bytes()).collect()
}

fn read_pcm(bytes: &[u8]) -> Vec<i16> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_le_bytes(*b))
        .collect()
}

fn upload(buffer: usize, format: u32, data: &[u8]) -> Result<(), MixerError> {
    mixer_buffer_data(
        buffer,
        format,
        data,
        RATE,
        mixer_get_frequency(),
        mixer_get_format(),
    )
}

fn buffer_with(format: u32, samples: &[i16]) -> usize {
    let handle = mixer_gen_buffers(1).unwrap()[0];
    upload(handle, format, &pcm(samples)).unwrap();
    BUFFERS.with_borrow_mut(|all| all.push(handle));
    handle
}

fn source() -> usize {
    let handle = mixer_gen_sources(1).unwrap()[0];
    SOURCES.with_borrow_mut(|all| all.push(handle));
    handle
}

fn queue(source: usize, buffer: usize) {
    mixer_source_queue_buffers(source, &[buffer]).unwrap();
}

fn state_of(source: usize, prop: SourceProp) -> i32 {
    mixer_get_source_i(source, prop).unwrap()
}

fn render(frames: usize) -> Vec<i16> {
    let mut stream = vec![0x55u8; frames * 4];
    mixer_mix_channels(&mut stream).unwrap();
    read_pcm(&stream)
}

fn play(source: usize) {
    mixer_source_play(source).unwrap();
}

const STATE_INITIAL: i32 = SourceState::Initial as i32;
const STATE_STOPPED: i32 = SourceState::Stopped as i32;
const STATE_PLAYING: i32 = SourceState::Playing as i32;
const STATE_PAUSED: i32 = SourceState::Paused as i32;
use SourceProp::{BuffersProcessed as PROCESSED, BuffersQueued as QUEUED, Looping as LOOPING, SourceState as STATE};

#[test]
fn init_reports_the_negotiated_frequency_and_format() {
    let _mixer = Mixer::start();

    assert_eq!(mixer_get_frequency(), RATE);
    assert_eq!(mixer_get_format(), MixerFormat::Stereo16);
    assert_eq!(mixer_get_error(), MixerError::NoError);
}

#[test]
fn buffer_data_is_recorded_with_its_original_properties() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[1, -1, 2, -2, 3, -3]);
    let value = |prop| mixer_get_buffer_i(buffer, prop).unwrap();
    assert_eq!(value(BufferProp::Frequency), RATE as i32);
    assert_eq!(value(BufferProp::Channels), 2);
    assert_eq!(value(BufferProp::Bits), 16);
    assert_eq!(value(BufferProp::Size), 12);
}

#[test]
pub(crate) fn a_queued_source_plays_its_samples_then_stops_with_the_buffer_processed() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[400, -400, 800, -800, 1200, -1200]);
    let source = source();
    assert_eq!(state_of(source, STATE), STATE_INITIAL);
    queue(source, buffer);
    assert_eq!(state_of(source, QUEUED), 1);

    play(source);
    assert_eq!(state_of(source, STATE), STATE_PLAYING);

    let out = render(4);
    assert_eq!(&out[..6], &[300, -300, 600, -600, 900, -900]);
    assert_eq!(
        &out[6..],
        &[0, 0],
        "past the end of the data the stream is silent"
    );
    assert_eq!(state_of(source, STATE), STATE_STOPPED);
    assert_eq!(state_of(source, PROCESSED), 1);

    assert_eq!(mixer_source_unqueue_buffers(source, 1).unwrap(), [buffer]);
    assert_eq!(state_of(source, QUEUED), 0);
}

#[test]
fn gain_scales_what_reaches_the_stream() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[1000, -1000, 2000, -2000]);
    let source = source();
    queue(source, buffer);
    mixer_source_f(source, SourceProp::Gain, 0.5).unwrap();
    let gain = mixer_get_source_f(source, SourceProp::Gain).unwrap();
    assert!((gain - 0.5).abs() < 1e-6);
    play(source);
    let out = render(2);
    assert_eq!(out, [375, -375, 750, -750]);
}

#[test]
fn the_looping_flag_is_kept_on_the_source_and_stopping_clears_the_queue() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[40, 80, 120, 160]);
    let source = source();
    queue(source, buffer);

    mixer_source_i(source, LOOPING, 1).unwrap();
    play(source);

    assert_eq!(state_of(source, LOOPING), 1);
    assert_eq!(render(2), [30, 60, 90, 120]);

    mixer_source_stop(source).unwrap();
    assert_eq!(state_of(source, STATE), STATE_STOPPED);
    assert_eq!(state_of(source, QUEUED), 0, "stopping unqueues everything");
    assert_eq!(render(2), [0, 0, 0, 0]);
}

#[test]
fn pausing_silences_the_source_and_resuming_continues_where_it_left_off() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[40, 80, 120, 160, 200, 240, 280, 320]);
    let source = source();
    queue(source, buffer);
    play(source);
    assert_eq!(render(1), [30, 60]);

    mixer_source_pause(source).unwrap();
    assert_eq!(state_of(source, STATE), STATE_PAUSED);
    assert_eq!(render(1), [0, 0]);

    play(source);
    assert_eq!(render(1), [90, 120]);
}

#[test]
fn rewinding_restarts_the_queue_from_the_first_sample() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[44, 48, 52, 56]);
    let source = source();
    queue(source, buffer);
    play(source);
    render(1);

    mixer_source_rewind(source).unwrap();
    assert_eq!(state_of(source, STATE), STATE_INITIAL);
    play(source);

    assert_eq!(render(1), [33, 36]);
}

#[test]
fn two_sources_are_summed_into_the_stream() {
    let _mixer = Mixer::start();
    let first = source();
    let second = source();
    queue(first, buffer_with(C_STEREO16, &[400, 800]));
    queue(second, buffer_with(C_STEREO16, &[40, 80]));

    play(first);
    play(second);

    assert_eq!(render(1), [330, 660]);
}

#[test]
fn queued_buffers_play_back_to_back() {
    let _mixer = Mixer::start();
    let source = source();
    let buffers = [
        buffer_with(C_STEREO16, &[40, 40]),
        buffer_with(C_STEREO16, &[80, 80]),
    ];
    mixer_source_queue_buffers(source, &buffers).unwrap();
    assert_eq!(state_of(source, QUEUED), 2);
    play(source);
    assert_eq!(render(2), [30, 30, 60, 60]);
    assert_eq!(state_of(source, PROCESSED), 2);
}

#[test]
fn deleting_objects_invalidates_their_handles() {
    let _mixer = Mixer::start();
    let sources = [source(), source()];
    let buffer = buffer_with(C_STEREO16, &[1, 2]);
    assert!(mixer_is_source(sources[0]));
    assert!(mixer_is_buffer(buffer));
    mixer_delete_sources(&sources).unwrap();
    mixer_delete_buffers(&[buffer]).unwrap();
    assert!(!mixer_is_source(sources[0]));
    assert!(!mixer_is_source(sources[1]));
    assert!(!mixer_is_buffer(buffer));
}

#[test]
fn empty_requests_are_refused_and_change_nothing() {
    let _mixer = Mixer::start();
    let source = source();
    assert!(mixer_gen_sources(0).unwrap_or_default().is_empty());
    assert!(mixer_gen_buffers(0).unwrap_or_default().is_empty());
    assert!(mixer_source_unqueue_buffers(source, 1).is_err());
    assert!(mixer_mix_channels(&mut []).is_ok());
    assert!(mixer_mix_fake(&mut []).is_ok());
    assert_eq!(state_of(source, STATE), STATE_INITIAL);
    assert_eq!(state_of(source, QUEUED), 0);
}

#[test]
fn a_zero_sized_or_malformed_buffer_upload_is_refused() {
    let _mixer = Mixer::start();
    let handle = mixer_gen_buffers(1).unwrap()[0];
    BUFFERS.with_borrow_mut(|all| all.push(handle));
    assert!(upload(handle, C_STEREO16, &[]).is_err());
    let _ = upload(handle, 0, &[0u8; 8]);
    assert_eq!(
        mixer_get_buffer_i(handle, BufferProp::Size).unwrap(),
        0,
        "neither upload took effect"
    );
}

#[test]
fn mono_buffers_are_accepted_by_a_stereo_mixer() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_MONO16, &[5, 6, 7, 8]);
    assert_eq!(mixer_get_buffer_i(buffer, BufferProp::Channels).unwrap(), 1);
    assert_eq!(mixer_get_buffer_i(buffer, BufferProp::Size).unwrap(), 8);
}

#[test]
fn fake_mode_renders_without_real_buffers() {
    let _mixer = Mixer::start_with_flags(MixerFlags::FakeData);
    let source = source();
    play(source);
    let mut stream = [0u8; 16];
    mixer_mix_fake(&mut stream).unwrap();
}

#[test]
fn every_test_that_drives_the_mixer_waits_for_the_engine_lock() {
    use crate::sound::test_support::Probe;

    let bodies: [fn(); 14] = [
        init_reports_the_negotiated_frequency_and_format,
        buffer_data_is_recorded_with_its_original_properties,
        a_queued_source_plays_its_samples_then_stops_with_the_buffer_processed,
        gain_scales_what_reaches_the_stream,
        the_looping_flag_is_kept_on_the_source_and_stopping_clears_the_queue,
        pausing_silences_the_source_and_resuming_continues_where_it_left_off,
        rewinding_restarts_the_queue_from_the_first_sample,
        two_sources_are_summed_into_the_stream,
        queued_buffers_play_back_to_back,
        deleting_objects_invalidates_their_handles,
        empty_requests_are_refused_and_change_nothing,
        a_zero_sized_or_malformed_buffer_upload_is_refused,
        mono_buffers_are_accepted_by_a_stereo_mixer,
        fake_mode_renders_without_real_buffers,
    ];
    let engine = ENGINE_TEST_LOCK.lock();
    let probes = bodies.map(Probe::spawn);
    let blocked = Probe::all_blocked_for_grace(&probes);
    drop(engine);

    let finished: Vec<bool> = probes.into_iter().map(|p| p.join().is_ok()).collect();
    assert!(blocked, "a mixer test ran without the engine lock");
    assert_eq!(finished, [true; 14]);
}
