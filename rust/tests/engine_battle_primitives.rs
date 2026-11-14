use uqm_engine::battle::{Battle, Cx, ElementId, RaceDesc, ShipId};
use uqm_engine::graphics::native_abi::POINT;
use uqm_engine::{Game, World};

fn owner(cx: &mut Cx<'_>, side: usize, position: POINT) -> (ShipId, ElementId) {
    let ship = {
        let mut ships = cx.kernel.ships(side);
        let link = ships.alloc().unwrap();
        let ship = ships.ship_id(link);
        ships.put(link);
        ship
    };
    let mut descriptor = uqm_engine::battle::record::RaceDescriptor::EMPTY;
    descriptor.ship_info.crew_level = 10;
    descriptor.ship_info.max_crew = 10;
    cx.attach_descriptor(ship, RaceDesc::fixture(Box::new(descriptor)));
    let element = cx.alloc_element().unwrap();
    cx.element_mut(element).flags = 4;
    cx.element_mut(element).life = 1;
    cx.element_mut(element).player = side as i16;
    cx.element_mut(element).current.location = position;
    cx.element_mut(element).next.location = position;
    cx.primitive_style_mut(element).kind = 1;
    cx.set_parent(element, Some(ship));
    cx.set_ship_element(ship, Some(element));
    cx.put(element);
    (ship, element)
}

fn with_context(rng: fn() -> u32, call: impl FnOnce(&mut Cx<'_>)) {
    let mut game = Game::new();
    game.kernel.reset_element_capacity(8);
    for side in 0..2 {
        game.kernel.reset_ship_capacity(side, 4);
    }
    let mut battle = Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, rng);
    call(&mut cx);
}

#[test]
fn interception_uses_the_supplied_projection_without_moving_live_records() {
    with_context(
        || panic!("interception must not draw RNG"),
        |cx| {
            let (_, source) = owner(cx, 0, POINT { x: 1000, y: 1000 });
            let (_, target) = owner(cx, 1, POINT { x: 3000, y: 1000 });
            let live = cx.weapon_origin(source);
            let other = cx.weapon_origin(target);
            let mut projected = live.clone();
            projected.current.location.x = 2900;
            projected.next.location = projected.current.location;
            assert_eq!(cx.plot_intercept(&live, &other, 1, 100), 0);
            assert_eq!(cx.plot_intercept(&projected, &other, 1, 100), 1);
            assert_eq!(cx.element(source).current.location.x, 1000);
        },
    );
}

#[test]
fn homing_keeps_first_equal_distance_target_without_drawing_rng() {
    with_context(
        || panic!("aligned homing must not draw RNG"),
        |cx| {
            let (_, tracker) = owner(cx, 0, POINT { x: 1000, y: 1000 });
            let (_, first) = owner(cx, 1, POINT { x: 1100, y: 1000 });
            let (_, second) = owner(cx, 1, POINT { x: 1000, y: 1100 });
            let mut facing = 4;
            assert_eq!(cx.track_ship(tracker, &mut facing), 0);
            assert_eq!(cx.target(tracker), Some(first));
            assert_eq!(facing, 4);
            assert_eq!(cx.elements().succ(first), Some(second));
        },
    );
}

#[test]
fn opposite_heading_homing_draws_once_and_retains_explicit_target() {
    with_context(
        || 1,
        |cx| {
            let (_, tracker) = owner(cx, 0, POINT { x: 1000, y: 1000 });
            let (_, target) = owner(cx, 1, POINT { x: 1100, y: 1000 });
            let (_, closer) = owner(cx, 1, POINT { x: 1001, y: 1000 });
            cx.set_target(tracker, Some(target));
            let mut facing = 12;
            assert_eq!(cx.track_ship(tracker, &mut facing), 8);
            assert_eq!(facing, 13);
            assert_eq!(cx.target(tracker), Some(target));
            assert_ne!(cx.target(tracker), Some(closer));
        },
    );
}

#[test]
fn gravity_uses_next_positions_and_preserves_order_for_unpublished_owners() {
    with_context(
        || panic!("gravity must not draw RNG"),
        |cx| {
            let (_, ship) = owner(cx, 0, POINT { x: 2000, y: 2000 });
            let planet = cx.alloc_element().unwrap();
            cx.element_mut(planet).mass = 200;
            cx.element_mut(planet).current.location = POINT { x: 4000, y: 4000 };
            cx.element_mut(planet).next.location = POINT { x: 400, y: 400 };
            cx.put(planet);
            cx.element_mut(ship).next.location = POINT { x: 400, y: 400 };
            let head = cx.elements().head();
            assert!(!cx.gravity(ship));
            cx.element_mut(ship).flags |= 2048;
            assert!(cx.gravity(ship));
            assert_eq!(cx.elements().head(), head);
            assert_eq!(cx.elements().succ(ship), Some(planet));
        },
    );
}

#[test]
fn record_operations_do_not_require_a_published_world() {
    let mut first = World::new();
    let second = World::new();
    first.set_flagship_crew(7);
    assert_eq!(first.data.flagship.crew_enlisted, 7);
    assert_ne!(
        first.data.flagship.crew_enlisted,
        second.data.flagship.crew_enlisted
    );
}
