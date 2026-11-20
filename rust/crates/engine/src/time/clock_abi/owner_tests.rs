use super::*;
use crate::World;

fn world() -> World {
    let mut world = World::new();
    world.story = Some(crate::world::story::StoryState::new());
    world.data.game.game_clock.year_index = 2156;
    world.data.game.game_clock.month_index = 2;
    world.data.game.game_clock.day_index = 28;
    world.clock.init();
    world
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
            assert_eq!(temporary.len(), crate::clock::NUM_EVENTS - 2);
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
    for _ in 0..crate::clock::NUM_EVENTS {
        assert!(world.clock.events().alloc().is_some());
    }
    assert!(world.clock.events().alloc().is_none());
}
