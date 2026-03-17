// @plan PLAN-20260225-AUDIO-HEART.P18
// @requirement REQ-CROSS-FFI-01..04, REQ-CROSS-GENERAL-03, REQ-CROSS-GENERAL-08
#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    clippy::missing_safety_doc
)]

//! FFI shim layer — C-callable wrappers for the Rust audio heart.
//!
//! Every function is a thin shim: convert C types → Rust types, call
//! the Rust API, convert results → C types. No logic beyond pointer
//! conversion and error translation. All unsafe code is confined here.

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_long, c_uint, c_void, CStr, CString};
use std::path::Path;
use std::ptr::{self, null_mut};
use std::sync::Arc;

use parking_lot::Mutex;

use super::control;
use super::decoder::{LimitedDecoder, SoundDecoder};
use super::fileinst;
use super::formats::AudioFormat;
use super::mixer::buffer as mixer_buffer;
use super::mixer::{mixer_get_format, mixer_get_frequency};
use super::music;
use super::sfx;
use super::stream;
use super::trackplayer;
// SubtitleRef no longer needed — subtitle iteration uses raw chunk pointers
use super::types::*;

// =============================================================================
// C FFI helpers — UIO + string table access
// =============================================================================

#[cfg(not(test))]
extern "C" {
    fn uio_fopen(dir: *mut c_void, path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn uio_fclose(fp: *mut c_void) -> c_int;
    fn uio_fread(buf: *mut c_void, size: usize, count: usize, fp: *mut c_void) -> usize;
    fn uio_fseek(fp: *mut c_void, offset: c_long, whence: c_int) -> c_int;
    fn uio_ftell(fp: *mut c_void) -> c_long;
    static contentDir: *mut c_void;

    fn AllocStringTable(num_entries: c_int, flags: c_int) -> *mut c_void;
    fn FreeStringTable(strtab: *mut c_void);
}

unsafe fn HMalloc(size: usize) -> *mut c_void {
    crate::memory::rust_hmalloc(size)
}

unsafe fn HFree(ptr: *mut c_void) {
    crate::memory::rust_hfree(ptr)
}

#[cfg(test)]
unsafe fn uio_fopen(_dir: *mut c_void, _path: *const c_char, _mode: *const c_char) -> *mut c_void {
    null_mut()
}
#[cfg(test)]
unsafe fn uio_fclose(_fp: *mut c_void) -> c_int {
    0
}
#[cfg(test)]
unsafe fn uio_fread(_buf: *mut c_void, _size: usize, _count: usize, _fp: *mut c_void) -> usize {
    0
}
#[cfg(test)]
unsafe fn uio_fseek(_fp: *mut c_void, _offset: c_long, _whence: c_int) -> c_int {
    0
}
#[cfg(test)]
unsafe fn uio_ftell(_fp: *mut c_void) -> c_long {
    0
}
#[cfg(test)]
static mut contentDir: *mut c_void = 0 as *mut c_void;
#[cfg(test)]
unsafe fn AllocStringTable(_n: c_int, _f: c_int) -> *mut c_void {
    null_mut()
}
#[cfg(test)]
unsafe fn FreeStringTable(_p: *mut c_void) {}
// HMalloc/HFree use crate::memory directly (no test stubs needed)

/// C STRING_TABLE_ENTRY_DESC layout — must match sc2/src/libs/strings/strintrn.h
#[repr(C)]
struct CStringTableEntry {
    data: *mut c_char,
    length: c_int,
    index: c_int,
    parent: *mut c_void,
}

/// C STRING_TABLE_DESC layout — must match sc2/src/libs/strings/strintrn.h
#[repr(C)]
struct CStringTable {
    flags: u16,
    size: c_int,
    strings: *mut CStringTableEntry,
    name_index: *mut c_void,
}

/// Read a file via UIO into a byte vector.
unsafe fn uio_read_file(path: *const c_char) -> Option<Vec<u8>> {
    let mode = b"rb\0".as_ptr() as *const c_char;
    let fp = uio_fopen(contentDir, path, mode);
    if fp.is_null() {
        return None;
    }
    uio_fseek(fp, 0, 2); // SEEK_END
    let length = uio_ftell(fp) as usize;
    uio_fseek(fp, 0, 0); // SEEK_SET
    if length == 0 {
        uio_fclose(fp);
        return None;
    }
    let mut buf = vec![0u8; length];
    let read = uio_fread(buf.as_mut_ptr() as *mut c_void, 1, length, fp);
    uio_fclose(fp);
    buf.truncate(read);
    Some(buf)
}

/// Create a Rust decoder for a file, based on extension.
fn create_decoder_for_extension(ext: &str) -> Option<Box<dyn SoundDecoder>> {
    let mut decoder: Box<dyn SoundDecoder> = match ext {
        "ogg" => Box::new(super::ogg::OggDecoder::new()),
        "wav" => Box::new(super::wav::WavDecoder::new()),
        "mod" => Box::new(super::mod_decoder::ModDecoder::new()),
        "aif" | "aiff" => Box::new(super::aiff::AiffDecoder::new()),
        _ => {
            eprintln!("create_decoder_for_extension: unknown ext '{}'", ext);
            return None;
        }
    };
    decoder.init();
    Some(decoder)
}

/// Get the file extension from a filename.
fn file_extension(name: &str) -> Option<&str> {
    Path::new(name).extension().and_then(|e| e.to_str())
}

/// Create per-page decoders with time-limited ranges, matching C's
/// `SoundDecoder_Load(dir, file, bufsize, dec_offset, time_stamps[page])` pattern.
///
/// Each page gets its own decoder that seeks to the correct offset and
/// limits decoding to its timestamp duration.
unsafe fn create_per_page_decoders(
    track_name: Option<&str>,
    track_text: Option<&str>,
    timestamp: Option<&str>,
) -> Vec<Box<dyn SoundDecoder>> {
    let name = match track_name {
        Some(n) => n,
        None => return Vec::new(),
    };
    let text = match track_text {
        Some(t) => t,
        None => return Vec::new(),
    };

    // Parse pages with same delimiter behavior as C trackplayer (split on CR or LF runs).
    let mut pages: Vec<&str> = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\r' || bytes[i] == b'\n' {
            pages.push(&text[start..i]);
            while i < bytes.len() && (bytes[i] == b'\r' || bytes[i] == b'\n') {
                i += 1;
            }

            start = i;
        } else {
            i += 1;
        }
    }
    if start < bytes.len() {
        pages.push(&text[start..]);
    }

    let num_pages = pages.len();

    let timestamps: Vec<i32> = timestamp
        .map(|ts| {
            ts.split(|c: char| c == ',' || c == '\n' || c == '\r')
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .filter(|&v| v > 0.0)
                .map(|v| v as i32)
                .collect()
        })
        .unwrap_or_default();

    // Build per-page run times with C SpliceTrack semantics:
    // - num_timestamps = parsed + 1 when timestamp list is provided
    // - if num_timestamps > num_pages, set the last timestamp to a large negative
    //   value to represent "play remainder" until more subtitle pages are appended
    // - if no timestamps are provided, default to one timing entry per subtitle page
    let mut page_run_times: Vec<i32> = if timestamps.is_empty() {
        (0..num_pages)
            .map(|page_idx| {
                let char_count = pages[page_idx].chars().count();
                (char_count as f64 * 80.0).max(1000.0) as i32
            })
            .collect()
    } else {
        let mut runs = timestamps;
        runs.push(0);
        if runs.len() > num_pages {
            if let Some(last) = runs.last_mut() {
                *last = -100000;
            }
        } else if runs.len() < num_pages {
            // Preserve available explicit timestamps; fallback by text length.
            while runs.len() < num_pages {
                let page_idx = runs.len();
                let char_count = pages[page_idx].chars().count();
                runs.push((char_count as f64 * 80.0).max(1000.0) as i32);
            }
        }
        runs
    };

    // Always ensure the final provided page timing is negative (play to end).
    if let Some(last) = page_run_times.last_mut() {
        if *last > 0 {
            *last = -*last;
        }
    }

    // Read the audio file once
    let ext = match file_extension(name) {
        Some(e) => e,
        None => return Vec::new(),
    };
    let c_name = match CString::new(name) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let file_data = match uio_read_file(c_name.as_ptr()) {
        Some(d) => d,
        None => return Vec::new(),
    };

    let mut decoders: Vec<Box<dyn SoundDecoder>> = Vec::with_capacity(page_run_times.len());
    let mut dec_offset_ms: u32 = 0;

    for page_idx in 0..page_run_times.len() {
        let run_time_ms = page_run_times[page_idx];

        // Create a fresh decoder for this page
        let mut dec = match create_decoder_for_extension(ext) {
            Some(d) => d,
            None => break,
        };
        match dec.open_from_bytes(&file_data, name) {
            Ok(()) => {}
            Err(e) => {
                eprintln!(
                    "[SpliceTrack] decoder open failed for {} page {}: {:?}",
                    name, page_idx, e
                );
                break;
            }
        }

        // Preserve sign like C SoundDecoder_Load(..., runTime):
        // negative run_time means "do not clamp" (play to end from offset).
        let limited = LimitedDecoder::new(dec, dec_offset_ms, run_time_ms);

        eprintln!(
            "[SpliceTrack] page {}/{}: offset={}ms run={}ms dec_len={:.2}s",
            page_idx + 1,
            page_run_times.len(),
            dec_offset_ms,
            run_time_ms,
            limited.length()
        );

        // Advance offset by actual decoder length (ms)
        dec_offset_ms += (limited.length() * 1000.0) as u32;

        decoders.push(Box::new(limited));
    }

    eprintln!(
        "[SpliceTrack] created {} per-page decoders for {} (total offset={}ms)",
        decoders.len(),
        name,
        dec_offset_ms
    );
    decoders
}

// =============================================================================
// Thread-local CString caches (FIX: ISSUE-FFI-01)
// =============================================================================

thread_local! {
    static SUBTITLE_CACHE: RefCell<CString> = RefCell::new(CString::default());
    static SUBTITLE_TEXT_CACHE: RefCell<CString> = RefCell::new(CString::default());
}

// =============================================================================
// Helpers
// =============================================================================

unsafe fn c_str_to_option(ptr: *const c_char) -> Option<&'static str> {
    if ptr.is_null() {
        None
    } else {
        CStr::from_ptr(ptr).to_str().ok()
    }
}

/// Convert a UNICODE* (which is really char* / C string) to Option<String>.
/// UQM's UNICODE is typedef'd to `char`, not u16.
unsafe fn unicode_ptr_to_option(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let cstr = std::ffi::CStr::from_ptr(ptr);
    let s = cstr.to_string_lossy().into_owned();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn cache_and_return_c_str_subtitle(text: &str) -> *const c_char {
    SUBTITLE_CACHE.with(|cache| {
        let cs = CString::new(text).unwrap_or_default();
        *cache.borrow_mut() = cs;
        cache.borrow().as_ptr()
    })
}

fn cache_and_return_c_str_text(text: &str) -> *const c_char {
    SUBTITLE_TEXT_CACHE.with(|cache| {
        let cs = CString::new(text).unwrap_or_default();
        *cache.borrow_mut() = cs;
        cache.borrow().as_ptr()
    })
}

/// MUSIC_REF is a double pointer: `Box<Arc<Mutex<SoundSample>>>` stored as
/// `*mut *mut c_void`. The outer Box is the "MUSIC_REF" handle.
/// Borrow the Arc from a MUSIC_REF (double-pointer) WITHOUT taking ownership.
unsafe fn music_ref_borrow(music_ref_ptr: *mut c_void) -> MusicRef {
    let arc_ptr = *(music_ref_ptr as *const *const Mutex<SoundSample>);
    Arc::increment_strong_count(arc_ptr);
    MusicRef(Arc::from_raw(arc_ptr))
}

/// Take ownership of the Arc from a MUSIC_REF (for destroy).
unsafe fn music_ref_take(music_ref_ptr: *mut c_void) -> MusicRef {
    let arc_ptr = *(music_ref_ptr as *const *const Mutex<SoundSample>);
    MusicRef(Arc::from_raw(arc_ptr))
}

/// Check if a MUSIC_REF is the `(MUSIC_REF)~0` sentinel meaning "current music".
fn is_music_ref_sentinel(ptr: *mut c_void) -> bool {
    ptr as usize == usize::MAX
}

/// Borrow an Arc from a raw pointer without consuming ownership.
/// Used by TFB_* functions that receive Arc::into_raw pointers directly.
unsafe fn arc_borrow<T>(ptr: *const T) -> Arc<T> {
    Arc::increment_strong_count(ptr);
    Arc::from_raw(ptr)
}

fn convert_c_callbacks(ptr: *mut c_void) -> Option<Box<dyn StreamCallbacks + Send>> {
    if ptr.is_null() {
        return None;
    }
    let callbacks = unsafe { *(ptr as *const CTfbSoundCallbacks) };
    Some(Box::new(CCallbackWrapper {
        callbacks,
        sample_ptr: ptr,
    }))
}

// (SUBTITLE_REF_CACHE removed — subtitle iteration now uses raw chunk pointers
// matching C's SUBTITLE_REF = TFB_SoundChunk* semantics)

// =============================================================================
// Stream FFI (18 functions)
// =============================================================================

#[no_mangle]
pub extern "C" fn InitStreamDecoder() -> c_int {
    match stream::init_stream_decoder() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("InitStreamDecoder: {}", e);
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn UninitStreamDecoder() {
    let _ = stream::uninit_stream_decoder();
}

#[no_mangle]
pub unsafe extern "C" fn TFB_CreateSoundSample(
    decoder_ptr: *mut c_void,
    num_buffers: c_uint,
    callbacks_ptr: *mut c_void,
) -> *mut c_void {
    // decoder_ptr is a fat pointer (Box::into_raw of Box<dyn SoundDecoder>)
    // stored as two usizes: (data_ptr, vtable_ptr). C code must pass this
    // through unchanged from wherever it was created (Rust side).
    let decoder: Option<Box<dyn SoundDecoder>> = if decoder_ptr.is_null() {
        None
    } else {
        // Reconstruct fat pointer from the raw pointer pair
        let fat_ptr = decoder_ptr as *mut Box<dyn SoundDecoder>;
        Some(*Box::from_raw(fat_ptr))
    };
    let callbacks = convert_c_callbacks(callbacks_ptr);
    match stream::create_sound_sample(decoder, num_buffers as u32, callbacks) {
        Ok(sample) => {
            let arc = Arc::new(Mutex::new(sample));
            Arc::into_raw(arc) as *mut c_void
        }
        Err(e) => {
            eprintln!("TFB_CreateSoundSample: {}", e);
            null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn TFB_DestroySoundSample(sample_ptr: *mut c_void) {
    if sample_ptr.is_null() {
        return;
    }
    let arc = Arc::from_raw(sample_ptr as *const Mutex<SoundSample>);
    let _ = stream::destroy_sound_sample(&mut *arc.lock());
}

#[no_mangle]
pub unsafe extern "C" fn TFB_SetSoundSampleData(sample_ptr: *mut c_void, data_ptr: *mut c_void) {
    if sample_ptr.is_null() {
        return;
    }
    let arc = arc_borrow(sample_ptr as *const Mutex<SoundSample>);
    let mut sample = arc.lock();
    let data = Box::new(data_ptr as usize);
    stream::set_sound_sample_data(&mut sample, data);
}

#[no_mangle]
pub unsafe extern "C" fn TFB_GetSoundSampleData(sample_ptr: *mut c_void) -> *mut c_void {
    if sample_ptr.is_null() {
        return null_mut();
    }
    let arc = arc_borrow(sample_ptr as *const Mutex<SoundSample>);
    let sample = arc.lock();
    match stream::get_sound_sample_data(&sample) {
        Some(data) => match data.downcast_ref::<usize>() {
            Some(&addr) => addr as *mut c_void,
            None => null_mut(),
        },
        None => null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn TFB_SetSoundSampleCallbacks(
    sample_ptr: *mut c_void,
    callbacks_ptr: *mut c_void,
) {
    if sample_ptr.is_null() {
        return;
    }
    let arc = arc_borrow(sample_ptr as *const Mutex<SoundSample>);
    let mut sample = arc.lock();
    let callbacks = convert_c_callbacks(callbacks_ptr);
    stream::set_sound_sample_callbacks(&mut sample, callbacks);
}

#[no_mangle]
pub unsafe extern "C" fn TFB_GetSoundSampleDecoder(sample_ptr: *mut c_void) -> *mut c_void {
    if sample_ptr.is_null() {
        return null_mut();
    }
    let arc = arc_borrow(sample_ptr as *const Mutex<SoundSample>);
    let sample = arc.lock();
    match stream::get_sound_sample_decoder(&sample) {
        Some(dec) => dec as *const dyn SoundDecoder as *mut c_void,
        None => null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn PlayStream(
    sample_ptr: *mut c_void,
    source: u32,
    looping: bool,
    scope: bool,
    rewind: bool,
) {
    if sample_ptr.is_null() {
        return;
    }
    let sample_arc = arc_borrow(sample_ptr as *const Mutex<SoundSample>);
    let _ = stream::play_stream(sample_arc, source as usize, looping, scope, rewind);
}

#[no_mangle]
pub extern "C" fn StopStream(source: u32) {
    let _ = stream::stop_stream(source as usize);
}

#[no_mangle]
pub extern "C" fn PauseStream(source: u32) {
    let _ = stream::pause_stream(source as usize);
}

#[no_mangle]
pub extern "C" fn ResumeStream(source: u32) {
    let _ = stream::resume_stream(source as usize);
}

#[no_mangle]
pub extern "C" fn SeekStream(source: u32, pos: u32) {
    let _ = stream::seek_stream(source as usize, pos);
}

#[no_mangle]
pub extern "C" fn PlayingStream(source: u32) -> u8 {
    if stream::playing_stream(source as usize) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn TFB_FindTaggedBuffer(
    sample_ptr: *mut c_void,
    buffer: c_uint,
) -> *mut c_void {
    if sample_ptr.is_null() {
        return null_mut();
    }
    let arc = arc_borrow(sample_ptr as *const Mutex<SoundSample>);
    let sample = arc.lock();
    match stream::find_tagged_buffer(&sample, buffer as usize) {
        Some(tag) => tag as *const SoundTag as *mut c_void,
        None => null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn TFB_TagBuffer(
    sample_ptr: *mut c_void,
    buffer: c_uint,
    data: c_uint,
) -> c_int {
    if sample_ptr.is_null() {
        return 0;
    }
    let arc = arc_borrow(sample_ptr as *const Mutex<SoundSample>);
    let mut sample = arc.lock();
    if stream::tag_buffer(&mut sample, buffer as usize, data as usize) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn TFB_ClearBufferTag(tag_ptr: *mut c_void) {
    if tag_ptr.is_null() {
        return;
    }
    let tag = &mut *(tag_ptr as *mut SoundTag);
    stream::clear_buffer_tag(tag);
}

#[no_mangle]
pub extern "C" fn SetMusicStreamFade(how_long: i32, end_volume: c_int) -> bool {
    stream::set_music_stream_fade(how_long.max(0) as u32, end_volume)
}

#[no_mangle]
pub unsafe extern "C" fn GraphForegroundStream(
    data_ptr: *mut u8,
    width: i32,
    height: i32,
    want_speech: c_int,
) -> c_int {
    if data_ptr.is_null() || width <= 0 || height <= 0 {
        return 0;
    }
    let slice = std::slice::from_raw_parts_mut(data_ptr, width as usize);
    stream::graph_foreground_stream(slice, width as usize, height as usize, want_speech != 0)
        as c_int
}

// =============================================================================
// Track Player FFI (17 functions)
// =============================================================================

/// C callback type: void (*CallbackFunction)(void* arg)
pub type CallbackFunction = unsafe extern "C" fn(*mut c_void);

#[no_mangle]
pub unsafe extern "C" fn SpliceTrack(
    track_name_ptr: *const c_char,
    track_text_ptr: *const c_char,
    timestamp_ptr: *const c_char,
    callback_ptr: Option<CallbackFunction>,
) {
    let name = unicode_ptr_to_option(track_name_ptr);
    let text = unicode_ptr_to_option(track_text_ptr);
    let timestamp = unicode_ptr_to_option(timestamp_ptr);
    eprintln!(
        "[SpliceTrack] name={:?} text_len={:?} ts_len={:?} cb={}",
        name,
        text.as_ref().map(|s| s.len()),
        timestamp.as_ref().map(|s| s.len()),
        callback_ptr.is_some()
    );
    let callback: Option<Box<dyn Fn(i32) + Send>> = callback_ptr.map(|f| {
        Box::new(move |val: i32| {
            f(val as *mut c_void);
        }) as Box<dyn Fn(i32) + Send>
    });

    // Create per-page decoders with time-limited ranges (like C's SoundDecoder_Load)
    let decoders = create_per_page_decoders(name.as_deref(), text.as_deref(), timestamp.as_deref());

    let _ = trackplayer::splice_track(
        name.as_deref(),
        text.as_deref(),
        timestamp.as_deref(),
        callback,
        decoders,
    );
}

#[no_mangle]
pub unsafe extern "C" fn SpliceMultiTrack(
    track_names_ptr: *const *const c_char,
    track_text_ptr: *const c_char,
) {
    if track_names_ptr.is_null() {
        return;
    }
    // Read null-terminated array of UNICODE* (char*) strings
    let mut names: Vec<Option<String>> = Vec::new();
    let mut i = 0;
    loop {
        let p = *track_names_ptr.add(i);
        if p.is_null() {
            break;
        }
        names.push(unicode_ptr_to_option(p));
        i += 1;
    }
    let text = unicode_ptr_to_option(track_text_ptr);
    let name_refs: Vec<Option<&str>> = names.iter().map(|n| n.as_deref()).collect();
    let text_refs: Vec<Option<&str>> = vec![text.as_deref(); name_refs.len()];
    let _ = trackplayer::splice_multi_track(&name_refs, &text_refs, None);
}

#[no_mangle]
pub extern "C" fn PlayTrack() {
    eprintln!("[PlayTrack] called");
    let _ = trackplayer::play_track(true);
}

#[no_mangle]
pub extern "C" fn StopTrack() {
    let _ = trackplayer::stop_track();
}

#[no_mangle]
pub extern "C" fn JumpTrack() {
    let _ = trackplayer::jump_track(0);
}

#[no_mangle]
pub extern "C" fn PauseTrack() {
    let _ = trackplayer::pause_track();
}

#[no_mangle]
pub extern "C" fn ResumeTrack() {
    let _ = trackplayer::resume_track();
}

#[no_mangle]
pub extern "C" fn PlayingTrack() -> u16 {
    trackplayer::playing_track_num()
}

#[no_mangle]
pub extern "C" fn FastReverse_Smooth() {
    let _ = trackplayer::fast_reverse_smooth();
}

#[no_mangle]
pub extern "C" fn FastForward_Smooth() {
    let _ = trackplayer::fast_forward_smooth();
}

#[no_mangle]
pub extern "C" fn FastReverse_Page() {
    let _ = trackplayer::fast_reverse_page();
}

#[no_mangle]
pub extern "C" fn FastForward_Page() {
    let _ = trackplayer::fast_forward_page();
}

#[no_mangle]
pub extern "C" fn GetTrackPosition(in_units: c_int) -> c_int {
    trackplayer::get_track_position(in_units as u32) as c_int
}

#[no_mangle]
pub extern "C" fn GetTrackSubtitle() -> *const c_char {
    // Return a stable pointer from the chunk's cached CString.
    // C's CheckSubtitles uses pointer identity to detect subtitle changes.
    trackplayer::get_track_subtitle_cstr()
}

#[no_mangle]
pub extern "C" fn GetFirstTrackSubtitle() -> *mut c_void {
    // Return the actual chunks_head pointer (matches C: returns chunks_head as SUBTITLE_REF)
    trackplayer::get_first_chunk_ptr() as *mut c_void
}

#[no_mangle]
pub extern "C" fn GetNextTrackSubtitle(last_ref: *mut c_void) -> *mut c_void {
    // Match C ABI: GetNextTrackSubtitle(SUBTITLE_REF LastRef)
    // LastRef is an opaque pointer to a SoundChunk in the linked list
    if last_ref.is_null() {
        return null_mut();
    }
    trackplayer::get_next_chunk_ptr(last_ref as *const u8) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn GetTrackSubtitleText(sub_ref_ptr: *mut c_void) -> *const c_char {
    if sub_ref_ptr.is_null() {
        return ptr::null();
    }
    // Return stable CString pointer from the chunk itself
    trackplayer::get_chunk_text_cstr(sub_ref_ptr as *const u8)
}

// =============================================================================
// Music FFI (10 functions)
// =============================================================================

#[no_mangle]
pub unsafe extern "C" fn PLRPlaySong(
    music_ref_ptr: *mut c_void,
    continuous: c_int,
    priority: c_int,
) {
    eprintln!(
        "[audio_heart] PLRPlaySong called: ptr={:?} continuous={} priority={}",
        music_ref_ptr, continuous, priority
    );
    if music_ref_ptr.is_null() {
        eprintln!("[audio_heart] PLRPlaySong: null ptr, returning");
        return;
    }
    if is_music_ref_sentinel(music_ref_ptr) {
        eprintln!("[audio_heart] PLRPlaySong: sentinel ~0, returning");
        return;
    }
    let music_ref = music_ref_borrow(music_ref_ptr);
    eprintln!("[audio_heart] PLRPlaySong: got music_ref, calling plr_play_song");
    match music::plr_play_song(&music_ref, continuous != 0, priority) {
        Ok(()) => eprintln!("[audio_heart] PLRPlaySong: success"),
        Err(e) => eprintln!("[audio_heart] PLRPlaySong: error: {:?}", e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn PLRStop(music_ref_ptr: *mut c_void) {
    if music_ref_ptr.is_null() {
        return;
    }
    if is_music_ref_sentinel(music_ref_ptr) {
        let _ = music::plr_stop_current();
        return;
    }
    let music_ref = music_ref_borrow(music_ref_ptr);
    let _ = music::plr_stop(&music_ref);
}

#[no_mangle]
pub unsafe extern "C" fn PLRPlaying(music_ref_ptr: *mut c_void) -> c_int {
    if music_ref_ptr.is_null() {
        return 0;
    }
    if is_music_ref_sentinel(music_ref_ptr) {
        return if music::plr_playing_current() { 1 } else { 0 };
    }
    let music_ref = music_ref_borrow(music_ref_ptr);
    if music::plr_playing(&music_ref) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn PLRSeek(music_ref_ptr: *mut c_void, pos: c_uint) {
    if music_ref_ptr.is_null() {
        return;
    }
    if is_music_ref_sentinel(music_ref_ptr) {
        let _ = music::plr_seek_current(pos);
        return;
    }
    let music_ref = music_ref_borrow(music_ref_ptr);
    let _ = music::plr_seek(&music_ref, pos);
}

#[no_mangle]
pub unsafe extern "C" fn PLRPause(music_ref_ptr: *mut c_void) {
    if music_ref_ptr.is_null() {
        return;
    }
    // C checks MusicRef == curMusicRef || MusicRef == ~0 before pausing
    // Rust just pauses the current stream regardless
    let _ = music::plr_pause();
}

#[no_mangle]
pub unsafe extern "C" fn PLRResume(music_ref_ptr: *mut c_void) {
    if music_ref_ptr.is_null() {
        return;
    }
    let _ = music::plr_resume();
}

#[no_mangle]
pub unsafe extern "C" fn snd_PlaySpeech(music_ref_ptr: *mut c_void) {
    if music_ref_ptr.is_null() {
        return;
    }
    if is_music_ref_sentinel(music_ref_ptr) {
        return;
    }
    let music_ref = music_ref_borrow(music_ref_ptr);
    let _ = music::snd_play_speech(&music_ref);
}

#[no_mangle]
pub extern "C" fn snd_StopSpeech() {
    let _ = music::snd_stop_speech();
}

#[no_mangle]
pub extern "C" fn SetMusicVolume(volume: c_int) {
    music::set_music_volume(volume);
}

#[no_mangle]
pub extern "C" fn FadeMusic(end_vol: u8, how_long: i16) -> u32 {
    let interval = if how_long < 0 { 0u32 } else { how_long as u32 };
    if crate::sound::types::quit_posted() {
        // Don't make users wait for fades on quit
        music::set_music_volume(end_vol as i32);
        return crate::sound::types::get_time_counter();
    }
    if !music::fade_music(interval, end_vol as i32) {
        music::set_music_volume(end_vol as i32);
        return crate::sound::types::get_time_counter();
    }
    crate::sound::types::get_time_counter() + interval
}

// =============================================================================
// SFX FFI (8 functions)
// =============================================================================

/// C signature: `PlayChannel(COUNT channel, SOUND snd, SoundPosition pos, void *obj, BYTE pri)`
/// SOUND is a `STRING` = `STRING_TABLE_ENTRY_DESC*`. The `.data` field is an
/// HMalloc'd slot containing a `*mut SoundSample`.
#[no_mangle]
pub unsafe extern "C" fn PlayChannel(
    channel: c_uint,
    snd_ptr: *mut c_void, // SOUND (STRING_TABLE_ENTRY_DESC*)
    pos: SoundPosition,   // passed by value in C
    positional_object: *mut c_void,
    priority: c_uint,
) {
    eprintln!(
        "[PlayChannel] channel={} snd_ptr={:?} priority={}",
        channel, snd_ptr, priority
    );
    if snd_ptr.is_null() {
        eprintln!("[PlayChannel] snd_ptr is null, returning");
        return;
    }
    // snd_ptr is a STRING_TABLE_ENTRY_DESC*. First field is `data` (a char*).
    let entry = &*(snd_ptr as *const CStringTableEntry);
    if entry.data.is_null() {
        return;
    }
    // entry.data points to an HMalloc'd slot containing *mut SoundSample
    let sample_ptr = *(entry.data as *const *const SoundSample);
    if sample_ptr.is_null() {
        return;
    }
    let sample = &*sample_ptr;

    let _ = sfx::play_sample(
        channel as usize,
        sample,
        pos,
        positional_object as usize,
        priority as i32,
    );
}

#[no_mangle]
pub extern "C" fn StopChannel(channel: c_uint, priority: c_int) {
    let _ = sfx::stop_channel(channel as usize, priority);
}

#[no_mangle]
pub extern "C" fn ChannelPlaying(channel: c_uint) -> c_int {
    if sfx::channel_playing(channel as usize) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn SetChannelVolume(channel: c_uint, volume: c_int, priority: c_int) {
    sfx::set_channel_volume(channel as usize, volume, priority);
}

#[no_mangle]
pub unsafe extern "C" fn UpdateSoundPosition(source_index: c_uint, pos_ptr: *const SoundPosition) {
    if pos_ptr.is_null() {
        return;
    }
    sfx::update_sound_position(source_index as usize, *pos_ptr);
}

#[no_mangle]
pub extern "C" fn GetPositionalObject(source_index: c_uint) -> c_uint {
    sfx::get_positional_object(source_index as usize) as c_uint
}

#[no_mangle]
pub extern "C" fn SetPositionalObject(source_index: c_uint, object: c_uint) {
    sfx::set_positional_object(source_index as usize, object as usize);
}

/// Free a SOUND_REF (STRING_TABLE containing SoundSample pointers).
/// Each entry's .data is an HMalloc'd slot pointing to a Box<SoundSample>.
#[no_mangle]
pub unsafe extern "C" fn DestroySound(bank_ptr: *mut c_void) {
    if bank_ptr.is_null() {
        return;
    }
    let strtab = &*(bank_ptr as *const CStringTable);
    for i in 0..strtab.size as usize {
        let entry = &mut *strtab.strings.add(i);
        if !entry.data.is_null() {
            let sample_pp = entry.data as *mut *mut SoundSample;
            let sample_ptr = *sample_pp;
            if !sample_ptr.is_null() {
                // Reconstruct and drop the Box<SoundSample>, destroying mixer buffers
                let mut sample = *Box::from_raw(sample_ptr);
                let _ = stream::destroy_sound_sample(&mut sample);
            }
            // Null out data so FreeStringTable won't double-free
            entry.data = null_mut();
        }
    }
    FreeStringTable(bank_ptr);
}

// =============================================================================
// Control FFI (7 functions)
// =============================================================================

#[no_mangle]
pub extern "C" fn InitSound(_argc: c_int, _argv: *const *const c_char) -> c_int {
    eprintln!("[audio_heart] InitSound called");
    match control::init_sound() {
        Ok(()) => {
            eprintln!("[audio_heart] InitSound: success");
            1
        }
        Err(e) => {
            eprintln!("[audio_heart] InitSound: error: {}", e);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn UninitSound() {
    control::uninit_sound();
}

#[no_mangle]
pub extern "C" fn StopSound() {
    control::stop_sound();
}

#[no_mangle]
pub extern "C" fn SoundPlaying() -> c_int {
    if control::sound_playing() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn WaitForSoundEnd(channel: u16) {
    // COUNT is u16; TFBSOUND_WAIT_ALL is (COUNT)~0 = 0xFFFF
    let ch = if channel == 0xFFFF {
        None
    } else {
        Some(channel as usize)
    };
    control::wait_for_sound_end(ch);
}

#[no_mangle]
pub extern "C" fn SetSFXVolume(volume: f32) {
    control::set_sfx_volume((volume * MAX_VOLUME as f32) as i32);
}

#[no_mangle]
pub extern "C" fn SetSpeechVolume(volume: f32) {
    control::set_speech_volume((volume * MAX_VOLUME as f32) as i32);
}

// =============================================================================
// File Loading FFI (4 functions)
// =============================================================================

/// Load a sound bank (.snd file listing). Returns a STRING_TABLE (C struct)
/// where each entry's `.data` points to a `*mut SoundSample`.
#[no_mangle]
pub unsafe extern "C" fn LoadSoundFile(filename: *const c_char) -> *mut c_void {
    if filename.is_null() {
        return null_mut();
    }
    let name = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return null_mut(),
    };

    eprintln!("LoadSoundFile: loading bank {}", name);

    // Read the bank file (text) via UIO
    let c_name = CString::new(name).unwrap_or_default();
    let data = match uio_read_file(c_name.as_ptr()) {
        Some(d) => d,
        None => {
            eprintln!("LoadSoundFile({}): failed to read file", name);
            return null_mut();
        }
    };

    // Extract directory prefix from the bank filename
    let dir_prefix = match name.rfind('/').or_else(|| name.rfind('\\')) {
        Some(pos) => &name[..=pos],
        None => "",
    };

    // Parse lines — each line is a sound filename relative to the bank's directory
    let text = String::from_utf8_lossy(&data);
    let mut samples: Vec<Box<SoundSample>> = Vec::new();

    for line in text.lines() {
        let snd_name = line.trim();
        if snd_name.is_empty() {
            continue;
        }

        // Build full path: dir_prefix + sound filename
        let full_path = format!("{}{}", dir_prefix, snd_name);
        eprintln!("LoadSoundFile: loading sound {}", full_path);

        let ext = match file_extension(snd_name) {
            Some(e) => e,
            None => {
                eprintln!("LoadSoundFile: no extension for '{}', skipping", snd_name);
                continue;
            }
        };

        let mut decoder = match create_decoder_for_extension(ext) {
            Some(d) => d,
            None => {
                eprintln!(
                    "LoadSoundFile: no decoder for .{}, skipping {}",
                    ext, snd_name
                );
                continue;
            }
        };

        // Read sound file
        let c_path = CString::new(full_path.as_str()).unwrap_or_default();
        let snd_data = match uio_read_file(c_path.as_ptr()) {
            Some(d) => d,
            None => {
                eprintln!("LoadSoundFile: couldn't read {}", full_path);
                continue;
            }
        };

        // Open decoder
        if let Err(e) = decoder.open_from_bytes(&snd_data, snd_name) {
            eprintln!("LoadSoundFile: decoder failed for {}: {:?}", full_path, e);
            continue;
        }

        let freq = decoder.frequency();
        let format = decoder.format();
        let length = decoder.length();

        eprintln!(
            "LoadSoundFile: decoder {} rate={} format={:?}",
            snd_name, freq, format
        );

        // Decode all PCM data
        let pcm = match super::types::decode_all(decoder.as_mut()) {
            Ok(d) => d,
            Err(e) => {
                eprintln!(
                    "LoadSoundFile: decode_all failed for {}: {:?}",
                    full_path, e
                );
                continue;
            }
        };

        eprintln!(
            "LoadSoundFile: decoded {} bytes for {}",
            pcm.len(),
            snd_name
        );

        // Create sample with 1 buffer (SFX: pre-decoded, no streaming decoder)
        let mut sample = match stream::create_sound_sample(None, 1, None) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("LoadSoundFile: create_sound_sample failed: {}", e);
                continue;
            }
        };
        sample.length = length;

        // Store decoded PCM in the mixer buffer
        if !sample.buffers.is_empty() && !pcm.is_empty() {
            let mixer_freq = mixer_get_frequency();
            let mixer_fmt = mixer_get_format();
            let _ = mixer_buffer::mixer_buffer_data(
                sample.buffers[0],
                format.to_mixer_format(),
                &pcm,
                freq,
                mixer_freq,
                mixer_fmt,
            );
        }

        samples.push(Box::new(sample));
    }

    if samples.is_empty() {
        eprintln!("LoadSoundFile({}): no sounds decoded", name);
        return null_mut();
    }

    let snd_ct = samples.len();

    // Allocate a C STRING_TABLE
    let strtab_ptr = AllocStringTable(snd_ct as c_int, 0);
    if strtab_ptr.is_null() {
        eprintln!("LoadSoundFile({}): AllocStringTable failed", name);
        // Clean up samples
        for mut s in samples {
            let _ = stream::destroy_sound_sample(&mut s);
        }
        return null_mut();
    }

    let strtab = &*(strtab_ptr as *const CStringTable);

    // Populate each entry's .data with a pointer to the SoundSample
    for (i, sample) in samples.into_iter().enumerate() {
        let entry = &mut *strtab.strings.add(i);
        let sample_ptr = Box::into_raw(sample);
        // Allocate a pointer-sized slot (matching C's `HMalloc(sizeof(TFB_SoundSample*))`)
        let target = HMalloc(std::mem::size_of::<*mut c_void>()) as *mut *mut SoundSample;
        *target = sample_ptr;
        entry.data = target as *mut c_char;
        entry.length = std::mem::size_of::<*mut c_void>() as c_int;
    }

    eprintln!("LoadSoundFile({}): loaded {} sounds", name, snd_ct);
    strtab_ptr
}

/// Load a music file. Returns a MUSIC_REF (double pointer: `*mut *mut Arc<Mutex<SoundSample>>`).
/// The caller (resource system) stores this as opaque `void*`.
#[no_mangle]
pub unsafe extern "C" fn LoadMusicFile(filename: *const c_char) -> *mut c_void {
    if filename.is_null() {
        return null_mut();
    }
    let name = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return null_mut(),
    };

    eprintln!("LoadMusicFile: loading {}", name);

    // Determine decoder type from extension
    let ext = match file_extension(name) {
        Some(e) => e,
        None => {
            eprintln!("LoadMusicFile({}): no file extension", name);
            return null_mut();
        }
    };
    let mut decoder = match create_decoder_for_extension(ext) {
        Some(d) => d,
        None => {
            eprintln!("LoadMusicFile({}): no decoder for .{}", name, ext);
            return null_mut();
        }
    };

    // Read file via UIO
    let c_name = CString::new(name).unwrap_or_default();
    let data = match uio_read_file(c_name.as_ptr()) {
        Some(d) => d,
        None => {
            eprintln!("LoadMusicFile({}): failed to read file", name);
            return null_mut();
        }
    };

    // Open decoder from bytes
    if let Err(e) = decoder.open_from_bytes(&data, name) {
        eprintln!("LoadMusicFile({}): decoder open failed: {:?}", name, e);
        return null_mut();
    }

    eprintln!(
        "LoadMusicFile: decoder rate={} format={:?} length={:.1}s",
        decoder.frequency(),
        decoder.format(),
        decoder.length()
    );

    // Create SoundSample with 64 buffers (streaming)
    let sample = match stream::create_sound_sample(Some(decoder), 64, None) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("LoadMusicFile({}): create_sound_sample failed: {}", name, e);
            return null_mut();
        }
    };

    // Wrap in Arc<Mutex> for shared streaming access
    let arc = Arc::new(Mutex::new(sample));

    // Allocate MUSIC_REF: a pointer-sized slot holding the Arc raw pointer.
    // This matches C's `h = AllocMusicData(sizeof(void*)); *h = sample;`
    let slot = HMalloc(std::mem::size_of::<*mut c_void>()) as *mut *const Mutex<SoundSample>;
    if slot.is_null() {
        eprintln!("LoadMusicFile({}): HMalloc failed", name);
        return null_mut();
    }
    *slot = Arc::into_raw(arc);
    slot as *mut c_void
}

/// Free a MUSIC_REF. The pointer is a double-pointer: `*mut *const Mutex<SoundSample>`.
/// We take ownership of the Arc (decrement refcount) and free the outer slot.
#[no_mangle]
pub unsafe extern "C" fn DestroyMusic(music_ref_ptr: *mut c_void) {
    if music_ref_ptr.is_null() {
        return;
    }
    // Take ownership of the Arc (drops refcount)
    let music_ref = music_ref_take(music_ref_ptr);
    let _ = music::release_music_data(music_ref);
    // Free the outer pointer slot
    HFree(music_ref_ptr);
}

// =============================================================================
// C Callback Wrapper
// =============================================================================

/// C callback function pointer types matching the C TFB_SoundCallbacks struct.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct CTfbSoundCallbacks {
    pub on_start_stream: Option<unsafe extern "C" fn(*mut c_void) -> c_int>,
    pub on_end_chunk: Option<unsafe extern "C" fn(*mut c_void, c_uint) -> c_int>,
    pub on_end_stream: Option<unsafe extern "C" fn(*mut c_void)>,
    pub on_tagged_buffer: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub on_queue_buffer: Option<unsafe extern "C" fn(*mut c_void, c_uint)>,
}

/// Wraps C function pointers in a Rust StreamCallbacks implementation.
struct CCallbackWrapper {
    callbacks: CTfbSoundCallbacks,
    sample_ptr: *mut c_void,
}

unsafe impl Send for CCallbackWrapper {}

impl StreamCallbacks for CCallbackWrapper {
    fn on_start_stream(&mut self, _sample: &mut SoundSample) -> bool {
        if let Some(f) = self.callbacks.on_start_stream {
            unsafe { f(self.sample_ptr) != 0 }
        } else {
            true
        }
    }

    fn on_end_chunk(&mut self, _sample: &mut SoundSample, buffer: usize) -> bool {
        if let Some(f) = self.callbacks.on_end_chunk {
            unsafe { f(self.sample_ptr, buffer as c_uint) != 0 }
        } else {
            true
        }
    }

    fn on_end_stream(&mut self, _sample: &mut SoundSample) {
        if let Some(f) = self.callbacks.on_end_stream {
            unsafe { f(self.sample_ptr) }
        }
    }

    fn on_tagged_buffer(&mut self, _sample: &mut SoundSample, tag: &SoundTag) {
        if let Some(f) = self.callbacks.on_tagged_buffer {
            unsafe { f(self.sample_ptr, tag as *const SoundTag as *mut c_void) }
        }
    }

    fn on_queue_buffer(&mut self, _sample: &mut SoundSample, buffer: usize) {
        if let Some(f) = self.callbacks.on_queue_buffer {
            unsafe { f(self.sample_ptr, buffer as c_uint) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- P19 TDD ---

    // REQ-CROSS-FFI-02: Null safety
    #[test]

    fn test_create_sound_sample_null_decoder() {
        let ptr = unsafe { TFB_CreateSoundSample(null_mut(), 4, null_mut()) };
        assert!(!ptr.is_null());
        unsafe { TFB_DestroySoundSample(ptr) };
    }

    #[test]

    fn test_destroy_sound_sample_null_ptr() {
        unsafe { TFB_DestroySoundSample(null_mut()) }; // should not panic
    }

    #[test]

    fn test_set_sound_sample_data_null_ptr() {
        unsafe { TFB_SetSoundSampleData(null_mut(), null_mut()) }; // no-op
    }

    #[test]

    fn test_get_sound_sample_data_null_ptr() {
        let result = unsafe { TFB_GetSoundSampleData(null_mut()) };
        assert!(result.is_null());
    }

    // REQ-CROSS-FFI-03: Error translation
    #[test]

    fn test_init_stream_decoder_return_type() {
        // Verify FFI function signature returns c_int
        let f: extern "C" fn() -> c_int = InitStreamDecoder;
        let _ = f; // type-check only; calling mutates global state
    }

    #[test]
    fn test_playing_stream_returns_int() {
        let result = PlayingStream(0);
        assert!(result == 0 || result == 1);
    }

    #[test]

    fn test_playing_track_returns_int() {
        let result = PlayingTrack();
        assert!(result >= 0);
    }

    #[test]

    fn test_sound_playing_returns_int() {
        let result = SoundPlaying();
        assert!(result == 0 || result == 1);
    }

    #[test]

    fn test_load_sound_file_null_returns_null() {
        let result = unsafe { LoadSoundFile(ptr::null()) };
        assert!(result.is_null());
    }

    // REQ-CROSS-FFI-04: String conversion (already working)
    #[test]
    fn test_c_str_to_option_empty() {
        let s = CString::new("").unwrap();
        let result = unsafe { c_str_to_option(s.as_ptr()) };
        assert_eq!(result, Some(""));
    }

    // REQ-CROSS-GENERAL-08: Callbacks
    #[test]
    fn test_callback_wrapper_default_on_start() {
        let mut wrapper = CCallbackWrapper {
            callbacks: CTfbSoundCallbacks {
                on_start_stream: None,
                on_end_chunk: None,
                on_end_stream: None,
                on_tagged_buffer: None,
                on_queue_buffer: None,
            },
            sample_ptr: null_mut(),
        };
        let mut sample = stream::create_sound_sample(None, 4, None).unwrap();
        assert!(wrapper.on_start_stream(&mut sample));
    }

    #[test]
    fn test_callback_wrapper_default_on_end_chunk() {
        let mut wrapper = CCallbackWrapper {
            callbacks: CTfbSoundCallbacks {
                on_start_stream: None,
                on_end_chunk: None,
                on_end_stream: None,
                on_tagged_buffer: None,
                on_queue_buffer: None,
            },
            sample_ptr: null_mut(),
        };
        let mut sample = stream::create_sound_sample(None, 4, None).unwrap();
        assert!(wrapper.on_end_chunk(&mut sample, 0));
    }

    #[test]
    fn test_c_callback_wrapper_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<CCallbackWrapper>();
    }
}
