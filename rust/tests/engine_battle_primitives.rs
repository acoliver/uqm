use uqm_engine::battle::{Battle, Cx, ElementId, RaceDesc, ShipId};
use uqm_engine::graphics::native_abi::POINT;
use uqm_engine::Game;

fn owner(cx: &mut Cx<'_>, side: usize, position: POINT) -> (ShipId, ElementId) {
    let ship = {
        let mut ships = cx.kernel.ships(side);
        let link = ships
            .alloc()
            .unwrap_or_else(|| panic!("fixture allocation must succeed"));
        let ship = ships.ship_id(link);
        ships.put(link);
        ship
    };
    let mut descriptor = uqm_engine::battle::record::RaceDescriptor::EMPTY;
    descriptor.ship_info.crew_level = 10;
    descriptor.ship_info.max_crew = 10;
    cx.attach_descriptor(ship, RaceDesc::fixture(Box::new(descriptor)));
    let element = cx
        .alloc_element()
        .unwrap_or_else(|| panic!("fixture allocation must succeed"));
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
#[serial_test::serial]
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
#[serial_test::serial]
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
#[serial_test::serial]
fn opposite_heading_homing_retains_explicit_target() {
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
#[serial_test::serial]
fn gravity_uses_next_positions_and_preserves_order_for_unpublished_owners() {
    with_context(
        || panic!("gravity must not draw RNG"),
        |cx| {
            let (_, ship) = owner(cx, 0, POINT { x: 2000, y: 2000 });
            let planet = cx
                .alloc_element()
                .unwrap_or_else(|| panic!("fixture allocation must succeed"));
            cx.element_mut(planet).mass = 200;
            cx.element_mut(planet).current.location = POINT { x: 4000, y: 4000 };
            cx.element_mut(planet).next.location = POINT { x: 400, y: 400 };
            cx.put(planet);
            cx.element_mut(ship).next.location = POINT { x: 400, y: 400 };
            let head = cx.elements().head();
            assert!(!uqm_engine::battle::gravity::calculate_gravity(
                cx,
                ship,
                (320, 240)
            ));
            cx.element_mut(ship).flags |= 2048;
            assert!(uqm_engine::battle::gravity::calculate_gravity(
                cx,
                ship,
                (320, 240)
            ));
            assert_eq!(cx.elements().head(), head);
            assert_eq!(cx.elements().succ(ship), Some(planet));
        },
    );
}

fn assert_dispatched(cx: &Cx<'_>, ship: ShipId, callback: usize) {
    use uqm_engine::collections::observation::{self, Event};
    observation::inspect_pending(|events| {
        assert!(events.iter().any(|event| matches!(
            event, Event::Hook { parent, callback: observed }
            if *parent == cx.ship_pointer(ship) as usize && *observed == callback
        )));
    });
}

fn pre(cx: &mut Cx<'_>, ship: ShipId) {
    assert_dispatched(cx, ship, pre as *const () as usize);
    cx.ship_state_mut(ship).energy_counter = 7;
    cx.set_postprocess(ship, uqm_engine::battle::Selection::Override(post));
}

fn post(cx: &mut Cx<'_>, ship: ShipId) {
    assert_dispatched(cx, ship, post as *const () as usize);
    cx.ship_state_mut(ship).weapon_counter = 9;
}

fn spawn(
    cx: &mut Cx<'_>,
    ship: ShipId,
    origin: uqm_engine::battle::WeaponOrigin,
) -> uqm_engine::battle::WeaponSpawn {
    assert_dispatched(cx, ship, spawn as *const () as usize);
    let element = cx
        .alloc_element()
        .unwrap_or_else(|| panic!("fixture allocation must succeed"));
    cx.element_mut(element).current.location = origin.current.location;
    vec![None, Some(element)]
}

static RACE: uqm_engine::battle::Race = uqm_engine::battle::Race {
    descriptor: |_| panic!("fixture descriptor is supplied explicitly"),
    preprocess: Some(pre),
    postprocess: None,
    init_weapon: Some(spawn),
    intelligence: None,
    uninit: None,
};

fn attach_race(cx: &mut Cx<'_>, ship: ShipId) {
    let mut descriptor = uqm_engine::battle::record::RaceDescriptor::EMPTY;
    descriptor.ship_info = cx.desc(ship).ship_info;
    let mut descriptor = RaceDesc::fixture(Box::new(descriptor));
    descriptor.set_race(&RACE);
    cx.attach_descriptor(ship, descriptor);
}

#[test]
#[serial_test::serial]
fn record_dispatch_observes_before_call_and_reloads_callback_selection() {
    use uqm_engine::collections::observation::{self, Event};
    observation::enable();
    with_context(
        || panic!("dispatch does not draw RNG"),
        |cx| {
            let (ship, _) = owner(cx, 0, POINT { x: 10, y: 20 });
            attach_race(cx, ship);
            observation::drain();
            assert!(!cx.invoke_ship_process(ship, true));
            assert!(observation::drain().is_empty());
            assert!(cx.invoke_ship_process(ship, false));
            assert_eq!(cx.ship_state(ship).energy_counter, 7);
            assert!(cx.invoke_ship_process(ship, true));
            assert_eq!(cx.ship_state(ship).weapon_counter, 9);
            cx.set_preprocess(ship, uqm_engine::battle::Selection::Disabled);
            assert!(!cx.invoke_ship_process(ship, false));
            let callbacks: Vec<_> = observation::drain()
                .into_iter()
                .filter_map(|event| match event {
                    Event::Hook { parent, callback } => Some((parent, callback)),
                    _ => None,
                })
                .collect();
            let parent = cx.ship_pointer(ship) as usize;
            assert_eq!(
                callbacks,
                [
                    (parent, pre as *const () as usize),
                    (parent, post as *const () as usize)
                ]
            );
        },
    );
}

#[test]
#[serial_test::serial]
fn weapon_dispatch_keeps_failed_slots_and_supplied_origin_without_linking() {
    use uqm_engine::collections::observation::{self, Event};
    observation::enable();
    with_context(
        || panic!("dispatch does not draw RNG"),
        |cx| {
            let (ship, source) = owner(cx, 0, POINT { x: 10, y: 20 });
            attach_race(cx, ship);
            let mut origin = cx.weapon_origin(source);
            origin.current.location.x = 123;
            let head = cx.elements().head();
            observation::drain();
            let weapons = cx.spawn_weapons(ship, origin.clone());
            assert_eq!(weapons.len(), 2);
            assert!(weapons[0].is_none());
            let weapon = weapons[1].unwrap_or_else(|| panic!("fixture allocation must succeed"));
            assert_eq!(cx.element(weapon).current.location.x, 123);
            assert_eq!(cx.element(source).current.location.x, 10);
            assert_eq!(cx.elements().head(), head);
            assert!(!cx.elements().linked(weapon));
            let hooks = observation::drain()
                .into_iter()
                .filter(|event| matches!(event, Event::Hook { .. }))
                .count();
            assert_eq!(hooks, 1);
            cx.dispose(weapon);
            cx.set_weapon(ship, uqm_engine::battle::Selection::Disabled);
            observation::drain();
            assert!(cx.spawn_weapons(ship, origin).is_empty());
            assert!(observation::drain().is_empty());
        },
    );
}

#[test]
#[serial_test::serial]
fn empty_neighbors_and_cloak_projections_use_owned_records() {
    with_context(
        || panic!("record queries do not draw RNG"),
        |cx| {
            let (_, id) = owner(cx, 0, POINT { x: 10, y: 20 });
            assert!(!cx.gravity(id));
            assert!(!cx.matter_conflict(id));
            assert!(!cx.object_cloaked(id));
            assert!(!cx.subject_cloaked(&cx.weapon_origin(id)));
            cx.primitive_style_mut(id).kind = 2;
            cx.primitive_style_mut(id).color = uqm_engine::graphics::native_abi::Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            };
            assert!(cx.object_cloaked(id));
            assert!(cx.subject_cloaked(&cx.weapon_origin(id)));
            cx.primitive_style_mut(id).color.a = 254;
            assert!(!cx.object_cloaked(id));
            assert!(!cx.subject_cloaked(&cx.weapon_origin(id)));
            cx.primitive_style_mut(id).kind = uqm_engine::graphics::native_abi::EMPTY_PRIMITIVE;
            assert!(cx.object_cloaked(id));
            assert!(cx.subject_cloaked(&cx.weapon_origin(id)));
        },
    );
}

#[test]
#[serial_test::serial]
#[should_panic(expected = "ship has no race")]
fn dispatch_rejects_an_uninstalled_race() {
    with_context(
        || panic!("dispatch does not draw RNG"),
        |cx| {
            let (ship, _) = owner(cx, 0, POINT { x: 10, y: 20 });
            cx.invoke_ship_process(ship, false);
        },
    );
}
