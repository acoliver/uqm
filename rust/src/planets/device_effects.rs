//! Story transitions for inventory-device use.

use crate::engine::{StoryKey, World};

pub(super) const ULTRON_COMPONENTS: [StoryKey; 3] = [
    StoryKey::ROSY_SPHERE_ON_SHIP,
    StoryKey::AQUA_HELIX_ON_SHIP,
    StoryKey::CLEAR_SPINDLE_ON_SHIP,
];

pub(super) fn repair_ultron(world: &mut World, component: StoryKey) -> u8 {
    let condition = world.story_state(StoryKey::ULTRON_CONDITION) as u8;
    if condition == 0 {
        return 0;
    }
    world.set_story_state(StoryKey::ULTRON_CONDITION, u32::from(condition + 1));
    for key in [
        component,
        StoryKey::DISCUSSED_ULTRON,
        StoryKey::SUPOX_ULTRON_HELP,
    ] {
        world.set_story_state(key, 0);
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(condition: u32) -> World {
        let mut world = World::new();
        world.story = Some(crate::engine::world::story::GameState::new());
        world.set_story_state(StoryKey::ULTRON_CONDITION, condition);
        world.set_story_state(StoryKey::DISCUSSED_ULTRON, 1);
        world.set_story_state(StoryKey::SUPOX_ULTRON_HELP, 1);
        for component in ULTRON_COMPONENTS {
            world.set_story_state(component, 1);
        }
        world
    }

    #[test]
    fn cannot_apply_a_component_without_the_ultron() {
        let mut world = world(0);
        assert_eq!(repair_ultron(&mut world, ULTRON_COMPONENTS[0]), 0);
        assert_eq!(world.story_state(ULTRON_COMPONENTS[0]), 1);
        assert_eq!(world.story_state(StoryKey::DISCUSSED_ULTRON), 1);
        assert_eq!(world.story_state(StoryKey::SUPOX_ULTRON_HELP), 1);
    }

    #[test]
    fn each_component_advances_the_ultron_and_resets_its_dialogue() {
        for (index, component) in ULTRON_COMPONENTS.into_iter().enumerate() {
            let mut world = world(index as u32 + 1);
            assert_eq!(repair_ultron(&mut world, component), 1);
            assert_eq!(
                world.story_state(StoryKey::ULTRON_CONDITION),
                index as u32 + 2
            );
            assert_eq!(world.story_state(StoryKey::DISCUSSED_ULTRON), 0);
            assert_eq!(world.story_state(StoryKey::SUPOX_ULTRON_HELP), 0);
            for other in ULTRON_COMPONENTS {
                assert_eq!(world.story_state(other), u32::from(other != component));
            }
        }
    }
}
