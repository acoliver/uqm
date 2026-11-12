use super::*;
use crate::engine::{clock::CEvent, world::story::StoryState, StoryKey, World};

#[test]
fn devices_change_only_the_supplied_unpublished_campaign() {
    let mut first = World::new();
    first.story = Some(StoryState::new());
    let mut second = World::new();
    second.story = Some(StoryState::new());
    give_devices(&mut first);
    for key in [StoryKey::UTWIG_BOMB_ON_SHIP, StoryKey::PORTAL_KEY_ON_SHIP] {
        assert_eq!(first.story_state(key), 1);
        assert_eq!(second.story_state(key), 0);
    }
}

#[test]
fn event_output_preserves_legacy_bytes_and_reports_writer_failure() {
    let event = CEvent {
        year_index: 2155,
        month_index: 2,
        day_index: 7,
        func_index: HYPERSPACE_ENCOUNTER_EVENT as u8,
    };
    let mut bytes = Vec::new();
    dump_event(&mut bytes, &event).unwrap();
    assert_eq!(bytes, b"2155/02/07: HYPERSPACE_ENCOUNTER_EVENT\n");
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        dump_event(&mut Broken, &event).unwrap_err().kind(),
        std::io::ErrorKind::BrokenPipe
    );
}

#[test]
fn numeric_labels_are_independent_owned_values() {
    let first = tectonics_string(17);
    let second = tectonics_string(18);
    assert_eq!(first, "17");
    assert_eq!(second, "18");
    assert_eq!(tectonics_string(0), "none");
}

#[test]
fn creature_values_keep_the_low_nibble_and_invalid_sentinel() {
    use crate::planet_side::creatures::{CreatureCatalog, CreatureKind};
    for value in 0..=u8::MAX {
        let expected = CreatureKind::new(value)
            .map(|kind| i32::from(CreatureCatalog::raw_lifedata(kind).1 & 15))
            .unwrap_or(-1);
        assert_eq!(uqm_debug_creature_bio_value(value), expected);
    }
}

#[test]
#[serial_test::serial]
fn diagnostic_text_preserves_raw_bytes_truncation_and_category_output() {
    let mut kernel = Kernel::new();
    let mut strings = vec![b"name\0".to_vec(); GAMESTR_COUNT as usize];
    strings[1] = Vec::new();
    strings[SCAN_STRING_BASE as usize + 4] = [vec![b'x'; 38], vec![0xff, 0]].concat();
    strings[SCAN_STRING_BASE as usize + 54] = b"world\0".to_vec();
    strings[SCAN_STRING_BASE as usize + 55] = b"gas\0".to_vec();
    kernel.install_game_text(crate::strings::owned_table::LoadedTextTable::fixture(
        strings,
    ));
    let first = planet_type_string(&kernel, 0);
    assert_eq!(first, [vec![b'x'; 38], vec![0xff]].concat());
    assert_eq!(planet_type_string(&kernel, FIRST_GAS_GIANT as i32), b"gas");
    assert_eq!(first[38], 0xff);
    let mut out = Vec::new();
    let mut errors = Vec::new();
    dump_strings(&kernel, &mut out, &mut errors).unwrap();
    assert!(errors.is_empty());
    assert!(out.starts_with(b"[ STAR_STRING_BASE + 0 ]  name\n"));
    assert!(out
        .windows(b"[ STAR_STRING_BASE + 1 ]  \n[ STAR_STRING_BASE + 2 ]  name\n".len())
        .any(|bytes| bytes == b"[ STAR_STRING_BASE + 1 ]  \n[ STAR_STRING_BASE + 2 ]  name\n"));
    assert!(out.windows(39).any(|bytes| bytes == first));
    let mut types = Vec::new();
    dump_planet_type(
        &kernel,
        &mut types,
        FIRST_GAS_GIANT as i32,
        &crate::planets::tables::PLANET_ARRAY.0[0],
    )
    .unwrap();
    assert!(types.starts_with(b"gas\n\tType: "));
    assert!(types
        .windows(b"Surface generation algoritm:".len())
        .any(|bytes| bytes == b"Surface generation algoritm:"));
    assert!(types.ends_with(b"\n\n"));
}

#[test]
fn empty_battle_resets_do_not_acquire_a_context_or_apply_an_effect() {
    let mut game = crate::engine::Game::new();
    game.world.story = Some(StoryState::new());
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        crate::math::random,
    );
    for activity in [0, IN_BATTLE as u16, IN_HYPERSPACE as u16 | IN_BATTLE as u16] {
        cx.world.set_activity(activity);
        reset_crew_battle(&mut cx, |_, _, _| panic!("no battle ship"));
        reset_energy_battle(&mut cx, |_, _, _| panic!("no battle ship"));
    }
}

#[test]
fn event_dump_walks_the_supplied_fifo_without_mutating_it() {
    let mut first = World::new();
    first.clock.init();
    let second = World::new();
    for selector in [ARILOU_ENTRANCE_EVENT, ARILOU_EXIT_EVENT] {
        crate::time::clock_abi::insert_event(&mut first.clock, (2155, 2, 7), selector as u8)
            .unwrap();
    }
    let head = first.clock.events_ref().head();
    let mut bytes = Vec::new();
    dump_events(&first, &mut bytes).unwrap();
    assert_eq!(
        bytes,
        b"2155/02/07: ARILOU_ENTRANCE_EVENT\n2155/02/07: ARILOU_EXIT_EVENT\n"
    );
    assert_eq!(first.clock.events_ref().head(), head);
    let mut bytes = Vec::new();
    dump_events(&second, &mut bytes).unwrap();
    assert!(bytes.is_empty());
}

#[test]
#[serial_test::serial]
fn battle_refills_borrow_the_player_descriptor_and_restore_status_selection() {
    use crate::engine::battle::{Race, RaceTemplate};
    use crate::graphics::context_abi::{self, LoadedContext};
    fn descriptor(_: &mut World) -> crate::engine::battle::RaceDesc {
        RaceTemplate::default().descriptor(&RACE)
    }
    static RACE: Race = Race {
        descriptor,
        preprocess: None,
        postprocess: None,
        init_weapon: None,
        intelligence: None,
        uninit: None,
    };
    let mut game = crate::engine::Game::new();
    game.world.story = Some(StoryState::new());
    game.world.set_activity(IN_BATTLE as u16);
    game.kernel.reset_ship_capacity(RPG_PLAYER_NUM as usize, 1);
    // SAFETY: this serialized fixture owns and releases the context after all leases.
    let status = unsafe { context_abi::CreateContextAux(c"refill status".as_ptr()) };
    game.kernel.status_context = status;
    let previous = LoadedContext::create(c"refill previous");
    let previous_context = previous.lease();
    previous_context.select();
    let ship = {
        let mut ships = game.kernel.ships(RPG_PLAYER_NUM as usize);
        let id = ships.build(1).unwrap();
        ships.ship_id(id)
    };
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0);
    let element = cx.alloc_element().unwrap();
    {
        let mut state = cx.element_mut(element);
        state.flags = PLAYER_SHIP as u16;
        state.player = RPG_PLAYER_NUM as i16;
    }
    cx.set_parent(element, Some(ship));
    cx.set_ship_element(ship, Some(element));
    cx.put(element);
    assert_eq!(find_player_ship(&cx, RPG_PLAYER_NUM as i16), Some(ship));
    reset_crew_battle(&mut cx, |_, _, _| panic!("missing descriptor"));
    reset_energy_battle(&mut cx, |_, _, _| panic!("missing descriptor"));
    let stats = descriptor(cx.world);
    cx.attach_descriptor(ship, stats);
    for (crew, energy) in [(0, 0), (19, 8), (20, 10), (22, 12)] {
        {
            let mut stats = cx.race_stats_mut(ship);
            stats.ship_info.crew_level = crew;
            stats.ship_info.max_crew = 20;
            stats.ship_info.energy_level = energy;
            stats.ship_info.max_energy = 10;
        }
        reset_crew_battle(&mut cx, |supplied, id, delta| {
            assert_eq!(id, element);
            assert_eq!(supplied.owner(id), ship);
            assert_eq!(delta, 20 - crew as i16);
            assert_eq!(
                previous_context
                    .selected_context()
                    .unwrap()
                    .diagnostic_name()
                    .as_bytes(),
                b"refill status"
            );
            true
        });
        reset_energy_battle(&mut cx, |_, id, delta| {
            assert_eq!(id, element);
            assert_eq!(delta, 10 - i16::from(energy));
            true
        });
        assert_eq!(
            previous_context
                .selected_context()
                .unwrap()
                .diagnostic_name()
                .as_bytes(),
            b"refill previous"
        );
    }
    // SAFETY: all refill calls and their temporary leases have ended.
    unsafe { context_abi::DestroyContext(status) };
}

#[test]
#[serial_test::serial]
fn debug_key_uses_the_supplied_rng_and_real_status_resources() {
    if !crate::native_test_process::own_graphics_process(
        "front_end::uqmdebug::supplied_tests::debug_key_uses_the_supplied_rng_and_real_status_resources",
    ) {
        return;
    }
    use crate::engine::battle::{frames::FrameSelection, link_queue::ShipPickerResources};
    use crate::graphics::{context_abi::ContextFrame, drawable_abi::LoadedDrawable};
    let mut game = crate::engine::Game::new();
    game.world.story = Some(StoryState::new());
    game.world.set_activity(IN_INTERPLANETARY as u16);
    let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
    game.kernel.screen_context = game.kernel.status_context;
    game.kernel.space_context = game.kernel.status_context;
    game.kernel
        .install_game_text(crate::strings::owned_table::LoadedTextTable::fixture(
            vec![b"A\0".to_vec(); GAMESTR_COUNT as usize],
        ));
    let art = LoadedDrawable::create(8, 64, 140, 100).unwrap();
    let frame = art.lease().frame(0).unwrap();
    game.kernel.flag_stat_frame = frame;
    game.kernel.font_grad_frame = frame;
    let context = ContextLease::require(game.kernel.status_context);
    context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
        game.kernel.screen,
    )));
    for species in 0..=BLACK_URQUAN_SHIP {
        let mut fleets = game.world.avail_races();
        let id = fleets.alloc().unwrap();
        fleets.get_mut(id).species_id = species;
        fleets.put(id);
        game.world.retain_fleet_resources(
            species,
            ShipPickerResources::new(None, FrameSelection::retain(frame)),
        );
    }
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 11);
    let global_rng_before = crate::math::random_state();
    debug_key_pressed_synchronous(
        &mut cx,
        |_, _, _| panic!("no battle crew effect"),
        |_, _, _| panic!("no battle energy effect"),
    );
    assert_eq!(crate::math::random_state(), global_rng_before);
    let ships = cx.world.built_ships_ref();
    assert_eq!(ships.count(), 12);
    let mut next = ships.head();
    while let Some(id) = next {
        assert_eq!(ships.get(id).captains_name_index, 16);
        next = ships.succ(id);
    }
    assert_eq!(cx.world.data.flagship.crew_enlisted, 250);
    assert_eq!(cx.world.data.flagship.fuel_on_board, 21_000);
    assert!(cx.kernel.instant_move);
    display.finish(game);
}

#[test]
#[serial_test::serial]
fn event_skip_uses_the_supplied_calendar_and_dispatches_equal_date_events() {
    if !crate::native_test_process::own_graphics_process(
        "front_end::uqmdebug::supplied_tests::event_skip_uses_the_supplied_calendar_and_dispatches_equal_date_events",
    ) {
        return;
    }
    use crate::graphics::context_abi::ContextFrame;
    let mut game = crate::engine::Game::new();
    game.world.story = Some(StoryState::new());
    game.world.set_activity(IN_INTERPLANETARY as u16);
    game.world.clock.init();
    let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
    let context = ContextLease::require(game.kernel.status_context);
    context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
        game.kernel.screen,
    )));
    let clock = &mut game.world.data.game.game_clock;
    clock.year_index = 2155;
    clock.month_index = 2;
    clock.day_index = 1;
    clock.day_in_ticks = 1;
    clock.tick_count = 1;
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
        panic!("interplanetary daily events must not draw encounter RNG")
    });
    forward_to_next_event(&mut cx, true);
    assert_eq!(cx.world.data.calendar_date(), (2, 1, 2155));
    for (day, selector) in [(2, HYPERSPACE_ENCOUNTER_EVENT), (3, ARILOU_UMGAH_CHECK)] {
        crate::time::clock_abi::insert_event(&mut cx.world.clock, (2155, 2, day), selector as u8)
            .unwrap();
    }
    forward_to_next_event(&mut cx, true);
    assert_eq!(cx.world.data.calendar_date(), (2, 3, 2155));
    assert_eq!(cx.world.story_state(StoryKey::ARILOU_CHECKED_UMGAH), 2);
    forward_to_next_event(&mut cx, false);
    assert_eq!(cx.world.data.calendar_date(), (2, 4, 2155));
    let events = cx.world.clock.events_ref();
    assert_eq!(events.count(), 1);
    let head = events.head().unwrap();
    assert_eq!(events.get(head).day_index, 5);
    assert_eq!(
        events.get(head).func_index,
        HYPERSPACE_ENCOUNTER_EVENT as u8
    );
    cx.world.data.game.game_clock.day_in_ticks = 0;
    forward_to_next_event(&mut cx, true);
    assert_eq!(cx.world.data.calendar_date(), (2, 4, 2155));
    assert_eq!(cx.world.clock.events_ref().head(), Some(head));
    display.finish(game);
}
