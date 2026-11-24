//! Subsystem startup on the SDL owner thread, replacing uqm.c's live init path.

/// Graphics flag bits shared with `gfx_common`.
const GFX_FLAG_FULLSCREEN: i32 = 1;
const GFX_FLAG_SHOW_FPS: i32 = 2;
const GFX_FLAG_SCANLINES: i32 = 4;

fn graphics_config(parsed: &super::options::ParsedOptions) -> (i32, i32) {
    let mut flags = parsed.scaler;
    if parsed.fullscreen {
        flags |= GFX_FLAG_FULLSCREEN;
    }
    if parsed.scanlines {
        flags |= GFX_FLAG_SCANLINES;
    }
    if parsed.show_fps {
        flags |= GFX_FLAG_SHOW_FPS;
    }
    (i32::from(!parsed.opengl), flags)
}

use crate::automation::input_ffi::graphics_options;
use crate::mainloop::{dir_prep, options, options_abi as globals};
use std::ffi::CString;
use uqm_engine::sound::volume_globals::update_sound_options;
use uqm_engine::{
    io::Dir,
    resource::settings::{Bindings, Settings},
};

pub struct Prepared {
    pub settings: Settings,
    pub bindings: Bindings,
    pub content: Dir,
    pub config: Dir,
    pub saves: Dir,
    pub teams: Dir,
}

/// Bring every subsystem up on the SDL owner thread, once, after command-line
/// parsing. Returns the process exit status: zero when the game may start.
pub fn initialize(
    input: &uqm_engine::input::InputSession,
    parsed: &options::ParsedOptions,
    args: &[String],
    observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
) -> Result<Prepared, i32> {
    if !start_services(parsed, args) {
        return Err(1);
    }
    let vfs = uqm_engine::io::Vfs::with_disk(uqm_engine::host::platform().disk());
    uqm_engine::io::install(vfs.clone());
    let mut settings = Settings::default();
    let mut bindings = Bindings::default();
    apply_options(input, parsed, &mut settings);
    if !prepare_directories(&vfs, input, parsed) {
        return Err(1);
    }
    let content = vfs.content_dir().expect("prepared content").snapshot();
    let config = vfs.config_dir().expect("prepared config").snapshot();
    let saves = vfs.save_dir().expect("prepared saves").snapshot();
    let teams = vfs.melee_dir().expect("prepared teams").snapshot();
    if !start_media(input, parsed, observer, &mut bindings, &content, &config) {
        return Err(1);
    }
    Ok(Prepared {
        settings,
        bindings,
        content,
        config,
        saves,
        teams,
    })
}

/// Logging, SDL, memory and the game thread.
fn start_services(parsed: &options::ParsedOptions, args: &[String]) -> bool {
    uqm_engine::log_abi::log_init(15);
    uqm_engine::bridge_log::rust_bridge_init();
    if let Some(path) = &parsed.log_file {
        if let Err(error) = uqm_platform::os::stderr::redirect_stderr(path.as_ref()) {
            eprintln!("Could not open logfile: {error}");
            return false;
        }
        for (i, arg) in args.iter().enumerate() {
            log::info!("argv[{i}] = [{arg}]");
        }
    }
    log::info!("The Ur-Quan Masters v{}\nThis software comes with ABSOLUTELY NO WARRANTY; for details see the included 'COPYING' file.", env!("CARGO_PKG_VERSION"));
    tracing::info!("Initializing base SDL functionality.");
    match uqm_platform::os::sdl::init_video() {
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
    uqm_engine::threading::init_game_thread();
    true
}

/// The config, content, melee and save directories and the addon shadows.
fn prepare_directories(
    vfs: &uqm_engine::io::Vfs,
    input: &uqm_engine::input::InputSession,
    parsed: &options::ParsedOptions,
) -> bool {
    if let Err(error) = dir_prep::prepare_config_dir(vfs, parsed.config_dir.as_deref()) {
        log::error!("Config directory: {error:?}");
        return false;
    }
    let addons: Result<Vec<_>, _> = parsed
        .addons
        .iter()
        .map(|name| CString::new(name.as_str()))
        .collect();
    match addons {
        Ok(addons) => input.configuration.set_addons(addons),
        Err(error) => {
            log::error!("Addon name: {error}");
            return false;
        }
    }
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
fn apply_options(
    input: &uqm_engine::input::InputSession,
    parsed: &options::ParsedOptions,
    settings: &mut Settings,
) {
    input.set_player_control(0, parsed.player1_control.unwrap_or(0) as u32);
    input.set_player_control(1, parsed.player2_control.unwrap_or(3) as u32);
    uqm_engine::input_port::sdl_input::forget_template_names(settings);
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
    input: &uqm_engine::input::InputSession,
    parsed: &options::ParsedOptions,
    observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
    bindings: &mut Bindings,
    content: &Dir,
    config: &Dir,
) -> bool {
    let (driver, flags) = graphics_config(parsed);
    uqm_engine::graphics::present::init_graphics(
        driver,
        flags,
        graphics_options(observer),
        parsed.resolution_width,
        parsed.resolution_height,
    );
    let gamma = if globals::set_gamma_correction(parsed.gamma) {
        parsed.gamma
    } else {
        1.0
    };
    input
        .configuration
        .update_preferences(|preferences| preferences.gamma = gamma);
    if let Err(error) = uqm_engine::input_port::sdl_input::init_input(
        input.controls(),
        bindings,
        Some(content),
        Some(config),
    ) {
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
