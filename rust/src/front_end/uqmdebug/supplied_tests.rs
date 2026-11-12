use super::*;
use crate::engine::{clock::CEvent, world::story::StoryState, StoryKey, World};

#[test]
fn devices_change_only_the_supplied_unpublished_campaign() {
    let mut first = World::new();
    first.story = Some(StoryState::new());
    let mut second = World::new();
    second.story = Some(StoryState::new());
    give_devices(&mut first);
    for key in [StoryKey::UTWIG_BOMB_ON_SHIP, StoryKey::PORTAL_KEY_ON_SHIP] {
        assert_eq!(first.story_state(key), 1);
        assert_eq!(second.story_state(key), 0);
    }
}

#[test]
fn event_output_preserves_legacy_bytes_and_reports_writer_failure() {
    let event = CEvent {
        year_index: 2155,
        month_index: 2,
        day_index: 7,
        func_index: HYPERSPACE_ENCOUNTER_EVENT as u8,
    };
    let mut bytes = Vec::new();
    dump_event(&mut bytes, &event).unwrap();
    assert_eq!(bytes, b"2155/02/07: HYPERSPACE_ENCOUNTER_EVENT\n");
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        dump_event(&mut Broken, &event).unwrap_err().kind(),
        std::io::ErrorKind::BrokenPipe
    );
}

#[test]
fn numeric_labels_are_independent_owned_values() {
    let first = tectonics_string(17);
    let second = tectonics_string(18);
    assert_eq!(first, "17");
    assert_eq!(second, "18");
    assert_eq!(tectonics_string(0), "none");
}

#[test]
fn creature_values_keep_the_low_nibble_and_invalid_sentinel() {
    use crate::planet_side::creatures::{CreatureCatalog, CreatureKind};
    for value in 0..=u8::MAX {
        let expected = CreatureKind::new(value)
            .map(|kind| i32::from(CreatureCatalog::raw_lifedata(kind).1 & 15))
            .unwrap_or(-1);
        assert_eq!(uqm_debug_creature_bio_value(value), expected);
    }
}
