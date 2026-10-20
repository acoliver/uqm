//! Transitional declarations, distinct from consumers of a Cargo feature.

use std::collections::{BTreeMap, BTreeSet};

use super::shrink::{self, PathVerdict};

pub(super) fn is_definition<'a>(
    path: &str,
    line: &str,
    flags: impl Iterator<Item = &'a str>,
) -> bool {
    let key = line
        .split_once('=')
        .map(|(key, _)| key.trim().trim_matches(['\'', '"']));
    flags.into_iter().any(|flag| {
        if flag == flag.to_ascii_uppercase() {
            // Keep the existing conservative native macro projection. Cargo
            // features, unlike preprocessor flags, are declared in a manifest.
            line.contains(flag)
        } else {
            path.ends_with("/Cargo.toml") && key == Some(flag)
        }
    })
}

pub(super) fn verdicts(
    base: &BTreeMap<String, Vec<String>>,
    head: &BTreeMap<String, Vec<String>>,
    mut diff: impl FnMut(&[String], &[String]) -> Result<String, String>,
) -> Result<Vec<PathVerdict>, String> {
    base.keys()
        .chain(head.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| base.get(*path) != head.get(*path))
        .map(|path| {
            shrink::path_verdict(
                path,
                base.contains_key(path),
                head.contains_key(path),
                || diff(&base[path], &head[path]),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::shrink::Verdict;
    use super::*;

    fn verdicts(
        base: &BTreeMap<String, Vec<String>>,
        head: &BTreeMap<String, Vec<String>>,
    ) -> Result<Vec<PathVerdict>, String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let authority = super::super::authority::load_authority(root).unwrap();
        super::verdicts(base, head, |base, head| {
            super::super::delta::projection_diff(root, &authority, base, head)
        })
    }

    fn projection(path: &str, lines: &[&str]) -> BTreeMap<String, Vec<String>> {
        BTreeMap::from([(
            path.into(),
            lines.iter().map(|line| line.to_string()).collect(),
        )])
    }

    #[test]
    fn cargo_feature_declarations_are_measured_but_consumers_are_not_declarations() {
        for line in [
            "linked_c_archive = []",
            "'linked_c_archive' = []",
            "\"linked_c_archive\" = []",
        ] {
            assert!(is_definition(
                "rust/Cargo.toml",
                line,
                ["linked_c_archive"].into_iter()
            ));
        }
        for (path, line) in [
            (
                "rust/Cargo.toml",
                "required-features = [\"linked_c_archive\"]",
            ),
            ("rust/src/new.rs", "#[cfg(feature = \"linked_c_archive\")]"),
            (
                "rust/src/new.rs",
                "// linked_c_archive is selected by the caller",
            ),
            (
                "rust/harness/test.sh",
                "cargo test --features linked_c_archive",
            ),
        ] {
            assert!(!is_definition(path, line, ["linked_c_archive"].into_iter()));
        }
        assert!(is_definition(
            "sc2/src/config.h",
            "#define USE_RUST_GFX 1",
            ["USE_RUST_GFX"].into_iter()
        ));
    }

    #[test]
    fn definitions_cannot_be_added_or_moved_to_a_new_path() {
        let old = projection("rust/Cargo.toml", &["linked_c_archive = []"]);
        let empty = BTreeMap::new();
        assert_eq!(verdicts(&empty, &old).unwrap()[0].verdict, Verdict::Fail);
        assert_eq!(verdicts(&old, &empty).unwrap()[0].verdict, Verdict::Pass);
        let moved = projection("rust/new/Cargo.toml", &["linked_c_archive = []"]);
        let moved_verdicts = verdicts(&old, &moved).unwrap();
        assert!(moved_verdicts
            .iter()
            .any(|entry| entry.verdict == Verdict::Fail));
    }

    #[test]
    fn projected_hunks_cannot_borrow_removed_tokens_across_unchanged_definitions() {
        let base = projection(
            "sc2/src/config.h",
            &[
                "#define USE_RUST_A 2",
                "#define USE_RUST_B 1",
                "#define USE_RUST_C 1",
            ],
        );
        let head = projection(
            "sc2/src/config.h",
            &["#define USE_RUST_B 1", "#define USE_RUST_C 2"],
        );
        assert_eq!(verdicts(&base, &head).unwrap()[0].verdict, Verdict::Fail);
    }

    #[test]
    fn definition_edits_still_obey_the_token_ratchet() {
        let base = projection(
            "sc2/src/config.h",
            &["#define USE_RUST_GFX 1", "#define USE_RUST_AUDIO 1"],
        );
        let removed = projection("sc2/src/config.h", &["#define USE_RUST_GFX 1"]);
        let changed = projection("sc2/src/config.h", &["#define USE_RUST_GFX 2"]);
        assert!(verdicts(&base, &base).unwrap().is_empty());
        assert_eq!(verdicts(&base, &removed).unwrap()[0].verdict, Verdict::Pass);
        assert_eq!(verdicts(&base, &changed).unwrap()[0].verdict, Verdict::Fail);
    }
}
