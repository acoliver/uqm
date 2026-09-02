//! Menu Binding Probe — Initialized-Child Production Query
//!
//! This probe performs the minimal real production initialization needed to
//! query the actual `menu.down.N` binding through production resources,
//! then calls `query_menu_binding` (which uses production `res_IsString`/
//! `res_GetString` and the production gesture parser), emits the resolved
//! VCONTROL_KEY binding and alternate id, and exits.
//!
//! It owns/reaps no child processes — it IS the initialized child.
//!
//! The resource system, resource index loader, UIO, and gesture parser are
//! Rust exports of this crate; the linked C archive still supplies the C
//! subsystem type registration (`InstallGraphicResTypes` and friends) that
//! `InitResourceSystem` drives, so this binary requires the
//! `linked_c_archive` feature.
//!
//! Usage: cargo run --features linked_c_archive --bin menu_binding_probe [content_path]
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P00

use std::ffi::{c_int, CString};
use std::ptr;

use uqm_rust::input::menu_binding::query_menu_binding;
use uqm_rust::io::uio_bridge::{
    uio_DirHandle, uio_closeDir, uio_closeRepository, uio_mountDir, uio_openDir, uio_openRepository,
};
use uqm_rust::resource::ffi_bridge::{InitResourceSystem, LoadResourceIndex};

/// Standard IO filesystem type (matches `uio_FSTYPE_STDIO` in fstypes.h).
const PROBE_FSTYPE_STDIO: c_int = 1;
/// `uio_MOUNT_RDONLY` (matches libs/uio/mount.h).
const PROBE_MOUNT_RDONLY: c_int = 1 << 1;
/// `uio_MOUNT_TOP` (matches libs/uio/mount.h).
const PROBE_MOUNT_TOP: c_int = 1 << 2;

/// Default content path relative to the repository root when the caller
/// does not supply one, matching the historical probe default.
const DEFAULT_CONTENT_PATH: &str = "../../sc2/content";

fn main() {
    let content_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CONTENT_PATH.to_string());

    eprintln!("menu_binding_probe: content_path={content_path}");

    // 1. Initialize the production resource system. This drives C subsystem
    //    type registration (InstallGraphicResTypes, InstallStringTableResType,
    //    etc.) from the linked C archive.
    // SAFETY: InitResourceSystem takes no pointer arguments.
    let res_idx = unsafe { InitResourceSystem() };
    if res_idx.is_null() {
        eprintln!("FAIL: InitResourceSystem returned NULL");
        println!("RESULT=FAIL reason=init_resource_system");
        std::process::exit(1);
    }
    eprintln!("menu_binding_probe: resource system initialized");

    // 2. Create a UIO repository and mount the content directory.
    // SAFETY: uio_openRepository takes no pointer arguments.
    let repo = unsafe { uio_openRepository(0) };
    if repo.is_null() {
        eprintln!("FAIL: uio_openRepository returned NULL");
        println!("RESULT=FAIL reason=open_repository");
        std::process::exit(1);
    }

    let content_path_c = match CString::new(content_path.as_str()) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("FAIL: content path is not a valid C string: {error}");
            println!("RESULT=FAIL reason=mount_content_dir");
            // SAFETY: repo came from uio_openRepository above and has not
            // been closed.
            unsafe { uio_closeRepository(repo) };
            std::process::exit(1);
        }
    };

    let root = c"/".as_ptr();
    // SAFETY: repo and root are valid; source_dir, auto_mount, and relative
    // are intentionally null; content_path_c is a valid NUL-terminated C
    // string that outlives the call.
    let mount = unsafe {
        uio_mountDir(
            repo,
            root,
            PROBE_FSTYPE_STDIO,
            ptr::null_mut(),
            ptr::null(),
            content_path_c.as_ptr(),
            ptr::null_mut(),
            PROBE_MOUNT_TOP | PROBE_MOUNT_RDONLY,
            ptr::null_mut(),
        )
    };
    if mount.is_null() {
        eprintln!("FAIL: uio_mountDir returned NULL for {content_path}");
        println!("RESULT=FAIL reason=mount_content_dir");
        // SAFETY: repo came from uio_openRepository above and has not been
        // closed.
        unsafe { uio_closeRepository(repo) };
        std::process::exit(1);
    }
    eprintln!("menu_binding_probe: content dir mounted");

    // 3. Open the root directory handle.
    // SAFETY: repo is valid and root is a valid NUL-terminated C string.
    let content_dir: *mut uio_DirHandle = unsafe { uio_openDir(repo, root, 0) };
    if content_dir.is_null() {
        eprintln!("FAIL: uio_openDir returned NULL");
        println!("RESULT=FAIL reason=open_content_dir");
        // SAFETY: repo came from uio_openRepository above and has not been
        // closed.
        unsafe { uio_closeRepository(repo) };
        std::process::exit(1);
    }
    eprintln!("menu_binding_probe: content dir opened");

    // 4. Load the menu.key resource index with "menu." prefix. This is the
    //    same call as register_menu_controls/initKeyConfig in input.c.
    // SAFETY: content_dir is a live directory handle from uio_openDir;
    // index_name and index_prefix are static NUL-terminated C strings.
    unsafe {
        LoadResourceIndex(content_dir.cast(), c"menu.key".as_ptr(), c"menu.".as_ptr());
    };
    eprintln!("menu_binding_probe: menu.key loaded");

    // 5. Query the actual menu.down binding through the Rust accessor, which
    //    resolves the binding through production res_IsString/res_GetString
    //    and the production gesture parser.
    let result = query_menu_binding("down");

    // 6. Validate and emit the result.
    println!("menu_binding_query=down");
    println!("found={}", i32::from(result.found));
    println!("key_code={}", result.key_code);
    println!("binding_id={}", result.binding_id);
    println!("num_alternates={}", result.num_alternates);

    if !result.found {
        println!("RESULT=FAIL reason=no_key_binding_found");
        // SAFETY: both handles are live and have not been closed.
        unsafe {
            uio_closeDir(content_dir);
            uio_closeRepository(repo);
        }
        std::process::exit(1);
    }

    // The accessor already filters for VCONTROL_KEY; double-check the key
    // code is a valid SDL keycode.
    if result.key_code <= 0 {
        println!("RESULT=FAIL reason=invalid_key_code");
        // SAFETY: both handles are live and have not been closed.
        unsafe {
            uio_closeDir(content_dir);
            uio_closeRepository(repo);
        }
        std::process::exit(1);
    }

    println!("RESULT=PASS");
    println!("binding_type=VCONTROL_KEY");

    // 7. Teardown.
    // SAFETY: both handles are live and have not been closed.
    unsafe {
        uio_closeDir(content_dir);
        uio_closeRepository(repo);
    }
}
