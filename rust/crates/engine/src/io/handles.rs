//! Directory handles shared by resource loading and application setup.

use super::uio_bridge::{uio_DirHandle, uio_MountHandle, uio_Repository};
use std::ptr;

#[no_mangle]
pub static mut contentDir: *mut uio_DirHandle = ptr::null_mut();
#[no_mangle]
pub static mut configDir: *mut uio_DirHandle = ptr::null_mut();
#[no_mangle]
pub static mut saveDir: *mut uio_DirHandle = ptr::null_mut();
#[no_mangle]
pub static mut meleeDir: *mut uio_DirHandle = ptr::null_mut();
#[no_mangle]
pub static mut contentMountHandle: *mut uio_MountHandle = ptr::null_mut();
#[no_mangle]
pub static mut repository: *mut uio_Repository = ptr::null_mut();
#[no_mangle]
pub static mut rootDir: *mut uio_DirHandle = ptr::null_mut();
