#![cfg(any(not(test), feature = "linked_c_archive"))]
#![allow(non_snake_case)]

use crate::navigation::abi::*;
use crate::navigation::globals::GlobData;

static mut LAST_MUSIC_REF: MUSIC_REF = std::ptr::null_mut();
static mut LAST_CONTINUOUS: BOOLEAN = 0;
static mut LAST_PRIORITY: BYTE = 0;

/// # Safety
/// The game settings and music player must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn ToggleMusic() {
    GlobData.Game_state.glob_flags ^= MUSIC_DISABLED as u8;
    if LAST_PRIORITY <= 1 {
        if GlobData.Game_state.glob_flags & (MUSIC_DISABLED as u8) != 0 {
            PLRStop(LAST_MUSIC_REF);
        } else if !LAST_MUSIC_REF.is_null() {
            PLRPlaySong(LAST_MUSIC_REF, LAST_CONTINUOUS, LAST_PRIORITY);
        }
    }
}

/// # Safety
/// The music reference and player must be valid on the game thread.
#[no_mangle]
pub unsafe extern "C" fn PlayMusic(music_ref: MUSIC_REF, continuous: BOOLEAN, priority: BYTE) {
    LAST_MUSIC_REF = music_ref;
    LAST_CONTINUOUS = continuous;
    LAST_PRIORITY = priority;
    if GlobData.Game_state.glob_flags & (MUSIC_DISABLED as u8) == 0 {
        PLRPlaySong(music_ref, continuous, priority);
    }
}

/// # Safety
/// The music player must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn StopMusic() {
    PLRStop(LAST_MUSIC_REF);
    LAST_MUSIC_REF = std::ptr::null_mut();
}

/// # Safety
/// The music player must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn ResumeMusic() {
    PLRResume(LAST_MUSIC_REF);
}

/// # Safety
/// The music player must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn PauseMusic() {
    PLRPause(LAST_MUSIC_REF);
}

/// # Safety
/// Game settings must be accessed on the game thread.
#[no_mangle]
pub unsafe extern "C" fn ToggleSoundEffect() {
    GlobData.Game_state.glob_flags ^= SOUND_DISABLED as u8;
}

/// # Safety
/// The sound player and positional object must remain valid for the call.
#[no_mangle]
pub unsafe extern "C" fn PlaySoundEffect(
    sound: SOUND,
    channel: COUNT,
    position: SoundPosition,
    positional_object: *mut ::std::ffi::c_void,
    priority: BYTE,
) {
    eprintln!(
        "[PlaySoundEffect] S={sound:p} ch={} pri={} sound_disabled={}",
        channel,
        priority,
        u8::from(GlobData.Game_state.glob_flags & (SOUND_DISABLED as u8) != 0)
    );
    if GlobData.Game_state.glob_flags & (SOUND_DISABLED as u8) == 0 {
        SetChannelVolume(channel, (MAX_VOLUME >> 1) as u16, priority);
        PlayChannel(channel, sound, position, positional_object, priority);
    }
}
