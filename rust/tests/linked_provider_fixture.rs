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

use uqm_rust::navigation::globals::InitGlobData;

use uqm_rust::state::ffi::rust_get_game_state;
use uqm_rust::state::ffi::rust_set_game_state;

/// Starting another game must not inherit the previous game's story flags.
///
/// This drives the production C entry point rather than the Rust function it
/// calls, so it covers the wiring that actually broke: `InitGlobData` zeroes its
/// own shadow array and relies on that call to reset the authoritative Rust
/// state. When it did not, a second New Game believed the Ur-Quan probe had
/// already been met and Luna's moon base already taken.
#[test]
fn a_second_new_game_starts_from_zero_state() {
    let moonbase = c"MOONBASE_DESTROYED";
    let on_ship = c"MOONBASE_ON_SHIP";

    let mut game = uqm_rust::engine::Game::new();
    // SAFETY: this fixture owns the synchronous lifecycle through both initializations.
    unsafe {
        uqm_rust::app::bridge::with_game(&mut game, || {
            crate::engine::World::current().write(|world| InitGlobData(world));
            crate::engine::World::current()
                .write(|world| rust_set_game_state(world, moonbase.as_ptr(), 1));
            crate::engine::World::current()
                .write(|world| rust_set_game_state(world, on_ship.as_ptr(), 1));
            assert_eq!(
                crate::engine::World::current()
                    .read(|world| rust_get_game_state(world, moonbase.as_ptr())),
                1,
                "precondition: the first game recorded the moon base as taken"
            );

            crate::engine::World::current().write(|world| InitGlobData(world));

            assert_eq!(
                crate::engine::World::current()
                    .read(|world| rust_get_game_state(world, moonbase.as_ptr())),
                0,
                "a new game must find Luna's moon base intact"
            );
            assert_eq!(
                crate::engine::World::current()
                    .read(|world| rust_get_game_state(world, on_ship.as_ptr())),
                0,
                "a new game must not start with the moon base already aboard"
            );
        })
    }
}
