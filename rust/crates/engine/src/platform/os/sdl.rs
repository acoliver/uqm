//! SDL start-up.

use sdl2::sys;

/// The SDL library version in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

/// Start SDL's video subsystem and report the library version. SDL is torn
/// down by the OS at process exit.
pub fn init_video() -> Result<Version, String> {
    // SAFETY: SDL_Init takes flags only; start-up runs once on the main thread.
    if unsafe { sys::SDL_Init(sys::SDL_INIT_VIDEO) } != 0 {
        return Err("Could not initialize SDL".into());
    }
    let mut version = sys::SDL_version {
        major: 0,
        minor: 0,
        patch: 0,
    };
    // SAFETY: SDL_GetVersion writes the whole struct it is given.
    unsafe { sys::SDL_GetVersion(&mut version) };
    Ok(Version {
        major: version.major,
        minor: version.minor,
        patch: version.patch,
    })
}
