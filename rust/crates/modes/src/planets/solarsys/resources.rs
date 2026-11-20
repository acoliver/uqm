//! Retained interplanetary resources borrowed from the supplied campaign Kernel.

use crate::engine::{kernel::solar_resources::SolarResources, Kernel};
use crate::graphics::{drawable::FrameId, drawable::LoadedDrawable};

pub(crate) fn retained(kernel: &Kernel) -> &SolarResources {
    kernel
        .solar_presentation
        .assets
        .as_ref()
        .unwrap_or_else(|| panic!("interplanetary resources are not loaded"))
}

pub(crate) fn frame(owner: &LoadedDrawable, index: u16) -> FrameId {
    owner.lease().context_frame(0).wrapped_frame(index).frame()
}

pub(crate) fn ship_facing(
    kernel: &Kernel,
    game: &crate::engine::battle::schema::GameRecord,
) -> u16 {
    retained(kernel)
        .flagship
        .lease()
        .frame_index(game.ship_stamp.frame)
        .unwrap_or_else(|| panic!("flagship frame is outside its retained drawable")) as u16
}
