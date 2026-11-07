//! Space animation ownership, ported from init.c (GPL-2.0-or-later).

#[cfg(not(test))]
use crate::collections::observation::observe_views;

const SPACE_ANIMATION_KEYS: [[&std::ffi::CStr; 3]; 3] = [
    [
        c"graphics.boom.large",
        c"graphics.boom.medium",
        c"graphics.boom.small",
    ],
    [
        c"graphics.blast.large",
        c"graphics.blast.medium",
        c"graphics.blast.small",
    ],
    [
        c"graphics.asteroid.large",
        c"graphics.asteroid.medium",
        c"graphics.asteroid.small",
    ],
];

fn ship_teardown_needed(kernel: &crate::engine::Kernel, activity: u16) -> bool {
    kernel.ships_initialized || activity as u8 == crate::ships::native::abi::IN_ENCOUNTER as u8
}

#[cfg(not(test))]
use crate::engine::battle::frames::{free_image, load_animation, load_graphic};

#[cfg(not(test))]
pub(crate) mod live {
    use super::*;
    use crate::ships::native::abi::{
        in_hqspace, IN_LAST_BATTLE, RPG_PLAYER_NUM, SPECIES_ID_SIS_SHIP_ID,
    };

    pub fn init_space(kernel: &mut crate::engine::Kernel, activity: u16) -> u32 {
        {
            let old = kernel.space_init_count;
            kernel.space_init_count = kernel.space_init_count.wrapping_add(1);
            if old == 0 && activity as u8 <= crate::ships::native::abi::IN_ENCOUNTER as u8 {
                let stars = &mut kernel.stars_in_space;
                *stars = load_graphic(c"graphics.stars");
                if stars.is_null() {
                    return 0;
                }
                observe_views(std::slice::from_ref(stars), "engine-global:stars", |f| {
                    *f as usize
                });
                if [
                    &mut kernel.explosion,
                    &mut kernel.blast,
                    &mut kernel.asteroid,
                ]
                .into_iter()
                .zip(SPACE_ANIMATION_KEYS)
                .zip([
                    "engine-global:explosion",
                    "engine-global:blast",
                    "engine-global:asteroid",
                ])
                .any(|((frames, [big, medium, small]), role)| {
                    load_animation(frames, [Some(big), Some(medium), Some(small)], role) == 0
                }) {
                    return 0;
                }
            }
            crate::collections::observation::record(
                crate::collections::observation::Event::SpaceActive(true),
            );
            1
        }
    }

    pub fn uninit_space(kernel: &mut crate::engine::Kernel) {
        {
            if kernel.space_init_count != 0 {
                kernel.space_init_count -= 1;
                if kernel.space_init_count == 0 {
                    crate::collections::observation::record(
                        crate::collections::observation::Event::SpaceActive(false),
                    );
                    free_image(&mut kernel.blast, "engine-global:blast");
                    free_image(&mut kernel.explosion, "engine-global:explosion");
                    free_image(&mut kernel.asteroid, "engine-global:asteroid");
                    let stars = &mut kernel.stars_in_space;
                    for plane in kernel.star_planes.iter_mut() {
                        plane
                            .frames
                            .fill(crate::engine::battle::frames::FrameSelection::ABSENT);
                    }
                    free_image(std::array::from_mut(stars), "engine-global:stars");
                }
            }
        }
    }

    use crate::graphics::context_abi::{ContextFrame, ContextLease};

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

    pub fn uninit_ships(cx: &mut crate::engine::battle::Cx<'_>) {
        use crate::ships::native::abi::*;
        if !super::ship_teardown_needed(cx.kernel, cx.world.activity()) {
            return;
        }
        crate::sound::heart_ffi::stop_sound();
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
            write_back_surviving_ship_crew(cx, ships);
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
                crate::navigation::hyper_data::free_hyperspace(cx.kernel);
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
            crate::game_init::world_resources::free_gravity_well(cx.kernel);
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

    unsafe fn init_published_ships() -> i16 {
        use crate::ships::native::abi::{ScreenHeight, ScreenWidth};
        crate::app::bridge::enter(|cx| {
            let players = init_ships(
                cx,
                (ScreenWidth - 64) as i16,
                ScreenHeight as i16,
                cx.kernel.configuration.preferences().melee_scale == 0,
            );
            if players == 1 {
                crate::navigation::hyper_data::load_hyperspace(cx, |cx, operation| {
                    use crate::navigation::hyper_data::EntryDisplay;
                    if let EntryDisplay::Palette(quasispace) = operation {
                        crate::engine::battle::hyperspace::select_palette(cx.kernel, quasispace);
                        return;
                    }
                    cx.with_legacy(|| match operation {
                        EntryDisplay::RepairBorder => crate::ui::sis::repair_sisborder(),
                        EntryDisplay::DrawFrame => crate::ui::border::draw_sisframe(),
                        EntryDisplay::DrawGauges => crate::ui::sis::ClearSISRect(
                            crate::navigation::abi::DRAW_SIS_DISPLAY as u8,
                        ),
                        EntryDisplay::ClearMessage => {
                            crate::ui::sis::DrawSISMessage(std::ptr::null())
                        }
                        EntryDisplay::ClearRadar => crate::ui::sis::ClearSISRect(
                            crate::navigation::abi::CLEAR_SIS_RADAR as u8,
                        ),
                        EntryDisplay::Palette(_) => unreachable!(),
                    });
                });
            }
            players
        })
    }

    #[no_mangle]
    pub extern "C" fn InitShips() -> i16 {
        // SAFETY: this existing ABI entry runs with a published game-thread context before battle.
        unsafe { init_published_ships() }
    }
    #[no_mangle]
    pub extern "C" fn UninitShips() {
        // SAFETY: the battle loop has relinquished Cx before this teardown entry.
        unsafe { crate::app::bridge::enter(uninit_ships) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::battle::record::recover_ship_crew;
    use crate::ships::native::abi::RaceDescriptor;

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
    #[test]
    fn animation_keys_keep_explosion_blast_asteroid_order_and_three_sizes() {
        assert_eq!(SPACE_ANIMATION_KEYS[0][0], c"graphics.boom.large");
        assert_eq!(SPACE_ANIMATION_KEYS[1][1], c"graphics.blast.medium");
        assert_eq!(SPACE_ANIMATION_KEYS[2][2], c"graphics.asteroid.small");
    }
}

#[cfg(not(test))]
pub use live::*;
