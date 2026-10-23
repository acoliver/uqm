//! Graphics facade and transition state, ported from gfx_common.c (GPL-2.0-or-later).
use super::dcqueue_abi::{TFB_BatchGraphics, TFB_GetBatchDepth, TFB_UnbatchGraphics};
use super::native_abi::{EXTENT, POINT, RECT};
use super::sdl::lifecycle_abi::TFB_UploadTransitionScreen;
use super::tfb_draw_abi::{TFB_DrawScreen_Copy, TFB_DrawScreen_WaitForSignal};
use crate::time::counter_abi::GetTimeCounter;
use std::ptr;

macro_rules! screen_global {
    ($($name:ident),*) => { $(#[no_mangle] pub static mut $name: i32 = 0;)* };
}
screen_global!(
    ScreenWidth,
    ScreenHeight,
    ScreenWidthActual,
    ScreenHeightActual,
    ScreenColorDepth,
    GraphicsDriver,
    TFB_DEBUG_HALT
);
#[no_mangle]
pub static mut TransitionAmount: i32 = 255;
#[no_mangle]
pub static mut TransitionClipRect: RECT = RECT {
    corner: POINT { x: 0, y: 0 },
    extent: EXTENT {
        width: 0,
        height: 0,
    },
};
static mut GSCALE: i32 = 256;
static mut GSCALE_MODE: i32 = 1;
extern "C" {
    fn SleepThread(ticks: i32);
}
/// # Safety
/// Optional rectangle must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn DrawFromExtraScreen(rect: *mut RECT) {
    TFB_DrawScreen_Copy(rect, 1, 0);
}
/// # Safety
/// Optional rectangle must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn LoadIntoExtraScreen(rect: *mut RECT) {
    TFB_DrawScreen_Copy(rect, 0, 1);
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn SetGraphicScale(scale: i32) -> i32 {
    let old = GSCALE;
    GSCALE = if scale == 0 { 256 } else { scale };
    old
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn GetGraphicScale() -> i32 {
    GSCALE
}
/// # Safety
/// Mode must be nearest, bilinear or trilinear; called on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn SetGraphicScaleMode(mode: i32) -> i32 {
    let old = GSCALE_MODE;
    assert!((1..=3).contains(&mode));
    GSCALE_MODE = mode;
    old
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn GetGraphicScaleMode() -> i32 {
    GSCALE_MODE
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn BatchGraphics() {
    TFB_BatchGraphics();
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn UnbatchGraphics() {
    TFB_UnbatchGraphics();
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn GetBatchDepth() -> i32 {
    TFB_GetBatchDepth()
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn FlushGraphics() {
    TFB_DrawScreen_WaitForSignal();
}
fn expand_rect(rect: &mut RECT, expansion: i32, width: i32, height: i32) {
    if i32::from(rect.corner.x) - expansion >= 0 {
        rect.extent.width = (i32::from(rect.extent.width) + expansion) as i16;
        rect.corner.x = (i32::from(rect.corner.x) - expansion) as i16;
    } else {
        rect.extent.width = rect.extent.width.wrapping_add(rect.corner.x);
        rect.corner.x = 0;
    }
    if i32::from(rect.corner.y) - expansion >= 0 {
        rect.extent.height = (i32::from(rect.extent.height) + expansion) as i16;
        rect.corner.y = (i32::from(rect.corner.y) - expansion) as i16;
    } else {
        rect.extent.height = rect.extent.height.wrapping_add(rect.corner.y);
        rect.corner.y = 0;
    }
    rect.extent.width =
        if i32::from(rect.corner.x) + i32::from(rect.extent.width) + expansion <= width {
            (i32::from(rect.extent.width) + expansion) as i16
        } else {
            (width - i32::from(rect.corner.x)) as i16
        };
    rect.extent.height =
        if i32::from(rect.corner.y) + i32::from(rect.extent.height) + expansion <= height {
            (i32::from(rect.extent.height) + expansion) as i16
        } else {
            (height - i32::from(rect.corner.y)) as i16
        };
}
/// # Safety
/// Optional rectangle must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn SetTransitionSource(rect: *const RECT) {
    let mut expanded;
    let rect = if let Some(rect) = rect.as_ref() {
        expanded = *rect;
        expand_rect(&mut expanded, 2, ScreenWidth, ScreenHeight);
        ptr::addr_of!(expanded)
    } else {
        ptr::null()
    };
    TFB_DrawScreen_Copy(rect, 0, 2);
}
/// # Safety
/// Optional rectangle must be readable; caller must own graphics thread while transition runs.
#[no_mangle]
pub unsafe extern "C" fn ScreenTransition(_kind: i32, rect: *const RECT) {
    const DURATION: u32 = 840 * 31 / 60;
    TransitionClipRect = rect.as_ref().copied().unwrap_or(RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: ScreenWidth as i16,
            height: ScreenHeight as i16,
        },
    });
    TFB_UploadTransitionScreen();
    ptr::write_volatile(ptr::addr_of_mut!(TransitionAmount), 0);
    FlushGraphics();
    let start = GetTimeCounter();
    while ptr::read_volatile(ptr::addr_of!(TransitionAmount)) < 255 {
        SleepThread(840 / 100);
        let delta = GetTimeCounter().wrapping_sub(start);
        let mut amount = (delta.wrapping_mul(255) / DURATION) as i32;
        if amount > 255 {
            amount = 255;
        }
        ptr::write_volatile(ptr::addr_of_mut!(TransitionAmount), amount);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transition_expansion_stops_at_screen_edges() {
        let mut r = RECT {
            corner: POINT { x: 1, y: 3 },
            extent: EXTENT {
                width: 8,
                height: 6,
            },
        };
        expand_rect(&mut r, 2, 10, 10);
        assert_eq!(
            (r.corner.x, r.corner.y, r.extent.width, r.extent.height),
            (0, 1, 10, 9)
        );
        let mut r = RECT {
            corner: POINT { x: 4, y: 4 },
            extent: EXTENT {
                width: 2,
                height: 1,
            },
        };
        expand_rect(&mut r, 2, 10, 10);
        assert_eq!(
            (r.corner.x, r.corner.y, r.extent.width, r.extent.height),
            (2, 2, 6, 5)
        );
    }
}
