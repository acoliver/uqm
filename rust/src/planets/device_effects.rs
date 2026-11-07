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

pub const DEVICE_STORY_KEYS: &[&str] = &[
    "ULTRON_CONDITION",
    "ROSY_SPHERE_ON_SHIP",
    "AQUA_HELIX_ON_SHIP",
    "CLEAR_SPINDLE_ON_SHIP",
    "USED_BROADCASTER",
    "GLOBAL_FLAGS_AND_DATA",
    "CHMMR_EMERGING",
    "READY_TO_CONFUSE_URQUAN",
    "SHIP_TO_COMPEL",
    "PORTAL_COUNTER",
];

pub fn device_effect_holds(
    id: u8,
    before: &std::collections::BTreeMap<String, u32>,
    after: &std::collections::BTreeMap<String, u32>,
) -> bool {
    let changed_to_one = |key: &str| before[key] == 0 && after[key] == 1;
    let encounter = before["activity"] & 0x0400 == 0 && after["activity"] & 0x0400 != 0;
    match id {
        0 => {
            before["fuel"].checked_sub(1000) == Some(after["fuel"])
                && changed_to_one("PORTAL_COUNTER")
        }
        1 => encounter,
        3 => changed_to_one("CHMMR_EMERGING"),
        4..=6 => {
            let key = [
                "ROSY_SPHERE_ON_SHIP",
                "AQUA_HELIX_ON_SHIP",
                "CLEAR_SPINDLE_ON_SHIP",
            ][usize::from(id - 4)];
            before[key] == 1
                && after[key] == 0
                && before["ULTRON_CONDITION"] > 0
                && Some(after["ULTRON_CONDITION"]) == before["ULTRON_CONDITION"].checked_add(1)
        }
        12 | 13 => {
            changed_to_one("USED_BROADCASTER")
                || (encounter && [16, 32, 128].contains(&after["GLOBAL_FLAGS_AND_DATA"]))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(condition: u32) -> World {
        let mut world = World::new();
        world.story = Some(crate::engine::world::story::StoryState::new());
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
