//! Resources retained by a menu across nested input and save/load calls.

use uqm_engine::graphics::{
    drawable::FrameId, drawable::LoadedDrawable, geometry::Point, geometry::TextAlign,
};
use uqm_engine::sound::resource_types::LoadedMusic;
use uqm_engine::world::SisState;
use uqm_engine::Kernel;

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
            .map_or(uqm_engine::graphics::drawable::FrameId::NULL, |d| {
                d.first_frame()
            })
    }
    pub(super) fn modules(&self) -> FrameId {
        self.modules
            .as_ref()
            .map_or(uqm_engine::graphics::drawable::FrameId::NULL, |d| {
                d.first_frame()
            })
    }
}
pub(super) fn text(buffer: &str, baseline: Point, align: TextAlign) {
    let text = std::ffi::CStr::from_bytes_until_nul(buffer.as_bytes())
        .unwrap_or_else(|e| panic!("menu text is not terminated: {e}"));
    uqm_engine::graphics::menu::text(text, baseline, align, u16::MAX);
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
        &mut uqm_engine::battle::Cx<'_>,
        &mut uqm_engine::observe::Io<'_>,
        u32,
        crate::comm::CommOptions,
    ) -> u16,
    pub game_options: fn(&mut uqm_engine::battle::Cx<'_>, &mut uqm_engine::observe::Io<'_>) -> u32,
    pub ship_spin: fn(
        &mut uqm_engine::battle::Cx<'_>,
        &mut uqm_engine::observe::Io<'_>,
        u16,
        Option<&LoadedMusic>,
    ),
}
