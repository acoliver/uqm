//! Ship runtime operations formerly supplied by rust_bridge_ships.c.

#[cfg(not(test))]
fn remaining_battle_crew(cx: &crate::engine::battle::Cx<'_>) -> u16 {
    let mut crew = 0u16;
    let mut next = cx.elements().head();
    while let Some(id) = next {
        if cx.element(id).flags & crate::ships::native::abi::CREW_OBJECT as u16 != 0 {
            crew = crew.wrapping_add(1);
        }
        next = cx.elements().succ(id);
    }
    crew
}

#[cfg(not(test))]
fn retire_element_ship(
    cx: &mut crate::engine::battle::Cx<'_>,
    id: crate::engine::battle::ElementId,
    crew: u16,
    replacement: crate::engine::battle::ElementHook,
) -> Option<(
    crate::engine::battle::ShipId,
    crate::engine::battle::RaceDesc,
)> {
    use crate::engine::battle::Parent;
    let element = cx.element(id);
    if element.flags & crate::ships::native::abi::PLAYER_SHIP as u16 == 0
        && !element
            .death
            .is_some_and(|callback| callback.address() == replacement.address())
    {
        return None;
    }
    let Parent::Ship(ship) = cx.parent(id) else {
        return None;
    };
    let mut descriptor = cx.take_descriptor(ship)?;
    cx.ship_state_mut(ship).crew = descriptor.recover_crew(crew);
    Some((ship, descriptor))
}

/// # Safety
/// Call on the game thread while the battle queues and their resources remain live.
#[cfg(not(test))]
pub unsafe fn uninit_ships() {
    use crate::ships::native::abi::*;

    StopSound();
    crate::app::bridge::enter(|cx| {
        crate::game_init::space_resources::uninit_space(cx.kernel);
        let mut ships = [None; 2];
        let crew = remaining_battle_crew(cx);
        let replacement =
            crate::engine::battle::ElementHook::Native(crate::battle::tactrans_native::new_ship);
        let mut next = cx.elements().head();
        while let Some(id) = next {
            next = cx.elements().succ(id);
            if let Some((ship, descriptor)) = retire_element_ship(cx, id, crew, replacement) {
                ships[cx.ship_state(ship).player as usize] = Some(ship);
                descriptor.unload(cx.world, true, true);
            }
        }
        let activity = cx.world.activity() & !(IN_BATTLE as u16);
        cx.world.set_activity(activity);
        if activity as u8 == IN_ENCOUNTER as u8 && activity & CHECK_ABORT as u16 == 0 {
            for i in (0..2).rev() {
                if let Some(ship) = ships[i].filter(|_| fleet_is_infinite(cx.world, i as u16) == 0)
                {
                    crate::battle::encount_native::engine::update_ship_frag_crew(cx, ship);
                }
            }
        }
        for side in 0..2 {
            let mut next = cx.first_ship(side);
            while let Some(ship) = next {
                next = cx.next_ship(ship);
                if let Some(descriptor) = cx.take_descriptor(ship) {
                    descriptor.unload(cx.world, true, true);
                }
            }
        }
        if cx.world.activity() as u8 != IN_ENCOUNTER as u8 {
            cx.reset_ships(0);
            cx.reset_ships(1);
            if in_hqspace(cx.world) != 0 {
                crate::navigation::hyper_data::FreeHyperspace();
            }
        }
        cx.complete_ship_reset();
    });
}

#[cfg(not(test))]
mod live {
    use crate::ships::native::abi::*;

    use crate::graphics::context_abi::{ContextFrame, ContextLease};

    struct ArenaGraphics {
        status: ContextLease,
        space: ContextLease,
        viewport: Option<(ContextLease, ContextFrame)>,
    }

    impl ArenaGraphics {
        fn retain(kernel: &crate::engine::Kernel, hyperspace: bool) -> Self {
            Self {
                status: ContextLease::retain(
                    std::ptr::NonNull::new(kernel.status_context)
                        .unwrap_or_else(|| panic!("missing status context")),
                ),
                space: ContextLease::retain(
                    std::ptr::NonNull::new(kernel.space_context)
                        .unwrap_or_else(|| panic!("missing space context")),
                ),
                viewport: (!hyperspace).then(|| {
                    (
                        ContextLease::retain(
                            std::ptr::NonNull::new(kernel.screen_context)
                                .unwrap_or_else(|| panic!("missing screen context")),
                        ),
                        ContextFrame::retain(std::ptr::NonNull::new(kernel.screen)),
                    )
                }),
            }
        }

        fn select(&self) {
            self.status.select();
            self.space.select();
        }

        fn show_viewport(&mut self, width: i16, height: i16) {
            let Some((screen, frame)) = self.viewport.take() else {
                return;
            };
            self.space.set_frame(frame);
            self.space.set_clip(crate::graphics::native_abi::RECT {
                corner: crate::graphics::native_abi::POINT { x: 0, y: 0 },
                extent: crate::graphics::native_abi::EXTENT { width, height },
            });
            let black = crate::graphics::native_abi::Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            };
            self.space.set_background(black);
            screen.set_background(black);
            screen.clear();
            self.space.select();
        }
    }

    fn create_arena_ships(cx: &mut crate::engine::battle::Cx<'_>, hyperspace: bool, planet: bool) {
        if hyperspace {
            cx.reset_ships(0);
            cx.reset_ships(1);
            let mut ships = cx.kernel.ships(0);
            if let Some(flagship) = ships.build(SPECIES_ID_SIS_SHIP_ID) {
                ships.edit(flagship, |ship| {
                    ship.player = RPG_PLAYER_NUM as i16;
                    ship.captain = 0;
                });
            }
        } else if planet {
            for _ in 0..5 {
                crate::battle::world_elements::create_asteroid(cx, None);
            }
            crate::battle::world_elements::spawn_planet(cx);
        }
    }

    fn prepare_arena(cx: &mut crate::engine::battle::Cx<'_>) -> Option<(ArenaGraphics, u16, bool)> {
        let activity = cx.world.activity();
        let hyperspace = in_hqspace(cx.world) != 0;
        if crate::game_init::space_resources::init_space(cx.kernel, activity) == 0 {
            return None;
        }
        Some((
            ArenaGraphics::retain(cx.kernel, hyperspace),
            activity,
            hyperspace,
        ))
    }

    /// # Safety
    /// Called on the game thread with live display, galaxy and hyperspace resources.
    pub unsafe fn rust_bridge_init_battle_arena() -> i16 {
        let Some((mut graphics, activity, hyperspace)) = crate::app::bridge::enter(prepare_arena)
        else {
            return 0;
        };
        graphics.select();
        crate::app::bridge::enter(|cx| {
            crate::battle::process_native::init_display_list(cx, optMeleeScale == 0)
        });
        crate::app::bridge::enter(crate::battle::galaxy::init_galaxy);
        let planet = activity as u8 != IN_LAST_BATTLE as u8;
        graphics.show_viewport((ScreenWidth - 64) as i16, ScreenHeight as i16);
        crate::app::bridge::enter(|cx| {
            if !hyperspace && !planet {
                crate::game_init::world_resources::free_gravity_well(cx.kernel);
            }
            create_arena_ships(cx, hyperspace, planet);
        });
        if hyperspace {
            crate::navigation::hyper_data::LoadHyperspace();
            1
        } else {
            2
        }
    }
}
#[cfg(not(test))]
pub use live::*;

#[cfg(test)]
mod crew_writeback_tests {
    use crate::engine::battle::record::recover_ship_crew;
    use crate::ships::native::abi::RaceDescriptor;

    #[test]
    fn battle_survivor_retains_native_descriptor_crew_and_recovers_pods() {
        let mut descriptor: RaceDescriptor = unsafe { std::mem::zeroed() };
        descriptor.ship_info.crew_level = 7;
        descriptor.ship_info.max_crew = 18;
        assert_eq!(
            recover_ship_crew(
                &mut descriptor.ship_info.crew_level,
                descriptor.ship_info.max_crew,
                3
            ),
            10
        );
        assert_eq!(descriptor.ship_info.crew_level, 10);
        assert_eq!(descriptor.ship_info.max_crew, 18);
        assert_eq!(
            recover_ship_crew(
                &mut descriptor.ship_info.crew_level,
                descriptor.ship_info.max_crew,
                20
            ),
            18
        );
        descriptor.ship_info.crew_level = 0;
        assert_eq!(
            recover_ship_crew(
                &mut descriptor.ship_info.crew_level,
                descriptor.ship_info.max_crew,
                20
            ),
            0
        );
    }
}
