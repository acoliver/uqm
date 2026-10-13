//! Randomised properties (fixture spec section H) over raw bytes, including
//! the high half of the byte range.

use proptest::collection::vec;
use proptest::prelude::*;

use super::components::*;

fn path_bytes() -> impl Strategy<Value = Vec<u8>> {
    // Skewed to produce separators and dots, but any byte can appear.
    vec(
        prop_oneof![3 => Just(b'/'), 2 => Just(b'.'), 2 => Just(b'a'), 3 => any::<u8>()],
        0..24,
    )
}

/// The reference for what the C loop visits: split on '/', and drop the empty
/// tail a trailing separator leaves (the loop stops at `start == end`).
fn split_model(path: &[u8]) -> Vec<&[u8]> {
    if path.is_empty() {
        return Vec::new();
    }
    let mut parts: Vec<&[u8]> = path.split(|&b| b == b'/').collect();
    if path.ends_with(b"/") {
        parts.pop();
    }
    parts
}

proptest! {
    // H1
    #[test]
    fn walking_splits_on_separators(path in path_bytes()) {
        let walked: Vec<&[u8]> = components(&path).collect();
        prop_assert_eq!(walked, split_model(&path));
    }

    // H2: from the last component, previous visits the others in reverse.
    #[test]
    fn previous_retraces_the_walk(path in path_bytes()) {
        let mut visited = vec![last_component(&path)];
        while visited.last().unwrap().start > 0 {
            let start = visited.last().unwrap().start;
            visited.push(previous_component(&path, start));
        }
        let mut backwards: Vec<&[u8]> =
            visited.iter().map(|s| &path[s.start..s.end]).collect();
        backwards.reverse();
        // Unlike the forward walk, the backward one ends at the start, so the
        // empty tail after a trailing separator is its first stop.
        let forwards: Vec<&[u8]> = path.split(|&b| b == b'/').collect();
        prop_assert_eq!(backwards, forwards);
        prop_assert_eq!(visited.last().unwrap().start, 0);
    }

    // H3
    #[test]
    fn validity_is_a_property_of_the_components(path in path_bytes()) {
        let expected = split_model(&path)
            .iter()
            .all(|name| !name.is_empty() && *name != b"." && *name != b"..");
        prop_assert_eq!(valid_path_name(&path), expected);
    }

    // H4
    #[test]
    fn decompose_then_compose_collapses_separator_runs(path in path_bytes()) {
        prop_assume!(!path.is_empty());
        let decomposed = decompose(&path).unwrap();
        prop_assert!(decomposed.names.iter().all(|n| !n.is_empty() && !n.contains(&b'/')));
        if !decomposed.names.is_empty() {
            let composed = compose(&decomposed.names, decomposed.absolute);
            let again = decompose(&composed).unwrap();
            prop_assert_eq!(&again, &decomposed);
            // Collapsing is the only change.
            let mut collapsed = Vec::new();
            for &b in &path {
                if b != b'/' || collapsed.last() != Some(&b'/') {
                    collapsed.push(b);
                }
            }
            if collapsed.len() > 1 && collapsed.ends_with(b"/") {
                collapsed.pop();
            }
            prop_assert_eq!(composed, collapsed);
        }
    }

    // H5
    #[test]
    fn compose_length_is_what_is_written(
        names in vec(vec(any::<u8>().prop_filter("no separator", |b| *b != b'/'), 0..6), 1..6),
        absolute in any::<bool>(),
    ) {
        let refs: Vec<&[u8]> = names.iter().map(Vec::as_slice).collect();
        let composed = compose(&refs, absolute);
        let separators = refs.len() - 1;
        let name_bytes: usize = refs.iter().map(|n| n.len()).sum();
        prop_assert_eq!(composed.len(), usize::from(absolute) + name_bytes + separators);
    }

    #[test]
    fn join_is_lossless(first in path_bytes(), second in path_bytes()) {
        prop_assume!(!second.starts_with(b"/"));
        let joined = join_paths(&first, &second);
        if first.is_empty() {
            prop_assert_eq!(joined, second);
        } else {
            let kept = first.strip_suffix(b"/").unwrap_or(&first);
            prop_assert!(joined.starts_with(kept));
            prop_assert!(joined.ends_with(&second));
            prop_assert_eq!(joined.len(), kept.len() + 1 + second.len());
        }
    }
}

/// The body `valid_mount_point` had before it called `valid_path_name`.
fn old_valid_mount_point(bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix(b"/").unwrap_or(bytes);
    let bytes = match bytes.strip_suffix(b"/") {
        Some(rest) if !rest.is_empty() => rest,
        _ => bytes,
    };
    bytes.is_empty()
        || bytes
            .split(|&b| b == b'/')
            .all(|name| !name.is_empty() && name != b"." && name != b"..")
}

// H6
#[test]
fn mount_point_check_is_unchanged_by_reusing_valid_path_name() {
    let alphabet = *b"/.a";
    let mut checked = 0;
    for length in 0..=6u32 {
        for code in 0..3usize.pow(length) {
            let bytes: Vec<u8> = (0..length)
                .map(|at| alphabet[code / 3usize.pow(at) % 3])
                .collect();
            let stripped = bytes.strip_prefix(b"/").unwrap_or(&bytes);
            assert_eq!(
                valid_path_name(stripped),
                old_valid_mount_point(&bytes),
                "{bytes:?}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 1093);
}
