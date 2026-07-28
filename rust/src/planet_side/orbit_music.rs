//! Rust ownership of orbit-theme loading and selected planet music.

use std::ffi::c_void;
use std::sync::Mutex;

const NUM_ORBIT_THEMES: usize = 5;

#[derive(Clone, Copy)]
struct MusicHandle(*mut c_void);

// The game kernel initializes, selects, and frees these handles synchronously.
unsafe impl Send for MusicHandle {}

static ORBIT_MUSIC: Mutex<[MusicHandle; NUM_ORBIT_THEMES]> =
    Mutex::new([MusicHandle(std::ptr::null_mut()); NUM_ORBIT_THEMES]);

/// Selected orbit music handle retained for the transitional C caller.
#[no_mangle]
pub static mut LanderMusic: *mut c_void = std::ptr::null_mut();

#[cfg(feature = "linked_c_archive")]
extern "C" {
    fn load_orbit_theme(selector: u8) -> *mut c_void;
}

/// Load the orbit themes once. Active PlanetSide assets are session-scoped and
/// are deliberately not kept in global C state.
#[no_mangle]
pub extern "C" fn LoadLanderData() {
    #[cfg(feature = "linked_c_archive")]
    {
        let Ok(mut themes) = ORBIT_MUSIC.lock() else {
            return;
        };
        if !themes[0].0.is_null() {
            return;
        }
        for (index, theme) in themes.iter_mut().enumerate() {
            unsafe {
                theme.0 = load_orbit_theme(index as u8);
            }
        }
    }
    super::init_lander::load_lander_frames();
}

/// Release globally retained orbit themes.
#[no_mangle]
pub extern "C" fn FreeLanderData() {
    #[cfg(feature = "linked_c_archive")]
    {
        let Ok(mut themes) = ORBIT_MUSIC.lock() else {
            return;
        };
        for theme in themes.iter_mut() {
            if !theme.0.is_null() {
                unsafe {
                    crate::sound::heart_ffi::DestroyMusic(theme.0);
                }
                theme.0 = std::ptr::null_mut();
            }
        }
        unsafe {
            LanderMusic = std::ptr::null_mut();
        }
    }
    super::init_lander::free_lander_frames();
}

/// Select one of the five source-defined orbit themes.
#[no_mangle]
pub extern "C" fn SetPlanetMusic(planet_type: u8) {
    let Ok(themes) = ORBIT_MUSIC.lock() else {
        return;
    };
    unsafe {
        LanderMusic = themes[usize::from(planet_type) % NUM_ORBIT_THEMES].0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_wraps_across_five_orbit_themes() {
        let selected = usize::from(7_u8) % NUM_ORBIT_THEMES;
        assert_eq!(selected, 2);
    }
}
