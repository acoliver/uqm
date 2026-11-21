use uqm_engine::world::MUSIC_DISABLED;

pub(crate) const OPT_PC: i32 = 2;
const SOUND_DISABLED: u8 = 16;
pub(crate) const GAME_SOUND_PRIORITY: u8 = 2;

pub fn toggle_music(world: &mut uqm_engine::World) {
    world.data.game.glob_flags ^= MUSIC_DISABLED;
    uqm_engine::sound::music::set_requested_enabled(
        world.data.game.glob_flags & MUSIC_DISABLED == 0,
    );
}

pub(crate) fn play_loaded_music(
    world: &uqm_engine::World,
    music: Option<&uqm_engine::sound::resource_types::LoadedMusic>,
    continuous: u32,
    priority: u8,
) {
    let request = music.map_or(uqm_engine::sound::music::MusicRequest::Absent, |music| {
        uqm_engine::sound::music::MusicRequest::Track(music.playback())
    });
    uqm_engine::sound::music::remember_music(
        request,
        continuous != 0,
        priority,
        world.data.game.glob_flags & MUSIC_DISABLED == 0,
    );
}
pub fn stop_music() {
    uqm_engine::sound::music::stop_requested_music();
}
pub(crate) fn stop_loaded_music(music: Option<&uqm_engine::sound::resource_types::LoadedMusic>) {
    let request = music.map_or(uqm_engine::sound::music::MusicRequest::Absent, |music| {
        uqm_engine::sound::music::MusicRequest::Track(music.playback())
    });
    uqm_engine::sound::music::stop_music_request(&request);
}
pub fn resume_music() {
    uqm_engine::sound::music::resume_requested_music();
}
pub fn pause_music() {
    uqm_engine::sound::music::pause_requested_music();
}
pub fn toggle_sound_effect(world: &mut uqm_engine::World) {
    world.data.game.glob_flags ^= SOUND_DISABLED;
}

pub(crate) fn play_menu_failure(kernel: &uqm_engine::Kernel, world: &uqm_engine::World) {
    play_menu_sound(kernel, world, 2);
}

pub(crate) fn play_menu_sound(kernel: &uqm_engine::Kernel, world: &uqm_engine::World, index: u16) {
    if let Some(bank) = kernel.menu_sound_bank() {
        play_loaded_sound(
            world,
            &bank.clip(index),
            0,
            uqm_engine::sound::types::SoundPosition::non_positional(),
            0,
            GAME_SOUND_PRIORITY,
        );
    }
}

pub fn play_loaded_sound(
    world: &uqm_engine::World,
    sound: &uqm_engine::sound::owned_bank::SoundClip,
    channel: usize,
    position: uqm_engine::sound::types::SoundPosition,
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
        let mut world = uqm_engine::World::new();
        world.data.game.glob_flags = MUSIC_DISABLED;
        play_loaded_music(&world, None, 1, 3);
        let (request, continuous, priority) = uqm_engine::sound::music::requested_music();
        assert!(matches!(
            request,
            uqm_engine::sound::music::MusicRequest::Absent
        ));
        assert_eq!((continuous, priority), (true, 3));
        assert_eq!(world.data.game.glob_flags, MUSIC_DISABLED);
        stop_loaded_music(None);
        stop_battle_music();
        let (request, _, _) = uqm_engine::sound::music::requested_music();
        assert!(matches!(
            request,
            uqm_engine::sound::music::MusicRequest::Absent
        ));
    }
}
