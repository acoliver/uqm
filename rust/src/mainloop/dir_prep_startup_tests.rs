//! Fixture rows S* and U* (tmp/issue38-port/dirs-startup/fixture-spec.md):
//! the Rust-owned startup resolves configured directories through the
//! `io::dirs` expansion, keeps raw path bytes, and reports failure as `-1`.
//!
//! Every row runs in a re-executed copy of this test binary whose environment
//! is exactly the one the row lists, so the parent process environment is
//! never touched. Rows that make directories use real temporary directories
//! and a private uio repository; no game process is involved.

use std::ffi::{c_void, CString, OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;

use super::c_dir_globals::{baseContentPath, configDir, contentDir, meleeDir, repository, saveDir};
use super::lifecycle_tests::read_through_uio;
use super::*;
use crate::io::dirs::test_support::{in_child, resolved_tempdir};
use crate::io::uio_bridge::{uio_init, uio_openRepository};

fn open_repository() {
    unsafe {
        uio_init();
        repository = uio_openRepository(0) as *mut c_void;
    }
}

/// Runs `body` in a child whose HOME and working directory are a real
/// temporary directory (created by the parent) and whose other variables are
/// `extra`. The child opens a fresh uio repository first.
fn in_home_child(module: &str, name: &str, extra: &[(&str, &[u8])], body: impl FnOnce(&Path)) {
    let (_guard, parent_home) = resolved_tempdir();
    let mut env: Vec<(&str, &[u8])> = vec![("HOME", parent_home.as_os_str().as_bytes())];
    env.extend_from_slice(extra);
    in_child(module, name, &env, Some(&parent_home), || {
        let home = PathBuf::from(std::env::var_os("HOME").expect("the parent sets HOME"));
        open_repository();
        body(&home);
    });
}

macro_rules! startup_row {
    ($name:ident, $extra:expr, $body:expr) => {
        #[test]
        fn $name() {
            in_home_child(module_path!(), stringify!($name), $extra, $body);
        }
    };
}

macro_rules! expand_row {
    ($name:ident, $env:expr, $body:expr) => {
        #[test]
        fn $name() {
            in_child(module_path!(), stringify!($name), $env, None, $body);
        }
    };
}

fn c(bytes: &[u8]) -> CString {
    CString::new(bytes).unwrap()
}

fn joined(home: &Path, tail: &str) -> OsString {
    let mut path = home.as_os_str().to_os_string();
    path.push(tail);
    path
}

fn long_absolute_path(len: usize) -> PathBuf {
    let mut bytes = vec![b'/'];
    bytes.extend(std::iter::repeat_n(b'a', len - 1));
    PathBuf::from(OsString::from_vec(bytes))
}

/// The expansion as exact bytes: `Path` equality would hide `//` runs.
fn expand(arg: &[u8]) -> Result<OsString, i32> {
    expand_startup_path(Path::new(OsStr::from_bytes(arg)), UNBOUNDED)
        .map(PathBuf::into_os_string)
        .map_err(errno_of)
}

fn errno_of(error: DirPrepError) -> i32 {
    match error {
        DirPrepError::InvalidPath { source, .. } | DirPrepError::Io(source) => source
            .raw_os_error()
            .expect("expansion errors carry an errno"),
        other => panic!("expected an errno-carrying error, got {other:?}"),
    }
}

fn config_with(arg: &[u8]) -> c_int {
    let arg = c(arg);
    unsafe { rust_prepare_config_dir(arg.as_ptr()) }
}

fn entries(dir: &Path) -> Vec<OsString> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect()
}

fn base_content_path_bytes() -> Vec<u8> {
    unsafe { CStr::from_ptr(ptr::addr_of!(baseContentPath).cast()) }
        .to_bytes()
        .to_vec()
}

// ---- S: startup rows ------------------------------------------------------

startup_row!(
    s01_tilde_default_style_config_dir_is_created_and_mounted,
    &[],
    |home| {
        assert_eq!(config_with(b"~/.uqm/"), 0);
        assert!(home.join(".uqm").is_dir());
        assert_eq!(
            std::env::var_os("UQM_CONFIG_DIR").unwrap(),
            joined(home, "/.uqm/")
        );
        std::fs::write(home.join(".uqm/marker"), b"config marker").unwrap();
        let seen = read_through_uio(unsafe { configDir }, "marker");
        assert_eq!(seen.as_deref(), Some(&b"config marker"[..]));
    }
);

startup_row!(
    s02_bare_tilde_config_dir_is_refused_without_side_effects,
    &[],
    |home| {
        assert_eq!(config_with(b"~"), -1);
        assert!(entries(home).is_empty(), "{:?}", entries(home));
        assert!(unsafe { configDir.is_null() });
        assert!(std::env::var_os("UQM_CONFIG_DIR").is_none());
    }
);

startup_row!(
    s03_dot_dot_in_the_config_dir_is_resolved_before_anything_is_made,
    &[],
    |home| {
        assert_eq!(config_with(&joined(home, "/a/../cfg").into_vec()), 0);
        assert_eq!(
            std::env::var_os("UQM_CONFIG_DIR").unwrap(),
            joined(home, "/cfg")
        );
        assert!(home.join("cfg").is_dir());
        assert!(!home.join("a").exists(), "the .. detour was created");
    }
);

startup_row!(
    s04_repeated_separators_in_the_config_dir_collapse,
    &[],
    |home| {
        assert_eq!(config_with(&joined(home, "//cfg///x").into_vec()), 0);
        assert_eq!(
            std::env::var_os("UQM_CONFIG_DIR").unwrap(),
            joined(home, "/cfg/x")
        );
        assert!(home.join("cfg/x").is_dir());
    }
);

startup_row!(
    s05_config_dir_that_is_not_utf8_keeps_its_bytes,
    &[],
    |home| {
        let mut arg = joined(home, "/caf").into_vec();
        arg.push(0xff);
        let status = config_with(&arg);
        if cfg!(target_os = "macos") {
            // APFS refuses file names that are not UTF-8: a clean -1, and no
            // entry with U+FFFD standing in for the byte.
            assert_eq!(status, -1);
            assert!(entries(home).is_empty(), "{:?}", entries(home));
        } else {
            assert_eq!(status, 0);
            let names = entries(home);
            assert_eq!(names.len(), 1);
            assert_eq!(names[0].as_bytes(), b"caf\xff");
            assert_eq!(std::env::var_os("UQM_CONFIG_DIR").unwrap().into_vec(), arg);
        }
    }
);

startup_row!(
    s06_save_dir_override_with_dot_dot_is_resolved,
    &[("UQM_SAVE_DIR", b"${UQM_CONFIG_DIR}/s/../custom")],
    |home| {
        assert_eq!(config_with(&joined(home, "/cfg").into_vec()), 0);
        assert_eq!(unsafe { rust_prepare_save_dir() }, 0);
        assert_eq!(
            std::env::var_os("UQM_SAVE_DIR").unwrap(),
            joined(home, "/cfg/custom")
        );
        assert!(home.join("cfg/custom").is_dir());
        assert!(!home.join("cfg/s").exists(), "the .. detour was created");
        std::fs::write(home.join("cfg/custom/slot1.sav"), b"saved").unwrap();
        let seen = read_through_uio(unsafe { saveDir }, "slot1.sav");
        assert_eq!(seen.as_deref(), Some(&b"saved"[..]));
    }
);

startup_row!(s07_default_save_dir_layout_is_unchanged, &[], |home| {
    assert_eq!(config_with(&joined(home, "/cfg").into_vec()), 0);
    assert_eq!(unsafe { rust_prepare_save_dir() }, 0);
    assert!(home.join("cfg/save").is_dir());
    std::fs::write(home.join("cfg/save/slot1.sav"), b"saved").unwrap();
    let seen = read_through_uio(unsafe { saveDir }, "slot1.sav");
    assert_eq!(seen.as_deref(), Some(&b"saved"[..]));
});

startup_row!(
    s08_content_dir_is_resolved_into_base_content_path,
    &[],
    |home| {
        std::fs::create_dir_all(home.join("content")).unwrap();
        std::fs::write(home.join("content/version"), b"0.8.0\n").unwrap();
        let arg = c(&joined(home, "/content/../content").into_vec());
        let exec = c(b"uqm");

        let status = unsafe { rust_prepare_content_dir(arg.as_ptr(), ptr::null(), exec.as_ptr()) };

        assert_eq!(status, 0);
        assert_eq!(
            base_content_path_bytes(),
            joined(home, "/content").into_vec()
        );
        let seen = read_through_uio(unsafe { contentDir }, "version");
        assert_eq!(seen.as_deref(), Some(&b"0.8.0\n"[..]));
    }
);

startup_row!(s09_bare_tilde_content_dir_is_refused, &[], |_home| {
    let arg = c(b"~");
    let exec = c(b"uqm");

    let status = unsafe { rust_prepare_content_dir(arg.as_ptr(), ptr::null(), exec.as_ptr()) };

    assert_eq!(status, -1);
    assert!(base_content_path_bytes().is_empty());
    assert!(unsafe { contentDir.is_null() });
});

startup_row!(
    s10_content_dir_without_a_version_file_is_refused,
    &[],
    |home| {
        let bare = home.join("bare");
        std::fs::create_dir(&bare).unwrap();
        let arg = c(bare.as_os_str().as_bytes());
        let exec = c(b"uqm");

        let status = unsafe { rust_prepare_content_dir(arg.as_ptr(), ptr::null(), exec.as_ptr()) };

        assert_eq!(status, -1);
        assert!(unsafe { contentDir.is_null() });
    }
);

startup_row!(
    s11_melee_dir_override_with_dot_dot_is_resolved,
    &[("UQM_MELEE_DIR", b"${UQM_CONFIG_DIR}/t/../m")],
    |home| {
        assert_eq!(config_with(&joined(home, "/cfg").into_vec()), 0);
        assert_eq!(unsafe { rust_prepare_melee_dir() }, 0);
        assert!(home.join("cfg/m").is_dir());
        assert!(!home.join("cfg/t").exists(), "the .. detour was created");
        std::fs::write(home.join("cfg/m/mine.mle"), b"team").unwrap();
        let seen = read_through_uio(unsafe { meleeDir }, "mine.mle");
        assert_eq!(seen.as_deref(), Some(&b"team"[..]));
    }
);

/// `<home>/content` followed by `/../content` repeated until the raw argument
/// is longer than `PATH_MAX`, while its lexical result stays `<home>/content`.
fn dotted_path_beyond_path_max(home: &Path) -> Vec<u8> {
    let mut arg = joined(home, "/content").into_vec();
    while arg.len() <= PATH_MAX {
        arg.extend_from_slice(b"/../content");
    }
    arg
}

startup_row!(
    s12_content_dir_longer_than_the_buffer_is_refused_not_truncated,
    &[],
    |home| {
        std::fs::create_dir_all(home.join("content")).unwrap();
        std::fs::write(home.join("content/version"), b"0.8.0\n").unwrap();
        let arg = c(&dotted_path_beyond_path_max(home));
        let exec = c(b"uqm");

        let status = unsafe { rust_prepare_content_dir(arg.as_ptr(), ptr::null(), exec.as_ptr()) };

        assert_eq!(status, -1);
        assert!(base_content_path_bytes().is_empty());
        assert!(unsafe { contentDir.is_null() });
    }
);

startup_row!(
    s13_config_dir_longer_than_path_max_has_no_artificial_cap,
    &[],
    |home| {
        assert_eq!(config_with(&dotted_path_beyond_path_max(home)), 0);
        assert_eq!(
            std::env::var_os("UQM_CONFIG_DIR").unwrap(),
            joined(home, "/content")
        );
        assert!(home.join("content").is_dir());
    }
);

// ---- U: expansion rows ----------------------------------------------------

expand_row!(
    u01_home_prefix_expands,
    &[("HOME", b"/tmp/testhome")],
    || {
        assert_eq!(expand(b"~/foo"), Ok(OsString::from("/tmp/testhome/foo")));
    }
);

expand_row!(
    u02_bare_tilde_is_einval,
    &[("HOME", b"/tmp/testhome")],
    || {
        assert_eq!(expand(b"~"), Err(libc::EINVAL));
    }
);

expand_row!(u03_absolute_path_is_unchanged, &[], || {
    assert_eq!(expand(b"/usr/local"), Ok(OsString::from("/usr/local")));
});

expand_row!(
    u04_variable_forms_expand,
    &[("MY_TEST_VAR", b"/opt/data")],
    || {
        assert_eq!(
            expand(b"${MY_TEST_VAR}/sub"),
            Ok(OsString::from("/opt/data/sub"))
        );
        assert_eq!(
            expand(b"$MY_TEST_VAR/sub"),
            Ok(OsString::from("/opt/data/sub"))
        );
    }
);

expand_row!(u05_unset_variable_expands_to_nothing, &[], || {
    assert_eq!(
        expand(b"$UNDEFINED_UQM_VAR/path"),
        Ok(OsString::from("/path"))
    );
});

startup_row!(
    u06_relative_path_is_made_absolute_with_the_working_directory,
    &[],
    |home| {
        assert_eq!(
            expand(b"foo/bar"),
            Ok(home.join("foo/bar").into_os_string())
        );
    }
);

expand_row!(
    u07_config_dir_variable_expands,
    &[("UQM_CONFIG_DIR", b"/home/user/.uqm")],
    || {
        assert_eq!(
            expand(b"${UQM_CONFIG_DIR}/save"),
            Ok(OsString::from("/home/user/.uqm/save"))
        );
    }
);

expand_row!(
    u08_adjacent_variables_collapse_separators,
    &[("A", b"/a"), ("B", b"/b")],
    || {
        assert_eq!(expand(b"$A/${B}/end"), Ok(OsString::from("/a/b/end")));
    }
);

expand_row!(
    u09_non_ascii_characters_survive,
    &[("UQM_CONFIG_DIR", "/opt/café".as_bytes())],
    || {
        assert_eq!(
            expand("${UQM_CONFIG_DIR}/sauvegarde-é/日本".as_bytes()),
            Ok(OsString::from("/opt/café/sauvegarde-é/日本"))
        );
    }
);

startup_row!(u10_dollar_without_a_name_expands_to_nothing, &[], |home| {
    assert_eq!(expand(b"cost$"), Ok(home.join("cost").into_os_string()));
    assert_eq!(expand(b"a$1b"), Ok(home.join("a").into_os_string()));
    assert_eq!(expand(b"$$"), Ok(joined(home, "/")));
});

expand_row!(u11_unterminated_brace_is_einval, &[], || {
    assert_eq!(expand(b"${unclosed"), Err(libc::EINVAL));
});

expand_row!(u12_tilde_user_is_einval, &[("HOME", b"/h")], || {
    assert_eq!(expand(b"~other/x"), Err(libc::EINVAL));
});

startup_row!(u13_only_a_leading_tilde_expands, &[], |home| {
    assert_eq!(expand(b"a/~/x"), Ok(home.join("a/~/x").into_os_string()));
});

expand_row!(u14_home_falls_back_to_the_passwd_entry, &[], || {
    let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
    let mut found: *mut libc::passwd = ptr::null_mut();
    let mut buffer = vec![0 as libc::c_char; 4096];
    let rc = unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            &mut entry,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut found,
        )
    };
    assert_eq!(rc, 0);
    assert!(!found.is_null(), "the test user has a passwd entry");
    let mut expected = unsafe { CStr::from_ptr(entry.pw_dir) }.to_bytes().to_vec();
    expected.extend_from_slice(b"/x");
    assert_eq!(expand(b"~/x"), Ok(OsString::from_vec(expected)));
});

// The empty HOME entry comes after the helper's temporary HOME, so it wins;
// the working directory is still the temporary directory.
startup_row!(
    u15_empty_home_wins_and_is_relative,
    &[("HOME", b"")],
    |_home| {
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(expand(b"~/x"), Ok(cwd.join("x").into_os_string()));
    }
);

expand_row!(
    u16_non_utf8_home_and_variables_keep_their_bytes,
    &[("HOME", b"/h\xff"), ("B", b"/b\xfe")],
    || {
        assert_eq!(expand(b"~/x").unwrap().as_bytes(), b"/h\xff/x");
        assert_eq!(expand(b"$B/y").unwrap().as_bytes(), b"/b\xfe/y");
    }
);

startup_row!(
    u17_a_working_directory_that_cannot_be_read_is_an_error,
    &[],
    |home| {
        std::fs::remove_dir(home).unwrap();
        assert_eq!(expand(b"x/y"), Err(libc::ENOENT));
    }
);

#[test]
fn u18_a_long_destination_has_no_artificial_cap() {
    let long = long_absolute_path(5000);
    assert_eq!(
        expand(long.as_os_str().as_bytes()).unwrap(),
        long.into_os_string()
    );
}

#[test]
fn u19_the_content_bound_is_the_base_content_path_buffer() {
    let fits = expand_startup_path(&long_absolute_path(PATH_MAX - 1), PATH_MAX).unwrap();
    assert_eq!(fits.as_os_str().len(), PATH_MAX - 1);
    let too_long = expand_startup_path(&long_absolute_path(PATH_MAX), PATH_MAX).map_err(errno_of);
    assert_eq!(too_long, Err(libc::ENAMETOOLONG));
}

expand_row!(
    u20_base_content_path_refuses_what_does_not_fit_with_its_nul,
    &[],
    || {
        store_base_content_path(&long_absolute_path(PATH_MAX - 1)).unwrap();
        assert_eq!(base_content_path_bytes().len(), PATH_MAX - 1);

        let refused = store_base_content_path(&long_absolute_path(PATH_MAX)).map_err(errno_of);

        assert_eq!(refused, Err(libc::ENAMETOOLONG));
        assert_eq!(
            base_content_path_bytes().len(),
            PATH_MAX - 1,
            "buffer untouched"
        );
    }
);
