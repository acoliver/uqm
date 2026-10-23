//! Rust implementation of the legacy C sound-decoder registry and API.
#![allow(
    clippy::missing_safety_doc,
    reason = "The exported pointer contracts mirror the existing decoder.h C API"
)]
use std::ffi::{c_char, c_int};
use std::ptr;
use std::sync::{Mutex, OnceLock};

use super::ffi::{TFB_DecoderFormats, TFB_SoundDecoder, TFB_SoundDecoderFuncs};

const OK: i32 = 0;

#[repr(C)]
pub struct TFB_RegSoundDecoder {
    builtin: bool,
    used: bool,
    ext: *const c_char,
    funcs: *const TFB_SoundDecoderFuncs,
}
unsafe impl Send for TFB_RegSoundDecoder {}

struct Registry {
    #[allow(
        clippy::vec_box,
        reason = "C registration handles must retain stable addresses"
    )]
    entries: Vec<Box<TFB_RegSoundDecoder>>,
    formats: TFB_DecoderFormats,
    flags: c_int,
}
unsafe impl Send for Registry {}
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| {
        Mutex::new(Registry {
            entries: Vec::new(),
            formats: TFB_DecoderFormats {
                big_endian: false,
                want_big_endian: false,
                mono8: 0,
                stereo8: 0,
                mono16: 0,
                stereo16: 0,
            },
            flags: 0,
        })
    })
}

extern "C" {
    static rust_wav_DecoderVtbl: TFB_SoundDecoderFuncs;
    static rust_mod_DecoderVtbl: TFB_SoundDecoderFuncs;
    static rust_ova_DecoderVtbl: TFB_SoundDecoderFuncs;
    static rust_duka_DecoderVtbl: TFB_SoundDecoderFuncs;
    static rust_aifa_DecoderVtbl: TFB_SoundDecoderFuncs;
}

fn add_builtin(r: &mut Registry, ext: &'static [u8], funcs: *const TFB_SoundDecoderFuncs) {
    r.entries.push(Box::new(TFB_RegSoundDecoder {
        builtin: true,
        used: true,
        ext: ext.as_ptr().cast(),
        funcs,
    }));
}
fn init_builtins(r: &mut Registry) {
    if !r.entries.is_empty() {
        return;
    }
    unsafe {
        add_builtin(r, b"wav\0", &rust_wav_DecoderVtbl);
        add_builtin(r, b"mod\0", &rust_mod_DecoderVtbl);
        add_builtin(r, b"ogg\0", &rust_ova_DecoderVtbl);
        add_builtin(r, b"duk\0", &rust_duka_DecoderVtbl);
        add_builtin(r, b"aif\0", &rust_aifa_DecoderVtbl);
    }
    r.entries.push(Box::new(TFB_RegSoundDecoder {
        builtin: false,
        used: false,
        ext: ptr::null(),
        funcs: ptr::null(),
    }));
}

#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_SwapWords(data: *mut u16, size: u32) {
    if data.is_null() {
        return;
    }
    let words = size / 2;
    for i in 0..words {
        let p = data.add(i as usize);
        *p = (*p).swap_bytes();
    }
}
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_Init(
    flags: c_int,
    formats: *mut TFB_DecoderFormats,
) -> c_int {
    if formats.is_null() {
        return 1;
    }
    let mut r = registry().lock().unwrap();
    r.formats = unsafe { *formats };
    r.flags = flags;
    init_builtins(&mut r);
    let mut failed = 0;
    for e in r.entries.iter().take_while(|e| e.used) {
        if !e.ext.is_null() && unsafe { ((*e.funcs).InitModule)(flags, &r.formats) == 0 } {
            failed = 1;
        }
    }
    failed
}
#[no_mangle]
pub extern "C" fn SoundDecoder_Uninit() {
    let mut r = registry().lock().unwrap();
    for e in &mut r.entries {
        if e.used && !e.ext.is_null() {
            unsafe { ((*e.funcs).TermModule)() };
        }
        if !e.builtin {
            e.used = false;
            e.ext = ptr::null();
            e.funcs = ptr::null();
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_Seek(d: *mut TFB_SoundDecoder, time: u32) {
    if d.is_null() || (*d).funcs.is_null() {
        return;
    }
    let pcm = (time as f64 / 1000.0 * (*d).frequency as f64) as u32;
    let at = ((*(*d).funcs).Seek)(d, (*d).start_sample.saturating_add(pcm));
    (*d).pos = at.saturating_mul((*d).bytes_per_samp);
    (*d).error = OK;
}
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_Rewind(d: *mut TFB_SoundDecoder) {
    SoundDecoder_Seek(d, 0)
}

#[cfg(test)]
mod tests {
    use super::SoundDecoder_SwapWords;

    #[test]
    fn swaps_only_complete_words() {
        let mut words = [0x1234, 0xabcd, 0x5678];
        unsafe { SoundDecoder_SwapWords(words.as_mut_ptr(), 5) };
        assert_eq!(words, [0x3412, 0xcdab, 0x5678]);
    }
}
