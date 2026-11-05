//! Linked fixtures for debug consumers and game-state initialization.

#[cfg(feature = "debug-process")]
use uqm_rust::front_end::uqmdebug::uqm_debug_creature_bio_value;

#[test]
#[cfg(feature = "debug-process")]
fn debug_consumer_observes_the_rust_creature_catalog() {
    for creature_type in 0..27 {
        let expected = uqm_rust::planet_side::creatures::rust_creature_bio_value(creature_type);
        let observed = uqm_debug_creature_bio_value(creature_type);
        assert_eq!(observed, expected, "creature type {creature_type}");
    }
}

use uqm_rust::state::ffi::rust_get_game_state;
use uqm_rust::state::ffi::rust_set_game_state;

/// Starting another game must not inherit the previous game's story flags.
///
/// Exercise the World reset used by the live new-game and melee paths, including
/// the state API's named flag mapping. A second game must find Luna's moon base
/// intact and must not start with it already aboard.
#[test]
fn a_second_new_game_starts_from_zero_state() {
    let moonbase = c"MOONBASE_DESTROYED";
    let on_ship = c"MOONBASE_ON_SHIP";

    let mut world = uqm_rust::engine::World::new();
    world.reset_game_data();
    // SAFETY: the keys are static NUL-terminated strings, and this fixture owns World.
    unsafe {
        rust_set_game_state(&mut world, moonbase.as_ptr(), 1);
        rust_set_game_state(&mut world, on_ship.as_ptr(), 1);
        assert_eq!(
            rust_get_game_state(&world, moonbase.as_ptr()),
            1,
            "precondition: the first game recorded the moon base as taken"
        );

        world.reset_game_data();

        assert_eq!(
            rust_get_game_state(&world, moonbase.as_ptr()),
            0,
            "a new game must find Luna's moon base intact"
        );
        assert_eq!(
            rust_get_game_state(&world, on_ship.as_ptr()),
            0,
            "a new game must not start with the moon base already aboard"
        );
    }
}
