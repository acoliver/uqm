//! Live starmap cursor and input callback, ported from `pstarmap.c`.
//!
//! The cursor belongs to the game thread. Automation only observes it; all
//! movement and destination selection still go through the menu callback.

use crate::comm::dispatch::CPoint;

#[no_mangle]
pub static mut cursorLoc: CPoint = CPoint { x: 0, y: 0 };
static mut ACTIVE: bool = false;
static mut LAST_BUF: [u8; 256] = [0; 256];

/// Standard `DoInput` header of `MENU_STATE`; no private fields are accessed.
pub use crate::ships::native::abi::MENU_STATE as MenuHeader;

#[cfg(not(test))]
use crate::planets::pstarmap::UpdateFuelRequirement;
#[cfg(test)]
extern "C" {
    fn UpdateFuelRequirement();
}
#[cfg(not(test))]
use crate::planets::pstarmap::ZoomStarMap;
#[cfg(test)]
extern "C" {
    fn ZoomStarMap(direction: i16);
}

use crate::battle::sounds::engine::PlayMenuSound;

use crate::mainloop::input_abi::live::SetMenuRepeatDelay;

use crate::mainloop::input_abi::live::SetMenuSounds;

use crate::planets::pstarmap::flashCurrentLocation;

use crate::planets::pstarmap::search::DoStarSearch;

use crate::planets::pstarmap::DrawStarMap;

use crate::planets::pstarmap::UpdateCursorInfo;

use crate::planets::pstarmap::UpdateCursorLocation;
#[allow(non_snake_case)]
extern "C" {
    static ScreenWidth: i32;
    static ScreenHeight: i32;
    static mut GlobData: crate::comm::locdata::CGlobData;
    static CurStarDescPtr: *const crate::savegame::runtime::star::CStarDesc;
}
use crate::threading::pacing::linked::sleep_thread_until as SleepThreadUntil;

/// Copy the active cursor on its owning game thread.
///
/// # Safety
/// Must be called on the game thread, never concurrently with menu input.
pub unsafe fn cursor() -> Option<(i16, i16)> {
    ACTIVE.then(|| {
        let point = std::ptr::addr_of!(cursorLoc).read();
        (point.x, point.y)
    })
}

/// `DoMoveCursor`, including ordinary search, zoom and debug input behavior.
///
/// # Safety
/// `menu` must be the live `MENU_STATE`, called by `DoInput` on the game thread.
#[no_mangle]
pub unsafe extern "C" fn DoMoveCursor(menu: *mut MenuHeader) -> u32 {
    use crate::c_bindings::controller_input::PulsedInputState;
    let time_in = crate::time::counter_abi::GetTimeCounter();
    let glob = std::ptr::addr_of_mut!(GlobData);
    let activity = (*glob).game_state.current_activity;
    let hyper = activity & 0xff == 3;
    let last = std::ptr::addr_of_mut!(LAST_BUF).cast::<u8>();
    if (*menu).Initialized == 0 {
        (*menu).Initialized = 1;
        (*menu).InputFunc = Some(DoMoveCursor);
        ACTIVE = true;
        let universe = if hyper {
            let sis = &(*glob).sis_state;
            let log_units_x = (((ScreenWidth - 64) << 5) >> 4) * 195;
            let log_units_y = ((ScreenHeight << 5) >> 4) * 25;
            CPoint {
                x: ((sis.log_x * 6250 + (log_units_x >> 1)) / log_units_x) as i16,
                y: (9999 - (sis.log_y * 625 + (log_units_y >> 1)) / log_units_y) as i16,
            }
        } else {
            (*CurStarDescPtr).star_pt
        };
        flashCurrentLocation(&universe);
        *last = 0;
        UpdateCursorInfo((last).cast());
        UpdateFuelRequirement();
        return 1;
    }
    let keys = std::ptr::addr_of!(PulsedInputState).read().menu;
    if keys[10] != 0 {
        ACTIVE = false;
        return 0;
    } else if keys[9] != 0 {
        let selected = std::ptr::addr_of!(cursorLoc).read();
        (*glob).game_state.autopilot = crate::comm::locdata::CPoint {
            x: selected.x,
            y: selected.y,
        };
        #[cfg(feature = "debug-process")]
        if crate::front_end::uqmdebug::INSTANT_MOVE != 0 {
            PlayMenuSound(3);
            if hyper {
                crate::front_end::uqmdebug::doInstantMove();
            } else if activity & 0xff == 4 {
                (*glob).game_state.current_activity |= 0x0800;
                crate::front_end::uqmdebug::DEBUG_HOOK =
                    Some(crate::front_end::uqmdebug::doInstantMove);
            }
            ACTIVE = false;
            return 0;
        }
        DrawStarMap(0, std::ptr::null_mut());
    } else if keys[21] != 0 {
        if crate::state::game_state_keys::get_game_state("ARILOU_SPACE_SIDE") <= 1 {
            let old = std::ptr::addr_of!(cursorLoc).read();
            if DoStarSearch(menu) == 0 {
                UpdateCursorLocation(0, 0, &old);
            }
            let invalid = b"  <random garbage>  \0";
            std::ptr::copy_nonoverlapping(invalid.as_ptr(), last, invalid.len());
            UpdateCursorInfo((last).cast());
            UpdateFuelRequirement();
            SetMenuRepeatDelay(14, 105, 7, 1);
            SetMenuSounds(0, 0);
        } else {
            PlayMenuSound(2);
        }
    } else {
        let zoom = if keys[16] != 0 {
            1
        } else if keys[17] != 0 {
            -1
        } else {
            0
        };
        ZoomStarMap(zoom);
        let sx = if keys[8] != 0 {
            1
        } else if keys[7] != 0 {
            -1
        } else {
            0
        };
        let sy = if keys[6] != 0 {
            1
        } else if keys[5] != 0 {
            -1
        } else {
            0
        };
        if sx != 0 || sy != 0 {
            UpdateCursorLocation(sx, sy, std::ptr::null());
            UpdateCursorInfo((last).cast());
            UpdateFuelRequirement();
        }
        SleepThreadUntil(time_in.wrapping_add(14));
    }
    flashCurrentLocation(std::ptr::null());
    let keep_going = (*glob).game_state.current_activity & 0x4000 == 0;
    if !keep_going {
        ACTIVE = false;
    }
    u32::from(keep_going)
}
