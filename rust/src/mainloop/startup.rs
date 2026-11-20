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

use crate::automation::input_ffi::graphics_options;
use crate::mainloop::{dir_prep, options, options_abi as globals};
use crate::sound::volume_globals::update_sound_options;
use std::ffi::CString;

fn c_option(value: &Option<String>) -> Option<CString> {
    value
        .as_ref()
        .map(|value| CString::new(value.as_str()).expect("option contains NUL"))
}

/// Bring every subsystem up on the SDL owner thread, once, after command-line
/// parsing. Returns the process exit status: zero when the game may start.
pub fn initialize(
    input: &crate::engine::input::InputSession,
    parsed: &options::ParsedOptions,
    args: &[String],
    observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
) -> i32 {
    if !start_services(parsed, args) {
        return 1;
    }
    let vfs = crate::io::Vfs::new();
    crate::io::install(vfs.clone());
    apply_options(input, parsed);
    if !prepare_directories(&vfs, input, parsed) {
        return 1;
    }
    if !start_media(input, parsed, observer) {
        return 1;
    }
    0
}

/// Logging, SDL, memory and the game thread.
fn start_services(parsed: &options::ParsedOptions, args: &[String]) -> bool {
    crate::log_abi::log_init(15);
    crate::bridge_log::rust_bridge_init();
    if let Some(path) = &parsed.log_file {
        if let Err(error) = crate::platform::os::stderr::redirect_stderr(path.as_ref()) {
            eprintln!("Could not open logfile: {error}");
            return false;
        }
        for (i, arg) in args.iter().enumerate() {
            log::info!("argv[{i}] = [{arg}]");
        }
    }
    log::info!("The Ur-Quan Masters v{}\nThis software comes with ABSOLUTELY NO WARRANTY; for details see the included 'COPYING' file.", env!("CARGO_PKG_VERSION"));
    tracing::info!("Initializing base SDL functionality.");
    match crate::platform::os::sdl::init_video() {
        Ok(version) => tracing::info!(
            "Using SDL version {}.{}.{} (compiled with {}.{}.{})",
            version.major,
            version.minor,
            version.patch,
            version.major,
            version.minor,
            version.patch,
        ),
        Err(error) => {
            tracing::error!("{error}");
            return false;
        }
    }
    if !crate::memory::rust_mem_init() {
        return false;
    }
    crate::threading::init_game_thread();
    true
}

/// The config, content, melee and save directories and the addon shadows.
fn prepare_directories(
    vfs: &crate::io::Vfs,
    input: &crate::engine::input::InputSession,
    parsed: &options::ParsedOptions,
) -> bool {
    if let Err(error) = dir_prep::prepare_config_dir(vfs, parsed.config_dir.as_deref()) {
        log::error!("Config directory: {error:?}");
        return false;
    }
    input.configuration.set_addons(
        parsed
            .addons
            .iter()
            .map(|name| CString::new(name.as_str()).expect("addon name contains NUL"))
            .collect(),
    );
    if let Err(error) = dir_prep::prepare_content_dir(
        vfs,
        &input.configuration,
        parsed.content_dir.as_deref(),
        parsed.addon_dir.as_deref(),
    ) {
        log::error!("Content directory: {error:?}");
        return false;
    }
    if let Err(error) = dir_prep::prepare_melee_dir(vfs) {
        log::error!("Melee directory: {error:?}");
        return false;
    }
    if let Err(error) = dir_prep::prepare_save_dir(vfs) {
        log::error!("Save directory: {error:?}");
        return false;
    }
    dir_prep::prepare_shadow_addons(vfs, &parsed.addons);
    true
}

/// Copy the parsed options into the sound options, the player controls and the
/// preferences the game reads.
fn apply_options(input: &crate::engine::input::InputSession, parsed: &options::ParsedOptions) {
    input.set_player_control(0, parsed.player1_control.unwrap_or(0) as u32);
    input.set_player_control(1, parsed.player2_control.unwrap_or(3) as u32);
    crate::input::sdl_input::forget_template_names();
    update_sound_options(|options| {
        options.driver = parsed.sound_driver;
        options.flags = parsed.sound_quality;
        options.music_scale = parsed.music_volume_scale;
        options.sfx_scale = parsed.sfx_volume_scale;
        options.speech_scale = parsed.speech_volume_scale;
    });
    input
        .configuration
        .set_preferences(uqm_engine::configuration::Preferences {
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
        });
}

/// Graphics, colour maps and input.
fn start_media(
    input: &crate::engine::input::InputSession,
    parsed: &options::ParsedOptions,
    observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
) -> bool {
    let (driver, flags) = graphics_config(parsed);
    let backend = graphics_options(c_option(&parsed.graphics_backend), observer);
    if !crate::engine::host::display::init_graphics(
        driver,
        flags,
        &backend,
        parsed.resolution_width,
        parsed.resolution_height,
    ) {
        return false;
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
    if let Err(error) = crate::input::sdl_input::init_input(input.controls()) {
        log::error!("{error}");
        return false;
    }
    true
}

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
