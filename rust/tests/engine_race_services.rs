use uqm_engine::battle::{Battle, Cx, ElementId, RaceDesc, ShipId};
use uqm_engine::Game;

fn with_context(call: impl FnOnce(&mut Cx<'_>)) {
    let mut game = Game::new();
    game.kernel.reset_element_capacity(8);
    game.kernel.reset_ship_capacity(0, 2);
    game.kernel.reset_ship_capacity(1, 2);
    let mut battle = Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
        panic!("race projections and winner selection must not draw RNG")
    });
    call(&mut cx);
}

fn owner(cx: &mut Cx<'_>, side: usize) -> (ShipId, ElementId) {
    let ship = {
        let mut ships = cx.kernel.ships(side);
        let link = ships
            .alloc()
            .unwrap_or_else(|| panic!("fixture allocation"));
        let ship = ships.ship_id(link);
        ships.put(link);
        ship
    };
    let mut descriptor = uqm_engine::battle::record::RaceDescriptor::EMPTY;
    descriptor.ship_info.crew_level = 10;
    cx.attach_descriptor(ship, RaceDesc::fixture(Box::new(descriptor)));
    let element = cx
        .alloc_element()
        .unwrap_or_else(|| panic!("fixture allocation"));
    cx.element_mut(element).flags = 4;
    cx.element_mut(element).life = 1;
    cx.element_mut(element).player = side as i16;
    cx.set_parent(element, Some(ship));
    cx.set_ship_element(ship, Some(element));
    cx.put(element);
    (ship, element)
}

#[test]
#[serial_test::serial]
fn race_effects_require_installation_on_the_supplied_kernel() {
    with_context(|cx| {
        let (_, dead) = owner(cx, 0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cx.select_self_destruct_winner(dead);
        }));
        let Err(error) = result else {
            panic!("uninstalled race services must fail");
        };
        assert_eq!(
            error.downcast_ref::<&str>(),
            Some(&"battle race services were not installed")
        );
        assert!(cx.winner().is_none());
        uqm_rust::battle::services::install(cx.kernel);
        cx.select_self_destruct_winner(dead);
        assert_eq!(cx.winner(), Some(cx.owner(dead)));
    });
}

#[test]
#[serial_test::serial]
fn self_destruct_selects_survivor_and_clears_only_its_own_victory_flag() {
    with_context(|cx| {
        uqm_rust::battle::services::install(cx.kernel);
        let (dead_ship, dead) = owner(cx, 0);
        let (survivor, _) = owner(cx, 1);
        cx.element_mut(dead).life = 0;
        cx.select_self_destruct_winner(dead);
        assert_eq!(cx.winner(), Some(survivor));
        cx.ship_state_mut(survivor).status |= 1;
        cx.select_self_destruct_winner(dead);
        assert_eq!(cx.ship_state(survivor).status & 1, 1);
        cx.select_self_destruct_winner(cx.ship_element(survivor));
        assert_eq!(cx.ship_state(survivor).status & 1, 0);
        assert_ne!(dead_ship, survivor);
    });
}

#[test]
#[serial_test::serial]
fn race_record_projections_do_not_require_mode_installation_or_publication() {
    with_context(|cx| {
        cx.set_flagship_crew(43);
        assert_eq!(cx.world.flagship_config().crew, 43);
        cx.set_autopilot(uqm_engine::graphics::native_abi::POINT { x: 17, y: 29 });
        assert_eq!((cx.autopilot().x, cx.autopilot().y), (17, 29));
        let mut velocity = cx.flagship_velocity();
        velocity.vector.width = 31;
        cx.set_flagship_velocity(velocity);
        assert_eq!(cx.flagship_velocity().vector.width, 31);
        cx.set_story_state(uqm_engine::StoryKey::PORTAL_COUNTER, 7);
        assert_eq!(cx.portal_counter(), 7);
        assert_eq!(cx.story_state(uqm_engine::StoryKey::PORTAL_COUNTER), 7);
    });
}
