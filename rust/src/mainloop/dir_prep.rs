//! Directory preparation for the UQM init sequence.
//!
//! Rust port of `options.c`'s `prepareConfigDir`, `prepareContentDir`,
//! `prepareMeleeDir`, `prepareSaveDir`, and `prepareShadowAddons`.
//! Path resolution, env-var expansion, and directory creation happen in
//! pure Rust; uio mounting calls into `crate::io::uio_bridge` (already
//! Rust-implemented), and C-side globals are set via bridge accessors.

use std::ffi::{CStr, CString, OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;

use libc::{c_char, c_int};

use crate::io::dirs::expand::{expand_path, EP_ALL_SYSTEM};
use crate::io::dirs::mkdirhier::mkdirhier;
use crate::io::dirs::process_env::ProcessEnvironment;
use crate::io::uio_bridge::{uio_DirHandle, uio_MountHandle, uio_Repository};

// ---------------------------------------------------------------------------
// Constants matching C headers
// ---------------------------------------------------------------------------

const UIO_FSTYPE_STDIO: c_int = 1;
const UIO_FSTYPE_ZIP: c_int = 2;

const UIO_MOUNT_RDONLY: c_int = 1 << 1; // 2
const UIO_MOUNT_TOP: c_int = 1 << 2; // 4
const UIO_MOUNT_BELOW: c_int = 2 << 2; // 8
const UIO_MOUNT_ABOVE: c_int = 3 << 2; // 12

const MATCH_PREFIX: c_int = 1;
const MATCH_REGEX: c_int = 4;

const CONFIGDIR_DEFAULT: &str = "~/.uqm/";

/// Bound for destinations that live in a `PathBuf`: nothing to overflow.
const UNBOUNDED: usize = usize::MAX;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum DirPrepError {
    Io(std::io::Error),
    /// A configured path that cannot be expanded or does not fit its consumer.
    InvalidPath {
        path: PathBuf,
        source: std::io::Error,
    },
    NotFound(String),
    MountFailed(String),
}

impl From<std::io::Error> for DirPrepError {
    fn from(e: std::io::Error) -> Self {
        DirPrepError::Io(e)
    }
}

// ---------------------------------------------------------------------------
// Path resolution: the `io::dirs` port of C `expandPath`
// ---------------------------------------------------------------------------

/// Expands a configured directory exactly as C `expandPath(buf, len, path,
/// EP_ALL_SYSTEM)` does, against the real process environment (`getenv`,
/// `getHomeDir`, `getcwd`), and keeps the raw bytes of the result.
///
/// `len` is the size of the buffer the result goes into; a destination that
/// is only a `PathBuf` passes [`UNBOUNDED`]. Failures carry the errno C sets.
pub(crate) fn expand_startup_path(input: &Path, len: usize) -> Result<PathBuf, DirPrepError> {
    expand_path(
        &ProcessEnvironment,
        input.as_os_str().as_bytes(),
        len,
        EP_ALL_SYSTEM,
    )
    .map(|bytes| PathBuf::from(OsString::from_vec(bytes)))
    .map_err(|source| DirPrepError::InvalidPath {
        path: input.to_path_buf(),
        source,
    })
}

// ---------------------------------------------------------------------------
// Direct C global access for directory handles (options.h/setup.c)
// ---------------------------------------------------------------------------

#[cfg(not(test))]
extern "C" {
    #[link_name = "repository"]
    #[allow(non_upper_case_globals)]
    pub static mut repository: *mut std::ffi::c_void;
    #[link_name = "configDir"]
    #[allow(non_upper_case_globals)]
    pub static mut configDir: *mut std::ffi::c_void;
    #[link_name = "contentDir"]
    #[allow(non_upper_case_globals)]
    pub static mut contentDir: *mut std::ffi::c_void;
    #[link_name = "saveDir"]
    #[allow(non_upper_case_globals)]
    pub static mut saveDir: *mut std::ffi::c_void;
    #[link_name = "meleeDir"]
    #[allow(non_upper_case_globals)]
    pub static mut meleeDir: *mut std::ffi::c_void;
    #[link_name = "contentMountHandle"]
    #[allow(non_upper_case_globals)]
    pub static mut contentMountHandle: *mut std::ffi::c_void;
    #[link_name = "baseContentPath"]
    #[allow(non_upper_case_globals)]
    pub static mut baseContentPath: [std::ffi::c_char; PATH_MAX];
}

/// Size of the C `baseContentPath[PATH_MAX]` buffer: the platform `<limits.h>`
/// value the C object is compiled with (1024 on macOS, 4096 on Linux).
pub const PATH_MAX: usize = libc::PATH_MAX as usize;

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod c_dir_globals {
    pub static mut repository: *mut std::ffi::c_void = std::ptr::null_mut();
    pub static mut configDir: *mut std::ffi::c_void = std::ptr::null_mut();
    pub static mut contentDir: *mut std::ffi::c_void = std::ptr::null_mut();
    pub static mut saveDir: *mut std::ffi::c_void = std::ptr::null_mut();
    pub static mut meleeDir: *mut std::ffi::c_void = std::ptr::null_mut();
    pub static mut contentMountHandle: *mut std::ffi::c_void = std::ptr::null_mut();
    pub static mut baseContentPath: [std::ffi::c_char; super::PATH_MAX] = [0; super::PATH_MAX];
}

#[cfg(test)]
use c_dir_globals::*;

// uio_* are Rust-implemented (USE_RUST_UIO) — same crate, call directly
use crate::io::uio_bridge::{
    uio_DirList_free, uio_closeDir, uio_getDirList, uio_mountDir, uio_openDir, uio_openDirRelative,
    uio_transplantDir,
};

// ---------------------------------------------------------------------------
// Helper: null AutoMount pointer (C uses `static uio_AutoMount *autoMount[] = { NULL }`)
// ---------------------------------------------------------------------------

/// The path's own bytes as a C string; a path built from C strings and
/// environment values cannot contain NUL.
fn c_string_of(path: &Path) -> CString {
    CString::new(path.as_os_str().as_bytes())
        .expect("a path from C strings and the environment holds no NUL")
}

/// Mount a stdio directory at "/" (or mountPoint) and return the handle.
unsafe fn mount_stdio_dir(
    repo: *mut uio_Repository,
    mount_point: &str,
    source_path: &Path,
    flags: c_int,
) -> *mut uio_MountHandle {
    let mp = CString::new(mount_point).unwrap();
    let sp = c_string_of(source_path);
    uio_mountDir(
        repo,
        mp.as_ptr(),
        UIO_FSTYPE_STDIO,
        ptr::null_mut(),
        ptr::null(),
        sp.as_ptr(),
        ptr::null_mut(),
        flags,
        ptr::null_mut(),
    )
}

/// Opens the directory `name` below the config directory, for a directory
/// the user wants kept at `host`. The original opened `<config>/name` whatever
/// `host` was and could not follow an override elsewhere (its own TODO). When
/// `host` is not `<config>/name`, it is mounted at `/name` first, so the
/// handle reaches the directory that was created.
unsafe fn open_config_subdir(name: &str, host: &Path) -> Result<*mut uio_DirHandle, DirPrepError> {
    let repo = repository as *mut uio_Repository;
    let config_dir = configDir as *mut uio_DirHandle;
    let config_host = std::env::var_os("UQM_CONFIG_DIR")
        .expect("prepare_config_dir runs before the directories below it");
    if host != Path::new(&config_host).join(name) {
        let mount = mount_stdio_dir(repo, name, host, UIO_MOUNT_TOP);
        if mount.is_null() {
            return Err(DirPrepError::MountFailed(format!(
                "Could not mount {name} dir: {}",
                host.display()
            )));
        }
    }
    let relative = CString::new(name).expect("a fixed directory name holds no NUL");
    let dir = uio_openDirRelative(config_dir, relative.as_ptr(), 0);
    if dir.is_null() {
        return Err(DirPrepError::MountFailed(format!(
            "Could not open {name} dir: {}",
            host.display()
        )));
    }
    Ok(dir)
}

/// Creates `path` and its missing parents with the C `mkdirhier` semantics.
fn create_hierarchy(path: &Path) -> Result<(), DirPrepError> {
    mkdirhier(path.as_os_str().as_bytes()).map_err(DirPrepError::Io)
}

/// Records the content path in the C `baseContentPath[PATH_MAX]` buffer, the
/// one consumer with a fixed size. A path that does not fit with its NUL is
/// refused rather than truncated or written past the buffer.
pub fn store_base_content_path(path: &Path) -> Result<(), DirPrepError> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.len() >= PATH_MAX {
        return Err(DirPrepError::InvalidPath {
            path: path.to_path_buf(),
            source: std::io::Error::from_raw_os_error(libc::ENAMETOOLONG),
        });
    }
    // SAFETY: `bytes.len() < PATH_MAX`, so the bytes and their NUL fit the buffer.
    unsafe {
        let dst = std::ptr::addr_of_mut!(baseContentPath) as *mut u8;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        *dst.add(bytes.len()) = 0;
    }
    Ok(())
}

/// Open a directory in the repository and return the handle.
unsafe fn open_dir(repo: *mut uio_Repository, path: &str) -> *mut uio_DirHandle {
    let p = CString::new(path).unwrap();
    uio_openDir(repo, p.as_ptr(), 0)
}

// ---------------------------------------------------------------------------
// Public API: prepare_config_dir
// ---------------------------------------------------------------------------

/// Prepare the config directory.
///
/// Resolves the path (default `~/.uqm/`), creates it if needed, mounts
/// it at "/" in the uio repository, and sets the C `configDir` global.
pub fn prepare_config_dir(config_dir: Option<&Path>) -> Result<(), DirPrepError> {
    let raw = config_dir.unwrap_or(Path::new(CONFIGDIR_DEFAULT));
    let path = expand_startup_path(raw, UNBOUNDED)?;

    // Set env var so UQM_SAVE_DIR / UQM_MELEE_DIR can reference it
    std::env::set_var("UQM_CONFIG_DIR", &path);

    create_hierarchy(&path)?;

    tracing::debug!("Using config dir '{}'", path.display());

    let repo = unsafe { repository };

    // Mount config dir at "/"
    let mount = unsafe { mount_stdio_dir(repo as *mut uio_Repository, "/", &path, UIO_MOUNT_TOP) };
    if mount.is_null() {
        return Err(DirPrepError::MountFailed(format!(
            "Could not mount config dir: {}",
            path.display()
        )));
    }

    // Open "/" and set global
    let dir = unsafe { open_dir(repo as *mut uio_Repository, "/") };
    if dir.is_null() {
        return Err(DirPrepError::MountFailed(
            "Could not open config dir after mount".to_string(),
        ));
    }

    unsafe { configDir = dir as *mut std::ffi::c_void };

    Ok(())
}

// ---------------------------------------------------------------------------
// Public API: prepare_save_dir
// ---------------------------------------------------------------------------

/// Prepare the save directory.
///
/// Resolves `$UQM_SAVE_DIR` (default `${UQM_CONFIG_DIR}/save`), creates
/// it, and opens `save` relative to the config dir.
pub fn prepare_save_dir() -> Result<(), DirPrepError> {
    let raw = std::env::var_os("UQM_SAVE_DIR")
        .unwrap_or_else(|| OsString::from("${UQM_CONFIG_DIR}/save"));
    let path = expand_startup_path(Path::new(&raw), UNBOUNDED)?;

    std::env::set_var("UQM_SAVE_DIR", &path);
    create_hierarchy(&path)?;

    tracing::debug!("Saved games are kept in {}.", path.display());

    let dir = unsafe { open_config_subdir("save", &path)? };
    unsafe { saveDir = dir as *mut std::ffi::c_void };
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API: prepare_melee_dir
// ---------------------------------------------------------------------------

/// Prepare the melee teams directory.
///
/// Resolves `$UQM_MELEE_DIR` (default `${UQM_CONFIG_DIR}/teams`), creates
/// it, and opens `teams` relative to the config dir.
pub fn prepare_melee_dir() -> Result<(), DirPrepError> {
    let raw = std::env::var_os("UQM_MELEE_DIR")
        .unwrap_or_else(|| OsString::from("${UQM_CONFIG_DIR}/teams"));
    let path = expand_startup_path(Path::new(&raw), UNBOUNDED)?;

    std::env::set_var("UQM_MELEE_DIR", &path);
    create_hierarchy(&path)?;

    let dir = unsafe { open_config_subdir("teams", &path)? };
    unsafe { meleeDir = dir as *mut std::ffi::c_void };
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API: prepare_content_dir
// ---------------------------------------------------------------------------

/// Prepare the content directory.
///
/// Finds the content dir (by looking for a `version` file), mounts it at
/// "/", mounts any `/packages/*.zip` files, and mounts addon packs.
pub fn prepare_content_dir(
    content_dir: Option<&Path>,
    addon_dir: Option<&Path>,
) -> Result<(), DirPrepError> {
    // Find the content path
    let content_path = if let Some(dir) = content_dir {
        let expanded = expand_startup_path(dir, PATH_MAX)?;
        if !expanded.join("version").exists() {
            return Err(DirPrepError::NotFound(format!(
                "Content dir '{}' does not contain a 'version' file",
                expanded.display()
            )));
        }
        expanded
    } else {
        find_content_dir()?
            .ok_or_else(|| DirPrepError::NotFound("Could not find content.".to_string()))?
    };

    tracing::debug!("Using '{}' as base content dir.", content_path.display());

    store_base_content_path(&content_path)?;

    let repo = unsafe { repository };

    // Mount content dir at "/"
    let mount = unsafe {
        mount_stdio_dir(
            repo as *mut uio_Repository,
            "/",
            &content_path,
            UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        )
    };
    if mount.is_null() {
        return Err(DirPrepError::MountFailed(format!(
            "Could not mount content dir: {}",
            content_path.display()
        )));
    }
    unsafe { contentMountHandle = mount as *mut std::ffi::c_void };

    // Open "/" and set global
    let dir = unsafe { open_dir(repo as *mut uio_Repository, "/") };
    if dir.is_null() {
        return Err(DirPrepError::MountFailed(
            "Could not open content dir after mount".to_string(),
        ));
    }
    unsafe { contentDir = dir as *mut std::ffi::c_void };

    // Mount /packages zips
    unsafe { mount_packages_zips(repo as *mut uio_Repository, dir) };

    // Mount addon dir
    if let Some(ad) = addon_dir {
        tracing::debug!("Using '{}' as addon dir.", ad.display());
        unsafe { mount_addon_dir_explicit(repo as *mut uio_Repository, ad) };
    }

    // Scan and list addon packs
    unsafe { scan_addon_packs(dir) };

    Ok(())
}

/// Find content dir by looking for `version` file in common locations.
fn find_content_dir() -> Result<Option<PathBuf>, DirPrepError> {
    let candidates: Vec<PathBuf> = vec![
        PathBuf::from("."),
        PathBuf::from("content"),
        PathBuf::from("../../content"),
        PathBuf::from("../../../content"),
    ];

    for c in &candidates {
        if c.join("version").exists() {
            let canonical = std::fs::canonicalize(c).unwrap_or_else(|_| c.clone());
            return Ok(Some(canonical));
        }
    }

    // Walk up from executable
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = match exe.parent() {
            Some(d) => d.to_path_buf(),
            None => return Ok(None),
        };
        for _ in 0..6 {
            let content = dir.join("content");
            if content.join("version").exists() {
                return Ok(Some(content));
            }
            if !dir.pop() {
                break;
            }
        }
    }

    Ok(None)
}

/// Mount zip files from /packages directory below content root.
unsafe fn mount_packages_zips(repo: *mut uio_Repository, _content_dir: *mut uio_DirHandle) {
    let packages = CString::new("/packages").unwrap();
    let pkg_dir = uio_openDir(repo, packages.as_ptr(), 0);
    if pkg_dir.is_null() {
        return;
    }
    mount_dir_zips(pkg_dir, c"/", UIO_MOUNT_BELOW);
    uio_closeDir(pkg_dir);
}

/// Mount an explicit addon directory (from `--addondir`).
unsafe fn mount_addon_dir_explicit(repo: *mut uio_Repository, addon_dir: &Path) {
    let mount = mount_stdio_dir(repo, "addons", addon_dir, UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    if mount.is_null() {
        tracing::warn!(
            "Could not mount addon directory: {}; '--addon' options are ignored.",
            addon_dir.display()
        );
    }
}

/// Scan the `addons/` directory inside content, list available packs,
/// and mount their zip files.
unsafe fn scan_addon_packs(content_dir: *mut uio_DirHandle) {
    let addons_str = CString::new("addons").unwrap();
    let addons_dir = uio_openDirRelative(content_dir, addons_str.as_ptr(), 0);
    if addons_dir.is_null() {
        tracing::warn!(
            "There's no 'addons' directory in the 'content' directory; \
             '--addon' options are ignored."
        );
        return;
    }

    // Mount zips directly in addons/
    mount_dir_zips(addons_dir, c"addons", UIO_MOUNT_BELOW);

    // List and log addon subdirectories
    let empty = CString::new("").unwrap();
    let dir_list = uio_getDirList(addons_dir, empty.as_ptr(), empty.as_ptr(), MATCH_PREFIX);
    if !dir_list.is_null() {
        let dl = &*dir_list;
        if dl.numNames == 0 || dl.names.is_null() {
            tracing::info!("0 available addon packs.");
        } else {
            let names_slice = std::slice::from_raw_parts(dl.names, dl.numNames as usize);
            let mut count = 0;
            for name_ptr in names_slice {
                if name_ptr.is_null() {
                    continue;
                }
                // The name's own bytes open the directory; only the log is lossy.
                let name = std::ffi::CStr::from_ptr(*name_ptr);

                if name.to_bytes().starts_with(b".") {
                    continue;
                }

                count += 1;
                tracing::info!("    {}. {}", count, name.to_string_lossy());

                // Mount zips inside each addon subdir
                let addon_dir_handle = uio_openDirRelative(addons_dir, name.as_ptr(), 0);
                if !addon_dir_handle.is_null() {
                    let mut mountname = b"addons/".to_vec();
                    mountname.extend_from_slice(name.to_bytes());
                    let mountname =
                        CString::new(mountname).expect("a C string name has no interior NUL");
                    mount_dir_zips(addon_dir_handle, &mountname, UIO_MOUNT_BELOW);
                    uio_closeDir(addon_dir_handle);
                }
            }

            if count == 0 {
                tracing::info!("0 available addon packs.");
            } else {
                tracing::info!(
                    "{} available addon pack{}.",
                    count,
                    if count == 1 { "" } else { "s" }
                );
            }
        }

        uio_DirList_free(dir_list);
    } else {
        tracing::info!("0 available addon packs.");
    }

    uio_closeDir(addons_dir);
}

/// Mount all `.zip` / `.uqm` files found in `dir_handle` at `mount_point`.
unsafe fn mount_dir_zips(
    dir_handle: *mut uio_DirHandle,
    mount_point: &CStr,
    relative_flags: c_int,
) {
    let empty = CString::new("").unwrap();
    let pattern = CString::new("\\.([zZ][iI][pP]|[uU][qQ][mM])$").unwrap();
    let content_mount = unsafe { contentMountHandle as *mut uio_MountHandle };

    let dir_list = uio_getDirList(dir_handle, empty.as_ptr(), pattern.as_ptr(), MATCH_REGEX);
    if dir_list.is_null() {
        return;
    }

    let dl = &*dir_list;
    if dl.numNames == 0 || dl.names.is_null() {
        uio_DirList_free(dir_list);
        return;
    }

    let names_slice = std::slice::from_raw_parts(dl.names, dl.numNames as usize);
    for name_ptr in names_slice {
        if name_ptr.is_null() {
            continue;
        }
        let name = std::ffi::CStr::from_ptr(*name_ptr);

        let result = uio_mountDir(
            unsafe { repository as *mut uio_Repository },
            mount_point.as_ptr(),
            UIO_FSTYPE_ZIP,
            dir_handle,
            *name_ptr,
            c"/".as_ptr() as *const c_char,
            ptr::null_mut(),
            relative_flags | UIO_MOUNT_RDONLY,
            content_mount,
        );
        if result.is_null() {
            tracing::warn!(
                "Could not mount '{}': {}.",
                name.to_string_lossy(),
                std::io::Error::last_os_error()
            );
        }
    }

    uio_DirList_free(dir_list);
}

// ---------------------------------------------------------------------------
// Public API: prepare_shadow_addons
// ---------------------------------------------------------------------------

/// Mount shadow content from each specified addon.
///
/// For each addon, looks for a `shadow-content` subdirectory and mounts
/// its zips (and non-zip content) on top of "/".
pub fn prepare_shadow_addons(addons: &[&OsStr]) -> Result<(), DirPrepError> {
    let content_dir = unsafe { contentDir as *mut uio_DirHandle };
    if content_dir.is_null() {
        return Ok(());
    }

    let addons_str = CString::new("addons").unwrap();
    let addons_dir = unsafe { uio_openDirRelative(content_dir, addons_str.as_ptr(), 0) };
    if addons_dir.is_null() {
        // No addon dir — fail silently (will fail again later)
        return Ok(());
    }

    let shadow_name = "shadow-content";

    for addon in addons {
        let addon_c = CString::new(addon.as_bytes()).expect("a C string name has no interior NUL");
        let addon_dir = unsafe { uio_openDirRelative(addons_dir, addon_c.as_ptr(), 0) };
        if addon_dir.is_null() {
            continue;
        }

        let shadow_c = CString::new(shadow_name).unwrap();
        let shadow_dir = unsafe { uio_openDirRelative(addon_dir, shadow_c.as_ptr(), 0) };
        if !shadow_dir.is_null() {
            tracing::debug!(
                "Mounting shadow content of '{}' addon",
                addon.to_string_lossy()
            );

            unsafe {
                // Mount shadow zips
                mount_dir_zips(shadow_dir, c"/", UIO_MOUNT_ABOVE);

                // Mount non-zipped shadow content
                let slash = CString::new("/").unwrap();
                uio_transplantDir(
                    slash.as_ptr(),
                    shadow_dir,
                    UIO_MOUNT_RDONLY | UIO_MOUNT_ABOVE,
                    contentMountHandle as *mut uio_MountHandle,
                );
            }

            unsafe { uio_closeDir(shadow_dir) };
        }

        unsafe { uio_closeDir(addon_dir) };
    }

    unsafe { uio_closeDir(addons_dir) };
    Ok(())
}

// ---------------------------------------------------------------------------
// FFI exports (called from C when RUST_OWNS_MAIN)
// ---------------------------------------------------------------------------

/// The bytes of an optional C path argument, unvalidated.
///
/// # Safety
/// `value` is null or a NUL-terminated string that outlives `'a`.
unsafe fn optional_path<'a>(value: *const c_char) -> Option<&'a Path> {
    if value.is_null() {
        None
    } else {
        Some(Path::new(OsStr::from_bytes(
            CStr::from_ptr(value).to_bytes(),
        )))
    }
}

fn status_of(step: &str, result: Result<(), DirPrepError>) -> c_int {
    match result {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("{step} failed: {:?}", e);
            -1
        }
    }
}

/// # Safety
/// `config_dir` is NULL or a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn rust_prepare_config_dir(config_dir: *const c_char) -> c_int {
    status_of(
        "prepare_config_dir",
        prepare_config_dir(optional_path(config_dir)),
    )
}

/// # Safety
/// `content_dir` and `addon_dir` are NULL or NUL-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn rust_prepare_content_dir(
    content_dir: *const c_char,
    addon_dir: *const c_char,
    _exec_file: *const c_char,
) -> c_int {
    status_of(
        "prepare_content_dir",
        prepare_content_dir(optional_path(content_dir), optional_path(addon_dir)),
    )
}

/// # Safety
/// No parameters — safe to call.
#[no_mangle]
pub unsafe extern "C" fn rust_prepare_save_dir() -> c_int {
    status_of("prepare_save_dir", prepare_save_dir())
}

/// # Safety
/// No parameters — safe to call.
#[no_mangle]
pub unsafe extern "C" fn rust_prepare_melee_dir() -> c_int {
    status_of("prepare_melee_dir", prepare_melee_dir())
}

/// # Safety
/// `addons` is NULL or a NULL-terminated array of NUL-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn rust_prepare_shadow_addons(addons: *const *const c_char) -> c_int {
    if addons.is_null() {
        return 0;
    }
    let mut list = Vec::new();
    let mut i = 0;
    loop {
        let ptr = *addons.add(i);
        if ptr.is_null() {
            break;
        }
        list.push(OsStr::from_bytes(CStr::from_ptr(ptr).to_bytes()));
        i += 1;
    }
    status_of("prepare_shadow_addons", prepare_shadow_addons(&list))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "dir_prep_lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(test)]
#[path = "dir_prep_startup_tests.rs"]
mod startup_tests;

#[cfg(test)]
#[path = "dir_prep_joint_tests.rs"]
mod joint_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_find_content_dir_finds_or_none() {
        // Should either find content or return None — never panic
        let result = find_content_dir().unwrap();
        if let Some(ref path) = result {
            assert!(
                path.join("version").exists(),
                "found content dir {} should have version file",
                path.display()
            );
        }
    }
}
