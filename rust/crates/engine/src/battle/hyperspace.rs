//! Hyperspace resource operations over the retained kernel.

use crate::{graphics::context_abi::ContextLease, Kernel};
use std::ptr::NonNull;

pub fn radar_context(kernel: &Kernel) -> ContextLease {
    ContextLease::retain(
        NonNull::new(kernel.radar_context)
            .unwrap_or_else(|| panic!("hyperspace radar context is not initialized")),
    )
}

pub fn space_context(kernel: &Kernel) -> ContextLease {
    ContextLease::retain(
        NonNull::new(kernel.space_context)
            .unwrap_or_else(|| panic!("hyperspace space context is not initialized")),
    )
}

pub fn status_context(kernel: &Kernel) -> ContextLease {
    ContextLease::retain(
        NonNull::new(kernel.status_context)
            .unwrap_or_else(|| panic!("hyperspace status context is not initialized")),
    )
}

pub fn load(kernel: &mut Kernel) {
    // SAFETY: the kernel exclusively owns every resource slot used by the loader.
    unsafe { load_owned(kernel) }
}

unsafe fn load_owned(kernel: &mut Kernel) {
    use crate::{
        graphics::{pixmap_abi::CaptureDrawable, resgfx::LoadGraphicInstance},
        strings::{sresins::LoadStringTableInstance, string_table::CaptureStringTable},
    };
    if kernel.hyperstars[0].is_null() {
        kernel.hyperstars[0] =
            CaptureDrawable(LoadGraphicInstance(c"graphics.ambient".as_ptr()).cast());
        kernel.hyperstars[1] =
            CaptureDrawable(LoadGraphicInstance(c"graphics.hyperstars".as_ptr()).cast());
        kernel.cmaps[0] =
            CaptureStringTable(LoadStringTableInstance(c"colortable.hyperspace".as_ptr()));
        kernel.hyperstars[2] =
            CaptureDrawable(LoadGraphicInstance(c"graphics.quasispace".as_ptr()).cast());
        kernel.cmaps[1] =
            CaptureStringTable(LoadStringTableInstance(c"colortable.quasispace".as_ptr()));
    }
}

pub fn free(kernel: &mut Kernel) {
    // SAFETY: the exclusive kernel releases its resources after their last hyperspace user.
    unsafe { free_owned(kernel) }
}

unsafe fn free_owned(kernel: &mut Kernel) {
    use crate::{
        graphics::{drawable_abi::DestroyDrawable, pixmap_abi::ReleaseDrawable},
        strings::string_table::{DestroyStringTable, ReleaseStringTable},
    };
    for frame in &mut kernel.hyperstars {
        DestroyDrawable(ReleaseDrawable(*frame));
        *frame = std::ptr::null_mut();
    }
    for map in &mut kernel.cmaps {
        DestroyStringTable(ReleaseStringTable(*map));
        *map = std::ptr::null_mut();
    }
}

/// # Safety
/// The kernel retains the loaded palette tables on the game thread throughout the copy.
pub unsafe fn select_palette(kernel: &Kernel, quasispace: bool) {
    use crate::graphics::cmap_abi::{GetColorMapAddress, SetColorMap};
    SetColorMap(GetColorMapAddress(kernel.cmaps[usize::from(quasispace)]));
}
