//! Strict linked-test fixture for DEBUG-only C consumers of Rust providers.

use std::os::raw::c_int;

#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
unsafe extern "C" {
    fn uqm_debug_creature_bio_value(creature_type: u8) -> c_int;
}

#[test]
fn debug_c_consumer_observes_the_canonical_rust_creature_catalog() {
    for creature_type in 0..27 {
        let expected = uqm_rust::planet_side::creatures::rust_creature_bio_value(creature_type);
        let observed = unsafe { uqm_debug_creature_bio_value(creature_type) };
        assert_eq!(observed, expected, "creature type {creature_type}");
    }
}

#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
unsafe extern "C" {
    /// The real per-game initialiser the game runs when starting a new game.
    fn InitGlobData();
}

unsafe extern "C" {
    fn rust_get_game_state(key: *const std::os::raw::c_char) -> u8;
    fn rust_set_game_state(key: *const std::os::raw::c_char, value: u8);
}

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

    unsafe {
        InitGlobData();
        rust_set_game_state(moonbase.as_ptr(), 1);
        rust_set_game_state(on_ship.as_ptr(), 1);
        assert_eq!(
            rust_get_game_state(moonbase.as_ptr()),
            1,
            "precondition: the first game recorded the moon base as taken"
        );

        InitGlobData();

        assert_eq!(
            rust_get_game_state(moonbase.as_ptr()),
            0,
            "a new game must find Luna's moon base intact"
        );
        assert_eq!(
            rust_get_game_state(on_ship.as_ptr()),
            0,
            "a new game must not start with the moon base already aboard"
        );
    }
}
