use super::*;

#[test]
fn moving_a_system_keeps_its_generation_data_and_rng_independent() {
    let mut first = SolarSys::new(&[], 0x40, 7, RandomContext::with_seed(11));
    let mut second = SolarSys::new(&[], 0x20, 9, RandomContext::with_seed(11));
    first.sun.data_index = 2;
    first.layout(|rng, table, size, kind, planets| {
        assert!(table.is_empty());
        assert_eq!((size, kind), (2, 2));
        planets[0].rand_seed = rng.next_random();
        planets[0].radius = 180;
        1
    });
    first.info.planet_info.scan_retrieve_mask = [u32::MAX, 0x8000_0000, 1];
    let mut moved = Box::new(first);
    assert_eq!(moved.sun.num_planets, 1);
    assert_eq!(moved.star_index(), 7);
    assert_eq!(second.star_index(), 9);
    assert_eq!(second.sun.num_planets, 0);
    assert_eq!(second.info.planet_info.scan_retrieve_mask, [0; 3]);
    assert_eq!(moved.planets[0].rand_seed, second.rng.next_random());
    assert_eq!(moved.rng.next_random(), second.rng.next_random());
    assert_eq!(
        moved.info.planet_info.scan_retrieve_mask,
        [u32::MAX, 0x8000_0000, 1]
    );
}

#[test]
fn moon_and_orbital_callbacks_borrow_disjoint_owned_records() {
    let mut system = SolarSys::new(&[], 0x40, 7, RandomContext::with_seed(21));
    system.sun.data_index = 2;
    system.planets[1].radius = 250;
    system.planets[1].location = POINT { x: 34, y: -12 };
    system.layout_moons(1, |rng, size, kind, parent, moons| {
        assert_eq!((size, kind, parent.radius), (2, 2, 250));
        moons[0].data_index = 12;
        moons[0].rand_seed = rng.next_random();
        moons[0].location = POINT { x: 7, y: 8 };
    });
    let moon = Body::Moon { planet: 1, moon: 0 };
    system.orbital(moon, |data| {
        assert_eq!(data.parent_radius, Some(250));
        assert_eq!(data.world.data_index, 12);
        assert_eq!(data.world.location, POINT { x: 7, y: 8 });
        assert_eq!((data.star_size, data.star_type), (2, 0x40));
        data.info.planet_info.gravity = 123;
    });
    assert_eq!(system.body_data_index(moon), 12);
    assert_eq!(system.orbital_location(moon), POINT { x: 34, y: -12 });
    assert_eq!(system.planet_info().gravity, 123);
}

#[test]
fn body_persistence_and_saved_orbit_use_their_distinct_encodings() {
    let planet = Body::Planet(5);
    assert_eq!(planet.persistence_moon(), 0);
    assert_eq!(planet.saved_orbit(), 1);
    for moon in 0..4 {
        let body = Body::Moon { planet: 5, moon };
        assert_eq!(body.planet(), 5);
        assert_eq!(body.persistence_moon(), moon + 1);
        assert_eq!(body.saved_orbit(), moon + 2);
    }
}
