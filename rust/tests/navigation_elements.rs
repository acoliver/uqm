#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::{Battle, NativeFrameSet as NativeFrames, Object};
use uqm_rust::engine::battle::{ElementHook, ElementId};
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
fn interplanetary_flagship_keeps_motion_and_native_hooks() {
    battle_context::run(|_| {
        let _owner = Battle::new(8);
        let frames = NativeFrames::new(16);
        // SAFETY: the serialized fixture owns and restores navigation's live projections.
        unsafe {
            use uqm_rust::navigation::abi as nav;
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
                nav::DoMissions(cx);
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
fn interplanetary_group_preserves_parent_and_updates_its_own_record() {
    battle_context::run(|_| {
        let _owner = Battle::new(8);
        let frames = NativeFrames::new(16);
        // SAFETY: this serialized fixture owns the group pool, solar descriptors and frames.
        unsafe {
            use uqm_rust::navigation::abi as nav;
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
                group.task = (nav::ON_STATION | nav::IGNORE_FLAGSHIP) as u8;
                group.loc = POINT { x: 300, y: 400 };
                group.melee_icon = frames.frame(0);
                queue.put(id);
                (id, queue.raw(id).cast::<nav::IP_GROUP>())
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
                nav::DoMissions(cx);
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
            nav::NotifyOthers(0, 2);
            assert_eq!((*group).dest_loc, 2);
            assert_ne!((*group).task & nav::IGNORE_FLAGSHIP as u8, 0);
            uqm_rust::engine::World::current().write(|world| world.ip_groups().reinit());
            (**uqm_rust::engine::WorldData::current()).game.ship_stamp = old_stamp;
            (**uqm_rust::engine::WorldData::current()).game.ip_location = old_location;
            (**uqm_rust::engine::Kernel::current()).stars_in_space = old_stars;
            uqm_rust::engine::World::current().write(|w| w.encounter_race = old_race);
        }
    });
}
