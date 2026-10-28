//! Direct graphic/font file loading, ported from filegfx.c (GPL-2.0-or-later).
use super::native_abi::*;
use std::ffi::{c_char, c_void};
use std::ptr;

use super::gfxload::{get_cel_data, get_font_data};

unsafe fn load(
    path: *const c_char,
    loader: unsafe extern "C" fn(*mut c_void, u32) -> *mut c_void,
) -> *mut c_void {
    if !_cur_resfile_name.is_null() {
        return ptr::null_mut();
    }
    let fp = res_OpenResFile(contentDir.cast(), path, c"rb".as_ptr());
    if fp.is_null() {
        return ptr::null_mut();
    }
    _cur_resfile_name = path;
    let data = loader(fp, LengthResFile(fp) as u32);
    _cur_resfile_name = ptr::null();
    res_CloseResFile(fp);
    data
}

/// # Safety
/// Path must be a valid C string; resource loading is serialized by the engine.
#[no_mangle]
pub unsafe extern "C" fn LoadGraphicFile(path: *const c_char) -> *mut drawable_desc {
    load(path, get_cel_data).cast()
}
/// # Safety
/// Path must be a valid C string; resource loading is serialized by the engine.
#[no_mangle]
pub unsafe extern "C" fn LoadFontFile(path: *const c_char) -> *mut font_desc {
    load(path, get_font_data).cast()
}
