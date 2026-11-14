#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::{Battle, NativeFrameSet as NativeFrames, Object};
use uqm_rust::engine::battle::{ElementHook, ElementId};
use uqm_rust::engine::world::queues::IpGroup;
use uqm_rust::navigation::interplanetary;
use uqm_rust::ships::native::abi::*;

fn transition_hook() -> ElementHook {
    ElementHook::Native(uqm_rust::navigation::hyper_encounters::transition)
}

fn call_transition(id: ElementId) {
    // SAFETY: the test owns the serialized element pool and retains no record borrow.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            uqm_rust::battle::process_native::invoke_element::invoke_element(
                cx,
                transition_hook(),
                id,
            );
        });
    }
}

#[test]
#[serial_test::serial]
fn encounter_transition_preserves_frame_cadence_and_completion() {
    battle_context::run(|_| {
        let owner = Battle::new(8);
        let frames = NativeFrames::new(12);
        let mut element = Object::new(&owner);
        // SAFETY: this test owns the allocated element and its frame set.
        unsafe {
            let record = element.element_mut();
            record.state_flags = DISAPPEARING as u16;
            record.current.image.frames = uqm_rust::engine::battle::frames::Frames::shared(
                uqm_rust::engine::battle::frames::SharedFrames::Fixture(0),
                frames.array(),
            );
            record.current.image.frame = frames.selection(10);
            record.next = record.current.clone();
            record.set_turn_wait(1);
            record.death_func = Some(transition_hook());
        }
        call_transition(element.id);
        // SAFETY: the callback has returned; the fixture still owns this element.
        unsafe {
            let record = element.element_mut();
            assert_eq!(record.state_flags & DISAPPEARING as u16, 0);
            assert_eq!(record.life_span(), 1);
            assert_eq!(record.turn_wait(), 0);
            assert_eq!(record.next.image.frame.raw(), frames.frame(10));
        }
        call_transition(element.id);
        // SAFETY: the callback has returned and no borrow crosses the next call.
        unsafe {
            let record = element.element_mut();
            assert_eq!(record.turn_wait(), 1);
            assert_eq!(record.next.image.frame.raw(), frames.frame(11));
            record.current = record.next.clone();
            record.set_turn_wait(0);
        }
        call_transition(element.id);
        // SAFETY: the callback has returned; the fixture retains the frame set.
        unsafe {
            let record = element.element_mut();
            assert!(record.death_func.is_none());
            record.set_hit_points(1);
            record.set_turn_wait(0);
            record.current.image.frame = frames.selection(0);
        }
        call_transition(element.id);
        // SAFETY: the fixture retains its live element and frame set.
        unsafe {
            assert_eq!(
                element.element_mut().next.image.frame.raw(),
                frames.frame(11)
            );
        }
    });
}

#[test]
#[serial_test::serial]
fn interplanetary_flagship_keeps_motion_and_native_hooks() {
    battle_context::run(|_| {
        let _owner = Battle::new(8);
        let frames = NativeFrames::new(16);
        // SAFETY: the serialized fixture owns and restores navigation's live projections.
        unsafe {
            let mut solar = uqm_rust::engine::World::current().read(|world| {
                uqm_rust::planets::solar_system::SolarSystem::new(
                    world,
                    uqm_rust::engine::StarId(0),
                )
            });
            solar.sys.sun.radius = 128 << 6;
            let old_race = uqm_rust::engine::World::current().read(|w| w.encounter_race);
            uqm_rust::engine::World::current().write(|w| w.encounter_race = -1);
            let old_stamp = (**uqm_rust::engine::WorldData::current()).game.ship_stamp;
            let old_location = (**uqm_rust::engine::WorldData::current()).game.ip_location;
            let old_velocity = (**uqm_rust::engine::WorldData::current()).game.velocity;
            (**uqm_rust::engine::WorldData::current())
                .game
                .ship_stamp
                .frame = frames.frame(0);
            (**uqm_rust::engine::WorldData::current())
                .game
                .ship_stamp
                .origin = POINT { x: 80, y: 60 };
            (**uqm_rust::engine::WorldData::current()).game.ip_location = POINT { x: 100, y: 200 };
            let mut velocity = old_velocity;
            velocity.set_components(20, -30);
            (**uqm_rust::engine::WorldData::current()).game.velocity = velocity;
            uqm_rust::app::bridge::enter(|cx| {
                solar.refresh_frame(cx);
                interplanetary::do_missions(cx);
                let flagship = cx.elements().head().expect("spawned flagship");
                let state = cx.element(flagship);
                let Some(ElementHook::Native(pre)) = state.pre else {
                    panic!("flagship preprocess must be Native");
                };
                assert!(matches!(
                    state.collide,
                    Some(uqm_rust::engine::battle::CollisionHook::Native(_))
                ));
                uqm_rust::battle::process_native::invoke_element::invoke_element(
                    cx,
                    ElementHook::Native(pre),
                    flagship,
                );
                assert_eq!(cx.element(flagship).thrust, 41);
            });
            let location = (**uqm_rust::engine::WorldData::current()).game.ip_location;
            assert_eq!(location.x, 120);
            assert_eq!(location.y, 170);
            (**uqm_rust::engine::WorldData::current()).game.ship_stamp = old_stamp;
            (**uqm_rust::engine::WorldData::current()).game.ip_location = old_location;
            (**uqm_rust::engine::WorldData::current()).game.velocity = old_velocity;
            uqm_rust::engine::World::current().write(|w| w.encounter_race = old_race);
        }
    });
}

#[test]
#[serial_test::serial]
fn interplanetary_group_preserves_parent_and_updates_its_own_record() {
    battle_context::run(|_| {
        let _owner = Battle::new(8);
        let frames = NativeFrames::new(16);
        // SAFETY: this serialized fixture owns the group pool, solar descriptors and frames.
        unsafe {
            let mut solar = uqm_rust::engine::World::current().read(|world| {
                uqm_rust::planets::solar_system::SolarSystem::new(
                    world,
                    uqm_rust::engine::StarId(0),
                )
            });
            solar.sys.sun.radius = 128 << 6;
            solar.sys.sun.num_planets = 1;
            solar.sys.planets[0].image.origin = POINT { x: 80, y: 60 };
            solar.sys.planets[0].num_planets = 1;
            solar.enter_inner(0);
            solar.select_body(uqm_rust::engine::world::generation::Body::Moon {
                planet: 0,
                moon: 0,
            });
            let old_race = uqm_rust::engine::World::current().read(|w| w.encounter_race);
            let old_stamp = (**uqm_rust::engine::WorldData::current()).game.ship_stamp;
            let old_location = (**uqm_rust::engine::WorldData::current()).game.ip_location;
            let old_stars = (**uqm_rust::engine::Kernel::current()).stars_in_space;
            let (group_id, group) = uqm_rust::engine::World::current().write(|world| {
                let mut queue = world.ip_groups();
                queue.reinit();
                let id = queue.alloc().unwrap();
                let group = queue.get_mut(id);
                group.race_id = 0;
                group.group_id = 7;
                group.in_system = 1;
                group.dest_loc = 1;
                group.task = IpGroup::ON_STATION | IpGroup::IGNORE_FLAGSHIP;
                group.loc = POINT { x: 300, y: 400 };
                group.melee_icon = frames.frame(0);
                queue.put(id);
                (id, queue.raw(id).cast::<IpGroup>())
            });
            uqm_rust::engine::World::current().write(|w| w.encounter_race = -1);
            (**uqm_rust::engine::Kernel::current()).stars_in_space = frames.frame(0);
            (**uqm_rust::engine::WorldData::current())
                .game
                .ship_stamp
                .frame = frames.frame(0);
            (**uqm_rust::engine::WorldData::current()).game.ip_location = POINT { x: 100, y: 200 };
            uqm_rust::app::bridge::enter(|cx| {
                solar.refresh_frame(cx);
                interplanetary::do_missions(cx);
                let flagship = cx.elements().head().expect("spawned flagship");
                assert_eq!(cx.element(flagship).turn, 1);
                let id = cx.elements().succ(flagship).expect("spawned group");
                assert_eq!(
                    cx.parent(id),
                    uqm_rust::engine::battle::Parent::IpGroup(group_id)
                );
                let state = cx.element(id);
                assert_eq!(cx.frame_number(state.current.image.frame.view()), 1);
                let hook = state.death.expect("group movement hook");
                assert!(matches!(hook, ElementHook::Native(_)));
                uqm_rust::battle::process_native::invoke_element::invoke_element(cx, hook, id);
                assert_eq!(cx.element(id).flags & CHANGING as u16, CHANGING as u16);
                assert_eq!(cx.element(id).flags & NONSOLID as u16, NONSOLID as u16);
                assert_eq!(
                    cx.parent(id),
                    uqm_rust::engine::battle::Parent::IpGroup(group_id)
                );
            });
            assert_ne!(((*group).loc.x, (*group).loc.y), (300, 400));
            assert_eq!((*group).sys_loc, 0);
            assert_eq!((*group).group_id, 7);
            uqm_rust::app::bridge::enter(|cx| {
                let flagship = cx.elements().head().expect("spawned flagship");
                let id = cx.elements().succ(flagship).expect("spawned group");
                let hook = cx.element(id).death.expect("group movement hook");
                cx.world.ip_groups().get_mut(group_id).sys_loc = 1;
                uqm_rust::battle::process_native::invoke_element::invoke_element(cx, hook, id);
                assert_eq!(
                    cx.element(id).flags & (DISAPPEARING | NONSOLID) as u16,
                    0,
                    "a group returning to the flagship's location must become collidable"
                );
            });
            uqm_rust::app::bridge::enter(|cx| interplanetary::notify_others(cx, 0, 2));
            assert_eq!((*group).dest_loc, 2);
            assert_ne!((*group).task & IpGroup::IGNORE_FLAGSHIP, 0);
            uqm_rust::engine::World::current().write(|world| world.ip_groups().reinit());
            (**uqm_rust::engine::WorldData::current()).game.ship_stamp = old_stamp;
            (**uqm_rust::engine::WorldData::current()).game.ip_location = old_location;
            (**uqm_rust::engine::Kernel::current()).stars_in_space = old_stars;
            uqm_rust::engine::World::current().write(|w| w.encounter_race = old_race);
        }
    });
}

#[test]
fn notifications_change_only_the_supplied_unpublished_campaign() {
    use uqm_rust::engine::{battle::Cx, world::queues::IpGroup, Game, World};
    use uqm_rust::navigation::interplanetary::notify_others;

    let mut game = Game::new();
    let mut other = World::new();
    for world in [&mut game.world, &mut other] {
        let mut groups = world.ip_groups();
        let id = groups
            .alloc()
            .unwrap_or_else(|| panic!("fixture group capacity"));
        *groups.get_mut(id) = IpGroup {
            race_id: 4,
            task: IpGroup::ON_STATION | IpGroup::IGNORE_FLAGSHIP,
            orbit_pos: 6,
            dest_loc: 3,
            ..Default::default()
        };
        groups.put(id);
    }
    let mut battle = uqm_rust::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
        panic!("targeted notifications must not draw RNG")
    });
    notify_others(&mut cx, 4, 2);
    let id = cx
        .world
        .ip_groups_ref()
        .head()
        .unwrap_or_else(|| panic!("fixture group"));
    assert_eq!(cx.world.ip_groups_ref().get(id).dest_loc, 2);
    notify_others(&mut cx, 4, 0);
    let group = *cx.world.ip_groups_ref().get(id);
    assert_eq!(
        (group.task, group.orbit_pos, group.dest_loc),
        (IpGroup::ON_STATION, 2, 0)
    );
    let groups = other.ip_groups_ref();
    let id = groups
        .head()
        .unwrap_or_else(|| panic!("other campaign group"));
    let group = groups.get(id);
    assert_eq!(
        (group.task, group.orbit_pos, group.dest_loc),
        (IpGroup::ON_STATION | IpGroup::IGNORE_FLAGSHIP, 6, 3)
    );
}

#[test]
fn all_clear_uses_the_supplied_rng_and_preserves_reforming_groups() {
    use uqm_rust::engine::{battle::Cx, world::queues::IpGroup, Game};
    use uqm_rust::navigation::interplanetary::notify_others;

    let mut game = Game::new();
    for (race, task, dest, orbit, counter) in [
        (4, IpGroup::EXPLORE, 0, 3, 99),
        (4, IpGroup::FLEE | IpGroup::REFORM_GROUP, 2, 5, 17),
        (5, IpGroup::EXPLORE, 1, 4, 33),
    ] {
        let mut groups = game.world.ip_groups();
        let id = groups
            .alloc()
            .unwrap_or_else(|| panic!("fixture group capacity"));
        *groups.get_mut(id) = IpGroup {
            race_id: race,
            task,
            dest_loc: dest,
            orbit_pos: orbit,
            group_counter: counter,
            ..Default::default()
        };
        groups.put(id);
    }
    let mut battle = uqm_rust::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 7);
    notify_others(&mut cx, 4, u8::MAX);
    let groups = cx.world.ip_groups_ref();
    let first = groups.head().unwrap_or_else(|| panic!("exploring group"));
    let second = groups
        .succ(first)
        .unwrap_or_else(|| panic!("reforming group"));
    let third = groups
        .succ(second)
        .unwrap_or_else(|| panic!("unrelated race"));
    let first = groups.get(first);
    assert_eq!(first.task, IpGroup::EXPLORE | IpGroup::IGNORE_FLAGSHIP);
    assert_eq!(
        (first.dest_loc, first.orbit_pos, first.group_counter),
        (3, 7, 32)
    );
    let second = groups.get(second);
    assert_eq!(
        second.task,
        IpGroup::FLEE | IpGroup::REFORM_GROUP | IpGroup::IGNORE_FLAGSHIP
    );
    assert_eq!(
        (second.dest_loc, second.orbit_pos, second.group_counter),
        (2, 5, 17)
    );
    let third = groups.get(third);
    assert_eq!(third.task, IpGroup::EXPLORE);
    assert_eq!(
        (third.dest_loc, third.orbit_pos, third.group_counter),
        (1, 4, 33)
    );
}
