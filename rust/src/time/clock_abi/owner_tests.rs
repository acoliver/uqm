use super::*;
use crate::engine::{clock::ARILOU_UMGAH_CHECK, StoryKey, World};

fn world() -> World {
    let mut world = World::new();
    world.story = Some(crate::engine::world::story::StoryState::new());
    world.data.game.game_clock.year_index = 2156;
    world.data.game.game_clock.month_index = 2;
    world.data.game.game_clock.day_index = 28;
    world.clock.init();
    world
}

#[test]
fn scheduling_dispatches_immediate_events_even_when_the_pool_is_full() {
    let mut world = world();
    let mut random = || panic!("story event sampled RNG");
    let mut dispatch = crate::navigation::events::dispatcher((320, 240), &mut random);
    assert!(schedule_event(&mut world, &mut dispatch, 0, 2, 27, 2156, 99).is_none());
    assert_eq!(world.clock.events_ref().count(), 0);
    for selector in 0..crate::engine::clock::NUM_EVENTS {
        assert!(schedule_event(&mut world, &mut dispatch, 1, 0, 1, 0, selector as u8).is_some());
    }
    assert!(schedule_event(&mut world, &mut dispatch, 1, 0, 1, 0, 99).is_none());
    assert!(schedule_event(
        &mut world,
        &mut dispatch,
        1,
        0,
        0,
        0,
        ARILOU_UMGAH_CHECK as u8
    )
    .is_none());
    assert_eq!(world.story_state(StoryKey::ARILOU_CHECKED_UMGAH), 2);
    assert_eq!(
        world.clock.events_ref().count(),
        crate::engine::clock::NUM_EVENTS
    );
}

#[test]
fn production_day_drain_removes_before_dispatch_and_frees_after_callback() {
    let mut world = world();
    let first = insert_event(&mut world.clock, (2156, 2, 28), 1).unwrap();
    insert_event(&mut world.clock, (2156, 2, 28), 2).unwrap();
    let mut seen = Vec::new();
    dispatch_day_events(&mut world, (2156, 2, 28), |world, selector| {
        seen.push(selector);
        if selector == 1 {
            let mut queue = world.clock.events();
            assert!(!queue.linked(first));
            assert!(queue.contains(first));
            let mut temporary = Vec::new();
            while let Some(id) = queue.alloc() {
                assert_ne!(id, first);
                temporary.push(id);
            }
            assert_eq!(temporary.len(), crate::engine::clock::NUM_EVENTS - 2);
            for id in temporary {
                queue.free(id);
            }
            insert_event(&mut world.clock, (2156, 2, 28), 3).unwrap();
        } else {
            assert!(!world.clock.events_ref().contains(first));
        }
    });
    assert_eq!(seen, [1, 2, 3]);
    assert_eq!(world.clock.events_ref().count(), 0);
    for _ in 0..crate::engine::clock::NUM_EVENTS {
        assert!(world.clock.events().alloc().is_some());
    }
    assert!(world.clock.events().alloc().is_none());
}
