//! The OS platform: system services, files, display, audio and paths.

use std::path::PathBuf;

pub mod audio_output;
pub mod host;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod os;
pub mod rodio_device;
pub mod system;

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
