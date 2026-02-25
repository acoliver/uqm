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
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::ptr::{self, null_mut};
use std::sync::Arc;

use parking_lot::Mutex;

use super::control;
use super::decoder::SoundDecoder;
use super::fileinst;
use super::music;
use super::sfx;
use super::stream;
use super::trackplayer;
use super::trackplayer::SubtitleRef;
use super::types::*;

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

unsafe fn utf16_ptr_to_option(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0usize;
    let mut p = ptr;
    while *p != 0 {
        len += 1;
        p = p.add(1);
    }
    if len == 0 {
        return None;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    Some(String::from_utf16_lossy(slice))
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

/// Borrow an Arc from a raw pointer without consuming ownership.
/// Caller must ensure `ptr` came from `Arc::into_raw`.
unsafe fn arc_borrow<T>(ptr: *const T) -> Arc<T> {
    Arc::increment_strong_count(ptr);
    Arc::from_raw(ptr)
}
/// Reconstruct a MusicRef from a raw pointer WITHOUT taking ownership.
unsafe fn reconstruct_music_ref_borrowed(ptr: *mut c_void) -> MusicRef {
    let typed_ptr = ptr as *const Mutex<SoundSample>;
    Arc::increment_strong_count(typed_ptr);
    MusicRef(Arc::from_raw(typed_ptr))
}

/// Reconstruct a MusicRef from a raw pointer, TAKING ownership (consumes the reference).
unsafe fn reconstruct_music_ref_owned(ptr: *mut c_void) -> MusicRef {
    MusicRef(Arc::from_raw(ptr as *const Mutex<SoundSample>))
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

// Thread-local subtitle ref cache for GetFirstTrackSubtitle/GetNextTrackSubtitle
thread_local! {
    static SUBTITLE_REF_CACHE: RefCell<Option<SubtitleRef>> = const { RefCell::new(None) };
}

// =============================================================================
// Stream FFI (18 functions)
// =============================================================================

#[no_mangle]
pub extern "C" fn InitStreamDecoder() -> c_int {
    match stream::init_stream_decoder() {
        Ok(()) => 0,
        Err(e) => {
            log::error!("InitStreamDecoder: {}", e);
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
            log::error!("TFB_CreateSoundSample: {}", e);
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
    source: c_int,
    looping: c_int,
    scope: c_int,
    rewind: c_int,
) {
    if sample_ptr.is_null() {
        return;
    }
    let sample_arc = arc_borrow(sample_ptr as *const Mutex<SoundSample>);
    let _ = stream::play_stream(
        sample_arc,
        source as usize,
        looping != 0,
        scope != 0,
        rewind != 0,
    );
}

#[no_mangle]
pub extern "C" fn StopStream(source: c_int) {
    let _ = stream::stop_stream(source as usize);
}

#[no_mangle]
pub extern "C" fn PauseStream(source: c_int) {
    let _ = stream::pause_stream(source as usize);
}

#[no_mangle]
pub extern "C" fn ResumeStream(source: c_int) {
    let _ = stream::resume_stream(source as usize);
}

#[no_mangle]
pub extern "C" fn SeekStream(source: c_int, pos: c_uint) {
    let _ = stream::seek_stream(source as usize, pos);
}

#[no_mangle]
pub extern "C" fn PlayingStream(source: c_int) -> c_int {
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
pub extern "C" fn SetMusicStreamFade(how_long: c_int, end_volume: c_int) -> c_int {
    if stream::set_music_stream_fade(how_long as u32, end_volume as i32) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn GraphForegroundStream(
    data_ptr: *mut i32,
    width: c_uint,
    height: c_uint,
    want_speech: c_int,
) -> c_uint {
    if data_ptr.is_null() || width == 0 || height == 0 {
        return 0;
    }
    let slice = std::slice::from_raw_parts_mut(data_ptr, width as usize);
    stream::graph_foreground_stream(slice, width as usize, height as usize, want_speech != 0)
        as c_uint
}

// =============================================================================
// Track Player FFI (17 functions)
// =============================================================================

pub type TrackCallback = unsafe extern "C" fn(c_int);

#[no_mangle]
pub unsafe extern "C" fn SpliceTrack(
    track_name_ptr: *const c_char,
    track_text_ptr: *const u16,
    timestamp_ptr: *const c_char,
    callback_ptr: Option<TrackCallback>,
) {
    let name = c_str_to_option(track_name_ptr);
    let text = utf16_ptr_to_option(track_text_ptr);
    let timestamp = c_str_to_option(timestamp_ptr);
    let callback: Option<Box<dyn Fn(i32) + Send>> = callback_ptr.map(|f| {
        Box::new(move |val: i32| {
            f(val as c_int);
        }) as Box<dyn Fn(i32) + Send>
    });
    let _ = trackplayer::splice_track(name, text.as_deref(), timestamp, callback);
}

#[no_mangle]
pub unsafe extern "C" fn SpliceMultiTrack(
    track_names_ptr: *const *const c_char,
    track_texts_ptr: *const *const u16,
    timestamp_ptr: *const c_char,
) {
    if track_names_ptr.is_null() {
        return;
    }
    // Read null-terminated array of C strings
    let mut names: Vec<Option<&str>> = Vec::new();
    let mut i = 0;
    loop {
        let p = *track_names_ptr.add(i);
        if p.is_null() {
            break;
        }
        names.push(c_str_to_option(p));
        i += 1;
    }
    // Read corresponding texts (null-terminated array of UTF-16 strings)
    let mut texts: Vec<Option<String>> = Vec::new();
    if !track_texts_ptr.is_null() {
        let mut j = 0;
        loop {
            let p = *track_texts_ptr.add(j);
            if p.is_null() {
                break;
            }
            texts.push(utf16_ptr_to_option(p));
            j += 1;
        }
    }
    let text_refs: Vec<Option<&str>> = texts.iter().map(|t| t.as_deref()).collect();
    let _ = trackplayer::splice_multi_track(&names, &text_refs, c_str_to_option(timestamp_ptr));
}

#[no_mangle]
pub extern "C" fn PlayTrack(scope: c_int) {
    let _ = trackplayer::play_track(scope != 0);
}

#[no_mangle]
pub extern "C" fn StopTrack() {
    let _ = trackplayer::stop_track();
}

#[no_mangle]
pub extern "C" fn JumpTrack(track_num: c_uint) {
    let _ = trackplayer::jump_track(track_num);
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
pub extern "C" fn PlayingTrack() -> c_int {
    if trackplayer::playing_track() {
        1
    } else {
        0
    }
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
pub extern "C" fn GetTrackPosition(in_units: c_uint) -> c_uint {
    trackplayer::get_track_position(in_units) as c_uint
}

#[no_mangle]
pub extern "C" fn GetTrackSubtitle() -> *const c_char {
    match trackplayer::get_track_subtitle() {
        Some(text) => cache_and_return_c_str_subtitle(&text),
        None => ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn GetFirstTrackSubtitle() -> *mut c_void {
    match trackplayer::get_first_track_subtitle() {
        Some(sub_ref) => {
            SUBTITLE_REF_CACHE.with(|cache| {
                *cache.borrow_mut() = Some(sub_ref);
            });
            SUBTITLE_REF_CACHE.with(|cache| {
                let borrow = cache.borrow();
                match borrow.as_ref() {
                    Some(r) => r as *const SubtitleRef as *mut c_void,
                    None => null_mut(),
                }
            })
        }
        None => null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn GetNextTrackSubtitle() -> *mut c_void {
    match trackplayer::get_next_track_subtitle() {
        Some(sub_ref) => {
            SUBTITLE_REF_CACHE.with(|cache| {
                *cache.borrow_mut() = Some(sub_ref);
            });
            SUBTITLE_REF_CACHE.with(|cache| {
                let borrow = cache.borrow();
                match borrow.as_ref() {
                    Some(r) => r as *const SubtitleRef as *mut c_void,
                    None => null_mut(),
                }
            })
        }
        None => null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn GetTrackSubtitleText(sub_ref_ptr: *mut c_void) -> *const c_char {
    if sub_ref_ptr.is_null() {
        return ptr::null();
    }
    let sub_ref = &*(sub_ref_ptr as *const SubtitleRef);
    cache_and_return_c_str_text(&sub_ref.text)
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
    if music_ref_ptr.is_null() {
        return;
    }
    let music_ref = reconstruct_music_ref_borrowed(music_ref_ptr);
    let _ = music::plr_play_song(&music_ref, continuous != 0, priority);
}

#[no_mangle]
pub unsafe extern "C" fn PLRStop(music_ref_ptr: *mut c_void) {
    if music_ref_ptr.is_null() {
        return;
    }
    let music_ref = reconstruct_music_ref_borrowed(music_ref_ptr);
    let _ = music::plr_stop(&music_ref);
}

#[no_mangle]
pub unsafe extern "C" fn PLRPlaying(music_ref_ptr: *mut c_void) -> c_int {
    if music_ref_ptr.is_null() {
        return 0;
    }
    let music_ref = reconstruct_music_ref_borrowed(music_ref_ptr);
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
    let music_ref = reconstruct_music_ref_borrowed(music_ref_ptr);
    let _ = music::plr_seek(&music_ref, pos);
}

#[no_mangle]
pub extern "C" fn PLRPause() {
    let _ = music::plr_pause();
}

#[no_mangle]
pub extern "C" fn PLRResume() {
    let _ = music::plr_resume();
}

#[no_mangle]
pub unsafe extern "C" fn snd_PlaySpeech(music_ref_ptr: *mut c_void) {
    if music_ref_ptr.is_null() {
        return;
    }
    let music_ref = reconstruct_music_ref_borrowed(music_ref_ptr);
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
pub extern "C" fn FadeMusic(end_vol: c_int, how_long: c_int) -> c_uint {
    if music::fade_music(how_long as u32, end_vol) {
        1
    } else {
        0
    }
}

// =============================================================================
// SFX FFI (8 functions)
// =============================================================================

#[no_mangle]
pub unsafe extern "C" fn PlayChannel(
    channel: c_uint,
    sound_bank_ptr: *mut c_void,
    sound_index: c_uint,
    pos_ptr: *const SoundPosition,
    positional_object: c_uint,
    priority: c_int,
) {
    if sound_bank_ptr.is_null() {
        return;
    }
    let bank = &*(sound_bank_ptr as *const SoundBank);
    let pos = if pos_ptr.is_null() {
        SoundPosition::default()
    } else {
        *pos_ptr
    };
    let _ = sfx::play_channel(
        channel as usize,
        bank,
        sound_index as usize,
        pos,
        positional_object as usize,
        priority,
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

#[no_mangle]
pub unsafe extern "C" fn DestroySound(bank_ptr: *mut c_void) {
    if bank_ptr.is_null() {
        return;
    }
    let bank = *Box::from_raw(bank_ptr as *mut SoundBank);
    let _ = sfx::release_sound_bank_data(bank);
}

// =============================================================================
// Control FFI (7 functions)
// =============================================================================

#[no_mangle]
pub extern "C" fn InitSound() -> c_int {
    match control::init_sound() {
        Ok(()) => 1,
        Err(e) => {
            log::error!("InitSound: {}", e);
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
pub extern "C" fn WaitForSoundEnd(channel: c_int) {
    let ch = if channel < 0 {
        None
    } else {
        Some(channel as usize)
    };
    control::wait_for_sound_end(ch);
}

#[no_mangle]
pub extern "C" fn SetSFXVolume(volume: c_int) {
    control::set_sfx_volume(volume);
}

#[no_mangle]
pub extern "C" fn SetSpeechVolume(volume: c_int) {
    control::set_speech_volume(volume);
}

// =============================================================================
// File Loading FFI (4 functions)
// =============================================================================

#[no_mangle]
pub unsafe extern "C" fn LoadSoundFile(filename: *const c_char) -> *mut c_void {
    if filename.is_null() {
        return null_mut();
    }
    let name = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return null_mut(),
    };
    match fileinst::load_sound_file(name) {
        Ok(bank) => Box::into_raw(Box::new(bank)) as *mut c_void,
        Err(e) => {
            log::error!("LoadSoundFile({}): {}", name, e);
            null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn LoadMusicFile(filename: *const c_char) -> *mut c_void {
    if filename.is_null() {
        return null_mut();
    }
    let name = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return null_mut(),
    };
    match fileinst::load_music_file(name) {
        Ok(music_ref) => Arc::into_raw(music_ref.0) as *mut c_void,
        Err(e) => {
            log::error!("LoadMusicFile({}): {}", name, e);
            null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn DestroyMusic(music_ref_ptr: *mut c_void) {
    if music_ref_ptr.is_null() {
        return;
    }
    let music_ref = reconstruct_music_ref_owned(music_ref_ptr);
    let _ = music::release_music_data(music_ref);
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
