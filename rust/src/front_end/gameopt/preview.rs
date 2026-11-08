//! Temporary flagship and escort state while rendering a saved-game summary.

use crate::collections::pool::OrderedPool;
use crate::engine::world::{queues::ShipFragment, SisState};
use crate::engine::World;

pub(super) struct SavedState {
    flagship: SisState,
    ships: OrderedPool<ShipFragment, 12>,
}

impl SavedState {
    pub(super) fn capture(world: &mut World) -> Self {
        Self {
            flagship: world.data.flagship,
            ships: std::mem::take(&mut world.queues.built_ships),
        }
    }

    pub(super) fn restore(self, world: &mut World) {
        world.data.flagship = self.flagship;
        world.queues.built_ships = self.ships;
    }
}

/// # Safety
/// Race templates must be live and SavedState must hold the campaign escorts.
#[cfg(not(test))]
pub(super) unsafe fn install(world: &mut World, summary: &crate::navigation::abi::SUMMARY_DESC) {
    world.data.flagship = summary.SS;
    for &species in &summary.ShipList[..usize::from(summary.NumShips)] {
        let fleets = world.avail_races_ref();
        let Some(id) = fleets.at(usize::from(species)) else {
            continue;
        };
        let template = *fleets.get(id);
        world.built_ships().clone_fragment(
            &template,
            species,
            0,
            crate::comm::dispatch::production_captain_name,
        );
    }
}

pub(super) fn lander(
    kernel: &crate::engine::Kernel,
    summary: &crate::navigation::abi::SUMMARY_DESC,
) {
    if let Some(owner) = &kernel.lander_resources {
        let assets = crate::planet_side::resources::PlanetSideAssets::retain(owner);
        crate::planet_side::init_lander::draw_lander(
            &crate::graphics::context_abi::ContextLease::require(kernel.radar_context),
            &assets,
            crate::planet_side::init_lander::LanderStatus::preview(&summary.SS, summary.Flags),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn lander_preview_draws_saved_values_without_mutating_the_current_flagship() {
        crate::planet_side::resources::tests::with_packaged_resources(|game| {
            use crate::graphics::{
                context_abi::{ContextFrame, ContextLease},
                native_abi::{Color, EXTENT, POINT, RECT},
            };
            game.kernel.radar_context = game.kernel.status_context;
            let radar = ContextLease::require(game.kernel.radar_context);
            radar.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
                game.kernel.screen,
            )));
            radar.set_clip(RECT {
                corner: POINT { x: 0, y: 0 },
                extent: EXTENT {
                    width: 56,
                    height: 53,
                },
            });
            let mut summary = crate::navigation::abi::SUMMARY_DESC {
                SS: game.world.data.flagship,
                Activity: 0,
                Flags: 0,
                day_index: 0,
                month_index: 0,
                year_index: 0,
                MCreditLo: 0,
                MCreditHi: 0,
                NumShips: 0,
                NumDevices: 0,
                ShipList: [0; 12],
                DeviceList: [0; 16],
                SaveName: [0; 64],
            };
            lander(&game.kernel, &summary);
            game.kernel.lander_resources =
                Some(crate::engine::kernel::lander_resources::LanderResources::load().unwrap());
            let current = game.world.data.flagship;
            let mut saved = current;
            saved.num_landers = 1;
            saved.module_slots[0] = crate::starbase::sis_modules::ModuleType::StorageBay as u8;
            saved.total_element_mass = 475;
            summary.SS = saved;
            summary.Flags = 0x52;
            let source = game
                .kernel
                .lander_resources
                .as_ref()
                .unwrap()
                .graphic(0)
                .wrapped_frame(32);
            let extent = source.rect().extent;
            let lit = (0..extent.height)
                .flat_map(|y| (0..extent.width).map(move |x| POINT { x, y }))
                .filter(|point| {
                    let pixel = source.pixel(*point);
                    pixel.r != 0 || pixel.g != 0 || pixel.b != 0
                })
                .count();
            // SAFETY: the serialized fixture owns graphics admission and its selected target.
            let admission = unsafe {
                (
                    crate::graphics::context_abi::_GraphicsStatusFlags,
                    crate::graphics::drawable_abi::_CurFramePtr == game.kernel.screen,
                    crate::graphics::gfx_common_abi::GetGraphicScale(),
                )
            };
            let target = ContextFrame::retain(std::ptr::NonNull::new(game.kernel.screen)).rect();
            let clip = radar.clip_rect();
            eprintln!(
                "preview source {}x{}, {lit} colored pixels; admission {admission:?}; target ({},{}) {}x{}; clip ({},{}) {}x{}",
                extent.width, extent.height,
                target.corner.x, target.corner.y, target.extent.width, target.extent.height,
                clip.corner.x, clip.corner.y, clip.extent.width, clip.extent.height,
            );
            lander(&game.kernel, &summary);
            radar.flush();
            let mut pixels = vec![Color::default(); 320 * 240];
            assert_ne!(
                // SAFETY: the supplied target and complete buffer remain live through readback.
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
            assert!(pixels
                .iter()
                .any(|pixel| pixel.r != 0 || pixel.g != 0 || pixel.b != 0));
            assert_eq!(game.world.data.flagship.num_landers, current.num_landers);
            assert_eq!(
                game.world.data.flagship.total_element_mass,
                current.total_element_mass
            );
            assert_eq!(game.world.data.flagship.module_slots, current.module_slots);
        });
    }

    #[test]
    fn restore_reinstates_the_flagship_and_escorts_without_rewinding_other_state() {
        let mut world = crate::engine::World::new();
        world.data.flagship.crew_enlisted = 20;
        let mut ships = world.built_ships();
        let first = ships.alloc().unwrap();
        ships.get_mut(first).crew_level = 9;
        ships.put(first);
        let second = ships.alloc().unwrap();
        ships.put(second);
        let saved = SavedState::capture(&mut world);
        assert_eq!(world.built_ships_ref().count(), 0);
        assert!(!world.built_ships_ref().contains(first));

        world.data.flagship.crew_enlisted = 5;
        let temporary = world.built_ships().alloc().unwrap();
        world.built_ships().put(temporary);
        world.data.game.glob_flags = 7;
        saved.restore(&mut world);

        assert_eq!(world.data.flagship.crew_enlisted, 20);
        let ships = world.built_ships_ref();
        assert_eq!(ships.count(), 2);
        assert_eq!(ships.head(), Some(first));
        assert_eq!(ships.succ(first), Some(second));
        assert_eq!(ships.get(first).crew_level, 9);
        assert!(!ships.contains(temporary));
        assert_eq!(world.data.game.glob_flags, 7);
    }
}
