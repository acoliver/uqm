//! Hyperspace graphics resources and entry/exit lifecycle.
#![cfg(not(test))]
#![allow(non_snake_case)]
use super::abi::*;

use super::{color, get, set};
use std::ptr;

/// # Safety
/// Game-thread access, after hyperspace users release their frames and maps.
pub unsafe fn FreeHyperData() {
    use crate::engine::Kernel;
    let frames = std::slice::from_raw_parts_mut(
        ptr::addr_of_mut!((*Kernel::current()).hyperstars).cast(),
        3,
    );
    for frame in frames {
        DestroyDrawable(ReleaseDrawable(*frame));
        *frame = ptr::null_mut();
    }
    let maps =
        std::slice::from_raw_parts_mut(ptr::addr_of_mut!((*Kernel::current()).cmaps).cast(), 2);
    for map in maps {
        DestroyStringTable(ReleaseStringTable(*map));
        *map = ptr::null_mut();
    }
}

unsafe fn load() {
    use crate::engine::Kernel;
    if (*Kernel::current()).hyperstars[0].is_null() {
        (*Kernel::current()).hyperstars[0] =
            CaptureDrawable(LoadGraphicInstance(AMBIENT_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        (*Kernel::current()).hyperstars[1] =
            CaptureDrawable(LoadGraphicInstance(HYPERSTARS_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        (*Kernel::current()).cmaps[0] =
            CaptureStringTable(LoadStringTableInstance(HYPER_COLOR_TAB.as_ptr().cast()).cast());
        (*Kernel::current()).hyperstars[2] =
            CaptureDrawable(LoadGraphicInstance(ARISPACE_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        (*Kernel::current()).cmaps[1] =
            CaptureStringTable(LoadStringTableInstance(ARISPACE_COLOR_TAB.as_ptr().cast()).cast());
    }
}

/// # Safety
/// World globals, contexts and resources must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn LoadHyperspace() -> BOOLEAN {
    use crate::engine::{Kernel, WorldData};
    super::hyperspace::live::rust_reset_hyper_distance();
    (*WorldData::current()).game.ship_stamp.origin = POINT { x: -9999, y: -9999 };
    load();
    crate::collections::observation::swap_image_view(
        &mut *ptr::addr_of_mut!((*Kernel::current()).hyperstars[0]),
        &mut *ptr::addr_of_mut!((*Kernel::current()).stars_in_space),
    );
    if (*Kernel::current()).last_activity & CHECK_LOAD as u16 == 0 {
        RepairSISBorder();
    } else if (*Kernel::current()).last_activity as u8 == 0 {
        DrawSISFrame();
    } else {
        ClearSISRect(DRAW_SIS_DISPLAY as u8);
        RepairSISBorder();
    }
    if (*WorldData::current()).game.autopilot.x == -1
        || (*WorldData::current()).game.autopilot.y == -1
    {
        DrawSISMessage(ptr::null());
    }
    SetContext((*Kernel::current()).radar_context);
    SetContextBackGroundColor(color(0, 14, 0));
    SetContext((*Kernel::current()).space_context);
    if get("ARILOU_SPACE_SIDE") <= 1 {
        SetContextBackGroundColor(color(7, 0, 0));
        SetColorMap(GetColorMapAddress((*Kernel::current()).cmaps[0]));
    } else {
        SetContextBackGroundColor(color(0, 26, 0));
        SetColorMap(GetColorMapAddress((*Kernel::current()).cmaps[1]));
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
    use crate::engine::Kernel;
    crate::collections::observation::swap_image_view(
        &mut *ptr::addr_of_mut!((*Kernel::current()).hyperstars[0]),
        &mut *ptr::addr_of_mut!((*Kernel::current()).stars_in_space),
    );
    1
}
