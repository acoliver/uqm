//! Rust port of the C `InitLander` function from the deleted `lander.c`.
//!
//! Draws the lander status display in the radar panel: the powered-down
//! lander sprite, shield indicators, and cargo capacity bar. Called by
//! `PickPlanetSide` in `scan.c` before the landing-site selection menu.

use std::ffi::c_void;
use std::sync::Mutex;

#[cfg(not(test))]
use super::resources::{CffiResourcePort, ResourcePort};
#[cfg(not(test))]
use crate::comm::locdata::{CColor, CPoint, CRect, CStamp};

/// Global lander graphic frames, matching the C `FRAME LanderFrame[8]`.
/// Loaded by `load_lander_data`, freed by `free_lander_data`.
struct LanderFrameState {
    frames: [*mut c_void; 8],
    sounds: *mut c_void,
    loaded: bool,
}

// The game kernel serialises all access on the game thread; the Mutex
// exists only to satisfy static-initialisation requirements.
unsafe impl Send for LanderFrameState {}

static LANDER_FRAMES: Mutex<LanderFrameState> = Mutex::new(LanderFrameState {
    frames: [std::ptr::null_mut(); 8],
    sounds: std::ptr::null_mut(),
    loaded: false,
});

/// Disaster shield bit indices (from planets.h DISASTER_TYPE enum).
const EARTHQUAKE_DISASTER: u8 = 1;

const BIOLOGICAL_DISASTER: u8 = 0;

const LIGHTNING_DISASTER: u8 = 2;

const LAVASPOT_DISASTER: u8 = 3;

/// `MAX_SCROUNGED` (from planets.h).
const MAX_SCROUNGED: u16 = 50;

/// `RADAR_WIDTH` = STATUS_WIDTH - 8 = 64 - 8 = 56 (from units.h).
const RADAR_WIDTH: i16 = 56;
/// `RADAR_HEIGHT` (from units.h).
const RADAR_HEIGHT: i16 = 53;

/// `FULL_CIRCLE` = 1 << CIRCLE_SHIFT = 1 << 6 = 64 (from units.h).
const FULL_CIRCLE: u16 = 64;
/// `CIRCLE_SHIFT` (from units.h).
const CIRCLE_SHIFT: u32 = 6;
/// `FACING_SHIFT` (from units.h).
const FACING_SHIFT: u32 = 4;

/// `ANGLE_TO_FACING(a) = (a + (1 << (CIRCLE_SHIFT - FACING_SHIFT - 1))) >> (CIRCLE_SHIFT - FACING_SHIFT)`
fn angle_to_facing(angle: u16) -> u16 {
    let shift = CIRCLE_SHIFT - FACING_SHIFT;
    (angle + (1 << (shift - 1))) >> shift
}

/// Black color matching C's `MAKE_RGB15(0,0,0)` which hardcodes `a = 0xff`.
/// `BUILD_COLOR` discards its palette-index argument; the alpha comes from the
/// RGB15 macro. With `a: 0` the fill is a fully-transparent no-op.
#[cfg(not(test))]
const BLACK_COLOR: CColor = CColor {
    r: 0,
    g: 0,
    b: 0,
    a: 0xff,
};

#[cfg(not(test))]
use crate::game_init::macro_abi::ReleaseSound;
#[cfg(not(test))]
use crate::graphics::context_abi::SetContext;
#[cfg(not(test))]
use crate::graphics::context_abi::SetContextForeGroundColor;
#[cfg(not(test))]
use crate::graphics::drawable_abi::DestroyDrawable;
#[cfg(not(test))]
use crate::graphics::frame_abi::DrawFilledRectangle;
#[cfg(not(test))]
use crate::graphics::frame_abi::DrawStamp;
#[cfg(not(test))]
use crate::graphics::pixmap_abi::IncFrameIndex;
#[cfg(not(test))]
use crate::graphics::pixmap_abi::ReleaseDrawable;
#[cfg(not(test))]
use crate::graphics::pixmap_abi::SetAbsFrameIndex;
#[cfg(not(test))]
use crate::ui::sis::capacity::GetStorageBayCapacity;
#[cfg(not(test))]
extern "C" {
    static mut RadarContext: *mut crate::graphics::native_abi::context_desc;
}
#[cfg(not(test))]
use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

/// Load the global LanderFrame graphics. Called by `load_lander_data`.
pub fn load_lander_frames() {
    #[cfg(not(test))]
    {
        let Ok(mut state) = LANDER_FRAMES.lock() else {
            return;
        };
        if state.loaded {
            return;
        }

        let keys: [&str; 8] = [
            "graphics.lander",
            "graphics.quake",
            "graphics.lightning",
            "graphics.lavaspot",
            "graphics.landershield",
            "graphics.landerlaunch",
            "graphics.landerreturn",
            "graphics.orbview",
        ];

        let mut port = CffiResourcePort::default();
        let mut loaded: Vec<(&str, *mut c_void)> = Vec::with_capacity(8);
        for key in &keys {
            match port.load(key) {
                Ok(handle) if !handle.is_null() => {
                    loaded.push((key, handle));
                }
                _ => {
                    // `port.free` performs ReleaseDrawable+DestroyDrawable
                    // internally; an explicit destroy here would double-free.
                    for (k, _) in loaded.iter().rev() {
                        port.free(k);
                    }
                    return;
                }
            }
        }

        let sounds = match port.load("sounds.lander") {
            Ok(handle) if !handle.is_null() => handle,
            _ => {
                for (key, _) in loaded.iter().rev() {
                    port.free(key);
                }
                return;
            }
        };
        for (i, (_, handle)) in loaded.iter().enumerate() {
            state.frames[i] = *handle;
        }
        state.sounds = sounds;
        state.loaded = true;
    }
}

/// Free the global LanderFrame graphics. Called by `free_lander_data`.
pub fn free_lander_frames() {
    #[cfg(not(test))]
    {
        let Ok(mut state) = LANDER_FRAMES.lock() else {
            return;
        };
        if !state.loaded {
            return;
        }
        unsafe {
            for frame in state.frames.iter_mut() {
                if !frame.is_null() {
                    DestroyDrawable(ReleaseDrawable((*frame).cast()));
                    *frame = std::ptr::null_mut();
                }
            }
            if !state.sounds.is_null() {
                crate::sound::heart_ffi::DestroySound((ReleaseSound((state.sounds).cast())).cast());
                state.sounds = std::ptr::null_mut();
            }
        }
        state.loaded = false;
    }
}

/// Borrow the production asset set loaded by `load_lander_data`.
pub fn borrowed_assets(
) -> Result<super::resources::BorrowedPlanetSideAssets, super::runtime::AdapterError> {
    let state = LANDER_FRAMES
        .lock()
        .map_err(|_| super::runtime::AdapterError::new("lander_assets_lock"))?;
    if !state.loaded || state.frames.iter().any(|frame| frame.is_null()) || state.sounds.is_null() {
        return Err(super::runtime::AdapterError::new(
            "lander_assets_not_loaded",
        ));
    }
    Ok(super::resources::BorrowedPlanetSideAssets {
        graphics: state.frames,
        sounds: state.sounds,
    })
}
/// Read `GLOBAL_SIS(NumLanders)` from the global SIS state.
#[cfg(not(test))]
fn sis_num_landers() -> u8 {
    use crate::comm::locdata::CGlobData;
    extern "C" {
        #[link_name = "GlobData"]
        static mut GLOB_DATA: CGlobData;
    }
    unsafe { std::ptr::addr_of!(GLOB_DATA.sis_state.num_landers).read() }
}

/// Read `GLOBAL_SIS(TotalElementMass)` from the global SIS state.
#[cfg(not(test))]
fn sis_total_element_mass() -> u16 {
    use crate::comm::locdata::CGlobData;
    extern "C" {
        #[link_name = "GlobData"]
        static mut GLOB_DATA: CGlobData;
    }
    unsafe { std::ptr::addr_of!(GLOB_DATA.sis_state.total_element_mass).read() }
}

/// Read a game state bit-field via the FFI singleton.
#[cfg(not(test))]
fn get_game_state(name: &str) -> u32 {
    crate::state::game_state_keys::get_game_state(name)
}

/// Rust port of C `InitLander(BYTE LanderFlags)`.
///
/// Draws the lander status display in the radar context. When `LanderFlags`
/// is zero, reads shield and cargo upgrade state from game state. When
/// non-zero (called from gameopt.c with `OVERRIDE_LANDER_FLAGS`), uses the
/// flag bits directly.
#[no_mangle]
pub extern "C" fn InitLander(lander_flags: u8) {
    #[cfg(not(test))]
    unsafe {
        let Ok(state) = LANDER_FRAMES.lock() else {
            return;
        };
        if !state.loaded || state.frames[0].is_null() {
            return;
        }
        let lander_frame_0 = state.frames[0];

        let old_context = SetContext(RadarContext);
        BatchGraphics();

        let mut r = CRect {
            corner: CPoint { x: 0, y: 0 },
            width: RADAR_WIDTH,
            height: RADAR_HEIGHT,
        };
        SetContextForeGroundColor(BLACK_COLOR);
        DrawFilledRectangle(std::ptr::from_mut(&mut r).cast());

        let num_landers = sis_num_landers();
        if num_landers != 0 || lander_flags != 0 {
            draw_lander_status(lander_frame_0, lander_flags);
        }

        UnbatchGraphics();
        SetContext(old_context);
    }
    #[cfg(test)]
    {
        let _ = lander_flags;
    }
}

#[cfg(not(test))]
unsafe fn draw_lander_status(lander_frame_0: *mut c_void, lander_flags: u8) {
    let facing = angle_to_facing(FULL_CIRCLE) << 1;

    let mut s = CStamp {
        origin: CPoint { x: 0, y: 0 },
        frame: (SetAbsFrameIndex((lander_frame_0).cast(), facing)).cast(),
    };
    DrawStamp(std::ptr::from_mut(&mut s).cast());

    let (shield_flags, capacity_shift);
    if lander_flags == 0 {
        shield_flags = get_game_state("LANDER_SHIELDS") as u8;
        capacity_shift = get_game_state("IMPROVED_LANDER_CARGO") as u8;
    } else {
        shield_flags = lander_flags
            & ((1 << EARTHQUAKE_DISASTER)
                | (1 << BIOLOGICAL_DISASTER)
                | (1 << LIGHTNING_DISASTER)
                | (1 << LAVASPOT_DISASTER));
        s.frame = (IncFrameIndex((s.frame).cast())).cast();
        DrawStamp(std::ptr::from_mut(&mut s).cast());
        if lander_flags & (1 << 4) != 0 {
            s.frame = (SetAbsFrameIndex((s.frame).cast(), 57)).cast();
        } else {
            s.frame = (SetAbsFrameIndex((s.frame).cast(), facing + 3)).cast();
            DrawStamp(std::ptr::from_mut(&mut s).cast());
            s.frame = (IncFrameIndex((s.frame).cast())).cast();
        }
        DrawStamp(std::ptr::from_mut(&mut s).cast());
        if lander_flags & (1 << 5) == 0 {
            capacity_shift = 0;
        } else {
            capacity_shift = 1;
            s.frame = (SetAbsFrameIndex((s.frame).cast(), 59)).cast();
            DrawStamp(std::ptr::from_mut(&mut s).cast());
        }
        if lander_flags & (1 << 6) != 0 {
            s.frame = (SetAbsFrameIndex((s.frame).cast(), 58)).cast();
        } else {
            s.frame = (SetAbsFrameIndex((s.frame).cast(), facing + 2)).cast();
        }
        DrawStamp(std::ptr::from_mut(&mut s).cast());
    }

    let free_space = GetStorageBayCapacity().wrapping_sub(sis_total_element_mass());
    let max_scrounged = MAX_SCROUNGED
        .checked_shl(u32::from(capacity_shift))
        .unwrap_or(MAX_SCROUNGED);
    if (free_space as i32) < (max_scrounged as i32) {
        let black_height = max_scrounged
            .saturating_sub(free_space >> capacity_shift)
            .saturating_add(1);
        let mut cargo_rect = CRect {
            corner: CPoint { x: 1, y: 0 },
            width: 4,
            height: black_height as i16,
        };
        SetContextForeGroundColor(BLACK_COLOR);
        DrawFilledRectangle(std::ptr::from_mut(&mut cargo_rect).cast());
    }

    s.frame = (SetAbsFrameIndex((lander_frame_0).cast(), 37)).cast();
    if shield_flags & (1 << EARTHQUAKE_DISASTER) != 0 {
        DrawStamp(std::ptr::from_mut(&mut s).cast());
    }
    s.frame = (IncFrameIndex((s.frame).cast())).cast();
    if shield_flags & (1 << BIOLOGICAL_DISASTER) != 0 {
        DrawStamp(std::ptr::from_mut(&mut s).cast());
    }
    s.frame = (IncFrameIndex((s.frame).cast())).cast();
    if shield_flags & (1 << LIGHTNING_DISASTER) != 0 {
        DrawStamp(std::ptr::from_mut(&mut s).cast());
    }
    s.frame = (IncFrameIndex((s.frame).cast())).cast();
    if shield_flags & (1 << LAVASPOT_DISASTER) != 0 {
        DrawStamp(std::ptr::from_mut(&mut s).cast());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angle_to_facing_full_circle_is_sixteen() {
        assert_eq!(angle_to_facing(FULL_CIRCLE), 16);
    }

    #[test]
    fn disaster_bits_match_enum() {
        assert_eq!(BIOLOGICAL_DISASTER, 0);
        assert_eq!(EARTHQUAKE_DISASTER, 1);
        assert_eq!(LIGHTNING_DISASTER, 2);
        assert_eq!(LAVASPOT_DISASTER, 3);
    }

    #[test]
    fn radar_dimensions_match_c() {
        assert_eq!(RADAR_WIDTH, 56);
        assert_eq!(RADAR_HEIGHT, 53);
    }

    #[test]
    fn max_scrounged_matches_c() {
        assert_eq!(MAX_SCROUNGED, 50);
    }

    #[cfg(not(test))]
    #[test]
    fn black_color_is_opaque_like_make_rgb15() {
        // MAKE_RGB15 hardcodes a = 0xff; BUILD_COLOR's palette index is discarded.
        assert_eq!(
            (BLACK_COLOR.r, BLACK_COLOR.g, BLACK_COLOR.b, BLACK_COLOR.a),
            (0, 0, 0, 0xff)
        );
    }
}
