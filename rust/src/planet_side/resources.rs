//! Retained asset snapshot for a synchronous lander trip or status preview.

use super::assets::LanderSoundTable;
use crate::engine::{
    graphics::context_abi::ContextFrame, kernel::lander_resources::LanderResources,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum LanderGraphic {
    Lander = 0,
    Earthquake = 1,
    Lightning = 2,
    Lava = 3,
    Shield = 4,
    Launch = 5,
    Return = 6,
    OrbitView = 7,
}

pub struct PlanetSideAssets {
    graphics: [ContextFrame; 8],
    sounds: LanderSoundTable,
}

impl PlanetSideAssets {
    pub fn retain(owner: &LanderResources) -> Self {
        Self {
            graphics: std::array::from_fn(|index| owner.graphic(index)),
            sounds: LanderSoundTable::from_bank(owner.sounds()),
        }
    }

    pub fn graphic(&self, graphic: LanderGraphic) -> ContextFrame {
        self.graphics[graphic as usize].clone()
    }

    pub fn sounds(&self) -> LanderSoundTable {
        self.sounds.clone()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        engine::Game,
        graphics::{
            context_abi::ContextLease,
            native_abi::{Color, EXTENT, POINT, RECT},
        },
        io::uio_bridge as uio,
    };
    use std::{ffi::CString, ptr};

    pub(crate) fn with_packaged_resources(test: impl FnOnce(&mut Game)) {
        let mut game = Game::new();
        let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
        let content_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../sc2/content")
            .canonicalize()
            .unwrap();
        let path = CString::new(content_root.to_str().unwrap()).unwrap();
        // SAFETY: this serialized fixture owns the temporary mount and restores its predecessor.
        unsafe {
            uio::uio_init();
            let repo = uio::uio_openRepository(0);
            assert!(!uio::uio_mountDir(
                repo,
                c"/".as_ptr(),
                1,
                ptr::null_mut(),
                ptr::null(),
                path.as_ptr(),
                ptr::null_mut(),
                6,
                ptr::null_mut()
            )
            .is_null());
            let content = uio::uio_openDir(repo, c"/".as_ptr(), 0);
            let previous = crate::mainloop::dir_prep::contentDir;
            crate::mainloop::dir_prep::contentDir = content.cast();
            crate::resource::ffi_bridge::LoadResourceIndex(
                content.cast(),
                c"uqm.rmp".as_ptr(),
                ptr::null(),
            );
            let palette =
                crate::strings::sresins::load_binary_entry(c"colortable.main", 0).unwrap();
            let map_count = usize::from(palette[1] - palette[0]) + 1;
            assert_eq!(palette.len(), 2 + map_count * 256 * 3);
            crate::graphics::cmap_abi::SetColorMap(palette.as_ptr());
            test(&mut game);
            crate::graphics::dcqueue_abi::TFB_FlushGraphics();
            crate::mainloop::dir_prep::contentDir = previous;
            uio::uio_closeDir(content);
            uio::uio_closeRepository(repo);
            uio::uio_unInit();
        }
        display.finish(game);
    }

    #[test]
    #[serial_test::serial]
    fn packaged_load_failures_release_each_acquired_graphic_in_reverse_order() {
        use crate::collections::observation::{self, Event};
        with_packaged_resources(|_| {
            let keys = [
                "graphics.lander",
                "graphics.quake",
                "graphics.lightning",
                "graphics.lavaspot",
                "graphics.landershield",
                "graphics.landerlaunch",
                "graphics.landerreturn",
                "graphics.orbview",
                "sounds.lander",
            ];
            observation::enable();
            for (successful, key) in keys.into_iter().enumerate() {
                // SAFETY: the fixture's resource mount stays live for every reload and rollback.
                unsafe {
                    crate::resource::ffi_bridge::LoadResourceIndex(
                        crate::mainloop::dir_prep::contentDir.cast(),
                        c"uqm.rmp".as_ptr(),
                        ptr::null(),
                    );
                    crate::graphics::dcqueue_abi::TFB_FlushGraphics();
                    observation::drain();
                    crate::resource::ffi_bridge::res_Remove(CString::new(key).unwrap().as_ptr());
                }
                assert!(matches!(LanderResources::load(), Err(failed) if failed == key));
                // SAFETY: the fixture owns the graphics queue through deferred release.
                unsafe { crate::graphics::dcqueue_abi::TFB_FlushGraphics() };
                let events = observation::drain();
                let acquired: Vec<_> = events
                    .iter()
                    .filter_map(|event| match event {
                        Event::Drawable(id) => Some(*id),
                        _ => None,
                    })
                    .collect();
                let released: Vec<_> = events
                    .iter()
                    .filter_map(|event| match event {
                        Event::DrawableFree(id) => Some(*id),
                        _ => None,
                    })
                    .collect();
                assert_eq!(acquired.len(), successful, "failed key {key}");
                assert_eq!(
                    released,
                    acquired.into_iter().rev().collect::<Vec<_>>(),
                    "failed key {key}"
                );
            }
        });
    }

    #[test]
    #[serial_test::serial]
    fn packaged_assets_retain_frames_sounds_and_music_after_independent_owner_release() {
        with_packaged_resources(|game| {
            let first = LanderResources::load().unwrap();
            let second = LanderResources::load().unwrap();
            let assets = PlanetSideAssets::retain(&first);
            let first_sound = first.sounds().clip(0);
            let second_sound = second.sounds().clip(0);
            assert_ne!(first_sound.identity(), second_sound.identity());
            let music = first.music(0).unwrap();
            assert!(std::sync::Arc::ptr_eq(&music.0, &first.music(5).unwrap().0));
            assert!(!std::sync::Arc::ptr_eq(
                &music.0,
                &second.music(0).unwrap().0
            ));
            let frame = assets.graphic(LanderGraphic::Lander);
            assert!(frame.frame_count() >= 60);
            let expected = frame.alpha_occupancy().unwrap();
            drop(first);
            assert_eq!(frame.alpha_occupancy().unwrap(), expected);
            let mut visuals = crate::planet_side::visual_adapter::CffiSurfaceVisuals::new(
                frame.clone(),
                None,
                std::array::from_fn(|_| None),
            )
            .unwrap();
            {
                let mut world = crate::planet_side::visual_adapter::CffiWorldVisuals::new(
                    &assets,
                    &mut visuals,
                );
                use crate::planet_side::assembly::WorldVisualPort;
                for facing in 0..16 {
                    let visual = world.shot_visual(facing).unwrap();
                    assert_eq!(visual.frame.index, 16 + u16::from(facing));
                    assert_eq!(
                        visual.mask,
                        crate::planet_side::mask_adapter::extract_frame_mask(
                            &frame.wrapped_frame(16 + u16::from(facing))
                        )
                        .unwrap()
                    );
                }
            }
            let radar = ContextLease::require(game.kernel.status_context);
            radar.set_frame(crate::graphics::context_abi::ContextFrame::retain(
                std::ptr::NonNull::new(game.kernel.screen),
            ));
            radar.set_clip(RECT {
                corner: POINT { x: 0, y: 0 },
                extent: EXTENT {
                    width: 56,
                    height: 53,
                },
            });
            let other = crate::graphics::context_abi::LoadedContext::create(c"preserved-selection");
            other.lease().select();
            // SAFETY: this fixture owns the selected context until the assertion below.
            let selected = unsafe { crate::graphics::context_abi::_pCurContext };
            crate::planet_side::init_lander::draw_lander(
                &radar,
                &assets,
                crate::planet_side::init_lander::LanderStatus::campaign(&game.world),
            );
            // SAFETY: this fixture owns both registered contexts and observes selection only.
            assert_eq!(
                unsafe { crate::graphics::context_abi::_pCurContext },
                selected
            );
            radar.flush();
            let mut pixels = vec![Color::default(); 320 * 240];
            // SAFETY: the registered target and complete destination buffer remain live.
            assert_ne!(
                unsafe {
                    crate::graphics::drawable_abi::ReadFramePixelColors(
                        game.kernel.screen,
                        pixels.as_mut_ptr(),
                        320,
                        240,
                    )
                },
                0
            );
            assert_eq!(
                (pixels[0].r, pixels[0].g, pixels[0].b, pixels[0].a),
                (0, 0, 0, 255)
            );
            drop((visuals, other, second));
        });
    }
}
