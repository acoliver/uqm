//! Native screen preparation and restoration around the typed star-map session.

#[cfg(not(test))]
use super::*;

#[cfg(not(test))]
pub(super) unsafe fn prepare_star_map(
    misc_data: *mut c_void,
    space: *mut c_void,
    autopilot: POINT,
) -> (MENU_STATE, POINT) {
    // SAFETY: the caller retains these native resources on the initialized game thread.
    unsafe {
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
        if which_space() <= 1 {
            update_map();
        }
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
