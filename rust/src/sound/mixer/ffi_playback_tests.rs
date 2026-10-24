//! The C mixer entry points driven the way the C sound layer drives them:
//! initialise, fill buffers, queue them on a source, play, and read back what
//! the mixer renders into the output stream.

use super::*;
use crate::sound::test_support::ENGINE_TEST_LOCK;
use std::ptr;

/// MIX_FORMAT_MAKE(bytes per channel, channels) as the C headers encode it.
const C_STEREO16: c_uint = 0x0017_0202;
const C_MONO16: c_uint = 0x0017_0102;
const RATE: c_uint = 44_100;

const LOOPING: c_uint = SourceProp::Looping as c_uint;
const GAIN: c_uint = SourceProp::Gain as c_uint;
const STATE: c_uint = SourceProp::SourceState as c_uint;
const QUEUED: c_uint = SourceProp::BuffersQueued as c_uint;
const PROCESSED: c_uint = SourceProp::BuffersProcessed as c_uint;

thread_local! {
    static SOURCES: std::cell::RefCell<Vec<MixerObject>> = const { std::cell::RefCell::new(Vec::new()) };
    static BUFFERS: std::cell::RefCell<Vec<MixerObject>> = const { std::cell::RefCell::new(Vec::new()) };
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
        Self::start_with_flags(0)
    }

    fn start_with_flags(flags: c_uint) -> Self {
        let lease = ENGINE_TEST_LOCK.lock();

        rust_mixer_uninit();
        assert_eq!(rust_mixer_init(RATE, C_STEREO16, 1, flags), 1);

        Mixer { _lease: lease }
    }
}

impl Drop for Mixer {
    fn drop(&mut self) {
        unsafe {
            for mut source in SOURCES.take() {
                rust_mixer_source_stop(source);
                rust_mixer_delete_sources(1, &mut source);
            }
            for mut buffer in BUFFERS.take() {
                rust_mixer_delete_buffers(1, &mut buffer);
            }
            rust_mixer_uninit();
        }
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

fn buffer_with(format: c_uint, samples: &[i16]) -> MixerObject {
    let mut handle = [0 as MixerObject; 1];
    let data = pcm(samples);
    unsafe {
        rust_mixer_gen_buffers(1, handle.as_mut_ptr());
        rust_mixer_buffer_data(handle[0], format, data.as_ptr(), data.len() as c_uint, RATE);
    }
    BUFFERS.with_borrow_mut(|all| all.push(handle[0]));
    handle[0]
}

fn source() -> MixerObject {
    let mut handle = [0 as MixerObject; 1];
    unsafe { rust_mixer_gen_sources(1, handle.as_mut_ptr()) };
    SOURCES.with_borrow_mut(|all| all.push(handle[0]));
    handle[0]
}

fn queue(source: MixerObject, buffer: MixerObject) {
    let mut handle = [buffer];
    unsafe { rust_mixer_source_queue_buffers(source, 1, handle.as_mut_ptr()) };
}

fn state_of(source: MixerObject, prop: c_uint) -> MixerIntVal {
    let mut value: MixerIntVal = -99;
    unsafe { rust_mixer_get_sourcei(source, prop, &mut value) };
    value
}

fn render(frames: usize) -> Vec<i16> {
    let mut stream = vec![0x55u8; frames * 4];
    unsafe { rust_mixer_mix_channels(ptr::null_mut(), stream.as_mut_ptr(), stream.len() as c_int) };
    read_pcm(&stream)
}

const STATE_INITIAL: MixerIntVal = SourceState::Initial as MixerIntVal;
const STATE_STOPPED: MixerIntVal = SourceState::Stopped as MixerIntVal;
const STATE_PLAYING: MixerIntVal = SourceState::Playing as MixerIntVal;
const STATE_PAUSED: MixerIntVal = SourceState::Paused as MixerIntVal;

#[test]
fn c_formats_decode_to_the_matching_mixer_format() {
    assert_eq!(decode_c_format(0x0017_0101), MixerFormat::Mono8);
    assert_eq!(decode_c_format(0x0017_0201), MixerFormat::Stereo8);
    assert_eq!(decode_c_format(C_MONO16), MixerFormat::Mono16);
    assert_eq!(decode_c_format(C_STEREO16), MixerFormat::Stereo16);
}

#[test]
fn init_reports_the_negotiated_frequency_and_format() {
    let _mixer = Mixer::start();

    assert_eq!(rust_mixer_get_frequency(), RATE);
    assert_eq!(rust_mixer_get_format(), MixerFormat::Stereo16 as c_uint);
    assert_eq!(rust_mixer_get_error(), 0);
}

#[test]
fn buffer_data_is_recorded_with_its_original_properties() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[1, -1, 2, -2, 3, -3]);
    let mut value: MixerIntVal = 0;
    unsafe {
        rust_mixer_get_bufferi(buffer, BufferProp::Frequency as c_uint, &mut value);
        assert_eq!(value, RATE as MixerIntVal);
        rust_mixer_get_bufferi(buffer, BufferProp::Channels as c_uint, &mut value);
        assert_eq!(value, 2);
        rust_mixer_get_bufferi(buffer, BufferProp::Bits as c_uint, &mut value);
        assert_eq!(value, 16);
        rust_mixer_get_bufferi(buffer, BufferProp::Size as c_uint, &mut value);
        assert_eq!(value, 12);
    }
}

#[test]
pub(crate) fn a_queued_source_plays_its_samples_then_stops_with_the_buffer_processed() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[400, -400, 800, -800, 1200, -1200]);
    let source = source();
    assert_eq!(state_of(source, STATE), STATE_INITIAL);
    queue(source, buffer);
    assert_eq!(state_of(source, QUEUED), 1);

    rust_mixer_source_play(source);
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

    let mut returned = [0 as MixerObject; 1];
    unsafe { rust_mixer_source_unqueue_buffers(source, 1, returned.as_mut_ptr()) };
    assert_eq!(returned[0], buffer);
    assert_eq!(state_of(source, QUEUED), 0);
}

#[test]
fn gain_scales_what_reaches_the_stream() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[1000, -1000, 2000, -2000]);
    let source = source();
    queue(source, buffer);
    unsafe {
        rust_mixer_sourcef(source, GAIN, 0.5);
        let mut gain = 0.0f32;
        rust_mixer_get_sourcef(source, GAIN, &mut gain);
        assert!((gain - 0.5).abs() < 1e-6);
        rust_mixer_source_play(source);
    }
    let out = render(2);
    assert_eq!(out, [375, -375, 750, -750]);
}

#[test]
fn the_looping_flag_is_kept_on_the_source_and_stopping_clears_the_queue() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[40, 80, 120, 160]);
    let source = source();
    queue(source, buffer);

    rust_mixer_sourcei(source, LOOPING, 1);
    rust_mixer_source_play(source);

    assert_eq!(state_of(source, LOOPING), 1);
    assert_eq!(render(2), [30, 60, 90, 120]);

    rust_mixer_source_stop(source);
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
    rust_mixer_source_play(source);
    assert_eq!(render(1), [30, 60]);

    rust_mixer_source_pause(source);
    assert_eq!(state_of(source, STATE), STATE_PAUSED);
    assert_eq!(render(1), [0, 0]);

    rust_mixer_source_play(source);
    assert_eq!(render(1), [90, 120]);
}

#[test]
fn rewinding_restarts_the_queue_from_the_first_sample() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_STEREO16, &[44, 48, 52, 56]);
    let source = source();
    queue(source, buffer);
    rust_mixer_source_play(source);
    render(1);

    rust_mixer_source_rewind(source);
    assert_eq!(state_of(source, STATE), STATE_INITIAL);
    rust_mixer_source_play(source);

    assert_eq!(render(1), [33, 36]);
}

#[test]
fn two_sources_are_summed_into_the_stream() {
    let _mixer = Mixer::start();
    let first = source();
    let second = source();
    queue(first, buffer_with(C_STEREO16, &[400, 800]));
    queue(second, buffer_with(C_STEREO16, &[40, 80]));

    rust_mixer_source_play(first);
    rust_mixer_source_play(second);

    assert_eq!(render(1), [330, 660]);
}

#[test]
fn queued_buffers_play_back_to_back() {
    let _mixer = Mixer::start();
    let source = source();
    let mut buffers = [
        buffer_with(C_STEREO16, &[40, 40]),
        buffer_with(C_STEREO16, &[80, 80]),
    ];
    unsafe { rust_mixer_source_queue_buffers(source, 2, buffers.as_mut_ptr()) };
    assert_eq!(state_of(source, QUEUED), 2);
    rust_mixer_source_play(source);
    assert_eq!(render(2), [30, 30, 60, 60]);
    assert_eq!(state_of(source, PROCESSED), 2);
}

#[test]
fn deleting_objects_invalidates_their_handles_and_zeroes_the_caller_array() {
    let _mixer = Mixer::start();
    let mut sources = [source(), source()];
    let mut buffers = [buffer_with(C_STEREO16, &[1, 2])];
    unsafe {
        assert_eq!(rust_mixer_is_source(sources[0]), 1);
        assert_eq!(rust_mixer_is_buffer(buffers[0]), 1);
        rust_mixer_delete_sources(2, sources.as_mut_ptr());
        rust_mixer_delete_buffers(1, buffers.as_mut_ptr());
    }
    assert_eq!(sources, [0, 0]);
    assert_eq!(buffers, [0]);
}

#[test]
fn unknown_properties_and_empty_requests_are_ignored() {
    let _mixer = Mixer::start();
    let source = source();
    unsafe {
        let mut value: MixerIntVal = 1234;
        rust_mixer_get_sourcei(source, 0x7777, &mut value);
        assert_eq!(value, 1234, "unknown property leaves the output alone");
        let mut gain = 9.0f32;
        rust_mixer_get_sourcef(source, 0x7777, &mut gain);
        assert_eq!(gain, 9.0);
        rust_mixer_get_sourcei(source, STATE, ptr::null_mut());
        rust_mixer_get_sourcef(source, GAIN, ptr::null_mut());
        rust_mixer_get_bufferi(source, BufferProp::Size as c_uint, ptr::null_mut());
        rust_mixer_sourcei(source, 0x7777, 5);
        rust_mixer_sourcef(source, 0x7777, 5.0);
        rust_mixer_sourcefv(source, GAIN, ptr::null());
        rust_mixer_gen_sources(0, ptr::null_mut());
        rust_mixer_gen_sources(2, ptr::null_mut());
        rust_mixer_gen_buffers(0, ptr::null_mut());
        rust_mixer_gen_buffers(2, ptr::null_mut());
        rust_mixer_delete_sources(0, ptr::null_mut());
        rust_mixer_delete_buffers(0, ptr::null_mut());
        rust_mixer_source_queue_buffers(source, 0, ptr::null_mut());
        rust_mixer_source_queue_buffers(source, 1, ptr::null_mut());
        rust_mixer_source_unqueue_buffers(source, 0, ptr::null_mut());
        rust_mixer_source_unqueue_buffers(source, 1, ptr::null_mut());
        rust_mixer_buffer_data(1, C_STEREO16, ptr::null(), 4, RATE);
        rust_mixer_mix_channels(ptr::null_mut(), ptr::null_mut(), 16);
        rust_mixer_mix_channels(ptr::null_mut(), [0u8; 4].as_mut_ptr(), 0);
        rust_mixer_mix_fake(ptr::null_mut(), ptr::null_mut(), 16);
        rust_mixer_mix_fake(ptr::null_mut(), [0u8; 4].as_mut_ptr(), -1);
    }
    assert_eq!(state_of(source, STATE), STATE_INITIAL);
    assert_eq!(state_of(source, QUEUED), 0);
}

#[test]
fn a_zero_sized_or_malformed_buffer_upload_is_refused() {
    let _mixer = Mixer::start();
    let mut handle = [0 as MixerObject; 1];
    unsafe {
        rust_mixer_gen_buffers(1, handle.as_mut_ptr());
        let data = [0u8; 8];
        rust_mixer_buffer_data(handle[0], C_STEREO16, data.as_ptr(), 0, RATE);
        rust_mixer_buffer_data(handle[0], 0, data.as_ptr(), 8, RATE);
        let mut size: MixerIntVal = -1;
        rust_mixer_get_bufferi(handle[0], BufferProp::Size as c_uint, &mut size);
        assert_eq!(size, 0, "neither upload took effect");
    }
}

#[test]
fn mono_buffers_are_accepted_by_a_stereo_mixer() {
    let _mixer = Mixer::start();
    let buffer = buffer_with(C_MONO16, &[5, 6, 7, 8]);
    let mut value: MixerIntVal = 0;
    unsafe { rust_mixer_get_bufferi(buffer, BufferProp::Channels as c_uint, &mut value) };
    assert_eq!(value, 1);
    unsafe { rust_mixer_get_bufferi(buffer, BufferProp::Size as c_uint, &mut value) };
    assert_eq!(value, 8);
}

#[test]
fn fake_mode_renders_without_real_buffers() {
    let _mixer = Mixer::start_with_flags(1);
    let source = source();
    rust_mixer_source_play(source);
    let mut stream = [0u8; 16];
    unsafe { rust_mixer_mix_fake(ptr::null_mut(), stream.as_mut_ptr(), 16) };
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
        deleting_objects_invalidates_their_handles_and_zeroes_the_caller_array,
        unknown_properties_and_empty_requests_are_ignored,
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
