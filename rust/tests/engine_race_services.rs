use uqm_engine::battle::{Battle, Cx, ElementId, RaceDesc, ShipId};
use uqm_engine::Game;

fn with_context(call: impl FnOnce(&mut Cx<'_>)) {
    let mut game = Game::new();
    game.world.story = Some(uqm_engine::world::story::StoryState::new());
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
        cx.ship_state_mut(survivor).status |= 512;
        cx.select_self_destruct_winner(dead);
        assert_eq!(cx.ship_state(survivor).status & 512, 512);
        cx.select_self_destruct_winner(cx.ship_element(survivor));
        assert_eq!(cx.ship_state(survivor).status & 512, 0);
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

#[test]
#[serial_test::serial]
fn queued_ship_effects_preserve_order_and_silence_suppresses_later_sounds() {
    with_context(|cx| {
        let (first, a) = owner(cx, 0);
        let (second, b) = owner(cx, 1);
        cx.process_ship_sound(first, 0, a);
        cx.process_ship_sound(second, 0, b);
        assert_eq!(cx.kernel.sounds.count, 2);
        assert_eq!(cx.kernel.sounds.objects[..2], [Some(a), Some(b)]);
        cx.silence_battle_sounds();
        cx.process_ship_sound(first, 0, a);
        assert_eq!(cx.kernel.sounds.count, 8);
        assert!(cx.kernel.sounds.objects.iter().all(Option::is_none));
        assert!(cx.kernel.sounds.sounds.iter().all(Option::is_none));
    });
}

#[test]
#[serial_test::serial]
fn real_death_and_unlink_services_use_only_the_supplied_owners() {
    with_context(|cx| {
        uqm_rust::battle::services::install(cx.kernel);
        let (_, a) = owner(cx, 0);
        let (_, b) = owner(cx, 1);
        cx.world.set_activity(uqm_rust::ships::native::abi::IN_ENCOUNTER as u16);
        cx.b.counters = [2, 3];
        cx.record_ship_death(a);
        assert_eq!(cx.b.counters, [1, 3]);
        cx.element_mut(b).mass = 11;
        cx.record_ship_death(b);
        assert_eq!(cx.b.counters, [1, 3]);
        cx.set_target(b, Some(a));
        cx.untarget(a);
        assert!(cx.target(b).is_none());
        cx.unlink_element(a);
        assert_eq!(cx.elements().head(), Some(b));
        assert!(!cx.elements().linked(a));
        cx.dispose(a);
    });
}

fn intelligence(cx: &mut Cx<'_>, ship: ShipId, concerns: &mut uqm_engine::battle::Concerns) {
    use uqm_engine::collections::observation::{self, Event};
    observation::inspect_pending(|events| {
        assert!(events.iter().any(|event| matches!(event,
            Event::Hook { callback, parent }
            if *callback == intelligence as *const () as usize
                && *parent == cx.ship_pointer(ship) as usize)));
    });
    concerns.subject.turn = 7;
    concerns.objects[4].which_turn = 19;
    cx.ship_state_mut(ship).input = 3;
}

static RACE: uqm_engine::battle::Race = uqm_engine::battle::Race {
    descriptor: |_| panic!("descriptor is supplied by fixture"),
    preprocess: None,
    postprocess: None,
    init_weapon: None,
    intelligence: Some(intelligence),
    uninit: None,
};

#[test]
#[serial_test::serial]
fn engine_intelligence_dispatch_observes_before_mutating_the_supplied_projection() {
    use uqm_engine::battle::{Concern, Concerns};
    use uqm_engine::collections::observation;
    with_context(|cx| {
        let (ship, element) = owner(cx, 0);
        let mut descriptor = uqm_engine::battle::record::RaceDescriptor::EMPTY;
        descriptor.ship_info.crew_level = 10;
        let mut descriptor = RaceDesc::fixture(Box::new(descriptor));
        descriptor.set_race(&RACE);
        cx.attach_descriptor(ship, descriptor);
        let mut concerns = Concerns {
            subject: cx.weapon_origin(element),
            objects: [Concern {
                object: None,
                facing: 0,
                which_turn: 0,
                movement: 0,
            }; 10],
            count: 4,
        };
        observation::enable();
        observation::drain();
        assert!(cx.race_intelligence(ship, &mut concerns));
        assert_eq!(concerns.subject.turn, 7);
        assert_eq!(concerns.objects[4].which_turn, 19);
        assert_eq!(cx.element(element).turn, 0);
        assert_eq!(cx.ship_state(ship).input, 3);
        assert_eq!(observation::drain().len(), 1);
    });
}
