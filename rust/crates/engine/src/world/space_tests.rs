use super::*;
use activity::ActivityKind;

#[test]
fn space_classification_masks_activity_flags_and_checks_both_sides() {
    let hyperspace = ActivityKind::InHyperspace as u16;
    let mut world = World::new();
    world.story = Some(StoryState::new());
    for activity in [hyperspace, hyperspace | 0xff00] {
        world.set_activity(activity);
        for side in 0..=3 {
            world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, side);
            assert!(world.in_hqspace());
            assert_eq!(world.in_hyper_space(), side <= 1);
            assert_eq!(world.in_quasi_space(), side > 1);
        }
    }
    world.set_activity(ActivityKind::InInterplanetary as u16);
    assert!(!world.in_hqspace());
    assert!(!world.in_hyper_space());
    assert!(!world.in_quasi_space());
}
