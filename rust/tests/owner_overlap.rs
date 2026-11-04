//! Legacy callers may borrow disjoint fields without reacquiring their owner.
#[test]
#[serial_test::serial]
fn battle_ditty_stop_uses_the_already_borrowed_kernel() {
    let mut game = uqm_rust::engine::Game::new();
    // SAFETY: this serialized fixture owns the published game and inactive audio.
    unsafe {
        uqm_rust::app::bridge::with_game(&mut game, || {
            uqm_rust::engine::Kernel::current().write(|kernel| {
                uqm_rust::battle::tactrans_native::stop_ditty(kernel);
            });
        });
    }
}

#[test]
#[serial_test::serial]
fn battle_song_release_uses_the_already_borrowed_kernel() {
    let mut game = uqm_rust::engine::Game::new();
    // SAFETY: this serialized fixture owns the published game and inactive audio.
    unsafe {
        uqm_rust::app::bridge::with_game(&mut game, || {
            uqm_rust::engine::Kernel::current().write(|kernel| {
                uqm_rust::battle::battle_native::free_battle_song(kernel);
                uqm_rust::battle::battle_native::free_battle_song(kernel);
            });
        });
    }
}

use std::mem;
use uqm_rust::{
    app::bridge,
    engine::{Game, World},
    ships::native::abi::*,
};

#[test]
fn public_campaign_copy_does_not_traverse_live_records() {
    use uqm_rust::automation::campaign_state::{CampaignView, LiveCampaign, StateKey, StateSource};
    let mut world = World::new();
    world.story = Some(uqm_rust::state::game_state::GameState::new());
    world.data.game.ship_stamp.frame = std::ptr::dangling_mut();
    let id = world.npc_built_ships().alloc().unwrap();
    world.npc_built_ships().put(id);
    let id = world.built_ships().alloc().unwrap();
    world.built_ships().put(id);
    let id = world.ip_groups().alloc().unwrap();
    world.ip_groups().put(id);
    let view = CampaignView::from_world(&world);
    assert_eq!(
        LiveCampaign {
            world: &view,
            flagship_destroyed: false
        }
        .read(StateKey::Story("STARBASE_AVAILABLE")),
        Some(0)
    );
}

#[test]
#[serial_test::serial]
fn empty_npc_phrase_does_not_acquire_an_owner() {
    let mut game = Game::new();
    // SAFETY: this serialized fixture publishes its game, and phrase zero is a no-op.
    unsafe {
        bridge::with_game(&mut game, || {
            World::current().write(|_| uqm_rust::comm::ffi::rust_npc_phrase_cb(0, None));
        });
    }
}

#[test]
#[serial_test::serial]
fn sis_slot_snapshot_releases_world_before_support_ship_drawing() {
    let mut game = Game::new();
    game.world.data.flagship.module_slots.fill(255);
    game.world.data.flagship.drive_slots.fill(255);
    game.world.data.flagship.jet_slots.fill(255);
    game.world.data.flagship.crew_enlisted = 12;
    // SAFETY: the serialized fixture owns the lifecycle; absent render handles
    // are valid for the inactive graphics backend used by this test.
    unsafe {
        bridge::with_game(&mut game, || {
            uqm_rust::ui::sis::DeltaSISGauges(i16::MAX, i16::MAX, i32::from(i16::MAX));
        });
    }
    assert_eq!(game.world.data.flagship.crew_enlisted, 12);
    assert_eq!(game.world.data.flagship.module_slots, [255; 16]);
}

#[test]
#[serial_test::serial]
fn summary_picker_clones_from_the_already_borrowed_race_queue() {
    let mut game = Game::new();
    // SAFETY: this fixture exclusively owns both initialized queues for the publication.
    unsafe {
        bridge::with_game(&mut game, || {
            World::current().write(|world| {
                let template = {
                    let mut races = world.avail_races();
                    let id = races.alloc().unwrap();
                    let template = races.get_mut(id);
                    template.species_id = SPECIES_ID_EARTHLING_ID;
                    template.crew_level = 12;
                    template.max_crew = 18;
                    races.put(id);
                    *races.get(id)
                };
                let mut destination = world.built_ships();
                let id = destination.clone_fragment(&template, 0, 0, || 5).unwrap();
                let cloned = destination.get(id);
                assert_eq!(cloned.crew_level, 12);
                assert_eq!(cloned.max_crew, 18);
                assert_eq!(cloned.species_id, SPECIES_ID_EARTHLING_ID);
            });
        });
    }
}

#[test]
#[serial_test::serial]
fn restored_star_lookup_finishes_before_selecting_the_owned_star() {
    use uqm_rust::engine::{
        world::{CStarDesc, CATALOG_LEN},
        StoryKey,
    };
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
    game.world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 0);
    // SAFETY: zero is valid for catalog descriptors, and the fixture keeps the table live.
    let mut catalog: Box<[CStarDesc; CATALOG_LEN]> = Box::new(unsafe { mem::zeroed() });
    for star in catalog.iter_mut() {
        star.star_pt.y = i16::MAX;
    }
    catalog[0].star_pt.x = 1752;
    catalog[0].star_pt.y = 1450;
    // SAFETY: the serialized fixture owns the temporary catalog and game lifecycle.
    unsafe {
        let previous = uqm_rust::planets::starmap::star_array;
        uqm_rust::planets::starmap::star_array = catalog.as_mut_ptr().cast();
        bridge::with_game(&mut game, || {
            let point = POINT { x: 1752, y: 1450 };
            World::current().write(|world| {
                let selected = uqm_rust::planets::starmap::find_star_in(world, None, point, 0, 0);
                assert_eq!(selected.cast::<CStarDesc>(), catalog.as_mut_ptr());
                world.select_star(selected.cast::<CStarDesc>().as_ref(), Some(&catalog));
                let foreign = *selected;
                let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    uqm_rust::planets::starmap::find_star_in(world, Some(&foreign), point, 0, 0)
                }));
                assert!(rejected.is_err());
                assert_eq!(world.star_pointer(Some(&catalog)), selected);
            });
        });
        uqm_rust::planets::starmap::star_array = previous;
    }
    assert_eq!(
        game.world.star_pointer(Some(&catalog)),
        catalog.as_mut_ptr()
    );
}

#[test]
#[serial_test::serial]
fn space_queries_use_the_world_of_the_entered_context() {
    let mut game = Game::new();
    game.world.set_activity(IN_HYPERSPACE as u16);
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
    // SAFETY: the serialized fixture suspends publication while Cx is borrowed.
    unsafe {
        bridge::with_game(&mut game, || {
            bridge::enter(|cx| {
                assert_ne!(uqm_rust::navigation::globals::in_hqspace(cx.world), 0);
                assert_ne!(uqm_rust::navigation::globals::in_hyper_space(cx.world), 0);
                cx.world
                    .set_story_state(uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 2);
                assert_ne!(uqm_rust::navigation::globals::in_quasi_space(cx.world), 0);
                assert_eq!(uqm_rust::navigation::globals::in_hyper_space(cx.world), 0);
            });
        });
    }
}

#[test]
#[serial_test::serial]
fn status_offsets_use_the_already_borrowed_kernel() {
    let mut game = Game::new();
    // SAFETY: the serialized fixture owns the lifecycle; status initialization only writes offsets.
    unsafe {
        bridge::with_game(&mut game, || {
            uqm_rust::engine::Kernel::current().write(|kernel| init_status_offsets(kernel, 240));
        });
    }
}

#[test]
#[serial_test::serial]
fn starbase_releases_world_before_communication_and_cleanup() {
    use uqm_rust::engine::observe::{
        CampaignView, CheckpointView, Injected, InputView, Io, Observer, Presented,
    };
    struct Recording(Vec<String>);
    impl Observer<Option<CampaignView>> for Recording {
        fn input_admitted(&mut self, _: &InputView, _: &Option<CampaignView>) -> Injected {
            Injected::default()
        }
        fn checkpoint(&mut self, view: &CheckpointView<'_>) {
            self.0.push(format!("{view:?}"));
        }
        fn presented(&mut self, _: &mut Presented) {}
    }
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
    game.world.set_activity(CHECK_ABORT as u16);
    let mut observer = Recording(Vec::new());
    // SAFETY: this serialized fixture restores the developer flag and owns the lifecycle.
    unsafe {
        let previous = uqm_rust::front_end::uqmdebug::DISABLE_INTERACTIVITY;
        uqm_rust::front_end::uqmdebug::DISABLE_INTERACTIVITY = 1;
        bridge::with_game(&mut game, || {
            bridge::enter(|cx| {
                uqm_rust::ui::starbase::run_starbase_visit(
                    cx,
                    &mut Io {
                        observer: &mut observer,
                    },
                );
            })
        });
        uqm_rust::front_end::uqmdebug::DISABLE_INTERACTIVITY = previous;
    }
    assert_eq!(game.world.activity(), CHECK_ABORT as u16);
    assert_eq!(
        observer.0,
        [format!(
            "EncounterDispatch({})",
            uqm_rust::navigation::abi::CONVERSATION_COMMANDER_CONVERSATION
        )]
    );
}

#[test]
#[serial_test::serial]
fn encounter_teardown_releases_world_before_activity_queries() {
    let mut game = Game::new();
    game.world.set_activity(CHECK_ABORT as u16);
    // SAFETY: the serialized fixture owns empty resources and the published lifecycle.
    unsafe {
        bridge::with_game(&mut game, || {
            assert_eq!(uqm_rust::battle::encount_native::UninitEncounter(), 0)
        });
    }
}

#[test]
#[serial_test::serial]
fn campaign_star_observation_releases_world_between_field_reads() {
    use uqm_rust::automation::campaign_state::{CampaignView, LiveCampaign, StateKey, StateSource};
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
    let view = CampaignView::from_world(&game.world);
    // SAFETY: the serialized fixture owns the lifecycle and has no selected star.
    unsafe {
        bridge::with_game(&mut game, || {
            assert_eq!(
                LiveCampaign {
                    world: &view,
                    flagship_destroyed: false
                }
                .read(StateKey::StarCurrent),
                None
            );
        });
    }
}

#[test]
#[serial_test::serial]
fn lander_status_releases_world_before_gauge_and_lander_drawing() {
    use uqm_rust::planet_side::{
        adapters::CffiShipStatus, runtime::ShipStatusPort, session::ShipDelta,
    };
    let mut game = Game::new();
    game.world.data.flagship.module_slots.fill(255);
    game.world.data.flagship.module_slots[0] = CREW_POD as u8;
    game.world.data.flagship.crew_enlisted = 12;
    game.world.data.flagship.num_landers = 2;
    // SAFETY: absent graphics handles use the inactive backend on this fixture thread.
    unsafe {
        bridge::with_game(&mut game, || {
            CffiShipStatus
                .apply(&ShipDelta {
                    crew: -2,
                    landers: -1,
                    ..ShipDelta::default()
                })
                .unwrap();
        });
    }
    assert_eq!(game.world.data.flagship.crew_enlisted, 10);
    assert_eq!(game.world.data.flagship.num_landers, 1);
}

#[test]
#[serial_test::serial]
fn hyperspace_encounter_updates_story_in_the_borrowed_world() {
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
    game.world
        .set_story_state(uqm_rust::engine::StoryKey::USED_BROADCASTER, 1);
    // SAFETY: this serialized fixture has empty fleet and encounter queues.
    unsafe {
        let old_size = (ScreenWidth, ScreenHeight);
        (ScreenWidth, ScreenHeight) = (320, 240);
        bridge::with_game(&mut game, || {
            uqm_rust::engine::World::current()
                .write(|world| uqm_rust::navigation::abi::check_hyperspace_encounter(world));
        });
        (ScreenWidth, ScreenHeight) = old_size;
    }
    assert_eq!(
        game.world
            .story_state(uqm_rust::engine::StoryKey::USED_BROADCASTER),
        0
    );
}

#[test]
#[serial_test::serial]
fn hyperspace_movement_releases_world_before_fuel_queries() {
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
    game.world.data.flagship.log_x = 1000;
    game.world.data.flagship.log_y = 1000;
    game.world.data.flagship.fuel_on_board = 100;
    let (mut dx, mut dy) = (0, 0);
    // SAFETY: the fixture owns the lifecycle, output deltas and unclipped flagship position.
    unsafe {
        let old_size = (ScreenWidth, ScreenHeight);
        (ScreenWidth, ScreenHeight) = (320, 240);
        bridge::with_game(&mut game, || {
            uqm_rust::navigation::abi::MoveSIS(&mut dx, &mut dy);
        });
        (ScreenWidth, ScreenHeight) = old_size;
    }
    assert_eq!((dx, dy), (0, 0));
    assert_eq!(game.world.data.flagship.fuel_on_board, 100);
}

#[test]
#[serial_test::serial]
fn campaign_events_schedule_and_dispatch_using_the_existing_world() {
    use uqm_rust::engine::StoryKey;
    use uqm_rust::navigation::abi::add_event;
    use uqm_rust::navigation::abi::{
        dispatch_event, ARILOU_ENTRANCE_EVENT, ARILOU_EXIT_EVENT, ARILOU_UMGAH_CHECK,
    };
    unsafe extern "C" {
        fn GameClockTick();
    }
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
    let clock = &mut game.world.data.game.game_clock;
    clock.year_index = 2155;
    clock.month_index = 1;
    clock.day_index = 1;
    // SAFETY: this fixture owns the event pool and World throughout dispatch and cleanup.
    unsafe {
        game.world.clock.init();
        bridge::with_game(&mut game, || {
            World::current().write(|world| {
                dispatch_event(world, ARILOU_ENTRANCE_EVENT as u8);
                assert_eq!(world.story_state(StoryKey::ARILOU_SPACE), 1);
                let events = world.clock.events_ref();
                let next = events.get(events.head().unwrap());
                assert_eq!(
                    (next.year_index, next.month_index, next.day_index),
                    (2155, 1, 4)
                );
                assert_eq!(next.func_index, ARILOU_EXIT_EVENT as u8);
            });
            World::current().write(|world| add_event(world, 1, 0, 0, 0, ARILOU_UMGAH_CHECK as u8));
            assert_eq!(
                World::current().read(|world| world.story_state(StoryKey::ARILOU_CHECKED_UMGAH)),
                2
            );
        });
        bridge::with_game(&mut game, || {
            World::current().write(|world| {
                world.set_story_state(StoryKey::ARILOU_CHECKED_UMGAH, 0);
                world.data.game.game_clock.tick_count = 1;
                world.data.game.game_clock.day_in_ticks = 1;
            });
            World::current().write(|world| add_event(world, 1, 0, 1, 0, ARILOU_UMGAH_CHECK as u8));
            GameClockTick();
            World::current().read(|world| {
                assert_eq!(world.data.game.game_clock.day_index, 2);
                assert_eq!(world.story_state(StoryKey::ARILOU_CHECKED_UMGAH), 2);
            });
        });
        game.world.clock.uninit();
    }
}

#[test]
#[serial_test::serial]
fn display_list_initializes_with_the_entered_context_and_explicit_scale() {
    let mut game = Game::new();
    // SAFETY: this serialized fixture owns the game and its idle battle state.
    unsafe {
        bridge::with_game(&mut game, || {
            for step in [false, true] {
                bridge::enter(|cx| {
                    uqm_rust::battle::process_native::init_display_list(cx, step);
                    assert!(cx.elements().head().is_none());
                });
                let zoom = bridge::battle(|battle| (battle.zoom_out, battle.max_zoom_out));
                let expected = if step {
                    ((MAX_VIS_REDUCTION + 1) as i16, MAX_VIS_REDUCTION as i16)
                } else {
                    (
                        (MAX_ZOOM_OUT + (1 << ZOOM_SHIFT)) as i16,
                        MAX_ZOOM_OUT as i16,
                    )
                };
                assert_eq!(zoom, expected);
            }
        });
    }
}

#[test]
#[serial_test::serial]
fn fleet_infinity_reads_the_supplied_world_without_publication() {
    let mut world = World::new();
    assert_eq!(fleet_is_infinite(&world, 0), 0);
    assert_eq!(fleet_is_infinite(&world, 1), 0);
    let id = world.npc_built_ships().alloc().unwrap();
    world.npc_built_ships().get_mut(id).crew_level = 7;
    world.npc_built_ships().put(id);
    assert_eq!(fleet_is_infinite(&world, 1), 0);
    world.npc_built_ships().get_mut(id).crew_level = u16::MAX;
    assert_eq!(fleet_is_infinite(&world, 1), 1);
    assert_eq!(fleet_is_infinite(&world, 0), 0);
}
