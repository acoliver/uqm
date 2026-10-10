//! Joint startup regression: the Rust-owned directory preparation exports run
//! in order against a staged HOME with real content, then the resource index
//! is read through the mounts they produced.

use std::ffi::{c_char, c_void, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

use super::c_dir_globals::{baseContentPath, contentDir, meleeDir, repository, saveDir};
use super::lifecycle_tests::read_through_uio;
use super::*;
use crate::io::dirs::test_support::{in_child, resolved_tempdir};
use crate::io::uio_bridge::{uio_init, uio_openRepository};
use crate::resource::ffi_bridge::{res_GetString, res_HasKey, LoadResourceIndex};

const INDEX: &[u8] = b"joint.greeting = STRING:hello\njoint.volume = INT32:7\n";

fn stage_content(home: &Path) -> PathBuf {
    let content = home.join("content");
    std::fs::create_dir_all(content.join("packages")).unwrap();
    std::fs::write(content.join("version"), b"0.8.0\n").unwrap();
    std::fs::write(content.join("uqm.rmp"), INDEX).unwrap();
    let mut zip =
        zip::ZipWriter::new(std::fs::File::create(content.join("packages/base.zip")).unwrap());
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("from_package.txt", stored).unwrap();
    std::io::Write::write_all(&mut zip, b"package data").unwrap();
    zip.finish().unwrap();
    content
}

fn cstring(text: &str) -> CString {
    CString::new(text).unwrap()
}

fn resource_string(key: &str) -> Option<String> {
    let key = cstring(key);
    let value = unsafe { res_GetString(key.as_ptr()) };
    (!value.is_null()).then(|| {
        unsafe { std::ffi::CStr::from_ptr(value) }
            .to_str()
            .unwrap()
            .to_owned()
    })
}

/// Config, content, save and melee preparation through the public C exports,
/// with the configured content directory written the way a user would write it.
#[test]
fn startup_exports_mount_config_content_save_and_teams_and_serve_the_index() {
    let (_guard, parent_home) = resolved_tempdir();
    let env: Vec<(&str, &[u8])> = vec![("HOME", parent_home.as_os_str().as_bytes())];
    in_child(
        module_path!(),
        "startup_exports_mount_config_content_save_and_teams_and_serve_the_index",
        &env,
        Some(&parent_home),
        || {
            let home = PathBuf::from(std::env::var_os("HOME").expect("the parent sets HOME"));
            unsafe {
                uio_init();
                repository = uio_openRepository(0) as *mut c_void;
            }
            let content = stage_content(&home);
            let index_before = std::fs::read(content.join("uqm.rmp")).unwrap();

            let content_arg = cstring("~/content/../content");
            let exec = cstring("uqm");
            assert_eq!(unsafe { rust_prepare_config_dir(ptr::null()) }, 0);
            assert_eq!(
                unsafe {
                    rust_prepare_content_dir(content_arg.as_ptr(), ptr::null(), exec.as_ptr())
                },
                0
            );
            assert_eq!(unsafe { rust_prepare_save_dir() }, 0);
            assert_eq!(unsafe { rust_prepare_melee_dir() }, 0);

            let recorded = unsafe {
                std::ffi::CStr::from_ptr(ptr::addr_of!(baseContentPath).cast::<c_char>())
            };
            assert_eq!(recorded.to_bytes(), content.as_os_str().as_bytes());
            assert!(home.join(".uqm/save").is_dir());
            assert!(home.join(".uqm/teams").is_dir());
            std::fs::write(home.join(".uqm/save/slot1.sav"), b"saved").unwrap();
            std::fs::write(home.join(".uqm/teams/fleet.mle"), b"fleet").unwrap();

            let content_handle = unsafe { contentDir };
            assert_eq!(
                read_through_uio(content_handle, "version").as_deref(),
                Some(&b"0.8.0\n"[..])
            );
            assert_eq!(
                read_through_uio(content_handle, "from_package.txt").as_deref(),
                Some(&b"package data"[..])
            );
            assert_eq!(
                read_through_uio(unsafe { saveDir }, "slot1.sav").as_deref(),
                Some(&b"saved"[..])
            );
            assert_eq!(
                read_through_uio(unsafe { meleeDir }, "fleet.mle").as_deref(),
                Some(&b"fleet"[..])
            );

            let index = cstring("uqm.rmp");
            unsafe { LoadResourceIndex(content_handle, index.as_ptr(), ptr::null()) };
            assert_eq!(resource_string("joint.greeting").as_deref(), Some("hello"));
            assert_ne!(unsafe { res_HasKey(cstring("joint.volume").as_ptr()) }, 0);
            assert_eq!(
                std::fs::read(content.join("uqm.rmp")).unwrap(),
                index_before
            );
        },
    );
}
