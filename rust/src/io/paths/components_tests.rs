//! The fixtures of `fixture-spec.md` sections A to E and G, hand-traced from
//! `paths.c` before the code ran.

use super::components::*;
use super::unc;

type JoinRow = (&'static [u8], &'static [u8], &'static [u8], &'static [u8]);
type ComposeRow = (Vec<&'static [u8]>, bool, &'static [u8]);

fn s(start: usize, end: usize) -> Span {
    Span::new(start, end)
}

fn walk(bytes: &[u8]) -> Vec<Span> {
    let mut spans = vec![first_component(bytes)];
    while spans.last().unwrap().start < bytes.len() {
        spans.push(next_component(bytes, *spans.last().unwrap()));
    }
    spans
}

#[test]
fn a_first_and_next() {
    assert_eq!(first_component(b""), s(0, 0)); // A1
    assert_eq!(first_component(b"abc"), s(0, 3)); // A2
    assert_eq!(first_component(b"a/b"), s(0, 1)); // A3
    assert_eq!(first_component(b"/a"), s(0, 0)); // A4
    assert_eq!(walk(b"a//b"), [s(0, 1), s(2, 2), s(3, 4), s(4, 4)]); // A5
    assert_eq!(walk(b"a/"), [s(0, 1), s(2, 2)]); // A6
    assert_eq!(walk(b"ab\xff/\x80c"), [s(0, 3), s(4, 6), s(6, 6)]); // A15
                                                                    // A16: dirEnd bounds the walk.
    assert_eq!(walk(&b"a/b"[..2]), [s(0, 1), s(2, 2)]);
}

#[test]
fn a_last_and_previous() {
    assert_eq!(last_component(b"a/b/c"), s(4, 5)); // A7
    assert_eq!(last_component(b"a/b/"), s(4, 4)); // A8
    assert_eq!(last_component(b""), s(0, 0)); // A9
    assert_eq!(last_component(b"/"), s(1, 1)); // A10
    assert_eq!(last_component(b"abc"), s(0, 3)); // A11
    let path = b"a/b/c";
    assert_eq!(previous_component(path, 4), s(2, 3)); // A12
    assert_eq!(previous_component(path, 2), s(0, 1));
    assert_eq!(previous_component(path, 0), s(0, 0));
    assert_eq!(previous_component(b"a/b/", 4), s(2, 3)); // A13
    assert_eq!(previous_component(b"//", 1), s(0, 0)); // A14
}

#[test]
#[should_panic(expected = "not at a separator")]
fn next_component_off_a_separator_is_a_caller_bug() {
    next_component(b"abc", s(0, 1));
}

#[test]
#[should_panic(expected = "past the path end")]
fn next_component_past_the_end_is_a_caller_bug() {
    next_component(b"abc", s(0, 4));
}

#[test]
fn b_valid_path_name() {
    for good in [
        &b""[..],
        b"a",
        b"a/b",
        b"a/",
        b"...",
        b".a",
        b"a.",
        b"\xff",
        b"a\0b",
    ] {
        assert!(valid_path_name(good), "{good:?}");
    }
    for bad in [
        &b"/"[..],
        b"/a",
        b"a//b",
        b"a//",
        b".",
        b"..",
        b"a/.",
        b"a/./b",
        b"a/../b",
        b"../a",
    ] {
        assert!(!valid_path_name(bad), "{bad:?}");
    }
    // The length bounds the check.
    assert!(valid_path_name(&b"a//b"[..1]));
    assert!(!valid_path_name(&b"a//b"[..3]));
    assert!(valid_path_name(&b"a/."[..2]));
}

#[test]
fn c_joins() {
    let rows: [JoinRow; 9] = [
        (b"", b"x", b"x", b"/x"),
        (b"a", b"b", b"a/b", b"/a/b"),
        (b"a/", b"b", b"a/b", b"/a/b"),
        (b"/", b"b", b"/b", b"//b"),
        (b"a//", b"b", b"a//b", b"/a//b"),
        (b"a", b"", b"a/", b"/a/"),
        (b"", b"", b"", b"/"),
        (b"/a", b"b", b"/a/b", b"//a/b"),
        (b"\xff", b"\x80", b"\xff/\x80", b"/\xff/\x80"),
    ];
    for (first, second, joined, absolute) in rows {
        assert_eq!(join_paths(first, second), joined, "{first:?} {second:?}");
        assert_eq!(
            join_paths_absolute(first, second),
            absolute,
            "{first:?} {second:?}"
        );
    }
}

#[test]
fn d_decompose() {
    assert_eq!(decompose(b""), Err(EmptyPath)); // D1
    for (path, absolute, names) in [
        (&b"/"[..], true, vec![]), // D2
        (b"///", true, vec![]),    // D3
        (b"a", false, vec![&b"a"[..]]),
        (b"a/b", false, vec![&b"a"[..], b"b"]),
        (b"/a//b/", true, vec![&b"a"[..], b"b"]),
        (b"\xff/\x80", false, vec![&b"\xff"[..], b"\x80"]),
    ] {
        assert_eq!(
            decompose(path),
            Ok(Decomposed { absolute, names }),
            "{path:?}"
        );
    }
}

#[test]
fn e_compose() {
    let rows: [ComposeRow; 5] = [
        (vec![b"a"], false, b"a"),
        (vec![b"a", b"b"], false, b"a/b"),
        (vec![b"a", b"b"], true, b"/a/b"),
        (vec![b"a", b"", b"b"], true, b"/a//b"),
        (vec![b"\xff", b"\x80"], false, b"\xff/\x80"),
    ];
    for (names, absolute, path) in rows {
        assert_eq!(compose(&names, absolute), path);
    }
    // Round trip of D6.
    let decomposed = decompose(b"/a//b/").unwrap();
    assert_eq!(compose(&decomposed.names, decomposed.absolute), b"/a/b");
}

#[test]
#[should_panic(expected = "at least one component")]
fn compose_of_nothing_is_a_caller_bug() {
    compose(&[], true);
}

#[test]
fn g_unc_skip() {
    assert_eq!(unc::skip(b""), 0);
    assert_eq!(unc::skip(br"\\srv\share\x"), 11);
    assert_eq!(unc::skip(br"\\srv/share/x"), 11);
    assert_eq!(unc::skip(br"\\srv"), 0);
    assert_eq!(unc::skip(br"\\srv\"), 6);
    assert_eq!(unc::skip(br"\\\share"), 8);
    assert_eq!(unc::skip(b"/srv/share"), 0);
    assert_eq!(unc::skip(br"\x"), 0);
}

#[test]
fn g_unc_get() {
    let found = unc::get(br"\\srv/share/x").unwrap();
    assert_eq!(
        (found.name.as_slice(), found.consumed),
        (&br"\\srv\share"[..], 12)
    );
    assert_eq!(found.name.len(), 11);
    let found = unc::get(br"\\srv/share//x").unwrap();
    assert_eq!(
        (found.name.as_slice(), found.consumed),
        (&br"\\srv\share"[..], 13)
    );
    assert_eq!(unc::get(br"\\srv"), None);
    assert_eq!(unc::get(br"\\/share"), None);
    assert_eq!(unc::get(b"/a"), None);
}
