//! Widget rendering and event ABI ported from `widgets.c` (GPL-2.0-or-later).
use super::native_abi::*;
use super::{
    context_abi::{SetContextFontEffect, SetContextForeGroundColor},
    font_abi::{font_DrawText, GetContextFontLeading, SetContextFont, TextRect},
    frame_abi::{DrawFilledRectangle, DrawStamp},
};
use std::ffi::c_char;
use std::ptr;

const EVENT_UP: i32 = 0;
const EVENT_DOWN: i32 = 1;
const EVENT_LEFT: i32 = 2;
const EVENT_RIGHT: i32 = 3;
const EVENT_SELECT: i32 = 4;
const EVENT_CANCEL: i32 = 5;
const EVENT_DELETE: i32 = 6;
const WTE_EDITING: u32 = 1;
const WTE_BLOCKCUR: u32 = 2;
const TRUE: i32 = 1;
const FALSE: i32 = 0;

const fn color(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
const ACTIVE: Color = color(0x1f, 0x1f, 0x00);
const INACTIVE: Color = color(0x18, 0x18, 0x1f);
const INACTIVE_SELECTED: Color = color(0x1f, 0x1f, 0x1f);
const CURSOR: Color = color(0, 0, 0);
const DIALOG_TEXT: Color = color(0, 0, 0);
static mut WIN_BG: Color = color(0x18, 0x18, 0x1f);
static mut WIN_MEDIUM: Color = color(0x10, 0x10, 0x18);
static mut WIN_DARK: Color = color(0x08, 0x08, 0x10);
static mut CUR_FONT: FONT = ptr::null_mut();
#[no_mangle]
pub static mut widget_focus: *mut WIDGET = ptr::null_mut();

extern "C" {
    static mut ScreenWidth: i32;
    static mut ScreenHeight: i32;
    fn BatchGraphics();
    fn UnbatchGraphics();
}

unsafe fn draw_text(text: &mut TEXT) {
    font_DrawText(text);
}
unsafe fn restore_font(old_effect: FRAME, old_font: FONT, old_color: Color) {
    SetContextFontEffect(old_effect);
    if !old_font.is_null() {
        SetContextFont(old_font);
    }
    SetContextForeGroundColor(old_color);
}
unsafe fn draw_tooltips(numlines: i32, tips: *const *const c_char) {
    let mut r: RECT = std::mem::zeroed();
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let oldfont = if CUR_FONT.is_null() {
        ptr::null_mut()
    } else {
        SetContextFont(CUR_FONT)
    };
    let oldtext = SetContextForeGroundColor(INACTIVE_SELECTED);
    r.corner.x = 2;
    r.corner.y = 2;
    r.extent.width = (ScreenWidth - 4) as i16;
    r.extent.height = (ScreenHeight - 4) as i16;
    let mut t: TEXT = std::mem::zeroed();
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.CharCount = u16::MAX;
    t.baseline.x = r.corner.x + (r.extent.width >> 1);
    t.baseline.y = r.corner.y + (r.extent.height - 8 - 8 * numlines as i16);
    for i in 0..numlines {
        t.pStr = *tips.offset(i as isize) as *const _;
        draw_text(&mut t);
        t.baseline.y += 8;
    }
    restore_font(old_effect, oldfont, oldtext);
}

/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn DrawShadowedBox(r: *mut RECT, bg: Color, dark: Color, medium: Color) {
    let mut t = *r;
    BatchGraphics();
    t.corner.x = t.corner.x.wrapping_sub(2);
    t.corner.y = t.corner.y.wrapping_sub(2);
    t.extent.width = t.extent.width.wrapping_add(4);
    t.extent.height = t.extent.height.wrapping_add(4);
    let old = SetContextForeGroundColor(dark);
    DrawFilledRectangle(&mut t);
    t.corner.x += 2;
    t.corner.y += 2;
    t.extent.width -= 2;
    t.extent.height -= 2;
    SetContextForeGroundColor(medium);
    DrawFilledRectangle(&mut t);
    t.corner.x -= 1;
    t.corner.y = t.corner.y.wrapping_add((*r).extent.height).wrapping_add(1);
    t.extent.height = 1;
    DrawFilledRectangle(&mut t);
    t.corner.x = t.corner.x.wrapping_add((*r).extent.width).wrapping_add(2);
    t.corner.y = t.corner.y.wrapping_sub((*r).extent.height).wrapping_sub(2);
    t.extent.width = 1;
    DrawFilledRectangle(&mut t);
    SetContextForeGroundColor(bg);
    DrawFilledRectangle(r);
    SetContextForeGroundColor(old);
    UnbatchGraphics();
}

/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn DrawLabelAsWindow(label: *mut WIDGET_LABEL, window_rect: *mut RECT) {
    let oldfg = SetContextForeGroundColor(DIALOG_TEXT);
    let mut oldfont = ptr::null_mut();
    let old_effect = SetContextFontEffect(ptr::null_mut());
    if !CUR_FONT.is_null() {
        oldfont = SetContextFont(CUR_FONT);
    }
    let win_h = ((*label).height.unwrap())(label.cast()).wrapping_add(16);
    let mut win_w = 0;
    for i in 0..(*label).line_count {
        let len =
            crate::strings::unicode::utf8_string_count((*(*label).lines.offset(i as isize)).cast());
        if len as i32 > win_w {
            win_w = len as i32;
        }
    }
    win_w = win_w.wrapping_mul(6).wrapping_add(16);
    BatchGraphics();
    let mut r: RECT = std::mem::zeroed();
    r.corner.x = ((ScreenWidth - win_w) >> 1) as i16;
    r.corner.y = ((ScreenHeight - win_h) >> 1) as i16;
    r.extent.width = win_w as i16;
    r.extent.height = win_h as i16;
    DrawShadowedBox(&mut r, WIN_BG, WIN_DARK, WIN_MEDIUM);
    let mut t: TEXT = std::mem::zeroed();
    t.baseline.x = r.corner.x + (r.extent.width >> 1);
    t.baseline.y = r.corner.y + 16;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.CharCount = u16::MAX;
    for i in 0..(*label).line_count {
        t.pStr = *(*label).lines.offset(i as isize) as *const _;
        t.CharCount = u16::MAX;
        draw_text(&mut t);
        t.baseline.y += 8;
    }
    UnbatchGraphics();
    SetContextFontEffect(old_effect);
    if !oldfont.is_null() {
        SetContextFont(oldfont);
    }
    SetContextForeGroundColor(oldfg);
    if !window_rect.is_null() {
        (*window_rect).corner.x = r.corner.x - 2;
        (*window_rect).corner.y = r.corner.y - 2;
        (*window_rect).extent.width = r.extent.width + 4;
        (*window_rect).extent.height = r.extent.height + 4;
    }
}

/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_SetWindowColors(bg: Color, dark: Color, medium: Color) {
    WIN_BG = bg;
    WIN_DARK = dark;
    WIN_MEDIUM = medium;
}
/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_SetFont(new_font: FONT) -> FONT {
    let old = CUR_FONT;
    CUR_FONT = new_font;
    old
}

/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_DrawMenuScreen(base: *mut WIDGET, _x: i32, _y: i32) {
    let self_ = base as *mut WIDGET_MENU_SCREEN;
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let oldfont = if CUR_FONT.is_null() {
        ptr::null_mut()
    } else {
        SetContextFont(CUR_FONT)
    };
    let mut r: RECT = std::mem::zeroed();
    r.corner.x = 2;
    r.corner.y = 2;
    r.extent.width = (ScreenWidth - 4) as i16;
    r.extent.height = (ScreenHeight - 4) as i16;
    let mut t: TEXT = std::mem::zeroed();
    t.baseline.x = r.corner.x + (r.extent.width >> 1);
    t.baseline.y = r.corner.y + 8;
    t.pStr = (*self_).title as *const _;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.CharCount = u16::MAX;
    DrawStamp(&mut (*self_).bgStamp);
    let oldtext = SetContextForeGroundColor(INACTIVE_SELECTED);
    draw_text(&mut t);
    t.baseline.y += 8;
    t.pStr = (*self_).subtitle as *const _;
    draw_text(&mut t);
    let mut height = 0;
    for i in 0..(*self_).num_children {
        let child = *(*self_).child.offset(i as isize);
        height += ((*child).height.unwrap())(child) + 8;
    }
    height -= 8;
    let mut widget_y = ((ScreenHeight - height) >> 1) as i16;
    for i in 0..(*self_).num_children {
        let child = *(*self_).child.offset(i as isize);
        ((*child).draw.unwrap())(child, 0, widget_y as i32);
        widget_y = widget_y.wrapping_add((((*child).height.unwrap())(child) + 8) as i16);
    }
    restore_font(old_effect, oldfont, oldtext);
}

/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_DrawChoice(base: *mut WIDGET, x: i32, y: i32) {
    let s = base as *mut WIDGET_CHOICE;
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let oldfont = if CUR_FONT.is_null() {
        ptr::null_mut()
    } else {
        SetContextFont(CUR_FONT)
    };
    let oldtext = SetContextForeGroundColor(if widget_focus == base {
        ACTIVE
    } else {
        INACTIVE_SELECTED
    });
    let mut t: TEXT = std::mem::zeroed();
    t.baseline.x = x as i16;
    t.baseline.y = y as i16;
    t.align = TEXT_ALIGN_ALIGN_LEFT;
    t.CharCount = u16::MAX;
    t.pStr = (*s).category as *const _;
    draw_text(&mut t);
    let home_x = t
        .baseline
        .x
        .wrapping_add(3 * (ScreenWidth / (((*s).maxcolumns + 1) * 2)) as i16);
    let home_y = t.baseline.y;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    for i in 0..(*s).numopts {
        t.baseline.x =
            home_x.wrapping_add(((i % 3) * (ScreenWidth / ((*s).maxcolumns + 1))) as i16);
        t.baseline.y = home_y.wrapping_add((8 * (i / 3)) as i16);
        let opt = &mut *(*s).options.offset(i as isize);
        t.pStr = opt.optname as *const _;
        if widget_focus == base && (*s).highlighted == i {
            SetContextForeGroundColor(ACTIVE);
            draw_tooltips(3, opt.tooltip.as_ptr());
        } else if i == (*s).selected {
            SetContextForeGroundColor(INACTIVE_SELECTED);
        } else {
            SetContextForeGroundColor(INACTIVE);
        }
        draw_text(&mut t);
    }
    restore_font(old_effect, oldfont, oldtext);
}
/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_DrawButton(base: *mut WIDGET, _x: i32, y: i32) {
    let s = base as *mut WIDGET_BUTTON;
    let oldfont = if CUR_FONT.is_null() {
        ptr::null_mut()
    } else {
        SetContextFont(CUR_FONT)
    };
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let mut t: TEXT = std::mem::zeroed();
    t.baseline.x = 160;
    t.baseline.y = y as i16;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.CharCount = u16::MAX;
    t.pStr = (*s).name as *const _;
    let oldtext = if widget_focus == base {
        draw_tooltips(3, (*s).tooltip.as_ptr());
        SetContextForeGroundColor(ACTIVE)
    } else {
        SetContextForeGroundColor(INACTIVE)
    };
    draw_text(&mut t);
    restore_font(old_effect, oldfont, oldtext);
}
/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_DrawLabel(base: *mut WIDGET, _x: i32, y: i32) {
    let s = base as *mut WIDGET_LABEL;
    let oldtext = SetContextForeGroundColor(INACTIVE_SELECTED);
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let oldfont = if CUR_FONT.is_null() {
        ptr::null_mut()
    } else {
        SetContextFont(CUR_FONT)
    };
    let mut t: TEXT = std::mem::zeroed();
    t.baseline.x = 160;
    t.baseline.y = y as i16;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.CharCount = u16::MAX;
    for i in 0..(*s).line_count {
        t.pStr = *(*s).lines.offset(i as isize) as *const _;
        draw_text(&mut t);
        t.baseline.y += 8;
    }
    restore_font(old_effect, oldfont, oldtext);
}
/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_DrawSlider(base: *mut WIDGET, x: i32, y: i32) {
    let s = base as *mut WIDGET_SLIDER;
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let oldfont = if CUR_FONT.is_null() {
        ptr::null_mut()
    } else {
        SetContextFont(CUR_FONT)
    };
    let tick = (ScreenWidth - x) / 8;
    let mut t: TEXT = std::mem::zeroed();
    t.baseline.x = x as i16;
    t.baseline.y = y as i16;
    t.align = TEXT_ALIGN_ALIGN_LEFT;
    t.CharCount = u16::MAX;
    t.pStr = (*s).category as *const _;
    let oldtext = if widget_focus == base {
        draw_tooltips(3, (*s).tooltip.as_ptr());
        SetContextForeGroundColor(ACTIVE)
    } else {
        SetContextForeGroundColor(INACTIVE_SELECTED)
    };
    draw_text(&mut t);
    let mut r: RECT = std::mem::zeroed();
    r.corner.x = t.baseline.x.wrapping_add(3 * tick as i16);
    r.corner.y = t.baseline.y - 4;
    r.extent.height = 2;
    r.extent.width = (3 * tick) as i16;
    DrawFilledRectangle(&mut r);
    r.extent.width = 3;
    r.extent.height = 8;
    r.corner.y = t.baseline.y - 7;
    r.corner.x = t
        .baseline
        .x
        .wrapping_add(
            (3 * tick + 3 * tick * ((*s).value - (*s).min) / ((*s).max - (*s).min)) as i16,
        )
        .wrapping_sub(1);
    DrawFilledRectangle(&mut r);
    ((*s).draw_value.unwrap())(
        s,
        t.baseline.x.wrapping_add(7 * tick as i16) as i32,
        t.baseline.y as i32,
    );
    restore_font(old_effect, oldfont, oldtext);
}
/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_Slider_DrawValue(s: *mut WIDGET_SLIDER, x: i32, y: i32) {
    let mut buf = [0i8; 16];
    let value = (*s).value;
    let bytes = value.to_string();
    for (i, b) in bytes.bytes().take(15).enumerate() {
        buf[i] = b as i8;
    }
    let mut t: TEXT = std::mem::zeroed();
    t.baseline.x = x as i16;
    t.baseline.y = y as i16;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    t.CharCount = u16::MAX;
    t.pStr = buf.as_ptr() as *const _;
    draw_text(&mut t);
}
/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_DrawTextEntry(base: *mut WIDGET, x: i32, y: i32) {
    let s = base as *mut WIDGET_TEXTENTRY;
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let oldfont = if CUR_FONT.is_null() {
        ptr::null_mut()
    } else {
        SetContextFont(CUR_FONT)
    };
    BatchGraphics();
    let oldtext;
    let mut t: TEXT = std::mem::zeroed();
    t.baseline.x = x as i16;
    t.baseline.y = y as i16;
    t.align = TEXT_ALIGN_ALIGN_LEFT;
    t.CharCount = u16::MAX;
    t.pStr = (*s).category as *const _;
    if widget_focus == base {
        SetContextForeGroundColor(ACTIVE);
    } else {
        SetContextForeGroundColor(INACTIVE_SELECTED);
    }
    draw_text(&mut t);
    (*s).value[49] = 0;
    t.baseline.y = y as i16;
    t.CharCount = crate::strings::unicode::utf8_string_count((*s).value.as_ptr().cast()) as u16;
    t.pStr = (*s).value.as_ptr() as *const _;
    if (*s).state & WTE_EDITING == 0 {
        t.baseline.x = 160;
        t.align = TEXT_ALIGN_ALIGN_CENTER;
        if widget_focus == base {
            oldtext = SetContextForeGroundColor(ACTIVE);
        } else {
            oldtext = SetContextForeGroundColor(INACTIVE);
        }
        draw_text(&mut t);
    } else {
        let mut text_r: RECT = std::mem::zeroed();
        let mut deltas = [0u8; 50];
        let mut r: RECT = std::mem::zeroed();
        let mut leading: i16 = 0;
        t.baseline.x = 90;
        t.align = TEXT_ALIGN_ALIGN_LEFT;
        GetContextFontLeading(&mut leading);
        r.corner.x = t.baseline.x - 1;
        r.corner.y = t.baseline.y.wrapping_sub(leading).wrapping_add(2);
        r.extent.width = (ScreenWidth - r.corner.x as i32 - 10) as i16;
        r.extent.height = leading + 2;
        TextRect(&mut t, &mut text_r, deltas.as_mut_ptr());
        oldtext = SetContextForeGroundColor(ACTIVE);
        DrawFilledRectangle(&mut r);
        let mut pdelta = deltas.as_ptr();
        for _ in 0..(*s).cursor_pos {
            r.corner.x = r.corner.x.wrapping_add(*pdelta as i16);
            pdelta = pdelta.add(1);
        }
        if (*s).cursor_pos < t.CharCount as i32 {
            r.corner.x -= 1;
        }
        if (*s).state & WTE_BLOCKCUR != 0 {
            if (*s).cursor_pos == t.CharCount as i32 {
                r.extent.width = 1;
            } else if (*s).cursor_pos + 1 == t.CharCount as i32 {
                r.extent.width = (*pdelta as i16) + 2;
            } else {
                r.extent.width = (*pdelta as i16) + 1;
            }
        } else {
            r.extent.width = 1;
        }
        r.corner.x += 1;
        r.corner.y += 1;
        r.extent.height -= 2;
        SetContextForeGroundColor(CURSOR);
        DrawFilledRectangle(&mut r);
        SetContextForeGroundColor(INACTIVE);
        draw_text(&mut t);
    }
    UnbatchGraphics();
    restore_font(old_effect, oldfont, oldtext);
}
/// # Safety
/// Widget records and callbacks must be live and operations serialized on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_DrawControlEntry(base: *mut WIDGET, x: i32, y: i32) {
    let s = base as *mut WIDGET_CONTROLENTRY;
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let oldfont = if CUR_FONT.is_null() {
        ptr::null_mut()
    } else {
        SetContextFont(CUR_FONT)
    };
    let oldtext = SetContextForeGroundColor(if widget_focus == base {
        ACTIVE
    } else {
        INACTIVE_SELECTED
    });
    let mut t: TEXT = std::mem::zeroed();
    t.baseline.x = x as i16;
    t.baseline.y = y as i16;
    t.align = TEXT_ALIGN_ALIGN_LEFT;
    t.CharCount = u16::MAX;
    t.pStr = (*s).category as *const _;
    draw_text(&mut t);
    let home_x = t.baseline.x.wrapping_add((ScreenWidth / 2) as i16);
    let home_y = t.baseline.y;
    t.align = TEXT_ALIGN_ALIGN_CENTER;
    for i in 0..2 {
        t.baseline.x = home_x.wrapping_add((i * (ScreenWidth / 3)) as i16);
        t.baseline.y = home_y + (8 * (i / 3)) as i16;
        let name = (*s).controlname[i as usize].as_ptr();
        t.pStr = if *name == 0 {
            c"---".as_ptr()
        } else {
            name as *const _
        };
        SetContextForeGroundColor(if widget_focus == base && (*s).highlighted == i {
            ACTIVE
        } else {
            INACTIVE_SELECTED
        });
        draw_text(&mut t);
    }
    restore_font(old_effect, oldfont, oldtext);
}

/// # Safety
/// Widget must be a live choice record on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_HeightChoice(s: *mut WIDGET) -> i32 {
    let n = (*(s as *mut WIDGET_CHOICE)).numopts;
    ((n + 2) / 3) * 8
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_HeightFullScreen(_: *mut WIDGET) -> i32 {
    ScreenHeight
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_HeightOneLine(_: *mut WIDGET) -> i32 {
    8
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_HeightLabel(s: *mut WIDGET) -> i32 {
    (*(s as *mut WIDGET_LABEL)).line_count * 8
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_WidthFullScreen(_: *mut WIDGET) -> i32 {
    ScreenWidth
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_ReceiveFocusSimple(s: *mut WIDGET, _event: i32) -> i32 {
    widget_focus = s;
    TRUE
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_ReceiveFocusChoice(s: *mut WIDGET, _event: i32) -> i32 {
    widget_focus = s;
    let c = s as *mut WIDGET_CHOICE;
    (*c).highlighted = (*c).selected;
    TRUE
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_ReceiveFocusControlEntry(s: *mut WIDGET, _event: i32) -> i32 {
    let c = s as *mut WIDGET_CONTROLENTRY;
    let mut old = 0;
    if (*widget_focus).tag == WIDGET_TYPE_WIDGET_TYPE_CONTROLENTRY {
        old = (*(widget_focus as *mut WIDGET_CONTROLENTRY)).highlighted;
    }
    widget_focus = s;
    (*c).highlighted = old;
    TRUE
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_ReceiveFocusMenuScreen(s: *mut WIDGET, event: i32) -> i32 {
    let m = s as *mut WIDGET_MENU_SCREEN;
    for i in 0..(*m).num_children {
        (**(*m).child.offset(i as isize)).parent = s;
    }
    let (mut x, dx, last) = if event == EVENT_UP {
        ((*m).num_children - 1, -1, -1)
    } else if event == EVENT_DOWN {
        (0, 1, (*m).num_children)
    } else {
        let c = *(*m).child.offset((*m).highlighted as isize);
        ((*c).receiveFocus.unwrap())(c, event);
        return TRUE;
    };
    while x != last {
        let c = *(*m).child.offset(x as isize);
        if ((*c).receiveFocus.unwrap())(c, event) != FALSE {
            (*m).highlighted = x;
            return TRUE;
        }
        x += dx;
    }
    FALSE
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_ReceiveFocusRefuseFocus(_: *mut WIDGET, _event: i32) -> i32 {
    FALSE
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_HandleEventIgnoreAll(_: *mut WIDGET, _event: i32) -> i32 {
    FALSE
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_HandleEventChoice(s: *mut WIDGET, event: i32) -> i32 {
    let c = s as *mut WIDGET_CHOICE;
    match event {
        EVENT_LEFT => {
            (*c).highlighted -= 1;
            if (*c).highlighted < 0 {
                (*c).highlighted = (*c).numopts - 1;
            }
            TRUE
        }
        EVENT_RIGHT => {
            (*c).highlighted += 1;
            if (*c).highlighted >= (*c).numopts {
                (*c).highlighted = 0;
            }
            TRUE
        }
        EVENT_SELECT => {
            let old = (*c).selected;
            (*c).selected = (*c).highlighted;
            if let Some(f) = (*c).onChange {
                f(c, old);
            }
            TRUE
        }
        _ => FALSE,
    }
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_HandleEventSlider(s: *mut WIDGET, event: i32) -> i32 {
    let c = s as *mut WIDGET_SLIDER;
    match event {
        EVENT_LEFT => {
            (*c).value = (*c).value.wrapping_sub((*c).step);
            if (*c).value < (*c).min {
                (*c).value = (*c).min;
            }
            TRUE
        }
        EVENT_RIGHT => {
            (*c).value = (*c).value.wrapping_add((*c).step);
            if (*c).value > (*c).max {
                (*c).value = (*c).max;
            }
            TRUE
        }
        _ => FALSE,
    }
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_HandleEventMenuScreen(s: *mut WIDGET, event: i32) -> i32 {
    let m = s as *mut WIDGET_MENU_SCREEN;
    let dx = match event {
        EVENT_UP => -1,
        EVENT_DOWN => 1,
        EVENT_CANCEL => {
            (*m).highlighted = (*m).num_children - 1;
            widget_focus = *(*m).child.offset((*m).highlighted as isize);
            let f = (*widget_focus).handleEvent.unwrap();
            return f(widget_focus, EVENT_SELECT);
        }
        _ => return FALSE,
    };
    let last = (*m).highlighted;
    let mut x = last + dx;
    while x != last {
        if x == -1 {
            x = (*m).num_children - 1;
        }
        if x == (*m).num_children {
            x = 0;
        }
        let child = *(*m).child.offset(x as isize);
        if ((*child).receiveFocus.unwrap())(child, event) != FALSE {
            (*m).highlighted = x;
            return TRUE;
        }
        x += dx;
    }
    FALSE
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_HandleEventTextEntry(s: *mut WIDGET, event: i32) -> i32 {
    let t = s as *mut WIDGET_TEXTENTRY;
    if event == EVENT_SELECT {
        if let Some(f) = (*t).handleEventSelect {
            return f(t);
        }
    }
    FALSE
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_HandleEventControlEntry(s: *mut WIDGET, event: i32) -> i32 {
    let c = s as *mut WIDGET_CONTROLENTRY;
    if event == EVENT_SELECT {
        if let Some(f) = (*c).onChange {
            f(c);
            return TRUE;
        }
    }
    if event == EVENT_DELETE {
        if let Some(f) = (*c).onDelete {
            f(c);
            return TRUE;
        }
    }
    if event == EVENT_RIGHT || event == EVENT_LEFT {
        (*c).highlighted = 1 - (*c).highlighted;
        return TRUE;
    }
    FALSE
}
/// # Safety
/// Widget records and callbacks must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn Widget_Event(event: i32) -> i32 {
    let mut w = widget_focus;
    while !w.is_null() {
        if ((*w).handleEvent.unwrap())(w, event) != FALSE {
            return TRUE;
        }
        w = (*w).parent;
    }
    FALSE
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};
    static OLD_CHOICE: AtomicI32 = AtomicI32::new(-1);
    unsafe extern "C" fn changed(choice: *mut WIDGET_CHOICE, old: i32) {
        assert_eq!((*choice).selected, (*choice).highlighted);
        OLD_CHOICE.store(old, Ordering::Relaxed);
    }
    unsafe extern "C" fn handles_select(_: *mut WIDGET, event: i32) -> i32 {
        i32::from(event == EVENT_SELECT)
    }
    #[test]
    #[serial_test::serial]
    fn unhandled_events_bubble_to_parent_and_focus_is_updated() {
        unsafe {
            let mut parent: WIDGET = std::mem::zeroed();
            parent.handleEvent = Some(handles_select);
            let mut child: WIDGET = std::mem::zeroed();
            child.parent = &mut parent;
            child.handleEvent = Some(Widget_HandleEventIgnoreAll);
            let previous = widget_focus;
            assert_eq!(Widget_ReceiveFocusSimple(&mut child, EVENT_DOWN), TRUE);
            let focused = widget_focus;
            assert_eq!(focused, ptr::addr_of_mut!(child));
            assert_eq!(Widget_Event(EVENT_SELECT), TRUE);
            assert_eq!(Widget_Event(EVENT_LEFT), FALSE);
            widget_focus = previous;
        }
    }
    #[test]
    fn choice_wraps_navigation_and_notifies_after_selection() {
        unsafe {
            let mut choice: WIDGET_CHOICE = std::mem::zeroed();
            choice.numopts = 3;
            choice.selected = 1;
            choice.onChange = Some(changed);
            let base = ptr::addr_of_mut!(choice).cast();
            assert_eq!(Widget_HandleEventChoice(base, EVENT_LEFT), TRUE);
            assert_eq!(choice.highlighted, 2);
            assert_eq!(Widget_HandleEventChoice(base, EVENT_SELECT), TRUE);
            assert_eq!(
                (choice.selected, OLD_CHOICE.load(Ordering::Relaxed)),
                (2, 1)
            );
            assert_eq!(Widget_HandleEventChoice(base, EVENT_RIGHT), TRUE);
            assert_eq!(choice.highlighted, 0);
            assert_eq!(Widget_HandleEventChoice(base, EVENT_CANCEL), FALSE);
            assert_eq!(choice.selected, 2);
        }
    }
    #[test]
    fn slider_clamps_both_ends_and_ignores_select() {
        unsafe {
            let mut slider: WIDGET_SLIDER = std::mem::zeroed();
            slider.min = 2;
            slider.max = 9;
            slider.step = 4;
            slider.value = 3;
            let base = ptr::addr_of_mut!(slider).cast();
            assert_eq!(Widget_HandleEventSlider(base, EVENT_LEFT), TRUE);
            assert_eq!(slider.value, 2);
            Widget_HandleEventSlider(base, EVENT_RIGHT);
            Widget_HandleEventSlider(base, EVENT_RIGHT);
            assert_eq!(slider.value, 9);
            assert_eq!(Widget_HandleEventSlider(base, EVENT_SELECT), FALSE);
            assert_eq!(slider.value, 9);
        }
    }
}
