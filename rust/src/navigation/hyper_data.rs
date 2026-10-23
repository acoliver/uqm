//! Hyperspace graphics resources and entry/exit lifecycle.
#![cfg(any(not(test), feature = "linked_c_archive"))]
#![allow(non_snake_case)]
use super::abi::*;
use super::globals::{GlobData, RadarContext};
use super::{get, set};
use std::ptr;

#[no_mangle]
pub static mut hyperstars: [FRAME; 3] = [ptr::null_mut(); 3];
static mut CMAPS: [COLORMAP; 2] = [ptr::null_mut(); 2];

pub(super) fn color(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

/// # Safety
/// Game-thread access, after hyperspace users release their frames and maps.
#[no_mangle]
pub unsafe extern "C" fn FreeHyperData() {
    let frames = ptr::addr_of_mut!(hyperstars).cast::<FRAME>();
    for i in 0..3 {
        DestroyDrawable(ReleaseDrawable(*frames.add(i)));
        *frames.add(i) = ptr::null_mut();
    }
    let maps = ptr::addr_of_mut!(CMAPS).cast::<COLORMAP>();
    for i in 0..2 {
        DestroyStringTable(ReleaseStringTable(*maps.add(i)));
        *maps.add(i) = ptr::null_mut();
    }
}

unsafe fn load() {
    if hyperstars[0].is_null() {
        hyperstars[0] =
            CaptureDrawable(LoadGraphicInstance(AMBIENT_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        hyperstars[1] =
            CaptureDrawable(LoadGraphicInstance(HYPERSTARS_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        CMAPS[0] =
            CaptureStringTable(LoadStringTableInstance(HYPER_COLOR_TAB.as_ptr().cast()).cast());
        hyperstars[2] =
            CaptureDrawable(LoadGraphicInstance(ARISPACE_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        CMAPS[1] =
            CaptureStringTable(LoadStringTableInstance(ARISPACE_COLOR_TAB.as_ptr().cast()).cast());
    }
}

/// # Safety
/// World globals, contexts and resources must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn LoadHyperspace() -> BOOLEAN {
    super::hyperspace::live::rust_reset_hyper_distance();
    GlobData.Game_state.ShipStamp.origin = POINT { x: -9999, y: -9999 };
    load();
    std::ptr::swap(
        ptr::addr_of_mut!(hyperstars[0]),
        ptr::addr_of_mut!(stars_in_space),
    );
    if LastActivity & CHECK_LOAD as u16 == 0 {
        RepairSISBorder();
    } else if LastActivity as u8 == 0 {
        DrawSISFrame();
    } else {
        ClearSISRect(DRAW_SIS_DISPLAY as u8);
        RepairSISBorder();
    }
    if GlobData.Game_state.autopilot.x == -1 || GlobData.Game_state.autopilot.y == -1 {
        DrawSISMessage(ptr::null());
    }
    SetContext(RadarContext);
    SetContextBackGroundColor(color(0, 14, 0));
    SetContext(SpaceContext);
    if get("ARILOU_SPACE_SIDE") <= 1 {
        SetContextBackGroundColor(color(7, 0, 0));
        SetColorMap(GetColorMapAddress(CMAPS[0]));
    } else {
        SetContextBackGroundColor(color(0, 26, 0));
        SetColorMap(GetColorMapAddress(CMAPS[1]));
        set("USED_BROADCASTER", 0);
        set("BROADCASTER_RESPONSE", 0);
    }
    ClearSISRect(CLEAR_SIS_RADAR as u8);
    1
}

/// # Safety
/// Must be paired with `LoadHyperspace` on the game thread.
#[no_mangle]
pub unsafe extern "C" fn FreeHyperspace() -> BOOLEAN {
    std::ptr::swap(
        ptr::addr_of_mut!(hyperstars[0]),
        ptr::addr_of_mut!(stars_in_space),
    );
    1
}
