//! Shared state entry points formerly supplied by rust_bridge_mainloop.c.
use crate::io::uio_bridge::{uio_DirHandle, uio_MountHandle, uio_Repository};

use crate::ships::native::abi as options;
use std::{ffi::c_char, ptr};
extern "C" {
    static mut contentMountHandle: *mut uio_MountHandle;
}

pub(crate) use crate::navigation::{get as game_byte, set as set_game_byte};

macro_rules! accessor {
    ($name:ident() -> $ret:ty = $body:expr) => {
        /// # Safety
        /// Called on the owner game thread with live shared state.
        pub unsafe fn $name() -> $ret {
            $body
        }
    };
    ($name:ident($arg:ident: $ty:ty) = $body:expr) => {
        /// # Safety
        /// Called on the owner game thread; pointers obey the underlying provider contract.
        pub unsafe fn $name($arg: $ty) {
            $body
        }
    };
}
accessor!(uqm_get_repository() -> *mut uio_Repository = crate::game_init::kernel_abi::repository);
accessor!(uqm_get_config_dir() -> *mut uio_DirHandle = options::configDir.cast());
accessor!(uqm_get_content_dir() -> *mut uio_DirHandle = options::contentDir.cast());
accessor!(uqm_get_content_mount_handle() -> *mut uio_MountHandle = contentMountHandle.cast());
accessor!(uqm_set_content_dir(value: *mut uio_DirHandle) = options::contentDir = value.cast());
accessor!(uqm_set_config_dir(value: *mut uio_DirHandle) = options::configDir = value.cast());
accessor!(uqm_set_save_dir(value: *mut uio_DirHandle) = options::saveDir = value.cast());
accessor!(uqm_set_melee_dir(value: *mut uio_DirHandle) = options::meleeDir = value.cast());
accessor!(uqm_set_content_mount_handle(value: *mut uio_MountHandle) = contentMountHandle = value.cast());
accessor!(uqm_set_base_content_path(path: *const c_char) = {
    let destination = ptr::addr_of_mut!(options::baseContentPath).cast::<c_char>();
    libc::strncpy(destination, path, super::dir_prep::PATH_MAX - 1);
    destination.add(super::dir_prep::PATH_MAX - 1).write(0);
});

/// # Safety
/// Data is null or points to a complete live LOCDATA on the game thread.
pub unsafe fn rust_copy_locdata_to_comm_data(data: *const std::ffi::c_void) {
    if !data.is_null() {
        crate::comm::locdata::COMM_DATA = data.cast::<crate::comm::locdata::CLocData>().read();
    }
}
