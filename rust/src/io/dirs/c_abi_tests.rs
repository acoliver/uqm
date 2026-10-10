//! Fixture rows X*, H* of the spec: the plain `extern "C"` wrappers called
//! the way `options.c` calls the C functions. Rows that read the process
//! environment run in a child with an explicit environment.

use std::ffi::{c_char, c_int, CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use super::c_abi::*;
use super::expand::{EP_ABSOLUTE, EP_ALL_SYSTEM, EP_HOME};
use super::test_support::{in_child, resolved_tempdir};
use crate::resource::abort_child::expect_abort;

const CANARY: u8 = 0xAA;

fn last_errno() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap()
}

/// Calls `expand_path` with `len` of a `len + 16` byte buffer full of canaries.
fn call_expand(src: &[u8], len: usize, what: c_int) -> (c_int, c_int, Vec<u8>) {
    let src = CString::new(src).unwrap();
    let mut buf = vec![CANARY; len + 16];
    // SAFETY: `buf` has at least `len` writable bytes and `src` is a C string.
    let status = unsafe { expand_path(buf.as_mut_ptr().cast(), len, src.as_ptr(), what) };
    (status, last_errno(), buf)
}

#[test]
fn expand_fills_exactly_the_caller_buffer_and_stops_at_its_size() {
    let (status, _, buf) = call_expand(b"/abc", 5, 0); // X01, len = strlen + 1
    assert_eq!(status, 0);
    assert_eq!(&buf[..5], b"/abc\0");
    assert!(buf[5..].iter().all(|byte| *byte == CANARY));

    let (status, errno, buf) = call_expand(b"/abc", 4, 0); // len = strlen
    assert_eq!((status, errno), (-1, libc::ENAMETOOLONG));
    assert!(
        buf.iter().all(|byte| *byte == CANARY),
        "dest stays untouched on failure"
    );

    let (status, errno, buf) = call_expand(b"/abc", 0, 0);
    assert_eq!((status, errno), (-1, libc::ENAMETOOLONG));
    assert!(buf.iter().all(|byte| *byte == CANARY));
}

#[test]
fn expand_reports_contract_errno() {
    let (status, errno, _) = call_expand(b"~x", 64, EP_HOME); // X04
    assert_eq!((status, errno), (-1, libc::EINVAL));
    let (status, errno, _) = call_expand(b"a", 1, EP_ABSOLUTE);
    assert_eq!((status, errno), (-1, libc::ERANGE));
}

#[test]
fn bytes_that_are_not_utf8_pass_through_the_wrapper() {
    let (status, _, buf) = call_expand(b"/a/\xff\xfe", 16, 0);
    assert_eq!(status, 0);
    assert_eq!(&buf[..8], b"/a/\xff\xfe\0\xAA\xAA");
}

#[test]
fn environment_variables_come_from_the_real_process_environment() {
    in_child(
        module_path!(),
        "environment_variables_come_from_the_real_process_environment",
        &[("UQM_DIRS_B", b"\xff\xfe"), ("HOME", b"/home/x")],
        None,
        || {
            let (status, _, buf) = call_expand(b"$UQM_DIRS_B/${UQM_DIRS_B}/~", 64, 4);
            assert_eq!(status, 0);
            let text = CStr::from_bytes_until_nul(&buf).unwrap();
            assert_eq!(text.to_bytes(), b"\xff\xfe/\xff\xfe/~");
            let (status, _, buf) = call_expand(b"~/.uqm/", 64, EP_ALL_SYSTEM);
            assert_eq!(status, 0);
            assert_eq!(
                CStr::from_bytes_until_nul(&buf).unwrap().to_bytes(),
                b"/home/x/.uqm/"
            );
        },
    );
}

#[test]
fn options_c_directory_preparation_sequence() {
    let (_home_guard, home) = resolved_tempdir();
    let (_cwd_guard, cwd) = resolved_tempdir();
    let config = format!("{}/.uqm", home.display());
    let in_parent = in_child(
        module_path!(),
        "options_c_directory_preparation_sequence",
        &[
            ("HOME", home.as_os_str().as_bytes()),
            ("UQM_CONFIG_DIR", config.as_bytes()),
        ],
        Some(&cwd),
        || {
            let max = libc::PATH_MAX as usize - 13;
            // prepareConfigDir: expand CONFIGDIR "~/.uqm/", then mkdirhier.
            let (status, _, buf) = call_expand(b"~/.uqm/", max, EP_ALL_SYSTEM); // X02
            assert_eq!(status, 0);
            let config_dir = CStr::from_bytes_until_nul(&buf).unwrap().to_owned();
            assert_eq!(
                config_dir.to_bytes(),
                format!("{}/.uqm/", std::env::var("HOME").unwrap()).as_bytes()
            );
            assert_eq!(unsafe { mkdirhier(config_dir.as_ptr()) }, 0); // X03
            assert!(Path::new(std::ffi::OsStr::from_bytes(config_dir.to_bytes())).is_dir());

            // prepareSaveDir: SAVEDIR refers to the config dir variable.
            let (status, _, buf) = call_expand(b"${UQM_CONFIG_DIR}/save", max, EP_ALL_SYSTEM);
            assert_eq!(status, 0);
            let save_dir = CStr::from_bytes_until_nul(&buf).unwrap().to_owned();
            assert_eq!(unsafe { mkdirhier(save_dir.as_ptr()) }, 0);
            assert!(Path::new(std::ffi::OsStr::from_bytes(save_dir.to_bytes())).is_dir());

            // prepareContentDir: relative content path into baseContentPath[4096].
            let (status, _, buf) = call_expand(b"../content", 4096, EP_ALL_SYSTEM);
            assert_eq!(status, 0);
            let parent = std::env::current_dir()
                .unwrap()
                .parent()
                .unwrap()
                .join("content");
            assert_eq!(
                CStr::from_bytes_until_nul(&buf).unwrap().to_bytes(),
                parent.as_os_str().as_bytes()
            );
        },
    );
    if in_parent {
        assert!(home.join(".uqm/save").is_dir());
    }
}

fn home_dir_case(name: &'static str, env: &'static [(&'static str, &'static [u8])]) {
    in_child(module_path!(), name, env, None, || {
        let pointer = unsafe { get_home_dir() };
        let home = std::env::var_os("HOME");
        match home {
            Some(value) => {
                // Same storage as getenv: the caller must not free it.
                let env_pointer = unsafe { libc::getenv(c"HOME".as_ptr()) };
                assert_eq!(pointer, env_pointer.cast_const());
                assert_eq!(
                    unsafe { CStr::from_ptr(pointer) }.to_bytes(),
                    value.as_bytes()
                );
            }
            None => {
                // Oracle: an independent getpwuid_r lookup.
                assert_eq!(unsafe { CStr::from_ptr(pointer) }.to_bytes(), passwd_dir());
            }
        }
    });
}

fn passwd_dir() -> Vec<u8> {
    let mut record: libc::passwd = unsafe { std::mem::zeroed() };
    let mut result: *mut libc::passwd = std::ptr::null_mut();
    let mut storage = vec![0 as c_char; 4096];
    let rc = unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            &mut record,
            storage.as_mut_ptr(),
            storage.len(),
            &mut result,
        )
    };
    assert_eq!(rc, 0);
    assert!(!result.is_null(), "this account has a passwd entry");
    unsafe { CStr::from_ptr(record.pw_dir) }.to_bytes().to_vec()
}

#[test]
fn home_dir_is_the_home_variable() {
    home_dir_case("home_dir_is_the_home_variable", &[("HOME", b"/x")]); // H01
}

#[test]
fn home_dir_prefers_an_empty_home_variable_over_passwd() {
    in_child(
        module_path!(),
        "home_dir_prefers_an_empty_home_variable_over_passwd",
        &[("HOME", b"")],
        None,
        || {
            let pointer = unsafe { get_home_dir() }; // H02
            assert!(!pointer.is_null());
            assert_eq!(unsafe { CStr::from_ptr(pointer) }.to_bytes(), b"");
        },
    );
}

#[test]
fn home_dir_falls_back_to_the_passwd_database() {
    home_dir_case("home_dir_falls_back_to_the_passwd_database", &[]); // H03
}

#[test]
fn home_dir_keeps_non_utf8_bytes() {
    home_dir_case("home_dir_keeps_non_utf8_bytes", &[("HOME", b"/h\xff")]); // H04
}

#[test]
fn create_directory_and_mkdirhier_wrappers_set_errno() {
    let (_guard, root) = resolved_tempdir();
    let path = CString::new(root.join("new").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { create_directory(path.as_ptr(), 0o755) }, 0);
    assert_eq!(unsafe { create_directory(path.as_ptr(), 0o755) }, -1);
    assert_eq!(last_errno(), libc::EEXIST);
    let nested = CString::new(root.join("a/b").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { mkdirhier(nested.as_ptr()) }, 0);
    let file = root.join("file");
    std::fs::write(&file, b"").unwrap();
    let below = CString::new(file.join("x").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { mkdirhier(below.as_ptr()) }, -1);
    assert_eq!(last_errno(), libc::ENOTDIR);
}

#[test]
fn expand_path_without_a_destination_aborts() {
    expect_abort(
        module_path!(),
        "expand_path_without_a_destination_aborts",
        "expandPath() FATAL: caller bug: dest is null",
        || {
            let src = CString::new("/a").unwrap();
            unsafe { expand_path(std::ptr::null_mut(), 8, src.as_ptr(), 0) };
        },
    );
}

#[test]
fn expand_path_without_a_source_aborts() {
    expect_abort(
        module_path!(),
        "expand_path_without_a_source_aborts",
        "expandPath() FATAL: caller bug: src is null",
        || {
            let mut buf = [0 as c_char; 8];
            unsafe { expand_path(buf.as_mut_ptr(), 8, std::ptr::null(), 0) };
        },
    );
}

#[test]
fn mkdirhier_without_a_path_aborts() {
    expect_abort(
        module_path!(),
        "mkdirhier_without_a_path_aborts",
        "mkdirhier() FATAL: caller bug: path is null",
        || {
            unsafe { mkdirhier(std::ptr::null()) };
        },
    );
}

#[test]
fn create_directory_without_a_path_aborts() {
    expect_abort(
        module_path!(),
        "create_directory_without_a_path_aborts",
        "createDirectory() FATAL: caller bug: dir is null",
        || {
            unsafe { create_directory(std::ptr::null(), 0o755) };
        },
    );
}
