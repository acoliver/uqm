//! C FFI bindings for Rust MOD decoder
//!
//! Provides C-compatible function pointers matching the `TFB_SoundDecoderFuncs`
//! vtable structure from `sc2/src/libs/sound/decoders/decoder.h`.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;
use std::sync::Mutex;

use crate::bridge_log::rust_bridge_log_msg;

use super::decoder::SoundDecoder;
use super::formats::DecoderFormats;
use super::mod_decoder::ModDecoder;

// Import types from the main ffi module
use super::ffi::{uio_DirHandle, TFB_DecoderFormats, TFB_SoundDecoder, TFB_SoundDecoderFuncs};

// External C functions for file I/O
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
// Rust MOD decoder wrapper for C FFI
// =============================================================================

/// Extended decoder structure with Rust-specific data
#[repr(C)]
pub struct TFB_RustModDecoder {
    /// Base decoder - must be first field
    pub base: TFB_SoundDecoder,
    /// Pointer to Rust ModDecoder (boxed)
    pub rust_decoder: *mut c_void,
}

static RUST_MOD_FORMATS: Mutex<Option<DecoderFormats>> = Mutex::new(None);

/// Decoder name as C string
static RUST_MOD_NAME: &[u8] = b"Rust MOD\0";

// =============================================================================
// FFI function implementations
// =============================================================================

extern "C" fn rust_mod_GetName() -> *const c_char {
    RUST_MOD_NAME.as_ptr() as *const c_char
}

extern "C" fn rust_mod_InitModule(flags: c_int, fmts: *const TFB_DecoderFormats) -> c_int {
    rust_bridge_log_msg(&format!(
        "RUST_MOD_INIT_MODULE: flags={} fmts={:?}",
        flags, fmts
    ));

    if fmts.is_null() {
        rust_bridge_log_msg("RUST_MOD_INIT_MODULE: ERROR - fmts is null!");
        return 0;
    }

    unsafe {
        rust_bridge_log_msg(&format!(
            "RUST_MOD_INIT_MODULE: formats mono8={} stereo8={} mono16={} stereo16={}",
            (*fmts).mono8,
            (*fmts).stereo8,
            (*fmts).mono16,
            (*fmts).stereo16
        ));

        let formats = DecoderFormats {
            big_endian: (*fmts).big_endian,
            want_big_endian: (*fmts).want_big_endian,
            mono8: (*fmts).mono8,
            stereo8: (*fmts).stereo8,
            mono16: (*fmts).mono16,
            stereo16: (*fmts).stereo16,
        };

        if let Ok(mut guard) = RUST_MOD_FORMATS.lock() {
            *guard = Some(formats);
            rust_bridge_log_msg("RUST_MOD_INIT_MODULE: formats stored successfully");
        } else {
            rust_bridge_log_msg("RUST_MOD_INIT_MODULE: ERROR - failed to lock RUST_MOD_FORMATS");
        }
    }

    1 // success
}

extern "C" fn rust_mod_TermModule() {
    rust_bridge_log_msg("RUST_MOD_TERM_MODULE");

    if let Ok(mut guard) = RUST_MOD_FORMATS.lock() {
        *guard = None;
    }
}

extern "C" fn rust_mod_GetStructSize() -> u32 {
    std::mem::size_of::<TFB_RustModDecoder>() as u32
}

extern "C" fn rust_mod_GetError(decoder: *mut TFB_SoundDecoder) -> c_int {
    if decoder.is_null() {
        return -1;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustModDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return -1;
        }

        let mod_dec = &mut *((*rust_dec).rust_decoder as *mut ModDecoder);
        mod_dec.get_error()
    }
}

extern "C" fn rust_mod_Init(decoder: *mut TFB_SoundDecoder) -> c_int {
    rust_bridge_log_msg("RUST_MOD_INIT");

    if decoder.is_null() {
        return 0;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustModDecoder;

        // Create new Rust ModDecoder
        let mut mod_dec = Box::new(ModDecoder::new());

        // Initialize with stored formats
        if let Ok(guard) = RUST_MOD_FORMATS.lock() {
            if let Some(formats) = guard.as_ref() {
                mod_dec.init_module(0, formats);
            }
        }

        mod_dec.init();

        (*rust_dec).rust_decoder = Box::into_raw(mod_dec) as *mut c_void;
        (*decoder).need_swap = false;
    }

    1 // success
}

extern "C" fn rust_mod_Term(decoder: *mut TFB_SoundDecoder) {
    rust_bridge_log_msg("RUST_MOD_TERM");

    if decoder.is_null() {
        return;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustModDecoder;
        if !(*rust_dec).rust_decoder.is_null() {
            let mod_dec = Box::from_raw((*rust_dec).rust_decoder as *mut ModDecoder);
            drop(mod_dec);
            (*rust_dec).rust_decoder = ptr::null_mut();
        }
    }
}

extern "C" fn rust_mod_Open(
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

        rust_bridge_log_msg(&format!("RUST_MOD_OPEN: {} (dir={:?})", filename_str, dir));

        // Open file via UIO
        let rust_dec = decoder as *mut TFB_RustModDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return 0;
        }

        let mod_dec = &mut *((*rust_dec).rust_decoder as *mut ModDecoder);

        // Use UIO to read the file
        let handle = uio_open(dir, filename, 0, 0); // O_RDONLY = 0
        if handle.is_null() {
            rust_bridge_log_msg(&format!(
                "RUST_MOD_OPEN_FAILED: uio_open returned null for {}",
                filename_str
            ));
            return 0;
        }

        // Get file size
        let mut stat_buf: libc::stat = std::mem::zeroed();
        if uio_fstat(handle, &mut stat_buf) != 0 {
            uio_close(handle);
            rust_bridge_log_msg("RUST_MOD_OPEN_FAILED: uio_fstat failed");
            return 0;
        }
        let file_size = stat_buf.st_size as usize;
        rust_bridge_log_msg(&format!("RUST_MOD_OPEN: file size = {} bytes", file_size));

        // Read entire file into memory
        let mut data = vec![0u8; file_size];
        let mut total_read = 0usize;
        while total_read < file_size {
            let bytes_read = uio_read(
                handle,
                data.as_mut_ptr().add(total_read),
                file_size - total_read,
            );
            if bytes_read <= 0 {
                break;
            }
            total_read += bytes_read as usize;
        }
        uio_close(handle);

        if total_read == 0 {
            rust_bridge_log_msg(&format!(
                "RUST_MOD_OPEN_FAILED: could not read any data from {}",
                filename_str
            ));
            return 0;
        }
        rust_bridge_log_msg(&format!(
            "RUST_MOD_OPEN: read {} bytes from UIO",
            total_read
        ));

        // Parse MOD data
        match mod_dec.open_from_bytes(&data[..total_read], filename_str) {
            Ok(()) => {
                // Update base decoder fields
                (*decoder).frequency = mod_dec.frequency();

                // MOD files are always stereo 16-bit
                let format_val = if let Ok(guard) = RUST_MOD_FORMATS.lock() {
                    if let Some(ref formats) = *guard {
                        formats.stereo16
                    } else {
                        23 // audio_FORMAT_STEREO16 fallback
                    }
                } else {
                    23
                };

                (*decoder).format = format_val;
                (*decoder).length = mod_dec.length();
                (*decoder).is_null = false;
                (*decoder).need_swap = mod_dec.needs_swap();

                rust_bridge_log_msg(&format!(
                    "RUST_MOD_OPEN_SUCCESS: freq={} format={} length={}",
                    (*decoder).frequency,
                    (*decoder).format,
                    (*decoder).length
                ));
                1
            }
            Err(e) => {
                rust_bridge_log_msg(&format!("RUST_MOD_OPEN_FAILED: {}", e));
                0
            }
        }
    }
}

extern "C" fn rust_mod_Close(decoder: *mut TFB_SoundDecoder) {
    rust_bridge_log_msg("RUST_MOD_CLOSE");

    if decoder.is_null() {
        return;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustModDecoder;
        if !(*rust_dec).rust_decoder.is_null() {
            let mod_dec = &mut *((*rust_dec).rust_decoder as *mut ModDecoder);
            mod_dec.close();
        }
    }
}

extern "C" fn rust_mod_Decode(
    decoder: *mut TFB_SoundDecoder,
    buf: *mut c_void,
    bufsize: i32,
) -> c_int {
    if decoder.is_null() || buf.is_null() || bufsize <= 0 {
        return -1;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustModDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return -1;
        }

        let mod_dec = &mut *((*rust_dec).rust_decoder as *mut ModDecoder);
        let buffer = std::slice::from_raw_parts_mut(buf as *mut u8, bufsize as usize);

        match mod_dec.decode(buffer) {
            Ok(bytes) => {
                // Log sparingly to avoid spam
                if bytes > 0 {
                    // rust_bridge_log_msg(&format!("RUST_MOD_DECODE: {} bytes", bytes));
                }
                bytes as c_int
            }
            Err(super::decoder::DecodeError::EndOfFile) => {
                rust_bridge_log_msg("RUST_MOD_DECODE_EOF");
                0
            }
            Err(e) => {
                rust_bridge_log_msg(&format!("RUST_MOD_DECODE_ERROR: {}", e));
                -1
            }
        }
    }
}

extern "C" fn rust_mod_Seek(decoder: *mut TFB_SoundDecoder, pcm_pos: u32) -> u32 {
    if decoder.is_null() {
        return 0;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustModDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return 0;
        }

        let mod_dec = &mut *((*rust_dec).rust_decoder as *mut ModDecoder);
        match mod_dec.seek(pcm_pos) {
            Ok(pos) => pos,
            Err(_) => pcm_pos,
        }
    }
}

extern "C" fn rust_mod_GetFrame(decoder: *mut TFB_SoundDecoder) -> u32 {
    if decoder.is_null() {
        return 0;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustModDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return 0;
        }

        let mod_dec = &*((*rust_dec).rust_decoder as *mut ModDecoder);
        mod_dec.get_frame()
    }
}

// =============================================================================
// Exported vtable
// =============================================================================

/// Rust MOD decoder vtable - exported for C linkage
#[no_mangle]
pub static rust_mod_DecoderVtbl: TFB_SoundDecoderFuncs = TFB_SoundDecoderFuncs {
    GetName: rust_mod_GetName,
    InitModule: rust_mod_InitModule,
    TermModule: rust_mod_TermModule,
    GetStructSize: rust_mod_GetStructSize,
    GetError: rust_mod_GetError,
    Init: rust_mod_Init,
    Term: rust_mod_Term,
    Open: rust_mod_Open,
    Close: rust_mod_Close,
    Decode: rust_mod_Decode,
    Seek: rust_mod_Seek,
    GetFrame: rust_mod_GetFrame,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_vtable_exists() {
        let name_ptr = (rust_mod_DecoderVtbl.GetName)();
        assert!(!name_ptr.is_null());

        let name = unsafe { CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Rust MOD");
    }

    #[test]
    fn test_mod_struct_sizes() {
        let size = rust_mod_GetStructSize();
        assert!(size > 0);
        assert!(size >= std::mem::size_of::<TFB_SoundDecoder>() as u32);
    }

    #[test]
    fn test_mod_null_decoder_handling() {
        assert_eq!(rust_mod_GetError(ptr::null_mut()), -1);
        rust_mod_Term(ptr::null_mut()); // Should not crash
        rust_mod_Close(ptr::null_mut()); // Should not crash
        assert_eq!(rust_mod_Decode(ptr::null_mut(), ptr::null_mut(), 0), -1);
        assert_eq!(rust_mod_Seek(ptr::null_mut(), 0), 0);
        assert_eq!(rust_mod_GetFrame(ptr::null_mut()), 0);
    }
}
