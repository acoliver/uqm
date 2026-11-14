//! SDL_RWops over UQM streams, ported from sdluio.c (GPL-2.0-or-later).
use crate::host::{FileStream, Files};
use crate::io::uio_owned::directory;
use crate::memory::{rust_hfree, rust_hmalloc};
use sdl2_sys::*;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::io::{Read, Seek, SeekFrom, Write};
use std::ptr;

use crate::graphics::sdl::png2sdl::TFB_png_to_sdl;

unsafe fn stream(context: *mut SDL_RWops) -> *mut FileStream {
    (*context).hidden.unknown.data1.cast()
}

unsafe fn io_error(error: std::io::Error) {
    let mut message = error.to_string().into_bytes();
    message.push(0);
    SDL_SetError(c"%s".as_ptr(), message.as_ptr().cast());
}

/// # Safety
/// Directory and filename must be valid UIO inputs.
pub unsafe extern "C" fn sdluio_load_image(
    dir: *mut c_void,
    filename: *const c_char,
) -> *mut SDL_Surface {
    let files = directory(dir.cast());
    let opened = files.and_then(|files| {
        files.open_read(std::path::Path::new(
            CStr::from_ptr(filename).to_string_lossy().as_ref(),
        ))
    });
    let fp = match opened {
        Ok(file) => Box::into_raw(Box::new(file)),
        Err(error) => {
            io_error(error);
            return ptr::null_mut();
        }
    };
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
/// Context must be one created by sdluio_load_image and still open.
pub unsafe extern "C" fn sdluio_seek(context: *mut SDL_RWops, offset: i64, whence: c_int) -> i64 {
    let position = match whence {
        libc::SEEK_SET => SeekFrom::Start(offset as u64),
        libc::SEEK_CUR => SeekFrom::Current(offset),
        libc::SEEK_END => SeekFrom::End(offset),
        _ => return -1,
    };
    match (*stream(context)).seek(position) {
        Ok(position) => position as i64,
        Err(error) => {
            io_error(error);
            -1
        }
    }
}
/// # Safety
/// Context must be open and ptr must hold size * maxnum writable bytes.
pub unsafe extern "C" fn sdluio_read(
    context: *mut SDL_RWops,
    ptr: *mut c_void,
    size: usize,
    maxnum: usize,
) -> usize {
    if size == 0 || maxnum == 0 {
        return 0;
    }
    let Some(len) = size.checked_mul(maxnum) else {
        return 0;
    };
    match (*stream(context)).read(std::slice::from_raw_parts_mut(ptr.cast(), len)) {
        Ok(read) => read / size,
        Err(error) => {
            io_error(error);
            0
        }
    }
}
/// # Safety
/// Context must be open and ptr must hold size * num readable bytes.
pub unsafe extern "C" fn sdluio_write(
    context: *mut SDL_RWops,
    ptr: *const c_void,
    size: usize,
    num: usize,
) -> usize {
    if size == 0 || num == 0 {
        return 0;
    }
    let Some(len) = size.checked_mul(num) else {
        return 0;
    };
    match (*stream(context)).write(std::slice::from_raw_parts(ptr.cast(), len)) {
        Ok(written) => written / size,
        Err(error) => {
            io_error(error);
            0
        }
    }
}
/// # Safety
/// Context must be an open sdluio RWops allocation, consumed by this call.
pub unsafe extern "C" fn sdluio_close(context: *mut SDL_RWops) -> c_int {
    let result = match Box::from_raw(stream(context)).finish() {
        Ok(()) => 0,
        Err(error) => {
            io_error(error);
            -1
        }
    };
    rust_hfree(context.cast());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::decoder_vtable_tests::Directory;

    #[test]
    #[serial_test::serial]
    fn stream_callbacks_write_seek_and_read_complete_items() {
        let directory = Directory::new();
        // SAFETY: the repository owns the stream and the allocated RWops is consumed by close.
        unsafe {
            assert!(sdluio_load_image(directory.handle().cast(), c"absent.png".as_ptr()).is_null());
            let root = concat!(env!("UQM_RUST_ROOT"), "/../tmp");
            std::fs::create_dir_all(root).unwrap();
            let read_write = tempfile::tempfile_in(root).unwrap();
            let file = Box::into_raw(Box::new(FileStream::new(
                crate::io::uio_bridge::FileData::File(read_write),
            )));
            let rw: *mut SDL_RWops = rust_hmalloc(std::mem::size_of::<SDL_RWops>()).cast();
            (*rw).hidden.unknown.data1 = file.cast();
            assert_eq!(sdluio_write(rw, b"abcdef".as_ptr().cast(), 2, 3), 3);
            assert_eq!(sdluio_seek(rw, 2, libc::SEEK_SET), 2);
            let mut bytes = [0; 4];
            assert_eq!(sdluio_read(rw, bytes.as_mut_ptr().cast(), 2, 2), 2);
            assert_eq!(&bytes, b"cdef");
            assert_eq!(sdluio_read(rw, bytes.as_mut_ptr().cast(), 2, 2), 0);
            assert_eq!(sdluio_seek(rw, 0, -100), -1);
            assert_eq!(sdluio_close(rw), 0);
        }
    }
}
