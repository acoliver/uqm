#![cfg(not(test))]
#![allow(non_snake_case)]

pub use crate::battle::context_access::reader_legacy::InitShipStatus;
use crate::engine::battle::record::Kernel;
use crate::engine::battle::{link_queue, ShipId};
use crate::navigation::abi::*;
use std::mem;

const STATUS_WIDTH: COORD = 64;
const SHIP_INFO_HEIGHT: COORD = 65;
const GAUGE_YOFFS: COORD = SHIP_INFO_HEIGHT - 10;
const STARCON_TEXT_HEIGHT: COORD = 7;
const CREW_XOFFS: COORD = 4;
const ENERGY_XOFFS: COORD = 52;
const STAT_WIDTH: COORD = 7;
const UNIT_WIDTH: COORD = 2;
const UNIT_HEIGHT: COORD = 1;
const MAX_CREW_SIZE: COUNT = 42;
const CREW_IMMUNE: u16 = 1 << 6;
const PLAYER_CAPTAIN: u16 = 1 << 13;
const OPT_PC: i32 = 0x02;
const RPG_PLAYER_NUM: COORD = 0;

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

/// # Safety
/// Status graphics, frames and the active context must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawCrewFuelString(y: COORD, state: SIZE) {
    let mut stamp: STAMP = mem::zeroed();
    stamp.origin.y = y + GAUGE_YOFFS + STARCON_TEXT_HEIGHT;
    if state == 0 {
        stamp.origin.x = CREW_XOFFS + (STAT_WIDTH >> 1) + 6;
        stamp.frame = SetAbsFrameIndex(StatusFrame, if optWhichMenu == OPT_PC { 4 } else { 0 });
        DrawStamp(&mut stamp);
    }
    stamp.origin.x = ENERGY_XOFFS + (STAT_WIDTH >> 1) - 5;
    stamp.frame = SetAbsFrameIndex(StatusFrame, if optWhichMenu == OPT_PC { 5 } else { 1 });
    if state >= 0 {
        DrawStamp(&mut stamp);
    } else {
        SetContextForeGroundColor(rgb15(0x1f, 0x1f, 0x0a));
        DrawFilledStamp(&mut stamp);
    }
}

unsafe fn draw_ship_name_string(p_str: *mut UNICODE, char_count: COUNT, y: COORD) {
    let old_font = SetContextFont(StarConFont);
    let mut text: TEXT = mem::zeroed();
    text.pStr = p_str;
    text.CharCount = char_count;
    text.align = TEXT_ALIGN_ALIGN_CENTER;
    text.baseline.y = STARCON_TEXT_HEIGHT + 3 + y;
    text.baseline.x = STATUS_WIDTH >> 1;
    SetContextForeGroundColor(rgb15(0x10, 0x10, 0x10));
    font_DrawText(&mut text);
    text.baseline.y -= 1;
    SetContextForeGroundColor(rgb15(0, 0, 0));
    font_DrawText(&mut text);
    SetContextFont(old_font);
}

/// # Safety
/// The status context must be active and graphics initialized.
#[no_mangle]
pub unsafe extern "C" fn ClearShipStatus(y: COORD) {
    SetContextForeGroundColor(rgb15(0x0a, 0x0a, 0x0a));
    let mut r: RECT = mem::zeroed();
    r.corner.x = 2;
    r.corner.y = 3 + y;
    r.extent.width = STATUS_WIDTH - 4;
    r.extent.height = SHIP_INFO_HEIGHT - 3;
    DrawFilledRectangle(&mut r);
}

/// # Safety
/// The status context must be active and graphics initialized.
#[no_mangle]
pub unsafe extern "C" fn OutlineShipStatus(y: COORD) {
    let mut r: RECT = mem::zeroed();
    SetContextForeGroundColor(rgb15(0x08, 0x08, 0x08));
    r.corner.x = 0;
    r.corner.y = 1 + y;
    r.extent.width = STATUS_WIDTH;
    r.extent.height = 1;
    DrawFilledRectangle(&mut r);
    r.corner.y += 1;
    r.extent.width -= 1;
    DrawFilledRectangle(&mut r);
    r.extent.width = 1;
    r.extent.height = SHIP_INFO_HEIGHT - 2;
    DrawFilledRectangle(&mut r);
    r.corner.x += 1;
    DrawFilledRectangle(&mut r);
    SetContextForeGroundColor(rgb15(0x10, 0x10, 0x10));
    r.corner.x = STATUS_WIDTH - 1;
    DrawFilledRectangle(&mut r);
    r.corner.x = STATUS_WIDTH - 2;
    r.corner.y += 1;
    r.extent.height -= 1;
    DrawFilledRectangle(&mut r);
    SetContextForeGroundColor(rgb15(0, 0, 0));
    r.corner.x = 0;
    r.corner.y = y;
    r.extent.width = STATUS_WIDTH;
    r.extent.height = 1;
    DrawFilledRectangle(&mut r);
}

/// # Safety
/// SIPtr must point to writable ship info. Optional pointers must be null or point to valid game objects.
pub(crate) unsafe fn init_ship_status(
    siptr: *mut SHIP_INFO,
    starship: Option<ShipId>,
    clip_rect: *mut RECT,
) {
    let starship = starship.map(|id| {
        let ships = link_queue::ships(id.side as usize);
        ships.get(ships.link(id))
    });
    let mut y: COORD = 0;
    let mut stamp: STAMP = mem::zeroed();
    let mut old_clip: RECT = mem::zeroed();
    let mut old_origin = POINT { x: 0, y: 0 };
    if let Some(ship) = starship {
        assert!(ship.player >= 0);
        y = Kernel::current().status_offsets[ship.player as usize];
    }
    let old_context = SetContext(StatusContext);
    if !clip_rect.is_null() {
        GetContextClipRect(&mut old_clip);
        let mut r = old_clip;
        r.corner.x += (*clip_rect).corner.x;
        r.corner.y += (*clip_rect).corner.y & !1;
        r.extent = (*clip_rect).extent;
        r.extent.height += (*clip_rect).corner.y & 1;
        SetContextClipRect(&mut r);
        old_origin = SetContextOrigin(POINT {
            x: -(*clip_rect).corner.x,
            y: -((*clip_rect).corner.y & !1),
        });
    }
    BatchGraphics();
    OutlineShipStatus(y);
    ClearShipStatus(y);
    stamp.origin.x = STATUS_WIDTH >> 1;
    stamp.origin.y = 31 + y;
    stamp.frame = IncFrameIndex((*siptr).icons);
    DrawStamp(&mut stamp);
    gauge_frames::draw_gauge_frames(&*siptr, y);

    draw_status_title(siptr, starship, y);
    let crew_delta = (*siptr).crew_level as SIZE;
    let energy_delta = (*siptr).energy_level as SIZE;
    (*siptr).crew_level = 0;
    (*siptr).energy_level = 0;
    DeltaStatistics(siptr, y, crew_delta, energy_delta);
    UnbatchGraphics();
    if !clip_rect.is_null() {
        SetContextOrigin(old_origin);
        SetContextClipRect(&mut old_clip);
    }
    SetContext(old_context);
}

unsafe fn draw_status_title(
    siptr: *mut SHIP_INFO,
    starship: Option<crate::engine::battle::link_queue::PickShip>,
    y: COORD,
) {
    if starship.is_none_or(|ship| ship.captain != 0) {
        DrawCrewFuelString(y, 0);
        let loc = SetAbsStringTableIndex((*siptr).race_strings, 1);
        draw_ship_name_string(GetStringAddress(loc).cast(), GetStringLength(loc), y);
        let old_font = SetContextFont(TinyFont);
        let mut buf = [0u8; 30];
        let (name, count) = if let Some(ship) = starship {
            let name = SetAbsStringTableIndex((*siptr).race_strings, ship.captain as COUNT);
            (GetStringAddress(name).cast(), GetStringLength(name))
        } else {
            libc::sprintf(
                buf.as_mut_ptr().cast(),
                c"%d".as_ptr(),
                (*siptr).ship_cost as libc::c_int,
            );
            (buf.as_mut_ptr().cast(), !0)
        };
        let mut text: TEXT = mem::zeroed();
        text.pStr = name;
        text.CharCount = count;
        text.align = TEXT_ALIGN_ALIGN_CENTER;
        text.baseline.x = STATUS_WIDTH >> 1;
        text.baseline.y = y + GAUGE_YOFFS + 3;
        SetContextForeGroundColor(rgb15(0, 0, 0));
        font_DrawText(&mut text);
        SetContextFont(old_font);
    } else if starship
        .unwrap_or_else(|| panic!("missing checked battle value"))
        .player
        == RPG_PLAYER_NUM
    {
        DrawCrewFuelString(y, 0);
        draw_ship_name_string(
            std::ptr::addr_of_mut!(crate::navigation::globals::GlobData.SIS_state.ShipName).cast(),
            !0,
            y,
        );
    }
}

/// # Safety
/// ShipInfoPtr must be writable and point to valid ship information; graphics context must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DeltaStatistics(
    ship: *mut SHIP_INFO,
    y_offs: COORD,
    crew_delta: SIZE,
    mut energy_delta: SIZE,
) {
    if crew_delta == 0 && energy_delta == 0 {
        return;
    }
    let x: COORD = 0;
    let y = GAUGE_YOFFS + y_offs;
    let mut r: RECT = mem::zeroed();
    r.extent.width = UNIT_WIDTH;
    r.extent.height = UNIT_HEIGHT;
    if crew_delta != 0 {
        let old_blocks = std::cmp::min((*ship).crew_level, MAX_CREW_SIZE as COUNT) as COORD;
        let new_level = ((*ship).crew_level as i32 + crew_delta as i32) as COUNT;
        let new_blocks = std::cmp::min(new_level, MAX_CREW_SIZE as COUNT) as COORD;
        if crew_delta > 0 {
            r.corner.y = (y + 1) - (((old_blocks + 1) >> 1) * (UNIT_HEIGHT + 1));
            SetContextForeGroundColor(if (*ship).ship_flags & CREW_IMMUNE != 0 {
                rgb15(0x0a, 0x0a, 0x0a)
            } else {
                rgb15(0, 0x14, 0)
            });
            for i in old_blocks..new_blocks {
                r.corner.x = x + CREW_XOFFS + 1;
                if i & 1 == 0 {
                    r.corner.x += UNIT_WIDTH + 1;
                    r.corner.y -= UNIT_HEIGHT + 1;
                }
                DrawFilledRectangle(&mut r);
            }
        } else {
            SetContextForeGroundColor(rgb15(0, 0, 0));
            r.corner.y = (y + 1) - (((old_blocks + 2) >> 1) * (UNIT_HEIGHT + 1));
            let mut i = old_blocks;
            while i > new_blocks {
                r.corner.x = x + CREW_XOFFS + 1 + UNIT_WIDTH + 1;
                if i & 1 == 0 {
                    r.corner.x -= UNIT_WIDTH + 1;
                    r.corner.y += UNIT_HEIGHT + 1;
                }
                DrawFilledRectangle(&mut r);
                i -= 1;
            }
        }
        if (*ship).ship_flags & PLAYER_CAPTAIN != 0
            && ((((*ship).crew_level > MAX_CREW_SIZE as COUNT)
                != (new_level > MAX_CREW_SIZE as COUNT))
                || (*ship).crew_level == 0)
            && new_level != 0
        {
            SetContextForeGroundColor(if new_level > MAX_CREW_SIZE as COUNT {
                rgb15(0, 0x14, 0)
            } else {
                rgb15(0x0a, 0x0a, 0x1f)
            });
            r.corner.x = x + CREW_XOFFS + 1 + UNIT_WIDTH + 1;
            r.corner.y = y - UNIT_HEIGHT;
            DrawFilledRectangle(&mut r);
        }
        (*ship).crew_level = new_level;
        if (*ship).max_crew > MAX_CREW_SIZE as COUNT || (*ship).ship_flags & PLAYER_CAPTAIN != 0 {
            DrawBattleCrewAmount(ship, y_offs);
        }
    }
    if energy_delta != 0 {
        if energy_delta > 0 {
            SetContextForeGroundColor(rgb15(0x14, 0, 0));
            r.corner.y = (y + 1) - (((*ship).energy_level as COORD + 1) >> 1) * (UNIT_HEIGHT + 1);
            while energy_delta > 0 {
                r.corner.x = x + ENERGY_XOFFS + 1;
                if (*ship).energy_level & 1 == 0 {
                    r.corner.x += UNIT_WIDTH + 1;
                    r.corner.y -= UNIT_HEIGHT + 1;
                }
                DrawFilledRectangle(&mut r);
                (*ship).energy_level += 1;
                energy_delta -= 1;
            }
        } else {
            SetContextForeGroundColor(rgb15(0, 0, 0));
            r.corner.y = (y + 1) - (((*ship).energy_level as COORD + 2) >> 1) * (UNIT_HEIGHT + 1);
            while energy_delta < 0 {
                r.corner.x = x + ENERGY_XOFFS + 1 + UNIT_WIDTH + 1;
                if (*ship).energy_level & 1 == 0 {
                    r.corner.x -= UNIT_WIDTH + 1;
                    r.corner.y += UNIT_HEIGHT + 1;
                }
                DrawFilledRectangle(&mut r);
                (*ship).energy_level -= 1;
                energy_delta += 1;
            }
        }
    }
}

#[path = "shipstat_gauges.rs"]
mod gauge_frames;
