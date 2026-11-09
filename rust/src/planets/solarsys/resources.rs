//! Retained interplanetary resources borrowed from the supplied campaign Kernel.

use crate::engine::{kernel::solar_resources::SolarResources, Kernel};
use crate::graphics::{drawable_abi::LoadedDrawable, native_abi::FRAME};

pub(crate) fn retained(kernel: &Kernel) -> &SolarResources {
    kernel
        .solar_presentation
        .assets
        .as_ref()
        .unwrap_or_else(|| panic!("interplanetary resources are not loaded"))
}

pub(crate) fn frame(owner: &LoadedDrawable, index: u16) -> FRAME {
    let first = owner
        .lease()
        .frame(0)
        .unwrap_or_else(|| panic!("interplanetary drawable has no frame"));
    // SAFETY: the supplied owner retains every frame in this native drawable.
    unsafe { crate::graphics::pixmap_abi::SetAbsFrameIndex(first, index) }
}
