//! Retained navigation graphics with separate kernel and campaign release boundaries.

use crate::graphics::{
    context_abi::{ContextFrame, LoadedContext},
    drawable_abi::LoadedDrawable,
    native_abi::{FRAME, RECT},
};

#[derive(Default)]
pub(super) struct NavigationResources {
    // Field order also preserves release order if the Kernel itself is dropped.
    radar: Option<LoadedContext>,
    gradient: Option<LoadedDrawable>,
    misc: Option<LoadedDrawable>,
    status: Option<LoadedDrawable>,
    play: Option<LoadedDrawable>,
}

fn first_frame(drawable: &Option<LoadedDrawable>) -> FRAME {
    drawable
        .as_ref()
        .and_then(|owner| owner.lease().frame(0))
        .unwrap_or(std::ptr::null_mut())
}

impl NavigationResources {
    fn load_display_with(
        &mut self,
        mut load: impl FnMut(&std::ffi::CStr) -> Option<LoadedDrawable>,
    ) -> bool {
        if self.status.is_none() {
            self.status = load(c"graphics.flagshipstatus");
            if self.status.is_none() {
                return false;
            }
            self.misc = load(c"graphics.miscdata");
            if self.misc.is_none() {
                return false;
            }
            self.gradient = load(c"graphics.fontgradient");
        }
        true
    }
}

impl crate::Kernel {
    pub fn load_navigation_play(&mut self) {
        assert!(
            self.assets.navigation.play.is_none(),
            "campaign play art already loaded"
        );
        self.assets.navigation.play = LoadedDrawable::load(c"graphics.playmenu");
        self.play_frame = first_frame(&self.assets.navigation.play);
    }

    pub fn load_navigation_graphics(&mut self, radar_clip: RECT) -> bool {
        let loaded = self
            .assets
            .navigation
            .load_display_with(LoadedDrawable::load);
        self.flag_stat_frame = first_frame(&self.assets.navigation.status);
        self.misc_data_frame = first_frame(&self.assets.navigation.misc);
        self.font_grad_frame = first_frame(&self.assets.navigation.gradient);
        if !loaded {
            return false;
        }
        if self.assets.navigation.radar.is_none() {
            let radar = LoadedContext::create(c"RadarContext");
            let context = radar.lease();
            context.with_selected(|| {
                context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(self.screen)));
                context.set_clip(radar_clip);
            });
            self.assets.navigation.radar = Some(radar);
        }
        self.radar_context = self
            .assets
            .navigation
            .radar
            .as_ref()
            .unwrap_or_else(|| panic!("navigation radar is not initialized"))
            .lease()
            .as_raw();
        true
    }

    pub fn release_navigation_graphics(&mut self) {
        drop(self.assets.navigation.radar.take());
        self.radar_context = std::ptr::null_mut();
        drop(self.assets.navigation.gradient.take());
        self.font_grad_frame = std::ptr::null_mut();
        drop(self.assets.navigation.misc.take());
        self.misc_data_frame = std::ptr::null_mut();
        drop(self.assets.navigation.status.take());
        self.flag_stat_frame = std::ptr::null_mut();
    }

    pub fn release_navigation_play(&mut self) {
        drop(self.assets.navigation.play.take());
        self.play_frame = std::ptr::null_mut();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::{context_abi::LoadedContext, drawable_abi::LoadedDrawable};

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
            assert_eq!(loaded, failed >= 2);
            assert_eq!(keys, expected[..(failed + 1).min(3)]);
            assert_eq!(assets.status.is_some(), failed != 0);
            assert_eq!(assets.misc.is_some(), failed > 1);
            assert_eq!(assets.gradient.is_some(), failed == 3);
        }
    }

    #[test]
    #[serial_test::serial]
    fn partial_load_keeps_authority_and_does_not_reload_existing_status() {
        let mut assets = NavigationResources::default();
        let mut keys = Vec::new();
        assert!(!assets.load_display_with(|key| {
            keys.push(key.to_bytes().to_vec());
            (keys.len() == 1).then(|| LoadedDrawable::create(2, 0, 0, 1).unwrap())
        }));
        assert_eq!(
            keys,
            [
                b"graphics.flagshipstatus".to_vec(),
                b"graphics.miscdata".to_vec()
            ]
        );
        let status = assets.status.as_ref().unwrap().lease().frame(0).unwrap();
        assert!(assets.misc.is_none());
        assert!(assets.gradient.is_none());
        assert!(assets.load_display_with(|_| panic!("status already loaded")));
        assert_eq!(
            assets.status.as_ref().unwrap().lease().frame(0),
            Some(status)
        );
    }

    #[test]
    #[serial_test::serial]
    fn radar_setup_projects_owned_assets_and_restores_the_selected_context() {
        use crate::graphics::native_abi::{EXTENT, POINT};
        let screen = LoadedDrawable::create(2, 0, 0, 1).unwrap();
        let previous = LoadedContext::create(c"navigation-previous");
        let selected = previous.lease();
        selected.select();
        let mut kernel = crate::Kernel::new();
        kernel.screen = screen.lease().frame(0).unwrap();
        assert!(kernel
            .assets
            .navigation
            .load_display_with(|_| LoadedDrawable::create(2, 0, 0, 1)));
        let clip = RECT {
            corner: POINT { x: 256, y: 184 },
            extent: EXTENT {
                width: 56,
                height: 53,
            },
        };
        assert!(kernel.load_navigation_graphics(clip));
        assert_eq!(
            selected.selected_context().unwrap().as_raw(),
            selected.as_raw()
        );
        let radar = crate::graphics::context_abi::ContextLease::require(kernel.radar_context);
        let (target, actual) = radar.with_selected(|| (radar.target(), radar.clip_rect()));
        assert_eq!(target.as_raw(), kernel.screen);
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
            first_frame(&kernel.assets.navigation.status)
        );
        assert_eq!(
            kernel.misc_data_frame,
            first_frame(&kernel.assets.navigation.misc)
        );
        assert_eq!(
            kernel.font_grad_frame,
            first_frame(&kernel.assets.navigation.gradient)
        );
        let address = kernel.radar_context;
        assert!(kernel.load_navigation_graphics(clip));
        assert_eq!(kernel.radar_context, address);
        kernel.release_navigation_graphics();
        drop(screen);
        assert!(target.frame_count() > 0);
        assert_eq!(
            selected.selected_context().unwrap().as_raw(),
            selected.as_raw()
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
        assert!(kernel
            .assets
            .navigation
            .load_display_with(|_| LoadedDrawable::create(2, 0, 0, 1)));
        kernel.assets.navigation.radar = Some(LoadedContext::create(c"navigation-test"));
        let radar = kernel.assets.navigation.radar.as_ref().unwrap().lease();
        let play = kernel.assets.navigation.play.as_ref().unwrap().lease();
        let gradient = kernel.assets.navigation.gradient.as_ref().unwrap().lease();
        let misc = kernel.assets.navigation.misc.as_ref().unwrap().lease();
        let status = kernel.assets.navigation.status.as_ref().unwrap().lease();
        let allocations = observation::drain();
        let address = |lease: &crate::graphics::drawable_abi::DrawableLease| {
            allocations
                .iter()
                .find_map(|event| match event {
                    Event::Frame { address, owner, .. }
                        if Some(*address) == lease.frame(0).map(|frame| frame as usize) =>
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
        assert!(kernel.radar_context.is_null());
        assert!(kernel.font_grad_frame.is_null());
        assert!(kernel.misc_data_frame.is_null());
        assert!(kernel.flag_stat_frame.is_null());
        assert!(kernel.assets.navigation.play.is_some());
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::graphics::context_abi::ContextLease::require(radar.as_raw())
        }))
        .is_err());
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
