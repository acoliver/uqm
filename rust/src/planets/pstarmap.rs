//! Star-map UI, ported from `planets/pstarmap.c` (GPL-2.0-or-later).
//!
//! All graphics handles here are native FRAME/CONTEXT pointers. The game
//! thread owns the map state; callbacks may reenter drawing and input code.

#![allow(non_snake_case, non_upper_case_globals)]

use crate::c_bindings::global_state::{
    Color, DrawFilledOval, DrawOval, EXTENT, FLEET_INFO, FONT, FRAME, GLOBDATA, MENU_STATE, POINT,
    QUEUE, RECT, STAMP, STRING, TEXT,
};
use crate::comm::locdata::CRect;
use crate::graphics::{font_abi, frame_abi, pixmap_abi};
use crate::planets::starmap::FindStar;
use crate::savegame::runtime::star::{CStarDesc, NUM_SOLAR_SYSTEMS};
use crate::strings::{string_table, unicode};
use std::ffi::{c_char, c_long, c_void};
use std::ptr;

#[path = "pstarmap/search.rs"]
mod search;
pub use search::DoStarSearch;

const MAX_X_UNIVERSE: i32 = 9999;
const MAX_Y_UNIVERSE: i32 = 9999;
const ONE_SECOND: u32 = 840;
const MAX_ZOOM_SHIFT: i32 = 4;
const ARILOU_SPACE: POINT = POINT { x: 438, y: 6372 };
const QUASI_SPACE: POINT = POINT { x: 5000, y: 5000 };
const ARILOU_HOME: POINT = POINT { x: 6134, y: 5900 };

static mut mapOrigin: POINT = POINT { x: 0, y: 0 };
static mut zoomLevel: i32 = 0;
static mut StarMapFrame: FRAME = ptr::null_mut();
static mut transition_pending: bool = false;

use crate::time::counter_abi::GetTimeCounter;
extern "C" {
    static mut cursorLoc: POINT;
    static mut star_array: *mut CStarDesc;
    static mut CurStarDescPtr: *mut CStarDesc;
    static mut GlobData: GLOBDATA;
    static mut SpaceContext: *mut c_void;
    static mut MiscDataFrame: FRAME;
    static mut TinyFont: FONT;
    static mut GameStrings: STRING;
    static ScreenWidth: i32;
    static ScreenHeight: i32;

    fn SetContext(context: *mut c_void) -> *mut c_void;
    fn SetContextForeGroundColor(color: Color) -> Color;
    fn SetContextBackGroundColor(color: Color) -> Color;
    fn SetContextClipRect(rect: *mut CRect);
    fn GetContextClipRect(rect: *mut RECT) -> u32;
    fn SetContextOrigin(point: POINT) -> POINT;
    fn ClearDrawable();
    fn DrawHyperCoords(point: POINT);
    fn DrawSISMessage(text: *const c_char);
    fn DrawStatusMessage(text: *const c_char);
    fn DrawSISMessageEx(text: *const c_char, cursor: i16, extent: i16, flags: u16) -> u32;
    fn LoadIntoExtraScreen(rect: *mut CRect);
    fn RepairBackRect(rect: *mut CRect);
    fn SleepThread(ticks: i32);
    fn SetTransitionSource(rect: *mut c_void);
    fn ScreenTransition(kind: i32, rect: *const c_void);
    fn GetFrameRect(frame: *mut c_void, rect: *mut c_void);
    fn GetClusterName(star: *const CStarDesc, buffer: *mut c_char);
    fn inHQSpace() -> i32;

    fn FlushInput();
    fn AnyButtonPress(check_special: u32) -> u32;
    fn SetMenuSounds(move_sound: u16, select_sound: u16);
    fn PlayMenuSound(sound: i32);
    fn SetMenuRepeatDelay(min: u32, max: u32, step: u32, gestalt: i32);
    fn SetDefaultMenuRepeatDelay();
    fn DoInput(state: *mut c_void, reset: u32);
    fn DoMoveCursor(menu: *mut MENU_STATE) -> u32;
    fn getGameState(state: *const u8, start: i32, end: i32) -> u8;
    fn sprintf(dst: *mut c_char, fmt: *const c_char, ...) -> i32;
}
use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

const RACE_COLORS: [Color; 23] = [
    rgb15(0, 0, 16),
    rgb15(0, 0, 0),
    rgb15(0, 1, 31),
    rgb15(14, 0, 14),
    rgb15(0, 6, 8),
    rgb15(0, 0, 0),
    rgb15(12, 5, 0),
    rgb15(12, 5, 0),
    rgb15(0, 6, 8),
    rgb15(0, 6, 8),
    rgb15(0, 0, 16),
    rgb15(10, 0, 17),
    rgb15(0, 6, 8),
    rgb15(15, 0, 0),
    rgb15(14, 0, 14),
    rgb15(14, 0, 14),
    rgb15(12, 5, 0),
    rgb15(10, 0, 17),
    rgb15(0, 8, 0),
    rgb15(15, 0, 0),
    rgb15(0, 0, 0),
    rgb15(6, 6, 6),
    rgb15(20, 7, 31),
];

unsafe fn width() -> i32 {
    ScreenWidth - 78
}
unsafe fn height() -> i32 {
    ScreenHeight - 13
}
unsafe fn which_space() -> u8 {
    getGameState(
        ptr::addr_of_mut!(GlobData.Game_state.GameState).cast(),
        42,
        43,
    )
}
unsafe fn portal_open() -> bool {
    getGameState(
        ptr::addr_of_mut!(GlobData.Game_state.GameState).cast(),
        41,
        41,
    ) != 0
}
unsafe fn game_string(index: u16) -> *const c_char {
    string_table::GetStringAddress(string_table::SetAbsStringTableIndex(
        GameStrings.cast(),
        index,
    ))
}
fn point(p: crate::comm::dispatch::CPoint) -> POINT {
    POINT { x: p.x, y: p.y }
}
fn zero_rect() -> RECT {
    RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 0,
            height: 0,
        },
    }
}

fn signed_div_with_error(mut value: c_long, divisor: c_long) -> c_long {
    let invert = value < 0;
    if invert {
        value = -value;
    }
    value = (value + (divisor >> 1)) / divisor;
    if invert {
        -value
    } else {
        value
    }
}

#[derive(Clone, Copy)]
struct Projection {
    origin: POINT,
    zoom: i32,
    width: i32,
    height: i32,
}
impl Projection {
    fn x(self, ux: c_long) -> i16 {
        let fit = 10000 / self.width + 1;
        (signed_div_with_error(
            ((ux - c_long::from(self.origin.x)) << self.zoom) * c_long::from(self.width),
            c_long::from(9999 + fit),
        ) + c_long::from((self.width - 1) >> 1)) as i16
    }
    fn y(self, uy: c_long) -> i16 {
        (signed_div_with_error(
            ((c_long::from(self.origin.y) - uy) << self.zoom) * c_long::from(self.height),
            10001,
        ) + c_long::from((self.height - 1) >> 1)) as i16
    }
    fn inverse_x(self, dx: i16) -> i16 {
        let fit = 10000 / self.width + 1;
        ((((c_long::from(dx) - c_long::from((self.width - 1) >> 1)) * c_long::from(9999 + fit))
            >> self.zoom)
            / c_long::from(self.width)
            + c_long::from(self.origin.x)) as i16
    }
    fn inverse_y(self, dy: i16) -> i16 {
        ((((c_long::from((self.height - 1) >> 1) - c_long::from(dy)) * 10001) >> self.zoom)
            / c_long::from(self.height)
            + c_long::from(self.origin.y)) as i16
    }
}
unsafe fn projection() -> Projection {
    Projection {
        origin: mapOrigin,
        zoom: zoomLevel,
        width: width(),
        height: height(),
    }
}

/// # Safety
/// Game-thread only; map state and screen dimensions must be initialized.
#[no_mangle]
pub unsafe extern "C" fn universeToDispx(ux: c_long) -> i16 {
    projection().x(ux)
}
/// # Safety
/// Game-thread only; map state and screen dimensions must be initialized.
#[no_mangle]
pub unsafe extern "C" fn universeToDispy(uy: c_long) -> i16 {
    projection().y(uy)
}
unsafe fn ux(x: i16) -> i16 {
    universeToDispx(c_long::from(x))
}
unsafe fn uy(y: i16) -> i16 {
    universeToDispy(c_long::from(y))
}

unsafe fn current_location() -> POINT {
    if inHQSpace() == 0 {
        point((*CurStarDescPtr).star_pt)
    } else {
        let lx = (((ScreenWidth - 64) << 5) >> 4) * 195;
        let ly = ((ScreenHeight << 5) >> 4) * 25;
        POINT {
            x: ((GlobData.SIS_state.log_x * 6250 + (lx >> 1)) / lx) as i16,
            y: (9999 - (GlobData.SIS_state.log_y * 625 + (ly >> 1)) / ly) as i16,
        }
    }
}

/// # Safety
/// Game-thread only, with native map frames and graphics context live.
#[no_mangle]
pub unsafe extern "C" fn flashCurrentLocation(where_: *const POINT) {
    static mut COLOR: u8 = 0;
    static mut DELTA: i32 = -2;
    static mut UNIVERSE: POINT = POINT { x: 0, y: 0 };
    static mut NEXT_TIME: u32 = 0;
    if !where_.is_null() {
        UNIVERSE = where_.read();
    }
    if GetTimeCounter() >= NEXT_TIME {
        NEXT_TIME = GetTimeCounter().wrapping_add(ONE_SECOND / 16);
        let old_context = SetContext(SpaceContext);
        if COLOR == 0 || COLOR == 0x1a {
            DELTA = -DELTA;
        }
        COLOR = COLOR.wrapping_add(DELTA as u8);
        let old_color = SetContextForeGroundColor(rgb15(COLOR, COLOR, COLOR));
        let mut stamp = STAMP {
            origin: POINT {
                x: ux(UNIVERSE.x),
                y: uy(UNIVERSE.y),
            },
            frame: pixmap_abi::IncFrameIndex(StarMapFrame.cast()).cast(),
        };
        frame_abi::DrawFilledStamp(ptr::from_mut(&mut stamp).cast());
        SetContextForeGroundColor(old_color);
        SetContext(old_context);
    }
}
unsafe fn draw_cursor(x: i16, y: i16) {
    let mut stamp = STAMP {
        origin: POINT { x, y },
        frame: StarMapFrame,
    };
    frame_abi::DrawStamp(ptr::from_mut(&mut stamp).cast());
}

// SIZE arithmetic narrows at the same assignments as the C line walker.
struct LineWalk {
    dx: i16,
    dy: i16,
    xi: i16,
    yi: i16,
    xe: i16,
    ye: i16,
    cycle: i16,
    delta: i16,
}
impl LineWalk {
    fn new(mut dx: i16, mut dy: i16) -> Self {
        let xi = if dx >= 0 {
            1
        } else {
            dx = dx.wrapping_neg();
            -1
        };
        dx = dx.wrapping_shl(1);
        let yi = if dy >= 0 {
            1
        } else {
            dy = dy.wrapping_neg();
            -1
        };
        dy = dy.wrapping_shl(1);
        let cycle = dx.max(dy);
        Self {
            dx,
            dy,
            xi,
            yi,
            xe: cycle >> 1,
            ye: cycle >> 1,
            cycle,
            delta: cycle >> 1,
        }
    }
    fn step(&mut self, p: &mut POINT) {
        self.xe = self.xe.wrapping_sub(self.dx);
        if self.xe <= 0 {
            p.x = p.x.wrapping_add(self.xi);
            self.xe = self.xe.wrapping_add(self.cycle);
        }
        self.ye = self.ye.wrapping_sub(self.dy);
        if self.ye <= 0 {
            p.y = p.y.wrapping_add(self.yi);
            self.ye = self.ye.wrapping_add(self.cycle);
        }
    }
}
unsafe fn draw_autopilot(destination: POINT) {
    let mut p = current_location();
    p = POINT {
        x: ux(p.x),
        y: uy(p.y),
    };
    let mut walk = LineWalk::new(
        ux(destination.x).wrapping_sub(p.x),
        uy(destination.y).wrapping_sub(p.y),
    );
    SetContextForeGroundColor(rgb15(4, 4, 31));
    walk.delta &= !1;
    loop {
        let previous = walk.delta;
        walk.delta = walk.delta.wrapping_sub(1);
        if previous == 0 {
            break;
        }
        if walk.delta & 1 == 0 {
            frame_abi::DrawPoint(ptr::from_mut(&mut p).cast());
        }
        walk.step(&mut p);
    }
}

fn box_union(a: RECT, b: RECT) -> RECT {
    let x = i32::from(a.corner.x).min(i32::from(b.corner.x));
    let y = i32::from(a.corner.y).min(i32::from(b.corner.y));
    RECT {
        corner: POINT {
            x: x as i16,
            y: y as i16,
        },
        extent: EXTENT {
            width: ((i32::from(a.corner.x) + i32::from(a.extent.width))
                .max(i32::from(b.corner.x) + i32::from(b.extent.width))
                - x) as i16,
            height: ((i32::from(a.corner.y) + i32::from(a.extent.height))
                .max(i32::from(b.corner.y) + i32::from(b.extent.height))
                - y) as i16,
        },
    }
}
unsafe fn fleet_queue() -> *mut QUEUE {
    ptr::addr_of_mut!(GlobData.Game_state.avail_race_q)
}
unsafe fn lock_fleet(handle: *mut c_void) -> *mut FLEET_INFO {
    let q = fleet_queue();
    debug_assert_eq!(
        usize::from((*q).object_size),
        std::mem::size_of::<FLEET_INFO>()
    );
    check_fleet_link(q, handle);
    handle.cast()
}
unsafe fn check_fleet_link(q: *const QUEUE, handle: *mut c_void) {
    debug_assert!(
        !(*q).pq_tab.is_null()
            && handle as usize >= (*q).pq_tab as usize
            && (handle as usize)
                < (*q).pq_tab as usize
                    + usize::from((*q).object_size) * usize::from((*q).num_objects)
    );
}
unsafe fn unlock_fleet(handle: *mut c_void) {
    check_fleet_link(fleet_queue(), handle);
}
unsafe fn sphere_text(fleet: *const FLEET_INFO, r: RECT) -> TEXT {
    font_abi::SetContextFont(TinyFont.cast());
    let string = string_table::SetAbsStringTableIndex((*fleet).race_strings.cast(), 1);
    TEXT {
        baseline: POINT {
            x: r.corner.x.wrapping_add(r.extent.width >> 1),
            y: r.corner
                .y
                .wrapping_add(r.extent.height >> 1)
                .wrapping_sub(1),
        },
        align: 1,
        CharCount: string_table::GetStringLength(string),
        pStr: string_table::GetStringAddress(string),
    }
}
unsafe fn get_sphere_rect(fleet: *const FLEET_INFO) -> (RECT, RECT) {
    let diameter = c_long::from(i32::from((*fleet).known_strength) * 11);
    let mut w = universeToDispx(diameter).wrapping_sub(ux(0));
    if w < 0 {
        w = w.wrapping_neg();
    } else if w == 0 {
        w = 1;
    }
    let mut h = universeToDispy(diameter).wrapping_sub(uy(0));
    if h < 0 {
        h = h.wrapping_neg();
    } else if h == 0 {
        h = 1;
    }
    let r = RECT {
        corner: POINT {
            x: ux((*fleet).known_loc.x).wrapping_sub(w >> 1),
            y: uy((*fleet).known_loc.y).wrapping_sub(h >> 1),
        },
        extent: EXTENT {
            width: w,
            height: h,
        },
    };
    let mut t = sphere_text(fleet, r);
    let mut repair = zero_rect();
    font_abi::TextRect(
        ptr::from_mut(&mut t).cast(),
        ptr::from_mut(&mut repair).cast(),
        ptr::null_mut(),
    );
    if repair.corner.x <= 0 {
        repair.corner.x = 1;
    } else if i32::from(repair.corner.x) + i32::from(repair.extent.width) >= width() {
        repair.corner.x = (width() - i32::from(repair.extent.width) - 1) as i16;
    }
    if repair.corner.y <= 0 {
        repair.corner.y = 1;
    } else if i32::from(repair.corner.y) + i32::from(repair.extent.height) >= height() {
        repair.corner.y = (height() - i32::from(repair.extent.height) - 1) as i16;
    }
    repair = box_union(repair, r);
    repair.extent.width = repair.extent.width.wrapping_add(1);
    repair.extent.height = repair.extent.height.wrapping_add(1);
    (r, repair)
}
fn intersects(a: RECT, b: RECT) -> bool {
    i32::from(a.corner.x) < i32::from(b.corner.x) + i32::from(b.extent.width)
        && i32::from(a.corner.y) < i32::from(b.corner.y) + i32::from(b.extent.height)
        && i32::from(a.corner.x) + i32::from(a.extent.width) > i32::from(b.corner.x)
        && i32::from(a.corner.y) + i32::from(a.extent.height) > i32::from(b.corner.y)
}

/// # Safety
/// Game-thread only. `clip` is null, the sentinel `(RECT*)-1`, or a live RECT.
#[no_mangle]
pub unsafe extern "C" fn DrawStarMap(race_update: u16, clip: *mut RECT) {
    let draw_cursor_flag = clip as isize != -1;
    let clip = if draw_cursor_flag {
        clip
    } else {
        ptr::null_mut()
    };
    let mut old_r = zero_rect();
    let mut old_origin = POINT { x: 0, y: 0 };
    SetContext(SpaceContext);
    if !clip.is_null() {
        GetContextClipRect(&mut old_r);
        (*clip).corner.x = (*clip).corner.x.wrapping_add(old_r.corner.x);
        (*clip).corner.y = (*clip).corner.y.wrapping_add(old_r.corner.y);
        SetContextClipRect(clip.cast());
        (*clip).corner.x = (*clip).corner.x.wrapping_sub(old_r.corner.x);
        (*clip).corner.y = (*clip).corner.y.wrapping_sub(old_r.corner.y);
        old_origin = SetContextOrigin(POINT {
            x: (*clip).corner.x.wrapping_neg(),
            y: (*clip).corner.y.wrapping_neg(),
        });
    }
    if transition_pending {
        SetTransitionSource(ptr::null_mut());
    }
    BatchGraphics();
    let space = which_space();
    let mut star = star_array.add(if space <= 1 { 0 } else { NUM_SOLAR_SYSTEMS + 1 });
    if space <= 1 {
        SetContextForeGroundColor(rgb15(0, 0, 7));
        SetContextBackGroundColor(rgb15(0, 0, 0));
    } else {
        SetContextForeGroundColor(rgb15(0, 11, 0));
        SetContextBackGroundColor(rgb15(0, 8, 0));
    }
    ClearDrawable();
    let mut diameter = c_long::from(GlobData.SIS_state.FuelOnBoard) << 1;
    if race_update == 0 && space < 2 && diameter != 0 {
        let location = current_location();
        if diameter > c_long::from(MAX_X_UNIVERSE * 4) {
            diameter = c_long::from(MAX_X_UNIVERSE * 4);
        }
        let mut w = universeToDispx(diameter).wrapping_sub(ux(0));
        if w < 0 {
            w = w.wrapping_neg();
        }
        let mut h = universeToDispy(diameter).wrapping_sub(uy(0));
        if h < 0 {
            h = h.wrapping_neg();
        }
        let mut r = RECT {
            corner: POINT {
                x: ux(location.x).wrapping_sub(w >> 1),
                y: uy(location.y).wrapping_sub(h >> 1),
            },
            extent: EXTENT {
                width: w,
                height: h,
            },
        };
        let old_color = SetContextForeGroundColor(rgb15(3, 3, 3));
        DrawFilledOval(&mut r);
        SetContextForeGroundColor(old_color);
    }
    let mut i = MAX_Y_UNIVERSE + 1;
    while i >= 0 {
        let mut r = RECT {
            corner: POINT {
                x: ux(0),
                y: universeToDispy(c_long::from(i)),
            },
            extent: EXTENT {
                width: (width() << zoomLevel) as i16,
                height: 1,
            },
        };
        frame_abi::DrawFilledRectangle(ptr::from_mut(&mut r).cast());
        r.corner.y = universeToDispy(c_long::from(MAX_Y_UNIVERSE + 1));
        r.extent.width = 1;
        r.extent.height = (height() << zoomLevel) as i16;
        let mut j = MAX_X_UNIVERSE + 1;
        while j >= 0 {
            r.corner.x = universeToDispx(c_long::from(j));
            frame_abi::DrawFilledRectangle(ptr::from_mut(&mut r).cast());
            j -= 500;
        }
        i -= 500;
    }
    let star_frame = pixmap_abi::SetRelFrameIndex(StarMapFrame.cast(), 2);
    if space <= 1 {
        let mut index = 0;
        let mut handle = (*fleet_queue()).head;
        while !handle.is_null() {
            let fleet = lock_fleet(handle);
            let next = (*fleet).succ;
            if (*fleet).known_strength != 0 {
                let (mut r, repair) = get_sphere_rect(fleet);
                if i32::from(r.corner.x) < width()
                    && i32::from(r.corner.y) < height()
                    && i32::from(r.corner.x) + i32::from(r.extent.width) > 0
                    && i32::from(r.corner.y) + i32::from(r.extent.height) > 0
                    && (clip.is_null() || intersects(repair, *clip))
                {
                    let mut color = RACE_COLORS[index];
                    SetContextForeGroundColor(if index + 1 == usize::from(race_update) {
                        rgb15(31, 31, 31)
                    } else {
                        color
                    });
                    DrawOval(&mut r, 0);
                    let mut t = sphere_text(fleet, r);
                    font_abi::TextRect(
                        ptr::from_mut(&mut t).cast(),
                        ptr::from_mut(&mut r).cast(),
                        ptr::null_mut(),
                    );
                    if r.corner.x <= 0 {
                        t.baseline.x = t.baseline.x.wrapping_sub(r.corner.x).wrapping_add(1);
                    } else if i32::from(r.corner.x) + i32::from(r.extent.width) >= width() {
                        t.baseline.x = (i32::from(t.baseline.x)
                            - (i32::from(r.corner.x) + i32::from(r.extent.width) - width() + 1))
                            as i16;
                    }
                    if r.corner.y <= 0 {
                        t.baseline.y = t.baseline.y.wrapping_sub(r.corner.y).wrapping_add(1);
                    } else if i32::from(r.corner.y) + i32::from(r.extent.height) >= height() {
                        t.baseline.y = (i32::from(t.baseline.y)
                            - (i32::from(r.corner.y) + i32::from(r.extent.height) - height() + 1))
                            as i16;
                    }
                    color.r = color.r.saturating_add(24);
                    color.g = color.g.saturating_add(24);
                    color.b = color.b.saturating_add(24);
                    SetContextForeGroundColor(color);
                    font_abi::font_DrawText(ptr::from_mut(&mut t).cast());
                }
            }
            unlock_fleet(handle);
            handle = next;
            index += 1;
        }
    }
    loop {
        let location = point((*star).star_pt);
        let index = if space <= 1 {
            i16::from(((*star).star_type & 0x18) >> 3) * 6 + i16::from((*star).star_type >> 5)
        } else if location.x == ARILOU_HOME.x && location.y == ARILOU_HOME.y {
            2 * 6 + 1
        } else {
            6 + 1
        };
        let mut stamp = STAMP {
            origin: POINT {
                x: ux(location.x),
                y: uy(location.y),
            },
            frame: pixmap_abi::SetRelFrameIndex(star_frame, index).cast(),
        };
        frame_abi::DrawStamp(ptr::from_mut(&mut stamp).cast());
        star = star.add(1);
        if i32::from((*star).star_pt.x) > MAX_X_UNIVERSE
            || i32::from((*star).star_pt.y) > MAX_Y_UNIVERSE
        {
            break;
        }
    }
    if portal_open() {
        let location = if space <= 1 {
            ARILOU_SPACE
        } else {
            QUASI_SPACE
        };
        let mut stamp = STAMP {
            origin: POINT {
                x: ux(location.x),
                y: uy(location.y),
            },
            frame: pixmap_abi::SetRelFrameIndex(star_frame, 6 + 1).cast(),
        };
        frame_abi::DrawStamp(ptr::from_mut(&mut stamp).cast());
    }
    let destination = GlobData.Game_state.autopilot;
    if race_update == 0 && destination.x != -1 && destination.y != -1 {
        draw_autopilot(destination);
    }
    if transition_pending {
        let mut r = zero_rect();
        GetContextClipRect(&mut r);
        ScreenTransition(3, ptr::from_ref(&r).cast());
        transition_pending = false;
    }
    UnbatchGraphics();
    if !clip.is_null() {
        SetContextClipRect(ptr::from_mut(&mut old_r).cast());
        SetContextOrigin(old_origin);
    }
    if race_update == 0 && draw_cursor_flag {
        let mut r = zero_rect();
        GetContextClipRect(&mut r);
        LoadIntoExtraScreen(ptr::from_mut(&mut r).cast());
        draw_cursor(ux(cursorLoc.x), uy(cursorLoc.y));
    }
}

unsafe fn erase_cursor(x: i16, y: i16) {
    let mut r = zero_rect();
    GetFrameRect(StarMapFrame.cast(), ptr::from_mut(&mut r).cast());
    r.corner.x = r.corner.x.wrapping_add(x);
    if r.corner.x < 0 {
        r.extent.width = r.extent.width.wrapping_add(r.corner.x);
        r.corner.x = 0;
    } else if i32::from(r.corner.x) + i32::from(r.extent.width) >= width() {
        r.extent.width = (width() - i32::from(r.corner.x)) as i16;
    }
    r.corner.y = r.corner.y.wrapping_add(y);
    if r.corner.y < 0 {
        r.extent.height = r.extent.height.wrapping_add(r.corner.y);
        r.corner.y = 0;
    } else if i32::from(r.corner.y) + i32::from(r.extent.height) >= height() {
        r.extent.height = (height() - i32::from(r.corner.y)) as i16;
    }
    RepairBackRect(ptr::from_mut(&mut r).cast());
}

/// # Safety
/// Game-thread only, with the star-map UI active.
pub unsafe fn ZoomStarMap(direction: i16) {
    if direction > 0 && zoomLevel < MAX_ZOOM_SHIFT {
        zoomLevel += 1;
        mapOrigin = cursorLoc;
        DrawStarMap(0, ptr::null_mut());
        SleepThread((ONE_SECOND / 8) as i32);
    } else if direction < 0 && zoomLevel > 0 {
        mapOrigin = if zoomLevel > 1 {
            cursorLoc
        } else {
            POINT { x: 4999, y: 4999 }
        };
        zoomLevel -= 1;
        DrawStarMap(0, ptr::null_mut());
        SleepThread((ONE_SECOND / 8) as i32);
    }
}

/// # Safety
/// Game-thread only. `newpt` is null or points to an absolute universe location.
#[no_mangle]
pub unsafe extern "C" fn UpdateCursorLocation(mut sx: i32, mut sy: i32, newpt: *const POINT) {
    let old = POINT {
        x: ux(cursorLoc.x),
        y: uy(cursorLoc.y),
    };
    let mut screen;
    if !newpt.is_null() {
        sx = 0;
        sy = 0;
        screen = POINT {
            x: ux((*newpt).x),
            y: uy((*newpt).y),
        };
        cursorLoc = *newpt;
    } else {
        screen = POINT {
            x: (i32::from(old.x) + sx) as i16,
            y: (i32::from(old.y) + sy) as i16,
        };
    }
    if sx != 0 {
        cursorLoc.x = (i32::from(projection().inverse_x(screen.x)) - sx) as i16;
        while ux(cursorLoc.x) == old.x {
            cursorLoc.x = (i32::from(cursorLoc.x) + sx) as i16;
        }
        cursorLoc.x = cursorLoc.x.clamp(0, 9999);
        screen.x = ux(cursorLoc.x);
    }
    if sy != 0 {
        cursorLoc.y = (i32::from(projection().inverse_y(screen.y)) + sy) as i16;
        while uy(cursorLoc.y) == old.y {
            cursorLoc.y = (i32::from(cursorLoc.y) - sy) as i16;
        }
        cursorLoc.y = cursorLoc.y.clamp(0, 9999);
        screen.y = uy(cursorLoc.y);
    }
    if screen.x < 0
        || screen.y < 0
        || i32::from(screen.x) >= width()
        || i32::from(screen.y) >= height()
    {
        mapOrigin = cursorLoc;
        DrawStarMap(0, ptr::null_mut());
    } else {
        erase_cursor(old.x, old.y);
        draw_cursor(screen.x, screen.y);
    }
}

/// # Safety
/// Game-thread only. `previous` points to a writable 256-byte C string buffer.
#[no_mangle]
pub unsafe extern "C" fn UpdateCursorInfo(previous: *mut c_char) {
    let mut text = [0 as c_char; 256];
    let screen = POINT {
        x: ux(cursorLoc.x),
        y: uy(cursorLoc.y),
    };
    let mut last = ptr::null_mut();
    let mut best: *mut CStarDesc = ptr::null_mut();
    loop {
        last = FindStar(last, ptr::addr_of_mut!(cursorLoc).cast(), 75, 75);
        if last.is_null() {
            break;
        }
        let p = (*last).star_pt;
        if ux(p.x) == screen.x
            && uy(p.y) == screen.y
            && (best.is_null() || ((*last).star_type & 0x18) >= ((*best).star_type & 0x18))
        {
            best = last;
        }
    }
    if !best.is_null() {
        cursorLoc = point((*best).star_pt);
        GetClusterName(best, text.as_mut_ptr());
    } else {
        cursorLoc = POINT {
            x: projection().inverse_x(screen.x).clamp(0, 9999),
            y: projection().inverse_y(screen.y).clamp(0, 9999),
        };
    }
    if portal_open() {
        let p = if which_space() <= 1 {
            ARILOU_SPACE
        } else {
            QUASI_SPACE
        };
        if ux(p.x) == screen.x && uy(p.y) == screen.y {
            cursorLoc = p;
            unicode::utf8_string_copy(
                text.as_mut_ptr().cast(),
                text.len(),
                game_string(132).cast(),
            );
        }
    }
    DrawHyperCoords(cursorLoc);
    if search::strcmp(text.as_ptr(), previous) != 0 {
        search::strcpy(previous, text.as_ptr());
        DrawSISMessage(text.as_ptr());
    }
}

fn fuel_requirement(from: POINT, to: POINT, space: u8, sqrt: impl FnOnce(u32) -> u16) -> u16 {
    let x = from.x.wrapping_sub(to.x);
    let y = from.y.wrapping_sub(to.y);
    let f = (c_long::from(x) * c_long::from(x) + c_long::from(y) * c_long::from(y)) as u32;
    if f == 0 || space > 1 {
        0
    } else {
        sqrt(f).wrapping_add(100 / 20)
    }
}

/// # Safety
/// Game-thread only, with the star-map status UI active.
pub unsafe fn UpdateFuelRequirement() {
    let fuel = fuel_requirement(current_location(), cursorLoc, which_space(), |f| {
        crate::math::square_root(f)
    });
    let mut buf = [0 as c_char; 80];
    sprintf(
        buf.as_mut_ptr(),
        c"%s %u.%u".as_ptr(),
        game_string(439),
        u32::from(fuel / 100),
        u32::from((fuel % 100) / 10),
    );
    DrawStatusMessage(buf.as_ptr());
}

fn repair_rectangle(last: RECT, next: RECT, width: i32, height: i32) -> RECT {
    let mut r = RECT {
        corner: POINT {
            x: (i32::from(next.corner.x) + i32::from(next.extent.width >> 1) - 25) as i16,
            y: (i32::from(next.corner.y) + i32::from(next.extent.height >> 1) - 9) as i16,
        },
        extent: EXTENT {
            width: 50,
            height: 9,
        },
    };
    if r.corner.x < 0 {
        r.corner.x = 0;
    } else if i32::from(r.corner.x) + 50 >= width {
        r.corner.x = (width - 50) as i16;
    }
    if r.corner.y < 0 {
        r.corner.y = 0;
    } else if i32::from(r.corner.y) + 9 >= height {
        r.corner.y = (height - 9) as i16;
    }
    r = box_union(last, r);
    r = box_union(next, r);
    if r.corner.x < 0 {
        r.extent.width = r.extent.width.wrapping_add(r.corner.x);
        r.corner.x = 0;
    }
    if i32::from(r.corner.x) + i32::from(r.extent.width) > width {
        r.extent.width = (width - i32::from(r.corner.x)) as i16;
    }
    if r.corner.y < 0 {
        r.extent.height = r.extent.height.wrapping_add(r.corner.y);
        r.corner.y = 0;
    }
    if i32::from(r.corner.y) + i32::from(r.extent.height) > height {
        r.extent.height = (height - i32::from(r.corner.y)) as i16;
    }
    r.extent.height = r.extent.height.wrapping_add(r.corner.y & 1);
    r.corner.y &= !1;
    r
}
unsafe fn repair_map(race: u16, last: &mut RECT, next: RECT) {
    let mut r = repair_rectangle(*last, next, width(), height());
    *last = next;
    DrawStarMap(race, &mut r);
}
unsafe fn interrupted(button: &mut bool, interrupted: &mut bool, drawn: &mut bool) -> bool {
    if *button {
        if AnyButtonPress(1) == 0 {
            *button = false;
        }
    } else {
        *interrupted = *interrupted || AnyButtonPress(1) != 0;
        if *interrupted {
            *drawn = true;
            return true;
        }
    }
    false
}
fn enlarge(r: &mut RECT) {
    r.extent.width = r.extent.width.wrapping_add(1);
    r.extent.height = r.extent.height.wrapping_add(1);
}

unsafe fn update_map() {
    FlushInput();
    let mut button = true;
    let mut drawn = false;
    let mut was_interrupted = false;
    let mut index = 1u16;
    let mut handle = (*fleet_queue()).head;
    while !handle.is_null() {
        let fleet = lock_fleet(handle);
        let next = (*fleet).succ;
        interrupted(&mut button, &mut was_interrupted, &mut drawn);
        if (*fleet).known_strength != 0 {
            let dx = (*fleet).loc.x.wrapping_sub((*fleet).known_loc.x);
            let dy = (*fleet).loc.y.wrapping_sub((*fleet).known_loc.y);
            if dx != 0 || dy != 0 {
                let mut walk = LineWalk::new(dx, dy);
                if !drawn {
                    DrawStarMap(u16::MAX, ptr::null_mut());
                    drawn = true;
                }
                let (initial, mut last) = get_sphere_rect(fleet);
                enlarge(&mut last);
                let mut visible = false;
                let mut final_r = zero_rect();
                let mut canceled = false;
                loop {
                    let (sphere, mut r) = loop {
                        let mut loc = (*fleet).known_loc;
                        walk.step(&mut loc);
                        (*fleet).known_loc = loc;
                        let pair = get_sphere_rect(fleet);
                        let before = walk.delta;
                        walk.delta = walk.delta.wrapping_sub(1);
                        if before == 0
                            || (walk.delta & 0x1f == 0
                                && (initial.corner.x != pair.0.corner.x
                                    || initial.corner.y != pair.0.corner.y))
                        {
                            break pair;
                        }
                    };
                    if interrupted(&mut button, &mut was_interrupted, &mut drawn) {
                        canceled = true;
                        break;
                    }
                    enlarge(&mut r);
                    if initial.corner.x != sphere.corner.x || initial.corner.y != sphere.corner.y {
                        visible = true;
                        repair_map(index, &mut last, r);
                    }
                    final_r = r;
                    if walk.delta < 0 {
                        break;
                    }
                }
                if !canceled && visible {
                    repair_map(u16::MAX, &mut last, final_r);
                }
                (*fleet).known_loc = (*fleet).loc;
            }
            let mut delta =
                (i32::from((*fleet).actual_strength) - i32::from((*fleet).known_strength)) as i16;
            if delta != 0 {
                if !drawn {
                    DrawStarMap(u16::MAX, ptr::null_mut());
                    drawn = true;
                }
                let dx = if delta > 0 {
                    1i16
                } else {
                    delta = delta.wrapping_neg();
                    -1i16
                };
                delta = delta.wrapping_sub(1);
                let (initial, mut last) = get_sphere_rect(fleet);
                enlarge(&mut last);
                let mut visible = false;
                let mut final_r = zero_rect();
                let mut final_sphere = initial;
                let mut canceled = false;
                loop {
                    let (sphere, mut r) = loop {
                        (*fleet).known_strength = (*fleet).known_strength.wrapping_add(dx as u16);
                        let pair = get_sphere_rect(fleet);
                        let before = delta;
                        delta = delta.wrapping_sub(1);
                        if before == 0
                            || (delta & 0xf == 0 && initial.extent.height != pair.0.extent.height)
                        {
                            break pair;
                        }
                    };
                    if interrupted(&mut button, &mut was_interrupted, &mut drawn) {
                        canceled = true;
                        break;
                    }
                    enlarge(&mut r);
                    if initial.extent.height != sphere.extent.height {
                        visible = true;
                        repair_map(index, &mut last, r);
                    }
                    final_r = r;
                    final_sphere = sphere;
                    if delta < 0 {
                        break;
                    }
                }
                if !canceled && (visible || initial.extent.width != final_sphere.extent.width) {
                    repair_map(u16::MAX, &mut last, final_r);
                }
                (*fleet).known_strength = (*fleet).actual_strength;
            }
        }
        unlock_fleet(handle);
        handle = next;
        index += 1;
    }
}

/// # Safety
/// Game-thread only, with game globals and native UI resources initialized.
#[no_mangle]
pub unsafe extern "C" fn StarMap() -> u32 {
    let mut menu: MENU_STATE = std::mem::zeroed();
    zoomLevel = 0;
    mapOrigin = POINT { x: 4999, y: 4999 };
    StarMapFrame = pixmap_abi::SetAbsFrameIndex(MiscDataFrame.cast(), 48).cast();
    let universe = current_location();
    cursorLoc = GlobData.Game_state.autopilot;
    if cursorLoc.x == -1 && cursorLoc.y == -1 {
        cursorLoc = universe;
    }
    menu.InputFunc = Some(DoMoveCursor);
    menu.Initialized = 0;
    transition_pending = true;
    if which_space() <= 1 {
        update_map();
    }
    DrawStarMap(0, (-1isize) as *mut RECT);
    transition_pending = false;
    BatchGraphics();
    let old_context = SetContext(SpaceContext);
    let mut clip = zero_rect();
    GetContextClipRect(&mut clip);
    SetContext(old_context);
    LoadIntoExtraScreen(ptr::from_mut(&mut clip).cast());
    draw_cursor(ux(cursorLoc.x), uy(cursorLoc.y));
    UnbatchGraphics();
    SetMenuSounds(0, 0);
    SetMenuRepeatDelay(ONE_SECOND / 60, ONE_SECOND / 8, ONE_SECOND / 120, 1);
    DoInput(ptr::from_mut(&mut menu).cast(), 0);
    SetMenuSounds(15, 16);
    SetDefaultMenuRepeatDelay();
    DrawHyperCoords(universe);
    DrawSISMessage(ptr::null());
    DrawStatusMessage(ptr::null());
    if GlobData.Game_state.autopilot.x == universe.x
        && GlobData.Game_state.autopilot.y == universe.y
    {
        GlobData.Game_state.autopilot = POINT { x: -1, y: -1 };
    }
    u32::from(GlobData.Game_state.autopilot.x != -1 && GlobData.Game_state.autopilot.y != -1)
}

#[cfg(test)]
#[path = "pstarmap/tests.rs"]
mod tests;
