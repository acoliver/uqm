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

pub(super) fn with_preview<R>(
    world: &mut World,
    summary: &crate::engine::save::summary_record::SummaryDesc,
    mut random: impl FnMut() -> u32,
    draw: impl FnOnce(&mut World) -> R,
) -> R {
    struct Restore<'a> {
        world: &'a mut World,
        state: Option<SavedState>,
    }
    impl Drop for Restore<'_> {
        fn drop(&mut self) {
            self.state.take().unwrap().restore(self.world);
        }
    }
    let state = SavedState::capture(world);
    let restore = Restore {
        world,
        state: Some(state),
    };
    restore.world.data.flagship = summary.sis;
    for &species in &summary.ship_list[..usize::from(summary.num_ships)] {
        let fleets = restore.world.avail_races_ref();
        let Some(id) = fleets.at(usize::from(species)) else {
            continue;
        };
        let template = *fleets.get(id);
        restore
            .world
            .built_ships()
            .clone_fragment(&template, species, 0, || {
                crate::ships::queue::pick_captain_name(&mut random)
            });
    }
    draw(restore.world)
}

pub(super) fn lander(
    kernel: &crate::engine::Kernel,
    summary: &crate::engine::save::summary_record::SummaryDesc,
) {
    if let Some(owner) = &kernel.lander_resources {
        let assets = crate::planet_side::resources::PlanetSideAssets::retain(owner);
        crate::planet_side::init_lander::draw_lander(
            &crate::graphics::context_abi::ContextLease::require(kernel.radar_context),
            &assets,
            crate::planet_side::init_lander::LanderStatus::preview(&summary.sis, summary.flags),
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
            let mut summary = crate::engine::save::summary_record::SummaryDesc {
                sis: game.world.data.flagship,
                activity: 0,
                flags: 0,
                day_index: 0,
                month_index: 0,
                year_index: 0,
                mcredit_lo: 0,
                mcredit_hi: 0,
                num_ships: 0,
                num_devices: 0,
                ship_list: [0; 12],
                device_list: [0; 16],
                save_name: [0; 64],
            };
            lander(&game.kernel, &summary);
            game.kernel.lander_resources =
                Some(crate::engine::kernel::lander_resources::LanderResources::load().unwrap());
            let current = game.world.data.flagship;
            let mut saved = current;
            saved.num_landers = 1;
            saved.module_slots[0] = crate::starbase::sis_modules::ModuleType::StorageBay as u8;
            saved.total_element_mass = 475;
            summary.sis = saved;
            summary.flags = 0x52;
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
    fn preview_keeps_duplicate_captain_draw_order_and_restores_on_unwind() {
        let mut world = World::new();
        let fleet = world.avail_races().alloc().unwrap();
        world.avail_races().get_mut(fleet).species_id = 0;
        world.avail_races().put(fleet);
        let original = world.built_ships().alloc().unwrap();
        world.built_ships().get_mut(original).crew_level = 42;
        world.built_ships().put(original);
        world.data.flagship.crew_enlisted = 17;
        let mut summary = crate::engine::save::summary_record::SummaryDesc::default();
        summary.sis.crew_enlisted = 9;
        summary.num_ships = 2;
        let mut draws = [4, 4, 5].into_iter();
        let seen = std::cell::Cell::new(0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_preview(
                &mut world,
                &summary,
                || {
                    seen.set(seen.get() + 1);
                    draws.next().expect("unexpected preview RNG draw")
                },
                |world| {
                    assert_eq!(seen.get(), 3);
                    assert_eq!(world.data.flagship.crew_enlisted, 9);
                    let ships = world.built_ships_ref();
                    assert_eq!(ships.count(), 2);
                    let first = ships.head().unwrap();
                    let second = ships.succ(first).unwrap();
                    assert_eq!(ships.get(first).captains_name_index, 9);
                    assert_eq!(ships.get(second).captains_name_index, 10);
                    panic!("drawing failed");
                },
            );
        }));
        assert!(result.is_err());
        assert_eq!(seen.get(), 3);
        assert_eq!(world.data.flagship.crew_enlisted, 17);
        assert_eq!(world.built_ships_ref().head(), Some(original));
        assert_eq!(world.built_ships_ref().get(original).crew_level, 42);
        assert_eq!(world.built_ships_ref().count(), 1);
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
