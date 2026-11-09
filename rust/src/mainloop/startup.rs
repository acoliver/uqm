//! Subsystem startup on the SDL owner thread, replacing uqm.c's live init path.

fn graphics_config(parsed: &super::options::ParsedOptions) -> (i32, i32) {
    use crate::ships::native::abi;
    let mut flags = parsed.scaler;
    if parsed.fullscreen {
        flags |= abi::TFB_GFXFLAGS_FULLSCREEN as i32;
    }
    if parsed.scanlines {
        flags |= abi::TFB_GFXFLAGS_SCANLINES as i32;
    }
    if parsed.show_fps {
        flags |= abi::TFB_GFXFLAGS_SHOWFPS as i32;
    }
    (i32::from(!parsed.opengl), flags)
}

#[cfg(not(test))]
mod live {

    use crate::automation::input_ffi::graphics_options;
    use crate::mainloop::{dir_prep, options, options_abi as globals};
    use crate::{game_init::kernel_abi, sound::volume_globals};
    use std::{
        ffi::{c_char, CString},
        ptr,
    };

    fn c_option(value: &Option<String>) -> Option<CString> {
        value
            .as_ref()
            .map(|value| CString::new(value.as_str()).expect("option contains NUL"))
    }

    /// # Safety
    /// Called once on the SDL owner thread after command-line parsing.
    pub unsafe fn initialize(
        input: &crate::engine::input::InputSession,
        parsed: &options::ParsedOptions,
        argc: i32,
        argv: *mut *mut c_char,
        observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
    ) -> i32 {
        crate::log_abi::log_init(15);
        crate::bridge_log::rust_bridge_init();
        if let Some(path) = &parsed.log_file {
            if let Err(error) = crate::platform::redirect_stderr(std::path::Path::new(path)) {
                eprintln!("Could not open logfile: {error}");
                return 1;
            }
            for i in 0..argc as usize {
                log::info!(
                    "argv[{i}] = [{}]",
                    std::ffi::CStr::from_ptr(*argv.add(i)).to_string_lossy()
                );
            }
        }
        log::info!("The Ur-Quan Masters v{}\nThis software comes with ABSOLUTELY NO WARRANTY; for details see the included 'COPYING' file.", env!("CARGO_PKG_VERSION"));
        if crate::mainloop::init_wrappers::rust_tfb_preinit() != 0 {
            return 1;
        }
        if !crate::memory::rust_mem_init() {
            return 1;
        }
        crate::threading::legacy_abi::InitThreadSystem();
        crate::mainloop::init_wrappers::rust_log_init_threads();
        if kernel_abi::initIO() != 0 {
            return 1;
        }
        if let Err(error) = dir_prep::prepare_config_dir(parsed.config_dir.as_deref()) {
            log::error!("Config directory: {error:?}");
            return 1;
        }

        input.set_player_control(0, parsed.player1_control.unwrap_or(0) as u32);
        input.set_player_control(1, parsed.player2_control.unwrap_or(3) as u32);
        for i in 1..=6 {
            let key = CString::new(format!("config.keys.{i}.name")).unwrap();
            crate::resource::ffi_bridge::res_Remove(key.as_ptr());
        }
        volume_globals::snddriver = parsed.sound_driver;
        volume_globals::soundflags = parsed.sound_quality;
        let preferences = uqm_engine::configuration::Preferences {
            music_3do: parsed.use_3do_music as u32,
            music_remix: parsed.use_remix_music as u32,
            speech: parsed.use_speech as u32,
            subtitles: parsed.subtitles as u32,
            stereo_sfx: parsed.stereo_sfx as u32,
            keep_aspect_ratio: parsed.keep_aspect_ratio as u32,
            coarse_scan: parsed.which_coarse_scan,
            menu: parsed.which_menu,
            fonts: parsed.which_fonts,
            intro: parsed.which_intro,
            shield: parsed.which_shield,
            smooth_scroll: parsed.smooth_scroll,
            melee_scale: parsed.melee_scale,
            ..Default::default()
        };
        input.configuration.set_preferences(preferences);
        volume_globals::musicVolumeScale = parsed.music_volume_scale;
        volume_globals::sfxVolumeScale = parsed.sfx_volume_scale;
        volume_globals::speechVolumeScale = parsed.speech_volume_scale;
        input.configuration.set_addons(
            parsed
                .addons
                .iter()
                .map(|name| CString::new(name.as_str()).expect("addon name contains NUL"))
                .collect(),
        );
        if let Err(error) = dir_prep::prepare_content_dir(
            &input.configuration,
            parsed.content_dir.as_deref(),
            parsed.addon_dir.as_deref(),
        ) {
            log::error!("Content directory: {error:?}");
            return 1;
        }
        if let Err(error) = dir_prep::prepare_melee_dir() {
            log::error!("Melee directory: {error:?}");
            return 1;
        }
        if let Err(error) = dir_prep::prepare_save_dir() {
            log::error!("Save directory: {error:?}");
            return 1;
        }
        if let Err(error) = dir_prep::prepare_shadow_addons(&parsed.addons) {
            log::warn!("Shadow addons: {error:?}");
        }
        crate::time::counter_abi::InitTimeSystem();
        crate::threading::task_abi::InitTaskSystem();
        crate::callback::ffi::Alarm_init();
        crate::callback::ffi::Callback_init();

        let (driver, flags) = super::graphics_config(parsed);
        let backend = graphics_options(c_option(&parsed.graphics_backend), observer);
        if crate::graphics::sdl::lifecycle_abi::TFB_InitGraphics(
            driver,
            flags,
            ptr::from_ref(&backend).cast(),
            parsed.resolution_width,
            parsed.resolution_height,
        ) != 0
        {
            return 1;
        }
        let gamma = if globals::setGammaCorrection(parsed.gamma) {
            parsed.gamma
        } else {
            1.0
        };
        input
            .configuration
            .update_preferences(|preferences| preferences.gamma = gamma);
        crate::graphics::cmap_abi::InitColorMaps();
        let immediate = input.immediate_ptr();
        crate::input::sdl_input::tfb_set_input_vectors(
            ptr::addr_of_mut!((*immediate).menu).cast(),
            24,
            ptr::addr_of_mut!((*immediate).key).cast(),
            6,
            7,
        );
        if crate::input::sdl_input::TFB_InitInput(0, 0) != 0 {
            return 1;
        }
        0
    }
}

#[cfg(not(test))]
pub use live::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graphics_options_preserve_scaler_and_combine_independent_flags() {
        let options = crate::mainloop::options::ParsedOptions {
            scaler: 0x4000,
            fullscreen: true,
            scanlines: true,
            show_fps: true,
            opengl: false,
            ..Default::default()
        };
        let (driver, flags) = graphics_config(&options);
        assert_eq!(driver, 1);
        assert_eq!(flags, 0x4000 | 1 | 2 | 4);
        let options = crate::mainloop::options::ParsedOptions {
            opengl: true,
            scaler: 0x4000,
            ..Default::default()
        };
        assert_eq!(graphics_config(&options), (0, 0x4000));
    }
}
