//! Production ship pools clear reused records and retain link order and capacity.
use uqm_rust::engine::battle::link_queue::{LinkQueue, RacePool, Starship};
use uqm_rust::engine::World;

#[test]
fn ship_construction_clears_payload_and_preserves_allocation_order() {
    let mut pool = RacePool::with_capacity(0, 2);
    let mut ships = LinkQueue::<Starship>::new(&mut pool);
    let first = ships.build(3).unwrap();
    let second = ships.build(7).unwrap();
    assert_eq!(ships.head(), Some(first));
    assert_eq!(ships.tail(), Some(second));
    assert_eq!(ships.succ(first), Some(second));
    assert_eq!(ships.pred(second), Some(first));
    assert_eq!(ships.get(first).species, 3);
    assert_eq!(ships.get(second).species, 7);
    assert!(ships.build(9).is_none());
    assert_eq!(ships.count(), 2);
    ships.edit(first, |ship| {
        ship.captain = 19;
        ship.crew = 200;
        ship.max_crew = 201;
        ship.cost = 12;
        ship.index = 18;
        ship.energy_counter = 13;
        ship.player = 1;
        ship.control = 7;
    });
    ships.remove(first);
    ships.free(first);
    let reused = ships.build(9).unwrap();
    assert_ne!(reused, first);
    assert_eq!(ships.head(), Some(second));
    assert_eq!(ships.tail(), Some(reused));
    let fresh = ships.get(reused);
    assert_eq!(fresh.species, 9);
    assert_eq!(fresh.captain, 0);
    assert_eq!(fresh.crew, 0);
    assert_eq!(fresh.max_crew, 0);
    assert_eq!(fresh.cost, 0);
    assert_eq!(fresh.index, 0);
    assert_eq!(fresh.energy_counter, 0);
    assert_eq!(fresh.player, 0);
    assert_eq!(fresh.control, 0);

    let mut world = World::new();
    let mut fragments = world.built_ships();
    let first = fragments.alloc().unwrap();
    fragments.get_mut(first).crew_level = 200;
    fragments.get_mut(first).species_id = 7;
    fragments.get_mut(first).index = 5;
    fragments.put(first);
    fragments.remove(first);
    fragments.free(first);
    let reused = fragments.alloc().unwrap();
    assert_ne!(reused, first);
    let fresh = fragments.get(reused);
    assert_eq!(fresh.crew_level, 0);
    assert_eq!(fresh.species_id, 0);
    assert_eq!(fresh.index, 0);
    assert!(fresh.race_strings.is_null());
    assert!(fresh.icons.is_null());
    assert!(fresh.melee_icon.is_null());
    fragments.put(reused);
    assert_eq!(fragments.count(), 1);
}
