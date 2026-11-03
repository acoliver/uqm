#![cfg(not(test))]

use crate::engine::WorldData;
use crate::navigation::abi::*;

static mut LAST_MUSIC_REF: MUSIC_REF = std::ptr::null_mut();
static mut LAST_CONTINUOUS: BOOLEAN = 0;
static mut LAST_PRIORITY: BYTE = 0;

/// # Safety
/// The game settings and music player must be initialized on the game thread.
pub unsafe fn toggle_music() {
    (**WorldData::current()).game.glob_flags ^= MUSIC_DISABLED as u8;
    if LAST_PRIORITY <= 1 {
        if WorldData::current().read(|owner| owner.game.glob_flags) & (MUSIC_DISABLED as u8) != 0 {
            PLRStop((LAST_MUSIC_REF).cast());
        } else if !LAST_MUSIC_REF.is_null() {
            PLRPlaySong(
                LAST_MUSIC_REF.cast(),
                LAST_CONTINUOUS as i32,
                LAST_PRIORITY.into(),
            );
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
    if WorldData::current().read(|owner| owner.game.glob_flags) & (MUSIC_DISABLED as u8) == 0 {
        PLRPlaySong(music_ref.cast(), continuous as i32, priority.into());
    }
}

/// # Safety
/// The music player must be initialized on the game thread.
pub unsafe fn stop_music() {
    PLRStop((LAST_MUSIC_REF).cast());
    LAST_MUSIC_REF = std::ptr::null_mut();
}

/// # Safety
/// The music player must be initialized on the game thread.
pub unsafe fn resume_music() {
    PLRResume((LAST_MUSIC_REF).cast());
}

/// # Safety
/// The music player must be initialized on the game thread.
pub unsafe fn pause_music() {
    PLRPause((LAST_MUSIC_REF).cast());
}

/// # Safety
/// Game settings must be accessed on the game thread.
pub unsafe fn toggle_sound_effect() {
    (**WorldData::current()).game.glob_flags ^= SOUND_DISABLED as u8;
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
        u8::from((**WorldData::current()).game.glob_flags & (SOUND_DISABLED as u8) != 0)
    );
    if WorldData::current().read(|owner| owner.game.glob_flags) & (SOUND_DISABLED as u8) == 0 {
        SetChannelVolume(
            u32::from(channel),
            (MAX_VOLUME >> 1) as i32,
            i32::from(priority),
        );
        crate::sound::heart_ffi::play_channel(
            u32::from(channel),
            sound.cast(),
            crate::sound::types::SoundPosition {
                positional: position.positional != 0,
                x: position.x,
                y: position.y,
            },
            positional_object as usize,
            priority,
        );
    }
}
