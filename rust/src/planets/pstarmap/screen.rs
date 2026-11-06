//! Native screen preparation and restoration around the typed star-map session.

#[cfg(not(test))]
use super::*;
use super::{box_union, EXTENT, POINT, RECT};
#[cfg(not(test))]
use std::ffi::c_void;

#[cfg(not(test))]
pub(super) unsafe fn prepare_star_map(
    cx: &mut crate::engine::battle::Cx<'_>,
    input: &crate::engine::input::InputSession,
    misc_data: *mut c_void,
    space: *mut c_void,
    autopilot: POINT,
) -> (MENU_STATE, POINT) {
    // SAFETY: the caller retains these native resources on the initialized game thread.
    unsafe {
        let (menu, universe, update) = cx.with_legacy(|| {
            let mut menu: MENU_STATE = std::mem::zeroed();
            ZOOM_LEVEL = 0;
            MAP_ORIGIN = POINT { x: 4999, y: 4999 };
            STAR_MAP_FRAME = pixmap_abi::SetAbsFrameIndex(misc_data.cast(), 48).cast();
            let universe = current_location();
            cursorLoc = autopilot;
            if cursorLoc.x == -1 && cursorLoc.y == -1 {
                cursorLoc = universe;
            }
            menu.Initialized = 0;
            TRANSITION_PENDING = true;
            (menu, universe, which_space() <= 1)
        });
        if update {
            update_map(cx, input);
        }
        cx.with_legacy(|| {
            DrawStarMap(0, (-1isize) as *mut RECT);
            TRANSITION_PENDING = false;
            BatchGraphics();
            let old_context = SetContext(space.cast());
            let mut clip = zero_rect();
            GetContextClipRect(&mut clip);
            SetContext(old_context);
            LoadIntoExtraScreen(ptr::from_mut(&mut clip).cast());
            draw_cursor(ux(cursorLoc.x), uy(cursorLoc.y));
            UnbatchGraphics();
            (menu, universe)
        })
    }
}

#[cfg(not(test))]
pub(super) unsafe fn finish_star_map(universe: POINT) {
    // SAFETY: the caller retains these native resources on the initialized game thread.
    unsafe {
        DrawHyperCoords(universe);
        DrawSISMessage(ptr::null());
        DrawStatusMessage(ptr::null());
    }
}

pub(super) fn repair_rectangle(last: RECT, next: RECT, width: i32, height: i32) -> RECT {
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

#[cfg(not(test))]
pub(super) fn enlarge(r: &mut RECT) {
    r.extent.width = r.extent.width.wrapping_add(1);
    r.extent.height = r.extent.height.wrapping_add(1);
}
