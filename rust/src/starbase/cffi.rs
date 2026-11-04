//! Graphics operations shared by native game menus.

use crate::engine::Kernel;
use crate::graphics::context_abi::SetContextClipRect;
use std::ffi::{c_int, c_void, CStr, CString};

use super::gfx::{
    Align, ContextHandle, ContextId, FlashTarget, FontEffectHandle, FontHandle, FontId, FrameId,
    Gfx, Point, Rect, Rgba, Stamp, TextDraw,
};

use crate::comm::locdata::{CColor, CPoint, CRect, CStamp};
use crate::comm::sis_graphics::CText;
use crate::engine::battle::link_queue::{LinkId, LinkQueue};
use crate::engine::world::queues::{Fleet, Fragment};
use crate::mainloop::restart_menu::c_extern as base;

/// `SFR_MENU_3DO` is `(RECT*)~0L`.
const SFR_MENU_3DO: usize = usize::MAX;
/// `SFR_MENU_ANY` is `(RECT*)~1L`.
const SFR_MENU_ANY: usize = usize::MAX - 1;

#[cfg(not(test))]
use crate::ui::sis::post_update_flash_rect;
#[cfg(test)]
extern "C" {
    fn post_update_flash_rect();
}
#[cfg(not(test))]
use crate::ui::sis::pre_update_flash_rect;
#[cfg(test)]
extern "C" {
    fn pre_update_flash_rect();
}

use crate::graphics::context_abi::SetContextForeGroundColor;

use crate::graphics::font_abi::SetContextFont;

use crate::graphics::context_abi::SetContextFontEffect;

use crate::graphics::font_abi::GetContextFontLeading;

use crate::graphics::font_abi::font_DrawText;

use crate::graphics::frame_abi::DrawFilledRectangle;

use crate::graphics::frame_abi::DrawRectangle;

use crate::graphics::frame_abi::DrawPoint;

use crate::graphics::frame_abi::DrawStamp;

use crate::graphics::frame_abi::DrawFilledStamp;

use crate::graphics::pixmap_abi::SetRelFrameIndex;

use crate::graphics::pixmap_abi::IncFrameIndex;

use crate::graphics::pixmap_abi::DecFrameIndex;

use crate::graphics::pixmap_abi::GetFrameCount;

use crate::graphics::pixmap_abi::GetFrameIndex;

use crate::ui::sis::SetFlashRect;

use crate::strings::string_table::SetAbsStringTableIndex;

use crate::strings::string_table::GetStringAddress;

extern "C" {
    static mut ScreenWidth: c_int;
    static mut ScreenHeight: c_int;

}
fn to_c_color(color: Rgba) -> CColor {
    CColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn from_c_color(color: CColor) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn to_c_point(point: Point) -> CPoint {
    CPoint {
        x: point.x,
        y: point.y,
    }
}

fn to_c_rect(rect: Rect) -> CRect {
    CRect {
        corner: to_c_point(rect.corner),
        width: rect.width,
        height: rect.height,
    }
}

fn to_c_stamp(stamp: Stamp) -> CStamp {
    CStamp {
        origin: to_c_point(stamp.origin),
        frame: stamp.frame as *mut c_void,
    }
}

fn c_string(text: &str) -> CString {
    CString::new(text).expect("game text has no interior NUL byte")
}

/// The implementation backed by the running game.
pub struct CffiPlatform;

impl CffiPlatform {
    fn context_ptr(ctx: ContextId) -> *mut c_void {
        // SAFETY: the context globals are set once at startup, before any
        // screen runs, and only read afterwards.
        unsafe {
            let kernel = Kernel::current();
            match ctx {
                ContextId::Screen => (std::ptr::addr_of!((**kernel).screen_context).read()).cast(),
                ContextId::Status => (std::ptr::addr_of!((**kernel).status_context).read()).cast(),
                ContextId::Space => std::ptr::addr_of!((**kernel).space_context).read().cast(),
            }
        }
    }

    fn font_ptr(font: FontId) -> *mut c_void {
        // SAFETY: the font globals are set once at startup.
        unsafe {
            let kernel = Kernel::current();
            match font {
                FontId::StarCon => std::ptr::addr_of!((**kernel).star_con_font).read().cast(),
                FontId::Tiny => std::ptr::addr_of!((**kernel).tiny_font).read().cast(),
                FontId::Micro => std::ptr::addr_of!((**kernel).micro_font).read().cast(),
            }
        }
    }
}

impl Gfx for CffiPlatform {
    fn screen_size(&self) -> (i32, i32) {
        // SAFETY: set at startup, read-only afterwards.
        unsafe {
            (
                std::ptr::addr_of!(ScreenWidth).read(),
                std::ptr::addr_of!(ScreenHeight).read(),
            )
        }
    }

    fn set_context(&self, ctx: ContextId) -> ContextHandle {
        // SAFETY: the pointer is one of the game's context objects.
        ContextHandle(unsafe { base::SetContext((Self::context_ptr(ctx)).cast()) } as usize)
    }

    fn restore_context(&self, previous: ContextHandle) {
        // SAFETY: `previous` was returned by `SetContext`.
        unsafe { base::SetContext((previous.0 as *mut c_void).cast()) };
    }

    fn context_clip_rect(&self) -> Rect {
        use crate::c_bindings::global_state as c;

        let mut rect = c::RECT {
            corner: c::POINT { x: 0, y: 0 },
            extent: c::EXTENT {
                width: 0,
                height: 0,
            },
        };
        // SAFETY: `rect` is a valid out-parameter.
        unsafe { c::GetContextClipRect(&mut rect) };
        Rect {
            corner: Point {
                x: rect.corner.x,
                y: rect.corner.y,
            },
            width: rect.extent.width,
            height: rect.extent.height,
        }
    }

    fn set_fg_color(&self, color: Rgba) -> Rgba {
        // SAFETY: plain value call into the graphics context.
        from_c_color(unsafe { SetContextForeGroundColor(to_c_color(color)) })
    }

    fn set_bg_color(&self, color: Rgba) -> Rgba {
        let color = base::Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        };
        // SAFETY: plain value call into the graphics context.
        let old = unsafe { base::SetContextBackGroundColor(color) };
        Rgba {
            r: old.r,
            g: old.g,
            b: old.b,
            a: old.a,
        }
    }

    fn set_font(&self, font: FontId) -> FontHandle {
        // SAFETY: the pointer is one of the game's font objects.
        FontHandle(unsafe { SetContextFont((Self::font_ptr(font)).cast()) } as usize)
    }

    fn restore_font(&self, previous: FontHandle) {
        // SAFETY: `previous` was returned by `SetContextFont`.
        unsafe { SetContextFont((previous.0 as *mut c_void).cast()) };
    }

    fn set_font_effect(&self, grad_index: Option<u16>) -> FontEffectHandle {
        // SAFETY: `FontGradFrame` is set at startup; the effect frame is
        // owned by the game.
        let effect = match grad_index {
            Some(index) => unsafe {
                let kernel = Kernel::current();
                base::SetAbsFrameIndex(
                    (std::ptr::addr_of!((**kernel).font_grad_frame).read()).cast(),
                    index,
                )
            },
            None => std::ptr::null_mut(),
        };
        // SAFETY: plain call into the graphics context.
        FontEffectHandle(unsafe { SetContextFontEffect(effect) } as usize)
    }

    fn restore_font_effect(&self, previous: FontEffectHandle) {
        // SAFETY: `previous` was returned by `SetContextFontEffect`.
        unsafe { SetContextFontEffect((previous.0 as *mut c_void).cast()) };
    }

    fn font_leading(&self) -> i16 {
        let mut leading = 0;
        // SAFETY: `leading` is a valid out-parameter.
        unsafe { GetContextFontLeading(&mut leading) };
        leading
    }

    fn set_context_clip_rect(&self, rect: Rect) {
        let mut rect = to_c_rect(rect);
        // SAFETY: `rect` is valid for the call and copied by it.
        unsafe { SetContextClipRect((&mut rect as *mut CRect).cast()) };
    }

    fn fill_rect(&self, rect: Rect) {
        let mut rect = to_c_rect(rect);
        // SAFETY: `rect` is valid for the call.
        unsafe { DrawFilledRectangle(std::ptr::from_mut(&mut rect).cast()) };
    }

    fn draw_rect(&self, rect: Rect) {
        let mut rect = to_c_rect(rect);
        // SAFETY: `rect` is valid for the call.
        unsafe { DrawRectangle(std::ptr::from_mut(&mut rect).cast()) };
    }

    fn draw_point(&self, point: Point) {
        let mut point = to_c_point(point);
        // SAFETY: `point` is valid for the call.
        unsafe { DrawPoint(&mut point) };
    }

    fn draw_stamp(&self, stamp: Stamp) {
        let mut stamp = to_c_stamp(stamp);
        // SAFETY: `stamp` is valid for the call.
        unsafe { DrawStamp(std::ptr::from_mut(&mut stamp).cast()) };
    }

    fn draw_filled_stamp(&self, stamp: Stamp) {
        let mut stamp = to_c_stamp(stamp);
        // SAFETY: `stamp` is valid for the call.
        unsafe { DrawFilledStamp(std::ptr::from_mut(&mut stamp).cast()) };
    }

    fn draw_text(&self, text: &TextDraw) {
        let string = c_string(&text.text);
        let align = match text.align {
            Align::Left => 0,
            Align::Center => 1,
            Align::Right => 2,
        };
        let mut c_text = CText::default();
        c_text.baseline = to_c_point(text.baseline);
        c_text.p_str = string.as_ptr();
        c_text.align = align;
        c_text.char_count = u16::MAX;
        // SAFETY: `c_text` and `string` are valid for the call.
        unsafe { font_DrawText(std::ptr::from_mut(&mut c_text).cast()) };
    }

    fn abs_frame(&self, frame: FrameId, index: u16) -> FrameId {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { base::SetAbsFrameIndex((frame as *mut c_void).cast(), index) as FrameId }
    }

    fn rel_frame(&self, frame: FrameId, offset: i16) -> FrameId {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { SetRelFrameIndex((frame as *mut c_void).cast(), offset) as FrameId }
    }

    fn inc_frame(&self, frame: FrameId) -> FrameId {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { IncFrameIndex((frame as *mut c_void).cast()) as FrameId }
    }

    fn dec_frame(&self, frame: FrameId) -> FrameId {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { DecFrameIndex((frame as *mut c_void).cast()) as FrameId }
    }

    fn frame_count(&self, frame: FrameId) -> u16 {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { GetFrameCount((frame as *mut c_void).cast()) }
    }

    fn frame_index(&self, frame: FrameId) -> u16 {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { GetFrameIndex((frame as *mut c_void).cast()) }
    }

    fn play_frame(&self) -> FrameId {
        // SAFETY: `PlayFrame` is set at startup.
        unsafe {
            let kernel = Kernel::current();
            std::ptr::addr_of!((**kernel).play_frame).read() as FrameId
        }
    }

    fn batch(&self) {
        // SAFETY: balanced with `unbatch` by the callers.
        unsafe { base::BatchGraphics() };
    }

    fn unbatch(&self) {
        // SAFETY: balanced with `batch` by the callers.
        unsafe { base::UnbatchGraphics() };
    }

    fn clear_drawable(&self) {
        // SAFETY: plain call into the current context.
        unsafe { base::ClearDrawable() };
    }

    fn set_transition_source_none(&self) {
        // SAFETY: a null source selects the whole screen.
        unsafe { base::SetTransitionSource(std::ptr::null_mut()) };
    }

    fn screen_transition(&self, kind: i32) {
        // SAFETY: a null rectangle selects the whole screen.
        unsafe { base::ScreenTransition(kind, std::ptr::null_mut()) };
    }

    fn set_flash(&self, target: FlashTarget) {
        match target {
            FlashTarget::Off => {
                // SAFETY: NULL stops the flash.
                unsafe { SetFlashRect(std::ptr::null()) };
            }
            FlashTarget::Menu3do => {
                // SAFETY: `SFR_MENU_3DO` is the C sentinel pointer `~0`.
                unsafe { SetFlashRect((SFR_MENU_3DO as *const c_void).cast()) };
            }
            FlashTarget::MenuAny => {
                // SAFETY: `SFR_MENU_ANY` is the C sentinel pointer `~1`.
                unsafe { SetFlashRect((SFR_MENU_ANY as *const c_void).cast()) };
            }
            FlashTarget::Rect(rect) => {
                let rect = to_c_rect(rect);
                // SAFETY: the rectangle is copied by the call.
                unsafe { SetFlashRect(std::ptr::from_ref(&rect).cast()) };
            }
        }
    }

    fn pre_update_flash(&self) {
        // SAFETY: balanced with `post_update_flash` by the callers.
        unsafe { pre_update_flash_rect() };
    }

    fn post_update_flash(&self) {
        // SAFETY: balanced with `pre_update_flash` by the callers.
        unsafe { post_update_flash_rect() };
    }

    fn game_string(&self, index: i32) -> String {
        // SAFETY: `GameStrings` is loaded before any screen runs; the
        // returned string lives as long as the table.
        unsafe {
            let kernel = Kernel::current();
            let index = u16::try_from(index).expect("game string indexes fit a COUNT");
            let entry = SetAbsStringTableIndex(
                (std::ptr::addr_of!((**kernel).game_strings).read()).cast(),
                index,
            );
            let text = GetStringAddress(entry);
            if text.is_null() {
                return String::new();
            }
            CStr::from_ptr(text).to_string_lossy().into_owned()
        }
    }
}
