//! The sound options chosen on the command line and in the setup menu.

use parking_lot::Mutex;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundOptions {
    /// Music volume last chosen in the setup menu (`NORMAL_VOLUME` initially).
    pub music_volume: i32,
    pub music_scale: f32,
    pub sfx_scale: f32,
    pub speech_scale: f32,
    /// Sound driver and quality selected on the command line.
    pub driver: i32,
    pub flags: i32,
}

static OPTIONS: Mutex<SoundOptions> = Mutex::new(SoundOptions {
    music_volume: 160,
    music_scale: 0.0,
    sfx_scale: 0.0,
    speech_scale: 0.0,
    driver: 0,
    flags: 0,
});

pub fn sound_options() -> SoundOptions {
    *OPTIONS.lock()
}

pub fn update_sound_options(update: impl FnOnce(&mut SoundOptions)) {
    update(&mut OPTIONS.lock());
}
