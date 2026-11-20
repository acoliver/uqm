//! Space animation ownership, ported from init.c (GPL-2.0-or-later).

fn ship_teardown_needed(kernel: &uqm_engine::Kernel, activity: u16) -> bool {
    kernel.ships_initialized
        || activity as u8 == uqm_engine::world::activity::activity_kinds::IN_ENCOUNTER as u8
}

pub(crate) mod live {
    use uqm_engine::battle::flags::{IN_LAST_BATTLE, RPG_PLAYER_NUM};
    use uqm_engine::graphics::{context::ContextLease, drawable::ContextFrame};
    use uqm_engine::species::species_id::SPECIES_ID_SIS_SHIP_ID;

    fn remaining_battle_crew(cx: &uqm_engine::battle::Cx<'_>) -> u16 {
        let mut crew = 0u16;
        let mut next = cx.elements().head();
        while let Some(id) = next {
            if cx.element(id).flags & uqm_engine::battle::flags::CREW_OBJECT as u16 != 0 {
                crew = crew.wrapping_add(1);
            }
            next = cx.elements().succ(id);
        }
        crew
    }

    fn retire_element_ship(
        cx: &mut uqm_engine::battle::Cx<'_>,
        id: uqm_engine::battle::ElementId,
        crew: u16,
        replacement: uqm_engine::battle::ElementHook,
    ) -> Option<(uqm_engine::battle::ShipId, uqm_engine::battle::RaceDesc)> {
        use uqm_engine::battle::Parent;
        let element = cx.element(id);
        if element.flags & uqm_engine::battle::flags::PLAYER_SHIP as u16 == 0
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

    pub fn uninit_ships(cx: &mut uqm_engine::battle::Cx<'_>) {
        use uqm_engine::world::activity::activity_flags::{CHECK_ABORT, IN_BATTLE};
        use uqm_engine::world::activity::activity_kinds::IN_ENCOUNTER;
        if !super::ship_teardown_needed(cx.kernel, cx.world.activity()) {
            return;
        }
        uqm_engine::sound::heart_ffi::stop_sound();
        cx.kernel.uninit_space();
        let mut ships = [None; 2];
        let crew = remaining_battle_crew(cx);
        let replacement =
            uqm_engine::battle::ElementHook::Native(uqm_modes::battle::tactrans_native::new_ship);
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
                uqm_engine::battle::hyperspace::free_hyperspace(cx.kernel);
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
        fn retain(kernel: &uqm_engine::Kernel, hyperspace: bool) -> Self {
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
            self.space.set_clip(uqm_engine::graphics::geometry::Rect {
                corner: uqm_engine::graphics::geometry::Point { x: 0, y: 0 },
                extent: uqm_engine::graphics::geometry::Extent { width, height },
            });
            let black = uqm_engine::graphics::geometry::Color {
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

    fn create_arena_ships(cx: &mut uqm_engine::battle::Cx<'_>, hyperspace: bool, planet: bool) {
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
                uqm_engine::battle::world_elements::create_asteroid(cx, None);
            }
            uqm_engine::battle::world_elements::spawn_planet(cx);
        }
    }

    fn prepare_arena(cx: &mut uqm_engine::battle::Cx<'_>) -> Option<(ArenaGraphics, u16, bool)> {
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
        cx: &mut uqm_engine::battle::Cx<'_>,
        width: i16,
        height: i16,
        step_zoom: bool,
    ) -> i16 {
        let Some((mut graphics, activity, hyperspace)) = prepare_arena(cx) else {
            return 0;
        };
        graphics.select();
        uqm_modes::battle::process_native::init_display_list(cx, step_zoom);
        uqm_modes::battle::galaxy::init_galaxy(cx);
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
        cx: &mut uqm_engine::battle::Cx<'_>,
        ships: [Option<uqm_engine::battle::ShipId>; 2],
    ) {
        for (side, ship) in ships.into_iter().enumerate().rev() {
            if let Some(ship) = ship {
                if uqm_modes::battle::encount_native::fleet_is_infinite(cx.world, side as u16) == 0
                {
                    uqm_modes::battle::context_access::reader_legacy::update_ship_frag_crew(
                        cx, ship,
                    );
                }
            }
        }
    }

    pub fn init_battle_ships(cx: &mut uqm_engine::battle::Cx<'_>) -> i16 {
        let (width, height) = cx.screen_dimensions();
        let players = init_ships(
            cx,
            (width - 64) as i16,
            height as i16,
            cx.kernel.configuration.preferences().melee_scale == 0,
        );
        if players == 1 {
            uqm_modes::navigation::hyper_data::load_hyperspace(cx, |cx, operation| {
                use uqm_modes::navigation::hyper_data::EntryDisplay;
                if let EntryDisplay::Palette(quasispace) = operation {
                    uqm_engine::battle::hyperspace::select_palette(cx.kernel, quasispace);
                    return;
                }
                let dimensions = cx.screen_dimensions();
                match operation {
                    EntryDisplay::RepairBorder => uqm_modes::ui::sis::repair_sis_border(
                        cx.kernel,
                        (dimensions.0, dimensions.1),
                    ),
                    EntryDisplay::DrawFrame => {
                        uqm_modes::ui::border::draw_sis_frame(cx.kernel, cx.world, dimensions)
                    }
                    EntryDisplay::DrawGauges => uqm_modes::ui::sis::clear_sis_rect(
                        cx.kernel,
                        cx.world,
                        (dimensions.0, dimensions.1),
                        uqm_modes::ui::sis::DRAW_SIS_DISPLAY,
                    ),
                    EntryDisplay::ClearMessage => uqm_modes::ui::sis::draw_campaign_message(
                        cx.kernel,
                        cx.world,
                        dimensions.0,
                        None,
                    ),
                    EntryDisplay::ClearRadar => uqm_modes::ui::sis::clear_sis_rect(
                        cx.kernel,
                        cx.world,
                        (dimensions.0, dimensions.1),
                        uqm_modes::ui::sis::CLEAR_SIS_RADAR,
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
    use uqm_engine::battle::record::recover_ship_crew;
    use uqm_engine::battle::record::RaceDescriptor;

    #[test]
    fn teardown_guard_belongs_to_the_kernel_and_preserves_encounter_recovery() {
        let mut first = uqm_engine::Game::new();
        let second = uqm_engine::Game::new();
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
