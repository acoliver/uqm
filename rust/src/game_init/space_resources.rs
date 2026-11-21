//! Space animation ownership, ported from init.c (GPL-2.0-or-later).

fn ship_teardown_needed(kernel: &crate::engine::Kernel, activity: u16) -> bool {
    kernel.ships_initialized
        || activity as u8 == crate::engine::world::activity::activity_kinds::IN_ENCOUNTER as u8
}

pub(crate) mod live {
    use crate::engine::battle::flags::{IN_LAST_BATTLE, RPG_PLAYER_NUM};
    use crate::engine::species::species_id::SPECIES_ID_SIS_SHIP_ID;
    use crate::graphics::{context::ContextLease, drawable::ContextFrame};

    fn remaining_battle_crew(cx: &crate::engine::battle::Cx<'_>) -> u16 {
        let mut crew = 0u16;
        let mut next = cx.elements().head();
        while let Some(id) = next {
            if cx.element(id).flags & crate::engine::battle::flags::CREW_OBJECT as u16 != 0 {
                crew = crew.wrapping_add(1);
            }
            next = cx.elements().succ(id);
        }
        crew
    }

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
        if element.flags & crate::engine::battle::flags::PLAYER_SHIP as u16 == 0
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

    pub fn uninit_ships(cx: &mut crate::engine::battle::Cx<'_>) {
        use crate::engine::world::activity::activity_flags::{CHECK_ABORT, IN_BATTLE};
        use crate::engine::world::activity::activity_kinds::IN_ENCOUNTER;
        if !super::ship_teardown_needed(cx.kernel, cx.world.activity()) {
            return;
        }
        crate::sound::heart_ffi::stop_sound();
        cx.kernel.uninit_space();
        let mut ships = [None; 2];
        let crew = remaining_battle_crew(cx);
        let replacement =
            crate::engine::battle::ElementHook::Native(crate::battle::tactrans_native::new_ship);
        let mut next = cx.elements().head();
        while let Some(id) = next {
            next = cx.elements().succ(id);
            if let Some((ship, descriptor)) = retire_element_ship(cx, id, crew, replacement) {
                ships[cx.ship_state(ship).player as usize] = Some(ship);
                descriptor.unload(cx.world, true);
            }
        }
        let activity = cx.world.activity() & !IN_BATTLE;
        cx.world.set_activity(activity);
        if activity as u8 == IN_ENCOUNTER as u8 && activity & CHECK_ABORT == 0 {
            write_back_surviving_ship_crew(cx, ships);
        }
        for side in 0..2 {
            let mut next = cx.first_ship(side);
            while let Some(ship) = next {
                next = cx.next_ship(ship);
                if let Some(descriptor) = cx.take_descriptor(ship) {
                    descriptor.unload(cx.world, true);
                }
            }
        }
        if cx.world.activity() as u8 != IN_ENCOUNTER as u8 {
            cx.reset_ships(0);
            cx.reset_ships(1);
            if cx.world.in_hqspace() {
                crate::engine::battle::hyperspace::free_hyperspace(cx.kernel);
            }
        }
        cx.complete_ship_reset();
        cx.kernel.ships_initialized = false;
    }

    struct ArenaGraphics {
        status: ContextLease,
        space: ContextLease,
        viewport: Option<(ContextLease, ContextFrame)>,
    }

    impl ArenaGraphics {
        fn retain(kernel: &crate::engine::Kernel, hyperspace: bool) -> Self {
            Self {
                status: ContextLease::retain(
                    kernel
                        .status_context
                        .unwrap_or_else(|| panic!("missing status context")),
                ),
                space: ContextLease::retain(
                    kernel
                        .space_context
                        .unwrap_or_else(|| panic!("missing space context")),
                ),
                viewport: (!hyperspace).then(|| {
                    (
                        ContextLease::retain(
                            kernel
                                .screen_context
                                .unwrap_or_else(|| panic!("missing screen context")),
                        ),
                        ContextFrame::retain(kernel.screen),
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
            self.space.set_clip(crate::graphics::geometry::Rect {
                corner: crate::graphics::geometry::Point { x: 0, y: 0 },
                extent: crate::graphics::geometry::Extent { width, height },
            });
            let black = crate::graphics::geometry::Color {
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
                crate::engine::battle::world_elements::create_asteroid(cx, None);
            }
            crate::engine::battle::world_elements::spawn_planet(cx);
        }
    }

    fn prepare_arena(cx: &mut crate::engine::battle::Cx<'_>) -> Option<(ArenaGraphics, u16, bool)> {
        let activity = cx.world.activity();
        let hyperspace = cx.world.in_hqspace();
        if !cx.kernel.init_space(activity) {
            return None;
        }
        Some((
            ArenaGraphics::retain(cx.kernel, hyperspace),
            activity,
            hyperspace,
        ))
    }

    pub fn init_ships(
        cx: &mut crate::engine::battle::Cx<'_>,
        width: i16,
        height: i16,
        step_zoom: bool,
    ) -> i16 {
        let Some((mut graphics, activity, hyperspace)) = prepare_arena(cx) else {
            return 0;
        };
        graphics.select();
        crate::battle::process_native::init_display_list(cx, step_zoom);
        crate::battle::galaxy::init_galaxy(cx);
        let planet = activity as u8 != IN_LAST_BATTLE as u8;
        graphics.show_viewport(width, height);
        if !hyperspace && !planet {
            cx.kernel.free_gravity_well();
        }
        create_arena_ships(cx, hyperspace, planet);
        cx.kernel.ships_initialized = true;
        if hyperspace {
            1
        } else {
            2
        }
    }

    fn write_back_surviving_ship_crew(
        cx: &mut crate::engine::battle::Cx<'_>,
        ships: [Option<crate::engine::battle::ShipId>; 2],
    ) {
        for (side, ship) in ships.into_iter().enumerate().rev() {
            if let Some(ship) = ship {
                if crate::battle::encount_native::fleet_is_infinite(cx.world, side as u16) == 0 {
                    crate::battle::context_access::reader_legacy::update_ship_frag_crew(cx, ship);
                }
            }
        }
    }

    pub fn init_battle_ships(cx: &mut crate::engine::battle::Cx<'_>) -> i16 {
        let (width, height) = cx.screen_dimensions();
        let players = init_ships(
            cx,
            (width - 64) as i16,
            height as i16,
            cx.kernel.configuration.preferences().melee_scale == 0,
        );
        if players == 1 {
            crate::navigation::hyper_data::load_hyperspace(cx, |cx, operation| {
                use crate::navigation::hyper_data::EntryDisplay;
                if let EntryDisplay::Palette(quasispace) = operation {
                    crate::engine::battle::hyperspace::select_palette(cx.kernel, quasispace);
                    return;
                }
                let dimensions = cx.screen_dimensions();
                match operation {
                    EntryDisplay::RepairBorder => {
                        crate::ui::sis::repair_sis_border(cx.kernel, (dimensions.0, dimensions.1))
                    }
                    EntryDisplay::DrawFrame => {
                        crate::ui::border::draw_sis_frame(cx.kernel, cx.world, dimensions)
                    }
                    EntryDisplay::DrawGauges => crate::ui::sis::clear_sis_rect(
                        cx.kernel,
                        cx.world,
                        (dimensions.0, dimensions.1),
                        crate::ui::sis::DRAW_SIS_DISPLAY,
                    ),
                    EntryDisplay::ClearMessage => crate::ui::sis::draw_campaign_message(
                        cx.kernel,
                        cx.world,
                        dimensions.0,
                        None,
                    ),
                    EntryDisplay::ClearRadar => crate::ui::sis::clear_sis_rect(
                        cx.kernel,
                        cx.world,
                        (dimensions.0, dimensions.1),
                        crate::ui::sis::CLEAR_SIS_RADAR,
                    ),
                    EntryDisplay::Palette(_) => unreachable!(),
                };
            });
        }
        players
    }
}

pub use live::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::battle::record::recover_ship_crew;
    use crate::engine::battle::record::RaceDescriptor;

    #[test]
    fn teardown_guard_belongs_to_the_kernel_and_preserves_encounter_recovery() {
        let mut first = crate::engine::Game::new();
        let second = crate::engine::Game::new();
        for activity in [0, 1, 3, 0x101, 0x103] {
            assert!(!ship_teardown_needed(&first.kernel, activity));
        }
        assert!(ship_teardown_needed(&first.kernel, 2));
        assert!(ship_teardown_needed(&first.kernel, 0x102));
        first.kernel.ships_initialized = true;
        for activity in [0, 1, 2, 3, 0x101, 0x103] {
            assert!(ship_teardown_needed(&first.kernel, activity));
        }
        assert!(!ship_teardown_needed(&second.kernel, 1));
    }

    #[test]
    fn battle_survivor_retains_native_descriptor_crew_and_recovers_pods() {
        let mut descriptor = RaceDescriptor::EMPTY;
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
