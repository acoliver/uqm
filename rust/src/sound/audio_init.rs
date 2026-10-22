//! Audio subsystem bring-up and tear-down: the Rust mixer, the C decoder
//! registry, and the stream decoder thread owned by the audio heart.

use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::sound::formats::DecoderFormats;
use crate::sound::heart_ffi::{
    InitStreamDecoder, SetMusicVolume, SetSFXVolume, SetSpeechVolume, UninitStreamDecoder,
};
use crate::sound::mixer::ffi::{rust_mixer_Init, rust_mixer_Uninit};
use crate::sound::volume_globals::{musicVolume, sfxVolumeScale, speechVolumeScale};

// Values of the `audio_FORMAT_*` enumerators in audiocore.h. The decoders
// tag their output with these.
const AUDIO_FORMAT_MONO16: u32 = 22;
const AUDIO_FORMAT_STEREO16: u32 = 23;
const AUDIO_FORMAT_MONO8: u32 = 24;
const AUDIO_FORMAT_STEREO8: u32 = 25;

const MIXER_FREQUENCY: u32 = 44100;
// MIX_FORMAT_MAKE(2, 2): 16-bit samples, two channels.
const MIXER_FORMAT_STEREO16: u32 = 0x0202;
const MIXER_QUALITY_MEDIUM: u32 = 1;

extern "C" {
    fn SoundDecoder_Init(flags: c_int, formats: *mut DecoderFormats) -> c_int;
    fn SoundDecoder_Uninit();
}

static AUDIO_INITED: AtomicBool = AtomicBool::new(false);

/// Starts the mixer, the sound decoders and the stream decoder thread.
/// Returns 0 on success and -1 when decoders or the stream decoder fail to
/// start. A mixer that cannot start is fatal.
///
/// # Safety
/// Must be called once from the main thread before any audio call.
#[no_mangle]
pub unsafe extern "C" fn initAudio(_driver: i32, flags: i32) -> i32 {
    log::info!("initAudio: Using Rust mixer backend");

    if rust_mixer_Init(
        MIXER_FREQUENCY,
        MIXER_FORMAT_STEREO16,
        MIXER_QUALITY_MEDIUM,
        0,
    ) == 0
    {
        log::error!("Rust mixer initialization failed.");
        std::process::exit(1);
    }

    log::info!("Initializing sound decoders.");
    let mut formats = DecoderFormats {
        big_endian: false,
        want_big_endian: false,
        mono8: AUDIO_FORMAT_MONO8,
        stereo8: AUDIO_FORMAT_STEREO8,
        mono16: AUDIO_FORMAT_MONO16,
        stereo16: AUDIO_FORMAT_STEREO16,
    };
    if SoundDecoder_Init(flags, &mut formats) != 0 {
        log::error!("Sound decoders initialization failed.");
        rust_mixer_Uninit();
        return -1;
    }
    log::info!("Sound decoders initialized.");

    if InitStreamDecoder() != 0 {
        log::error!("Stream decoder initialization failed.");
        rust_mixer_Uninit();
        return -1;
    }

    libc::atexit(unInitAudio);

    SetSFXVolume(sfxVolumeScale);
    SetSpeechVolume(speechVolumeScale);
    SetMusicVolume(musicVolume);

    AUDIO_INITED.store(true, Ordering::SeqCst);
    0
}

/// Stops the audio subsystem. Does nothing when `initAudio` has not
/// succeeded or the subsystem is already down.
#[no_mangle]
pub extern "C" fn unInitAudio() {
    if !AUDIO_INITED.swap(false, Ordering::SeqCst) {
        return;
    }

    // SAFETY: the audio subsystem was brought up by `initAudio`, and the swap
    // above makes this teardown run once.
    unsafe {
        UninitStreamDecoder();
        SoundDecoder_Uninit();
        rust_mixer_Uninit();
    }
}
