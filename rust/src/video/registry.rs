//! Decoder registry and dispatch, ported from videodec.c.
#![allow(non_snake_case)]

use super::ffi::{
    TFB_PixelFormat, TFB_VideoDecoder, TFB_VideoDecoderFuncs, RUST_DUKV_DECODER_VTBL,
};
use crate::io::ffi::uio_DirHandle;
use crate::memory::{rust_hcalloc, rust_hfree, rust_hmalloc};
use std::ffi::{c_char, c_int, CStr};
use std::ptr;

const MAX_DECODERS: usize = 31;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RegisteredDecoder {
    builtin: bool,
    used: bool,
    extension: *const c_char,
    functions: *const TFB_VideoDecoderFuncs,
}
const EMPTY: RegisteredDecoder = RegisteredDecoder {
    builtin: false,
    used: false,
    extension: ptr::null(),
    functions: ptr::null(),
};
// As with the original library, registration and lifecycle belong to the game thread.
static mut DECODERS: [RegisteredDecoder; MAX_DECODERS + 1] = {
    let mut entries = [EMPTY; MAX_DECODERS + 1];
    entries[0] = RegisteredDecoder {
        builtin: true,
        used: true,
        extension: c"duk".as_ptr(),
        functions: &RUST_DUKV_DECODER_VTBL,
    };
    entries
};
static mut INITIALIZED: bool = false;
static mut FLAGS: c_int = 0;
static mut FORMAT: TFB_PixelFormat = unsafe { std::mem::zeroed() };

fn masks(mut mask: u32) -> (u32, u32) {
    let mut shift = 0;
    let mut loss = 8u32;
    if mask != 0 {
        while mask & 1 == 0 {
            shift += 1;
            mask >>= 1;
        }
        while mask & 1 != 0 {
            loss = loss.wrapping_sub(1);
            mask >>= 1;
        }
    }
    (shift, loss)
}

fn pixel_format(depth: c_int, r: u32, g: u32, b: u32, a: u32) -> Option<TFB_PixelFormat> {
    if !(15..=32).contains(&depth)
        || r & g != 0
        || r & b != 0
        || r & a != 0
        || g & b != 0
        || g & a != 0
        || b & a != 0
    {
        return None;
    }
    let (Rshift, Rloss) = masks(r);
    let (Gshift, Gloss) = masks(g);
    let (Bshift, Bloss) = masks(b);
    let (Ashift, Aloss) = masks(a);
    Some(TFB_PixelFormat {
        BitsPerPixel: depth as u32,
        BytesPerPixel: (depth as u32).div_ceil(8),
        Rmask: r,
        Gmask: g,
        Bmask: b,
        Amask: a,
        Rshift,
        Gshift,
        Bshift,
        Ashift,
        Rloss,
        Gloss,
        Bloss,
        Aloss,
    })
}

/// # Safety
/// Must be serialized with all decoder registration and lifetime operations.
pub unsafe extern "C" fn VideoDecoder_Init(
    flags: c_int,
    depth: c_int,
    r: u32,
    g: u32,
    b: u32,
    a: u32,
) -> bool {
    INITIALIZED = false;
    let Some(format) = pixel_format(depth, r, g, b, a) else {
        return false;
    };
    FORMAT = format;
    let mut entry = ptr::addr_of_mut!(DECODERS).cast::<RegisteredDecoder>();
    while !(*entry).extension.is_null() {
        ((*(*entry).functions).InitModule)(flags);
        entry = entry.add(1);
    }
    FLAGS = flags;
    INITIALIZED = true;
    true
}

/// # Safety
/// No decoders may remain live, and registry access must be serialized.
pub unsafe extern "C" fn VideoDecoder_Uninit() {
    let mut entry = ptr::addr_of_mut!(DECODERS).cast::<RegisteredDecoder>();
    while (*entry).used {
        if !(*entry).extension.is_null() {
            ((*(*entry).functions).TermModule)();
        }
        if !(*entry).builtin {
            (*entry).used = false;
            (*entry).extension = ptr::null();
        }
        entry = entry.add(1);
    }
    INITIALIZED = false;
}

/// # Safety
/// Extension and vtable must remain valid until unregistration; calls are serialized.
pub unsafe extern "C" fn VideoDecoder_Register(
    extension: *const c_char,
    functions: *mut TFB_VideoDecoderFuncs,
) -> *mut RegisteredDecoder {
    if functions.is_null() || extension.is_null() {
        return ptr::null_mut();
    }
    let base = ptr::addr_of_mut!(DECODERS).cast::<RegisteredDecoder>();
    let mut entry = base;
    let mut slot: *mut RegisteredDecoder = ptr::null_mut();
    while (*entry).used
        && ((*entry).extension.is_null() || libc::strcmp((*entry).extension, extension) != 0)
    {
        if slot.is_null() && (*entry).extension.is_null() {
            slot = entry;
        }
        entry = entry.add(1);
    }
    if entry >= base.add(MAX_DECODERS) || !(*entry).extension.is_null() {
        return ptr::null_mut();
    }
    if !((*functions).InitModule)(FLAGS) {
        return ptr::null_mut();
    }
    if slot.is_null() {
        slot = entry;
        (*slot).used = true;
        (*entry.add(1)).builtin = false;
        (*entry.add(1)).used = false;
        (*entry.add(1)).extension = ptr::null();
    }
    (*slot).extension = extension;
    (*slot).functions = functions;
    slot
}

/// # Safety
/// Registry access must be serialized. `entry` is a registration handle.
pub unsafe extern "C" fn VideoDecoder_Unregister(entry: *mut RegisteredDecoder) {
    let base = ptr::addr_of_mut!(DECODERS).cast::<RegisteredDecoder>();
    if entry < base
        || entry >= base.add(MAX_DECODERS)
        || (*entry).extension.is_null()
        || (*entry).functions.is_null()
    {
        return;
    }
    ((*(*entry).functions).TermModule)();
    (*entry).extension = ptr::null();
    (*entry).functions = ptr::null();
}

/// # Safety
/// `extension` is a terminated string, and registry access must be serialized.
pub unsafe extern "C" fn VideoDecoder_Lookup(
    extension: *const c_char,
) -> *const TFB_VideoDecoderFuncs {
    let mut entry = ptr::addr_of!(DECODERS).cast::<RegisteredDecoder>();
    while (*entry).used
        && ((*entry).extension.is_null() || libc::strcmp((*entry).extension, extension) != 0)
    {
        entry = entry.add(1);
    }
    if (*entry).extension.is_null() {
        ptr::null()
    } else {
        (*entry).functions
    }
}

/// # Safety
/// `filename` is terminated, `dir` is live, and decoder lifecycle access is serialized.
pub unsafe extern "C" fn VideoDecoder_Load(
    dir: *mut uio_DirHandle,
    filename: *const c_char,
) -> *mut TFB_VideoDecoder {
    if !INITIALIZED {
        return ptr::null_mut();
    }
    let bytes = CStr::from_ptr(filename).to_bytes();
    let Some(dot) = bytes.iter().rposition(|&b| b == b'.') else {
        return ptr::null_mut();
    };
    let functions = VideoDecoder_Lookup(filename.add(dot + 1));
    if functions.is_null() {
        return ptr::null_mut();
    }
    let decoder = rust_hcalloc(((*functions).GetStructSize)() as usize).cast::<TFB_VideoDecoder>();
    (*decoder).funcs = functions;
    if !((*functions).Init)(decoder, ptr::addr_of_mut!(FORMAT)) {
        rust_hfree(decoder.cast());
        return ptr::null_mut();
    }
    (*decoder).dir = dir;
    (*decoder).filename = rust_hmalloc(bytes.len() + 1).cast();
    ptr::copy_nonoverlapping(filename, (*decoder).filename, bytes.len() + 1);
    (*decoder).error = 0;
    if !((*functions).Open)(decoder, dir, filename) {
        VideoDecoder_Free(decoder);
        return ptr::null_mut();
    }
    decoder
}

/// # Safety
/// `decoder` is null or a live decoder.
pub unsafe extern "C" fn VideoDecoder_GetName(decoder: *mut TFB_VideoDecoder) -> *const c_char {
    if decoder.is_null() || (*decoder).funcs.is_null() {
        c"(Null)".as_ptr()
    } else {
        ((*(*decoder).funcs).GetName)()
    }
}

/// # Safety
/// `decoder` is null or a live decoder owned by this thread.
pub unsafe extern "C" fn VideoDecoder_Decode(decoder: *mut TFB_VideoDecoder) -> c_int {
    if decoder.is_null() {
        return 0;
    }
    let functions = &*(*decoder).funcs;
    (*decoder).cur_frame = (functions.GetFrame)(decoder);
    (*decoder).pos = (functions.GetTime)(decoder);
    let result = (functions.DecodeNext)(decoder);
    (*decoder).error = if result == 0 {
        2
    } else if result < 0 {
        1
    } else {
        0
    };
    result
}

/// # Safety
/// `decoder` is null or a live decoder owned by this thread.
pub unsafe extern "C" fn VideoDecoder_Seek(decoder: *mut TFB_VideoDecoder, position: f32) -> f32 {
    if decoder.is_null() {
        return 0.0;
    }
    (*decoder).pos = ((*(*decoder).funcs).SeekTime)(decoder, position);
    (*decoder).cur_frame = ((*(*decoder).funcs).GetFrame)(decoder);
    (*decoder).pos
}

/// # Safety
/// `decoder` is null or a live decoder owned by this thread.
pub unsafe extern "C" fn VideoDecoder_SeekFrame(decoder: *mut TFB_VideoDecoder, frame: u32) -> u32 {
    if decoder.is_null() {
        return 0;
    }
    (*decoder).cur_frame = ((*(*decoder).funcs).SeekFrame)(decoder, frame);
    (*decoder).pos = ((*(*decoder).funcs).GetTime)(decoder);
    (*decoder).cur_frame
}

/// # Safety
/// `decoder` is null or a live decoder owned by this thread.
pub unsafe extern "C" fn VideoDecoder_Rewind(decoder: *mut TFB_VideoDecoder) {
    if !decoder.is_null() {
        VideoDecoder_Seek(decoder, 0.0);
    }
}

/// # Safety
/// `decoder` is null or an owned decoder, and must not be used after this call.
pub unsafe extern "C" fn VideoDecoder_Free(decoder: *mut TFB_VideoDecoder) {
    if decoder.is_null() {
        return;
    }
    ((*(*decoder).funcs).Close)(decoder);
    ((*(*decoder).funcs).Term)(decoder);
    rust_hfree((*decoder).filename.cast());
    rust_hfree(decoder.cast());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn masks_match_rgb565_and_absent_alpha() {
        assert_eq!(masks(0xf800), (11, 3));
        assert_eq!(masks(0x7e0), (5, 2));
        assert_eq!(masks(0x1f), (0, 3));
        assert_eq!(masks(0), (0, 8));
    }
    #[test]
    fn format_rejects_invalid_depth_and_overlapping_channels() {
        assert!(pixel_format(14, 0, 0, 0, 0).is_none());
        assert!(pixel_format(33, 0, 0, 0, 0).is_none());
        assert!(pixel_format(32, 0xff, 0xff, 0, 0).is_none());
        let format = pixel_format(15, 0x7c00, 0x3e0, 0x1f, 0).unwrap();
        assert_eq!(format.BytesPerPixel, 2);
        assert_eq!(format.Rshift, 10);
        assert_eq!(format.Aloss, 8);
    }
    #[test]
    fn registry_reuses_removed_slots_and_rejects_duplicates() {
        unsafe {
            assert!(VideoDecoder_Init(0, 32, 0xff, 0xff00, 0xff0000, 0xff000000));
            let functions = ptr::addr_of!(RUST_DUKV_DECODER_VTBL).cast_mut();
            let first = VideoDecoder_Register(c"one".as_ptr(), functions);
            assert!(!first.is_null());
            assert!(VideoDecoder_Register(c"one".as_ptr(), functions).is_null());
            assert_eq!(VideoDecoder_Lookup(c"one".as_ptr()), functions);
            VideoDecoder_Unregister(first);
            assert!(VideoDecoder_Lookup(c"one".as_ptr()).is_null());
            let second = VideoDecoder_Register(c"two".as_ptr(), functions);
            assert_eq!(first, second);
            assert!(!VideoDecoder_Lookup(c"duk".as_ptr()).is_null());
            VideoDecoder_Uninit();
            assert!(VideoDecoder_Lookup(c"two".as_ptr()).is_null());
            assert!(!VideoDecoder_Lookup(c"duk".as_ptr()).is_null());
        }
    }
}
