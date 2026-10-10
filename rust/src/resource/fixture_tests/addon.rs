//! Rows AD1-AD7 and LC1-LC2: addon namespaces, archive priority, shadow
//! content, localization collisions, and the mount-order model.

use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use serial_test::serial;
use tempfile::TempDir;

use super::declared::*;
use super::game::{build_world, write_files, Game};
use super::res;
use super::uio_api::{Mount, Repo, Session};

struct World {
    game: Game,
    _dir: TempDir,
}

fn world_a() -> World {
    let dir = build_world(WORLD_A_LOOSE, WORLD_A_ZIPS);
    let game = Game::start(dir.path());
    game.mount_addon_dir();
    World { game, _dir: dir }
}

fn read(world: &World, path: &str) -> Result<Vec<u8>, i32> {
    world.game.content.read(path)
}

fn assert_reads(world: &World, rows: &[(&str, &[u8])]) {
    let mut wrong = Vec::new();
    for (path, want) in rows {
        match read(world, path) {
            Ok(got) if got == *want => {}
            other => wrong.push(format!("{path}: got {other:?}, want {want:?}")),
        }
    }
    assert!(wrong.is_empty(), "wrong reads:\n{}", wrong.join("\n"));
}

fn assert_absent(world: &World, paths: &[&str]) {
    for path in paths {
        assert!(read(world, path).is_err(), "{path} must not be visible");
    }
}

#[test]
#[serial]
fn ad1_addon_files_live_under_their_own_namespace() {
    let _session = Session::begin();
    let dir = build_world(WORLD_A_LOOSE, WORLD_A_ZIPS);
    let game = Game::start(dir.path());
    assert_eq!(game.mount_addon_dir(), WORLD_A_ADDON_NAMES);
    let world = World { game, _dir: dir };

    assert_reads(
        &world,
        &[
            ("addons/aaa/voice.txt", b"AAA-LOOSE-VOICE"),
            ("addons/aaa/sounds/beep.txt", b"AAA-BEEP"),
            ("addons/bbb/pick.txt", b"B2"),
            ("addons/bbb/bbb.rmp", BBB_RMP),
        ],
    );
    assert_absent(
        &world,
        &["voice.txt", "pick.txt", "sounds/beep.txt", "bbb.rmp"],
    );
}

#[test]
#[serial]
fn ad2_only_requested_addons_are_read_and_the_last_loaded_wins() {
    let _session = Session::begin();
    let world = world_a();
    res::load(&world.game.content, "uqm.rmp", None);
    world.game.prepare_addons(&["bbb", "aaa"]);

    let mut rows = res::Rows::default();
    rows.check("title", res::text("title"), "AAA".to_string());
    rows.check("lang.hello", res::text("lang.hello"), "Bonjour".to_string());
    rows.check("stock.only", res::text("stock.only"), "S".to_string());
    rows.check("aaa.only", res::text("aaa.only"), "A".to_string());
    rows.check("bbb.only", res::text("bbb.only"), "B".to_string());
    rows.check("ccc.only", res::has("ccc.only"), false);
    rows.check("hidden.key", res::has("hidden.key"), false);
    rows.finish();

    res::uninit();
    res::load(&world.game.content, "uqm.rmp", None);
    world.game.prepare_addons(&["aaa", "bbb"]);
    assert_eq!(res::text("title"), "BBB");
    assert_eq!(res::text("lang.hello"), "Bonjour");
}

#[test]
#[serial]
fn ad3_later_archive_in_list_order_wins_below_the_same_mount() {
    let _session = Session::begin();
    let world = world_a();
    assert_reads(
        &world,
        &[
            ("addons/bbb/pick.txt", b"B2"),
            ("shared.txt", b"P2"),
            ("p1-only.txt", b"P1-ONLY"),
        ],
    );
}

#[test]
#[serial]
fn ad4_only_zip_and_uqm_names_mount_and_the_pattern_is_anchored() {
    let _session = Session::begin();
    let world = world_a();
    assert_reads(
        &world,
        &[
            ("upper.txt", b"UPPER-ZIP"),
            ("stock-override.txt", b"STOCK"),
        ],
    );
    let packages = world.game.content.open_relative("packages");
    let zips = packages.list(ZIP_PATTERN, MATCH_REGEX);
    assert_eq!(zips, ["p1.uqm", "p2.uqm", "pkg.ZIP"]);
}

#[test]
#[serial]
fn ad5_shadow_content_sits_above_stock_and_the_first_addon_wins() {
    let _session = Session::begin();
    let world = world_a();
    assert_reads(&world, &[("stock-override.txt", b"STOCK")]);

    world.game.prepare_shadow_addons(&["bbb", "aaa"]);
    assert_reads(
        &world,
        &[
            ("stock-override.txt", b"BBB-SHADOW"),
            ("zipped.txt", b"BBB-SHADOW-ZIP"),
        ],
    );
    assert_absent(&world, &["shadow-ccc.txt"]);

    let reversed = world_a();
    reversed.game.prepare_shadow_addons(&["aaa", "bbb"]);
    assert_reads(&reversed, &[("stock-override.txt", b"AAA-SHADOW")]);
}

#[test]
#[serial]
fn ad6_mount_kinds_keep_the_c_array_order_whatever_their_mount_ids() {
    let _session = Session::begin();
    let repo = Repo::open();
    let names = ["bottom", "config", "content", "package"];
    let dirs: Vec<TempDir> = names
        .iter()
        .map(|name| {
            let dir = TempDir::new().unwrap();
            write_files(dir.path(), &[("f.txt", name.as_bytes())]);
            dir
        })
        .collect();
    let bottom = repo.mount_stdio("/", dirs[0].path(), MOUNT_BOTTOM, Mount::NONE);
    let config = repo.mount_stdio("/", dirs[1].path(), MOUNT_TOP, Mount::NONE);
    let content = repo.mount_stdio("/", dirs[2].path(), MOUNT_TOP | MOUNT_RDONLY, Mount::NONE);
    let package = repo.mount_stdio("/", dirs[3].path(), MOUNT_BELOW | MOUNT_RDONLY, content);
    let root = repo.open_dir("/");

    let mut seen = Vec::new();
    for mount in [content, package, config, bottom] {
        seen.push(String::from_utf8(root.read("f.txt").unwrap()).unwrap());
        assert_eq!(repo.unmount(mount), 0);
    }
    assert_eq!(seen, ["content", "package", "config", "bottom"]);
}

#[test]
#[serial]
fn lc1_same_index_name_in_two_addons_stays_separate() {
    let _session = Session::begin();
    let dir = build_world(WORLD_B_LOOSE, WORLD_B_ZIPS);
    let game = Game::start(dir.path());
    assert_eq!(game.mount_addon_dir(), ["de", "fr"]);
    let world = World { game, _dir: dir };

    assert_reads(
        &world,
        &[
            ("addons/fr/strings.rmp", FR_STRINGS),
            ("addons/de/strings.rmp", DE_STRINGS),
        ],
    );
    assert_absent(&world, &["strings.rmp"]);
}

#[test]
#[serial]
fn lc2_colliding_translations_resolve_by_load_order() {
    let _session = Session::begin();
    let dir = build_world(WORLD_B_LOOSE, WORLD_B_ZIPS);
    let game = Game::start(dir.path());
    game.mount_addon_dir();

    game.prepare_addons(&["fr", "de"]);
    assert_eq!(res::text("greeting"), "Hallo");
    assert_eq!(res::text("fr.only"), "F");
    assert_eq!(res::text("de.only"), "D");

    res::uninit();
    game.prepare_addons(&["de", "fr"]);
    assert_eq!(res::text("greeting"), "Bonjour");
}

/// Where a mount goes, relative mounts naming an earlier mount by index.
#[derive(Clone, Debug)]
enum Place {
    Top,
    Bottom,
    Above(usize),
    Below(usize),
}

fn place() -> impl Strategy<Value = Place> {
    prop_oneof![
        Just(Place::Top),
        Just(Place::Bottom),
        (0usize..8).prop_map(Place::Above),
        (0usize..8).prop_map(Place::Below),
    ]
}

/// The deleted C `uio_repositoryAddMount`: one array, index 0 on top.
fn c_array_order(places: &[Place]) -> Vec<usize> {
    let mut order: Vec<usize> = Vec::new();
    for (id, place) in places.iter().enumerate() {
        let index_of = |order: &[usize], rel: usize| {
            let rel = rel % id;
            order
                .iter()
                .position(|&m| m == rel)
                .expect("relative is mounted")
        };
        match place {
            Place::Bottom => order.push(id),
            Place::Above(rel) if id > 0 => {
                let at = index_of(&order, *rel);
                order.insert(at, id);
            }
            Place::Below(rel) if id > 0 => {
                let at = index_of(&order, *rel);
                order.insert(at + 1, id);
            }
            Place::Top | Place::Above(_) | Place::Below(_) => order.insert(0, id),
        }
    }
    order
}

#[test]
#[serial]
fn ad7_mount_order_matches_the_c_array_model() {
    let _session = Session::begin();
    let config = Config {
        cases: 256,
        failure_persistence: None,
        ..Config::default()
    };
    let rng = TestRng::from_seed(RngAlgorithm::ChaCha, &[0x38; 32]);
    let mut runner = TestRunner::new_with_rng(config, rng);
    runner
        .run(&proptest::collection::vec(place(), 1..8), |places| {
            let expected = c_array_order(&places);
            let repo = Repo::open();
            let dirs: Vec<TempDir> = (0..places.len())
                .map(|id| {
                    let dir = TempDir::new().unwrap();
                    write_files(dir.path(), &[("who", id.to_string().as_bytes())]);
                    dir
                })
                .collect();
            let mut mounts: Vec<Mount> = Vec::new();
            for (id, place) in places.iter().enumerate() {
                let (flags, relative) = match place {
                    Place::Above(rel) if id > 0 => (MOUNT_ABOVE, mounts[rel % id]),
                    Place::Below(rel) if id > 0 => (MOUNT_BELOW, mounts[rel % id]),
                    Place::Bottom => (MOUNT_BOTTOM, Mount::NONE),
                    _ => (MOUNT_TOP, Mount::NONE),
                };
                let mount = repo.mount_stdio("/", dirs[id].path(), flags, relative);
                prop_assert!(!mount.is_null());
                mounts.push(mount);
            }
            let root = repo.open_dir("/");
            let mut seen = Vec::new();
            for &id in &expected {
                let who = root.read("who").unwrap();
                seen.push(String::from_utf8(who).unwrap().parse::<usize>().unwrap());
                prop_assert_eq!(repo.unmount(mounts[id]), 0);
            }
            prop_assert_eq!(seen, expected);
            Ok(())
        })
        .unwrap();
}
