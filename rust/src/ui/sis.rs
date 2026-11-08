#[cfg(not(test))]
use crate::engine::World;
use crate::engine::{Kernel, WorldData};
use crate::graphics::context_abi::ContextLease;
#[cfg(not(test))]
use crate::planets::starmap::current_catalog;
#[cfg(not(test))]
use crate::ui::palette::pilot_color;
use crate::ui::rgb15;

use crate::navigation::abi::*;

#[cfg(not(test))]
use std::{mem, ptr};

pub(crate) mod gauges;
pub use gauges::delta_sis_gauges;
mod message_display;
pub use message_display::{draw_message, set_menu_flash};

mod message;
pub use message::default_message_text;

mod status;
pub use status::{draw_status_message, format_date, status_message_text};

mod title;
pub use title::{draw_hyper_coords, draw_title};

pub(crate) mod capacity;

#[cfg(not(test))]
static mut LAST_PILOT: BOOLEAN = 0;
#[cfg(not(test))]
static mut NEXT_PILOT_TIME: TimeCount = 0;
#[cfg(not(test))]
static mut PILOT_CYCLE_INDEX: DWORD = 0;

#[cfg(not(test))]
const UNDEFINED_DELTA_LOCAL: SIZE = 0x7fff;
#[cfg(not(test))]
const SFR_MENU_3DO: usize = usize::MAX;
#[cfg(not(test))]
const SFR_MENU_ANY: usize = usize::MAX - 1;
#[cfg(not(test))]
const BATTLE_FRAME_RATE: u32 = ONE_SECOND / 24;
const OPT_PC: i32 = crate::navigation::abi::OPT_PC as i32;
const SIS_MESSAGE_HEIGHT: SIZE = crate::navigation::abi::SIS_MESSAGE_HEIGHT as SIZE;

#[cfg(not(test))]
pub(super) unsafe fn game_string(index: u32) -> *mut UNICODE {
    GetStringAddress(SetAbsStringTableIndex(
        Kernel::current().read(|owner| owner.game_strings),
        index as u16,
    ))
    .cast()
}

const SIS_ORG_X: SIZE = 7;
const SIS_ORG_Y: SIZE = 10;
const SIS_TITLE_BOX_WIDTH: SIZE = 57;
const SIS_TITLE_WIDTH: SIZE = 55;
const SIS_TITLE_HEIGHT: SIZE = 8;
const SIS_SPACER_BOX_WIDTH: SIZE = 12;
const STATUS_MESSAGE_WIDTH: SIZE = 60;
const STATUS_MESSAGE_HEIGHT: SIZE = 7;
const RADAR_WIDTH: SIZE = 56;
const RADAR_HEIGHT: SIZE = 53;
#[cfg(not(test))]
const CLEAR_SIS_RADAR: BYTE = 1 << 2;
#[cfg(not(test))]
const DRAW_SIS_DISPLAY: BYTE = 1 << 3;

#[inline]
#[cfg(not(test))]
unsafe fn sis_screen_width() -> i32 {
    ScreenWidth - 78
}
#[inline]
#[cfg(not(test))]
unsafe fn sis_screen_height() -> i32 {
    ScreenHeight - 13
}
#[inline]
#[cfg(not(test))]
unsafe fn radar_x() -> i32 {
    ScreenWidth - 60
}
#[inline]
#[cfg(not(test))]
unsafe fn radar_y() -> i32 {
    ScreenHeight - 56
}

#[inline]
fn sis() -> crate::engine::Projection<SIS_STATE> {
    // SAFETY: flagship is a field of the leased World; map keeps its thread
    // assertion and exclusive borrow flag until the SIS projection is dropped.
    unsafe { WorldData::current().map(|world| &raw mut (*world).flagship) }
}
#[inline]
#[cfg(not(test))]
unsafe fn set_color(color: Color) {
    SetContextForeGroundColor(color);
}

/// # Safety
/// The graphics system and screen contexts must be initialized on the game thread.
#[cfg(not(test))]
pub unsafe fn repair_sisborder() {
    let old = SetContext(Kernel::current().read(|owner| owner.screen_context));
    BatchGraphics();
    let mut r: RECT = mem::zeroed();
    r.corner.x = SIS_ORG_X - 1;
    r.corner.y = SIS_ORG_Y - 1;
    r.extent.width = 1;
    r.extent.height = (sis_screen_height() + 2) as SIZE;
    set_color(rgb15(0x10, 0x10, 0x10));
    DrawFilledRectangle(&mut r);
    set_color(rgb15(0x08, 0x08, 0x08));
    r.corner.x = (i32::from(r.corner.x) + (sis_screen_width() + 2) - 1) as SIZE;
    DrawFilledRectangle(&mut r);
    r.corner.x = SIS_ORG_X - 1;
    r.corner.y = (i32::from(r.corner.y) + (sis_screen_height() + 2) - 1) as SIZE;
    r.extent.width = (sis_screen_width() + 2) as SIZE;
    r.extent.height = 1;
    DrawFilledRectangle(&mut r);
    UnbatchGraphics();
    SetContext(old);
}

/// # Safety
/// The SIS status context and graphics system must be active.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn ClearSISRect(clear_flags: BYTE) {
    let old = SetContext(Kernel::current().read(|owner| owner.status_context));
    let old_color = SetContextForeGroundColor(rgb15(0x0a, 0x0a, 0x0a));
    BatchGraphics();
    if clear_flags & DRAW_SIS_DISPLAY as BYTE != 0 {
        DeltaSISGauges(
            UNDEFINED_DELTA_LOCAL,
            UNDEFINED_DELTA_LOCAL,
            UNDEFINED_DELTA_LOCAL as i32,
        );
    }
    if clear_flags & CLEAR_SIS_RADAR as BYTE != 0 {
        DrawMenuStateStrings(!0, 1);
    }
    UnbatchGraphics();
    SetContextForeGroundColor(old_color);
    SetContext(old);
}

/// # Safety
/// pStr must be a valid NUL-terminated UTF-8 string and graphics state initialized.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn DrawSISTitle(p_str: *const UNICODE) {
    let text = std::ffi::CStr::from_ptr(p_str.cast());
    let width = ScreenWidth;
    Kernel::current().read(|kernel| draw_title(kernel, width, text));
}

/// # Safety
/// Graphics state must be initialized on the game thread.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn DrawHyperCoords(universe: POINT) {
    let text = title::hyper_coord_text(universe);
    DrawSISTitle(text.as_ptr().cast());
}

/// # Safety
/// pStr must be null or a valid NUL-terminated string; UI graphics state must be initialized.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn DrawSISMessage(p_str: *const UNICODE) {
    DrawSISMessageEx(p_str, -1, -1, DSME_NONE as COUNT);
}

/// # Safety
/// pStr must be null or valid UTF-8 and graphics state must be initialized on the game thread.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn DrawSISMessageEx(
    p_str: *const UNICODE,
    cur_pos: SIZE,
    ex_pos: SIZE,
    flags: COUNT,
) -> BOOLEAN {
    let text = (!p_str.is_null()).then(|| std::ffi::CStr::from_ptr(p_str.cast()));
    let width = ScreenWidth;
    let catalog = current_catalog();
    Kernel::current().read(|kernel| {
        let text = text.map_or_else(
            || {
                std::borrow::Cow::Owned(
                    World::current().read(|world| default_message_text(kernel, world, catalog)),
                )
            },
            std::borrow::Cow::Borrowed,
        );
        u32::from(draw_message(kernel, width, &text, cur_pos, ex_pos, flags))
    })
}

/// # Safety
/// buf must have bufLen writable bytes.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn DateToString(
    buf: *mut std::ffi::c_char,
    buf_len: usize,
    month_index: BYTE,
    day_index: BYTE,
    year_index: COUNT,
) {
    let month =
        std::ffi::CStr::from_ptr(game_string(MONTHS_STRING_BASE + month_index as u32 - 1).cast());
    let text = format_date(month, day_index, year_index);
    libc::snprintf(buf, buf_len, c"%s".as_ptr(), text.as_ptr());
}

/// # Safety
/// r must point to writable RECT storage.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn GetStatusMessageRect(r: *mut RECT) {
    *r = status::message_rect();
}

/// # Safety
/// Status graphics and SIS state must be initialized.
#[cfg(not(test))]
pub unsafe fn draw_captains_name() {
    Kernel::current()
        .read(|kernel| World::current().read(|world| gauges::draw_captains_name(kernel, world)));
}

/// # Safety
/// Graphics, fonts, game strings and SIS inventory must be initialized.
#[cfg(not(test))]
pub unsafe fn draw_flagship_stats() {
    let s = sis().read(|state| *state);
    let super::flagship_stats::Stats {
        energy_regen,
        energy_wait,
        thrust,
        turn_wait,
        fuel,
    } = super::flagship_stats::calculate(&s);
    let old = SetContext(Kernel::current().read(|owner| owner.space_context));
    let old_font = SetContextFont(Kernel::current().read(|owner| owner.star_con_font));
    let effect = SetContextFontEffect(ptr::null_mut());
    let mut leading: SIZE = 0;
    GetContextFontLeading(&mut leading);
    let mut r: RECT = mem::zeroed();
    r.corner.y = (sis_screen_height() - (4 * i32::from(leading))) as SIZE;
    r.extent.width = sis_screen_width() as SIZE;
    r.extent.height = (4 * i32::from(leading)) as SIZE;
    let old_color = SetContextForeGroundColor(rgb15(0, 0, 0));
    DrawFilledRectangle(&mut r);
    leading = (i32::from(leading) - 3) as SIZE;
    let mut t: TEXT = mem::zeroed();
    t.baseline.x = (sis_screen_width() / 6) as SIZE;
    t.baseline.y = (i32::from(r.corner.y) + i32::from(leading) + 3) as SIZE;
    t.align = TEXT_ALIGN_ALIGN_RIGHT;
    t.CharCount = !0;
    SetContextFontEffect(SetAbsFrameIndex(
        Kernel::current().read(|owner| owner.font_grad_frame),
        4,
    ));
    for i in 0..4 {
        t.pStr = game_string(FLAGSHIP_STRING_BASE + i);
        font_DrawText(&mut t);
        if i != 3 {
            t.baseline.y = (i32::from(t.baseline.y) + i32::from(leading)) as SIZE;
        }
    }
    let mut b = [0u8; 128];
    t.baseline.x = (i32::from(t.baseline.x) + 5) as SIZE;
    t.baseline.y = (i32::from(r.corner.y) + i32::from(leading) + 3) as SIZE;
    t.align = TEXT_ALIGN_ALIGN_LEFT;
    t.pStr = b.as_mut_ptr().cast();
    for (n, slot) in [15usize, 14, 13, 0].iter().enumerate() {
        weapon_name(s.module_slots[*slot], b.as_mut_ptr());
        font_DrawText(&mut t);
        if n != 3 {
            t.baseline.y = (i32::from(t.baseline.y) + i32::from(leading)) as SIZE;
        }
    }
    t.baseline.x = (i32::from(r.extent.width) - 25) as SIZE;
    t.baseline.y = (i32::from(r.corner.y) + i32::from(leading) + 3) as SIZE;
    t.align = TEXT_ALIGN_ALIGN_RIGHT;
    SetContextFontEffect(SetAbsFrameIndex(
        Kernel::current().read(|owner| owner.font_grad_frame),
        5,
    ));
    for i in 4..8 {
        t.pStr = game_string(FLAGSHIP_STRING_BASE + i);
        font_DrawText(&mut t);
        if i != 7 {
            t.baseline.y = (i32::from(t.baseline.y) + i32::from(leading)) as SIZE;
        }
    }
    t.baseline.x = (i32::from(r.extent.width) - 2) as SIZE;
    t.baseline.y = (i32::from(r.corner.y) + i32::from(leading) + 3) as SIZE;
    t.align = TEXT_ALIGN_ALIGN_RIGHT;
    t.pStr = b.as_mut_ptr().cast();
    libc::snprintf(
        b.as_mut_ptr().cast(),
        b.len(),
        c"%4u".as_ptr(),
        i32::from(thrust) * 4,
    );
    font_DrawText(&mut t);
    t.baseline.y = (i32::from(t.baseline.y) + i32::from(leading)) as SIZE;
    libc::snprintf(
        b.as_mut_ptr().cast(),
        b.len(),
        c"%4u".as_ptr(),
        1 + 17 - i32::from(turn_wait),
    );
    font_DrawText(&mut t);
    t.baseline.y = (i32::from(t.baseline.y) + i32::from(leading)) as SIZE;
    let energy = ((100 * ONE_SECOND * energy_regen as u32
        / ((1 + energy_wait as u32) * BATTLE_FRAME_RATE))
        + 5)
        / 10;
    libc::snprintf(
        b.as_mut_ptr().cast(),
        b.len(),
        c"%2u.%1u".as_ptr(),
        energy / 10,
        energy % 10,
    );
    font_DrawText(&mut t);
    t.baseline.y = (i32::from(t.baseline.y) + i32::from(leading)) as SIZE;
    libc::snprintf(
        b.as_mut_ptr().cast(),
        b.len(),
        c"%4u".as_ptr(),
        fuel / FUEL_TANK_SCALE,
    );
    font_DrawText(&mut t);
    SetContextFontEffect(effect);
    SetContextForeGroundColor(old_color);
    SetContextFont(old_font);
    SetContext(old);
}

#[cfg(not(test))]
unsafe fn weapon_name(module: BYTE, out: *mut u8) {
    let n = match u32::from(module) {
        GUN_WEAPON => 8,
        BLASTER_WEAPON => 9,
        CANNON_WEAPON => 10,
        BOMB_MODULE_0..=BOMB_MODULE_5 => 11,
        _ => 12,
    };
    libc::snprintf(
        out.cast(),
        128,
        c"%-7.7s".as_ptr(),
        game_string(FLAGSHIP_STRING_BASE + n),
    );
}

/// # Safety
/// Status graphics and flagship frame must be initialized.
#[cfg(not(test))]
pub unsafe fn draw_landers() {
    Kernel::current()
        .read(|kernel| World::current().read(|world| gauges::draw_landers(kernel, world)));
}

/// # Safety
/// Status graphics and SIS inventory must be initialized.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn DrawStorageBays(refresh: BOOLEAN) {
    Kernel::current().read(|kernel| {
        World::current().read(|world| gauges::draw_storage_bays(kernel, world, refresh != 0))
    });
}

/// # Safety
/// pRect must point to writable RECT storage.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn GetGaugeRect(p_rect: *mut RECT, is_crew_rect: BOOLEAN) {
    *p_rect = gauges::gauge_rect(is_crew_rect != 0);
}

/// # Safety
/// SIS graphics, game state and asset resources must be initialized on the game thread.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn DeltaSISGauges(crew_delta: SIZE, fuel_delta: SIZE, resunit_delta: i32) {
    Kernel::current().read(|kernel| {
        World::current().write(|world| {
            gauges::delta_sis_gauges(kernel, world, crew_delta, fuel_delta, resunit_delta)
        })
    });
}

/// # Safety
/// Game state, fonts and graphics must be initialized.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn DrawAutoPilotMessage(reset: BOOLEAN) {
    if reset != 0 {
        LAST_PILOT = 0;
        return;
    }
    let autopilot = WorldData::current().read(|owner| owner.game.autopilot);
    let on =
        (autopilot.x != -1 && autopilot.y != -1) || sis().read(|owner| owner.fuel_on_board) == 0;
    if on || LAST_PILOT != 0 {
        if !on {
            DrawSISMessage(ptr::null());
            PILOT_CYCLE_INDEX = 0;
        } else if GetTimeCounter() >= NEXT_PILOT_TIME {
            if crate::engine::World::current().read(|world| world.activity()) & CHECK_ABORT as u16
                == 0
                && sis().read(|owner| owner.crew_enlisted) != COUNT::MAX
            {
                let old = SetContext(Kernel::current().read(|owner| owner.off_screen_context));
                SetContextForeGroundColor(pilot_color(PILOT_CYCLE_INDEX as usize));
                DrawSISMessageEx(
                    game_string(
                        NAVIGATION_STRING_BASE
                            + if sis().read(|owner| owner.fuel_on_board) == 0 {
                                2
                            } else {
                                3
                            },
                    ),
                    -1,
                    -1,
                    DSME_MYCOLOR as COUNT,
                );
                SetContext(old);
            }
            PILOT_CYCLE_INDEX = PILOT_CYCLE_INDEX.wrapping_add(1) % 7 as DWORD;
            NEXT_PILOT_TIME = GetTimeCounter().wrapping_add(ONE_SECOND * 3 / 40);
        }
        LAST_PILOT = on as BOOLEAN;
    }
}

pub fn set_flash_rect(
    highlight: &crate::engine::kernel::SisHighlight,
    screen: &ContextLease,
    clip: RECT,
    rect: Option<RECT>,
) {
    screen.with_selected(|| {
        let mut slot = highlight.context.borrow_mut();
        let current = &mut *slot;
        if let Some(mut rect) = rect.filter(|rect| rect.extent.width != 0) {
            rect.corner.x = (i32::from(rect.corner.x) + i32::from(clip.corner.x)) as SIZE;
            rect.corner.y = (i32::from(rect.corner.y) + i32::from(clip.corner.y)) as SIZE;
            if let Some(current) = current {
                current.set_rect(rect);
            } else {
                let mut highlight = screen.highlight(rect);
                highlight.set_merge_factors(3, 2, 2);
                highlight.set_speed(0, ONE_SECOND / 16, 0, ONE_SECOND / 16);
                highlight.set_frame_time(ONE_SECOND / 16);
                *current = Some(highlight.schedule());
            }
        } else {
            *current = None;
        }
    });
}

/// # Safety
/// pRect must be null, a valid RECT, or one of the SIS menu sentinel pointers; flash and graphics systems must be initialized.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn SetFlashRect(p_rect: *const RECT) {
    let (retained_status_context, screen, preferences, highlight) =
        Kernel::current().read(|kernel| {
            (
                kernel.status_context,
                kernel.screen_context,
                kernel.configuration.preferences(),
                kernel.sis_highlight.clone(),
            )
        });
    let mut clip: RECT = mem::zeroed();
    let mut temp: RECT = mem::zeroed();
    let mut rect = p_rect;
    if rect as usize != SFR_MENU_3DO && rect as usize != SFR_MENU_ANY {
        GetContextClipRect(&mut clip);
    } else if preferences.menu == OPT_PC && rect as usize != SFR_MENU_ANY {
        rect = ptr::null();
    } else {
        let old = SetContext(retained_status_context);
        GetContextClipRect(&mut clip);
        temp.corner.x = (radar_x() - i32::from(clip.corner.x)) as SIZE;
        temp.corner.y = (radar_y() - i32::from(clip.corner.y)) as SIZE;
        temp.extent.width = RADAR_WIDTH;
        temp.extent.height = RADAR_HEIGHT;
        SetContext(old);
        rect = &temp;
    }
    if !rect.is_null() && (*rect).extent.width != 0 {
        let screen = ContextLease::retain(
            std::ptr::NonNull::new(screen)
                .unwrap_or_else(|| panic!("flash screen context not admitted")),
        );
        set_flash_rect(&highlight, &screen, clip, Some(*rect));
    } else {
        *highlight.context.borrow_mut() = None;
    }
}
/// # Safety
/// pStr must be null or a valid NUL-terminated string; status graphics must be initialized.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn DrawStatusMessage(p_str: *const UNICODE) {
    let text = (!p_str.is_null()).then(|| std::ffi::CStr::from_ptr(p_str.cast()));
    Kernel::current().read(|kernel| {
        World::current().read(|world| draw_status_message(kernel, world, text));
    });
}

/// # Safety
/// Graphics, fonts and SIS strings must be initialized.
#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn DrawFlagshipName(in_status_area: BOOLEAN) {
    let width = ScreenWidth;
    Kernel::current().read(|kernel| {
        World::current()
            .read(|world| gauges::draw_flagship_name(kernel, world, width, in_status_area != 0))
    });
}
