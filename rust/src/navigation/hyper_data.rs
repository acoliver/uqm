//! Hyperspace graphics resources and entry/exit lifecycle.
#![cfg(not(test))]
use super::abi::*;
use crate::engine::StoryKey;
use crate::engine::World as WorldOwner;
use crate::engine::{Kernel as KernelOwner, WorldData as WorldFields};

use super::{color, get, set};
use std::ptr;

/// # Safety
/// Game-thread access, after hyperspace users release their frames and maps.
pub unsafe fn free_hyper_data() {
    let kernel = KernelOwner::current();
    let frames = std::slice::from_raw_parts_mut(ptr::addr_of_mut!((**kernel).hyperstars).cast(), 3);
    for frame in frames {
        DestroyDrawable(ReleaseDrawable(*frame));
        *frame = ptr::null_mut();
    }
    let maps = std::slice::from_raw_parts_mut(ptr::addr_of_mut!((**kernel).cmaps).cast(), 2);
    for map in maps {
        DestroyStringTable(ReleaseStringTable(*map));
        *map = ptr::null_mut();
    }
}

unsafe fn load() {
    if KernelOwner::current()
        .read(|owner| owner.hyperstars[0])
        .is_null()
    {
        (**KernelOwner::current()).hyperstars[0] =
            CaptureDrawable(LoadGraphicInstance(AMBIENT_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        (**KernelOwner::current()).hyperstars[1] =
            CaptureDrawable(LoadGraphicInstance(HYPERSTARS_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        (**KernelOwner::current()).cmaps[0] =
            CaptureStringTable(LoadStringTableInstance(HYPER_COLOR_TAB.as_ptr().cast()).cast());
        (**KernelOwner::current()).hyperstars[2] =
            CaptureDrawable(LoadGraphicInstance(ARISPACE_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        (**KernelOwner::current()).cmaps[1] =
            CaptureStringTable(LoadStringTableInstance(ARISPACE_COLOR_TAB.as_ptr().cast()).cast());
    }
}

/// # Safety
/// World globals, contexts and resources must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn LoadHyperspace() -> BOOLEAN {
    super::hyperspace::live::rust_reset_hyper_distance();
    (**WorldFields::current()).game.ship_stamp.origin = POINT { x: -9999, y: -9999 };
    load();
    KernelOwner::current().write(|kernel| {
        crate::collections::observation::swap_image_view(
            &mut kernel.hyperstars[0],
            &mut kernel.stars_in_space,
        );
    });
    if KernelOwner::current().read(|kernel| kernel.last_activity) & CHECK_LOAD as u16 == 0 {
        repair_sisborder();
    } else if KernelOwner::current().read(|kernel| kernel.last_activity) as u8 == 0 {
        draw_sisframe();
    } else {
        ClearSISRect(DRAW_SIS_DISPLAY as u8);
        repair_sisborder();
    }
    if WorldFields::current().read(|owner| owner.game.autopilot.x) == -1
        || WorldFields::current().read(|owner| owner.game.autopilot.y) == -1
    {
        DrawSISMessage(ptr::null());
    }
    SetContext(KernelOwner::current().read(|kernel| kernel.radar_context));
    SetContextBackGroundColor(color(0, 14, 0));
    SetContext(KernelOwner::current().read(|kernel| kernel.space_context));
    if WorldOwner::current().read(|world| get(world, StoryKey::ARILOU_SPACE_SIDE)) <= 1 {
        SetContextBackGroundColor(color(7, 0, 0));
        SetColorMap(GetColorMapAddress(
            KernelOwner::current().read(|kernel| kernel.cmaps[0]),
        ));
    } else {
        SetContextBackGroundColor(color(0, 26, 0));
        SetColorMap(GetColorMapAddress(
            KernelOwner::current().read(|kernel| kernel.cmaps[1]),
        ));
        WorldOwner::current().write(|world| set(world, StoryKey::USED_BROADCASTER, 0));
        WorldOwner::current().write(|world| set(world, StoryKey::BROADCASTER_RESPONSE, 0));
    }
    ClearSISRect(CLEAR_SIS_RADAR as u8);
    1
}

pub fn free_hyperspace(kernel: &mut KernelOwner) {
    crate::collections::observation::swap_image_view(
        &mut kernel.hyperstars[0],
        &mut kernel.stars_in_space,
    );
}
