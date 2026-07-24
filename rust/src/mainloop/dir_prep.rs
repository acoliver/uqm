//! Directory preparation for the UQM init sequence.
//!
//! Rust port of `options.c`'s `prepareConfigDir`, `prepareContentDir`,
//! `prepareMeleeDir`, `prepareSaveDir`, and `prepareShadowAddons`.
//! Path resolution, env-var expansion, and directory creation happen in
//! pure Rust; uio mounting calls into `crate::io::uio_bridge` (already
//! Rust-implemented), and C-side globals are set via bridge accessors.

use std::ffi::CString;
use std::path::PathBuf;
use std::ptr;

use libc::{c_char, c_int};

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

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum DirPrepError {
    Io(std::io::Error),
    NotFound(String),
    MountFailed(String),
}

impl From<std::io::Error> for DirPrepError {
    fn from(e: std::io::Error) -> Self {
        DirPrepError::Io(e)
    }
}

// ---------------------------------------------------------------------------
// Path resolution (pure Rust — replaces C expandPath)
// ---------------------------------------------------------------------------

/// Expand `~` and `${VAR}` patterns in a path string, returning an
/// absolute [`PathBuf`].
///
/// Mirrors C `expandPath(buf, len, path, EP_ALL_SYSTEM)`:
/// - `~` or `~/...` → `$HOME/...`
/// - `${VAR}` / `$VAR` → env value
pub fn expand_path(input: &str) -> Result<PathBuf, DirPrepError> {
    let expanded = expand_env_vars(input);

    let path = PathBuf::from(&expanded);
    if path.is_absolute() {
        Ok(path)
    } else {
        // Relative path — make absolute against CWD
        Ok(std::env::current_dir()?.join(path))
    }
}

/// Replace `~` prefix with `$HOME` and expand `${VAR}` / `$VAR`.
fn expand_env_vars(input: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();

    let with_home = if input == "~" {
        home.clone()
    } else if let Some(rest) = input.strip_prefix("~/") {
        format!("{home}/{rest}")
    } else {
        input.to_string()
    };

    expand_dollar_vars(&with_home)
}

/// Expand `$VAR` and `${VAR}` references using `std::env::var`.
fn expand_dollar_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'$' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                // ${VAR}
                if let Some(end) = input[i + 2..].find('}') {
                    let var_name = &input[i + 2..i + 2 + end];
                    if let Ok(val) = std::env::var(var_name) {
                        result.push_str(&val);
                    }
                    i = i + 2 + end + 1;
                    continue;
                }
            } else if i + 1 < bytes.len()
                && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_')
            {
                // $VAR (no braces)
                let start = i + 1;
                let mut end = start;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                let var_name = &input[start..end];
                if let Ok(val) = std::env::var(var_name) {
                    result.push_str(&val);
                }
                i = end;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }

    result
}

// ---------------------------------------------------------------------------
// C bridge FFI (getter/setter for C globals + uio calls)
// ---------------------------------------------------------------------------

extern "C" {
    fn uqm_get_repository() -> *mut uio_Repository;
    #[allow(
        improper_ctypes,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    fn uqm_get_config_dir() -> *mut uio_DirHandle;
    #[allow(
        improper_ctypes,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    fn uqm_get_content_dir() -> *mut uio_DirHandle;
    fn uqm_get_content_mount_handle() -> *mut uio_MountHandle;
    #[allow(
        improper_ctypes,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    fn uqm_set_config_dir(d: *mut uio_DirHandle);
    #[allow(
        improper_ctypes,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    fn uqm_set_content_dir(d: *mut uio_DirHandle);
    #[allow(
        improper_ctypes,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    fn uqm_set_save_dir(d: *mut uio_DirHandle);
    #[allow(
        improper_ctypes,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    fn uqm_set_melee_dir(d: *mut uio_DirHandle);
    fn uqm_set_content_mount_handle(h: *mut uio_MountHandle);
    fn uqm_set_base_content_path(path: *const c_char);
}

// uio_* are Rust-implemented (USE_RUST_UIO) — same crate, call directly
use crate::io::uio_bridge::{
    uio_DirList_free, uio_closeDir, uio_getDirList, uio_mountDir, uio_openDir, uio_openDirRelative,
    uio_transplantDir,
};

// ---------------------------------------------------------------------------
// Helper: null AutoMount pointer (C uses `static uio_AutoMount *autoMount[] = { NULL }`)
// ---------------------------------------------------------------------------

/// Mount a stdio directory at "/" (or mountPoint) and return the handle.
unsafe fn mount_stdio_dir(
    repository: *mut uio_Repository,
    mount_point: &str,
    source_path: &str,
    flags: c_int,
) -> *mut uio_MountHandle {
    let mp = CString::new(mount_point).unwrap();
    let sp = CString::new(source_path).unwrap();
    uio_mountDir(
        repository,
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

/// Open a directory in the repository and return the handle.
unsafe fn open_dir(repository: *mut uio_Repository, path: &str) -> *mut uio_DirHandle {
    let p = CString::new(path).unwrap();
    uio_openDir(repository, p.as_ptr(), 0)
}

// ---------------------------------------------------------------------------
// Public API: prepare_config_dir
// ---------------------------------------------------------------------------

/// Prepare the config directory.
///
/// Resolves the path (default `~/.uqm/`), creates it if needed, mounts
/// it at "/" in the uio repository, and sets the C `configDir` global.
pub fn prepare_config_dir(config_dir: Option<&str>) -> Result<(), DirPrepError> {
    let raw = config_dir.unwrap_or(CONFIGDIR_DEFAULT);
    let path = expand_path(raw)?;
    let path_str = path.to_string_lossy().to_string();

    // Set env var so UQM_SAVE_DIR / UQM_MELEE_DIR can reference it
    std::env::set_var("UQM_CONFIG_DIR", &path_str);

    // Create directory hierarchy
    std::fs::create_dir_all(&path)?;

    tracing::debug!("Using config dir '{}'", path_str);

    let repo = unsafe { uqm_get_repository() };

    // Mount config dir at "/"
    let mount = unsafe { mount_stdio_dir(repo, "/", &path_str, UIO_MOUNT_TOP) };
    if mount.is_null() {
        return Err(DirPrepError::MountFailed(format!(
            "Could not mount config dir: {path_str}"
        )));
    }

    // Open "/" and set global
    let dir = unsafe { open_dir(repo, "/") };
    if dir.is_null() {
        return Err(DirPrepError::MountFailed(
            "Could not open config dir after mount".to_string(),
        ));
    }

    unsafe { uqm_set_config_dir(dir) };

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
    let raw =
        std::env::var("UQM_SAVE_DIR").unwrap_or_else(|_| "${UQM_CONFIG_DIR}/save".to_string());
    let path = expand_path(&raw)?;
    let path_str = path.to_string_lossy().to_string();

    std::env::set_var("UQM_SAVE_DIR", &path_str);
    std::fs::create_dir_all(&path)?;

    tracing::debug!("Saved games are kept in {}.", path_str);

    let config_dir = unsafe { uqm_get_config_dir() };
    let save = CString::new("save").unwrap();
    let dir = unsafe { uio_openDirRelative(config_dir, save.as_ptr(), 0) };
    if dir.is_null() {
        return Err(DirPrepError::MountFailed(format!(
            "Could not open save dir: {path_str}"
        )));
    }

    unsafe { uqm_set_save_dir(dir) };
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
    let raw =
        std::env::var("UQM_MELEE_DIR").unwrap_or_else(|_| "${UQM_CONFIG_DIR}/teams".to_string());
    let path = expand_path(&raw)?;
    let path_str = path.to_string_lossy().to_string();

    std::env::set_var("UQM_MELEE_DIR", &path_str);
    std::fs::create_dir_all(&path)?;

    let config_dir = unsafe { uqm_get_config_dir() };
    let teams = CString::new("teams").unwrap();
    let dir = unsafe { uio_openDirRelative(config_dir, teams.as_ptr(), 0) };
    if dir.is_null() {
        return Err(DirPrepError::MountFailed(format!(
            "Could not open melee teams dir: {path_str}"
        )));
    }

    unsafe { uqm_set_melee_dir(dir) };
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
    content_dir: Option<&str>,
    addon_dir: Option<&str>,
) -> Result<(), DirPrepError> {
    // Find the content path
    let content_path = if let Some(dir) = content_dir {
        let expanded = expand_path(dir)?;
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

    let path_str = content_path.to_string_lossy().to_string();

    tracing::debug!("Using '{}' as base content dir.", path_str);

    // Set the C global
    let c_path = CString::new(path_str.as_str()).unwrap();
    unsafe { uqm_set_base_content_path(c_path.as_ptr()) };

    let repo = unsafe { uqm_get_repository() };

    // Mount content dir at "/"
    let mount = unsafe { mount_stdio_dir(repo, "/", &path_str, UIO_MOUNT_TOP | UIO_MOUNT_RDONLY) };
    if mount.is_null() {
        return Err(DirPrepError::MountFailed(format!(
            "Could not mount content dir: {path_str}"
        )));
    }
    unsafe { uqm_set_content_mount_handle(mount) };

    // Open "/" and set global
    let dir = unsafe { open_dir(repo, "/") };
    if dir.is_null() {
        return Err(DirPrepError::MountFailed(
            "Could not open content dir after mount".to_string(),
        ));
    }
    unsafe { uqm_set_content_dir(dir) };

    // Mount /packages zips
    unsafe { mount_packages_zips(repo, dir) };

    // Mount addon dir
    if let Some(ad) = addon_dir {
        tracing::debug!("Using '{}' as addon dir.", ad);
        unsafe { mount_addon_dir_explicit(repo, ad) };
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
    mount_dir_zips(pkg_dir, "/", UIO_MOUNT_BELOW);
    uio_closeDir(pkg_dir);
}

/// Mount an explicit addon directory (from `--addondir`).
unsafe fn mount_addon_dir_explicit(repo: *mut uio_Repository, addon_dir: &str) {
    let mount = mount_stdio_dir(repo, "addons", addon_dir, UIO_MOUNT_TOP | UIO_MOUNT_RDONLY);
    if mount.is_null() {
        tracing::warn!(
            "Could not mount addon directory: {}; '--addon' options are ignored.",
            addon_dir
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
    mount_dir_zips(addons_dir, "addons", UIO_MOUNT_BELOW);

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
                let name = std::ffi::CStr::from_ptr(*name_ptr);
                let name_str = name.to_string_lossy();

                if name_str.starts_with('.') {
                    continue;
                }

                count += 1;
                tracing::info!("    {}. {}", count, name_str);

                // Mount zips inside each addon subdir
                let addon_c = CString::new(name_str.to_string()).unwrap();
                let addon_dir_handle = uio_openDirRelative(addons_dir, addon_c.as_ptr(), 0);
                if !addon_dir_handle.is_null() {
                    let mountname = format!("addons/{name_str}");
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
unsafe fn mount_dir_zips(dir_handle: *mut uio_DirHandle, mount_point: &str, relative_flags: c_int) {
    let empty = CString::new("").unwrap();
    let pattern = CString::new("\\.([zZ][iI][pP]|[uU][qQ][mM])$").unwrap();
    let content_mount = uqm_get_content_mount_handle();

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

        let mp = CString::new(mount_point).unwrap();
        let result = uio_mountDir(
            uqm_get_repository(),
            mp.as_ptr(),
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
pub fn prepare_shadow_addons(addons: &[String]) -> Result<(), DirPrepError> {
    let content_dir = unsafe { uqm_get_content_dir() };
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
        let addon_c = CString::new(addon.as_str()).unwrap();
        let addon_dir = unsafe { uio_openDirRelative(addons_dir, addon_c.as_ptr(), 0) };
        if addon_dir.is_null() {
            continue;
        }

        let shadow_c = CString::new(shadow_name).unwrap();
        let shadow_dir = unsafe { uio_openDirRelative(addon_dir, shadow_c.as_ptr(), 0) };
        if !shadow_dir.is_null() {
            tracing::debug!("Mounting shadow content of '{}' addon", addon);

            unsafe {
                // Mount shadow zips
                mount_dir_zips(shadow_dir, "/", UIO_MOUNT_ABOVE);

                // Mount non-zipped shadow content
                let slash = CString::new("/").unwrap();
                uio_transplantDir(
                    slash.as_ptr(),
                    shadow_dir,
                    UIO_MOUNT_RDONLY | UIO_MOUNT_ABOVE,
                    uqm_get_content_mount_handle(),
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

/// # Safety
/// C strings must be valid UTF-8 or NULL.
#[no_mangle]
pub unsafe extern "C" fn rust_prepare_config_dir(config_dir: *const c_char) -> c_int {
    let dir = if config_dir.is_null() {
        None
    } else {
        Some(
            std::ffi::CStr::from_ptr(config_dir)
                .to_string_lossy()
                .to_string(),
        )
    };
    match prepare_config_dir(dir.as_deref()) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("prepare_config_dir failed: {:?}", e);
            -1
        }
    }
}

/// # Safety
/// C strings must be valid UTF-8 or NULL.
#[no_mangle]
pub unsafe extern "C" fn rust_prepare_content_dir(
    content_dir: *const c_char,
    addon_dir: *const c_char,
    _exec_file: *const c_char,
) -> c_int {
    let cdir = if content_dir.is_null() {
        None
    } else {
        Some(
            std::ffi::CStr::from_ptr(content_dir)
                .to_string_lossy()
                .to_string(),
        )
    };
    let adir = if addon_dir.is_null() {
        None
    } else {
        Some(
            std::ffi::CStr::from_ptr(addon_dir)
                .to_string_lossy()
                .to_string(),
        )
    };
    match prepare_content_dir(cdir.as_deref(), adir.as_deref()) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("prepare_content_dir failed: {:?}", e);
            -1
        }
    }
}

/// # Safety
/// No parameters — safe to call.
#[no_mangle]
pub unsafe extern "C" fn rust_prepare_save_dir() -> c_int {
    match prepare_save_dir() {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("prepare_save_dir failed: {:?}", e);
            -1
        }
    }
}

/// # Safety
/// No parameters — safe to call.
#[no_mangle]
pub unsafe extern "C" fn rust_prepare_melee_dir() -> c_int {
    match prepare_melee_dir() {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("prepare_melee_dir failed: {:?}", e);
            -1
        }
    }
}

/// # Safety
/// `addons` must be a NULL-terminated array of C strings.
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
        list.push(std::ffi::CStr::from_ptr(ptr).to_string_lossy().to_string());
        i += 1;
    }
    match prepare_shadow_addons(&list) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("prepare_shadow_addons failed: {:?}", e);
            -1
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_path_home() {
        std::env::set_var("HOME", "/tmp/testhome");
        let p = expand_path("~/foo").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/testhome/foo"));
    }

    #[test]
    fn test_expand_path_tilde_only() {
        std::env::set_var("HOME", "/tmp/testhome");
        let p = expand_path("~").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/testhome"));
    }

    #[test]
    fn test_expand_path_absolute() {
        let p = expand_path("/usr/local").unwrap();
        assert_eq!(p, PathBuf::from("/usr/local"));
    }

    #[test]
    fn test_expand_env_var_braces() {
        std::env::set_var("MY_TEST_VAR", "/opt/data");
        let p = expand_path("${MY_TEST_VAR}/sub").unwrap();
        assert_eq!(p, PathBuf::from("/opt/data/sub"));
    }

    #[test]
    fn test_expand_env_var_no_braces() {
        std::env::set_var("MY_TEST_VAR2", "/opt/data2");
        let p = expand_path("$MY_TEST_VAR2/sub").unwrap();
        assert_eq!(p, PathBuf::from("/opt/data2/sub"));
    }

    #[test]
    fn test_expand_env_var_undefined() {
        // Undefined var → expands to empty string
        std::env::remove_var("UNDEFINED_UQM_VAR");
        let p = expand_path("$UNDEFINED_UQM_VAR/path").unwrap();
        assert_eq!(p, PathBuf::from("/path"));
    }

    #[test]
    fn test_expand_path_relative() {
        let p = expand_path("foo/bar").unwrap();
        assert!(p.is_absolute());
        assert!(p.ends_with("foo/bar"));
    }

    #[test]
    fn test_expand_dollar_vars_nested() {
        std::env::set_var("UQM_CONFIG_DIR", "/home/user/.uqm");
        let result = expand_dollar_vars("${UQM_CONFIG_DIR}/save");
        assert_eq!(result, "/home/user/.uqm/save");
    }

    #[test]
    fn test_expand_dollar_vars_multiple() {
        std::env::set_var("A", "/a");
        std::env::set_var("B", "/b");
        let result = expand_dollar_vars("$A/${B}/end");
        assert_eq!(result, "/a//b/end");
    }

    #[test]
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
