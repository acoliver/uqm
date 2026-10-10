//! The mount choreography of `sc2/src/options.c`, driven through the public
//! UIO functions, plus builders for the on-disk fixture worlds.

use std::io::Write;
use std::path::Path;

use tempfile::TempDir;

use super::declared::{
    Files, ZipSpec, MATCH_PREFIX, MATCH_REGEX, MOUNT_ABOVE, MOUNT_BELOW, MOUNT_RDONLY, MOUNT_TOP,
    RMP_PATTERN, ZIP_PATTERN,
};
use super::res;
use super::uio_api::{Dir, Mount, Repo};

pub(super) fn write_files(root: &Path, files: &[(&str, &[u8])]) {
    for (name, bytes) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("fixture file has a parent")).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}

pub(super) fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    std::fs::create_dir_all(path.parent().expect("zip has a parent")).unwrap();
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in entries {
        zip.start_file(*name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

/// A content directory on disk built from declared tables.
pub(super) fn build_world(loose: Files, zips: &[ZipSpec]) -> TempDir {
    let root = TempDir::new().unwrap();
    write_files(root.path(), loose);
    for spec in zips {
        write_zip(&root.path().join(spec.path), spec.entries);
    }
    root
}

/// A started game: one repository with the content directory mounted the way
/// `mountContentDir` does it, and `contentDir` open on "/".
pub(super) struct Game {
    pub(super) content: Dir,
    pub(super) content_mount: Mount,
    pub(super) repo: Repo,
}

impl Game {
    /// `mountContentDir`: STDIO at "/" on TOP, read only, then every zip of
    /// `/packages` BELOW it at "/".
    pub(super) fn start(content_host: &Path) -> Game {
        let repo = Repo::open();
        let content_mount =
            repo.mount_stdio("/", content_host, MOUNT_TOP | MOUNT_RDONLY, Mount::NONE);
        assert!(!content_mount.is_null(), "content mount failed");
        let content = repo.open_dir("/");
        let game = Game {
            content,
            content_mount,
            repo,
        };
        let packages = game.repo.open_dir("/packages");
        game.mount_dir_zips(&packages, "/", MOUNT_BELOW, content_mount);
        game
    }

    /// `mountDirZips`: every `.zip`/`.uqm` (any case of the letters) in
    /// `dir`, mounted in list order.
    pub(super) fn mount_dir_zips(
        &self,
        dir: &Dir,
        point: &str,
        relative_flags: i32,
        relative: Mount,
    ) {
        for name in dir.list(ZIP_PATTERN, MATCH_REGEX) {
            let mount =
                self.repo
                    .mount_zip(point, dir, &name, relative_flags | MOUNT_RDONLY, relative);
            assert!(!mount.is_null(), "could not mount {name}");
        }
    }

    /// `mountAddonDir` with no `--addon` directory. Returns the addon names it
    /// logged ("available addon packs").
    pub(super) fn mount_addon_dir(&self) -> Vec<String> {
        let addons = self.content.open_relative("addons");
        self.mount_dir_zips(&addons, "addons", MOUNT_BELOW, self.content_mount);
        let mut names = Vec::new();
        for entry in addons.list("", MATCH_PREFIX) {
            if entry.starts_with('.') || !addons.is_dir(&entry) {
                continue;
            }
            let dir = addons.open_relative(&entry);
            self.mount_dir_zips(
                &dir,
                &format!("addons/{entry}"),
                MOUNT_BELOW,
                self.content_mount,
            );
            names.push(entry);
        }
        names
    }

    /// `prepareShadowAddons`: each addon's `shadow-content` zips and loose
    /// files go ABOVE the content mount at "/".
    pub(super) fn prepare_shadow_addons(&self, addons: &[&str]) {
        let addons_dir = self.content.open_relative("addons");
        for name in addons {
            let addon = addons_dir.open_relative(name);
            if !addon.is_dir("shadow-content") {
                continue;
            }
            let shadow = addon.open_relative("shadow-content");
            self.mount_dir_zips(&shadow, "/", MOUNT_ABOVE, self.content_mount);
            let mount =
                self.repo
                    .transplant("/", &shadow, MOUNT_RDONLY | MOUNT_ABOVE, self.content_mount);
            assert!(
                !mount.is_null(),
                "could not transplant shadow content of {name}"
            );
        }
    }

    /// `loadIndices`: every `.rmp` of `dir`, in list order. Returns how many.
    pub(super) fn load_indices(&self, dir: &Dir) -> usize {
        let names = dir.list(RMP_PATTERN, MATCH_REGEX);
        for name in &names {
            res::load(dir, name, None);
        }
        names.len()
    }

    /// `loadAddon`.
    pub(super) fn load_addon(&self, addon: &str) -> bool {
        let addons_dir = self.content.open_relative("addons");
        let dir = addons_dir.open_relative(addon);
        self.load_indices(&dir) > 0
    }

    /// `prepareAddons`: stops at the first addon that loads nothing.
    pub(super) fn prepare_addons(&self, addons: &[&str]) {
        for addon in addons {
            if !self.load_addon(addon) {
                break;
            }
        }
    }
}
