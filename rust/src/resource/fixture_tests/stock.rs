//! Rows ST1-ST7: the checked-in stock content package, the loose tree, and
//! the real addons, through real mounts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serial_test::serial;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::declared::*;
use super::game::{write_files, Game};
use super::res::{self, Rows};
use super::uio_api::{Dir, Mount, Repo, Session};

/// The checked-in content, with no `..` left in it: UIO tells host paths from
/// virtual ones by prefix, and an unnormalized mount root would not match its own children.
fn content_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sc2/content")
        .canonicalize()
        .expect("sc2/content is checked in")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn host_text(name: &str) -> String {
    std::fs::read_to_string(content_root().join(name)).unwrap()
}

/// The stock package alone: ZIP at "/" with nothing else mounted.
fn package_only(repo: &Repo) -> Dir {
    let packages = repo.open_dir(content_root().join("packages").to_str().unwrap());
    let name = STOCK_PACKAGE.rsplit('/').next().unwrap();
    let mount = repo.mount_zip(
        "/",
        &packages,
        name,
        MOUNT_BOTTOM | MOUNT_RDONLY,
        Mount::NONE,
    );
    assert!(!mount.is_null(), "stock package did not mount");
    repo.open_dir("/")
}

/// A loose tree on TOP of the real stock package, laid out as the game finds
/// it: the package sits in `packages/` of the content directory and is mounted
/// BELOW the content mount by `mountContentDir`.
fn loose_over_package(loose: &Path) -> Game {
    std::fs::create_dir_all(loose.join("packages")).unwrap();
    std::fs::copy(
        content_root().join(STOCK_PACKAGE),
        loose.join(STOCK_PACKAGE),
    )
    .unwrap();
    Game::start(loose)
}

fn assert_stock_index_loaded() {
    let oracle = res::oracle_entries(&host_text("uqm.rmp"));
    assert_eq!(oracle.len(), STOCK_RMP_KEYS, "declared key count");
    let mut rows = Rows::default();
    for (key, value) in &oracle {
        rows.check(&format!("has {key}"), res::has(key), true);
        rows.check(
            &format!("type {key}"),
            res::type_of(key),
            Some(res::oracle_type(value).to_string()),
        );
    }
    rows.finish();
}

#[test]
#[serial]
fn st1_package_alone_serves_the_stock_files_byte_for_byte() {
    let _session = Session::begin();
    let repo = Repo::open();
    let root = package_only(&repo);
    for (name, len, sha) in STOCK_FILES {
        let bytes = root
            .read(name)
            .unwrap_or_else(|e| panic!("{name}: errno {e}"));
        assert_eq!(bytes.len(), len, "{name} length");
        assert_eq!(sha256_hex(&bytes), sha, "{name} sha256");
    }
    let (_, size) = root.stat("base/lander/lander.ani").unwrap();
    assert_eq!(size, LANDER_ANI_SIZE);
}

#[test]
#[serial]
fn st2_package_listing_filters_by_the_declared_patterns() {
    let _session = Session::begin();
    let repo = Repo::open();
    let root = package_only(&repo);
    assert_eq!(root.list(RMP_PATTERN, MATCH_REGEX), ["uqm.rmp"]);
    assert_eq!(root.list(".key", MATCH_SUFFIX), ["menu.key", "uqm.key"]);
    assert!(
        root.read("version").is_err(),
        "version is not in the archive"
    );
}

#[test]
#[serial]
fn st3_stock_index_loads_every_key_with_its_recorded_type() {
    let _session = Session::begin();
    let repo = Repo::open();
    let root = package_only(&repo);
    res::load(&root, "uqm.rmp", None);
    assert_stock_index_loaded();
    for (key, ty) in STOCK_TYPES {
        assert_eq!(res::type_of(key).as_deref(), Some(ty), "{key}");
    }
    assert!(!res::has("no.such.key"));
    assert_eq!(res::type_of("no.such.key"), None);
}

#[test]
#[serial]
fn st4_key_maps_load_with_their_game_prefixes() {
    let _session = Session::begin();
    let repo = Repo::open();
    let root = package_only(&repo);
    // input.c: LoadResourceIndex (contentDir, "menu.key", "menu.") and "uqm.key" with "keys."
    res::load(&root, "menu.key", Some("menu."));
    res::load(&root, "uqm.key", Some("keys."));
    let mut rows = Rows::default();
    for (file, prefix, count) in [
        ("menu.key", "menu.", MENU_KEY_KEYS),
        ("uqm.key", "keys.", UQM_KEY_KEYS),
    ] {
        let oracle = res::oracle_entries(&host_text(file));
        rows.check(&format!("{file} declared count"), oracle.len(), count);
        for (key, value) in oracle {
            let want = value
                .strip_prefix("STRING:")
                .expect("key files are STRING")
                .to_string();
            rows.check(
                &format!("{prefix}{key}"),
                res::text(&format!("{prefix}{key}")),
                want,
            );
            rows.check(&format!("bare {key} absent"), res::has(&key), false);
        }
    }
    rows.check(
        "keys.1.name",
        res::text("keys.1.name"),
        "Arrows".to_string(),
    );
    rows.check(
        "keys.1.up.1",
        res::text("keys.1.up.1"),
        "key Up".to_string(),
    );
    rows.finish();
}

#[test]
#[serial]
fn st5_package_supplies_files_the_loose_tree_lacks() {
    let _session = Session::begin();
    let loose = TempDir::new().unwrap();
    write_files(loose.path(), &[("readme.txt", b"loose only")]);
    let game = loose_over_package(loose.path());
    let root = &game.content;
    let bytes = root.read("uqm.rmp").unwrap();
    assert_eq!(sha256_hex(&bytes), STOCK_FILES[0].2);
    assert_eq!(root.read("readme.txt").unwrap(), b"loose only");
    res::load(root, "uqm.rmp", None);
    assert_stock_index_loaded();
}

#[test]
#[serial]
fn st6_loose_files_override_the_package_and_merge_with_it() {
    let _session = Session::begin();
    let loose = TempDir::new().unwrap();
    write_files(
        loose.path(),
        &[
            ("uqm.rmp", LOOSE_RMP),
            ("base/lander/lander.ani", LOOSE_ANI),
        ],
    );
    let game = loose_over_package(loose.path());
    let root = &game.content;

    assert_eq!(root.read("uqm.rmp").unwrap(), LOOSE_RMP);
    res::load(root, "uqm.rmp", None);
    assert_eq!(res::text("marker.override"), "loose");
    for (key, _) in res::oracle_entries(&host_text("uqm.rmp")) {
        assert!(
            !res::has(&key),
            "stock key {key} leaked through the loose override"
        );
    }

    assert_eq!(root.read("base/lander/lander.ani").unwrap(), LOOSE_ANI);
    let (name, len) = LAUNCH_PNG;
    assert_eq!(
        root.read(name).unwrap().len(),
        len,
        "package-only file in the same directory"
    );
    let lander = root.open_relative("base/lander");
    let names = lander.list_all();
    assert_eq!(names.iter().filter(|n| *n == "lander.ani").count(), 1);
    assert!(names.iter().any(|n| n == "launch-012.png"));
}

#[test]
#[serial]
fn st7_real_addons_load_after_stock_with_last_writer_wins() {
    let _session = Session::begin();
    let game = Game::start(&content_root());
    let addons = game.mount_addon_dir();
    assert_eq!(addons, REAL_ADDONS);

    res::load(&game.content, "uqm.rmp", None);
    let mut final_types: BTreeMap<String, String> = BTreeMap::new();
    for (key, value) in res::oracle_entries(&host_text("uqm.rmp")) {
        final_types.insert(key, res::oracle_type(&value).to_string());
    }
    for addon in REAL_ADDONS {
        assert!(game.load_addon(addon), "addon {addon} loaded no index");
        let mut indexes: Vec<_> = std::fs::read_dir(content_root().join("addons").join(addon))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "rmp"))
            .collect();
        indexes.sort();
        for path in indexes {
            let text = std::fs::read_to_string(path).unwrap();
            for (key, value) in res::oracle_entries(&text) {
                final_types.insert(key, res::oracle_type(&value).to_string());
            }
        }
    }
    assert_eq!(
        final_types.len(),
        STOCK_PLUS_ADDONS_KEYS,
        "declared union count"
    );
    let mut rows = Rows::default();
    for (key, ty) in &final_types {
        rows.check(&format!("type {key}"), res::type_of(key), Some(ty.clone()));
    }
    rows.finish();

    // The remix archive mounted at "addons" and the loose remix tree overlap;
    // either way the file has the size `unzip -v` reports.
    let (name, size) = REMIX_ZIP_ENTRY;
    let (_, got) = game.content.stat(name).unwrap();
    assert_eq!(got, size);
}
