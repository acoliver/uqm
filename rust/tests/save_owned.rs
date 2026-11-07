use uqm_rust::engine::{save, world::queues::FleetInfo, World};

#[test]
fn snapshot_derives_summary_from_owned_story_clock_and_escort_order() {
    let saved = uqm_save::decode_save(include_bytes!("fixtures/savegame/legacy-save.00")).unwrap();
    let mut world = World::new();
    world.story = Some(uqm_rust::engine::world::story::StoryState::new());
    world.set_activity(3);
    world.set_story_state(uqm_rust::engine::StoryKey::ARILOU_SPACE_SIDE, 2);
    world.data.game.game_clock.month_index = 11;
    world.data.game.game_clock.day_index = 5;
    world.data.game.game_clock.year_index = 2155;
    for _ in 0..24 {
        let mut fleets = world.avail_races();
        let id = fleets.alloc().unwrap();
        *fleets.get_mut(id) = FleetInfo::default();
        fleets.put(id);
    }
    for race in [4, 3] {
        let mut ships = world.built_ships();
        let id = ships.alloc().unwrap();
        ships.get_mut(id).race_id = race;
        ships.put(id);
    }
    let captured = save::snapshot(
        &mut world,
        save::SessionView {
            solar: save::summary::SessionSummary::None,
            star: saved.star,
            ip_location: uqm_rust::graphics::native_abi::POINT { x: 0, y: 0 },
        },
        &[b'x'; 80],
    )
    .unwrap();
    assert_eq!(captured.summary.activity, save::summary::IN_QUASISPACE);
    assert_eq!(captured.summary.month_index, 11);
    assert_eq!(captured.summary.day_index, 5);
    assert_eq!(captured.summary.year_index, 2155);
    assert_eq!(captured.summary.num_ships, 2);
    assert_eq!(&captured.summary.ship_list[..2], &[4, 3]);
    assert_eq!(captured.summary.save_name.as_bytes(), &[b'x'; 63]);
    assert_eq!(captured.global.current_activity, 3);
}

#[test]
fn apply_returns_saved_activity_without_changing_live_activity_or_acquiring_an_owner() {
    let saved = uqm_save::decode_save(include_bytes!("fixtures/savegame/legacy-save.00")).unwrap();
    let mut world = World::new();
    world.set_activity(0x1000);
    for _ in 0..24 {
        let mut fleets = world.avail_races();
        let id = fleets.alloc().unwrap();
        *fleets.get_mut(id) = FleetInfo::default();
        fleets.put(id);
    }
    let activity = save::apply(saved.clone(), &mut world).unwrap();
    assert_eq!(activity, saved.global.current_activity);
    assert_eq!(world.activity(), 0x1000);
    assert_eq!(
        world.data.flagship.crew_enlisted,
        saved.summary.sis.crew_enlisted
    );
    assert_eq!(world.data.flagship.ship_name, saved.summary.sis.ship_name);
    assert_eq!(world.built_ships_ref().count(), saved.ship_queue.len());
    assert_eq!(world.clock.events_ref().count(), saved.events.len());
}

#[test]
fn legacy_apply_installs_original_image_order_without_publishing_the_world() {
    let saved = uqm_save::decode_save(include_bytes!("fixtures/savegame/legacy-save.00")).unwrap();
    let mut world = World::new();
    world.set_activity(0x1000);
    for _ in 0..24 {
        let mut fleets = world.avail_races();
        let id = fleets.alloc().unwrap();
        *fleets.get_mut(id) = FleetInfo::default();
        fleets.put(id);
    }
    let node = uqm_save::CannedNode {
        star: 1,
        planet: 0,
        moon: 0,
        node: 2,
        value: 3,
        x: 4,
        y: 5,
    };
    world.files.can_node(node);
    assert_eq!(world.files.canned_node(node.key()), Some(node));
    let images = [vec![1, 2, 3], vec![4, 5], vec![6, 7, 8, 9]];
    let expected = images.clone();
    let activity = save::apply_legacy(saved.clone(), &mut world, images, 543).unwrap();
    assert_eq!(activity, saved.global.current_activity);
    assert_eq!(world.activity(), 0x1000);
    assert_eq!(world.data.game.battle_group_ref, 543);
    for (index, image) in [0, 2, 1].into_iter().zip(expected) {
        assert_eq!(world.files.get_file(index).unwrap().contents(), image);
    }
    assert_eq!(world.files.canned_node(node.key()), None);
}

#[test]
fn snapshot_preserves_saved_location_and_normalizes_only_live_location() {
    let saved = uqm_save::decode_save(include_bytes!("fixtures/savegame/legacy-save.00")).unwrap();
    let mut sis = saved.summary.sis;
    sis.element_amounts = [1, 2, 3, 4, 5, 6, 7, 8];
    let summary = uqm_save::Summary {
        sis,
        activity: 0,
        flags: 0,
        month_index: 0,
        day_index: 0,
        year_index: 0,
        mcredit_lo: 0,
        mcredit_hi: 0,
        num_ships: 0,
        num_devices: 0,
        ship_list: [0; uqm_save::MAX_BUILT_SHIPS],
        device_list: [0; uqm_save::MAX_EXCLUSIVE_DEVICES],
        save_name: saved.summary.save_name,
    };
    let mut world = World::new();
    world.story = Some(uqm_rust::engine::world::story::StoryState::new());
    world.data.flagship = (&summary.sis).into();
    world.data.game.ip_location.x = 21;
    world.data.game.ip_location.y = 31;
    world.data.game.in_orbit = 2;
    let view = save::SessionView {
        solar: save::summary::SessionSummary::None,
        star: saved.star,
        ip_location: uqm_rust::graphics::native_abi::POINT { x: 91, y: 92 },
    };
    let captured = save::snapshot(&mut world, view, summary.save_name.as_bytes()).unwrap();
    assert_eq!(
        captured.global.ip_location,
        uqm_save::Point { x: 21, y: 31 }
    );
    assert_eq!(captured.global.in_orbit, 2);
    assert_eq!(
        (world.data.game.ip_location.x, world.data.game.ip_location.y),
        (91, 92)
    );
    assert_eq!(world.data.game.in_orbit, 0);
    assert_eq!(captured.summary, summary);
}

#[test]
fn solar_scan_save_uses_unpublished_story_and_files_and_clears_the_flag_after_writing() {
    use uqm_rust::engine::{
        save::location::{apply_solar_location, OrbitLocation, SolarLocation},
        world::planet_info::ScanRetrieveMask,
        StoryKey,
    };
    let mut world = World::new();
    world.story = Some(uqm_rust::engine::world::story::StoryState::new());
    world.data.game.ip_planet = 2;
    world.data.game.ship_facing = 9;
    world.set_story_state(StoryKey::PLANETARY_CHANGE, 1);
    world.files.replace_contents(0, &[0; 4]).unwrap();
    let mut moon_counts = [0; 16];
    moon_counts[..3].copy_from_slice(&[1, 2, 0]);
    apply_solar_location(
        &mut world,
        SolarLocation::Orbit(OrbitLocation {
            in_orbit: 3,
            star_index: 0,
            planet: 1,
            moon: 2,
            scan: ScanRetrieveMask::from_array(&[1, 2, 3]),
            planet_count: 3,
            moon_counts,
        }),
    )
    .unwrap();
    assert_eq!(world.data.game.in_orbit, 3);
    assert_eq!(world.data.game.ship_facing, 9);
    assert_eq!(world.story_state(StoryKey::PLANETARY_CHANGE), 0);
    let image = world.files.get_file(0).unwrap().contents();
    assert_eq!(image.len(), 4 + 6 * 12);
    assert_eq!(&image[..4], &4u32.to_le_bytes());
    assert_eq!(&image[52..64], &[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
}

#[test]
fn hyper_save_uses_unpublished_owners_and_leaves_absent_flagship_state_unchanged() {
    let mut game = uqm_rust::engine::Game::new();
    game.world.data.game.ship_facing = 42;
    game.world.data.game.ip_location.x = 91;
    game.world.data.game.in_orbit = 3;
    assert_eq!(
        save::hyper_save::prepare_hyper_state(&mut game.world, &game.kernel),
        Ok(save::hyper_save::HyperFlagship::Absent)
    );
    assert_eq!(game.world.data.game.ship_facing, 42);
    assert_eq!(game.world.data.game.ip_location.x, 91);
    assert_eq!(game.world.data.game.in_orbit, 3);
}
