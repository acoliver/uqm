//! C FFI bindings for Rust AIFF decoder
//!
//! Provides C-compatible function pointers matching the `TFB_SoundDecoderFuncs`
//! vtable structure, replacing `aiffaud.c`.
//!
//! @plan PLAN-20260225-AIFF-DECODER.P15..P17
//! @requirement REQ-FF-1..15

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;
use std::sync::Mutex;

use crate::bridge_log::rust_bridge_log_msg;

use super::aiff::AiffDecoder;
use super::decoder::SoundDecoder;
use super::formats::{AudioFormat, DecoderFormats};

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
pub struct TFB_RustAiffDecoder {
    pub base: TFB_SoundDecoder,
    pub rust_decoder: *mut c_void,
}

static RUST_AIFA_FORMATS: Mutex<Option<DecoderFormats>> = Mutex::new(None);

static RUST_AIFA_NAME: &[u8] = b"Rust AIFF\0";

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

extern "C" fn rust_aifa_GetName() -> *const c_char {
    RUST_AIFA_NAME.as_ptr() as *const c_char
}

extern "C" fn rust_aifa_InitModule(flags: c_int, fmts: *const TFB_DecoderFormats) -> c_int {
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
        if let Ok(mut guard) = RUST_AIFA_FORMATS.lock() {
            *guard = Some(formats);
        }
    }
    let _ = flags;
    1
}

extern "C" fn rust_aifa_TermModule() {
    if let Ok(mut guard) = RUST_AIFA_FORMATS.lock() {
        *guard = None;
    }
}

extern "C" fn rust_aifa_GetStructSize() -> u32 {
    std::mem::size_of::<TFB_RustAiffDecoder>() as u32
}

extern "C" fn rust_aifa_GetError(decoder: *mut TFB_SoundDecoder) -> c_int {
    if decoder.is_null() {
        return -1;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustAiffDecoder;
        if (*rd).rust_decoder.is_null() {
            return -1;
        }
        let dec = &mut *((*rd).rust_decoder as *mut AiffDecoder);
        dec.get_error()
    }
}

// REQ-FF-4: Init allocates Box<AiffDecoder> and propagates formats
extern "C" fn rust_aifa_Init(decoder: *mut TFB_SoundDecoder) -> c_int {
    if decoder.is_null() {
        return 0;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustAiffDecoder;
        let mut dec = Box::new(AiffDecoder::new());

        // Propagate formats from global Mutex to instance
        if let Ok(guard) = RUST_AIFA_FORMATS.lock() {
            if let Some(ref formats) = *guard {
                dec.init_module(0, formats);
            }
        }
        dec.init();

        (*rd).rust_decoder = Box::into_raw(dec) as *mut c_void;
        (*decoder).need_swap = false;
    }
    1
}

// REQ-FF-5: Term deallocates
extern "C" fn rust_aifa_Term(decoder: *mut TFB_SoundDecoder) {
    if decoder.is_null() {
        return;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustAiffDecoder;
        if !(*rd).rust_decoder.is_null() {
            let dec = Box::from_raw((*rd).rust_decoder as *mut AiffDecoder);
            drop(dec);
            (*rd).rust_decoder = ptr::null_mut();
        }
    }
}

// REQ-FF-6..8: Open reads file via UIO and calls open_from_bytes
extern "C" fn rust_aifa_Open(
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

        rust_bridge_log_msg(&format!("RUST_AIFF_OPEN: {}", filename_str));

        let rd = decoder as *mut TFB_RustAiffDecoder;
        if (*rd).rust_decoder.is_null() {
            return 0;
        }

        let dec = &mut *((*rd).rust_decoder as *mut AiffDecoder);

        // Read file via UIO
        let file_data = match read_uio_file(dir, filename) {
            Some(d) => d,
            None => {
                rust_bridge_log_msg(&format!("RUST_AIFF_OPEN: failed to read {}", filename_str));
                return 0;
            }
        };

        match dec.open_from_bytes(&file_data, filename_str) {
            Ok(()) => {
                // Update base struct fields
                (*decoder).frequency = dec.frequency();

                if let Ok(guard) = RUST_AIFA_FORMATS.lock() {
                    if let Some(ref formats) = *guard {
                        let format_code = match dec.format() {
                            AudioFormat::Mono8 => formats.mono8,
                            AudioFormat::Stereo8 => formats.stereo8,
                            AudioFormat::Mono16 => formats.mono16,
                            AudioFormat::Stereo16 => formats.stereo16,
                        };
                        (*decoder).format = format_code;
                    } else {
                        rust_bridge_log_msg(
                            "RUST_AIFF_OPEN: formats not initialized (InitModule not called)",
                        );
                        return 0;
                    }
                }

                (*decoder).length = dec.length();
                (*decoder).is_null = false;
                (*decoder).need_swap = dec.needs_swap();

                rust_bridge_log_msg(&format!(
                    "RUST_AIFF_OPEN: OK freq={} format={:?} length={:.2}s swap={}",
                    dec.frequency(),
                    dec.format(),
                    dec.length(),
                    dec.needs_swap()
                ));
                1
            }
            Err(e) => {
                rust_bridge_log_msg(&format!("RUST_AIFF_OPEN: error: {:?}", e));
                0
            }
        }
    }
}

// REQ-FF-15: Close
extern "C" fn rust_aifa_Close(decoder: *mut TFB_SoundDecoder) {
    if decoder.is_null() {
        return;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustAiffDecoder;
        if !(*rd).rust_decoder.is_null() {
            let dec = &mut *((*rd).rust_decoder as *mut AiffDecoder);
            dec.close();
        }
    }
}

// REQ-FF-9: Decode
extern "C" fn rust_aifa_Decode(
    decoder: *mut TFB_SoundDecoder,
    buf: *mut c_void,
    bufsize: c_int,
) -> c_int {
    if decoder.is_null() || buf.is_null() || bufsize <= 0 {
        return 0;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustAiffDecoder;
        if (*rd).rust_decoder.is_null() {
            return 0;
        }
        let dec = &mut *((*rd).rust_decoder as *mut AiffDecoder);
        let slice = std::slice::from_raw_parts_mut(buf as *mut u8, bufsize as usize);
        match dec.decode(slice) {
            Ok(n) => n as c_int,
            Err(_) => 0,
        }
    }
}

// REQ-FF-13: Seek
extern "C" fn rust_aifa_Seek(decoder: *mut TFB_SoundDecoder, pcm_pos: u32) -> u32 {
    if decoder.is_null() {
        return pcm_pos;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustAiffDecoder;
        if (*rd).rust_decoder.is_null() {
            return pcm_pos;
        }
        let dec = &mut *((*rd).rust_decoder as *mut AiffDecoder);
        match dec.seek(pcm_pos) {
            Ok(pos) => pos,
            Err(_) => pcm_pos,
        }
    }
}

// REQ-FF-14: GetFrame
extern "C" fn rust_aifa_GetFrame(decoder: *mut TFB_SoundDecoder) -> u32 {
    if decoder.is_null() {
        return 0;
    }
    unsafe {
        let rd = decoder as *mut TFB_RustAiffDecoder;
        if (*rd).rust_decoder.is_null() {
            return 0;
        }
        let dec = &*((*rd).rust_decoder as *mut AiffDecoder);
        dec.get_frame()
    }
}

// =============================================================================
// Vtable export
// =============================================================================

#[no_mangle]
pub static rust_aifa_DecoderVtbl: TFB_SoundDecoderFuncs = TFB_SoundDecoderFuncs {
    GetName: rust_aifa_GetName,
    InitModule: rust_aifa_InitModule,
    TermModule: rust_aifa_TermModule,
    GetStructSize: rust_aifa_GetStructSize,
    GetError: rust_aifa_GetError,
    Init: rust_aifa_Init,
    Term: rust_aifa_Term,
    Open: rust_aifa_Open,
    Close: rust_aifa_Close,
    Decode: rust_aifa_Decode,
    Seek: rust_aifa_Seek,
    GetFrame: rust_aifa_GetFrame,
};
