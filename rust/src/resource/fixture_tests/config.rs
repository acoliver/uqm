//! Rows EN1-EN5 and VT1-VT6: config strings as bytes, value types, typed
//! getter defaults, and the save/load round trip, all through a real stdio
//! mount and `LoadResourceIndex`.

use serial_test::serial;
use tempfile::TempDir;

use super::declared::*;
use super::res::{self, Rows};
use super::uio_api::{Dir, Mount, Repo, Session};

/// A temp directory mounted writable at "/" with `root` open on it.
pub(super) struct Config {
    pub(super) root: Dir,
    _repo: Repo,
    pub(super) dir: TempDir,
}

pub(super) fn config_dir(files: &[(&str, &[u8])]) -> Config {
    let dir = TempDir::new().unwrap();
    for (name, bytes) in files {
        std::fs::write(dir.path().join(name), bytes).unwrap();
    }
    let repo = Repo::open();
    assert!(!repo
        .mount_stdio("/", dir.path(), MOUNT_TOP, Mount::NONE)
        .is_null());
    let root = repo.open_dir("/");
    Config {
        root,
        _repo: repo,
        dir,
    }
}

fn rmp_of(lines: &[String]) -> Vec<u8> {
    let mut text = lines.join("\n");
    text.push('\n');
    text.into_bytes()
}

#[test]
#[serial]
fn en1_utf8_value_comes_back_as_its_nine_bytes() {
    let _session = Session::begin();
    let mut file = b"name = STRING:".to_vec();
    file.extend_from_slice(UTF8_VALUE);
    file.push(b'\n');
    let cfg = config_dir(&[("uqm.cfg", &file)]);
    res::load(&cfg.root, "uqm.cfg", Some("config."));
    assert_eq!(UTF8_VALUE.len(), 9);
    assert_eq!(res::string("config.name"), UTF8_VALUE);
}

#[test]
#[serial]
fn en2_latin1_line_does_not_drop_the_file_and_keeps_its_bytes() {
    let _session = Session::begin();
    let mut file = b"first = STRING:one\nlatin = STRING:".to_vec();
    file.extend_from_slice(LATIN1_VALUE);
    file.extend_from_slice(b"\nlast = STRING:three\n");
    let cfg = config_dir(&[("uqm.cfg", &file)]);
    res::load(&cfg.root, "uqm.cfg", Some("config."));

    let mut rows = Rows::default();
    rows.check("first", res::string("config.first"), b"one".to_vec());
    rows.check("latin", res::string("config.latin"), LATIN1_VALUE.to_vec());
    rows.check("last", res::string("config.last"), b"three".to_vec());
    rows.finish();
}

#[test]
#[serial]
fn en3_crlf_line_ends_are_whitespace() {
    let _session = Session::begin();
    let cfg = config_dir(&[("uqm.cfg", b"a = INT32:5\r\nb = BOOLEAN:true\r\n")]);
    res::load(&cfg.root, "uqm.cfg", Some("config."));
    assert_eq!(res::integer("config.a"), 5);
    assert!(res::boolean("config.b"));
}

#[test]
#[serial]
fn en4_prefixed_keys_are_capped_at_254_bytes() {
    let _session = Session::begin();
    let long_key = "k".repeat(300);
    let cfg = config_dir(&[("uqm.cfg", format!("{long_key} = STRING:v\n").as_bytes())]);
    res::load(&cfg.root, "uqm.cfg", Some("config."));
    let stored = format!("config.{long_key}");
    assert!(
        res::has(&stored[..PREFIX_KEY_CAP]),
        "254-byte key is stored"
    );
    assert!(
        !res::has(&stored[..PREFIX_KEY_CAP + 1]),
        "255-byte key is not"
    );
}

#[test]
fn en4_a_cut_inside_a_character_does_not_panic() {
    // "config." is 7 bytes; with 244..=250 'k' the two bytes of "é" start at
    // byte 252..=258, so the 254-byte cut meets every alignment around it.
    for ks in 244..=250 {
        let key = format!("{}é", "k".repeat(ks));
        let text = format!("{key} = STRING:v\n");
        let mut keys = Vec::new();
        crate::resource::propfile::parse_propfile(
            &text,
            &mut |k, _| keys.push(k.to_string()),
            Some("config."),
        );
        assert_eq!(keys.len(), 1, "{ks} k");
        assert!(
            keys[0].len() <= PREFIX_KEY_CAP,
            "{ks} k: key is {} bytes",
            keys[0].len()
        );
        assert!(keys[0].starts_with("config.kkk"), "{ks} k");
    }
}

#[test]
#[serial]
fn en6_saved_latin1_value_is_written_back_byte_for_byte() {
    let _session = Session::begin();
    let mut file = b"latin = STRING:".to_vec();
    file.extend_from_slice(LATIN1_VALUE);
    file.push(b'\n');
    let cfg = config_dir(&[("in.cfg", &file)]);
    res::load(&cfg.root, "in.cfg", None);
    res::save(&cfg.root, "out.cfg", None);
    assert_eq!(std::fs::read(cfg.dir.path().join("out.cfg")).unwrap(), file);
}

#[test]
fn en7_byte_parser_follows_the_c_scanner_on_nul_and_vertical_tab() {
    let mut seen: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    // C works on strlen: nothing after the first NUL is read. isspace also
    // counts \v, which Rust's is_ascii_whitespace does not.
    let data = b"a\x0b=\x0bINT32:1\x0b\nlatin = STRING:\xE7\n\0skipped = INT32:2\n";
    crate::resource::propfile::parse_propfile_bytes(
        data,
        &mut |k, v| seen.push((k.to_vec(), v.to_vec())),
        None,
    );
    assert_eq!(
        seen,
        [
            (b"a".to_vec(), b"INT32:1".to_vec()),
            (b"latin".to_vec(), b"STRING:\xE7".to_vec()),
        ]
    );
}

#[test]
#[serial]
fn en5_long_values_are_not_capped_on_load() {
    let _session = Session::begin();
    let value = "x".repeat(LONG_VALUE_LEN);
    let cfg = config_dir(&[("uqm.cfg", format!("long = STRING:{value}\n").as_bytes())]);
    res::load(&cfg.root, "uqm.cfg", Some("config."));
    assert_eq!(res::text("config.long"), value);
}

#[test]
#[serial]
fn vt1_to_vt3_descriptors_load_with_the_c_value_rules() {
    let _session = Session::begin();
    let mut lines = Vec::new();
    for (n, (descriptor, _)) in INT_ROWS.iter().enumerate() {
        lines.push(format!("i{n} = INT32:{descriptor}"));
    }
    for (n, (descriptor, _)) in BOOL_ROWS.iter().enumerate() {
        lines.push(format!("b{n} = BOOLEAN:{descriptor}"));
    }
    for (n, (descriptor, _)) in COLOR_ROWS.iter().enumerate() {
        lines.push(format!("c{n} = COLOR:{descriptor}"));
    }
    let cfg = config_dir(&[("t.rmp", &rmp_of(&lines))]);
    res::load(&cfg.root, "t.rmp", None);

    let mut rows = Rows::default();
    for (n, (descriptor, want)) in INT_ROWS.iter().enumerate() {
        rows.check(
            &format!("INT32:{descriptor:?}"),
            res::integer(&format!("i{n}")),
            *want,
        );
    }
    for (n, (descriptor, want)) in BOOL_ROWS.iter().enumerate() {
        rows.check(
            &format!("BOOLEAN:{descriptor:?}"),
            res::boolean(&format!("b{n}")),
            *want,
        );
    }
    for (n, (descriptor, want)) in COLOR_ROWS.iter().enumerate() {
        rows.check(
            &format!("COLOR:{descriptor:?}"),
            res::color(&format!("c{n}")),
            *want,
        );
    }
    rows.finish();
}

#[test]
#[serial]
fn vt4_getters_check_the_entry_type_and_default_for_missing_keys() {
    let _session = Session::begin();
    let cfg = config_dir(&[(
        "t.rmp",
        b"s = STRING:text\ni = INT32:7\nb = BOOLEAN:true\nc = COLOR:rgb(1, 2, 3)\n",
    )]);
    res::load(&cfg.root, "t.rmp", None);

    let mut rows = Rows::default();
    rows.check("GetString(s)", res::text("s"), "text".to_string());
    rows.check("GetInteger(i)", res::integer("i"), 7);
    rows.check("GetBoolean(b)", res::boolean("b"), true);
    rows.check("GetColor(c)", res::color("c"), 0x0102_03ff);

    rows.check("GetString(i)", res::text("i"), String::new());
    rows.check("GetString(b)", res::text("b"), String::new());
    rows.check("GetString(c)", res::text("c"), String::new());
    rows.check("GetInteger(s)", res::integer("s"), 0);
    rows.check("GetInteger(b)", res::integer("b"), 0);
    rows.check("GetInteger(c)", res::integer("c"), 0);
    rows.check("GetBoolean(s)", res::boolean("s"), false);
    rows.check("GetBoolean(i)", res::boolean("i"), false);
    rows.check("GetBoolean(c)", res::boolean("c"), false);
    rows.check("GetColor(s)", res::color("s"), 0);
    rows.check("GetColor(i)", res::color("i"), 0);
    rows.check("GetColor(b)", res::color("b"), 0);

    rows.check("GetString(missing)", res::text("missing"), String::new());
    rows.check("GetInteger(missing)", res::integer("missing"), 0);
    rows.check("GetBoolean(missing)", res::boolean("missing"), false);
    rows.check("GetColor(missing)", res::color("missing"), 0);

    rows.check("kinds(s)", res::kinds("s"), [true, false, false, false]);
    rows.check("kinds(i)", res::kinds("i"), [false, true, false, false]);
    rows.check("kinds(b)", res::kinds("b"), [false, false, true, false]);
    rows.check("kinds(c)", res::kinds("c"), [false, false, false, true]);
    rows.check("kinds(missing)", res::kinds("missing"), [false; 4]);
    rows.finish();
}

#[test]
#[serial]
fn vt5_last_writer_wins_and_an_entry_may_change_type() {
    let _session = Session::begin();
    let cfg = config_dir(&[(
        "t.rmp",
        b"dup = INT32:1\ndup = INT32:2\nchg = STRING:x\nchg = INT32:9\n",
    )]);
    res::load(&cfg.root, "t.rmp", None);
    assert_eq!(res::integer("dup"), 2);
    assert_eq!(res::integer("chg"), 9);
    assert_eq!(res::kinds("chg"), [false, true, false, false]);
    assert_eq!(res::text("chg"), "");
}

#[test]
#[serial]
fn vt6_saved_index_loads_back_to_the_same_typed_values() {
    let _session = Session::begin();
    let source = b"s = STRING:two words here\nn = INT32:-12\nt = BOOLEAN:true\n\
o = COLOR:rgb(17, 34, 51)\na = COLOR:rgba(17, 34, 51, 68)\n";
    let cfg = config_dir(&[("src.rmp", source)]);
    res::load(&cfg.root, "src.rmp", None);
    res::save(&cfg.root, "out.cfg", None);

    let saved = std::fs::read_to_string(cfg.dir.path().join("out.cfg")).unwrap();
    let mut lines: Vec<&str> = saved.lines().collect();
    lines.sort_unstable();
    assert_eq!(
        lines,
        [
            "a = COLOR:rgba(0x11, 0x22, 0x33, 0x44)",
            "n = INT32:-12",
            "o = COLOR:rgb(0x11, 0x22, 0x33)",
            "s = STRING:two words here",
            "t = BOOLEAN:true",
        ]
    );

    res::uninit();
    res::load(&cfg.root, "out.cfg", None);
    let mut rows = Rows::default();
    rows.check("s", res::text("s"), "two words here".to_string());
    rows.check("n", res::integer("n"), -12);
    rows.check("t", res::boolean("t"), true);
    rows.check("o", res::color("o"), 0x1122_33ff);
    rows.check("a", res::color("a"), 0x1122_3344);
    rows.finish();
}
