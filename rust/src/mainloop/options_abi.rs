//! Runtime options and resource-index loading, ported from options.c.
#![allow(non_snake_case)]
pub use crate::input::templates::input_templates;
pub use crate::io::handles::{configDir, contentDir, contentMountHandle, meleeDir, saveDir};
use crate::io::uio_bridge::*;
use std::{
    ffi::{c_char, CStr},
    ptr,
};

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
/// Addons is a NUL-terminated array of NUL-terminated names; resources are initialized.
pub unsafe fn prepareAddons(addons: &[std::ffi::CString]) {
    for addon in addons {
        log::info!("Loading addon '{}'", addon.to_string_lossy());
        if loadAddon(addon.as_ptr()) == 0 {
            break;
        }
    }
}

/// # Safety
/// No directory users remain. Handles are null or owned by this subsystem.
pub unsafe fn unprepareAllDirs() {
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

pub fn setGammaCorrection(gamma: f32) -> bool {
    let set = crate::graphics::sdl::platform_abi::TFB_SetGamma(gamma);
    if set {
        log::info!("Gamma correction set to {gamma:.4}.");
    } else {
        log::warn!("Unable to set gamma correction.");
    }
    set
}
