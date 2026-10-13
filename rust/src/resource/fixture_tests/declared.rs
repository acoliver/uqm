//! Inputs and expected results for the #38 fixture rows.
//!
//! Written from the contract in the fixture spec (stock package, loose
//! override, addons, localization, case, path, encoding, value types) before
//! any of these rows ran against the Rust code. The expected values come from
//! the original C (`options.c`, `resinit.c`, `propfile.c`, the deleted
//! `uio/mount.c`) and from host tools (`shasum`, `unzip -v`, `grep`) run on
//! the checked-in content, never from the Rust output.

/// `uio_MountLocation` / mount flag values from `sc2/src/libs/uio/io.h` and
/// `mount.h`, kept apart from the Rust constants on purpose.
pub(super) const MOUNT_BOTTOM: i32 = 0;
pub(super) const MOUNT_RDONLY: i32 = 1 << 1;
pub(super) const MOUNT_TOP: i32 = 1 << 2;
pub(super) const MOUNT_BELOW: i32 = 2 << 2;
pub(super) const MOUNT_ABOVE: i32 = 3 << 2;

/// `uio_FSTYPE_*` from `fstypes.h`.
pub(super) const FSTYPE_STDIO: i32 = 1;
pub(super) const FSTYPE_ZIP: i32 = 2;

/// `match_MatchType` from `match.h` (HAVE_GLOB undefined).
pub(super) const MATCH_LITERAL: i32 = 0;
pub(super) const MATCH_PREFIX: i32 = 1;
pub(super) const MATCH_SUFFIX: i32 = 2;
pub(super) const MATCH_REGEX: i32 = 4;

/// The two patterns `options.c` passes to `uio_getDirList`.
pub(super) const ZIP_PATTERN: &str = "\\.([zZ][iI][pP]|[uU][qQ][mM])$";
pub(super) const RMP_PATTERN: &str = "\\.[rR][mM][pP]$";

// ---------------------------------------------------------------------------
// Stock content (checked in under sc2/content)
// ---------------------------------------------------------------------------

pub(super) const STOCK_PACKAGE: &str = "packages/uqm-0.8.0-content.uqm";

/// (file inside the package and in the loose tree, byte length, sha-256).
pub(super) const STOCK_FILES: [(&str, usize, &str); 3] = [
    (
        "uqm.rmp",
        60298,
        "b390c4017c061a3779440c20297ad5574626bfca6df39367663b2ac33c84bffe",
    ),
    (
        "menu.key",
        2851,
        "a9f4830e6fba7d36ec0e875cc01c4b9e7292f81c936ca9f98a086e0fdc757427",
    ),
    (
        "uqm.key",
        2118,
        "c5847094dd367218a7215797901e599d4916d01a82b9a89ac819776472ef8617",
    ),
];

pub(super) const STOCK_RMP_KEYS: usize = 963;
pub(super) const MENU_KEY_KEYS: usize = 60;
pub(super) const UQM_KEY_KEYS: usize = 66;
/// Distinct keys over stock `uqm.rmp` plus the four real addon indexes.
pub(super) const STOCK_PLUS_ADDONS_KEYS: usize = 998;
pub(super) const REAL_ADDONS: [&str; 4] = ["3domusic", "3dovideo", "3dovoice", "remix"];

/// (key, recorded type) pairs read off `uqm.rmp` with grep.
pub(super) const STOCK_TYPES: [(&str, &str); 3] = [
    ("comm.arilou.dialogue", "CONVERSATION"),
    ("music.battle", "MUSICRES"),
    ("colortable.main", "BINTAB"),
];

pub(super) const LANDER_ANI_SIZE: i64 = 1558;
pub(super) const LAUNCH_PNG: (&str, usize) = ("base/lander/launch-012.png", 933);
pub(super) const LOOSE_RMP: &[u8] = b"marker.override = STRING:loose\n";
pub(super) const LOOSE_ANI: &[u8] = b"LOOSE-ANI";
pub(super) const REMIX_ZIP_ENTRY: (&str, i64) =
    ("addons/remix/4/04_-_Shofixti_-_Bonsai!.ogg", 3979201);

// ---------------------------------------------------------------------------
// World A: stock, packages and addons
// ---------------------------------------------------------------------------

pub(super) type Files = &'static [(&'static str, &'static [u8])];

pub(super) struct ZipSpec {
    pub(super) path: &'static str,
    pub(super) entries: Files,
}

pub(super) const BBB_RMP: &[u8] =
    b"title = STRING:BBB\nbbb.only = STRING:B\nlang.hello = STRING:Bonjour\n";

pub(super) const WORLD_A_LOOSE: Files = &[
    (
        "uqm.rmp",
        b"title = STRING:Stock\nlang.hello = STRING:Hello\nstock.only = STRING:S\n",
    ),
    ("stock-override.txt", b"STOCK"),
    (
        "addons/aaa/aaa.rmp",
        b"title = STRING:AAA\naaa.only = STRING:A\n",
    ),
    ("addons/aaa/voice.txt", b"AAA-LOOSE-VOICE"),
    (
        "addons/aaa/shadow-content/stock-override.txt",
        b"AAA-SHADOW",
    ),
    ("addons/bbb/bbb.rmp", BBB_RMP),
    (
        "addons/bbb/shadow-content/stock-override.txt",
        b"BBB-SHADOW",
    ),
    (
        "addons/ccc/ccc.rmp",
        b"title = STRING:CCC\nccc.only = STRING:C\n",
    ),
    ("addons/ccc/voice.txt", b"CCC-VOICE"),
    ("addons/ccc/shadow-content/shadow-ccc.txt", b"CCC-SHADOW"),
    ("addons/.hidden/h.rmp", b"hidden.key = STRING:H\n"),
    ("addons/notadir.txt", b"file"),
    ("packages/skip.zipx", b"not a zip"),
    ("packages/skip.zip.bak", b"not a zip"),
];

pub(super) const WORLD_A_ZIPS: &[ZipSpec] = &[
    ZipSpec {
        path: "addons/aaa/aaa-pack.zip",
        entries: &[
            ("sounds/beep.txt", b"AAA-BEEP"),
            ("voice.txt", b"AAA-ZIP-VOICE"),
        ],
    },
    ZipSpec {
        path: "addons/bbb/b1.zip",
        entries: &[("pick.txt", b"B1")],
    },
    ZipSpec {
        path: "addons/bbb/b2.zip",
        entries: &[("pick.txt", b"B2")],
    },
    ZipSpec {
        path: "addons/bbb/shadow-content/sh.zip",
        entries: &[("zipped.txt", b"BBB-SHADOW-ZIP")],
    },
    ZipSpec {
        path: "packages/p1.uqm",
        entries: &[("shared.txt", b"P1"), ("p1-only.txt", b"P1-ONLY")],
    },
    ZipSpec {
        path: "packages/p2.uqm",
        entries: &[("shared.txt", b"P2")],
    },
    ZipSpec {
        path: "packages/pkg.ZIP",
        entries: &[("upper.txt", b"UPPER-ZIP")],
    },
];

pub(super) const WORLD_A_ADDON_NAMES: [&str; 3] = ["aaa", "bbb", "ccc"];

// ---------------------------------------------------------------------------
// World B: localization packs
// ---------------------------------------------------------------------------

pub(super) const FR_STRINGS: &[u8] = b"greeting = STRING:Bonjour\nfr.only = STRING:F\n";
pub(super) const DE_STRINGS: &[u8] = b"greeting = STRING:Hallo\nde.only = STRING:D\n";

pub(super) const WORLD_B_LOOSE: Files = &[
    ("addons/fr/strings.rmp", FR_STRINGS),
    ("addons/de/strings.rmp", DE_STRINGS),
];

pub(super) const WORLD_B_ZIPS: &[ZipSpec] = &[ZipSpec {
    path: "addons/de/strings.zip",
    entries: &[("strings.rmp", b"greeting = STRING:ZIP-LOST\n")],
}];

// ---------------------------------------------------------------------------
// Case rows (ZIP only: names that differ only in case cannot all exist on a
// case-insensitive host file system)
// ---------------------------------------------------------------------------

pub(super) const CASE_ZIP: Files = &[
    ("Case/Readme.TXT", b"UPPER"),
    ("case/readme.txt", b"lower"),
    ("CASE/README.txt", b"SHOUT"),
];

pub(super) const CASE_LISTING_ZIP: Files = &[
    ("a.rmp", b"1"),
    ("B.RMP", b"1"),
    ("c.Rmp", b"1"),
    ("d.rmpx", b"1"),
    ("e.rmp.bak", b"1"),
    ("Addons", b"1"),
    ("addons", b"1"),
];

// ---------------------------------------------------------------------------
// Value type rows: (descriptor after "TYPE:", expected)
// ---------------------------------------------------------------------------

pub(super) const INT_ROWS: [(&str, i32); 9] = [
    ("42", 42),
    ("-5", -5),
    ("+7", 7),
    (" 9", 9),
    ("12abc", 12),
    ("1.9", 1),
    ("0x10", 0),
    ("abc", 0),
    ("", 0),
];

pub(super) const BOOL_ROWS: [(&str, bool); 9] = [
    ("true", true),
    ("TRUE", true),
    ("TrUe", true),
    ("false", false),
    ("yes", false),
    ("1", false),
    ("", false),
    ("truee", false),
    (" true", false),
];

pub(super) const COLOR_ROWS: [(&str, u32); 7] = [
    ("rgb(1, 2, 3)", 0x0102_03ff),
    ("rgb ( 1 , 2 , 3 )", 0x0102_03ff),
    ("rgb(0x10, 010, 8)", 0x1008_08ff),
    ("rgba(1,2,3,4)", 0x0102_0304),
    ("rgb(300, -4, 5)", 0xff00_05ff),
    ("rgb15(31, 0, 16)", 0xff00_84ff),
    ("red", 0),
];

// ---------------------------------------------------------------------------
// Encoding rows
// ---------------------------------------------------------------------------

/// `caf\xC3\xA9 \xE2\x82\xAC`: six characters, nine bytes.
pub(super) const UTF8_VALUE: &[u8] = b"caf\xC3\xA9 \xE2\x82\xAC";
/// `Fran\xE7ais` in Latin-1, which is not valid UTF-8.
pub(super) const LATIN1_VALUE: &[u8] = b"Fran\xE7ais";
pub(super) const PREFIX_KEY_CAP: usize = 254;
pub(super) const LONG_VALUE_LEN: usize = 5000;
