use super::conv;
use crate::engine::{StoryKey, World};

pub(super) fn restored_conversation(world: &World) -> u32 {
    let flags = world.story_state(StoryKey::GLOBAL_FLAGS_AND_DATA);
    if world.story_state(StoryKey::FOUND_PLUTO_SPATHI) == 1 {
        conv::SPATHI
    } else if flags == 0 {
        conv::TALKING_PET
    } else if flags & ((1 << 4) | (1 << 5)) != 0 {
        conv::ILWRATH
    } else {
        conv::CHMMR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_conversation_preserves_pluto_and_global_flag_precedence() {
        let mut world = World::new();
        world.story = Some(crate::engine::world::story::GameState::new());
        for (pluto, flags, expected) in [
            (1, 0, conv::SPATHI),
            (1, 0x30, conv::SPATHI),
            (0, 0, conv::TALKING_PET),
            (0, 0x10, conv::ILWRATH),
            (0, 0x20, conv::ILWRATH),
            (0, 0x30, conv::ILWRATH),
            (0, 1, conv::CHMMR),
        ] {
            world.set_story_state(StoryKey::FOUND_PLUTO_SPATHI, pluto);
            world.set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, flags);
            assert_eq!(restored_conversation(&world), expected);
        }
    }
}
