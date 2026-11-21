//! The OS platform: system services, files, display, audio, network and
//! paths behind the engine's host ports. The game installs it at start-up.

//! Unsafe code is allowed only in the modules that xtask quality's
//! `UNSAFE_ALLOWED` lists: `sdl`, `macos`, `qos` and `os`.

use std::path::PathBuf;

pub mod audio_output;
pub mod host;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod net;
pub mod os;
#[cfg(target_os = "macos")]
pub mod qos;
pub mod rodio_device;
pub mod sdl;
pub mod system;

/// SDL allows one live context per process; tests that open one take turns.
#[cfg(test)]
pub(crate) static SDL_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Build and install the process platform: the replay clock and audio for a
/// replay, the OS clock and rodio otherwise.
pub fn install(replay: bool) {
    uqm_engine::host::install_platform(host::new(replay));
}

/// Keep upstream's per-user location; packaged content never changes this path.
pub fn default_config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(|home| PathBuf::from(home).join("AppData/Roaming"))
            })
            .unwrap_or_else(|| panic!("Windows must provide APPDATA or USERPROFILE"))
            .join("uqm")
    }
    #[cfg(unix)]
    {
        let Some(home) = std::env::home_dir() else {
            panic!("OS home directory unavailable");
        };
        home.join(".uqm/")
    }
}
