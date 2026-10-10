//! Which content a gameplay proof retains, and the view the game runs on.
//!
//! The game needs decoded files, but a content tree that holds both a package
//! and its decoded files carries the same bytes twice and is far larger, in
//! member count, than the evidence limits allow. The proof therefore retains
//! the package and only the loose files that add to it or differ from it, and
//! derives the decoded view from those at run time. The view is removed after
//! the run, so the retained tree is exactly what the identity snapshot names.

use super::content_materialize::{
    materialize_content_package, MaterializedContent, PackageIdentity,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

/// Root-level file the game uses to recognise a content directory.
const VERSION_FILE: &str = "version";
const PACKAGES_DIR: &str = "packages/";
const PACKAGE_EXTENSION: &str = ".uqm";
/// Optional content packs the proof invocation never selects: it passes no
/// `--addon`, and starts from an empty config directory.
const UNSELECTED_DIR: &str = "addons/";

/// One regular file of a content tree: its path relative to the root, hash and size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentEntry {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

/// How a source content tree divides into retained inputs and the rest.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// Packages, the version marker, and loose files the packages do not already provide.
    pub retained: Vec<ContentEntry>,
    /// Loose files byte-identical to a package member of the same path.
    pub duplicated_by_package: Vec<ContentEntry>,
    /// Files of packs the invocation does not select.
    pub unselected: Vec<ContentEntry>,
}

fn is_package(path: &str) -> bool {
    path.strip_prefix(PACKAGES_DIR)
        .is_some_and(|name| !name.contains('/') && name.ends_with(PACKAGE_EXTENSION))
}

/// Hash every file member of a package, keyed by its normalised path.
pub fn package_members(package: &Path) -> Result<BTreeMap<String, ContentEntry>, String> {
    let file = fs::File::open(package)
        .map_err(|error| format!("open content package {}: {error}", package.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("parse content package {}: {error}", package.display()))?;
    let mut members = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("read content package entry {index}: {error}"))?;
        if entry.is_dir() {
            continue;
        }
        let path = entry
            .enclosed_name()
            .and_then(|name| name.to_str().map(str::to_owned))
            .ok_or_else(|| format!("content package entry {index} has an unsafe path"))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        let mut bytes = 0_u64;
        loop {
            let read = entry
                .read(&mut buffer)
                .map_err(|error| format!("hash content package entry {path}: {error}"))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            bytes += read as u64;
        }
        let member = ContentEntry {
            path: path.clone(),
            sha256: format!("{:x}", hasher.finalize()),
            bytes,
        };
        if members.insert(path.clone(), member).is_some() {
            return Err(format!("content package repeats the path {path}"));
        }
    }
    Ok(members)
}

/// Divide `entries` (a whole source tree) into retained, duplicated and unselected files.
///
/// `members` are the file members of every package in the tree. A loose file is a
/// duplicate only when its hash and length equal the member of the same path, so
/// a loose file that overrides a member is retained.
pub fn select_retained(
    entries: &[ContentEntry],
    members: &BTreeMap<String, ContentEntry>,
) -> Selection {
    let mut selection = Selection::default();
    for entry in entries {
        if entry.path.starts_with(UNSELECTED_DIR) {
            selection.unselected.push(entry.clone());
        } else if entry.path == VERSION_FILE || entry.path.starts_with(PACKAGES_DIR) {
            selection.retained.push(entry.clone());
        } else if members
            .get(&entry.path)
            .is_some_and(|member| member.sha256 == entry.sha256 && member.bytes == entry.bytes)
        {
            selection.duplicated_by_package.push(entry.clone());
        } else {
            selection.retained.push(entry.clone());
        }
    }
    selection
}

/// Hash every package listed in `retained` under `root` and divide `entries`.
pub fn select_from_tree(root: &Path, entries: &[ContentEntry]) -> Result<Selection, String> {
    let mut members = BTreeMap::new();
    for entry in entries.iter().filter(|entry| is_package(&entry.path)) {
        for (path, member) in package_members(&root.join(&entry.path))? {
            if members.insert(path.clone(), member).is_some() {
                return Err(format!("two content packages provide the path {path}"));
            }
        }
    }
    Ok(select_retained(entries, &members))
}

/// Whether a retained tree holds a package, so the game's view must be derived.
pub fn needs_expansion(retained: &[ContentEntry]) -> bool {
    retained.iter().any(|entry| is_package(&entry.path))
}

/// The decoded view derived from retained packages, removed on drop.
#[derive(Default)]
pub struct ContentView {
    materialized: Vec<MaterializedContent>,
}

impl ContentView {
    /// Expand every retained package into `root` without replacing retained loose files.
    ///
    /// A retained tree with no package expands nothing.
    pub fn expand(
        root: &Path,
        retained: &[ContentEntry],
        budget_bytes: u64,
    ) -> Result<ContentView, String> {
        let overrides: BTreeSet<PathBuf> = retained
            .iter()
            .filter(|entry| entry.path != VERSION_FILE && !entry.path.starts_with(PACKAGES_DIR))
            .map(|entry| PathBuf::from(&entry.path))
            .collect();
        let mut view = ContentView::default();
        for package in retained.iter().filter(|entry| is_package(&entry.path)) {
            let expanded = materialize_content_package(
                &root.join(&package.path),
                root,
                budget_bytes,
                Some(PackageIdentity {
                    byte_length: package.bytes,
                    sha256: &package.sha256,
                }),
                &overrides,
            )?;
            view.materialized.push(expanded);
        }
        Ok(view)
    }

    /// Whether any package was expanded.
    pub fn is_expanded(&self) -> bool {
        !self.materialized.is_empty()
    }

    /// Remove everything the expansion created, leaving the retained files.
    pub fn remove(&mut self) -> Result<(), String> {
        while let Some(mut expanded) = self.materialized.pop() {
            expanded.cleanup()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use zip::write::SimpleFileOptions;

    fn write_package(path: &Path, members: &[(&str, &[u8])]) {
        let mut archive = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, bytes) in members {
            archive
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            archive.write_all(bytes).unwrap();
        }
        archive.finish().unwrap();
    }

    fn entry_of(root: &Path, path: &str) -> ContentEntry {
        let bytes = fs::read(root.join(path)).unwrap();
        ContentEntry {
            path: path.to_owned(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes: bytes.len() as u64,
        }
    }

    fn lay_out(root: &Path, files: &[(&str, &[u8])]) {
        for (path, bytes) in files {
            let target = root.join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, bytes).unwrap();
        }
    }

    fn sorted_entries(root: &Path, paths: &[&str]) -> Vec<ContentEntry> {
        let mut entries: Vec<_> = paths.iter().map(|path| entry_of(root, path)).collect();
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        entries
    }

    #[test]
    fn identical_loose_files_are_duplicates_and_differing_ones_are_retained() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("packages")).unwrap();
        write_package(
            &root.path().join("packages/c.uqm"),
            &[("a.txt", b"same"), ("base/b.txt", b"packaged")],
        );
        lay_out(
            root.path(),
            &[
                ("version", b"0.8.0\n"),
                ("a.txt", b"same"),
                ("base/b.txt", b"loose override"),
                ("extra.txt", b"only loose"),
                ("addons/pack/x.txt", b"unselected"),
            ],
        );
        let entries = sorted_entries(
            root.path(),
            &[
                "a.txt",
                "addons/pack/x.txt",
                "base/b.txt",
                "extra.txt",
                "packages/c.uqm",
                "version",
            ],
        );

        let selection = select_from_tree(root.path(), &entries).unwrap();

        let paths = |entries: &[ContentEntry]| -> Vec<String> {
            entries.iter().map(|entry| entry.path.clone()).collect()
        };
        assert_eq!(paths(&selection.duplicated_by_package), ["a.txt"]);
        assert_eq!(paths(&selection.unselected), ["addons/pack/x.txt"]);
        assert_eq!(
            paths(&selection.retained),
            ["base/b.txt", "extra.txt", "packages/c.uqm", "version"]
        );
    }

    #[test]
    fn expansion_adds_package_files_keeps_overrides_and_removes_cleanly() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("packages")).unwrap();
        write_package(
            &root.path().join("packages/c.uqm"),
            &[
                ("a.txt", b"from package"),
                ("base/b.txt", b"packaged"),
                ("base/ui/c.txt", b"deep"),
            ],
        );
        lay_out(
            root.path(),
            &[("version", b"0.8.0\n"), ("base/b.txt", b"loose override")],
        );
        let retained = sorted_entries(root.path(), &["base/b.txt", "packages/c.uqm", "version"]);

        let mut view = ContentView::expand(root.path(), &retained, 1024).unwrap();

        assert!(view.is_expanded());
        assert_eq!(
            fs::read(root.path().join("a.txt")).unwrap(),
            b"from package"
        );
        assert_eq!(
            fs::read(root.path().join("base/ui/c.txt")).unwrap(),
            b"deep"
        );
        assert_eq!(
            fs::read(root.path().join("base/b.txt")).unwrap(),
            b"loose override"
        );

        view.remove().unwrap();

        assert!(!root.path().join("a.txt").exists());
        assert!(!root.path().join("base/ui").exists());
        assert_eq!(
            fs::read(root.path().join("base/b.txt")).unwrap(),
            b"loose override"
        );
        assert!(root.path().join("packages/c.uqm").is_file());
    }

    #[test]
    fn a_package_swapped_after_retention_is_refused() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("packages")).unwrap();
        write_package(&root.path().join("packages/c.uqm"), &[("a.txt", b"one")]);
        lay_out(root.path(), &[("version", b"0.8.0\n")]);
        let retained = sorted_entries(root.path(), &["packages/c.uqm", "version"]);
        write_package(&root.path().join("packages/c.uqm"), &[("a.txt", b"two")]);

        assert!(ContentView::expand(root.path(), &retained, 1024).is_err());
        assert!(!root.path().join("a.txt").exists());
    }

    #[test]
    fn expansion_over_budget_leaves_nothing_behind() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("packages")).unwrap();
        write_package(
            &root.path().join("packages/c.uqm"),
            &[("a.txt", &[7_u8; 4096])],
        );
        lay_out(root.path(), &[("version", b"0.8.0\n")]);
        let retained = sorted_entries(root.path(), &["packages/c.uqm", "version"]);

        assert!(ContentView::expand(root.path(), &retained, 100).is_err());
        assert!(!root.path().join("a.txt").exists());
    }

    #[test]
    fn a_tree_without_packages_expands_nothing() {
        let root = tempfile::tempdir().unwrap();
        lay_out(root.path(), &[("version", b"0.8.0\n"), ("data", b"loose")]);
        let retained = sorted_entries(root.path(), &["data", "version"]);

        let view = ContentView::expand(root.path(), &retained, 1024).unwrap();

        assert!(!view.is_expanded());
    }

    #[test]
    fn two_packages_providing_one_path_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("packages")).unwrap();
        write_package(&root.path().join("packages/one.uqm"), &[("a.txt", b"1")]);
        write_package(&root.path().join("packages/two.uqm"), &[("a.txt", b"2")]);
        let entries = sorted_entries(root.path(), &["packages/one.uqm", "packages/two.uqm"]);

        assert!(select_from_tree(root.path(), &entries).is_err());
    }
}
