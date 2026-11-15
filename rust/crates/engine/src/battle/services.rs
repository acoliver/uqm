//! Explicit presentation services for engine-owned battle primitives.
use super::views::FrameSelection;
use super::{Cx, ElementId, ShipId};
use crate::graphics::native_abi::{POINT, RECT};

pub trait PrimitiveServices {
    fn draw_statistics(&self, cx: &mut Cx<'_>, ship: ShipId, crew: i16, energy: i16);
    fn init_ship_status(&self, cx: &mut Cx<'_>, ship: ShipId, clip: Option<RECT>);
    fn silhouette_at(
        &self,
        cx: &mut Cx<'_>,
        source: ElementId,
        origin: POINT,
        frame: FrameSelection,
        flags: u8,
    );
}
impl crate::Kernel {
    pub fn install_primitive_services(&mut self, services: &'static dyn PrimitiveServices) {
        self.primitive_services = Some(services);
    }
    pub fn primitive_services(&self) -> &'static dyn PrimitiveServices {
        self.primitive_services
            .unwrap_or_else(|| panic!("battle primitive services were not installed"))
    }
}
#[cfg(not(test))]
impl Cx<'_> {
    pub fn draw_statistics(&mut self, ship: ShipId, crew: i16, energy: i16) {
        self.kernel
            .primitive_services()
            .draw_statistics(self, ship, crew, energy);
    }
    pub fn init_ship_status(&mut self, ship: ShipId, clip: Option<RECT>) {
        self.kernel
            .primitive_services()
            .init_ship_status(self, ship, clip);
    }
    pub fn silhouette_at(
        &mut self,
        source: ElementId,
        origin: POINT,
        frame: FrameSelection,
        flags: u8,
    ) {
        self.kernel
            .primitive_services()
            .silhouette_at(self, source, origin, frame, flags);
    }
    pub fn silhouette(
        &mut self,
        source: ElementId,
        frame: crate::battle::views::FrameSelection,
        flags: u8,
    ) {
        self.silhouette_at(source, POINT { x: 0, y: 0 }, frame, flags);
    }
    pub fn silhouette_frame(&mut self, id: ElementId, frame: FrameSelection, flags: u8) {
        self.silhouette(id, frame, flags);
    }
    pub fn set_star_image(&mut self, id: ElementId) {
        use crate::battle::views::{Frames, SharedFrames};
        let frames = Frames::retain_shared(
            SharedFrames::Stars,
            std::slice::from_ref(&self.kernel.stars_in_space),
        );
        let frame = self.retreat_frame(frames.view(0));
        self.set_frame(id, false, frame);
        self.set_frames(id, false, frames);
    }
    pub fn set_blast_frames(&mut self, id: ElementId) {
        use crate::battle::views::{Frames, SharedFrames};
        let frames = Frames::retain_shared(SharedFrames::Blast, &self.kernel.blast);
        self.set_frames(id, false, frames);
    }
    pub fn planet_frames(&self) -> crate::battle::views::Frames {
        crate::battle::views::Frames::retain_shared(
            crate::battle::views::SharedFrames::Planet,
            &self.kernel.planet,
        )
    }
    pub fn game_sound(&mut self, index: u16, source: ElementId) {
        let sound = self
            .kernel
            .game_sounds
            .as_ref()
            .map(|bank| bank.clip(index));
        self.kernel.sounds.push(sound, Some(source));
    }

    pub fn delta_energy(&mut self, id: ElementId, delta: i16) -> bool {
        super::statistics::delta_energy(self, id, delta)
    }
    pub fn delta_crew(&mut self, id: ElementId, delta: i16) -> bool {
        super::statistics::delta_crew(self, id, delta)
    }
    pub fn missile(&mut self, value: &mut super::views::Missile) -> Option<ElementId> {
        super::weapon::initialize_missile(self, value)
    }
    pub fn laser(&mut self, value: &mut super::views::Laser) -> Option<ElementId> {
        super::weapon::initialize_laser(self, value)
    }
    pub fn weapon_collision(
        &mut self,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) -> Option<ElementId> {
        super::weapon::weapon_collision(self, a, pa, b, pb)
    }
}
