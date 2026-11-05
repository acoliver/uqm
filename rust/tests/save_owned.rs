use uqm_rust::engine::{save, world::queues::FleetInfo, World};

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
    let mut summary = saved.summary;
    summary.sis.element_amounts = [1, 2, 3, 4, 5, 6, 7, 8];
    let mut world = World::new();
    world.story = Some(uqm_rust::engine::world::story::GameState::new());
    world.data.game.ip_location.x = 21;
    world.data.game.ip_location.y = 31;
    world.data.game.in_orbit = 2;
    let view = save::SessionView {
        summary: summary.clone(),
        star: saved.star,
        ip_location: uqm_rust::graphics::native_abi::POINT { x: 91, y: 92 },
    };
    let captured = save::snapshot(&mut world, view).unwrap();
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
