//! C FFI bindings for Rust DukAud decoder
//!
//! Provides C-compatible function pointers matching the `TFB_SoundDecoderFuncs`
//! vtable structure, replacing `dukaud.c`.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;
use std::sync::Mutex;

use crate::bridge_log::rust_bridge_log_msg;

use super::dukaud::DukAudDecoder;
use super::formats::DecoderFormats;

use super::ffi::{uio_DirHandle, TFB_DecoderFormats, TFB_SoundDecoder, TFB_SoundDecoderFuncs};

extern "C" {
    fn uio_open(
        dir: *mut uio_DirHandle,
        path: *const c_char,
        flags: c_int,
        mode: c_int,
    ) -> *mut c_void;
    fn uio_read(handle: *mut c_void, buf: *mut u8, count: usize) -> isize;
    fn uio_close(handle: *mut c_void) -> c_int;
    fn uio_fstat(handle: *mut c_void, stat_buf: *mut libc::stat) -> c_int;
}

// =============================================================================
// FFI wrapper struct
// =============================================================================

#[repr(C)]
pub struct TFB_RustDukAudDecoder {
    pub base: TFB_SoundDecoder,
    pub rust_decoder: *mut c_void,
}

static RUST_DUKA_FORMATS: Mutex<Option<DecoderFormats>> = Mutex::new(None);

static RUST_DUKA_NAME: &[u8] = b"Rust DukAud\0";

// =============================================================================
// Helper: read a file via uio into a Vec<u8>
// =============================================================================

unsafe fn read_uio_file(dir: *mut uio_DirHandle, path: *const c_char) -> Option<Vec<u8>> {
    let handle = uio_open(dir, path, 0, 0);
    if handle.is_null() {
        return None;
    }

    let mut stat_buf: libc::stat = std::mem::zeroed();
    if uio_fstat(handle, &mut stat_buf) != 0 {
        uio_close(handle);
        return None;
    }
    let size = stat_buf.st_size as usize;

    let mut data = vec![0u8; size];
    let mut total = 0usize;
    while total < size {
        let n = uio_read(handle, data.as_mut_ptr().add(total), size - total);
        if n <= 0 {
            break;
        }
        total += n as usize;
    }
    uio_close(handle);

    if total == 0 {
        return None;
    }
    data.truncate(total);
    Some(data)
}

// =============================================================================
// Vtable functions
// =============================================================================

extern "C" fn rust_duka_GetName() -> *const c_char {
    RUST_DUKA_NAME.as_ptr() as *const c_char
}

extern "C" fn rust_duka_InitModule(flags: c_int, fmts: *const TFB_DecoderFormats) -> c_int {
    if fmts.is_null() {
        return 0;
    }
    unsafe {
        let formats = DecoderFormats {
            big_endian: (*fmts).big_endian,
            want_big_endian: (*fmts).want_big_endian,
            mono8: (*fmts).mono8,
            stereo8: (*fmts).stereo8,
            mono16: (*fmts).mono16,
            stereo16: (*fmts).stereo16,
        };
        if let Ok(mut guard) = RUST_DUKA_FORMATS.lock() {
            *guard = Some(formats);
        }
    }
    let _ = flags;
    1
}

extern "C" fn rust_duka_TermModule() {
    if let Ok(mut guard) = RUST_DUKA_FORMATS.lock() {
        *guard = None;
    }
}

extern "C" fn rust_duka_GetStructSize() -> u32 {
    std::mem::size_of::<TFB_RustDukAudDecoder>() as u32
}

extern "C" fn rust_duka_GetError(decoder: *mut TFB_SoundDecoder) -> c_int {
    if decoder.is_null() {
        return -1;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustDukAudDecoder;
        if (*rd).rust_decoder.is_null() {
            return -1;
        }
        let dec = &mut *((*rd).rust_decoder as *mut DukAudDecoder);
        dec.get_error()
    }
}

extern "C" fn rust_duka_Init(decoder: *mut TFB_SoundDecoder) -> c_int {
    if decoder.is_null() {
        return 0;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustDukAudDecoder;
        let dec = Box::new(DukAudDecoder::new());
        (*rd).rust_decoder = Box::into_raw(dec) as *mut c_void;
        (*decoder).need_swap = false;
    }
    1
}

extern "C" fn rust_duka_Term(decoder: *mut TFB_SoundDecoder) {
    if decoder.is_null() {
        return;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustDukAudDecoder;
        if !(*rd).rust_decoder.is_null() {
            let dec = Box::from_raw((*rd).rust_decoder as *mut DukAudDecoder);
            drop(dec);
            (*rd).rust_decoder = ptr::null_mut();
        }
    }
}

extern "C" fn rust_duka_Open(
    decoder: *mut TFB_SoundDecoder,
    dir: *mut uio_DirHandle,
    filename: *const c_char,
) -> c_int {
    if decoder.is_null() || filename.is_null() {
        return 0;
    }

    unsafe {
        let filename_str = match CStr::from_ptr(filename).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        rust_bridge_log_msg(&format!("RUST_DUKAUD_OPEN: {}", filename_str));

        let rd = decoder as *mut TFB_RustDukAudDecoder;
        if (*rd).rust_decoder.is_null() {
            return 0;
        }

        // Read .duk file
        let duk_data = match read_uio_file(dir, filename) {
            Some(d) => d,
            None => {
                rust_bridge_log_msg(&format!(
                    "RUST_DUKAUD_OPEN: failed to read {}",
                    filename_str
                ));
                return 0;
            }
        };

        // Build .frm filename (replace last 3 chars with "frm")
        let mut frm_name = filename_str.to_string();
        if frm_name.len() >= 3 {
            let cut = frm_name.len() - 3;
            frm_name.truncate(cut);
            frm_name.push_str("frm");
        } else {
            return 0;
        }
        let frm_cstr = match std::ffi::CString::new(frm_name.as_str()) {
            Ok(c) => c,
            Err(_) => return 0,
        };

        let frm_data = match read_uio_file(dir, frm_cstr.as_ptr()) {
            Some(d) => d,
            None => {
                rust_bridge_log_msg(&format!(
                    "RUST_DUKAUD_OPEN: failed to read {}",
                    frm_name
                ));
                return 0;
            }
        };

        let dec = &mut *((*rd).rust_decoder as *mut DukAudDecoder);
        match dec.open_from_data(&duk_data, &frm_data) {
            Ok(()) => {
                (*decoder).frequency = dec.frequency();

                // Map format to C enum value via stored formats
                let format_val = if let Ok(guard) = RUST_DUKA_FORMATS.lock() {
                    if let Some(ref fmts) = *guard {
                        fmts.stereo16 // DukAud is always stereo 16-bit
                    } else {
                        23 // fallback audio_FORMAT_STEREO16
                    }
                } else {
                    23
                };

                (*decoder).format = format_val;
                (*decoder).length = dec.length();
                (*decoder).is_null = false;
                (*decoder).need_swap = false;

                rust_bridge_log_msg(&format!(
                    "RUST_DUKAUD_OPEN: OK freq={} format={} length={:.2}s",
                    dec.frequency(),
                    format_val,
                    dec.length()
                ));

                1
            }
            Err(e) => {
                rust_bridge_log_msg(&format!("RUST_DUKAUD_OPEN: failed: {}", e));
                0
            }
        }
    }
}

extern "C" fn rust_duka_Close(decoder: *mut TFB_SoundDecoder) {
    if decoder.is_null() {
        return;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustDukAudDecoder;
        if !(*rd).rust_decoder.is_null() {
            let dec = &mut *((*rd).rust_decoder as *mut DukAudDecoder);
            dec.close();
        }
    }
}

extern "C" fn rust_duka_Decode(
    decoder: *mut TFB_SoundDecoder,
    buf: *mut c_void,
    bufsize: i32,
) -> c_int {
    if decoder.is_null() || buf.is_null() || bufsize <= 0 {
        return -1;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustDukAudDecoder;
        if (*rd).rust_decoder.is_null() {
            return -1;
        }
        let dec = &mut *((*rd).rust_decoder as *mut DukAudDecoder);
        let buffer = std::slice::from_raw_parts_mut(buf as *mut u8, bufsize as usize);
        match dec.decode(buffer) {
            Ok(n) => n as c_int,
            Err(super::decoder::DecodeError::EndOfFile) => 0,
            Err(_) => -1,
        }
    }
}

extern "C" fn rust_duka_Seek(decoder: *mut TFB_SoundDecoder, pcm_pos: u32) -> u32 {
    if decoder.is_null() {
        return 0;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustDukAudDecoder;
        if (*rd).rust_decoder.is_null() {
            return 0;
        }
        let dec = &mut *((*rd).rust_decoder as *mut DukAudDecoder);
        match dec.seek(pcm_pos) {
            Ok(pos) => pos,
            Err(_) => pcm_pos,
        }
    }
}

extern "C" fn rust_duka_GetFrame(decoder: *mut TFB_SoundDecoder) -> u32 {
    if decoder.is_null() {
        return 0;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustDukAudDecoder;
        if (*rd).rust_decoder.is_null() {
            return 0;
        }
        let dec = &*((*rd).rust_decoder as *mut DukAudDecoder);
        dec.get_frame()
    }
}

// =============================================================================
// Exported vtable
// =============================================================================

#[no_mangle]
pub static rust_duka_DecoderVtbl: TFB_SoundDecoderFuncs = TFB_SoundDecoderFuncs {
    GetName: rust_duka_GetName,
    InitModule: rust_duka_InitModule,
    TermModule: rust_duka_TermModule,
    GetStructSize: rust_duka_GetStructSize,
    GetError: rust_duka_GetError,
    Init: rust_duka_Init,
    Term: rust_duka_Term,
    Open: rust_duka_Open,
    Close: rust_duka_Close,
    Decode: rust_duka_Decode,
    Seek: rust_duka_Seek,
    GetFrame: rust_duka_GetFrame,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duka_vtable_name() {
        let name_ptr = (rust_duka_DecoderVtbl.GetName)();
        assert!(!name_ptr.is_null());
        let name = unsafe { CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Rust DukAud");
    }

    #[test]
    fn test_duka_struct_size() {
        let size = rust_duka_GetStructSize();
        assert!(size > 0);
        assert!(size >= std::mem::size_of::<TFB_SoundDecoder>() as u32);
    }

    #[test]
    fn test_duka_null_handling() {
        assert_eq!(rust_duka_GetError(ptr::null_mut()), -1);
        rust_duka_Term(ptr::null_mut());
        rust_duka_Close(ptr::null_mut());
        assert_eq!(rust_duka_Decode(ptr::null_mut(), ptr::null_mut(), 0), -1);
        assert_eq!(rust_duka_Seek(ptr::null_mut(), 0), 0);
        assert_eq!(rust_duka_GetFrame(ptr::null_mut()), 0);
    }
}
