//! C FFI bindings for Rust sound decoders
//!
//! Provides C-compatible function pointers matching the `TFB_SoundDecoderFuncs`
//! vtable structure from `sc2/src/libs/sound/decoders/decoder.h`.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;

use crate::bridge_log::rust_bridge_log_msg;

use super::decoder::SoundDecoder;
use super::formats::DecoderFormats;
use super::ogg::OggDecoder;

// =============================================================================
// C-compatible structures matching decoder.h
// =============================================================================

/// C-compatible decoder formats structure
/// Matches TFB_DecoderFormats from decoder.h
#[repr(C)]
pub struct TFB_DecoderFormats {
    pub big_endian: bool,
    pub want_big_endian: bool,
    pub mono8: u32,
    pub stereo8: u32,
    pub mono16: u32,
    pub stereo16: u32,
}

/// Opaque C directory handle
#[repr(C)]
pub struct uio_DirHandle {
    _private: [u8; 0],
}

/// C-compatible sound decoder base structure
/// Matches TFB_SoundDecoder from decoder.h
/// Field order MUST match the C struct exactly!
#[repr(C)]
pub struct TFB_SoundDecoder {
    // decoder virtual funcs - R/O
    pub funcs: *const TFB_SoundDecoderFuncs,
    
    // public R/O, set by decoder
    pub format: u32,
    pub frequency: u32,
    pub length: f32,
    pub is_null: bool,      // C bool
    pub need_swap: bool,    // C bool
    
    // public R/O, set by wrapper  
    pub buffer: *mut c_void,
    pub buffer_size: u32,
    pub error: i32,         // sint32
    pub bytes_per_samp: u32,
    
    // public R/W
    pub looping: bool,      // C bool
    
    // semi-private - padding may be needed
    pub dir: *mut uio_DirHandle,
    pub filename: *mut c_char,
    pub pos: u32,
    pub start_sample: u32,
    pub end_sample: u32,
}

/// C-compatible decoder vtable
/// Matches TFB_SoundDecoderFuncs from decoder.h
#[repr(C)]
pub struct TFB_SoundDecoderFuncs {
    pub GetName: extern "C" fn() -> *const c_char,
    pub InitModule: extern "C" fn(flags: c_int, fmts: *const TFB_DecoderFormats) -> c_int,
    pub TermModule: extern "C" fn(),
    pub GetStructSize: extern "C" fn() -> u32,
    pub GetError: extern "C" fn(decoder: *mut TFB_SoundDecoder) -> c_int,
    pub Init: extern "C" fn(decoder: *mut TFB_SoundDecoder) -> c_int,
    pub Term: extern "C" fn(decoder: *mut TFB_SoundDecoder),
    pub Open: extern "C" fn(decoder: *mut TFB_SoundDecoder, dir: *mut uio_DirHandle, filename: *const c_char) -> c_int,
    pub Close: extern "C" fn(decoder: *mut TFB_SoundDecoder),
    pub Decode: extern "C" fn(decoder: *mut TFB_SoundDecoder, buf: *mut c_void, bufsize: i32) -> c_int,
    pub Seek: extern "C" fn(decoder: *mut TFB_SoundDecoder, pcm_pos: u32) -> u32,
    pub GetFrame: extern "C" fn(decoder: *mut TFB_SoundDecoder) -> u32,
}

// =============================================================================
// Rust Ogg decoder wrapper for C FFI
// =============================================================================

/// Extended decoder structure with Rust-specific data
#[repr(C)]
pub struct TFB_RustOggDecoder {
    /// Base decoder - must be first field
    pub base: TFB_SoundDecoder,
    /// Pointer to Rust OggDecoder (boxed)
    pub rust_decoder: *mut c_void,
}

/// Global formats storage (set during InitModule)
static RUST_OGG_FORMATS: Mutex<Option<DecoderFormats>> = Mutex::new(None);

/// Decoder name as C string
static RUST_OGG_NAME: &[u8] = b"Rust Ogg Vorbis\0";

// =============================================================================
// Vtable function implementations
// =============================================================================

extern "C" fn rust_ova_GetName() -> *const c_char {
    RUST_OGG_NAME.as_ptr() as *const c_char
}

extern "C" fn rust_ova_InitModule(flags: c_int, fmts: *const TFB_DecoderFormats) -> c_int {
    rust_bridge_log_msg("RUST_OGG_INIT_MODULE");
    
    if fmts.is_null() {
        return 0;
    }
    
    unsafe {
        let c_fmts = &*fmts;
        rust_bridge_log_msg(&format!(
            "RUST_OGG_INIT_MODULE: mono8={} stereo8={} mono16={} stereo16={}",
            c_fmts.mono8, c_fmts.stereo8, c_fmts.mono16, c_fmts.stereo16
        ));
        let formats = DecoderFormats {
            big_endian: c_fmts.big_endian,
            want_big_endian: c_fmts.want_big_endian,
            mono8: c_fmts.mono8,
            stereo8: c_fmts.stereo8,
            mono16: c_fmts.mono16,
            stereo16: c_fmts.stereo16,
        };
        
        if let Ok(mut guard) = RUST_OGG_FORMATS.lock() {
            *guard = Some(formats);
        }
    }
    
    let _ = flags; // unused for now
    1 // success
}

extern "C" fn rust_ova_TermModule() {
    rust_bridge_log_msg("RUST_OGG_TERM_MODULE");
    
    if let Ok(mut guard) = RUST_OGG_FORMATS.lock() {
        *guard = None;
    }
}

extern "C" fn rust_ova_GetStructSize() -> u32 {
    std::mem::size_of::<TFB_RustOggDecoder>() as u32
}

extern "C" fn rust_ova_GetError(decoder: *mut TFB_SoundDecoder) -> c_int {
    if decoder.is_null() {
        return -1;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustOggDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return -1;
        }
        
        let ogg = &mut *((*rust_dec).rust_decoder as *mut OggDecoder);
        ogg.get_error()
    }
}

extern "C" fn rust_ova_Init(decoder: *mut TFB_SoundDecoder) -> c_int {
    rust_bridge_log_msg("RUST_OGG_INIT");
    
    if decoder.is_null() {
        return 0;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustOggDecoder;
        
        // Create new Rust OggDecoder
        let mut ogg = Box::new(OggDecoder::new());
        
        // Initialize with stored formats
        if let Ok(guard) = RUST_OGG_FORMATS.lock() {
            if let Some(ref formats) = *guard {
                ogg.init_module(0, formats);
            }
        }
        
        ogg.init();
        
        (*rust_dec).rust_decoder = Box::into_raw(ogg) as *mut c_void;
        (*decoder).need_swap = false;
    }
    
    1 // success
}

extern "C" fn rust_ova_Term(decoder: *mut TFB_SoundDecoder) {
    rust_bridge_log_msg("RUST_OGG_TERM");
    
    if decoder.is_null() {
        return;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustOggDecoder;
        if !(*rust_dec).rust_decoder.is_null() {
            let ogg = Box::from_raw((*rust_dec).rust_decoder as *mut OggDecoder);
            drop(ogg);
            (*rust_dec).rust_decoder = ptr::null_mut();
        }
    }
}

// Import uio_open and uio_read from our io module
extern "C" {
    fn uio_open(dir: *mut uio_DirHandle, path: *const c_char, flags: c_int, mode: c_int) -> *mut c_void;
    fn uio_read(handle: *mut c_void, buf: *mut u8, count: usize) -> isize;
    fn uio_close(handle: *mut c_void) -> c_int;
    fn uio_fstat(handle: *mut c_void, stat_buf: *mut libc::stat) -> c_int;
}

extern "C" fn rust_ova_Open(
    decoder: *mut TFB_SoundDecoder,
    dir: *mut uio_DirHandle,
    filename: *const c_char,
) -> c_int {
    if decoder.is_null() || filename.is_null() {
        return 0;
    }
    
    let filename_str = unsafe {
        match CStr::from_ptr(filename).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };
    
    rust_bridge_log_msg(&format!("RUST_OGG_OPEN: {} (dir={:?})", filename_str, dir));
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustOggDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return 0;
        }
        
        let ogg = &mut *((*rust_dec).rust_decoder as *mut OggDecoder);
        
        // Use UIO to open and read the file through the virtual filesystem
        // This handles addons, zip files, and path resolution correctly
        let handle = uio_open(dir, filename, 0 /* O_RDONLY */, 0);
        if handle.is_null() {
            rust_bridge_log_msg(&format!("RUST_OGG_OPEN_FAILED: uio_open returned null for {}", filename_str));
            return 0;
        }
        
        // Get file size via fstat
        let mut stat_buf: libc::stat = std::mem::zeroed();
        if uio_fstat(handle, &mut stat_buf) != 0 {
            rust_bridge_log_msg("RUST_OGG_OPEN_FAILED: uio_fstat failed");
            uio_close(handle);
            return 0;
        }
        let file_size = stat_buf.st_size as usize;
        rust_bridge_log_msg(&format!("RUST_OGG_OPEN: file size = {} bytes", file_size));
        
        // Read entire file into memory
        let mut data = vec![0u8; file_size];
        let mut total_read = 0usize;
        while total_read < file_size {
            let n = uio_read(handle, data.as_mut_ptr().add(total_read), file_size - total_read);
            if n <= 0 {
                break;
            }
            total_read += n as usize;
        }
        uio_close(handle);
        
        if total_read == 0 {
            rust_bridge_log_msg(&format!("RUST_OGG_OPEN_FAILED: could not read any data from {}", filename_str));
            return 0;
        }
        
        rust_bridge_log_msg(&format!("RUST_OGG_OPEN: read {} bytes from UIO", total_read));
        
        // Open from bytes instead of path
        match ogg.open_from_bytes(&data[..total_read], filename_str) {
            Ok(()) => {
                // Update base decoder fields
                (*decoder).frequency = ogg.frequency();
                
                // Get format codes from stored formats (set during InitModule)
                // These are the audio_FORMAT_* enum values passed from C
                let format_code = if let Ok(guard) = RUST_OGG_FORMATS.lock() {
                    if let Some(ref formats) = *guard {
                        match ogg.format() {
                            super::formats::AudioFormat::Mono8 => formats.mono8,
                            super::formats::AudioFormat::Mono16 => formats.mono16,
                            super::formats::AudioFormat::Stereo8 => formats.stereo8,
                            super::formats::AudioFormat::Stereo16 => formats.stereo16,
                        }
                    } else {
                        // Fallback - shouldn't happen if InitModule was called
                        rust_bridge_log_msg("RUST_OGG_OPEN: WARNING - no formats set, using fallback");
                        match ogg.format() {
                            super::formats::AudioFormat::Mono8 => 24,    // audio_FORMAT_MONO8
                            super::formats::AudioFormat::Mono16 => 22,   // audio_FORMAT_MONO16  
                            super::formats::AudioFormat::Stereo8 => 25,  // audio_FORMAT_STEREO8
                            super::formats::AudioFormat::Stereo16 => 23, // audio_FORMAT_STEREO16
                        }
                    }
                } else {
                    rust_bridge_log_msg("RUST_OGG_OPEN: WARNING - could not lock formats");
                    23 // audio_FORMAT_STEREO16 as default
                };
                
                (*decoder).format = format_code;
                (*decoder).length = ogg.length();
                (*decoder).is_null = false;
                (*decoder).need_swap = ogg.needs_swap();
                
                rust_bridge_log_msg(&format!(
                    "RUST_OGG_OPEN_SUCCESS: freq={} format={} length={}",
                    (*decoder).frequency, (*decoder).format, (*decoder).length
                ));
                1 // success
            }
            Err(e) => {
                rust_bridge_log_msg(&format!("RUST_OGG_OPEN_FAILED: {}", e));
                0 // failure
            }
        }
    }
}

extern "C" fn rust_ova_Close(decoder: *mut TFB_SoundDecoder) {
    rust_bridge_log_msg("RUST_OGG_CLOSE");
    
    if decoder.is_null() {
        return;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustOggDecoder;
        if !(*rust_dec).rust_decoder.is_null() {
            let ogg = &mut *((*rust_dec).rust_decoder as *mut OggDecoder);
            ogg.close();
        }
    }
}

extern "C" fn rust_ova_Decode(
    decoder: *mut TFB_SoundDecoder,
    buf: *mut c_void,
    bufsize: i32,
) -> c_int {
    if decoder.is_null() || buf.is_null() || bufsize <= 0 {
        return -1;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustOggDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return -1;
        }
        
        let ogg = &mut *((*rust_dec).rust_decoder as *mut OggDecoder);
        let buffer = std::slice::from_raw_parts_mut(buf as *mut u8, bufsize as usize);
        
        match ogg.decode(buffer) {
            Ok(bytes) => {
                rust_bridge_log_msg(&format!("RUST_OGG_DECODE: {} bytes", bytes));
                bytes as c_int
            }
            Err(super::decoder::DecodeError::EndOfFile) => {
                rust_bridge_log_msg("RUST_OGG_DECODE_EOF");
                0
            }
            Err(e) => {
                rust_bridge_log_msg(&format!("RUST_OGG_DECODE_ERROR: {}", e));
                -1
            }
        }
    }
}

extern "C" fn rust_ova_Seek(decoder: *mut TFB_SoundDecoder, pcm_pos: u32) -> u32 {
    rust_bridge_log_msg(&format!("RUST_OGG_SEEK: pcm_pos={}", pcm_pos));
    
    if decoder.is_null() {
        return 0;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustOggDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return 0;
        }
        
        let ogg = &mut *((*rust_dec).rust_decoder as *mut OggDecoder);
        match ogg.seek(pcm_pos) {
            Ok(pos) => {
                rust_bridge_log_msg(&format!("RUST_OGG_SEEK_OK: pos={}", pos));
                pos
            }
            Err(e) => {
                rust_bridge_log_msg(&format!("RUST_OGG_SEEK_ERROR: {}", e));
                pcm_pos // Return requested position on error
            }
        }
    }
}

extern "C" fn rust_ova_GetFrame(decoder: *mut TFB_SoundDecoder) -> u32 {
    if decoder.is_null() {
        return 0;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustOggDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return 0;
        }
        
        let ogg = &*((*rust_dec).rust_decoder as *mut OggDecoder);
        ogg.get_frame()
    }
}

// =============================================================================
// Exported vtable
// =============================================================================

/// Rust Ogg Vorbis decoder vtable - exported for C linkage
#[no_mangle]
pub static rust_ova_DecoderVtbl: TFB_SoundDecoderFuncs = TFB_SoundDecoderFuncs {
    GetName: rust_ova_GetName,
    InitModule: rust_ova_InitModule,
    TermModule: rust_ova_TermModule,
    GetStructSize: rust_ova_GetStructSize,
    GetError: rust_ova_GetError,
    Init: rust_ova_Init,
    Term: rust_ova_Term,
    Open: rust_ova_Open,
    Close: rust_ova_Close,
    Decode: rust_ova_Decode,
    Seek: rust_ova_Seek,
    GetFrame: rust_ova_GetFrame,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vtable_exists() {
        // Verify the vtable is properly initialized
        let name_ptr = (rust_ova_DecoderVtbl.GetName)();
        assert!(!name_ptr.is_null());
        
        let name = unsafe { CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Rust Ogg Vorbis");
    }

    #[test]
    fn test_struct_sizes() {
        // Verify struct sizes are reasonable
        let size = rust_ova_GetStructSize();
        assert!(size > 0);
        assert!(size >= std::mem::size_of::<TFB_SoundDecoder>() as u32);
    }

    #[test]
    fn test_init_module() {
        let formats = TFB_DecoderFormats {
            big_endian: false,
            want_big_endian: false,
            mono8: 0x1100,
            stereo8: 0x1102,
            mono16: 0x1101,
            stereo16: 0x1103,
        };
        
        let result = rust_ova_InitModule(0, &formats);
        assert_eq!(result, 1);
        
        rust_ova_TermModule();
    }

    #[test]
    fn test_null_decoder_handling() {
        // All functions should handle null decoder gracefully
        assert_eq!(rust_ova_GetError(ptr::null_mut()), -1);
        rust_ova_Term(ptr::null_mut()); // Should not crash
        rust_ova_Close(ptr::null_mut()); // Should not crash
        assert_eq!(rust_ova_Decode(ptr::null_mut(), ptr::null_mut(), 0), -1);
        assert_eq!(rust_ova_Seek(ptr::null_mut(), 0), 0);
        assert_eq!(rust_ova_GetFrame(ptr::null_mut()), 0);
    }
}
