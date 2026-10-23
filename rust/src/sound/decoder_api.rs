//! Rust implementation of the legacy C sound-decoder registry and API.
#![allow(
    clippy::missing_safety_doc,
    reason = "The exported pointer contracts mirror the existing decoder.h C API"
)]
use crate::memory::{rust_hcalloc, rust_hfree, rust_hmalloc, rust_hrealloc};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use std::sync::{Mutex, OnceLock};

use super::ffi::{uio_DirHandle, TFB_DecoderFormats, TFB_SoundDecoder, TFB_SoundDecoderFuncs};

const MAX_REG_DECODERS: usize = 31;
const OK: i32 = 0;
const ERROR: i32 = 1;
const EOF: i32 = 2;

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
    fn fileExists2(dir: *mut uio_DirHandle, filename: *const c_char) -> bool;
    static rust_wav_DecoderVtbl: TFB_SoundDecoderFuncs;
    static rust_mod_DecoderVtbl: TFB_SoundDecoderFuncs;
    static rust_ova_DecoderVtbl: TFB_SoundDecoderFuncs;
    static rust_duka_DecoderVtbl: TFB_SoundDecoderFuncs;
    static rust_aifa_DecoderVtbl: TFB_SoundDecoderFuncs;
}

#[repr(C)]
struct BufferDecoder {
    base: TFB_SoundDecoder,
    data: *mut c_void,
    max_pcm: u32,
    cur_pcm: u32,
}
#[repr(C)]
struct NullDecoder {
    base: TFB_SoundDecoder,
    cur_pcm: u32,
}
static BUFFER_NAME: &[u8] = b"Buffer\0";
static NULL_NAME: &[u8] = b"Null\0";
static NULL_FUNCS: TFB_SoundDecoderFuncs = TFB_SoundDecoderFuncs {
    GetName: null_name,
    InitModule: never_init,
    TermModule: empty,
    GetStructSize: null_size,
    GetError: zero_error,
    Init: null_init,
    Term: null_term,
    Open: null_open,
    Close: null_close,
    Decode: null_decode,
    Seek: null_seek,
    GetFrame: zero_frame,
};
static BUFFER_FUNCS: TFB_SoundDecoderFuncs = TFB_SoundDecoderFuncs {
    GetName: buffer_name,
    InitModule: never_init,
    TermModule: empty,
    GetStructSize: buffer_size,
    GetError: zero_error,
    Init: buffer_init,
    Term: buffer_term,
    Open: null_open,
    Close: buffer_close,
    Decode: buffer_decode,
    Seek: buffer_seek,
    GetFrame: zero_frame,
};
extern "C" fn null_name() -> *const c_char {
    NULL_NAME.as_ptr().cast()
}
extern "C" fn buffer_name() -> *const c_char {
    BUFFER_NAME.as_ptr().cast()
}
extern "C" fn never_init(_: c_int, _: *const TFB_DecoderFormats) -> c_int {
    0
}
extern "C" fn empty() {}
extern "C" fn null_size() -> u32 {
    std::mem::size_of::<NullDecoder>() as u32
}
extern "C" fn buffer_size() -> u32 {
    std::mem::size_of::<BufferDecoder>() as u32
}
extern "C" fn zero_error(_: *mut TFB_SoundDecoder) -> c_int {
    0
}
extern "C" fn zero_frame(_: *mut TFB_SoundDecoder) -> u32 {
    0
}
extern "C" fn null_init(d: *mut TFB_SoundDecoder) -> c_int {
    if d.is_null() {
        return 0;
    }
    unsafe {
        (*(d.cast::<NullDecoder>())).cur_pcm = 0;
        (*d).need_swap = false;
    }
    1
}
extern "C" fn null_term(d: *mut TFB_SoundDecoder) {
    null_close(d)
}
extern "C" fn null_open(
    d: *mut TFB_SoundDecoder,
    _: *mut uio_DirHandle,
    _: *const c_char,
) -> c_int {
    if d.is_null() {
        return 0;
    }
    unsafe {
        (*d).frequency = 11025;
        (*d).format = registry().lock().unwrap().formats.mono16;
        (*d).is_null = true;
    }
    1
}
extern "C" fn null_close(d: *mut TFB_SoundDecoder) {
    if !d.is_null() {
        unsafe {
            (*(d.cast::<NullDecoder>())).cur_pcm = 0;
        }
    }
}
extern "C" fn null_decode(d: *mut TFB_SoundDecoder, out: *mut c_void, n: c_int) -> c_int {
    if d.is_null() || out.is_null() || n <= 0 {
        return 0;
    }
    unsafe {
        let b = &mut *d.cast::<NullDecoder>();
        let bytes_per = b.base.bytes_per_samp;
        if bytes_per == 0 {
            return 0;
        }
        let max = (b.base.length * b.base.frequency as f32) as u32;
        let pcm = ((n as u32) / bytes_per).min(max.saturating_sub(b.cur_pcm));
        let bytes = pcm * bytes_per;
        ptr::write_bytes(out.cast::<u8>(), 0, bytes as usize);
        b.cur_pcm += pcm;
        bytes as c_int
    }
}
extern "C" fn null_seek(d: *mut TFB_SoundDecoder, pos: u32) -> u32 {
    if d.is_null() {
        return 0;
    }
    unsafe {
        let b = &mut *d.cast::<NullDecoder>();
        let max = (b.base.length * b.base.frequency as f32) as u32;
        b.cur_pcm = pos.min(max);
        b.cur_pcm
    }
}
extern "C" fn buffer_init(d: *mut TFB_SoundDecoder) -> c_int {
    if d.is_null() {
        return 0;
    }
    unsafe {
        let b = &mut *d.cast::<BufferDecoder>();
        b.base.need_swap = false;
        b.data = b.base.buffer;
        b.max_pcm = b
            .base
            .buffer_size
            .checked_div(b.base.bytes_per_samp)
            .unwrap_or(0);
        b.cur_pcm = b.max_pcm;
    }
    1
}
extern "C" fn buffer_term(d: *mut TFB_SoundDecoder) {
    buffer_close(d)
}
extern "C" fn buffer_close(d: *mut TFB_SoundDecoder) {
    if !d.is_null() {
        unsafe {
            let b = &mut *d.cast::<BufferDecoder>();
            if !b.data.is_null() {
                b.base.buffer = b.data;
                b.data = ptr::null_mut()
            }
            b.cur_pcm = 0;
        }
    }
}
extern "C" fn buffer_decode(d: *mut TFB_SoundDecoder, _: *mut c_void, n: c_int) -> c_int {
    if d.is_null() || n <= 0 {
        return 0;
    }
    unsafe {
        let b = &mut *d.cast::<BufferDecoder>();
        let size = b.base.bytes_per_samp;
        if size == 0 {
            return 0;
        }
        let pcm = ((n as u32) / size).min(b.max_pcm.saturating_sub(b.cur_pcm));
        let bytes = pcm * size;
        b.base.buffer = b.data.cast::<u8>().add((b.cur_pcm * size) as usize).cast();
        b.cur_pcm += pcm;
        bytes as c_int
    }
}
extern "C" fn buffer_seek(d: *mut TFB_SoundDecoder, pos: u32) -> u32 {
    if d.is_null() {
        return 0;
    }
    unsafe {
        let b = &mut *d.cast::<BufferDecoder>();
        b.cur_pcm = pos.min(b.max_pcm);
        b.cur_pcm
    }
}

fn cstr_eq(a: *const c_char, b: *const c_char) -> bool {
    if a.is_null() || b.is_null() {
        return false;
    }
    unsafe { CStr::from_ptr(a) == CStr::from_ptr(b) }
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
pub unsafe extern "C" fn SoundDecoder_GetName(d: *mut TFB_SoundDecoder) -> *const c_char {
    if d.is_null() || (*d).funcs.is_null() {
        return c"(Null)".as_ptr();
    }
    ((*(*d).funcs).GetName)()
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
pub unsafe extern "C" fn SoundDecoder_Register(
    ext: *const c_char,
    funcs: *mut TFB_SoundDecoderFuncs,
) -> *mut TFB_RegSoundDecoder {
    if ext.is_null() || funcs.is_null() {
        return ptr::null_mut();
    }
    let mut r = registry().lock().unwrap();
    init_builtins(&mut r);
    if r.entries
        .iter()
        .any(|e| e.used && !e.ext.is_null() && cstr_eq(e.ext, ext))
    {
        return ptr::null_mut();
    }
    if ((*funcs).InitModule)(r.flags, &r.formats) == 0 {
        return ptr::null_mut();
    }
    let slot = r
        .entries
        .iter_mut()
        .find(|e| !e.builtin && (!e.used || e.ext.is_null()));
    if let Some(e) = slot {
        e.used = true;
        e.ext = ext;
        e.funcs = funcs;
        return &mut **e;
    }
    if r.entries.len() >= MAX_REG_DECODERS {
        return ptr::null_mut();
    }
    r.entries.push(Box::new(TFB_RegSoundDecoder {
        builtin: false,
        used: true,
        ext,
        funcs,
    }));
    &mut **r.entries.last_mut().unwrap()
}
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_Unregister(reg: *mut TFB_RegSoundDecoder) {
    if reg.is_null() {
        return;
    }
    let mut r = registry().lock().unwrap();
    if let Some(index) = r.entries.iter().position(|e| std::ptr::eq(&**e, reg)) {
        let e = &mut r.entries[index];
        if e.builtin || e.ext.is_null() || e.funcs.is_null() {
            return;
        }
        ((*e.funcs).TermModule)();
        e.ext = ptr::null();
        e.funcs = ptr::null();
    }
}
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_Lookup(ext: *const c_char) -> *const TFB_SoundDecoderFuncs {
    if ext.is_null() {
        return ptr::null();
    }
    let mut r = registry().lock().unwrap();
    init_builtins(&mut r);
    r.entries
        .iter()
        .find(|e| e.used && cstr_eq(e.ext, ext))
        .map_or(ptr::null(), |e| e.funcs)
}
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_Load(
    dir: *mut uio_DirHandle,
    filename: *mut c_char,
    buffer_size: u32,
    start_time: u32,
    run_time: i32,
) -> *mut TFB_SoundDecoder {
    if filename.is_null() {
        return ptr::null_mut();
    }
    let path = CStr::from_ptr(filename);
    let bytes = path.to_bytes();
    let ext = bytes
        .iter()
        .rposition(|b| *b == b'.')
        .map(|i| &bytes[i + 1..]);
    let mut r = registry().lock().unwrap();
    init_builtins(&mut r);
    let mut funcs = ext.and_then(|x| {
        r.entries
            .iter()
            .find(|e| e.used && CStr::from_ptr(e.ext).to_bytes() == x)
            .map(|e| e.funcs)
    });
    let mut rt = run_time;
    let mut start = start_time;
    if funcs.is_none() || !fileExists2(dir, filename) {
        if run_time == 0 {
            return ptr::null_mut();
        }
        rt = run_time.saturating_abs();
        start = 0;
        funcs = Some(&NULL_FUNCS)
    }
    let funcs = funcs.unwrap();
    let formats = r.formats;
    drop(r);
    let size =
        ((*funcs).GetStructSize)().max(std::mem::size_of::<TFB_SoundDecoder>() as u32) as usize;
    let d = rust_hcalloc(size).cast::<TFB_SoundDecoder>();
    if d.is_null() {
        return ptr::null_mut();
    }
    (*d).funcs = funcs;
    if ((*funcs).Init)(d) == 0 {
        rust_hfree(d.cast());
        return ptr::null_mut();
    }
    if ((*funcs).Open)(d, dir, filename) == 0 {
        ((*funcs).Term)(d);
        rust_hfree(d.cast());
        return ptr::null_mut();
    }
    (*d).buffer = rust_hmalloc(buffer_size as usize);
    (*d).buffer_size = buffer_size;
    (*d).looping = false;
    (*d).error = OK;
    (*d).dir = dir;
    let dup = CString::new(path.to_bytes()).unwrap();
    (*d).filename = rust_hmalloc(dup.as_bytes_with_nul().len()).cast();
    ptr::copy_nonoverlapping(dup.as_ptr(), (*d).filename, dup.as_bytes_with_nul().len());
    if (*d).is_null {
        (*d).length = rt as f32 / 1000.0
    }
    (*d).length = ((*d).length - start as f32 / 1000.0).max(0.0);
    if rt > 0 && rt as f32 / 1000.0 < (*d).length {
        (*d).length = rt as f32 / 1000.0
    }
    (*d).start_sample = (start as f64 / 1000.0 * (*d).frequency as f64) as u32;
    (*d).end_sample = (*d).start_sample + ((*d).length * (*d).frequency as f32) as u32;
    if (*d).start_sample > 0 {
        ((*funcs).Seek)(d, (*d).start_sample);
    }
    (*d).bytes_per_samp = match (*d).format {
        f if f == formats.mono8 => 1,
        f if f == formats.mono16 => 2,
        f if f == formats.stereo8 => 2,
        f if f == formats.stereo16 => 4,
        _ => 0,
    };
    (*d).pos = (*d).start_sample * (*d).bytes_per_samp;
    d
}
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_Decode(d: *mut TFB_SoundDecoder) -> u32 {
    if d.is_null() || (*d).funcs.is_null() || (*d).buffer.is_null() || (*d).bytes_per_samp == 0 {
        return 0;
    }
    let mut capacity = (*d).buffer_size as usize;
    let max = if !(*d).looping && (*d).end_sample > 0 {
        (*d).end_sample.saturating_mul((*d).bytes_per_samp)
    } else {
        u32::MAX
    };
    if max != u32::MAX {
        capacity = capacity.min(max.saturating_sub((*d).pos) as usize)
    }
    if capacity == 0 {
        (*d).error = EOF;
        return 0;
    }
    let mut total = 0usize;
    let mut rc = 1;
    while rc > 0 && total < capacity {
        rc = ((*(*d).funcs).Decode)(
            d,
            (*d).buffer.cast::<u8>().add(total).cast(),
            (capacity - total).min(i32::MAX as usize) as i32,
        );
        if rc > 0 {
            total += rc as usize
        } else if rc == 0 && (*d).looping {
            SoundDecoder_Rewind(d);
            if (*d).error != OK {
                break;
            }
            rc = 1
        }
    }
    (*d).pos = (*d).pos.saturating_add(total as u32);
    (*d).error = if rc < 0 {
        ERROR
    } else if rc == 0 || (*d).pos >= max {
        EOF
    } else {
        OK
    };
    if (*d).need_swap
        && total > 0
        && ((*d).format == registry().lock().unwrap().formats.mono16
            || (*d).format == registry().lock().unwrap().formats.stereo16)
    {
        SoundDecoder_SwapWords((*d).buffer.cast(), total as u32)
    }
    total as u32
}
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_DecodeAll(d: *mut TFB_SoundDecoder) -> u32 {
    if d.is_null() || (*d).funcs.is_null() || (*d).looping {
        return 0;
    }
    let mut total = 0usize;
    let step = ((*d).buffer_size as usize).max(4096);
    loop {
        if total >= (*d).buffer_size as usize {
            let new_size = (*d).buffer_size as usize + step;
            let p = rust_hrealloc((*d).buffer, new_size);
            if p.is_null() {
                break;
            }
            (*d).buffer = p;
            (*d).buffer_size = new_size as u32;
        }
        let n = ((*(*d).funcs).Decode)(
            d,
            (*d).buffer.cast::<u8>().add(total).cast(),
            ((*d).buffer_size as usize - total).min(i32::MAX as usize) as i32,
        );
        if n <= 0 {
            if n < 0 {
                (*d).error = ERROR;
                break;
            } else {
                break;
            }
        }
        total += n as usize;
    }
    (*d).buffer_size = total as u32;
    (*d).pos = (*d).pos.saturating_add(total as u32);
    (*d).buffer = rust_hrealloc((*d).buffer, total);
    if (*d).need_swap {
        SoundDecoder_SwapWords((*d).buffer.cast(), total as u32)
    }
    if (*d).error == ERROR {
        return total as u32;
    }
    ((*(*d).funcs).Close)(d);
    ((*(*d).funcs).Term)(d);
    (*d).funcs = &BUFFER_FUNCS;
    (BUFFER_FUNCS.Init)(d);
    (*d).pos = 0;
    (*d).start_sample = 0;
    (*d).error = OK;
    total as u32
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
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_Free(d: *mut TFB_SoundDecoder) {
    if d.is_null() || (*d).funcs.is_null() {
        return;
    }
    ((*(*d).funcs).Close)(d);
    ((*(*d).funcs).Term)(d);
    rust_hfree((*d).buffer);
    rust_hfree((*d).filename.cast());
    rust_hfree(d.cast());
}
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_GetTime(d: *mut TFB_SoundDecoder) -> f32 {
    if d.is_null() || (*d).bytes_per_samp == 0 || (*d).frequency == 0 {
        return 0.0;
    }
    (((*d).pos / (*d).bytes_per_samp).saturating_sub((*d).start_sample) as f32)
        / (*d).frequency as f32
}
#[no_mangle]
pub unsafe extern "C" fn SoundDecoder_GetFrame(d: *mut TFB_SoundDecoder) -> u32 {
    if d.is_null() || (*d).funcs.is_null() {
        0
    } else {
        ((*(*d).funcs).GetFrame)(d)
    }
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
