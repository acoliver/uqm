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
    use crate::graphics::{
        palette_cycle::PaletteCycle, pixmap_abi::CaptureDrawable, resgfx::LoadGraphicInstance,
    };
    if kernel.hyperstars[0].is_null() {
        kernel.hyperstars[0] =
            CaptureDrawable(LoadGraphicInstance(c"graphics.ambient".as_ptr()).cast());
        kernel.hyperstars[1] =
            CaptureDrawable(LoadGraphicInstance(c"graphics.hyperstars".as_ptr()).cast());
        kernel.cmaps[0] = PaletteCycle::load(c"colortable.hyperspace");
        kernel.hyperstars[2] =
            CaptureDrawable(LoadGraphicInstance(c"graphics.quasispace".as_ptr()).cast());
        kernel.cmaps[1] = PaletteCycle::load(c"colortable.quasispace");
    }
}

pub fn free_hyper_data(kernel: &mut Kernel) {
    // SAFETY: the exclusive kernel releases its resources after their last hyperspace user.
    unsafe { free_owned(kernel) }
}

unsafe fn free_owned(kernel: &mut Kernel) {
    use crate::graphics::{drawable_abi::DestroyDrawable, pixmap_abi::ReleaseDrawable};
    for frame in &mut kernel.hyperstars {
        DestroyDrawable(ReleaseDrawable(*frame));
        *frame = std::ptr::null_mut();
    }
    for map in &mut kernel.cmaps {
        *map = None;
    }
}

pub fn select_palette(kernel: &Kernel, quasispace: bool) {
    if let Some(palette) = &kernel.cmaps[usize::from(quasispace)] {
        palette.apply();
    }
}

pub fn free_hyperspace(kernel: &mut Kernel) {
    crate::collections::observation::swap_image_view(
        &mut kernel.hyperstars[0],
        &mut kernel.stars_in_space,
    );
}
