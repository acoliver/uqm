//! Drawing resources retained by one synchronous orbit visit.

use std::{cell::RefCell, rc::Rc};
use uqm_engine::graphics::{context::ContextLease, context::LoadedContext, drawable::ContextFrame};
use uqm_engine::Kernel;

pub struct OrbitPresentation {
    planet: LoadedContext,
    scan: crate::planets::scan::state::ScanState,
    sphere: Rc<RefCell<crate::planets::sphere::SphereState>>,
}

impl OrbitPresentation {
    pub fn new(
        kernel: &Kernel,
        width: i16,
        reports: crate::planets::report::typography::ReportResources,
        background: Option<ContextFrame>,
    ) -> Self {
        let space = ContextLease::require(kernel.space_context);
        let planet = space.with_selected(|| {
            let mut clip = space.clip_rect();
            let planet = LoadedContext::create(c"PlanetContext");
            let context = planet.lease();
            context.set_frame(ContextFrame::retain(kernel.screen));
            clip.extent.height -= 75 + 5;
            context.set_clip(clip);
            planet
        });
        let sphere = Rc::new(RefCell::new(crate::planets::sphere::SphereState::default()));
        Self {
            planet,
            scan: crate::planets::scan::state::ScanState::new(
                kernel,
                width,
                sphere.clone(),
                reports,
                background,
            ),
            sphere,
        }
    }

    pub fn context(&self) -> ContextLease {
        self.planet.lease()
    }

    pub fn scan(&self) -> &crate::planets::scan::state::ScanState {
        &self.scan
    }

    pub fn sphere(&self) -> Rc<RefCell<crate::planets::sphere::SphereState>> {
        self.sphere.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planets::sphere::images::{image, OrbitImages};

    #[test]
    #[serial_test::serial]
    fn independent_orbits_release_images_after_their_last_retained_frame() {
        let mut game = uqm_engine::Game::new();
        let display =
            uqm_engine::test_support::campaign_display::CampaignDisplay::install(&mut game)
                .unwrap();
        game.kernel.space_context = game.kernel.off_screen_context;
        let space = ContextLease::require(game.kernel.space_context);
        space.set_frame(ContextFrame::retain(game.kernel.screen));
        space.set_clip(uqm_engine::graphics::geometry::Rect {
            corner: uqm_engine::graphics::geometry::Point { x: 7, y: 13 },
            extent: uqm_engine::graphics::geometry::Extent {
                width: 242,
                height: 227,
            },
        });
        let first = OrbitPresentation::new(&game.kernel, 242, Default::default(), None);
        let second = OrbitPresentation::new(&game.kernel, 242, Default::default(), None);
        let first_sphere = first.sphere();
        assert!(Rc::ptr_eq(&first_sphere, &first.scan().sphere));
        {
            let mut sphere = first_sphere.borrow_mut();
            sphere.images = Some(OrbitImages::new(242));
            let images = sphere.images_mut();
            images.topography = Some(image(2, 242, 75, 1));
            assert!(images.object().is_none());
            images.shield = Some(image(2 | 8, 89, 89, 1));
            let shield = images.object().unwrap();
            assert!(shield.owns_frame(images.shield.as_ref().unwrap().lease().frame(0).unwrap()));
            images.throb = Some(image(2 | 8, 89, 89, 2));
            assert!(images
                .object()
                .unwrap()
                .owns_frame(images.throb.as_ref().unwrap().lease().frame(1).unwrap()));
            assert_eq!(images.tint.lease().frame_rect(0).extent.width, 242);
            assert_eq!(images.tint_color.a, 255);
        }
        let retained = first_sphere.borrow().topography().context_frame(0);
        assert_eq!(first_sphere.borrow().sphere_frame().rect().extent.width, 75);
        assert_eq!(
            first_sphere
                .borrow()
                .zoomed_topography()
                .frame_rect(0)
                .extent
                .width,
            968
        );
        assert!(second.sphere().borrow().images.is_none());
        assert_eq!(first.context().clip_rect().extent.height, 147);
        assert_eq!(second.scan().context().clip_rect().extent.height, 75);
        drop(first_sphere);
        drop(first);
        assert_eq!(retained.rect().extent.width, 242);
        assert_eq!(second.context().clip_rect().extent.height, 147);
        drop(retained);
        drop(second);
        drop(space);
        display.finish(game);
    }
}
