//! Lifecycle tests for the directory preparation steps, run against a real
//! mount table and real temporary directories.

use std::io::Write;
use std::path::Path;

use serial_test::serial;
use tempfile::TempDir;

use super::*;
use crate::engine::host::Files;
use crate::io::Vfs;

const ENV_KEYS: [&str; 4] = ["HOME", "UQM_CONFIG_DIR", "UQM_SAVE_DIR", "UQM_MELEE_DIR"];

/// Owns a fresh file system and a private HOME, and restores the process
/// environment when dropped.
struct Fixture {
    home: TempDir,
    vfs: Vfs,
    configuration: uqm_engine::configuration::Configuration,
    saved_env: Vec<(&'static str, Option<String>)>,
}

impl Fixture {
    fn new() -> Self {
        let saved_env = ENV_KEYS
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        let home = TempDir::new().unwrap();
        std::env::set_var("HOME", home.path());
        for key in &ENV_KEYS[1..] {
            std::env::remove_var(key);
        }
        Fixture {
            home,
            vfs: Vfs::new(),
            saved_env,
            configuration: Default::default(),
        }
    }

    fn home(&self) -> &Path {
        self.home.path()
    }

    fn config_path(&self) -> std::path::PathBuf {
        self.home().join("cfg")
    }

    fn prepare_config(&self) {
        let path = self.config_path();
        prepare_config_dir(&self.vfs, Some(path.to_str().unwrap())).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for (key, value) in &self.saved_env {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn read_through_files(dir: Option<Dir>, name: &str) -> Option<Vec<u8>> {
    dir?.read(std::path::Path::new(name)).ok()
}

fn list_names(dir: Dir, pattern: &str, kind: Match) -> Vec<String> {
    let mut names = dir.list_matching(Path::new(""), pattern, kind);
    names.sort();
    names
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in entries {
        zip.start_file(*name, options).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
}

/// Lays out `content/` with a version file, a packages zip, a top-level addon
/// zip and an addon subdirectory carrying its own zip and shadow content.
fn build_content_tree(root: &Path) -> std::path::PathBuf {
    let content = root.join("content");
    std::fs::create_dir_all(content.join("packages")).unwrap();
    std::fs::create_dir_all(content.join("addons/alpha/shadow-content")).unwrap();
    std::fs::write(content.join("version"), b"0.8.0\n").unwrap();
    write_zip(
        &content.join("packages/base.zip"),
        &[("from_package.txt", b"package data")],
    );
    write_zip(
        &content.join("addons/top.uqm"),
        &[("from_top_addon.txt", b"top addon data")],
    );
    write_zip(
        &content.join("addons/alpha/alpha.ZIP"),
        &[("from_alpha_addon.txt", b"alpha addon data")],
    );
    std::fs::write(
        content.join("addons/alpha/shadow-content/shadowed.txt"),
        b"shadow data",
    )
    .unwrap();
    content
}

#[test]
#[serial]
fn config_dir_is_created_exported_and_mounted() {
    let fx = Fixture::new();
    let path = fx.config_path();
    assert!(!path.exists());

    prepare_config_dir(&fx.vfs, Some(path.to_str().unwrap())).unwrap();

    assert!(path.is_dir());
    assert_eq!(
        std::env::var("UQM_CONFIG_DIR").unwrap(),
        path.to_str().unwrap()
    );
    assert!(fx.vfs.config_dir().is_some());
    std::fs::write(path.join("marker.txt"), b"config marker").unwrap();
    let seen = read_through_files(fx.vfs.config_dir(), "marker.txt");
    assert_eq!(seen.as_deref(), Some(&b"config marker"[..]));
}

#[test]
#[serial]
fn config_dir_defaults_to_dot_uqm_in_home() {
    let fx = Fixture::new();

    prepare_config_dir(&fx.vfs, None).unwrap();

    let expected = fx.home().join(".uqm");
    assert!(expected.is_dir());
    // The default keeps its trailing separator, as the C option default does.
    assert_eq!(
        std::env::var("UQM_CONFIG_DIR").unwrap(),
        format!("{}/", expected.to_str().unwrap())
    );
}

#[test]
#[serial]
fn config_dir_below_a_regular_file_is_an_io_error() {
    let fx = Fixture::new();
    let blocker = fx.home().join("blocker");
    std::fs::write(&blocker, b"not a directory").unwrap();
    let path = blocker.join("cfg");

    let result = prepare_config_dir(&fx.vfs, Some(path.to_str().unwrap()));

    assert!(matches!(result, Err(DirPrepError::Io(_))), "{result:?}");
    assert!(fx.vfs.config_dir().is_none());
}

#[test]
#[serial]
fn save_dir_defaults_below_the_config_dir() {
    let fx = Fixture::new();
    fx.prepare_config();

    prepare_save_dir(&fx.vfs).unwrap();

    let expected = fx.config_path().join("save");
    assert!(expected.is_dir());
    assert_eq!(
        std::env::var("UQM_SAVE_DIR").unwrap(),
        expected.to_str().unwrap()
    );
    std::fs::write(expected.join("slot1.sav"), b"saved game").unwrap();
    assert_eq!(
        read_through_files(fx.vfs.save_dir(), "slot1.sav").as_deref(),
        Some(&b"saved game"[..])
    );
}

#[test]
#[serial]
fn save_dir_expands_variables_in_an_override() {
    let fx = Fixture::new();
    fx.prepare_config();
    std::env::set_var("UQM_SAVE_DIR", "${UQM_CONFIG_DIR}/custom_saves");

    prepare_save_dir(&fx.vfs).unwrap();

    let expected = fx.config_path().join("custom_saves");
    assert!(expected.is_dir());
    assert_eq!(
        std::env::var("UQM_SAVE_DIR").unwrap(),
        expected.to_str().unwrap()
    );
}

#[test]
#[serial]
fn melee_dir_defaults_to_teams_below_the_config_dir() {
    let fx = Fixture::new();
    fx.prepare_config();

    prepare_melee_dir(&fx.vfs).unwrap();

    let expected = fx.config_path().join("teams");
    assert!(expected.is_dir());
    assert_eq!(
        std::env::var("UQM_MELEE_DIR").unwrap(),
        expected.to_str().unwrap()
    );
    std::fs::write(expected.join("mine.mle"), b"team").unwrap();
    assert_eq!(
        read_through_files(fx.vfs.melee_dir(), "mine.mle").as_deref(),
        Some(&b"team"[..])
    );
}

#[test]
#[serial]
fn save_dir_override_that_cannot_be_created_is_an_io_error() {
    let fx = Fixture::new();
    fx.prepare_config();
    let blocker = fx.home().join("blocker");
    std::fs::write(&blocker, b"file").unwrap();
    std::env::set_var("UQM_SAVE_DIR", blocker.join("saves"));

    let result = prepare_save_dir(&fx.vfs);

    assert!(matches!(result, Err(DirPrepError::Io(_))), "{result:?}");
    assert!(fx.vfs.save_dir().is_none());
}

#[test]
#[serial]
fn explicit_content_dir_without_version_file_is_rejected() {
    let fx = Fixture::new();
    let bare = fx.home().join("bare_content");
    std::fs::create_dir_all(&bare).unwrap();

    let result = prepare_content_dir(
        &fx.vfs,
        &fx.configuration,
        Some(bare.to_str().unwrap()),
        None,
    );

    match result {
        Err(DirPrepError::NotFound(message)) => {
            assert!(message.contains("bare_content"), "{message}");
            assert!(message.contains("version"), "{message}");
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
    assert!(fx.vfs.content_dir().is_none());
    assert!(fx.vfs.content_mount().is_none());
}

#[test]
#[serial]
fn content_dir_is_mounted_with_packages_and_addon_packs() {
    let fx = Fixture::new();
    let content = build_content_tree(fx.home());

    prepare_content_dir(
        &fx.vfs,
        &fx.configuration,
        Some(content.to_str().unwrap()),
        None,
    )
    .unwrap();

    let recorded = fx.configuration.content_path().unwrap();
    let recorded = recorded.to_str().unwrap();
    assert_eq!(recorded, content.to_str().unwrap());
    assert!(fx.vfs.content_mount().is_some());
    let dir = fx.vfs.content_dir();
    assert!(dir.is_some());
    assert_eq!(
        read_through_files(dir.clone(), "version").as_deref(),
        Some(&b"0.8.0\n"[..])
    );
    assert_eq!(
        read_through_files(dir.clone(), "from_package.txt").as_deref(),
        Some(&b"package data"[..])
    );
    assert_eq!(
        read_through_files(dir.clone(), "addons/from_top_addon.txt").as_deref(),
        Some(&b"top addon data"[..])
    );
    assert_eq!(
        read_through_files(dir, "addons/alpha/from_alpha_addon.txt").as_deref(),
        Some(&b"alpha addon data"[..])
    );
}

#[test]
#[serial]
fn explicit_addon_dir_is_mounted_under_addons() {
    let fx = Fixture::new();
    let content = build_content_tree(fx.home());
    let extra = fx.home().join("extra_addons");
    std::fs::create_dir_all(&extra).unwrap();
    std::fs::write(extra.join("extra.txt"), b"extra data").unwrap();

    prepare_content_dir(
        &fx.vfs,
        &fx.configuration,
        Some(content.to_str().unwrap()),
        Some(extra.to_str().unwrap()),
    )
    .unwrap();

    assert_eq!(
        read_through_files(fx.vfs.content_dir(), "addons/extra.txt").as_deref(),
        Some(&b"extra data"[..])
    );
}

#[test]
#[serial]
fn content_dir_without_an_addons_directory_still_prepares() {
    let fx = Fixture::new();
    let content = fx.home().join("slim_content");
    std::fs::create_dir_all(&content).unwrap();
    std::fs::write(content.join("version"), b"0.8.0\n").unwrap();

    prepare_content_dir(
        &fx.vfs,
        &fx.configuration,
        Some(content.to_str().unwrap()),
        None,
    )
    .unwrap();

    assert_eq!(
        read_through_files(fx.vfs.content_dir(), "version").as_deref(),
        Some(&b"0.8.0\n"[..])
    );
}

#[test]
#[serial]
fn shadow_addons_without_content_dir_do_nothing() {
    let fx = Fixture::new();
    assert!(fx.vfs.content_dir().is_none());

    prepare_shadow_addons(&fx.vfs, &["alpha".to_string()]);

    assert!(fx.vfs.content_mount().is_none());
}

#[test]
#[serial]
fn shadow_content_of_a_listed_addon_overlays_the_content_dir() {
    let fx = Fixture::new();
    let content = build_content_tree(fx.home());
    prepare_content_dir(
        &fx.vfs,
        &fx.configuration,
        Some(content.to_str().unwrap()),
        None,
    )
    .unwrap();
    let dir = fx.vfs.content_dir();
    assert_eq!(read_through_files(dir.clone(), "shadowed.txt"), None);

    prepare_shadow_addons(&fx.vfs, &["missing".to_string(), "alpha".to_string()]);

    assert_eq!(
        read_through_files(dir, "shadowed.txt").as_deref(),
        Some(&b"shadow data"[..])
    );
}

#[test]
#[serial]
fn the_mounted_content_dir_lists_its_top_level_entries() {
    let fx = Fixture::new();
    let content = build_content_tree(fx.home());
    prepare_content_dir(
        &fx.vfs,
        &fx.configuration,
        Some(content.to_str().unwrap()),
        None,
    )
    .unwrap();

    let names = list_names(fx.vfs.content_dir().unwrap(), "", Match::Prefix);

    for expected in ["addons", "packages", "version"] {
        assert!(
            names.iter().any(|name| name == expected),
            "{expected} missing from {names:?}"
        );
    }
}

#[test]
#[serial]
fn expanding_a_path_keeps_non_ascii_characters_intact() {
    let _fx = Fixture::new();
    std::env::set_var("UQM_CONFIG_DIR", "/opt/café");

    let expanded = expand_path("${UQM_CONFIG_DIR}/sauvegarde-é/日本").unwrap();

    assert_eq!(expanded, PathBuf::from("/opt/café/sauvegarde-é/日本"));
}

#[test]
#[serial]
fn dollar_signs_that_do_not_start_a_variable_stay_literal() {
    let _fx = Fixture::new();

    assert_eq!(expand_dollar_vars("cost$"), "cost$");
    assert_eq!(expand_dollar_vars("a$1b"), "a$1b");
    assert_eq!(expand_dollar_vars("${unclosed"), "${unclosed");
    assert_eq!(expand_dollar_vars("$$"), "$$");
}

#[test]
#[serial]
fn tilde_is_only_expanded_as_a_leading_home_reference() {
    let fx = Fixture::new();
    let home = fx.home().to_str().unwrap().to_string();

    assert_eq!(expand_env_vars("~"), home);
    assert_eq!(expand_env_vars("~/x"), format!("{home}/x"));
    assert_eq!(expand_env_vars("~other/x"), "~other/x");
    assert_eq!(expand_env_vars("a/~/x"), "a/~/x");
}
