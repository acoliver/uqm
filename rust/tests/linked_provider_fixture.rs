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

/// The C declaration of `baseContentPath` generated from `options.h`.
#[allow(non_upper_case_globals, non_camel_case_types, dead_code)]
mod options_abi {
    include!(concat!(env!("OUT_DIR"), "/options_abi.rs"));
}

fn real_base_content_path_prefix(len: usize) -> Vec<u8> {
    let buffer = std::ptr::addr_of!(options_abi::baseContentPath).cast::<u8>();
    (0..len).map(|i| unsafe { *buffer.add(i) }).collect()
}

/// The real `baseContentPath` in the linked C archive bounds what the Rust
/// startup path may store, independent of the bound the crate assumes.
///
/// The C bound is the array length `options.h` declares for the object that
/// `options.c` (whose bytes are pinned here) defines. It is compared with the
/// Rust bound before anything is written, so a wrong Rust bound fails this test
/// instead of overrunning the C global.
#[test]
fn base_content_path_store_honours_the_real_c_buffer_bound() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    use sha2::{Digest, Sha256};
    use uqm_rust::mainloop::dir_prep::{
        rust_prepare_content_dir, store_base_content_path, DirPrepError, PATH_MAX,
    };

    const OPTIONS_C_SHA256: &str =
        "488444d1871e1f4cae041ad3bbc31d83ded7bba54fd18b75bdf38b243d1f4b39";
    let options_c = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sc2/src/options.c");
    let digest = Sha256::digest(std::fs::read(&options_c).unwrap());
    assert_eq!(
        format!("{digest:x}"),
        OPTIONS_C_SHA256,
        "the pinned source the C archive object is built from"
    );

    let c_bound =
        std::mem::size_of_val(unsafe { &*std::ptr::addr_of!(options_abi::baseContentPath) });
    assert_eq!(
        PATH_MAX, c_bound,
        "the Rust bound must equal the C array length"
    );
    assert_eq!(real_base_content_path_prefix(1), [0], "nothing stored yet");

    let enametoolong = |error: DirPrepError| match error {
        DirPrepError::InvalidPath { source, .. } => source.raw_os_error(),
        other => panic!("expected InvalidPath, got {other:?}"),
    };
    let path_of = |len: usize| {
        let mut bytes = vec![b'a'; len];
        bytes[0] = b'/';
        bytes
    };

    let fits = path_of(c_bound - 1);
    store_base_content_path(Path::new(std::ffi::OsStr::from_bytes(&fits))).unwrap();
    let stored = real_base_content_path_prefix(c_bound);
    assert_eq!(&stored[..c_bound - 1], &fits[..]);
    assert_eq!(stored[c_bound - 1], 0, "the NUL lands on the last byte");

    for len in [c_bound, c_bound + 1, 4096] {
        let too_long = path_of(len);
        let refused = store_base_content_path(Path::new(std::ffi::OsStr::from_bytes(&too_long)));
        assert_eq!(
            refused.map_err(enametoolong),
            Err(Some(libc::ENAMETOOLONG)),
            "length {len}"
        );
        assert_eq!(
            real_base_content_path_prefix(c_bound),
            stored,
            "length {len}"
        );

        let arg = CString::new(too_long).unwrap();
        let status =
            unsafe { rust_prepare_content_dir(arg.as_ptr(), std::ptr::null(), c"uqm".as_ptr()) };
        assert_eq!(status, -1, "length {len}");
        assert_eq!(
            real_base_content_path_prefix(c_bound),
            stored,
            "length {len}"
        );
    }

    store_base_content_path(Path::new("")).unwrap();
    assert_eq!(real_base_content_path_prefix(1), [0]);
}
