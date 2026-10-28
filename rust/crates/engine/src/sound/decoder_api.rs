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

use super::aiff_ffi::rust_aifa_DecoderVtbl;
use super::dukaud_ffi::rust_duka_DecoderVtbl;
use super::ffi::rust_ova_DecoderVtbl;
use super::mod_ffi::rust_mod_DecoderVtbl;
use super::wav_ffi::rust_wav_DecoderVtbl;

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
    add_builtin(r, b"wav\0", &rust_wav_DecoderVtbl);
    add_builtin(r, b"mod\0", &rust_mod_DecoderVtbl);
    add_builtin(r, b"ogg\0", &rust_ova_DecoderVtbl);
    add_builtin(r, b"duk\0", &rust_duka_DecoderVtbl);
    add_builtin(r, b"aif\0", &rust_aifa_DecoderVtbl);
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
pub fn init_decoders(flags: c_int, formats: &TFB_DecoderFormats) -> c_int {
    let mut r = registry().lock().unwrap();
    r.formats = *formats;
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
pub fn uninit_decoders() {
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
    use super::*;

    #[test]
    fn registry_lifecycle_accepts_shared_formats_with_rust_signatures() {
        let _: fn(c_int, &super::super::formats::DecoderFormats) -> c_int = init_decoders;
        let _: fn() = uninit_decoders;
    }

    #[test]
    #[serial_test::serial]
    fn registry_initializes_all_builtins_with_the_requested_formats() {
        let formats = super::super::decoder_vtable_tests::formats();
        assert_eq!(init_decoders(0, &formats), 0);
        {
            let r = registry().lock().unwrap();
            assert_eq!(r.flags, 0);
            assert_eq!(r.formats.mono8, formats.mono8);
            assert_eq!(r.formats.stereo16, formats.stereo16);
            assert_eq!(r.entries.iter().filter(|e| e.builtin && e.used).count(), 5);
            let names: Vec<_> = r
                .entries
                .iter()
                .filter(|e| e.used)
                .map(|entry| unsafe {
                    std::ffi::CStr::from_ptr(((*entry.funcs).GetName)())
                        .to_str()
                        .unwrap()
                })
                .collect();
            assert_eq!(
                names,
                [
                    "Rust Wave",
                    "Rust MOD",
                    "Rust Ogg Vorbis",
                    "Rust DukAud",
                    "Rust AIFF"
                ]
            );
        }
        uninit_decoders();
    }

    #[test]
    fn swaps_only_complete_words() {
        let mut words = [0x1234, 0xabcd, 0x5678];
        unsafe { SoundDecoder_SwapWords(words.as_mut_ptr(), 5) };
        assert_eq!(words, [0x3412, 0xcdab, 0x5678]);
    }
}
