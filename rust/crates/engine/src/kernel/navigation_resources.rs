//! Retained navigation graphics with separate kernel and campaign release boundaries.

use crate::graphics::{
    context::LoadedContext, drawable::ContextFrame, drawable::FrameId, drawable::LoadedDrawable,
    geometry::Rect,
};

#[derive(Default)]
pub(super) struct NavigationResources {
    // Field order also preserves release order if the Kernel itself is dropped.
    radar: Option<LoadedContext>,
    display: Option<NavigationDisplay>,
    play: Option<LoadedDrawable>,
}

struct NavigationDisplay {
    gradient: Option<LoadedDrawable>,
    misc: LoadedDrawable,
    status: LoadedDrawable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("required navigation graphic could not be loaded: {resource}")]
pub struct NavigationLoadError {
    pub resource: &'static str,
}

fn first_frame(drawable: &Option<LoadedDrawable>) -> FrameId {
    drawable
        .as_ref()
        .map_or(crate::graphics::drawable::FrameId::NULL, |owner| {
            owner.first_frame()
        })
}

impl NavigationResources {
    fn load_display_with(
        &mut self,
        mut load: impl FnMut(&std::ffi::CStr) -> Option<LoadedDrawable>,
    ) -> Result<(), NavigationLoadError> {
        if self.display.is_none() {
            let status = load(c"graphics.flagshipstatus").ok_or(NavigationLoadError {
                resource: "graphics.flagshipstatus",
            })?;
            let misc = load(c"graphics.miscdata").ok_or(NavigationLoadError {
                resource: "graphics.miscdata",
            })?;
            let gradient = load(c"graphics.fontgradient");
            self.display = Some(NavigationDisplay {
                gradient,
                misc,
                status,
            });
        }
        Ok(())
    }
}

impl crate::Kernel {
    pub fn load_navigation_play(&mut self) {
        assert!(
            self.assets.navigation.play.is_none(),
            "campaign play art already loaded"
        );
        self.assets.navigation.play = LoadedDrawable::load(&self.content(), c"graphics.playmenu");
        self.play_frame = first_frame(&self.assets.navigation.play);
    }

    pub fn load_navigation_graphics(
        &mut self,
        radar_clip: Rect,
    ) -> Result<(), NavigationLoadError> {
        let content = self.content();
        self.assets
            .navigation
            .load_display_with(|key| LoadedDrawable::load(&content, key))?;
        let display = self
            .assets
            .navigation
            .display
            .as_ref()
            .unwrap_or_else(|| panic!("navigation display is not initialized"));
        self.flag_stat_frame = display.status.first_frame();
        self.misc_data_frame = display.misc.first_frame();
        self.font_grad_frame = first_frame(&display.gradient);
        if self.assets.navigation.radar.is_none() {
            let radar = LoadedContext::create(c"RadarContext");
            let context = radar.lease();
            context.with_selected(|| {
                context.set_frame(ContextFrame::retain(self.screen));
                context.set_clip(radar_clip);
            });
            self.assets.navigation.radar = Some(radar);
        }
        self.radar_context = Some(
            self.assets
                .navigation
                .radar
                .as_ref()
                .unwrap_or_else(|| panic!("navigation radar is not initialized"))
                .context(),
        );
        Ok(())
    }

    pub fn release_navigation_graphics(&mut self) {
        drop(self.assets.navigation.radar.take());
        self.radar_context = None;
        drop(self.assets.navigation.display.take());
        self.font_grad_frame = crate::graphics::drawable::FrameId::NULL;
        self.misc_data_frame = crate::graphics::drawable::FrameId::NULL;
        self.flag_stat_frame = crate::graphics::drawable::FrameId::NULL;
    }

    pub fn release_navigation_play(&mut self) {
        drop(self.assets.navigation.play.take());
        self.play_frame = crate::graphics::drawable::FrameId::NULL;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::{context::LoadedContext, drawable::LoadedDrawable};

    #[test]
    #[serial_test::serial]
    fn load_failures_preserve_resource_order_and_the_optional_gradient() {
        let expected = [
            c"graphics.flagshipstatus",
            c"graphics.miscdata",
            c"graphics.fontgradient",
        ];
        for failed in 0..=3 {
            let mut assets = NavigationResources::default();
            let mut keys = Vec::new();
            let loaded = assets.load_display_with(|key| {
                let index = keys.len();
                keys.push(key.to_owned());
                (index != failed).then(|| LoadedDrawable::create(2, 0, 0, 1).unwrap())
            });
            assert_eq!(loaded.is_ok(), failed >= 2);
            assert_eq!(keys, expected[..(failed + 1).min(3)]);
            assert_eq!(assets.display.is_some(), failed >= 2);
            if let Some(display) = &assets.display {
                assert!(!display.status.first_frame().is_null());
                assert!(!display.misc.first_frame().is_null());
                assert_eq!(display.gradient.is_some(), failed == 3);
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn failed_required_load_releases_staged_status_before_retry() {
        use crate::collections::observation::{self, Event};
        observation::enable();
        observation::drain();
        let mut assets = NavigationResources::default();
        let error = assets
            .load_display_with(|key| {
                (key == c"graphics.flagshipstatus")
                    .then(|| LoadedDrawable::create(2, 0, 0, 1).unwrap())
            })
            .unwrap_err();
        assert_eq!(error.resource, "graphics.miscdata");
        assert!(assets.display.is_none());
        assert_eq!(
            observation::drain()
                .iter()
                .filter(|event| matches!(event, Event::DrawableFree(_)))
                .count(),
            1
        );
        assets
            .load_display_with(|_| LoadedDrawable::create(2, 0, 0, 1))
            .unwrap();
        let status = assets.display.as_ref().unwrap().status.first_frame();
        assets
            .load_display_with(|_| panic!("complete display already loaded"))
            .unwrap();
        assert_eq!(
            assets.display.as_ref().unwrap().status.first_frame(),
            status
        );
    }

    #[test]
    #[serial_test::serial]
    fn radar_setup_projects_owned_assets_and_restores_the_selected_context() {
        use crate::graphics::geometry::{Extent, Point};
        let screen = LoadedDrawable::create(2, 0, 0, 1).unwrap();
        let previous = LoadedContext::create(c"navigation-previous");
        let selected = previous.lease();
        selected.select();
        let mut kernel = crate::Kernel::new();
        kernel.screen = screen.lease().frame(0).unwrap();
        kernel
            .assets
            .navigation
            .load_display_with(|_| LoadedDrawable::create(2, 0, 0, 1))
            .unwrap();
        let clip = Rect {
            corner: Point { x: 256, y: 184 },
            extent: Extent {
                width: 56,
                height: 53,
            },
        };
        kernel.load_navigation_graphics(clip).unwrap();
        assert_eq!(
            selected.selected_context().unwrap().context(),
            selected.context()
        );
        let radar = crate::graphics::context::ContextLease::retain(kernel.radar_context.unwrap());
        let (target, actual) = radar.with_selected(|| (radar.target(), radar.clip_rect()));
        assert_eq!(target.frame(), kernel.screen);
        assert_eq!(
            (
                actual.corner.x,
                actual.corner.y,
                actual.extent.width,
                actual.extent.height
            ),
            (256, 184, 56, 53)
        );
        assert_eq!(
            kernel.flag_stat_frame,
            kernel
                .assets
                .navigation
                .display
                .as_ref()
                .unwrap()
                .status
                .first_frame()
        );
        assert_eq!(
            kernel.misc_data_frame,
            kernel
                .assets
                .navigation
                .display
                .as_ref()
                .unwrap()
                .misc
                .first_frame()
        );
        assert_eq!(
            kernel.font_grad_frame,
            first_frame(&kernel.assets.navigation.display.as_ref().unwrap().gradient)
        );
        let address = kernel.radar_context;
        kernel.load_navigation_graphics(clip).unwrap();
        assert_eq!(kernel.radar_context, address);
        kernel.release_navigation_graphics();
        drop(screen);
        assert!(target.frame_count() > 0);
        assert_eq!(
            selected.selected_context().unwrap().context(),
            selected.context()
        );
    }

    #[test]
    #[serial_test::serial]
    fn graphics_and_play_release_use_owners_not_mutable_projections() {
        use crate::collections::observation::{self, Event};
        observation::enable();
        observation::drain();
        let mut kernel = crate::Kernel::new();
        kernel.assets.navigation.play = LoadedDrawable::create(2, 0, 0, 1);
        kernel
            .assets
            .navigation
            .load_display_with(|_| LoadedDrawable::create(2, 0, 0, 1))
            .unwrap();
        kernel.assets.navigation.radar = Some(LoadedContext::create(c"navigation-test"));
        let radar = kernel.assets.navigation.radar.as_ref().unwrap().lease();
        let play = kernel.assets.navigation.play.as_ref().unwrap().lease();
        let display = kernel.assets.navigation.display.as_ref().unwrap();
        let gradient = display.gradient.as_ref().unwrap().lease();
        let misc = display.misc.lease();
        let status = display.status.lease();
        let allocations = observation::drain();
        let address = |lease: &crate::graphics::drawable::DrawableLease| {
            allocations
                .iter()
                .find_map(|event| match event {
                    Event::Frame { address, owner, .. }
                        if Some(*address) == lease.frame(0).map(|frame| frame.bits()) =>
                    {
                        Some(*owner)
                    }
                    _ => None,
                })
                .unwrap()
        };
        let expected = [address(&gradient), address(&misc), address(&status)];
        // Projections are deliberately absent. They must never supply release authority.
        observation::drain();
        kernel.release_navigation_graphics();
        let freed: Vec<_> = observation::drain()
            .into_iter()
            .filter_map(|event| match event {
                Event::DrawableFree(address) => Some(address),
                _ => None,
            })
            .collect();
        assert_eq!(freed, expected);
        assert!(kernel.radar_context.is_none());
        assert!(kernel.font_grad_frame.is_null());
        assert!(kernel.misc_data_frame.is_null());
        assert!(kernel.flag_stat_frame.is_null());
        assert!(kernel.assets.navigation.play.is_some());
        // the projection's lease keeps the released radar context alive (#961)
        drop(crate::graphics::context::ContextLease::retain(
            radar.context(),
        ));
        assert!(gradient.frame(0).is_some());
        assert!(misc.frame(0).is_some());
        assert!(status.frame(0).is_some());
        kernel.release_navigation_play();
        assert!(
            matches!(observation::drain().as_slice(), [Event::DrawableFree(value)] if *value == address(&play))
        );
        assert!(play.frame(0).is_some());
        kernel.release_navigation_graphics();
        kernel.release_navigation_play();
        assert!(observation::drain().is_empty());
    }
}
