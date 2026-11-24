//! Retained asset snapshot for a synchronous lander trip or status preview.

use super::assets::LanderSoundTable;
use uqm_engine::graphics::drawable::ContextFrame;
use uqm_engine::kernel::lander_resources::LanderResources;

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
    use uqm_engine::graphics::{
        context::ContextLease, geometry::Extent, geometry::Point, geometry::Rect,
    };
    use uqm_engine::Game;

    pub(crate) fn with_packaged_resources(test: impl FnOnce(&mut Game)) {
        let mut game = Game::new();
        let display =
            uqm_engine::test_support::campaign_display::CampaignDisplay::install(&mut game)
                .unwrap();
        let content_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../sc2/content")
            .canonicalize()
            .unwrap();
        let mount = uqm_engine::test_support::native_session::ContentMount::new(&content_root);
        game.kernel.admit_content(mount.load_index("uqm.rmp"));
        let palette = uqm_engine::strings::sresins::load_binary_entry(
            &game.kernel.content(),
            c"colortable.main",
            0,
        )
        .unwrap();
        uqm_engine::test_support::native_probe::set_color_map(&palette);
        test(&mut game);
        uqm_engine::test_support::native_probe::flush_graphics();
        drop(mount);
        display.finish(game);
    }

    #[test]
    #[serial_test::serial]
    fn packaged_load_failures_release_each_acquired_graphic_in_reverse_order() {
        use uqm_engine::collections::observation::{self, Event};
        with_packaged_resources(|game| {
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
                uqm_engine::test_support::native_session::reload_content_index(
                    &mut game.kernel,
                    "uqm.rmp",
                );
                uqm_engine::test_support::native_probe::flush_graphics();
                observation::drain();
                uqm_engine::test_support::native_session::remove_resource(&mut game.kernel, key);
                assert!(
                    matches!(LanderResources::load(&game.kernel.content()), Err(failed) if failed == key)
                );
                uqm_engine::test_support::native_probe::flush_graphics();
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
            let first = LanderResources::load(&game.kernel.content()).unwrap();
            let second = LanderResources::load(&game.kernel.content()).unwrap();
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
                game.kernel.content(),
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
            radar.set_frame(uqm_engine::graphics::drawable::ContextFrame::retain(
                game.kernel.screen,
            ));
            radar.set_clip(Rect {
                corner: Point { x: 0, y: 0 },
                extent: Extent {
                    width: 56,
                    height: 53,
                },
            });
            let other =
                uqm_engine::graphics::context::LoadedContext::create(c"preserved-selection");
            other.lease().select();
            let selected = uqm_engine::test_support::native_probe::selected_context();
            crate::planet_side::init_lander::draw_lander(
                &radar,
                &assets,
                crate::planet_side::init_lander::LanderStatus::campaign(&game.world),
            );
            assert_eq!(
                uqm_engine::test_support::native_probe::selected_context(),
                selected
            );
            radar.flush();
            let pixels =
                uqm_engine::test_support::native_probe::read_pixels(game.kernel.screen, 320, 240)
                    .unwrap();
            assert_eq!(
                (pixels[0].r, pixels[0].g, pixels[0].b, pixels[0].a),
                (0, 0, 0, 255)
            );
            drop((visuals, other, second));
        });
    }
}
