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
    owner.lease().context_frame(0).wrapped_frame(index).as_raw()
}

pub(crate) fn ship_facing(
    kernel: &Kernel,
    game: &crate::engine::battle::schema::GAME_STATE,
) -> u16 {
    retained(kernel)
        .flagship
        .lease()
        .frame_index(game.ship_stamp.frame.cast())
        .unwrap_or_else(|| panic!("flagship frame is outside its retained drawable")) as u16
}
