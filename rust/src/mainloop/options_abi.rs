//! Runtime options and resource-index loading, ported from options.c.
#![allow(non_snake_case)]
use crate::io::uio_bridge::*;
use std::{
    ffi::{c_char, CStr},
    ptr,
};

macro_rules! global {
    ($($name:ident: $ty:ty),* $(,)?) => { $(#[no_mangle] pub static mut $name: $ty = unsafe { std::mem::zeroed() };)* };
}
global!(
    optWhichCoarseScan: i32, optWhichMenu: i32, optWhichFonts: i32,
    optWhichIntro: i32, optWhichShield: i32, optSmoothScroll: i32, optMeleeScale: i32,
    optAddons: *const *const c_char,
    opt3doMusic: u32, optRemixMusic: u32, optSpeech: u32, optSubtitles: u32,
    optStereoSFX: u32, optKeepAspectRatio: u32, optGamma: f32,
    contentDir: *mut uio_DirHandle, configDir: *mut uio_DirHandle,
    saveDir: *mut uio_DirHandle, meleeDir: *mut uio_DirHandle,
    contentMountHandle: *mut uio_MountHandle,
    baseContentPath: [c_char; libc::PATH_MAX as usize], input_templates: [[c_char; 30]; 6],
);

fn fatal(result: Result<(), super::dir_prep::DirPrepError>) {
    if let Err(error) = result {
        log::error!("Directory preparation failed: {error:?}");
        std::process::exit(1);
    }
}
unsafe fn optional<'a>(value: *const c_char) -> Option<&'a str> {
    if value.is_null() {
        None
    } else {
        Some(
            CStr::from_ptr(value)
                .to_str()
                .expect("directory path is UTF-8"),
        )
    }
}
unsafe fn addon_list(mut values: *const *const c_char) -> Vec<String> {
    let mut result = vec![];
    while !(*values).is_null() {
        result.push(
            CStr::from_ptr(*values)
                .to_str()
                .expect("addon name is UTF-8")
                .to_owned(),
        );
        values = values.add(1);
    }
    result
}

/// # Safety
/// String pointers are null or NUL-terminated paths; IO is initialized.
#[no_mangle]
pub unsafe extern "C" fn prepareContentDir(
    content: *const c_char,
    addon: *const c_char,
    _executable: *const c_char,
) {
    fatal(super::dir_prep::prepare_content_dir(
        optional(content),
        optional(addon),
    ));
}
/// # Safety
/// Path is null or NUL-terminated; IO is initialized.
#[no_mangle]
pub unsafe extern "C" fn prepareConfigDir(path: *const c_char) {
    fatal(super::dir_prep::prepare_config_dir(optional(path)));
}
#[no_mangle]
pub extern "C" fn prepareSaveDir() {
    fatal(super::dir_prep::prepare_save_dir());
}
#[no_mangle]
pub extern "C" fn prepareMeleeDir() {
    fatal(super::dir_prep::prepare_melee_dir());
}

/// # Safety
/// Directory is a live UIO handle on the IO owner thread.
#[no_mangle]
pub unsafe extern "C" fn loadIndices(dir: *mut uio_DirHandle) -> i32 {
    let indices = uio_getDirList(dir, c"".as_ptr(), c"\\.[rR][mM][pP]$".as_ptr(), 4);
    let mut loaded = 0;
    if !indices.is_null() {
        for i in 0..(*indices).numNames {
            let name = *(*indices).names.add(i as usize);
            log::debug!(
                "Loading resource index '{}'",
                CStr::from_ptr(name).to_string_lossy()
            );
            crate::resource::ffi_bridge::LoadResourceIndex(dir.cast(), name, ptr::null());
            loaded += 1;
        }
    }
    uio_DirList_free(indices);
    loaded
}

/// # Safety
/// Addon is NUL-terminated; contentDir and the resource system are initialized.
#[no_mangle]
pub unsafe extern "C" fn loadAddon(addon: *const c_char) -> u32 {
    let addons = uio_openDirRelative(contentDir, c"addons".as_ptr(), 0);
    if addons.is_null() {
        log::warn!("There's no 'addons' directory in the 'content' directory; '--addon' options are ignored.");
        return 0;
    }
    let dir = uio_openDirRelative(addons, addon, 0);
    if dir.is_null() {
        log::warn!(
            "Addon '{}' not found",
            CStr::from_ptr(addon).to_string_lossy()
        );
        uio_closeDir(addons);
        return 0;
    }
    let loaded = loadIndices(dir);
    if loaded == 0 {
        log::error!(
            "No RMP index files were loaded for addon '{}'",
            CStr::from_ptr(addon).to_string_lossy()
        );
    }
    uio_closeDir(dir);
    uio_closeDir(addons);
    u32::from(loaded > 0)
}

/// # Safety
/// Addons is a NUL-terminated array of NUL-terminated names; directories are initialized.
#[no_mangle]
pub unsafe extern "C" fn prepareShadowAddons(addons: *const *const c_char) {
    if let Err(error) = super::dir_prep::prepare_shadow_addons(&addon_list(addons)) {
        log::warn!("Could not mount shadow content: {error:?}");
    }
}

/// # Safety
/// Addons is a NUL-terminated array of NUL-terminated names; resources are initialized.
#[no_mangle]
pub unsafe extern "C" fn prepareAddons(mut addons: *const *const c_char) {
    while !(*addons).is_null() {
        log::info!(
            "Loading addon '{}'",
            CStr::from_ptr(*addons).to_string_lossy()
        );
        if loadAddon(*addons) == 0 {
            break;
        }
        addons = addons.add(1);
    }
}

/// # Safety
/// No directory users remain. Handles are null or owned by this subsystem.
#[no_mangle]
pub unsafe extern "C" fn unprepareAllDirs() {
    for slot in [
        ptr::addr_of_mut!(saveDir),
        ptr::addr_of_mut!(meleeDir),
        ptr::addr_of_mut!(contentDir),
        ptr::addr_of_mut!(configDir),
    ] {
        if !(*slot).is_null() {
            uio_closeDir(*slot);
            *slot = ptr::null_mut();
        }
    }
}

#[no_mangle]
pub extern "C" fn setGammaCorrection(gamma: f32) -> bool {
    let set = crate::graphics::sdl::platform_abi::TFB_SetGamma(gamma);
    if set {
        log::info!("Gamma correction set to {gamma:.4}.");
    } else {
        log::warn!("Unable to set gamma correction.");
    }
    set
}
