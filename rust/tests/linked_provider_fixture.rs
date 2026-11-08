//! Linked fixtures for debug consumers and game-state initialization.

#[cfg(feature = "debug-process")]
use uqm_rust::front_end::uqmdebug::uqm_debug_creature_bio_value;

#[test]
#[cfg(feature = "debug-process")]
fn debug_consumer_observes_the_rust_creature_catalog() {
    use uqm_rust::planet_side::creatures::{CreatureCatalog, CreatureKind};
    for creature_type in 0..=u8::MAX {
        let expected = CreatureKind::new(creature_type)
            .map(|kind| i32::from(CreatureCatalog::raw_lifedata(kind).1 & 0x0f))
            .unwrap_or(-1);
        let observed = uqm_debug_creature_bio_value(creature_type);
        assert_eq!(observed, expected, "creature type {creature_type}");
    }
}

use uqm_rust::engine::StoryKey;

/// Starting another game must not inherit the previous game's story flags.
///
/// Exercise the World reset used by the live new-game and melee paths, including
/// the state API's typed flag mapping. A second game must find Luna's moon base
/// intact and must not start with it already aboard.
#[test]
fn a_second_new_game_starts_from_zero_state() {
    let moonbase = StoryKey::MOONBASE_DESTROYED;
    let on_ship = StoryKey::MOONBASE_ON_SHIP;

    let mut world = uqm_rust::engine::World::new();
    world.reset_game_data();
    world.set_story_state(moonbase, 1);
    world.set_story_state(on_ship, 1);
    assert_eq!(
        world.story_state(moonbase),
        1,
        "precondition: the first game recorded the moon base as taken"
    );

    world.reset_game_data();

    assert_eq!(
        world.story_state(moonbase),
        0,
        "a new game must find Luna's moon base intact"
    );
    assert_eq!(
        world.story_state(on_ship),
        0,
        "a new game must not start with the moon base already aboard"
    );
}
