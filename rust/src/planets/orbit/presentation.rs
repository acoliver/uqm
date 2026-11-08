//! Drawing resources retained by one synchronous orbit visit.

use crate::engine::{
    graphics::context_abi::{ContextFrame, ContextLease, LoadedContext},
    Kernel,
};
use std::ptr::NonNull;

pub struct OrbitPresentation {
    planet: LoadedContext,
}

impl OrbitPresentation {
    pub fn new(kernel: &Kernel) -> Self {
        let space = ContextLease::require(kernel.space_context);
        let planet = space.with_selected(|| {
            let mut clip = space.clip_rect();
            let planet = LoadedContext::create(c"PlanetContext");
            let context = planet.lease();
            context.set_frame(ContextFrame::retain(NonNull::new(kernel.screen)));
            clip.extent.height -= 75 + 5;
            context.set_clip(clip);
            planet
        });
        Self { planet }
    }

    pub fn context(&self) -> ContextLease {
        self.planet.lease()
    }
}
