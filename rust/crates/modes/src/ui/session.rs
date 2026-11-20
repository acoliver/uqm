//! Resources retained by a menu across nested input and save/load calls.

use crate::engine::world::SisState;
use crate::engine::Kernel;
use crate::graphics::{
    drawable::FrameId, drawable::LoadedDrawable, geometry::Point, geometry::TextAlign,
};
use crate::sound::resource_types::LoadedMusic;

#[derive(Default)]
pub(super) struct MenuAssets {
    pub(super) art: Option<LoadedDrawable>,
    pub(super) modules: Option<LoadedDrawable>,
    pub(super) music: Option<LoadedMusic>,
}
impl MenuAssets {
    pub(super) fn art(&self) -> FrameId {
        self.art
            .as_ref()
            .map_or(crate::graphics::drawable::FrameId::NULL, |d| {
                d.first_frame()
            })
    }
    pub(super) fn modules(&self) -> FrameId {
        self.modules
            .as_ref()
            .map_or(crate::graphics::drawable::FrameId::NULL, |d| {
                d.first_frame()
            })
    }
}
pub(super) fn text(buffer: &str, baseline: Point, align: TextAlign) {
    let text = std::ffi::CStr::from_bytes_until_nul(buffer.as_bytes())
        .unwrap_or_else(|e| panic!("menu text is not terminated: {e}"));
    crate::graphics::menu::text(text, baseline, align, u16::MAX);
}
pub(super) fn draw_piece(
    kernel: &Kernel,
    sis: &SisState,
    dimensions: (i32, i32),
    frame: FrameId,
    piece: u16,
    slot: u16,
    blueprint: bool,
) {
    crate::starbase::draw::draw_ship_piece(
        &crate::starbase::graphics::Graphics::new(kernel, dimensions),
        &sis.module_slots,
        (frame).bits(),
        piece,
        slot,
        blueprint,
    );
}

/// Nested modes supplied by the campaign composition, not acquired from ambient state.
pub struct MenuServices {
    pub communicate: fn(
        &mut crate::engine::battle::Cx<'_>,
        &mut crate::engine::observe::Io<'_>,
        u32,
        crate::comm::CommOptions,
    ) -> u16,
    pub game_options:
        fn(&mut crate::engine::battle::Cx<'_>, &mut crate::engine::observe::Io<'_>) -> u32,
    pub ship_spin: fn(
        &mut crate::engine::battle::Cx<'_>,
        &mut crate::engine::observe::Io<'_>,
        u16,
        Option<&LoadedMusic>,
    ),
}
