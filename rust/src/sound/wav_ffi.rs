//! C FFI bindings for Rust WAV decoder
//!
//! Provides C-compatible function pointers matching the `TFB_SoundDecoderFuncs`
//! vtable structure from `sc2/src/libs/sound/decoders/decoder.h`.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;
use std::sync::Mutex;

use crate::bridge_log::rust_bridge_log_msg;

use super::decoder::SoundDecoder;
use super::formats::DecoderFormats;
use super::wav::WavDecoder;

// Import types from the main ffi module
use super::ffi::{TFB_DecoderFormats, TFB_SoundDecoder, TFB_SoundDecoderFuncs, uio_DirHandle};

// External C functions for file I/O
extern "C" {
    fn uio_open(dir: *mut uio_DirHandle, path: *const c_char, flags: c_int, mode: c_int) -> *mut c_void;
    fn uio_read(handle: *mut c_void, buf: *mut u8, count: usize) -> isize;
    fn uio_close(handle: *mut c_void) -> c_int;
    fn uio_fstat(handle: *mut c_void, stat_buf: *mut libc::stat) -> c_int;
}

// =============================================================================
// Rust WAV decoder wrapper for C FFI
// =============================================================================

/// Extended decoder structure with Rust-specific data
#[repr(C)]
pub struct TFB_RustWavDecoder {
    /// Base decoder - must be first field
    pub base: TFB_SoundDecoder,
    /// Pointer to Rust WavDecoder (boxed)
    pub rust_decoder: *mut c_void,
}

static RUST_WAV_FORMATS: Mutex<Option<DecoderFormats>> = Mutex::new(None);

/// Decoder name as C string
static RUST_WAV_NAME: &[u8] = b"Rust Wave\0";

// =============================================================================
// FFI function implementations
// =============================================================================

extern "C" fn rust_wav_GetName() -> *const c_char {
    RUST_WAV_NAME.as_ptr() as *const c_char
}

extern "C" fn rust_wav_InitModule(flags: c_int, fmts: *const TFB_DecoderFormats) -> c_int {
    rust_bridge_log_msg(&format!("RUST_WAV_INIT_MODULE: flags={} fmts={:?}", flags, fmts));
    
    if fmts.is_null() {
        rust_bridge_log_msg("RUST_WAV_INIT_MODULE: ERROR - fmts is null!");
        return 0;
    }
    
    unsafe {
        // Log the format values we received from C
        rust_bridge_log_msg(&format!(
            "RUST_WAV_INIT_MODULE: formats mono8={} stereo8={} mono16={} stereo16={}",
            (*fmts).mono8, (*fmts).stereo8, (*fmts).mono16, (*fmts).stereo16
        ));
        
        let formats = DecoderFormats {
            big_endian: (*fmts).big_endian,
            want_big_endian: (*fmts).want_big_endian,
            mono8: (*fmts).mono8,
            stereo8: (*fmts).stereo8,
            mono16: (*fmts).mono16,
            stereo16: (*fmts).stereo16,
        };
        
        if let Ok(mut guard) = RUST_WAV_FORMATS.lock() {
            *guard = Some(formats);
            rust_bridge_log_msg("RUST_WAV_INIT_MODULE: formats stored successfully");
        } else {
            rust_bridge_log_msg("RUST_WAV_INIT_MODULE: ERROR - failed to lock RUST_WAV_FORMATS");
        }
    }
    
    1 // success
}

extern "C" fn rust_wav_TermModule() {
    rust_bridge_log_msg("RUST_WAV_TERM_MODULE");
    
    if let Ok(mut guard) = RUST_WAV_FORMATS.lock() {
        *guard = None;
    }
}

extern "C" fn rust_wav_GetStructSize() -> u32 {
    std::mem::size_of::<TFB_RustWavDecoder>() as u32
}

extern "C" fn rust_wav_GetError(decoder: *mut TFB_SoundDecoder) -> c_int {
    if decoder.is_null() {
        return -1;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustWavDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return -1;
        }
        
        let wav = &mut *((*rust_dec).rust_decoder as *mut WavDecoder);
        wav.get_error()
    }
}

extern "C" fn rust_wav_Init(decoder: *mut TFB_SoundDecoder) -> c_int {
    rust_bridge_log_msg("RUST_WAV_INIT");
    
    if decoder.is_null() {
        return 0;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustWavDecoder;
        
        // Create new Rust WavDecoder
        let mut wav = Box::new(WavDecoder::new());
        
        // Initialize with stored formats
        if let Ok(guard) = RUST_WAV_FORMATS.lock() {
            if let Some(formats) = guard.as_ref() {
                wav.init_module(0, formats);
            }
        }
        
        wav.init();
        
        (*rust_dec).rust_decoder = Box::into_raw(wav) as *mut c_void;
        (*decoder).need_swap = false;
    }
    
    1 // success
}

extern "C" fn rust_wav_Term(decoder: *mut TFB_SoundDecoder) {
    rust_bridge_log_msg("RUST_WAV_TERM");
    
    if decoder.is_null() {
        return;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustWavDecoder;
        if !(*rust_dec).rust_decoder.is_null() {
            let wav = Box::from_raw((*rust_dec).rust_decoder as *mut WavDecoder);
            drop(wav);
            (*rust_dec).rust_decoder = ptr::null_mut();
        }
    }
}

extern "C" fn rust_wav_Open(
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
        
        rust_bridge_log_msg(&format!("RUST_WAV_OPEN: {} (dir={:?})", filename_str, dir));
        
        // Open file via UIO
        let rust_dec = decoder as *mut TFB_RustWavDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return 0;
        }
        
        let wav = &mut *((*rust_dec).rust_decoder as *mut WavDecoder);
        
        // Use UIO to read the file
        let handle = uio_open(dir, filename, 0, 0); // O_RDONLY = 0
        if handle.is_null() {
            rust_bridge_log_msg(&format!("RUST_WAV_OPEN_FAILED: uio_open returned null for {}", filename_str));
            return 0;
        }
        
        // Get file size
        let mut stat_buf: libc::stat = std::mem::zeroed();
        if uio_fstat(handle, &mut stat_buf) != 0 {
            uio_close(handle);
            rust_bridge_log_msg("RUST_WAV_OPEN_FAILED: uio_fstat failed");
            return 0;
        }
        let file_size = stat_buf.st_size as usize;
        rust_bridge_log_msg(&format!("RUST_WAV_OPEN: file size = {} bytes", file_size));
        
        // Read entire file into memory
        let mut data = vec![0u8; file_size];
        let mut total_read = 0usize;
        while total_read < file_size {
            let bytes_read = uio_read(handle, data.as_mut_ptr().add(total_read), file_size - total_read);
            if bytes_read <= 0 {
                break;
            }
            total_read += bytes_read as usize;
        }
        uio_close(handle);
        
        if total_read == 0 {
            rust_bridge_log_msg(&format!("RUST_WAV_OPEN_FAILED: could not read any data from {}", filename_str));
            return 0;
        }
        rust_bridge_log_msg(&format!("RUST_WAV_OPEN: read {} bytes from UIO", total_read));
        
        // Parse WAV data
        match wav.open_from_bytes(&data[..total_read], filename_str) {
            Ok(()) => {
                // Update base decoder fields
                (*decoder).frequency = wav.frequency();
                
                // Get format from stored decoder_formats (set during InitModule)
                // These are the audio_FORMAT_* enum values passed from audiodrv_sdl.c
                // IMPORTANT: The C layer expects an audio_FORMAT_* enum value here,
                // which gets translated via audiodrv.EnumLookup[] to MIX_FORMAT_* values
                let (format_val, format_str) = if let Ok(guard) = RUST_WAV_FORMATS.lock() {
                    if let Some(ref formats) = *guard {
                        match wav.format() {
                            super::formats::AudioFormat::Mono8 => (formats.mono8, "mono8 from formats"),
                            super::formats::AudioFormat::Stereo8 => (formats.stereo8, "stereo8 from formats"),
                            super::formats::AudioFormat::Mono16 => (formats.mono16, "mono16 from formats"),
                            super::formats::AudioFormat::Stereo16 => (formats.stereo16, "stereo16 from formats"),
                        }
                    } else {
                        // No formats stored - use audio_FORMAT_* enum values directly
                        // audio_FORMAT_MONO8 = 24, audio_FORMAT_STEREO8 = 25
                        // audio_FORMAT_MONO16 = 22, audio_FORMAT_STEREO16 = 23
                        rust_bridge_log_msg("RUST_WAV_OPEN: WARNING - no decoder formats stored!");
                        match wav.format() {
                            super::formats::AudioFormat::Mono8 => (24, "fallback mono8"),
                            super::formats::AudioFormat::Stereo8 => (25, "fallback stereo8"),
                            super::formats::AudioFormat::Mono16 => (22, "fallback mono16"),
                            super::formats::AudioFormat::Stereo16 => (23, "fallback stereo16"),
                        }
                    }
                } else {
                    // Lock failed - use audio_FORMAT_* enum values directly
                    rust_bridge_log_msg("RUST_WAV_OPEN: WARNING - failed to lock formats!");
                    match wav.format() {
                        super::formats::AudioFormat::Mono8 => (24, "fallback mono8"),
                        super::formats::AudioFormat::Stereo8 => (25, "fallback stereo8"),
                        super::formats::AudioFormat::Mono16 => (22, "fallback mono16"),
                        super::formats::AudioFormat::Stereo16 => (23, "fallback stereo16"),
                    }
                };
                
                rust_bridge_log_msg(&format!(
                    "RUST_WAV_OPEN: setting format={} ({})",
                    format_val, format_str
                ));
                
                (*decoder).format = format_val;
                (*decoder).length = wav.length();
                (*decoder).is_null = false;
                (*decoder).need_swap = wav.needs_swap();
                
                rust_bridge_log_msg(&format!(
                    "RUST_WAV_OPEN_SUCCESS: freq={} format={} length={}",
                    (*decoder).frequency, (*decoder).format, (*decoder).length
                ));
                1
            }
            Err(e) => {
                rust_bridge_log_msg(&format!("RUST_WAV_OPEN_FAILED: {}", e));
                0
            }
        }
    }
}

extern "C" fn rust_wav_Close(decoder: *mut TFB_SoundDecoder) {
    rust_bridge_log_msg("RUST_WAV_CLOSE");
    
    if decoder.is_null() {
        return;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustWavDecoder;
        if !(*rust_dec).rust_decoder.is_null() {
            let wav = &mut *((*rust_dec).rust_decoder as *mut WavDecoder);
            wav.close();
        }
    }
}

extern "C" fn rust_wav_Decode(
    decoder: *mut TFB_SoundDecoder,
    buf: *mut c_void,
    bufsize: i32,
) -> c_int {
    if decoder.is_null() || buf.is_null() || bufsize <= 0 {
        return -1;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustWavDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return -1;
        }
        
        let wav = &mut *((*rust_dec).rust_decoder as *mut WavDecoder);
        let buffer = std::slice::from_raw_parts_mut(buf as *mut u8, bufsize as usize);
        
        match wav.decode(buffer) {
            Ok(bytes) => bytes as c_int,
            Err(super::decoder::DecodeError::EndOfFile) => 0,
            Err(_) => -1,
        }
    }
}

extern "C" fn rust_wav_Seek(decoder: *mut TFB_SoundDecoder, pcm_pos: u32) -> u32 {
    if decoder.is_null() {
        return 0;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustWavDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return 0;
        }
        
        let wav = &mut *((*rust_dec).rust_decoder as *mut WavDecoder);
        match wav.seek(pcm_pos) {
            Ok(pos) => pos,
            Err(_) => pcm_pos,
        }
    }
}

extern "C" fn rust_wav_GetFrame(decoder: *mut TFB_SoundDecoder) -> u32 {
    if decoder.is_null() {
        return 0;
    }
    
    unsafe {
        let rust_dec = decoder as *mut TFB_RustWavDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return 0;
        }
        
        let wav = &*((*rust_dec).rust_decoder as *mut WavDecoder);
        wav.get_frame()
    }
}

// =============================================================================
// Exported vtable
// =============================================================================

/// Rust WAV decoder vtable - exported for C linkage
#[no_mangle]
pub static rust_wav_DecoderVtbl: TFB_SoundDecoderFuncs = TFB_SoundDecoderFuncs {
    GetName: rust_wav_GetName,
    InitModule: rust_wav_InitModule,
    TermModule: rust_wav_TermModule,
    GetStructSize: rust_wav_GetStructSize,
    GetError: rust_wav_GetError,
    Init: rust_wav_Init,
    Term: rust_wav_Term,
    Open: rust_wav_Open,
    Close: rust_wav_Close,
    Decode: rust_wav_Decode,
    Seek: rust_wav_Seek,
    GetFrame: rust_wav_GetFrame,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wav_vtable_exists() {
        let name_ptr = (rust_wav_DecoderVtbl.GetName)();
        assert!(!name_ptr.is_null());
        
        let name = unsafe { CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Rust Wave");
    }

    #[test]
    fn test_wav_struct_sizes() {
        let size = rust_wav_GetStructSize();
        assert!(size > 0);
        assert!(size >= std::mem::size_of::<TFB_SoundDecoder>() as u32);
    }

    #[test]
    fn test_wav_null_decoder_handling() {
        assert_eq!(rust_wav_GetError(ptr::null_mut()), -1);
        rust_wav_Term(ptr::null_mut()); // Should not crash
        rust_wav_Close(ptr::null_mut()); // Should not crash
        assert_eq!(rust_wav_Decode(ptr::null_mut(), ptr::null_mut(), 0), -1);
        assert_eq!(rust_wav_Seek(ptr::null_mut(), 0), 0);
        assert_eq!(rust_wav_GetFrame(ptr::null_mut()), 0);
    }
}
