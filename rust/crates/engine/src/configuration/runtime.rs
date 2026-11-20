//! Serialized access to the application's native display and audio settings.
use crate::{
    graphics::{context::ContextLease, state},
    io::Dir,
    sound::{
        heart_ffi::{set_music_volume, set_speech_volume},
        volume_globals::{sound_options, update_sound_options},
    },
    video::control::{InitVideoPlayer, UninitVideoPlayer},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DisplaySettings {
    pub driver: i32,
    pub flags: i32,
    pub width: i32,
    pub height: i32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AudioSettings {
    pub driver: i32,
    pub flags: i32,
    pub music: f32,
    pub sfx: f32,
    pub speech: f32,
}
#[derive(Clone, Debug)]
pub struct RuntimeSettings {
    pub display: DisplaySettings,
    pub audio: AudioSettings,
    pub directory: Dir,
}

impl RuntimeSettings {
    pub fn capture(context: &ContextLease) -> Self {
        let _target = context.target();
        let mode = state::video_mode();
        let display = [mode.driver, mode.flags, mode.width, mode.height];
        let sound = sound_options();
        let directory = crate::io::config_dir()
            .unwrap_or_else(|| panic!("configuration directory not initialized"));
        Self {
            directory,
            display: DisplaySettings {
                driver: display[0],
                flags: display[1],
                width: display[2],
                height: display[3],
            },
            audio: AudioSettings {
                driver: sound.driver,
                flags: sound.flags,
                music: sound.music_scale,
                sfx: sound.sfx_scale,
                speech: sound.speech_scale,
            },
        }
    }

    pub fn apply(
        context: &ContextLease,
        display: Option<DisplaySettings>,
        volumes: Option<[f32; 3]>,
    ) {
        let _target = context.target();
        if let Some(DisplaySettings {
            driver,
            flags,
            width,
            height,
        }) = display
        {
            assert!(width > 0 && height > 0, "invalid display dimensions");
            context.flush();
            // SAFETY: the context verifies the owner thread and retains the display resources.
            unsafe { UninitVideoPlayer() };
            state::reinit_video(state::VideoMode {
                driver,
                flags,
                width,
                height,
            });
            context.flush();
            // SAFETY: the synchronous video reinitialization above has completed.
            unsafe { InitVideoPlayer(1) };
        }
        if let Some([music, sfx, speech]) = volumes {
            update_sound_options(|options| {
                options.music_scale = music;
                options.sfx_scale = sfx;
                options.speech_scale = speech;
            });
            let current_volume = sound_options().music_volume as u16;
            set_music_volume(i32::from(current_volume));
            set_speech_volume(speech);
        }
    }
}
