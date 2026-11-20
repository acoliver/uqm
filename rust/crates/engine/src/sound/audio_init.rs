//! Audio subsystem bring-up and tear-down: the Rust mixer and the stream
//! decoder thread owned by the audio heart.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::sound::heart_ffi::{
    init_stream_decoder, set_music_volume, set_sfx_volume, set_speech_volume, uninit_stream_decoder,
};
use crate::sound::mixer::ffi::{rust_mixer_init, rust_mixer_uninit};
use crate::sound::volume_globals::sound_options;

const MIXER_FREQUENCY: u32 = 44100;
// MIX_FORMAT_MAKE(2, 2): 16-bit samples, two channels.
const MIXER_FORMAT_STEREO16: u32 = 0x0202;
const MIXER_QUALITY_MEDIUM: u32 = 1;

static AUDIO_INITED: AtomicBool = AtomicBool::new(false);

/// Starts the mixer and the stream decoder thread.
/// Returns 0 on success and -1 when the stream decoder fails to start. A mixer that cannot start is fatal.
///
/// Must be called once from the main thread before any audio call.
pub fn init_audio() -> i32 {
    log::info!("initAudio: Using Rust mixer backend");

    if rust_mixer_init(
        MIXER_FREQUENCY,
        MIXER_FORMAT_STEREO16,
        MIXER_QUALITY_MEDIUM,
        0,
    ) == 0
    {
        log::error!("Rust mixer initialization failed.");
        std::process::exit(1);
    }

    if init_stream_decoder() != 0 {
        log::error!("Stream decoder initialization failed.");
        rust_mixer_uninit();
        return -1;
    }

    crate::platform::os::process::at_exit(uninit_audio_at_exit);

    let options = sound_options();
    set_sfx_volume(options.sfx_scale);
    set_speech_volume(options.speech_scale);
    set_music_volume(options.music_volume);

    AUDIO_INITED.store(true, Ordering::SeqCst);
    0
}

extern "C" fn uninit_audio_at_exit() {
    uninit_audio();
}

/// Stops the audio subsystem. Does nothing when `init_audio` has not
/// succeeded or the subsystem is already down.
pub fn uninit_audio() {
    if !AUDIO_INITED.swap(false, Ordering::SeqCst) {
        return;
    }

    uninit_stream_decoder();
    rust_mixer_uninit();
}
