use crate::engine::WorldData;
use crate::navigation::abi::*;

pub fn toggle_music(world: &mut crate::engine::World) {
    world.data.game.glob_flags ^= MUSIC_DISABLED as u8;
    crate::sound::music::set_requested_enabled(
        world.data.game.glob_flags & MUSIC_DISABLED as u8 == 0,
    );
}

/// # Safety
/// The reference identifies a live native music resource on the game thread.
#[no_mangle]
pub unsafe extern "C" fn PlayMusic(music_ref: MUSIC_REF, continuous: BOOLEAN, priority: BYTE) {
    crate::engine::World::current().read(|world| {
        // SAFETY: the caller retains the native reference throughout synchronous admission.
        unsafe { play_music_for_world(world, music_ref, continuous, priority) }
    });
}

/// # Safety
/// The reference is live during synchronous admission into the audio owner's cache.
pub(crate) unsafe fn play_music_for_world(
    world: &crate::engine::World,
    raw: MUSIC_REF,
    continuous: BOOLEAN,
    priority: BYTE,
) {
    use crate::sound::music::MusicRequest;
    let request = if raw.is_null() {
        MusicRequest::Absent
    } else if raw as usize == usize::MAX {
        MusicRequest::Current
    } else {
        // SAFETY: the caller retains the live native resource until its sample is cloned.
        MusicRequest::Track(unsafe { crate::sound::heart_ffi::music_ref_borrow(raw.cast()) })
    };
    crate::sound::music::remember_music(
        request,
        continuous != 0,
        priority,
        world.data.game.glob_flags & MUSIC_DISABLED as u8 == 0,
    );
}
pub(crate) fn play_loaded_music(
    world: &crate::engine::World,
    music: Option<&crate::sound::resource_types::LoadedMusic>,
    continuous: BOOLEAN,
    priority: BYTE,
) {
    let request = music.map_or(crate::sound::music::MusicRequest::Absent, |music| {
        crate::sound::music::MusicRequest::Track(music.playback())
    });
    crate::sound::music::remember_music(
        request,
        continuous != 0,
        priority,
        world.data.game.glob_flags & MUSIC_DISABLED as u8 == 0,
    );
}
pub fn stop_music() {
    crate::sound::music::stop_requested_music();
}
pub(crate) fn stop_loaded_music(music: Option<&crate::sound::resource_types::LoadedMusic>) {
    let request = music.map_or(crate::sound::music::MusicRequest::Absent, |music| {
        crate::sound::music::MusicRequest::Track(music.playback())
    });
    crate::sound::music::stop_music_request(&request);
}
pub fn resume_music() {
    crate::sound::music::resume_requested_music();
}
pub fn pause_music() {
    crate::sound::music::pause_requested_music();
}
pub fn toggle_sound_effect(world: &mut crate::engine::World) {
    world.data.game.glob_flags ^= SOUND_DISABLED as u8;
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
    let sound_disabled =
        WorldData::current().read(|owner| owner.game.glob_flags) & (SOUND_DISABLED as u8) != 0;
    eprintln!(
        "[PlaySoundEffect] S={sound:p} ch={} pri={} sound_disabled={}",
        channel,
        priority,
        u8::from(sound_disabled)
    );
    if !sound_disabled {
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

pub(crate) fn play_menu_sound(
    kernel: &crate::engine::Kernel,
    world: &crate::engine::World,
    index: u16,
) {
    if let Some(bank) = kernel.menu_sound_bank() {
        play_loaded_sound(
            world,
            &bank.clip(index),
            0,
            crate::sound::types::SoundPosition::non_positional(),
            0,
            GAME_SOUND_PRIORITY as u8,
        );
    }
}

pub(crate) fn play_loaded_sound(
    world: &crate::engine::World,
    sound: &crate::sound::owned_bank::SoundClip,
    channel: usize,
    position: crate::sound::types::SoundPosition,
    object: usize,
    priority: u8,
) {
    world.play_sound_effect(sound, channel, position, object, priority);
}

pub(crate) fn stop_battle_music() {
    stop_music()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn absent_loaded_music_preserves_options_and_clears_playback() {
        let mut world = crate::engine::World::new();
        world.data.game.glob_flags = MUSIC_DISABLED as u8;
        play_loaded_music(&world, None, 1, 3);
        // SAFETY: this serialized test owns the settings playback state.
        let playback = unsafe { (LAST_MUSIC_REF.is_null(), LAST_CONTINUOUS, LAST_PRIORITY) };
        assert_eq!(playback, (true, 1, 3));
        assert_eq!(world.data.game.glob_flags, MUSIC_DISABLED as u8);
        stop_loaded_music(None);
        stop_battle_music();
        // SAFETY: no other settings call runs on this serialized test thread.
        assert!(unsafe { LAST_MUSIC_REF.is_null() });
    }
}
