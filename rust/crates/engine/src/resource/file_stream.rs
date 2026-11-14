//! Resource ABI stream ownership; loaders borrow the same FileStream.
use super::*;
use crate::host::FileStream;
use std::io::{Read, Seek, SeekFrom, Write};
pub(super) unsafe fn stream_open(
    dir: *mut c_void,
    path: *const c_char,
    mode: *const c_char,
) -> *mut c_void {
    // SAFETY: the resource ABI supplies a live directory and terminated path/mode strings.
    let result = unsafe {
        (|| {
            let files = directory(dir.cast())?;
            let name = CStr::from_ptr(path).to_string_lossy();
            let mode = CStr::from_ptr(mode).to_bytes();
            let file = if mode.contains(&b'w') {
                files.open_write(Path::new(name.as_ref()))?
            } else if mode.contains(&b'r') {
                files.open_read(Path::new(name.as_ref()))?
            } else {
                return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
            };
            Ok(file)
        })()
    };
    match result {
        Ok(file) => Box::into_raw(Box::new(file)).cast(),
        Err(error) => {
            crate::platform::set_errno(crate::platform::io_errno(&error));
            ptr::null_mut()
        }
    }
}

pub(super) unsafe fn stream_close(fp: *mut c_void) -> c_int {
    // SAFETY: the resource ABI consumes exactly the allocation returned by stream_open.
    let file = unsafe { Box::from_raw(fp.cast::<FileStream>()) };
    if file.finish().is_ok() {
        0
    } else {
        -1
    }
}
pub(super) unsafe fn stream_read(
    buf: *mut c_void,
    size: usize,
    count: usize,
    fp: *mut c_void,
) -> usize {
    if size == 0 || count == 0 {
        return 0;
    }
    let Some(length) = size.checked_mul(count) else {
        return 0;
    };
    // SAFETY: the resource ABI supplies a live exclusive stream and length writable bytes.
    unsafe {
        let bytes = std::slice::from_raw_parts_mut(buf.cast(), length);
        (&mut *fp.cast::<FileStream>()).read(bytes).unwrap_or(0) / size
    }
}
pub(super) unsafe fn stream_write(
    buf: *const c_void,
    size: usize,
    count: usize,
    fp: *mut c_void,
) -> usize {
    if size == 0 || count == 0 {
        return 0;
    }
    let Some(length) = size.checked_mul(count) else {
        return 0;
    };
    // SAFETY: the resource ABI supplies a live exclusive stream and length readable bytes.
    unsafe {
        let bytes = std::slice::from_raw_parts(buf.cast(), length);
        (&mut *fp.cast::<FileStream>()).write(bytes).unwrap_or(0) / size
    }
}
pub(super) unsafe fn stream_seek(fp: *mut c_void, offset: c_long, whence: c_int) -> c_int {
    #[cfg(any(windows, target_pointer_width = "32"))]
    let offset = i64::from(offset);
    let position = match whence {
        0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return -1,
    };
    // SAFETY: the resource ABI lends a live exclusive FileStream.
    unsafe { (&mut *fp.cast::<FileStream>()).seek(position) }.map_or(-1, |_| 0)
}
pub(super) unsafe fn stream_tell(fp: *mut c_void) -> c_long {
    // SAFETY: the resource ABI supplies a live FileStream.
    unsafe { (&*fp.cast::<FileStream>()).position() as c_long }
}
pub(super) unsafe fn stream_get_char(fp: *mut c_void) -> c_int {
    let mut byte = [0];
    // SAFETY: the resource ABI lends a live exclusive FileStream.
    if unsafe { (&mut *fp.cast::<FileStream>()).read(&mut byte) }.ok() == Some(1) {
        byte[0] as c_int
    } else {
        -1
    }
}
pub(super) unsafe fn stream_put_char(ch: c_int, fp: *mut c_void) -> c_int {
    // SAFETY: the resource ABI lends a live exclusive FileStream.
    if unsafe { (&mut *fp.cast::<FileStream>()).write(&[ch as u8]) }.ok() == Some(1) {
        ch as u8 as c_int
    } else {
        -1
    }
}
