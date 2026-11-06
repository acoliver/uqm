#![cfg(not(test))]

use crate::battle::status_native::draw_battle_crew_amount as draw_crew;
use crate::engine::battle::link_queue::PickShip;
use crate::engine::{Kernel, WorldData};
use crate::graphics::context_abi::_pCurContext;
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
    let (retained_status_frame, preferences) =
        Kernel::current().read(|owner| (owner.status_frame, owner.configuration.preferences()));
    let mut stamp: STAMP = mem::zeroed();
    stamp.origin.y = y + GAUGE_YOFFS + STARCON_TEXT_HEIGHT;
    if state == 0 {
        stamp.origin.x = CREW_XOFFS + (STAT_WIDTH >> 1) + 6;
        stamp.frame = SetAbsFrameIndex(
            retained_status_frame,
            if preferences.menu == OPT_PC { 4 } else { 0 },
        );
        DrawStamp(&mut stamp);
    }
    stamp.origin.x = ENERGY_XOFFS + (STAT_WIDTH >> 1) - 5;
    stamp.frame = SetAbsFrameIndex(
        Kernel::current().read(|owner| owner.status_frame),
        if preferences.menu == OPT_PC { 5 } else { 1 },
    );
    if state >= 0 {
        DrawStamp(&mut stamp);
    } else {
        SetContextForeGroundColor(rgb15(0x1f, 0x1f, 0x0a));
        DrawFilledStamp(&mut stamp);
    }
}

unsafe fn draw_ship_name_string(p_str: *mut UNICODE, char_count: COUNT, y: COORD) {
    let old_font = SetContextFont(Kernel::current().read(|owner| owner.star_con_font));
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
    let context = crate::graphics::context_abi::ContextLease::retain(
        std::ptr::NonNull::new(_pCurContext).unwrap_or_else(|| panic!("missing status context")),
    );
    clear_ship_status(&context, y);
}

pub fn clear_ship_status(context: &crate::graphics::context_abi::ContextLease, y: COORD) {
    context.fill_rectangle(
        rgb15(0x0a, 0x0a, 0x0a),
        RECT {
            corner: POINT { x: 2, y: 3 + y },
            extent: EXTENT {
                width: STATUS_WIDTH - 4,
                height: SHIP_INFO_HEIGHT - 3,
            },
        },
    );
}

/// # Safety
/// The status context must be active and graphics initialized.
#[no_mangle]
pub unsafe extern "C" fn OutlineShipStatus(y: COORD) {
    let context = crate::graphics::context_abi::ContextLease::retain(
        std::ptr::NonNull::new(_pCurContext).unwrap_or_else(|| panic!("missing status context")),
    );
    outline_ship_status(&context, y);
}

pub fn outline_ship_status(context: &crate::graphics::context_abi::ContextLease, y: COORD) {
    let mut r = RECT {
        corner: POINT { x: 0, y: 1 + y },
        extent: EXTENT {
            width: STATUS_WIDTH,
            height: 1,
        },
    };
    let dark = rgb15(0x08, 0x08, 0x08);
    context.fill_rectangle(dark, r);
    r.corner.y += 1;
    r.extent.width -= 1;
    context.fill_rectangle(dark, r);
    r.extent.width = 1;
    r.extent.height = SHIP_INFO_HEIGHT - 2;
    context.fill_rectangle(dark, r);
    r.corner.x += 1;
    context.fill_rectangle(dark, r);
    let light = rgb15(0x10, 0x10, 0x10);
    r.corner.x = STATUS_WIDTH - 1;
    context.fill_rectangle(light, r);
    r.corner.x = STATUS_WIDTH - 2;
    r.corner.y += 1;
    r.extent.height -= 1;
    context.fill_rectangle(light, r);
    r.corner.x = 0;
    r.corner.y = y;
    r.extent.width = STATUS_WIDTH;
    r.extent.height = 1;
    context.fill_rectangle(rgb15(0, 0, 0), r);
}

/// # Safety
/// SIPtr must point to writable ship info. Optional pointers must be null or point to valid game objects.
pub(crate) unsafe fn init_ship_status(
    font: Option<&crate::graphics::native_font::FontLease>,
    siptr: *mut SHIP_INFO,
    starship: Option<PickShip>,
    clip_rect: *mut RECT,
) {
    let mut y: COORD = 0;
    let mut stamp: STAMP = mem::zeroed();
    let mut old_clip: RECT = mem::zeroed();
    let mut old_origin = POINT { x: 0, y: 0 };
    if let Some(ship) = starship {
        assert!(ship.player >= 0);
        y = Kernel::current().read(|owner| owner.status_offsets[ship.player as usize]);
    }
    let (context, pc_menu) = Kernel::current().read(|owner| {
        (
            owner.status_context,
            owner.configuration.preferences().menu == OPT_PC,
        )
    });
    let old_context = SetContext(context);
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
    let gauge_context = crate::graphics::context_abi::ContextLease::retain(
        std::ptr::NonNull::new(context.cast())
            .unwrap_or_else(|| panic!("status gauge context not admitted")),
    );
    draw_gauge_frames(&gauge_context, (*siptr).max_crew, (*siptr).max_energy, y);

    draw_status_title(siptr, starship, y);
    let crew_delta = (*siptr).crew_level as SIZE;
    let energy_delta = (*siptr).energy_level as SIZE;
    (*siptr).crew_level = 0;
    (*siptr).energy_level = 0;
    delta_statistics(pc_menu, font, siptr, y, crew_delta, energy_delta);
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
    let world = WorldData::current();
    if starship.is_none_or(|ship| ship.captain != 0) {
        DrawCrewFuelString(y, 0);
        let loc = SetAbsStringTableIndex((*siptr).race_strings, 1);
        draw_ship_name_string(GetStringAddress(loc).cast(), GetStringLength(loc), y);
        let old_font = SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
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
            std::ptr::addr_of_mut!((**world).flagship.ship_name).cast(),
            !0,
            y,
        );
    }
}

/// # Safety
/// ShipInfoPtr must be writable and point to valid ship information; graphics context must be initialized.
pub(crate) unsafe fn delta_statistics(
    pc_menu: bool,
    font: Option<&crate::graphics::native_font::FontLease>,
    ship: *mut SHIP_INFO,
    y_offs: COORD,
    crew_delta: SIZE,
    energy_delta: SIZE,
) {
    if crew_delta == 0 && energy_delta == 0 {
        return;
    }
    let context = crate::graphics::context_abi::ContextLease::retain(
        std::ptr::NonNull::new(_pCurContext)
            .unwrap_or_else(|| panic!("statistics context not admitted")),
    );
    update_statistics(
        &context,
        crate::graphics::context_abi::_GraphicsStatusFlags & 4 != 0,
        pc_menu,
        font,
        &mut *ship,
        y_offs,
        (crew_delta, energy_delta),
    );
}

pub fn update_statistics(
    context: &crate::graphics::context_abi::ContextLease,
    numeric_crew: bool,
    pc_menu: bool,
    font: Option<&crate::graphics::native_font::FontLease>,
    ship: &mut SHIP_INFO,
    y_offs: COORD,
    deltas: (SIZE, SIZE),
) {
    let (crew_delta, energy_delta) = deltas;
    if crew_delta != 0 {
        draw_crew_delta(
            context,
            numeric_crew,
            pc_menu,
            font,
            ship,
            y_offs,
            crew_delta,
        );
    }
    if energy_delta != 0 {
        draw_energy_delta(context, ship, y_offs, energy_delta);
    }
}

fn draw_crew_delta(
    context: &crate::graphics::context_abi::ContextLease,
    numeric_crew: bool,
    pc_menu: bool,
    font: Option<&crate::graphics::native_font::FontLease>,
    ship: &mut SHIP_INFO,
    y_offs: COORD,
    crew_delta: SIZE,
) {
    let y = GAUGE_YOFFS + y_offs;
    let mut r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: UNIT_WIDTH,
            height: UNIT_HEIGHT,
        },
    };
    let old_blocks = std::cmp::min(ship.crew_level, MAX_CREW_SIZE as COUNT) as COORD;
    let new_level = (ship.crew_level as i32 + crew_delta as i32) as COUNT;
    let new_blocks = std::cmp::min(new_level, MAX_CREW_SIZE as COUNT) as COORD;
    if crew_delta > 0 {
        r.corner.y = (y + 1) - (((old_blocks + 1) >> 1) * (UNIT_HEIGHT + 1));
        let color = if ship.ship_flags & CREW_IMMUNE != 0 {
            rgb15(0x0a, 0x0a, 0x0a)
        } else {
            rgb15(0, 0x14, 0)
        };
        context.set_foreground(color);
        for i in old_blocks..new_blocks {
            r.corner.x = CREW_XOFFS + 1;
            if i & 1 == 0 {
                r.corner.x += UNIT_WIDTH + 1;
                r.corner.y -= UNIT_HEIGHT + 1;
            }
            context.fill_rectangle(color, r);
        }
    } else {
        let color = rgb15(0, 0, 0);
        context.set_foreground(color);
        r.corner.y = (y + 1) - (((old_blocks + 2) >> 1) * (UNIT_HEIGHT + 1));
        let mut i = old_blocks;
        while i > new_blocks {
            r.corner.x = CREW_XOFFS + 1 + UNIT_WIDTH + 1;
            if i & 1 == 0 {
                r.corner.x -= UNIT_WIDTH + 1;
                r.corner.y += UNIT_HEIGHT + 1;
            }
            context.fill_rectangle(color, r);
            i -= 1;
        }
    }
    draw_crew_marker(context, ship, new_level, y);
    ship.crew_level = new_level;
    if numeric_crew
        && crate::battle::status_native::has_crew_counter(ship.max_crew, ship.ship_flags)
    {
        draw_crew(
            context,
            font.unwrap_or_else(|| panic!("numeric crew display requires the loaded battle font")),
            ship.crew_level,
            y_offs,
            pc_menu,
        );
    }
}

fn draw_energy_delta(
    context: &crate::graphics::context_abi::ContextLease,
    ship: &mut SHIP_INFO,
    y_offs: COORD,
    mut energy_delta: SIZE,
) {
    let y = GAUGE_YOFFS + y_offs;
    let mut r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: UNIT_WIDTH,
            height: UNIT_HEIGHT,
        },
    };
    if energy_delta > 0 {
        let color = rgb15(0x14, 0, 0);
        context.set_foreground(color);
        r.corner.y = (y + 1) - ((ship.energy_level as COORD + 1) >> 1) * (UNIT_HEIGHT + 1);
        while energy_delta > 0 {
            r.corner.x = ENERGY_XOFFS + 1;
            if ship.energy_level & 1 == 0 {
                r.corner.x += UNIT_WIDTH + 1;
                r.corner.y -= UNIT_HEIGHT + 1;
            }
            context.fill_rectangle(color, r);
            ship.energy_level += 1;
            energy_delta -= 1;
        }
    } else {
        let color = rgb15(0, 0, 0);
        context.set_foreground(color);
        r.corner.y = (y + 1) - ((ship.energy_level as COORD + 2) >> 1) * (UNIT_HEIGHT + 1);
        while energy_delta < 0 {
            r.corner.x = ENERGY_XOFFS + 1 + UNIT_WIDTH + 1;
            if ship.energy_level & 1 == 0 {
                r.corner.x -= UNIT_WIDTH + 1;
                r.corner.y += UNIT_HEIGHT + 1;
            }
            context.fill_rectangle(color, r);
            ship.energy_level -= 1;
            energy_delta += 1;
        }
    }
}

fn draw_crew_marker(
    context: &crate::graphics::context_abi::ContextLease,
    ship: &SHIP_INFO,
    new_level: COUNT,
    y: COORD,
) {
    if ship.ship_flags & PLAYER_CAPTAIN != 0
        && ((ship.crew_level > MAX_CREW_SIZE as COUNT) != (new_level > MAX_CREW_SIZE as COUNT)
            || ship.crew_level == 0)
        && new_level != 0
    {
        let color = if new_level > MAX_CREW_SIZE as COUNT {
            rgb15(0, 0x14, 0)
        } else {
            rgb15(0x0a, 0x0a, 0x1f)
        };
        let rect = RECT {
            corner: POINT {
                x: CREW_XOFFS + 1 + UNIT_WIDTH + 1,
                y: y - UNIT_HEIGHT,
            },
            extent: EXTENT {
                width: UNIT_WIDTH,
                height: UNIT_HEIGHT,
            },
        };
        context.fill_rectangle(color, rect);
    }
}

pub fn draw_gauge_frames(
    context: &crate::graphics::context_abi::ContextLease,
    max_crew: COUNT,
    max_energy: BYTE,
    y: COORD,
) {
    let crew = (((std::cmp::min(max_crew, MAX_CREW_SIZE) + 1) & !1) + 1) as SIZE;
    let energy = ((((max_energy + 1) >> 1) << 1) + 1) as COORD;
    let bottom = GAUGE_YOFFS + y;
    let draw = |color, x, y, width, height| {
        context.fill_rectangle(
            color,
            RECT {
                corner: POINT { x, y },
                extent: EXTENT { width, height },
            },
        );
    };
    let dark = rgb15(0x08, 0x08, 0x08);
    draw(dark, CREW_XOFFS - 1, bottom + 1, STAT_WIDTH + 2, 1);
    draw(dark, ENERGY_XOFFS - 1, bottom + 1, STAT_WIDTH + 2, 1);
    draw(
        dark,
        ENERGY_XOFFS + STAT_WIDTH,
        bottom + 1 - energy,
        1,
        energy,
    );
    draw(dark, CREW_XOFFS + STAT_WIDTH, bottom + 1 - crew, 1, crew);
    let light = rgb15(0x10, 0x10, 0x10);
    draw(light, CREW_XOFFS - 1, bottom - crew, STAT_WIDTH + 2, 1);
    draw(light, ENERGY_XOFFS - 1, bottom - energy, STAT_WIDTH + 2, 1);
    draw(light, ENERGY_XOFFS - 1, bottom - energy, 1, energy + 1);
    draw(light, CREW_XOFFS - 1, bottom - crew, 1, crew + 1);
    let black = rgb15(0, 0, 0);
    draw(black, CREW_XOFFS, bottom + 1 - crew, STAT_WIDTH, crew);
    draw(black, ENERGY_XOFFS, bottom + 1 - energy, STAT_WIDTH, energy);
}
