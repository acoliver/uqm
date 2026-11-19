//! Portal animation state used by hyperspace radar and contact elements.

use super::hyper_units::{ARILOU_SPACE_X, ARILOU_SPACE_Y, QUASI_SPACE_X, QUASI_SPACE_Y};
use crate::engine::graphics::native_abi::POINT;
use crate::engine::{Kernel, StoryKey, World};

pub(super) fn portals(
    world: &mut World,
    kernel: &mut Kernel,
    universe: POINT,
) -> ([(POINT, u16); 2], usize) {
    let counter = world.story_state(StoryKey::ARILOU_SPACE_COUNTER) as u8;
    let side = world.story_state(StoryKey::ARILOU_SPACE_SIDE) as u8;
    let mut portal = world.story_state(StoryKey::PORTAL_COUNTER) as u8;
    let mut portals = [(POINT { x: 0, y: 0 }, 0u16); 2];
    let mut count = 0usize;
    if portal != 0 {
        if portal == 1 {
            kernel.portal_point = universe;
        }
        portals[count] = (kernel.portal_point, ((portal as u16 - 1) >> 1) + 18);
        count += 1;
        portal += 1;
        if portal == 11 {
            portal = 10;
        }
        world.set_story_state(StoryKey::PORTAL_COUNTER, u32::from(portal));
    }
    if counter != 0 {
        let (point, index) = if side <= 1 {
            (
                POINT {
                    x: ARILOU_SPACE_X as i16,
                    y: ARILOU_SPACE_Y as i16,
                },
                (counter as u16 >> 1) + 18,
            )
        } else {
            (
                POINT {
                    x: QUASI_SPACE_X as i16,
                    y: QUASI_SPACE_Y as i16,
                },
                counter as u16 >> 1,
            )
        };
        portals[count] = (point, index);
        count += 1;
    }
    (portals, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let mut world = World::new();
        world.story = Some(crate::engine::world::story::StoryState::new());
        world
    }

    #[test]
    fn inactive_portals_produce_no_elements() {
        let mut world = world();
        let mut kernel = Kernel::new();
        let (_, count) = portals(&mut world, &mut kernel, POINT { x: 12, y: 34 });
        assert_eq!(count, 0);
    }

    #[test]
    fn manual_portal_captures_position_once_and_holds_its_last_frame() {
        let mut world = world();
        let mut kernel = Kernel::new();
        world.set_story_state(StoryKey::PORTAL_COUNTER, 1);
        let (entries, count) = portals(&mut world, &mut kernel, POINT { x: 12, y: 34 });
        assert_eq!(count, 1);
        assert_eq!((entries[0].0.x, entries[0].0.y, entries[0].1), (12, 34, 18));
        assert_eq!(world.story_state(StoryKey::PORTAL_COUNTER), 2);
        world.set_story_state(StoryKey::PORTAL_COUNTER, 10);
        let (entries, _) = portals(&mut world, &mut kernel, POINT { x: 56, y: 78 });
        assert_eq!((entries[0].0.x, entries[0].0.y, entries[0].1), (12, 34, 22));
        assert_eq!(world.story_state(StoryKey::PORTAL_COUNTER), 10);
    }

    #[test]
    fn arilou_side_selects_the_original_coordinates_and_frame_band() {
        for side in 0..4 {
            let mut world = world();
            let mut kernel = Kernel::new();
            world.set_story_state(StoryKey::PORTAL_COUNTER, 2);
            world.set_story_state(StoryKey::ARILOU_SPACE_COUNTER, 8);
            world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, side);
            let (entries, count) = portals(&mut world, &mut kernel, POINT { x: 0, y: 0 });
            assert_eq!(count, 2);
            let (point, frame) = entries[1];
            let expected = if side <= 1 {
                (ARILOU_SPACE_X as i16, ARILOU_SPACE_Y as i16, 22)
            } else {
                (QUASI_SPACE_X as i16, QUASI_SPACE_Y as i16, 4)
            };
            assert_eq!((point.x, point.y, frame), expected);
        }
    }
}
