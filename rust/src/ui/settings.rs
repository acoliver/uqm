const MUSIC_DISABLED: u8 = 8;
const SOUND_DISABLED: u8 = 16;
const GAME_SOUND_PRIORITY: u8 = 2;

pub fn toggle_music(world: &mut crate::engine::World) {
    world.data.game.glob_flags ^= MUSIC_DISABLED;
    crate::sound::music::set_requested_enabled(world.data.game.glob_flags & MUSIC_DISABLED == 0);
}

pub(crate) fn play_loaded_music(
    world: &crate::engine::World,
    music: Option<&crate::sound::resource_types::LoadedMusic>,
    continuous: u32,
    priority: u8,
) {
    let request = music.map_or(crate::sound::music::MusicRequest::Absent, |music| {
        crate::sound::music::MusicRequest::Track(music.playback())
    });
    crate::sound::music::remember_music(
        request,
        continuous != 0,
        priority,
        world.data.game.glob_flags & MUSIC_DISABLED == 0,
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
    world.data.game.glob_flags ^= SOUND_DISABLED;
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
            GAME_SOUND_PRIORITY,
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
        world.data.game.glob_flags = MUSIC_DISABLED;
        play_loaded_music(&world, None, 1, 3);
        let (request, continuous, priority) = crate::sound::music::requested_music();
        assert!(matches!(request, crate::sound::music::MusicRequest::Absent));
        assert_eq!((continuous, priority), (true, 3));
        assert_eq!(world.data.game.glob_flags, MUSIC_DISABLED);
        stop_loaded_music(None);
        stop_battle_music();
        let (request, _, _) = crate::sound::music::requested_music();
        assert!(matches!(request, crate::sound::music::MusicRequest::Absent));
    }
}
