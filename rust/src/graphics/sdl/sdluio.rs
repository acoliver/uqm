//! SDL_RWops over UQM streams, ported from sdluio.c (GPL-2.0-or-later).
use crate::io::uio_bridge;
use crate::memory::{rust_hfree, rust_hmalloc};
use sdl2_sys::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

extern "C" {
    fn TFB_png_to_sdl(src: *mut SDL_RWops) -> *mut SDL_Surface;
}

unsafe fn stream(context: *mut SDL_RWops) -> *mut uio_bridge::uio_Stream {
    (*context).hidden.unknown.data1.cast()
}

unsafe fn io_error(format: &std::ffi::CStr) {
    SDL_SetError(
        format.as_ptr(),
        libc::strerror(std::io::Error::last_os_error().raw_os_error().unwrap_or(0)),
    );
}

/// # Safety
/// Directory and filename must be valid UIO inputs.
#[no_mangle]
pub unsafe extern "C" fn sdluio_loadImage(
    dir: *mut c_void,
    filename: *const c_char,
) -> *mut SDL_Surface {
    let fp = uio_bridge::uio_fopen(dir.cast(), filename, c"rb".as_ptr());
    if fp.is_null() {
        SDL_SetError(
            c"Couldn't open '%s': %s".as_ptr(),
            filename,
            libc::strerror(std::io::Error::last_os_error().raw_os_error().unwrap_or(0)),
        );
        return ptr::null_mut();
    }
    let rw: *mut SDL_RWops = rust_hmalloc(std::mem::size_of::<SDL_RWops>()).cast();
    (*rw).seek = Some(sdluio_seek);
    (*rw).read = Some(sdluio_read);
    (*rw).write = Some(sdluio_write);
    (*rw).close = Some(sdluio_close);
    (*rw).hidden.unknown.data1 = fp.cast();
    let result = TFB_png_to_sdl(rw);
    sdluio_close(rw);
    result
}

/// # Safety
/// Context must be one created by sdluio_loadImage and still open.
#[no_mangle]
pub unsafe extern "C" fn sdluio_seek(context: *mut SDL_RWops, offset: i64, whence: c_int) -> i64 {
    if uio_bridge::uio_fseek(stream(context), offset as libc::c_long, whence) == -1 {
        io_error(c"Error seeking in uio_Stream: %s");
        return -1;
    }
    uio_bridge::uio_ftell(stream(context)) as i64
}
/// # Safety
/// Context must be open and ptr must hold size * maxnum writable bytes.
#[no_mangle]
pub unsafe extern "C" fn sdluio_read(
    context: *mut SDL_RWops,
    ptr: *mut c_void,
    size: usize,
    maxnum: usize,
) -> usize {
    let read = uio_bridge::uio_fread(ptr, size, maxnum, stream(context));
    if read == 0 && uio_bridge::uio_ferror(stream(context)) != 0 {
        io_error(c"Error reading from uio_Stream: %s");
        return 0;
    }
    (read as c_int) as usize
}
/// # Safety
/// Context must be open and ptr must hold size * num readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sdluio_write(
    context: *mut SDL_RWops,
    ptr: *const c_void,
    size: usize,
    num: usize,
) -> usize {
    let written = uio_bridge::uio_fwrite(ptr, size, num, stream(context));
    if written == 0 && uio_bridge::uio_ferror(stream(context)) != 0 {
        io_error(c"Error writing to uio_Stream: %s");
        return 0;
    }
    written
}
/// # Safety
/// Context must be an open sdluio RWops allocation, consumed by this call.
#[no_mangle]
pub unsafe extern "C" fn sdluio_close(context: *mut SDL_RWops) -> c_int {
    let result = uio_bridge::uio_fclose(stream(context));
    rust_hfree(context.cast());
    result
}
