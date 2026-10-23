//! Save failure popup, preserving save.c's measurement and screen restoration.
use crate::comm::locdata::{CColor, CRect, CStamp};
use crate::comm::sis_graphics::CText;
use crate::starbase::cffi::CffiPlatform;
use crate::starbase::gfx::{Align, ContextId, FontId, Gfx, Point, Rgba, Stamp, TextDraw};
use std::ffi::c_void;

extern "C" {
    fn TextRect(text: *mut CText, rect: *mut CRect, deltas: *mut u8) -> i32;
    fn SaveContextFrame(rect: *mut c_void) -> CStamp;
    fn DrawStarConBox(
        rect: *mut CRect,
        border: i16,
        medium: CColor,
        dark: CColor,
        fill: i32,
        background: CColor,
    );
    fn FlushGraphics();
    fn WaitForAnyButton(new_button: u32, duration: u32, reset: u32) -> u32;
}

fn color(v: u8) -> CColor {
    let c = Rgba::rgb15(v, v, v);
    CColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// # Safety
/// Game thread with the SIS graphics contexts and string table initialized.
#[no_mangle]
pub unsafe extern "C" fn SaveProblem() {
    let g = CffiPlatform;
    let old = g.set_context(ContextId::Space);
    g.set_font(FontId::StarCon);
    let text = g.game_string(454 + 2);
    let string = std::ffi::CString::new(text.clone()).unwrap();
    let mut t = CText::default();
    t.align = 1;
    t.p_str = string.as_ptr();
    t.char_count = u16::MAX;
    let mut rect = CRect::default();
    if !text.is_empty() {
        TextRect(&mut t, &mut rect, std::ptr::null_mut());
    }
    let (width, height) = g.screen_size();
    let x = (((width - 78) >> 1) as i16 - (rect.width >> 1)) - rect.corner.x;
    let y = (((height - 13) >> 1) as i16 - (rect.height >> 1)) - rect.corner.y;
    rect.corner.x += x - 4;
    rect.corner.y += y - 4;
    rect.width += 8;
    rect.height += 8;
    let saved = SaveContextFrame((&mut rect as *mut CRect).cast());
    g.batch();
    DrawStarConBox(&mut rect, 2, color(16), color(8), 1, color(10));
    g.set_fg_color(Rgba::rgb15(20, 20, 20));
    if !text.is_empty() {
        g.draw_text(&TextDraw {
            baseline: Point { x, y },
            align: Align::Center,
            text,
        });
    }
    g.unbatch();
    FlushGraphics();
    WaitForAnyButton(1, u32::MAX, 0);
    g.draw_stamp(Stamp {
        origin: Point {
            x: saved.origin.x,
            y: saved.origin.y,
        },
        frame: saved.frame as usize,
    });
    g.restore_context(old);
    crate::graphics::drawable_abi::DestroyDrawable(crate::graphics::pixmap_abi::ReleaseDrawable(
        saved.frame.cast(),
    ));
}
