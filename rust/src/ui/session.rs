//! Resources retained by a menu across nested input and save/load calls.

use crate::{
    engine::{world::SisState, Kernel},
    graphics::{drawable_abi::LoadedDrawable, native_abi::*},
    sound::resource_types::LoadedMusic,
};

#[derive(Default)]
pub(super) struct MenuAssets {
    pub(super) art: Option<LoadedDrawable>,
    pub(super) modules: Option<LoadedDrawable>,
    pub(super) music: Option<LoadedMusic>,
}
impl MenuAssets {
    pub(super) fn art(&self) -> FRAME {
        self.art
            .as_ref()
            .and_then(|d| d.lease().frame(0))
            .unwrap_or(std::ptr::null_mut())
    }
    pub(super) fn modules(&self) -> FRAME {
        self.modules
            .as_ref()
            .and_then(|d| d.lease().frame(0))
            .unwrap_or(std::ptr::null_mut())
    }
}
pub(super) fn text(buffer: &str, baseline: POINT, align: TEXT_ALIGN) {
    let text = std::ffi::CStr::from_bytes_until_nul(buffer.as_bytes())
        .unwrap_or_else(|e| panic!("menu text is not terminated: {e}"));
    crate::graphics::menu::text(text, baseline, align, u16::MAX);
}
pub(super) fn draw_piece(
    kernel: &Kernel,
    sis: &SisState,
    dimensions: (i32, i32),
    frame: FRAME,
    piece: u16,
    slot: u16,
    blueprint: bool,
) {
    crate::starbase::draw::draw_ship_piece(
        &crate::starbase::graphics::Graphics::new(kernel, dimensions),
        &sis.module_slots,
        frame as usize,
        piece,
        slot,
        blueprint,
    );
}

/// Nested modes supplied by the campaign composition, not acquired from ambient state.
pub struct MenuServices {
    pub game_options:
        fn(&mut crate::engine::battle::Cx<'_>, &mut crate::engine::observe::Io<'_>) -> u32,
    pub ship_spin: fn(
        &mut crate::engine::battle::Cx<'_>,
        &mut crate::engine::observe::Io<'_>,
        u16,
        Option<&LoadedMusic>,
    ),
}
