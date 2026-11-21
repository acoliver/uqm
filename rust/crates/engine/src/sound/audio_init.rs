//! Audio subsystem bring-up and tear-down: the Rust mixer and the stream
//! decoder thread owned by the audio heart.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::sound::heart_ffi::{
    init_stream_decoder, set_music_volume, set_sfx_volume, set_speech_volume, uninit_stream_decoder,
};
use crate::sound::mixer::{mixer_init, mixer_uninit, MixerFlags, MixerFormat, MixerQuality};
use crate::sound::volume_globals::sound_options;

const MIXER_FREQUENCY: u32 = 44100;

/// Start the mixer at 44.1 kHz, 16-bit stereo, medium quality.
fn start_mixer() -> bool {
    let log = crate::bridge_log::rust_bridge_log_msg;
    // The log keeps the C encoding: MIX_FORMAT_MAKE(2, 2) and quality 1.
    log(&format!(
        "RUST_MIXER_INIT: freq={MIXER_FREQUENCY} format=0x202 quality=1 flags=0"
    ));
    log(&format!(
        "RUST_MIXER_INIT: decoded format={:?}",
        MixerFormat::Stereo16
    ));
    match mixer_init(
        MIXER_FREQUENCY,
        MixerFormat::Stereo16,
        MixerQuality::Medium,
        MixerFlags::None,
    ) {
        Ok(()) => {
            log("RUST_MIXER_INIT: success");
            true
        }
        Err(error) => {
            log(&format!("RUST_MIXER_INIT: failed {error:?}"));
            false
        }
    }
}

static AUDIO_INITED: AtomicBool = AtomicBool::new(false);

/// Starts the mixer and the stream decoder thread.
/// Returns 0 on success and -1 when the stream decoder fails to start. A mixer that cannot start is fatal.
///
/// Must be called once from the main thread before any audio call.
pub fn init_audio() -> i32 {
    log::info!("initAudio: Using Rust mixer backend");

    if !start_mixer() {
        log::error!("Rust mixer initialization failed.");
        crate::host::exit(1);
    }

    if init_stream_decoder() != 0 {
        log::error!("Stream decoder initialization failed.");
        let _ = mixer_uninit();
        return -1;
    }

    crate::host::at_exit(uninit_audio);

    let options = sound_options();
    set_sfx_volume(options.sfx_scale);
    set_speech_volume(options.speech_scale);
    set_music_volume(options.music_volume);

    AUDIO_INITED.store(true, Ordering::SeqCst);
    0
}

/// Stops the audio subsystem. Does nothing when `init_audio` has not
/// succeeded or the subsystem is already down.
pub fn uninit_audio() {
    if !AUDIO_INITED.swap(false, Ordering::SeqCst) {
        return;
    }

    uninit_stream_decoder();
    let _ = mixer_uninit();
}
