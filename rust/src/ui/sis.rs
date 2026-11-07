#![cfg(not(test))]

use crate::engine::{Kernel, World, WorldData};
use crate::graphics::context_abi::{ContextLease, ScheduledHighlight};
use crate::planets::starmap::current_catalog;
use crate::ui::{palette::pilot_color, rgb15};

use crate::navigation::abi::*;

use std::{mem, ptr};

mod message;
pub use message::default_message_text;

mod status;
pub use status::{draw_status_message, format_date, status_message_text};

mod title;
pub use title::{draw_hyper_coords, draw_title};

pub(crate) mod capacity;
use capacity::{GetCrewPodCapacity, GetFuelTankCapacity};

static mut LAST_PILOT: BOOLEAN = 0;
static mut NEXT_PILOT_TIME: TimeCount = 0;
static mut PILOT_CYCLE_INDEX: DWORD = 0;
static mut FLASH_CONTEXT: Option<ScheduledHighlight> = None;

const UNDEFINED_DELTA_LOCAL: SIZE = 0x7fff;
const SFR_MENU_3DO: usize = usize::MAX;
const SFR_MENU_ANY: usize = usize::MAX - 1;
const BATTLE_FRAME_RATE: u32 = ONE_SECOND / 24;
const EMPTY_SLOT: BYTE = NUM_MODULES as BYTE;
const OPT_PC: i32 = crate::navigation::abi::OPT_PC as i32;
const SIS_MESSAGE_HEIGHT: SIZE = crate::navigation::abi::SIS_MESSAGE_HEIGHT as SIZE;
const STORAGE_BAY_CAPACITY: COUNT = crate::navigation::abi::STORAGE_BAY_CAPACITY as COUNT;

pub(super) unsafe fn game_string(index: u32) -> *mut UNICODE {
    GetStringAddress(SetAbsStringTableIndex(
        Kernel::current().read(|owner| owner.game_strings),
        index as u16,
    ))
    .cast()
}

const SIS_ORG_X: SIZE = 7;
const SIS_ORG_Y: SIZE = 10;
const STATUS_WIDTH: SIZE = 64;
const SIS_TITLE_BOX_WIDTH: SIZE = 57;
const SIS_TITLE_WIDTH: SIZE = 55;
const SIS_TITLE_HEIGHT: SIZE = 8;
const SIS_SPACER_BOX_WIDTH: SIZE = 12;
const STATUS_MESSAGE_WIDTH: SIZE = 60;
const STATUS_MESSAGE_HEIGHT: SIZE = 7;
const SHIP_NAME_WIDTH: SIZE = 60;
const SHIP_NAME_HEIGHT: SIZE = 7;
const RADAR_WIDTH: SIZE = 56;
const RADAR_HEIGHT: SIZE = 53;
const CLEAR_SIS_RADAR: BYTE = 1 << 2;
const DRAW_SIS_DISPLAY: BYTE = 1 << 3;

#[inline]
unsafe fn sis_screen_width() -> i32 {
    ScreenWidth - 78
}
#[inline]
unsafe fn sis_screen_height() -> i32 {
    ScreenHeight - 13
}
#[inline]
unsafe fn sis_message_width() -> i32 {
    sis_screen_width() - i32::from(SIS_TITLE_BOX_WIDTH) - i32::from(SIS_SPACER_BOX_WIDTH) - 2
}
#[inline]
unsafe fn radar_x() -> i32 {
    ScreenWidth - 60
}
#[inline]
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
unsafe fn set_color(color: Color) {
    SetContextForeGroundColor(color);
}

/// # Safety
/// The graphics system and screen contexts must be initialized on the game thread.
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
pub unsafe extern "C" fn DrawSISTitle(p_str: *const UNICODE) {
    let text = std::ffi::CStr::from_ptr(p_str.cast());
    let width = ScreenWidth;
    Kernel::current().read(|kernel| draw_title(kernel, width, text));
}

/// # Safety
/// Graphics state must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn DrawHyperCoords(universe: POINT) {
    let text = title::hyper_coord_text(universe);
    DrawSISTitle(text.as_ptr().cast());
}

/// # Safety
/// pStr must be null or a valid NUL-terminated string; UI graphics state must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawSISMessage(p_str: *const UNICODE) {
    DrawSISMessageEx(p_str, -1, -1, DSME_NONE as COUNT);
}

/// # Safety
/// pStr must be null or valid UTF-8 and graphics state must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn DrawSISMessageEx(
    p_str: *const UNICODE,
    cur_pos: SIZE,
    ex_pos: SIZE,
    flags: COUNT,
) -> BOOLEAN {
    let old = SetContext(Kernel::current().read(|owner| owner.off_screen_context));
    let mut r: RECT = mem::zeroed();
    r.corner.x = SIS_ORG_X + 1;
    r.corner.y = SIS_ORG_Y - SIS_MESSAGE_HEIGHT;
    r.extent.width = sis_message_width() as SIZE;
    r.extent.height = SIS_MESSAGE_HEIGHT - 1;
    SetContextFGFrame(Kernel::current().read(|owner| owner.screen));
    SetContextClipRect(&mut r);
    BatchGraphics();
    SetContextBackGroundColor(rgb15(0, 0, 0x14));
    let default_text = p_str.is_null().then(|| {
        let catalog = current_catalog();
        World::current().read(|world| {
            Kernel::current().read(|kernel| default_message_text(kernel, world, catalog))
        })
    });
    let text = default_text
        .as_ref()
        .map_or(p_str, |text| text.as_ptr().cast());
    if flags & DSME_MYCOLOR as COUNT == 0 {
        SetContextForeGroundColor(rgb15(0x1b, 0, 0x1b));
    }
    let mut t: TEXT = mem::zeroed();
    t.baseline.y = SIS_MESSAGE_HEIGHT - 2;
    t.pStr = text as *mut UNICODE;
    t.CharCount = !0;
    SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
    if flags & DSME_CLEARFR as COUNT != 0 {
        SetFlashRect(ptr::null());
    }
    if cur_pos < 0 && ex_pos < 0 {
        ClearDrawable();
        t.baseline.x = (sis_message_width() >> 1) as SIZE;
        t.align = TEXT_ALIGN_ALIGN_CENTER;
        font_DrawText(&mut t);
    } else {
        let mut text_r: RECT = mem::zeroed();
        let mut deltas = [0u8; 128];
        t.baseline.x = 3;
        t.align = TEXT_ALIGN_ALIGN_LEFT;
        TextRect(&mut t, &mut text_r, deltas.as_mut_ptr());
        if i32::from(text_r.extent.width) + i32::from(t.baseline.x) + 2 >= i32::from(r.extent.width)
        {
            UnbatchGraphics();
            SetContextClipRect(ptr::null_mut());
            SetContext(old);
            return 0;
        }
        ClearDrawable();
        if cur_pos >= 0 && i32::from(cur_pos) <= i32::from(t.CharCount) {
            let mut cr = text_r;
            for d in &deltas[..cur_pos as usize] {
                cr.corner.x = (i32::from(cr.corner.x) + i32::from(*d)) as SIZE;
            }
            if i32::from(cur_pos) < i32::from(t.CharCount) {
                cr.corner.x = (i32::from(cr.corner.x) - 1) as SIZE;
            }
            if flags & DSME_BLOCKCUR as COUNT != 0 && i32::from(cur_pos) < i32::from(t.CharCount) {
                cr.extent.width = deltas[cur_pos as usize] as SIZE
                    + if i32::from(cur_pos) + 1 == i32::from(t.CharCount) {
                        2
                    } else {
                        1
                    };
            } else {
                cr.extent.width = 1;
            }
            cr.corner.y = 0;
            cr.extent.height = r.extent.height;
            SetContextForeGroundColor(rgb15(0, 0, 0));
            DrawFilledRectangle(&mut cr);
        }
        SetContextForeGroundColor(rgb15(0x1b, 0, 0x1b));
        if ex_pos >= 0 && i32::from(ex_pos) < i32::from(t.CharCount) {
            t.CharCount = ex_pos as COUNT;
            font_DrawText(&mut t);
            SetContextForeGroundColor(rgb15(0x12, 0, 0x12));
            for d in &deltas[..ex_pos as usize] {
                t.baseline.x = (i32::from(t.baseline.x) + i32::from(*d)) as SIZE;
            }
            let string = std::ffi::CStr::from_ptr(t.pStr.cast());
            let (prefix, _) =
                crate::strings::unicode::split_utf8_prefix(string.to_bytes(), ex_pos as usize);
            t.pStr = t.pStr.wrapping_add(prefix.len());
            t.CharCount = !0;
            font_DrawText(&mut t);
        } else {
            font_DrawText(&mut t);
        }
    }
    if flags & DSME_SETFR as COUNT != 0 {
        r.corner.x = 0;
        r.corner.y = 0;
        SetFlashRect(&r);
    }
    UnbatchGraphics();
    SetContextClipRect(ptr::null_mut());
    SetContext(old);
    1
}

/// # Safety
/// buf must have bufLen writable bytes.
#[no_mangle]
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
pub unsafe extern "C" fn GetStatusMessageRect(r: *mut RECT) {
    *r = status::message_rect();
}

/// # Safety
/// Status graphics and SIS state must be initialized.
pub unsafe fn draw_captains_name() {
    let old = SetContext(Kernel::current().read(|owner| owner.status_context));
    let old_font = SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
    let old_color = SetContextForeGroundColor(rgb15(0, 0, 0x14));
    let mut r: RECT = mem::zeroed();
    r.corner.x = 3;
    r.corner.y = 10;
    r.extent.width = SHIP_NAME_WIDTH - 2;
    r.extent.height = SHIP_NAME_HEIGHT;
    DrawFilledRectangle(&mut r);
    let mut t: TEXT = mem::zeroed();
    t.baseline.x = (STATUS_WIDTH >> 1) - 1;
    t.baseline.y = r.corner.y + 6;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.pStr = (**sis()).commander_name.as_mut_ptr().cast();
    t.CharCount = !0;
    SetContextForeGroundColor(rgb15(0x16, 0x0b, 0x1f));
    font_DrawText(&mut t);
    SetContextForeGroundColor(old_color);
    SetContextFont(old_font);
    SetContext(old);
}

/// # Safety
/// Graphics, fonts, game strings and SIS inventory must be initialized.
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
pub unsafe fn draw_landers() {
    let old = SetContext(Kernel::current().read(|owner| owner.status_context));
    let mut stamp: STAMP = mem::zeroed();
    stamp.frame = IncFrameIndex(Kernel::current().read(|owner| owner.flag_stat_frame));
    let mut r: RECT = mem::zeroed();
    GetFrameRect(stamp.frame, &mut r);
    let count = sis().read(|owner| owner.num_landers);
    r.corner.x = ((i32::from(STATUS_WIDTH) >> 1) - i32::from(r.corner.x)) as SIZE;
    stamp.origin.x = (i32::from(r.corner.x)
        - ((i32::from(r.extent.width) * i32::from(count) + 2 * (i32::from(count) - 1)) >> 1))
        as SIZE;
    stamp.origin.y = 29;
    let width = (i32::from(r.extent.width) + 2) as SIZE;
    r.extent.width =
        (i32::from(r.extent.width) * MAX_LANDERS as i32 + 2 * (MAX_LANDERS as i32 - 1) + 2) as SIZE;
    r.corner.x = (i32::from(r.corner.x) - (i32::from(r.extent.width) >> 1)) as SIZE;
    r.corner.y = (i32::from(r.corner.y) + i32::from(stamp.origin.y)) as SIZE;
    SetContextForeGroundColor(rgb15(0, 0, 0));
    DrawFilledRectangle(&mut r);
    let mut i = count;
    while i != 0 {
        DrawStamp(&mut stamp);
        stamp.origin.x = (i32::from(stamp.origin.x) + i32::from(width)) as SIZE;
        i -= 1;
    }
    SetContext(old);
}

/// # Safety
/// Status graphics and SIS inventory must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawStorageBays(refresh: BOOLEAN) {
    let old = SetContext(Kernel::current().read(|owner| owner.status_context));
    let mut r: RECT = mem::zeroed();
    r.extent.width = 2;
    r.extent.height = 4;
    r.corner.y = 123;
    if refresh != 0 {
        r.extent.width = (NUM_MODULE_SLOTS * 3) as SIZE;
        r.corner.x = (STATUS_WIDTH >> 1) - (r.extent.width >> 1);
        SetContextForeGroundColor(rgb15(0, 0, 0));
        DrawFilledRectangle(&mut r);
        r.extent.width = 2;
    }
    let mut i = CountSISPieces(STORAGE_BAY as BYTE) as BYTE;
    if i != 0 {
        r.corner.x = ((i32::from(STATUS_WIDTH) >> 1) - ((i32::from(i) * 3) >> 1)) as SIZE;
        SetContextForeGroundColor(rgb15(0x0a, 0x0a, 0x0a));
        let mut j = sis().read(|owner| owner.total_element_mass);
        while j >= STORAGE_BAY_CAPACITY {
            DrawFilledRectangle(&mut r);
            r.corner.x = (i32::from(r.corner.x) + 3) as SIZE;
            i = (i32::from(i) - 1) as BYTE;
            j = (i32::from(j) - i32::from(STORAGE_BAY_CAPACITY)) as COUNT;
        }
        r.extent.height = ((4 * i32::from(j) + (i32::from(STORAGE_BAY_CAPACITY) - 1))
            / i32::from(STORAGE_BAY_CAPACITY)) as SIZE;
        if r.extent.height != 0 {
            r.corner.y = (i32::from(r.corner.y) + (4 - i32::from(r.extent.height))) as SIZE;
            DrawFilledRectangle(&mut r);
            r.extent.height = (4 - i32::from(r.extent.height)) as SIZE;
            if r.extent.height != 0 {
                r.corner.y = 123;
                SetContextForeGroundColor(rgb15(6, 6, 6));
                DrawFilledRectangle(&mut r);
            }
            r.corner.x = (i32::from(r.corner.x) + 3) as SIZE;
            i = (i32::from(i) - 1) as BYTE;
        }
        r.extent.height = 4;
        SetContextForeGroundColor(rgb15(6, 6, 6));
        while i != 0 {
            DrawFilledRectangle(&mut r);
            r.corner.x = (i32::from(r.corner.x) + 3) as SIZE;
            i = (i32::from(i) - 1) as BYTE;
        }
    }
    SetContext(old);
}

/// # Safety
/// pRect must point to writable RECT storage.
#[no_mangle]
pub unsafe extern "C" fn GetGaugeRect(p_rect: *mut RECT, is_crew_rect: BOOLEAN) {
    (*p_rect).extent.width = 24;
    (*p_rect).corner.x = (STATUS_WIDTH >> 1) - 12;
    (*p_rect).extent.height = 5;
    (*p_rect).corner.y = if is_crew_rect != 0 { 117 } else { 38 };
}

unsafe fn draw_pc_sis() {
    let mut r: RECT = mem::zeroed();
    GetGaugeRect(&mut r, 0);
    let mut t: TEXT = mem::zeroed();
    t.baseline.x = STATUS_WIDTH >> 1;
    t.baseline.y = r.corner.y - 1;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.CharCount = !0;
    SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
    SetContextForeGroundColor(rgb15(0, 0, 0));
    r.corner.y -= 6;
    r.corner.x -= 1;
    r.extent.width += 2;
    DrawFilledRectangle(&mut r);
    SetContextFontEffect(SetAbsFrameIndex(
        Kernel::current().read(|owner| owner.font_grad_frame),
        1,
    ));
    t.pStr = game_string(STATUS_STRING_BASE + 3) as *mut UNICODE;
    font_DrawText(&mut t);
    r.corner.y += 79;
    t.baseline.y += 79;
    DrawFilledRectangle(&mut r);
    SetContextFontEffect(SetAbsFrameIndex(
        Kernel::current().read(|owner| owner.font_grad_frame),
        2,
    ));
    t.pStr = game_string(STATUS_STRING_BASE + 4) as *mut UNICODE;
    font_DrawText(&mut t);
    SetContextFontEffect(ptr::null_mut());
    r.corner.x = 3;
    r.corner.y = 3;
    r.extent.width = 58;
    r.extent.height = 7;
    SetContextForeGroundColor(rgb15(0, 0, 0x14));
    DrawFilledRectangle(&mut r);
    SetContextForeGroundColor(rgb15(2, 4, 0x1e));
    t.baseline.y = r.corner.y + 6;
    t.pStr = game_string(STATUS_STRING_BASE + 5) as *mut UNICODE;
    font_DrawText(&mut t);
}
unsafe fn draw_slots(slots: *const BYTE, len: usize, y: SIZE, mode: u32) {
    let mut stamp: STAMP = mem::zeroed();
    stamp.origin.x = 1;
    stamp.origin.y = y;
    for i in 0..len {
        let piece = *slots.add(i);
        if piece < EMPTY_SLOT {
            stamp.frame = SetAbsFrameIndex(
                Kernel::current().read(|owner| owner.flag_stat_frame),
                (u32::from(piece) + 1 + mode) as COUNT,
            );
            DrawStamp(&mut stamp);
            if mode != 2 {
                stamp.frame = IncFrameIndex(stamp.frame);
                DrawStamp(&mut stamp);
            }
        }
        stamp.origin.y = (i32::from(stamp.origin.y) - 3) as SIZE;
    }
}
unsafe fn draw_support_ships() {
    let points = [
        (3, 62),
        (45, 62),
        (3, 78),
        (45, 78),
        (3, 46),
        (45, 46),
        (3, 94),
        (45, 94),
        (3, 30),
        (45, 30),
        (3, 110),
        (45, 110),
    ];
    let icons = World::current().read(|world| {
        let ships = world.built_ships_ref();
        std::iter::successors(ships.head(), |&id| ships.succ(id))
            .map(|id| ships.get(id).icons)
            .collect::<Vec<_>>()
    });
    for (icon, (x, y)) in icons.into_iter().zip(points) {
        let mut stamp = STAMP {
            origin: POINT { x, y },
            frame: icon,
        };
        DrawStamp(&mut stamp);
    }
}

unsafe fn delta_crew(delta: SIZE) {
    if delta == 0 {
        return;
    }
    if delta != UNDEFINED_DELTA_LOCAL {
        let cap = GetCrewPodCapacity();
        let st = sis();
        if delta < 0 && (**st).crew_enlisted <= (-i32::from(delta)) as COUNT {
            (**st).crew_enlisted = 0;
        } else {
            (**st).crew_enlisted = (((**st).crew_enlisted as i32 + delta as i32) as COUNT).min(cap);
        }
    }
    let mut b = [0u8; 60];
    libc::snprintf(
        b.as_mut_ptr().cast(),
        b.len(),
        c"%u".as_ptr(),
        i32::from(sis().read(|owner| owner.crew_enlisted)),
    );
    let mut r: RECT = mem::zeroed();
    GetGaugeRect(&mut r, 1);
    let mut t: TEXT = mem::zeroed();
    t.baseline.x = STATUS_WIDTH >> 1;
    t.baseline.y = r.corner.y + r.extent.height;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.pStr = b.as_mut_ptr().cast();
    t.CharCount = !0;
    SetContextForeGroundColor(rgb15(0, 0, 0));
    DrawFilledRectangle(&mut r);
    SetContextForeGroundColor(rgb15(0, 0x0e, 0));
    font_DrawText(&mut t);
}
unsafe fn delta_fuel(delta: SIZE) {
    if delta == 0 {
        return;
    }
    let cap = GetFuelTankCapacity();
    let st = sis();
    let old = if delta == UNDEFINED_DELTA_LOCAL {
        COUNT::MAX
    } else {
        ((**st).fuel_on_board / FUEL_TANK_SCALE) as COUNT
    };
    if delta != UNDEFINED_DELTA_LOCAL {
        if delta < 0 && (**st).fuel_on_board <= (-i32::from(delta)) as DWORD {
            (**st).fuel_on_board = 0;
        } else {
            (**st).fuel_on_board = (**st)
                .fuel_on_board
                .wrapping_add(i32::from(delta) as DWORD)
                .min(cap);
        }
    }
    let new = ((**st).fuel_on_board / FUEL_TANK_SCALE) as COUNT;
    if new != old {
        let mut b = [0u8; 60];
        libc::snprintf(
            b.as_mut_ptr().cast(),
            b.len(),
            c"%u".as_ptr(),
            i32::from(new),
        );
        let mut r: RECT = mem::zeroed();
        GetGaugeRect(&mut r, 0);
        let mut t: TEXT = mem::zeroed();
        t.baseline.x = STATUS_WIDTH >> 1;
        t.baseline.y = r.corner.y + r.extent.height;
        t.align = TEXT_ALIGN_ALIGN_CENTER;
        t.pStr = b.as_mut_ptr().cast();
        t.CharCount = !0;
        SetContextForeGroundColor(rgb15(0, 0, 0));
        DrawFilledRectangle(&mut r);
        SetContextForeGroundColor(rgb15(0x13, 0, 0));
        font_DrawText(&mut t);
    }
}
unsafe fn delta_res(delta: SIZE, mode: StatMsgMode) {
    if delta == 0 {
        return;
    }
    if delta != UNDEFINED_DELTA_LOCAL {
        let s = sis();
        if delta < 0 && (**s).res_units <= (-i32::from(delta)) as DWORD {
            (**s).res_units = 0;
        } else {
            (**s).res_units = (**s).res_units.wrapping_add(i32::from(delta) as DWORD);
        }
        assert!(mode == StatMsgMode_SMM_RES_UNITS);
    } else {
        let mut r: RECT = mem::zeroed();
        r.corner.x = 2;
        r.corner.y = 130;
        r.extent.width = STATUS_MESSAGE_WIDTH;
        r.extent.height = STATUS_MESSAGE_HEIGHT;
        SetContextForeGroundColor(rgb15(0, 8, 0));
        DrawFilledRectangle(&mut r);
    }
    DrawStatusMessage(ptr::null());
}

/// # Safety
/// SIS graphics, game state and asset resources must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn DeltaSISGauges(crew_delta: SIZE, fuel_delta: SIZE, resunit_delta: i32) {
    let (retained_status_context, preferences, mode) = Kernel::current().read(|owner| {
        (
            owner.status_context,
            owner.configuration.preferences(),
            owner.status_message_mode,
        )
    });
    if crew_delta == 0 && fuel_delta == 0 && resunit_delta == 0 {
        return;
    }
    let old = SetContext(retained_status_context);
    BatchGraphics();
    if crew_delta == UNDEFINED_DELTA_LOCAL {
        let mut stamp: STAMP = mem::zeroed();
        stamp.frame = Kernel::current().read(|owner| owner.flag_stat_frame);
        DrawStamp(&mut stamp);
        if preferences.fonts == OPT_PC {
            draw_pc_sis();
        }
        let s = sis().read(|state| *state);
        draw_slots(s.drive_slots.as_ptr(), NUM_DRIVE_SLOTS as usize, 0, 0);
        draw_slots(s.jet_slots.as_ptr(), NUM_JET_SLOTS as usize, 0, 1);
        draw_slots(s.module_slots.as_ptr(), NUM_MODULE_SLOTS as usize, 1, 2);
        draw_support_ships();
    }
    SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
    delta_crew(crew_delta);
    delta_fuel(fuel_delta);
    if crew_delta == UNDEFINED_DELTA_LOCAL {
        DrawFlagshipName(1);
        draw_captains_name();
        draw_landers();
        DrawStorageBays(0);
    }
    delta_res(resunit_delta as SIZE, mode);
    UnbatchGraphics();
    SetContext(old);
}

/// # Safety
/// Game state, fonts and graphics must be initialized.
#[no_mangle]
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

pub fn set_flash_rect(screen: &ContextLease, clip: RECT, rect: Option<RECT>) {
    screen.with_selected(|| {
        // SAFETY: selection checks the graphics thread before borrowing this existing
        // shared UI slot. Its retained context pins that thread until cancellation.
        let current = unsafe { &mut *ptr::addr_of_mut!(FLASH_CONTEXT) };
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
pub unsafe extern "C" fn SetFlashRect(p_rect: *const RECT) {
    let (retained_status_context, preferences) =
        Kernel::current().read(|owner| (owner.status_context, owner.configuration.preferences()));
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
            std::ptr::NonNull::new(Kernel::current().read(|owner| owner.screen_context))
                .unwrap_or_else(|| panic!("flash screen context not admitted")),
        );
        set_flash_rect(&screen, clip, Some(*rect));
    } else {
        *ptr::addr_of_mut!(FLASH_CONTEXT) = None;
    }
}
/// # Safety
/// Flash alarm and context state must be accessed on the game thread.
pub unsafe fn pre_update_flash_rect() {
    if let Some(current) = &mut *ptr::addr_of_mut!(FLASH_CONTEXT) {
        current.pre_update();
    }
}
/// # Safety
/// Flash alarm and context state must be accessed on the game thread.
pub unsafe fn post_update_flash_rect() {
    if let Some(current) = &mut *ptr::addr_of_mut!(FLASH_CONTEXT) {
        current.post_update();
    }
}
/// # Safety
/// Flash alarm system must be initialized on the game thread.
pub unsafe fn pause_flash() {
    if let Some(current) = &mut *ptr::addr_of_mut!(FLASH_CONTEXT) {
        current.pause();
    }
}
/// # Safety
/// Flash alarm system must be initialized on the game thread.
pub unsafe fn continue_flash() {
    if let Some(current) = &mut *ptr::addr_of_mut!(FLASH_CONTEXT) {
        current.resume();
    }
}

/// # Safety
/// pStr must be null or a valid NUL-terminated string; status graphics must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawStatusMessage(p_str: *const UNICODE) {
    let text = (!p_str.is_null()).then(|| std::ffi::CStr::from_ptr(p_str.cast()));
    Kernel::current().read(|kernel| {
        World::current().read(|world| draw_status_message(kernel, world, text));
    });
}

/// # Safety
/// Graphics, fonts and SIS strings must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawFlagshipName(in_status_area: BOOLEAN) {
    let (retained_status_context, preferences) =
        Kernel::current().read(|owner| (owner.status_context, owner.configuration.preferences()));
    let mut r: RECT = mem::zeroed();
    let mut buf = [0u8; 250];
    let old = if in_status_area != 0 {
        SetContext(retained_status_context)
    } else {
        SetContext(Kernel::current().read(|owner| owner.space_context))
    };
    let old_font = SetContextFont(if in_status_area != 0 {
        Kernel::current().read(|owner| owner.star_con_font)
    } else {
        Kernel::current().read(|owner| owner.micro_font)
    });
    r.corner.x = if in_status_area != 0 { 2 } else { 0 };
    r.corner.y = if in_status_area != 0 { 20 } else { 1 };
    r.extent.width = if in_status_area != 0 {
        SHIP_NAME_WIDTH
    } else {
        sis_screen_width() as SIZE
    };
    r.extent.height = SHIP_NAME_HEIGHT;
    let name = (**sis()).ship_name.as_mut_ptr();
    let p = if in_status_area != 0 {
        name
    } else {
        libc::snprintf(
            buf.as_mut_ptr().cast(),
            buf.len(),
            c"%s %s".as_ptr(),
            game_string(NAMING_STRING_BASE + 1),
            name,
        );
        let mut ch = buf.as_mut_ptr().cast::<libc::c_char>();
        while *ch != 0 {
            *ch = libc::toupper(*ch as libc::c_int) as libc::c_char;
            ch = ch.add(1);
        }
        buf.as_mut_ptr().cast()
    };
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let old_color = SetContextForeGroundColor(rgb15(0, 0, 0));
    DrawFilledRectangle(&mut r);
    let mut t: TEXT = mem::zeroed();
    t.baseline.x = (i32::from(r.corner.x) + (i32::from(r.extent.width) >> 1)) as SIZE;
    t.baseline.y =
        (r.corner.y as u32 + SHIP_NAME_HEIGHT as u32).wrapping_sub(in_status_area) as SIZE;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.CharCount = !0;
    t.pStr = p.cast();
    if preferences.fonts == OPT_PC {
        SetContextFontEffect(SetAbsFrameIndex(
            Kernel::current().read(|owner| owner.font_grad_frame),
            if in_status_area != 0 { 0 } else { 3 },
        ));
    } else {
        SetContextForeGroundColor(rgb15(0x14, 0x0a, 0));
    }
    font_DrawText(&mut t);
    SetContextFontEffect(old_effect);
    SetContextForeGroundColor(old_color);
    SetContextFont(old_font);
    SetContext(old);
}
