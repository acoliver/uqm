//! Behavior of `LoadDirEntryTable` against real UIO mount stacks: stdio
//! directories and zip archives, mount precedence, and the table the C
//! consumers (`gfxload.c`, `loadmele.c`) receive.

use std::ffi::{c_int, CStr, CString};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr::{self, NonNull};

use proptest::prelude::*;
use serial_test::serial;
use tempfile::TempDir;

use super::c_abi::{table_or_null, LoadDirEntryTable};
use super::{load_dir_entry_table, scan_dir_entries, table_from_names, DirEntryError, MatchType};
use crate::io::uio_bridge::{
    uio_DirHandle, uio_Repository, uio_closeDir, uio_closeRepository, uio_mountDir, uio_openDir,
    uio_openRepository,
};
use crate::resource::string_table::tests::c_free_string_table;
use crate::resource::string_table::{OwnedStringTable, StringTable, StringTableError, MAX_ENTRIES};

// uio_bridge keeps these private; the values are fixed by the UIO API.
const FSTYPE_STDIO: c_int = 1;
const FSTYPE_ZIP: c_int = 2;
const MOUNT_RDONLY: c_int = 1 << 1;
const MOUNT_TOP: c_int = 1 << 2;

fn cstring(text: &str) -> CString {
    CString::new(text).unwrap()
}

/// A repository whose mounts all cover `/`, like the game's `contentDir`.
struct Fixture {
    repo: *mut uio_Repository,
    root: *mut uio_DirHandle,
    hosts: Vec<TempDir>,
}

impl Fixture {
    fn new() -> Self {
        // SAFETY: a fresh repository and a handle opened on it.
        unsafe {
            let repo = uio_openRepository(0);
            let root = uio_openDir(repo, c"/".as_ptr(), 0);
            assert!(!root.is_null());
            Fixture {
                repo,
                root,
                hosts: Vec::new(),
            }
        }
    }

    fn mount(&self, fs_type: c_int, host: &Path) {
        let host = CString::new(host.as_os_str().as_bytes()).unwrap();
        // SAFETY: all strings are NUL-terminated and outlive the call.
        let handle = unsafe {
            uio_mountDir(
                self.repo,
                c"/".as_ptr(),
                fs_type,
                ptr::null_mut(),
                ptr::null(),
                host.as_ptr(),
                ptr::null_mut(),
                MOUNT_TOP | MOUNT_RDONLY,
                ptr::null_mut(),
            )
        };
        assert!(!handle.is_null(), "mount of {host:?} failed");
    }

    /// Creates a host directory with `files`, mounts it above everything
    /// mounted so far, and returns its path for further changes.
    fn mount_dir(&mut self, files: &[&str]) -> PathBuf {
        let host = TempDir::new().unwrap();
        for name in files {
            std::fs::write(host.path().join(name), b"data").unwrap();
        }
        let path = host.path().to_path_buf();
        self.hosts.push(host);
        self.mount(FSTYPE_STDIO, &path);
        path
    }

    /// Mounts a zip holding `files` (empty content) and directory entries
    /// `dirs` above everything mounted so far.
    fn mount_zip(&mut self, files: &[&str], dirs: &[&str]) {
        let host = TempDir::new().unwrap();
        let archive = host.path().join("pack.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for dir in dirs {
            zip.add_directory(*dir, options).unwrap();
        }
        for name in files {
            zip.start_file(*name, options).unwrap();
            zip.write_all(b"data").unwrap();
        }
        zip.finish().unwrap();
        self.hosts.push(host);
        self.mount(FSTYPE_ZIP, &archive);
    }

    fn base(&self) -> NonNull<uio_DirHandle> {
        NonNull::new(self.root).unwrap()
    }

    fn scan_bytes(&self, path: &str, pattern: &str, match_type: MatchType) -> Vec<Vec<u8>> {
        let path = cstring(path);
        let pattern = cstring(pattern);
        // SAFETY: `base` is live for the duration of the call.
        let names = unsafe { scan_dir_entries(self.base(), &path, &pattern, match_type) }.unwrap();
        names.into_iter().map(CString::into_bytes).collect()
    }

    fn scan(&self, path: &str, pattern: &str, match_type: MatchType) -> Vec<String> {
        self.scan_bytes(path, pattern, match_type)
            .into_iter()
            .map(|name| String::from_utf8(name).unwrap())
            .collect()
    }

    fn load(&self, path: &str, pattern: &str, match_type: MatchType) -> Option<OwnedStringTable> {
        self.try_load(path, pattern, match_type).unwrap()
    }

    fn try_load(
        &self,
        path: &str,
        pattern: &str,
        match_type: MatchType,
    ) -> Result<Option<OwnedStringTable>, DirEntryError> {
        let path = cstring(path);
        let pattern = cstring(pattern);
        // SAFETY: `base` is live for the duration of the call.
        unsafe { load_dir_entry_table(self.base(), &path, &pattern, match_type) }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // SAFETY: both pointers came from `new` and are released once.
        unsafe {
            uio_closeDir(self.root);
            uio_closeRepository(self.repo);
        }
    }
}

fn strs(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

// --- selection ------------------------------------------------------------

#[test]
#[serial]
fn only_visible_regular_files_are_listed() {
    let mut fixture = Fixture::new();
    let host = fixture.mount_dir(&["real.mle", ".hidden.mle", "target.dat"]);
    std::fs::create_dir(host.join("folder.mle")).unwrap();
    std::os::unix::fs::symlink(host.join("target.dat"), host.join("link.mle")).unwrap();
    std::os::unix::fs::symlink(host.join("folder.mle"), host.join("dirlink.mle")).unwrap();
    std::os::unix::fs::symlink(host.join("absent"), host.join("dangling.mle")).unwrap();

    // A symlink to a file is a regular file to stat; a symlink to a
    // directory is not; a dangling link cannot be stat'ed at all.
    assert_eq!(
        fixture.scan("", ".mle", MatchType::Suffix),
        strs(&["link.mle", "real.mle"])
    );
}

#[test]
#[serial]
fn a_dot_prefix_hides_a_name_even_when_the_pattern_names_it() {
    let mut fixture = Fixture::new();
    fixture.mount_dir(&[".secret", "visible"]);

    assert!(fixture.scan("", ".secret", MatchType::Literal).is_empty());
    assert!(fixture.scan("", ".", MatchType::Prefix).is_empty());
    assert_eq!(
        fixture.scan("", "e", MatchType::Substring),
        strs(&["visible"])
    );
}

#[test]
#[serial]
fn names_come_back_in_byte_order_not_case_folded_or_numeric_order() {
    let mut fixture = Fixture::new();
    fixture.mount_dir(&[
        "alpha.mle",
        "Beta.mle",
        "Zeta.mle",
        "9.mle",
        "10.mle",
        "\u{e9}.mle",
        "gamma.mle",
    ]);

    // strcasecmp would put alpha first and Zeta last; a natural sort would
    // put 9 before 10; é encodes as 0xC3 0xA9 and sorts after ASCII.
    let expected: Vec<Vec<u8>> = [
        &b"10.mle"[..],
        b"9.mle",
        b"Beta.mle",
        b"Zeta.mle",
        b"alpha.mle",
        b"gamma.mle",
        "\u{e9}.mle".as_bytes(),
    ]
    .iter()
    .map(|name| name.to_vec())
    .collect();
    assert_eq!(fixture.scan_bytes("", ".mle", MatchType::Suffix), expected);
}

#[test]
#[serial]
fn every_match_type_selects_what_its_name_says() {
    let mut fixture = Fixture::new();
    fixture.mount_dir(&["a.mle", "ab.mle", "ba.txt", "a.txt", "xa"]);

    let cases: [(&str, MatchType, &[&str]); 7] = [
        ("a.txt", MatchType::Literal, &["a.txt"]),
        ("a", MatchType::Prefix, &["a.mle", "a.txt", "ab.mle"]),
        (".mle", MatchType::Suffix, &["a.mle", "ab.mle"]),
        ("a.", MatchType::Substring, &["a.mle", "a.txt", "ba.txt"]),
        ("^a.*\\.mle$", MatchType::Regex, &["a.mle", "ab.mle"]),
        (".MLE", MatchType::Suffix, &[]),
        (
            "",
            MatchType::Suffix,
            &["a.mle", "a.txt", "ab.mle", "ba.txt", "xa"],
        ),
    ];
    for (pattern, match_type, expected) in cases {
        assert_eq!(
            fixture.scan("", pattern, match_type),
            strs(expected),
            "{match_type:?} {pattern:?}"
        );
    }
}

#[test]
#[serial]
fn the_game_call_sites_find_their_files() {
    let mut fixture = Fixture::new();
    let host = fixture.mount_dir(&[]);
    std::fs::create_dir(host.join("teams")).unwrap();
    std::fs::create_dir(host.join("font")).unwrap();
    for name in ["Zed.mle", "alpha.mle", "notes.txt"] {
        std::fs::write(host.join("teams").join(name), b"team").unwrap();
    }
    for name in ["0041.png", "0020.png", "readme"] {
        std::fs::write(host.join("font").join(name), b"glyph").unwrap();
    }

    // loadmele.c: LoadDirEntryTable (meleeDir, "", ".mle", match_MATCH_SUFFIX)
    assert_eq!(
        fixture.scan("teams", ".mle", MatchType::Suffix),
        strs(&["Zed.mle", "alpha.mle"])
    );
    // gfxload.c: LoadDirEntryTable (contentDir, fontDir, ".", match_MATCH_SUBSTRING)
    assert_eq!(
        fixture.scan("font", ".", MatchType::Substring),
        strs(&["0020.png", "0041.png"])
    );
}

#[test]
#[serial]
fn a_file_named_by_bytes_that_are_not_utf8_keeps_those_bytes() {
    let mut fixture = Fixture::new();
    let host = fixture.mount_dir(&["plain.mle"]);
    let name: &[u8] = b"caf\xe9.mle";
    match std::fs::write(host.join(std::ffi::OsStr::from_bytes(name)), b"data") {
        Ok(()) => assert_eq!(
            fixture.scan_bytes("", "", MatchType::Substring),
            vec![name.to_vec(), b"plain.mle".to_vec()],
            "byte order, the name unchanged"
        ),
        Err(error) => {
            // Apple file systems cannot hold such a name; the byte path is
            // covered by the UIO name tests there.
            assert_eq!(error.raw_os_error(), Some(libc::EILSEQ), "{error}");
            if cfg!(not(target_vendor = "apple")) {
                panic!("only Apple file systems refuse byte names");
            }
        }
    }
}

// --- mounts ---------------------------------------------------------------

#[test]
#[serial]
fn a_name_present_in_several_mounts_is_listed_once() {
    let mut fixture = Fixture::new();
    fixture.mount_dir(&["common.mle", "low.mle"]);
    fixture.mount_dir(&["common.mle", "up.mle"]);

    assert_eq!(
        fixture.scan("", ".mle", MatchType::Suffix),
        strs(&["common.mle", "low.mle", "up.mle"])
    );
}

#[test]
#[serial]
fn a_directory_in_a_higher_mount_hides_the_file_beneath_it() {
    let mut fixture = Fixture::new();
    fixture.mount_dir(&["shadowed.mle", "plain.mle"]);
    let upper = fixture.mount_dir(&[]);
    std::fs::create_dir(upper.join("shadowed.mle")).unwrap();

    // uio_stat resolves the topmost object, which is the directory.
    assert_eq!(
        fixture.scan("", ".mle", MatchType::Suffix),
        strs(&["plain.mle"])
    );
}

#[test]
#[serial]
fn zip_entries_are_listed_alongside_loose_files() {
    let mut fixture = Fixture::new();
    fixture.mount_zip(&["packed.mle", "both.mle", "inner/deep.mle"], &["inner/"]);
    fixture.mount_dir(&["loose.mle", "both.mle"]);

    assert_eq!(
        fixture.scan("", ".mle", MatchType::Suffix),
        strs(&["both.mle", "loose.mle", "packed.mle"])
    );
    assert_eq!(
        fixture.scan("inner", ".mle", MatchType::Suffix),
        strs(&["deep.mle"])
    );
}

#[test]
#[serial]
fn a_zip_directory_is_not_a_regular_file() {
    let mut fixture = Fixture::new();
    fixture.mount_zip(&["file.mle", "folder.mle/inside"], &["folder.mle/"]);

    assert_eq!(
        fixture.scan("", ".mle", MatchType::Suffix),
        strs(&["file.mle"])
    );
}

// --- table ----------------------------------------------------------------

#[test]
#[serial]
fn no_matching_entry_means_no_table() {
    let mut fixture = Fixture::new();
    let host = fixture.mount_dir(&["a.txt", ".hidden.mle"]);
    std::fs::create_dir(host.join("folder.mle")).unwrap();
    std::fs::create_dir(host.join("empty")).unwrap();

    assert!(fixture.load("", ".mle", MatchType::Suffix).is_none());
    assert!(fixture.load("empty", "", MatchType::Substring).is_none());
    // A directory the repository does not have cannot be opened: that is an
    // error, not an empty table (the C loader asserts on it).
    assert!(matches!(
        fixture.try_load("missing", ".mle", MatchType::Suffix),
        Err(DirEntryError::OpenDir)
    ));
}

#[test]
#[serial]
fn a_large_directory_keeps_count_order_and_indices() {
    let mut fixture = Fixture::new();
    let names: Vec<String> = (0..300).map(|n| format!("team{n:03}.mle")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    fixture.mount_dir(&refs);

    let table = fixture.load("", ".mle", MatchType::Suffix).unwrap();
    assert_eq!(table.len(), 300);
    let raw = table.into_raw();
    unsafe {
        let StringTable { strings, size, .. } = raw.read();
        assert_eq!(size, 300);
        for (slot, name) in names.iter().enumerate() {
            let entry = strings.add(slot).read();
            assert_eq!(entry.index as usize, slot);
            assert_eq!(CStr::from_ptr(entry.data).to_str().unwrap(), name);
        }
        c_free_string_table(raw);
    }
}

// --- C ABI ----------------------------------------------------------------

#[test]
#[serial]
fn the_c_entry_point_returns_a_table_the_c_side_can_walk_and_free() {
    let mut fixture = Fixture::new();
    fixture.mount_dir(&["b.mle", "a.mle", "skip.txt"]);

    let table = unsafe {
        LoadDirEntryTable(
            fixture.root,
            c"".as_ptr(),
            c".mle".as_ptr(),
            MatchType::Suffix as c_int,
        )
    };
    assert!(!table.is_null());
    unsafe {
        // CaptureStringTable / GetStringAddress / DestroyStringTable
        let StringTable { size, strings, .. } = table.read();
        assert_eq!(size, 2);
        assert_eq!(CStr::from_ptr(strings.read().data).to_bytes(), b"a.mle");
        c_free_string_table(table);
    }
}

#[test]
#[serial]
fn the_c_entry_point_returns_null_when_nothing_matches() {
    let mut fixture = Fixture::new();
    fixture.mount_dir(&["a.txt"]);

    let table = unsafe {
        LoadDirEntryTable(
            fixture.root,
            c"".as_ptr(),
            c".mle".as_ptr(),
            MatchType::Suffix as c_int,
        )
    };
    assert!(table.is_null());
}

#[test]
fn match_types_follow_the_c_enum() {
    for (value, expected) in [
        (0, MatchType::Literal),
        (1, MatchType::Prefix),
        (2, MatchType::Suffix),
        (3, MatchType::Substring),
        (4, MatchType::Regex),
    ] {
        assert_eq!(MatchType::from_c(value), Some(expected));
        assert_eq!(expected as c_int, value);
    }
    assert_eq!(MatchType::from_c(5), None);
    assert_eq!(MatchType::from_c(-1), None);
}

#[test]
fn errors_name_the_failing_uio_call() {
    assert_eq!(
        DirEntryError::OpenDir.to_string(),
        "uio_openDirRelative failed"
    );
    assert_eq!(DirEntryError::ListDir.to_string(), "uio_getDirList failed");
}

// --- listings beyond what a table can address -------------------------------
//
// No directory with 65536 files is created: the selected names are plain
// owned Rust strings by the time the table is built, so the limit is
// exercised with that many names directly.

fn many_names(count: usize) -> Vec<CString> {
    (0..count)
        .map(|n| CString::new(format!("f{n}.mle")).unwrap())
        .collect()
}

#[test]
fn a_listing_of_the_largest_addressable_size_becomes_a_table() {
    let table = table_from_names(&many_names(MAX_ENTRIES))
        .unwrap()
        .expect("table");
    assert_eq!(table.len(), MAX_ENTRIES);
}

#[test]
fn a_listing_past_the_addressable_size_is_an_error_not_a_wrapped_count() {
    for count in [MAX_ENTRIES + 1, MAX_ENTRIES + 2] {
        // C's COUNT would have held 0 and 1 here.
        assert_eq!(
            table_from_names(&many_names(count)).err(),
            Some(DirEntryError::Table(StringTableError::TooManyEntries {
                count
            }))
        );
    }
}

#[test]
fn an_empty_selection_is_still_no_table() {
    assert!(table_from_names(&[]).unwrap().is_none());
}

#[test]
fn the_c_view_of_an_oversized_listing_is_null() {
    let error = table_from_names(&many_names(MAX_ENTRIES + 1)).err();
    assert!(table_or_null(Err(error.unwrap())).is_null());
}

#[test]
fn the_c_view_of_a_loaded_table_is_a_walkable_table() {
    let table = table_from_names(&many_names(3)).unwrap();
    let raw = table_or_null(Ok(table));
    assert!(!raw.is_null());
    unsafe {
        let StringTable { size, strings, .. } = raw.read();
        assert_eq!(size, 3);
        assert_eq!(
            CStr::from_ptr(strings.add(2).read().data).to_bytes(),
            b"f2.mle"
        );
        c_free_string_table(raw);
    }
}

#[test]
fn the_c_view_of_no_match_is_null() {
    assert!(table_or_null(Ok(None)).is_null());
}

#[test]
fn a_failing_open_still_aborts_like_the_c_assert() {
    crate::resource::abort_child::expect_abort(
        module_path!(),
        "a_failing_open_still_aborts_like_the_c_assert",
        "uio_openDirRelative failed",
        || {
            table_or_null(Err(DirEntryError::OpenDir));
        },
    );
}

#[test]
fn a_failing_list_still_aborts_like_the_c_assert() {
    crate::resource::abort_child::expect_abort(
        module_path!(),
        "a_failing_list_still_aborts_like_the_c_assert",
        "uio_getDirList failed",
        || {
            table_or_null(Err(DirEntryError::ListDir));
        },
    );
}

#[test]
fn the_oversize_error_reads_like_the_table_error() {
    let error = DirEntryError::Table(StringTableError::TooManyEntries { count: 65536 });
    assert_eq!(
        error.to_string(),
        "65536 entries exceed the 65535 a string table can address"
    );
}

// --- properties -----------------------------------------------------------

fn file_name() -> impl Strategy<Value = String> {
    prop_oneof!["[a-z0-9_][a-z0-9_.]{0,7}", "\\.[a-z0-9_][a-z0-9_.]{0,6}",]
}

fn matches(name: &str, pattern: &str, match_type: MatchType) -> bool {
    match match_type {
        MatchType::Literal => name == pattern,
        MatchType::Prefix => name.starts_with(pattern),
        MatchType::Suffix => name.ends_with(pattern),
        MatchType::Substring => name.contains(pattern),
        MatchType::Regex => unreachable!("regex is covered by explicit cases"),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    #[serial]
    fn scans_agree_with_a_model_of_the_selection_rules(
        files in prop::collection::btree_set(file_name(), 0..12),
        folders in prop::collection::btree_set(file_name(), 0..4),
        pattern in "[a-z0-9_.]{1,3}",
        match_type in prop_oneof![
            Just(MatchType::Literal),
            Just(MatchType::Prefix),
            Just(MatchType::Suffix),
            Just(MatchType::Substring),
        ],
    ) {
        let mut fixture = Fixture::new();
        let host = fixture.mount_dir(&[]);
        for name in &files {
            std::fs::write(host.join(name), b"data").unwrap();
        }
        for name in folders.difference(&files) {
            std::fs::create_dir(host.join(name)).unwrap();
        }

        // A BTreeSet iterates in byte order, which is the order promised.
        let expected: Vec<String> = files
            .iter()
            .filter(|name| !name.starts_with('.') && matches(name, &pattern, match_type))
            .cloned()
            .collect();
        prop_assert_eq!(fixture.scan("", &pattern, match_type), expected);
    }
}

// --- caller bugs abort, never unwind into C ----------------------------------

/// Runs the violation in a child process; see `abort_child`.
macro_rules! aborts {
    ($name:ident, $stderr:expr, |$fixture:ident| $violation:expr) => {
        #[test]
        fn $name() {
            crate::resource::abort_child::expect_abort(
                module_path!(),
                stringify!($name),
                $stderr,
                || {
                    let $fixture = Fixture::new();
                    // SAFETY: the violation passes a null or invalid argument
                    // on purpose; the entry point must abort before using it.
                    let _ = unsafe { $violation };
                },
            );
        }
    };
}

aborts!(
    a_null_directory_handle_aborts,
    "LoadDirEntryTable: directory handle is null",
    |_fixture| LoadDirEntryTable(
        ptr::null_mut(),
        c"".as_ptr(),
        c".mle".as_ptr(),
        MatchType::Suffix as c_int
    )
);
aborts!(
    a_null_path_aborts,
    "LoadDirEntryTable: path or pattern is null",
    |fixture| LoadDirEntryTable(
        fixture.root,
        ptr::null(),
        c".mle".as_ptr(),
        MatchType::Suffix as c_int
    )
);
aborts!(
    a_null_pattern_aborts,
    "LoadDirEntryTable: path or pattern is null",
    |fixture| LoadDirEntryTable(
        fixture.root,
        c"".as_ptr(),
        ptr::null(),
        MatchType::Suffix as c_int
    )
);
aborts!(
    an_unknown_match_type_aborts,
    "LoadDirEntryTable: unknown match type",
    |fixture| LoadDirEntryTable(fixture.root, c"".as_ptr(), c".mle".as_ptr(), 5)
);
aborts!(
    a_negative_match_type_aborts,
    "LoadDirEntryTable: unknown match type",
    |fixture| LoadDirEntryTable(fixture.root, c"".as_ptr(), c".mle".as_ptr(), -1)
);
