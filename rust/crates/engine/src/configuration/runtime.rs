//! Serialized access to the application's native display and audio settings.
use crate::{
    graphics::{
        context_abi::ContextLease,
        gfx_common_abi::{GraphicsDriver, ScreenHeightActual, ScreenWidthActual},
        sdl::lifecycle_abi::GfxFlags,
        tfb_draw_abi::TFB_DrawScreen_ReinitVideo,
    },
    io::{handles::configDir, uio_owned::Directory},
    sound::{
        heart_ffi::{set_music_volume, set_speech_volume},
        volume_globals::{
            musicVolume, musicVolumeScale, sfxVolumeScale, snddriver, soundflags, speechVolumeScale,
        },
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
    pub directory: Directory,
}

impl RuntimeSettings {
    pub fn capture(context: &ContextLease) -> Self {
        let _target = context.target();
        // SAFETY: the retained context checks the serialized graphics owner thread.
        let (display, audio, volumes, directory) = unsafe {
            (
                [
                    GraphicsDriver,
                    GfxFlags,
                    ScreenWidthActual,
                    ScreenHeightActual,
                ],
                [snddriver, soundflags],
                [musicVolumeScale, sfxVolumeScale, speechVolumeScale],
                configDir,
            )
        };
        let directory = std::ptr::NonNull::new(directory)
            .unwrap_or_else(|| panic!("configuration directory not initialized"));
        // SAFETY: application initialization retains this live directory on the owner thread.
        let directory = unsafe { Directory::retain(directory) };
        Self {
            directory,
            display: DisplaySettings {
                driver: display[0],
                flags: display[1],
                width: display[2],
                height: display[3],
            },
            audio: AudioSettings {
                driver: audio[0],
                flags: audio[1],
                music: volumes[0],
                sfx: volumes[1],
                speech: volumes[2],
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
            unsafe {
                UninitVideoPlayer();
                TFB_DrawScreen_ReinitVideo(driver, flags, width, height);
            }
            context.flush();
            // SAFETY: the synchronous video reinitialization above has completed.
            unsafe { InitVideoPlayer(1) };
        }
        if let Some([music, sfx, speech]) = volumes {
            // SAFETY: the context's application thread serializes the audio settings too.
            let current_volume = unsafe {
                musicVolumeScale = music;
                sfxVolumeScale = sfx;
                speechVolumeScale = speech;
                musicVolume as u16
            };
            set_music_volume(i32::from(current_volume));
            set_speech_volume(speech);
        }
    }
}
