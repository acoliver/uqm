#![cfg(any(not(test), feature = "linked_c_archive"))]
#![allow(non_snake_case)]

use crate::callback::ffi::{AlarmHandle, Alarm_addAbsoluteMs, Alarm_remove};
use crate::navigation::abi::*;
use crate::navigation::globals::GlobData;
use std::{ffi::c_void, mem, ptr};

mod capacity;
use capacity::{GetCrewPodCapacity, GetFuelTankCapacity};

static mut CUR_MSG_MODE: StatMsgMode = StatMsgMode_SMM_DEFAULT;
static mut LAST_PILOT: BOOLEAN = 0;
static mut NEXT_PILOT_TIME: TimeCount = 0;
static mut PILOT_CYCLE_INDEX: DWORD = 0;
static mut FLASH_CONTEXT: *mut FlashContext = ptr::null_mut();
static mut FLASH_RECT: RECT = unsafe { mem::zeroed() };
static mut FLASH_ALARM: *mut AlarmHandle = ptr::null_mut();
static mut FLASH_PAUSED: BOOLEAN = 0;
#[no_mangle]
pub static mut updateFlashRectRecursion: COUNT = 0;

const UNDEFINED_DELTA_LOCAL: SIZE = 0x7fff;
const SFR_MENU_3DO: usize = usize::MAX;
const SFR_MENU_ANY: usize = usize::MAX - 1;
const BATTLE_FRAME_RATE: u32 = ONE_SECOND / 24;
const EMPTY_SLOT: BYTE = NUM_MODULES as BYTE;
const OPT_PC: i32 = crate::navigation::abi::OPT_PC as i32;
const SIS_MESSAGE_HEIGHT: SIZE = crate::navigation::abi::SIS_MESSAGE_HEIGHT as SIZE;
const STORAGE_BAY_CAPACITY: COUNT = crate::navigation::abi::STORAGE_BAY_CAPACITY as COUNT;

unsafe fn game_string(index: u32) -> *mut UNICODE {
    GetStringAddress(SetAbsStringTableIndex(GameStrings, index as u16)).cast()
}

unsafe fn game_state(name: &str) -> u8 {
    let (start, end) =
        crate::state::game_state::lookup_game_state_bits(name).expect("known SIS game-state key");
    crate::state::ffi::rust_get_game_state_bits(start as i32, end as i32)
}

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
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
fn crew_color(i: usize) -> Color {
    const RGB: [(u8, u8, u8); 10] = [
        (10, 30, 9),
        (0, 30, 0),
        (0, 27, 0),
        (0, 24, 0),
        (0, 21, 0),
        (0, 18, 0),
        (0, 16, 0),
        (0, 13, 0),
        (0, 10, 0),
        (0, 7, 0),
    ];
    let (r, g, b) = RGB[i];
    rgb15(r, g, b)
}
fn storage_color(i: usize) -> Color {
    const V: [u8; 10] = [31, 28, 24, 21, 18, 16, 13, 10, 8, 5];
    rgb15(V[i], V[i], V[i])
}
fn fuel_color(i: usize) -> Color {
    const RGB: [(u8, u8, u8); 11] = [
        (15, 0, 0),
        (19, 0, 0),
        (23, 0, 0),
        (27, 0, 0),
        (31, 3, 0),
        (31, 7, 0),
        (31, 10, 0),
        (31, 14, 0),
        (31, 17, 0),
        (31, 21, 0),
        (31, 24, 0),
    ];
    let (r, g, b) = RGB[i];
    rgb15(r, g, b)
}
fn pilot_color(i: usize) -> Color {
    const RGB: [(u8, u8, u8); 7] = [
        (10, 20, 24),
        (6, 16, 22),
        (3, 14, 20),
        (2, 12, 17),
        (1, 11, 15),
        (1, 9, 13),
        (0, 7, 11),
    ];
    let (r, g, b) = RGB[i];
    rgb15(r, g, b)
}
#[inline]
unsafe fn sis() -> *mut SIS_STATE {
    ptr::addr_of_mut!(GlobData.SIS_state)
}
#[inline]
unsafe fn set_color(color: Color) {
    SetContextForeGroundColor(color);
}

/// # Safety
/// The graphics system and screen contexts must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn RepairSISBorder() {
    let old = SetContext(ScreenContext);
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
    let old = SetContext(StatusContext);
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
pub unsafe extern "C" fn DrawSISTitle(pStr: *mut UNICODE) {
    let mut t: TEXT = mem::zeroed();
    t.baseline.x = SIS_TITLE_WIDTH >> 1;
    t.baseline.y = SIS_TITLE_HEIGHT - 2;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.pStr = pStr;
    t.CharCount = !0;
    let old = SetContext(OffScreenContext);
    let mut r: RECT = mem::zeroed();
    r.corner.x =
        (i32::from(SIS_ORG_X) + sis_screen_width() - i32::from(SIS_TITLE_BOX_WIDTH) + 1) as SIZE;
    r.corner.y = SIS_ORG_Y - SIS_TITLE_HEIGHT;
    r.extent.width = SIS_TITLE_WIDTH;
    r.extent.height = SIS_TITLE_HEIGHT - 1;
    SetContextFGFrame(Screen);
    SetContextClipRect(&mut r);
    SetContextFont(TinyFont);
    BatchGraphics();
    SetContextBackGroundColor(rgb15(0, 0, 0x14));
    ClearDrawable();
    SetContextForeGroundColor(rgb15(0x1b, 0, 0x1b));
    font_DrawText(&mut t);
    UnbatchGraphics();
    SetContextClipRect(ptr::null_mut());
    SetContext(old);
}

/// # Safety
/// Graphics state must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn DrawHyperCoords(universe: POINT) {
    let mut b = [0u8; 100];
    libc::snprintf(
        b.as_mut_ptr().cast(),
        b.len(),
        c"%03u.%01u : %03u.%01u".as_ptr(),
        i32::from(universe.x) / 10,
        i32::from(universe.x) % 10,
        i32::from(universe.y) / 10,
        i32::from(universe.y) % 10,
    );
    DrawSISTitle(b.as_mut_ptr().cast());
}

/// # Safety
/// pStr must be null or a valid NUL-terminated string; UI graphics state must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawSISMessage(pStr: *const UNICODE) {
    DrawSISMessageEx(pStr, -1, -1, DSME_NONE as COUNT);
}

/// # Safety
/// pStr must be null or valid UTF-8 and graphics state must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn DrawSISMessageEx(
    pStr: *const UNICODE,
    cur_pos: SIZE,
    ex_pos: SIZE,
    flags: COUNT,
) -> BOOLEAN {
    let old = SetContext(OffScreenContext);
    let mut r: RECT = mem::zeroed();
    r.corner.x = SIS_ORG_X + 1;
    r.corner.y = SIS_ORG_Y - SIS_MESSAGE_HEIGHT;
    r.extent.width = sis_message_width() as SIZE;
    r.extent.height = SIS_MESSAGE_HEIGHT - 1;
    SetContextFGFrame(Screen);
    SetContextClipRect(&mut r);
    BatchGraphics();
    SetContextBackGroundColor(rgb15(0, 0, 0x14));
    let mut buf = [0u8; 256];
    let mut text = pStr;
    if text.is_null() {
        text = match u32::from(GlobData.Game_state.CurrentActivity as BYTE) {
            IN_LAST_BATTLE | IN_INTERPLANETARY => {
                GetClusterName(
                    crate::planets::starmap::CurStarDescPtr.cast(),
                    buf.as_mut_ptr().cast(),
                );
                buf.as_ptr().cast()
            }
            IN_HYPERSPACE => game_string(if inHyperSpace() != 0 {
                NAVIGATION_STRING_BASE
            } else {
                NAVIGATION_STRING_BASE + 1
            }),
            _ => c"".as_ptr().cast(),
        };
    }
    if flags & DSME_MYCOLOR as COUNT == 0 {
        SetContextForeGroundColor(rgb15(0x1b, 0, 0x1b));
    }
    let mut t: TEXT = mem::zeroed();
    t.baseline.y = SIS_MESSAGE_HEIGHT - 2;
    t.pStr = text as *mut UNICODE;
    t.CharCount = !0;
    SetContextFont(TinyFont);
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
            t.pStr = skipUTF8Chars(t.pStr, ex_pos as usize);
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
    buf: *mut i8,
    buf_len: usize,
    month_index: BYTE,
    day_index: BYTE,
    year_index: COUNT,
) {
    libc::snprintf(
        buf,
        buf_len,
        c"%s %02d·%04d".as_ptr(),
        game_string(MONTHS_STRING_BASE + month_index as u32 - 1),
        day_index as i32,
        year_index as i32,
    );
}

/// # Safety
/// r must point to writable RECT storage.
#[no_mangle]
pub unsafe extern "C" fn GetStatusMessageRect(r: *mut RECT) {
    (*r).corner.x = 2;
    (*r).corner.y = 130;
    (*r).extent.width = STATUS_MESSAGE_WIDTH;
    (*r).extent.height = STATUS_MESSAGE_HEIGHT;
}

/// # Safety
/// pStr must be null or a valid NUL-terminated string; status graphics must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawStatusMessage(pStr: *const UNICODE) {
    let old = SetContext(StatusContext);
    let mut ctx: RECT = mem::zeroed();
    GetContextClipRect(&mut ctx);
    SetContext(OffScreenContext);
    SetContextFGFrame(Screen);
    let mut r: RECT = mem::zeroed();
    GetStatusMessageRect(&mut r);
    r.corner.x = (i32::from(r.corner.x) + i32::from(ctx.corner.x)) as SIZE;
    r.corner.y = (i32::from(r.corner.y) + i32::from(ctx.corner.y)) as SIZE;
    SetContextClipRect(&mut r);
    BatchGraphics();
    SetContextBackGroundColor(rgb15(0, 8, 0));
    ClearDrawable();
    let mut buf = [0u8; 128];
    let mut text = pStr;
    if text.is_null() {
        if CUR_MSG_MODE == StatMsgMode_SMM_CREDITS {
            let credits = u16::from(game_state("MELNORME_CREDIT0"))
                | (u16::from(game_state("MELNORME_CREDIT1")) << 8);
            libc::snprintf(
                buf.as_mut_ptr().cast(),
                buf.len(),
                c"%u %s".as_ptr(),
                credits as u32,
                game_string(STATUS_STRING_BASE),
            );
        } else if CUR_MSG_MODE == StatMsgMode_SMM_RES_UNITS {
            if game_state("CHMMR_BOMB_STATE") < 2 {
                libc::snprintf(
                    buf.as_mut_ptr().cast(),
                    buf.len(),
                    c"%u %s".as_ptr(),
                    (*sis()).ResUnits,
                    game_string(STATUS_STRING_BASE + 1),
                );
            } else {
                libc::snprintf(
                    buf.as_mut_ptr().cast(),
                    buf.len(),
                    c"%s %s".as_ptr(),
                    if optWhichMenu == OPT_PC {
                        game_string(STATUS_STRING_BASE + 2)
                    } else {
                        c"∞".as_ptr().cast()
                    },
                    game_string(STATUS_STRING_BASE + 1),
                );
            }
        } else {
            DateToString(
                buf.as_mut_ptr().cast(),
                buf.len(),
                GlobData.Game_state.GameClock.month_index,
                GlobData.Game_state.GameClock.day_index,
                GlobData.Game_state.GameClock.year_index,
            );
        }
        text = buf.as_ptr().cast();
    }
    let mut t: TEXT = mem::zeroed();
    t.baseline.x = STATUS_MESSAGE_WIDTH >> 1;
    t.baseline.y = STATUS_MESSAGE_HEIGHT - 1;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.pStr = text as *mut UNICODE;
    t.CharCount = !0;
    SetContextFont(TinyFont);
    SetContextForeGroundColor(rgb15(0, 0x10, 0));
    font_DrawText(&mut t);
    UnbatchGraphics();
    SetContextClipRect(ptr::null_mut());
    SetContext(old);
}

/// # Safety
/// Called on the game thread.
#[no_mangle]
pub unsafe extern "C" fn SetStatusMessageMode(new_mode: StatMsgMode) -> StatMsgMode {
    let old = CUR_MSG_MODE;
    CUR_MSG_MODE = new_mode;
    old
}

/// # Safety
/// Status graphics and SIS state must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawCaptainsName() {
    let old = SetContext(StatusContext);
    let old_font = SetContextFont(TinyFont);
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
    t.pStr = (*sis()).CommanderName.as_mut_ptr();
    t.CharCount = !0;
    SetContextForeGroundColor(rgb15(0x16, 0x0b, 0x1f));
    font_DrawText(&mut t);
    SetContextForeGroundColor(old_color);
    SetContextFont(old_font);
    SetContext(old);
}

/// # Safety
/// Graphics, fonts and SIS strings must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawFlagshipName(in_status_area: BOOLEAN) {
    let mut r: RECT = mem::zeroed();
    let mut buf = [0u8; 250];
    let old = if in_status_area != 0 {
        SetContext(StatusContext)
    } else {
        SetContext(SpaceContext)
    };
    let old_font = SetContextFont(if in_status_area != 0 {
        StarConFont
    } else {
        MicroFont
    });
    r.corner.x = if in_status_area != 0 { 2 } else { 0 };
    r.corner.y = if in_status_area != 0 { 20 } else { 1 };
    r.extent.width = if in_status_area != 0 {
        SHIP_NAME_WIDTH
    } else {
        sis_screen_width() as SIZE
    };
    r.extent.height = SHIP_NAME_HEIGHT;
    let name = (*sis()).ShipName.as_mut_ptr();
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
    t.pStr = p;
    if optWhichFonts == OPT_PC {
        SetContextFontEffect(SetAbsFrameIndex(
            FontGradFrame,
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

/// # Safety
/// Graphics, fonts, game strings and SIS inventory must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawFlagshipStats() {
    let s = sis();
    let mut energy_regen: u8 = 1;
    let mut energy_wait: u8 = 10;
    let mut thrust: u16 = 10;
    let mut turn_wait: u8 = 17;
    let mut fuel: u32 = 10 * FUEL_TANK_SCALE;
    for &m in &(*s).ModuleSlots {
        match u32::from(m) {
            FUEL_TANK => fuel += FUEL_TANK_CAPACITY,
            HIGHEFF_FUELSYS => fuel += HEFUEL_TANK_CAPACITY,
            DYNAMO_UNIT => energy_wait = ((i32::from(energy_wait) - 2) as BYTE).max(4),
            SHIVA_FURNACE => energy_regen += 1,
            _ => {}
        }
    }
    for &d in &(*s).DriveSlots {
        if u32::from(d) == FUSION_THRUSTER {
            thrust += 2;
        }
    }
    for &j in &(*s).JetSlots {
        if u32::from(j) == TURNING_JETS {
            turn_wait -= 2;
        }
    }
    let old = SetContext(SpaceContext);
    let old_font = SetContextFont(StarConFont);
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
    SetContextFontEffect(SetAbsFrameIndex(FontGradFrame, 4));
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
        weapon_name((*s).ModuleSlots[*slot], b.as_mut_ptr());
        font_DrawText(&mut t);
        if n != 3 {
            t.baseline.y = (i32::from(t.baseline.y) + i32::from(leading)) as SIZE;
        }
    }
    t.baseline.x = (i32::from(r.extent.width) - 25) as SIZE;
    t.baseline.y = (i32::from(r.corner.y) + i32::from(leading) + 3) as SIZE;
    t.align = TEXT_ALIGN_ALIGN_RIGHT;
    SetContextFontEffect(SetAbsFrameIndex(FontGradFrame, 5));
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
#[no_mangle]
pub unsafe extern "C" fn DrawLanders() {
    let old = SetContext(StatusContext);
    let mut stamp: STAMP = mem::zeroed();
    stamp.frame = IncFrameIndex(FlagStatFrame);
    let mut r: RECT = mem::zeroed();
    GetFrameRect(stamp.frame, &mut r);
    let count = (*sis()).NumLanders;
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
    let old = SetContext(StatusContext);
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
        let mut j = (*sis()).TotalElementMass;
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
    SetContextFont(TinyFont);
    SetContextForeGroundColor(rgb15(0, 0, 0));
    r.corner.y -= 6;
    r.corner.x -= 1;
    r.extent.width += 2;
    DrawFilledRectangle(&mut r);
    SetContextFontEffect(SetAbsFrameIndex(FontGradFrame, 1));
    t.pStr = game_string(STATUS_STRING_BASE + 3) as *mut UNICODE;
    font_DrawText(&mut t);
    r.corner.y += 79;
    t.baseline.y += 79;
    DrawFilledRectangle(&mut r);
    SetContextFontEffect(SetAbsFrameIndex(FontGradFrame, 2));
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
            stamp.frame = SetAbsFrameIndex(FlagStatFrame, (u32::from(piece) + 1 + mode) as COUNT);
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
    let q = ptr::addr_of_mut!(GlobData.Game_state.built_ship_q);
    let mut h = (*q).head;
    let mut i = 0;
    while !h.is_null() {
        let p = h.cast::<SHIP_FRAGMENT>();
        let next = (*p).succ;
        let mut stamp: STAMP = mem::zeroed();
        stamp.origin.x = points[i].0;
        stamp.origin.y = points[i].1;
        stamp.frame = (*p).icons;
        DrawStamp(&mut stamp);
        h = next;
        i += 1;
    }
}

unsafe fn delta_crew(delta: SIZE) {
    if delta == 0 {
        return;
    }
    if delta != UNDEFINED_DELTA_LOCAL {
        let st = sis();
        if delta < 0 && (*st).CrewEnlisted <= (-i32::from(delta)) as COUNT {
            (*st).CrewEnlisted = 0;
        } else {
            (*st).CrewEnlisted = ((*st).CrewEnlisted as i32 + delta as i32) as COUNT;
            let cap = GetCrewPodCapacity();
            if (*st).CrewEnlisted > cap {
                (*st).CrewEnlisted = cap;
            }
        }
    }
    let mut b = [0u8; 60];
    libc::snprintf(
        b.as_mut_ptr().cast(),
        b.len(),
        c"%u".as_ptr(),
        i32::from((*sis()).CrewEnlisted),
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
    let st = sis();
    let old = if delta == UNDEFINED_DELTA_LOCAL {
        COUNT::MAX
    } else {
        ((*st).FuelOnBoard / FUEL_TANK_SCALE) as COUNT
    };
    if delta != UNDEFINED_DELTA_LOCAL {
        if delta < 0 && (*st).FuelOnBoard <= (-i32::from(delta)) as DWORD {
            (*st).FuelOnBoard = 0;
        } else {
            let cap = GetFuelTankCapacity();
            (*st).FuelOnBoard = (*st).FuelOnBoard.wrapping_add(i32::from(delta) as DWORD);
            if (*st).FuelOnBoard > cap {
                (*st).FuelOnBoard = cap;
            }
        }
    }
    let new = ((*st).FuelOnBoard / FUEL_TANK_SCALE) as COUNT;
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
unsafe fn delta_res(delta: SIZE) {
    if delta == 0 {
        return;
    }
    if delta != UNDEFINED_DELTA_LOCAL {
        let s = sis();
        if delta < 0 && (*s).ResUnits <= (-i32::from(delta)) as DWORD {
            (*s).ResUnits = 0;
        } else {
            (*s).ResUnits = (*s).ResUnits.wrapping_add(i32::from(delta) as DWORD);
        }
        assert!(CUR_MSG_MODE == StatMsgMode_SMM_RES_UNITS);
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
    if crew_delta == 0 && fuel_delta == 0 && resunit_delta == 0 {
        return;
    }
    let old = SetContext(StatusContext);
    BatchGraphics();
    if crew_delta == UNDEFINED_DELTA_LOCAL {
        let mut stamp: STAMP = mem::zeroed();
        stamp.frame = FlagStatFrame;
        DrawStamp(&mut stamp);
        if optWhichFonts == OPT_PC {
            draw_pc_sis();
        }
        let s = sis();
        draw_slots((*s).DriveSlots.as_ptr(), NUM_DRIVE_SLOTS as usize, 0, 0);
        draw_slots((*s).JetSlots.as_ptr(), NUM_JET_SLOTS as usize, 0, 1);
        draw_slots((*s).ModuleSlots.as_ptr(), NUM_MODULE_SLOTS as usize, 1, 2);
        draw_support_ships();
    }
    SetContextFont(TinyFont);
    delta_crew(crew_delta);
    delta_fuel(fuel_delta);
    if crew_delta == UNDEFINED_DELTA_LOCAL {
        DrawFlagshipName(1);
        DrawCaptainsName();
        DrawLanders();
        DrawStorageBays(0);
    }
    delta_res(resunit_delta as SIZE);
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
    let autopilot = GlobData.Game_state.autopilot;
    let on = (autopilot.x != -1 && autopilot.y != -1) || (*sis()).FuelOnBoard == 0;
    if on || LAST_PILOT != 0 {
        if !on {
            DrawSISMessage(ptr::null());
            PILOT_CYCLE_INDEX = 0;
        } else if GetTimeCounter() >= NEXT_PILOT_TIME {
            if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 == 0
                && (*sis()).CrewEnlisted != COUNT::MAX
            {
                let old = SetContext(OffScreenContext);
                SetContextForeGroundColor(pilot_color(PILOT_CYCLE_INDEX as usize));
                DrawSISMessageEx(
                    game_string(
                        NAVIGATION_STRING_BASE + if (*sis()).FuelOnBoard == 0 { 2 } else { 3 },
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

extern "C" fn update_flash_rect(_arg: *mut c_void) {
    unsafe {
        if FLASH_CONTEXT.is_null() {
            return;
        }
        Flash_process(FLASH_CONTEXT);
        schedule_flash_alarm();
    }
}
unsafe fn schedule_flash_alarm() {
    let next = Flash_nextTime(FLASH_CONTEXT);
    let ms = (next / ONE_SECOND)
        .wrapping_mul(1000)
        .wrapping_add((next % ONE_SECOND) * 1000 / ONE_SECOND);
    FLASH_ALARM = Alarm_addAbsoluteMs(ms, update_flash_rect, ptr::null_mut());
}
/// # Safety
/// pRect must be null, a valid RECT, or one of the SIS menu sentinel pointers; flash and graphics systems must be initialized.
#[no_mangle]
pub unsafe extern "C" fn SetFlashRect(p_rect: *const RECT) {
    let mut clip: RECT = mem::zeroed();
    let mut temp: RECT = mem::zeroed();
    let mut rect = p_rect;
    if rect as usize != SFR_MENU_3DO && rect as usize != SFR_MENU_ANY {
        GetContextClipRect(&mut clip);
    } else if optWhichMenu == OPT_PC && rect as usize != SFR_MENU_ANY {
        rect = ptr::null();
    } else {
        let old = SetContext(StatusContext);
        GetContextClipRect(&mut clip);
        temp.corner.x = (radar_x() - i32::from(clip.corner.x)) as SIZE;
        temp.corner.y = (radar_y() - i32::from(clip.corner.y)) as SIZE;
        temp.extent.width = RADAR_WIDTH;
        temp.extent.height = RADAR_HEIGHT;
        SetContext(old);
        rect = &temp;
    }
    if !rect.is_null() && (*rect).extent.width != 0 {
        FLASH_RECT = *rect;
        FLASH_RECT.corner.x = (i32::from(FLASH_RECT.corner.x) + i32::from(clip.corner.x)) as SIZE;
        FLASH_RECT.corner.y = (i32::from(FLASH_RECT.corner.y) + i32::from(clip.corner.y)) as SIZE;
        if FLASH_CONTEXT.is_null() {
            FLASH_CONTEXT = Flash_createHighlight(ScreenContext, ptr::addr_of!(FLASH_RECT));
            Flash_setMergeFactors(FLASH_CONTEXT, 3, 2, 2);
            Flash_setSpeed(FLASH_CONTEXT, 0, ONE_SECOND / 16, 0, ONE_SECOND / 16);
            Flash_setFrameTime(FLASH_CONTEXT, ONE_SECOND / 16);
            Flash_start(FLASH_CONTEXT);
            schedule_flash_alarm();
        } else {
            Flash_setRect(FLASH_CONTEXT, ptr::addr_of!(FLASH_RECT));
        }
    } else if !FLASH_CONTEXT.is_null() {
        Alarm_remove(FLASH_ALARM);
        FLASH_ALARM = ptr::null_mut();
        Flash_terminate(FLASH_CONTEXT);
        FLASH_CONTEXT = ptr::null_mut();
    }
}
/// # Safety
/// Flash alarm and context state must be accessed on the game thread.
#[no_mangle]
pub unsafe extern "C" fn PreUpdateFlashRect() {
    if !FLASH_ALARM.is_null() {
        updateFlashRectRecursion = updateFlashRectRecursion.wrapping_add(1);
        if updateFlashRectRecursion > 1 {
            return;
        }
        Flash_preUpdate(FLASH_CONTEXT);
    }
}
/// # Safety
/// Flash alarm and context state must be accessed on the game thread.
#[no_mangle]
pub unsafe extern "C" fn PostUpdateFlashRect() {
    if !FLASH_ALARM.is_null() {
        updateFlashRectRecursion = updateFlashRectRecursion.wrapping_sub(1);
        if updateFlashRectRecursion > 0 {
            return;
        }
        Flash_postUpdate(FLASH_CONTEXT);
    }
}
/// # Safety
/// Flash alarm system must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn PauseFlash() {
    if !FLASH_CONTEXT.is_null() {
        Alarm_remove(FLASH_ALARM);
        FLASH_ALARM = ptr::null_mut();
        FLASH_PAUSED = 1;
    }
}
/// # Safety
/// Flash alarm system must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn ContinueFlash() {
    if FLASH_PAUSED != 0 {
        schedule_flash_alarm();
        FLASH_PAUSED = 0;
    }
}
