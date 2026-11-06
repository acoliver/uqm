//! Live starmap cursor and input callback, ported from `pstarmap.c`.
//!
//! The cursor belongs to the game thread. Automation only observes it; all
//! movement and destination selection still go through the menu callback.

use crate::comm::dispatch::CPoint;
use crate::engine::StoryKey;

#[no_mangle]
pub static mut cursorLoc: CPoint = CPoint { x: 0, y: 0 };
static mut ACTIVE: bool = false;
static mut LAST_BUF: [u8; 256] = [0; 256];

/// Standard `DoInput` header of `MENU_STATE`; no private fields are accessed.
pub use crate::ships::native::abi::MENU_STATE as MenuHeader;

#[cfg(not(test))]
use crate::planets::pstarmap::update_fuel_requirement;
#[cfg(test)]
extern "C" {
    fn update_fuel_requirement();
}
#[cfg(not(test))]
use crate::planets::pstarmap::zoom_star_map;
#[cfg(test)]
extern "C" {
    fn zoom_star_map(direction: i16);
}

use crate::mainloop::input_abi::live::SetMenuRepeatDelay;

use crate::mainloop::input_abi::live::SetMenuSounds;

use crate::planets::pstarmap::flashCurrentLocation;

use crate::planets::pstarmap::search::do_star_search;

use crate::planets::pstarmap::DrawStarMap;

use crate::planets::pstarmap::UpdateCursorInfo;

use crate::planets::pstarmap::UpdateCursorLocation;
#[allow(non_snake_case)]
extern "C" {
    static mut ScreenWidth: i32;
    static mut ScreenHeight: i32;

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

unsafe fn initialize_cursor(
    cx: &mut crate::engine::battle::Cx<'_>,
    menu: &mut MenuHeader,
    hyper: bool,
) {
    let last = std::ptr::addr_of_mut!(LAST_BUF).cast::<u8>();
    menu.Initialized = 1;
    ACTIVE = true;
    let universe = if hyper {
        let sis = &cx.world.data.flagship;
        let log_units_x = (((ScreenWidth - 64) << 5) >> 4) * 195;
        let log_units_y = ((ScreenHeight << 5) >> 4) * 25;
        CPoint {
            x: ((sis.log_x * 6250 + (log_units_x >> 1)) / log_units_x) as i16,
            y: (9999 - (sis.log_y * 625 + (log_units_y >> 1)) / log_units_y) as i16,
        }
    } else {
        (*cx.world
            .star_pointer(crate::planets::starmap::current_catalog()))
        .star_pt
    };
    cx.with_legacy(|| {
        flashCurrentLocation(&universe);
        *last = 0;
        UpdateCursorInfo((last).cast());
        update_fuel_requirement();
    });
}

/// Starmap search, zoom and destination selection for its retained input frame.
///
/// # Safety
/// The menu, starmap and graphics resources must be live on the game thread.
pub unsafe fn do_move_cursor(
    cx: &mut crate::engine::battle::Cx<'_>,
    menu: &mut MenuHeader,
    input: &crate::engine::input::InputSession,
) -> u32 {
    let time_in = crate::time::counter_abi::GetTimeCounter();
    let activity = cx.world.activity();
    let hyper = activity & 0xff == 3;
    let last = std::ptr::addr_of_mut!(LAST_BUF).cast::<u8>();
    if menu.Initialized == 0 {
        initialize_cursor(cx, menu, hyper);
        return 1;
    }
    let keys = input.pulsed().menu;
    if keys[10] != 0 {
        ACTIVE = false;
        return 0;
    } else if keys[9] != 0 {
        let selected = std::ptr::addr_of!(cursorLoc).read();
        cx.world.data.game.autopilot = crate::comm::locdata::CPoint {
            x: selected.x,
            y: selected.y,
        };
        #[cfg(feature = "debug-process")]
        if crate::front_end::uqmdebug::INSTANT_MOVE != 0 {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 3);
            if hyper {
                cx.with_legacy(|| crate::front_end::uqmdebug::do_instant_move());
            } else if activity & 0xff == 4 {
                cx.world.set_activity(activity | 0x0800);
                crate::front_end::uqmdebug::DEBUG_HOOK =
                    Some(crate::front_end::uqmdebug::do_instant_move);
            }
            ACTIVE = false;
            return 0;
        }
        cx.with_legacy(|| DrawStarMap(0, std::ptr::null_mut()));
    } else if keys[21] != 0 {
        if cx.world.story_state(StoryKey::ARILOU_SPACE_SIDE) <= 1 {
            let old = std::ptr::addr_of!(cursorLoc).read();
            if cx.with_legacy(|| do_star_search(input, menu)) == 0 {
                cx.with_legacy(|| UpdateCursorLocation(0, 0, &old));
            }
            cx.with_legacy(|| {
                let invalid = b"  <random garbage>  \0";
                std::ptr::copy_nonoverlapping(invalid.as_ptr(), last, invalid.len());
                UpdateCursorInfo((last).cast());
                update_fuel_requirement();
                SetMenuRepeatDelay(input, 14, 105, 7, 1);
                SetMenuSounds(input, 0, 0);
            });
        } else {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
        }
    } else {
        let zoom = if keys[16] != 0 {
            1
        } else if keys[17] != 0 {
            -1
        } else {
            0
        };
        cx.with_legacy(|| zoom_star_map(zoom));
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
            cx.with_legacy(|| {
                UpdateCursorLocation(sx, sy, std::ptr::null());
                UpdateCursorInfo((last).cast());
                update_fuel_requirement();
            });
        }
        cx.with_legacy(|| SleepThreadUntil(time_in.wrapping_add(14)));
    }
    cx.with_legacy(|| flashCurrentLocation(std::ptr::null()));
    let keep_going = cx.world.activity() & 0x4000 == 0;
    if !keep_going {
        ACTIVE = false;
    }
    u32::from(keep_going)
}
