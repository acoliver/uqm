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
