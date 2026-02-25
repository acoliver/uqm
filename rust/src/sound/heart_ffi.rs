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
use super::fileinst;
use super::music;
use super::sfx;
use super::stream;
use super::trackplayer;
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

// =============================================================================
// Stream FFI (18 functions)
// =============================================================================

#[no_mangle]
pub extern "C" fn InitStreamDecoder() -> c_int {
    todo!("P20: InitStreamDecoder")
}

#[no_mangle]
pub extern "C" fn UninitStreamDecoder() {
    todo!("P20: UninitStreamDecoder")
}

#[no_mangle]
pub unsafe extern "C" fn TFB_CreateSoundSample(
    decoder_ptr: *mut c_void,
    num_buffers: c_uint,
    callbacks_ptr: *mut c_void,
) -> *mut c_void {
    todo!("P20: TFB_CreateSoundSample")
}

#[no_mangle]
pub unsafe extern "C" fn TFB_DestroySoundSample(sample_ptr: *mut c_void) {
    todo!("P20: TFB_DestroySoundSample")
}

#[no_mangle]
pub unsafe extern "C" fn TFB_SetSoundSampleData(sample_ptr: *mut c_void, data_ptr: *mut c_void) {
    todo!("P20: TFB_SetSoundSampleData")
}

#[no_mangle]
pub unsafe extern "C" fn TFB_GetSoundSampleData(sample_ptr: *mut c_void) -> *mut c_void {
    todo!("P20: TFB_GetSoundSampleData")
}

#[no_mangle]
pub unsafe extern "C" fn TFB_SetSoundSampleCallbacks(
    sample_ptr: *mut c_void,
    callbacks_ptr: *mut c_void,
) {
    todo!("P20: TFB_SetSoundSampleCallbacks")
}

#[no_mangle]
pub unsafe extern "C" fn TFB_GetSoundSampleDecoder(sample_ptr: *mut c_void) -> *mut c_void {
    todo!("P20: TFB_GetSoundSampleDecoder")
}

#[no_mangle]
pub unsafe extern "C" fn PlayStream(
    sample_ptr: *mut c_void,
    source: c_int,
    looping: c_int,
    scope: c_int,
    rewind: c_int,
) {
    todo!("P20: PlayStream")
}

#[no_mangle]
pub extern "C" fn StopStream(source: c_int) {
    todo!("P20: StopStream")
}

#[no_mangle]
pub extern "C" fn PauseStream(source: c_int) {
    todo!("P20: PauseStream")
}

#[no_mangle]
pub extern "C" fn ResumeStream(source: c_int) {
    todo!("P20: ResumeStream")
}

#[no_mangle]
pub extern "C" fn SeekStream(source: c_int, pos: c_uint) {
    todo!("P20: SeekStream")
}

#[no_mangle]
pub extern "C" fn PlayingStream(source: c_int) -> c_int {
    todo!("P20: PlayingStream")
}

#[no_mangle]
pub unsafe extern "C" fn TFB_FindTaggedBuffer(
    sample_ptr: *mut c_void,
    buffer: c_uint,
) -> *mut c_void {
    todo!("P20: TFB_FindTaggedBuffer")
}

#[no_mangle]
pub unsafe extern "C" fn TFB_TagBuffer(
    sample_ptr: *mut c_void,
    buffer: c_uint,
    data: c_uint,
) -> c_int {
    todo!("P20: TFB_TagBuffer")
}

#[no_mangle]
pub unsafe extern "C" fn TFB_ClearBufferTag(tag_ptr: *mut c_void) {
    todo!("P20: TFB_ClearBufferTag")
}

#[no_mangle]
pub extern "C" fn SetMusicStreamFade(how_long: c_int, end_volume: c_int) -> c_int {
    todo!("P20: SetMusicStreamFade")
}

#[no_mangle]
pub unsafe extern "C" fn GraphForegroundStream(
    data_ptr: *mut i32,
    width: c_uint,
    height: c_uint,
    want_speech: c_int,
) -> c_uint {
    todo!("P20: GraphForegroundStream")
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
    todo!("P20: SpliceTrack")
}

#[no_mangle]
pub unsafe extern "C" fn SpliceMultiTrack(
    track_names_ptr: *const *const c_char,
    track_texts_ptr: *const *const u16,
    timestamp_ptr: *const c_char,
) {
    todo!("P20: SpliceMultiTrack")
}

#[no_mangle]
pub extern "C" fn PlayTrack(scope: c_int) {
    todo!("P20: PlayTrack")
}

#[no_mangle]
pub extern "C" fn StopTrack() {
    todo!("P20: StopTrack")
}

#[no_mangle]
pub extern "C" fn JumpTrack(track_num: c_uint) {
    todo!("P20: JumpTrack")
}

#[no_mangle]
pub extern "C" fn PauseTrack() {
    todo!("P20: PauseTrack")
}

#[no_mangle]
pub extern "C" fn ResumeTrack() {
    todo!("P20: ResumeTrack")
}

#[no_mangle]
pub extern "C" fn PlayingTrack() -> c_int {
    todo!("P20: PlayingTrack")
}

#[no_mangle]
pub extern "C" fn FastReverse_Smooth() {
    todo!("P20: FastReverse_Smooth")
}

#[no_mangle]
pub extern "C" fn FastForward_Smooth() {
    todo!("P20: FastForward_Smooth")
}

#[no_mangle]
pub extern "C" fn FastReverse_Page() {
    todo!("P20: FastReverse_Page")
}

#[no_mangle]
pub extern "C" fn FastForward_Page() {
    todo!("P20: FastForward_Page")
}

#[no_mangle]
pub extern "C" fn GetTrackPosition(in_units: c_uint) -> c_uint {
    todo!("P20: GetTrackPosition")
}

#[no_mangle]
pub extern "C" fn GetTrackSubtitle() -> *const c_char {
    todo!("P20: GetTrackSubtitle")
}

#[no_mangle]
pub extern "C" fn GetFirstTrackSubtitle() -> *mut c_void {
    todo!("P20: GetFirstTrackSubtitle")
}

#[no_mangle]
pub extern "C" fn GetNextTrackSubtitle() -> *mut c_void {
    todo!("P20: GetNextTrackSubtitle")
}

#[no_mangle]
pub unsafe extern "C" fn GetTrackSubtitleText(sub_ref_ptr: *mut c_void) -> *const c_char {
    todo!("P20: GetTrackSubtitleText")
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
    todo!("P20: PLRPlaySong")
}

#[no_mangle]
pub unsafe extern "C" fn PLRStop(music_ref_ptr: *mut c_void) {
    todo!("P20: PLRStop")
}

#[no_mangle]
pub unsafe extern "C" fn PLRPlaying(music_ref_ptr: *mut c_void) -> c_int {
    todo!("P20: PLRPlaying")
}

#[no_mangle]
pub unsafe extern "C" fn PLRSeek(music_ref_ptr: *mut c_void, pos: c_uint) {
    todo!("P20: PLRSeek")
}

#[no_mangle]
pub extern "C" fn PLRPause() {
    todo!("P20: PLRPause")
}

#[no_mangle]
pub extern "C" fn PLRResume() {
    todo!("P20: PLRResume")
}

#[no_mangle]
pub unsafe extern "C" fn snd_PlaySpeech(music_ref_ptr: *mut c_void) {
    todo!("P20: snd_PlaySpeech")
}

#[no_mangle]
pub extern "C" fn snd_StopSpeech() {
    todo!("P20: snd_StopSpeech")
}

#[no_mangle]
pub extern "C" fn SetMusicVolume(volume: c_int) {
    todo!("P20: SetMusicVolume")
}

#[no_mangle]
pub extern "C" fn FadeMusic(end_vol: c_int, how_long: c_int) -> c_uint {
    todo!("P20: FadeMusic")
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
    todo!("P20: PlayChannel")
}

#[no_mangle]
pub extern "C" fn StopChannel(channel: c_uint, priority: c_int) {
    todo!("P20: StopChannel")
}

#[no_mangle]
pub extern "C" fn ChannelPlaying(channel: c_uint) -> c_int {
    todo!("P20: ChannelPlaying")
}

#[no_mangle]
pub extern "C" fn SetChannelVolume(channel: c_uint, volume: c_int, priority: c_int) {
    todo!("P20: SetChannelVolume")
}

#[no_mangle]
pub unsafe extern "C" fn UpdateSoundPosition(source_index: c_uint, pos_ptr: *const SoundPosition) {
    todo!("P20: UpdateSoundPosition")
}

#[no_mangle]
pub extern "C" fn GetPositionalObject(source_index: c_uint) -> c_uint {
    todo!("P20: GetPositionalObject")
}

#[no_mangle]
pub extern "C" fn SetPositionalObject(source_index: c_uint, object: c_uint) {
    todo!("P20: SetPositionalObject")
}

#[no_mangle]
pub unsafe extern "C" fn DestroySound(bank_ptr: *mut c_void) {
    todo!("P20: DestroySound")
}

// =============================================================================
// Control FFI (7 functions)
// =============================================================================

#[no_mangle]
pub extern "C" fn InitSound() -> c_int {
    todo!("P20: InitSound")
}

#[no_mangle]
pub extern "C" fn UninitSound() {
    todo!("P20: UninitSound")
}

#[no_mangle]
pub extern "C" fn StopSound() {
    todo!("P20: StopSound")
}

#[no_mangle]
pub extern "C" fn SoundPlaying() -> c_int {
    todo!("P20: SoundPlaying")
}

#[no_mangle]
pub extern "C" fn WaitForSoundEnd(channel: c_int) {
    todo!("P20: WaitForSoundEnd")
}

#[no_mangle]
pub extern "C" fn SetSFXVolume(volume: c_int) {
    todo!("P20: SetSFXVolume")
}

#[no_mangle]
pub extern "C" fn SetSpeechVolume(volume: c_int) {
    todo!("P20: SetSpeechVolume")
}

// =============================================================================
// File Loading FFI (4 functions)
// =============================================================================

#[no_mangle]
pub unsafe extern "C" fn LoadSoundFile(filename: *const c_char) -> *mut c_void {
    todo!("P20: LoadSoundFile")
}

#[no_mangle]
pub unsafe extern "C" fn LoadMusicFile(filename: *const c_char) -> *mut c_void {
    todo!("P20: LoadMusicFile")
}

#[no_mangle]
pub unsafe extern "C" fn DestroyMusic(music_ref_ptr: *mut c_void) {
    todo!("P20: DestroyMusic")
}

// =============================================================================
// C Callback Wrapper
// =============================================================================

/// C callback function pointer types matching the C TFB_SoundCallbacks struct.
#[repr(C)]
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- P19 TDD ---

    // REQ-CROSS-FFI-02: Null safety
    #[test]
    #[ignore = "P20: TFB_CreateSoundSample stub"]
    fn test_create_sound_sample_null_decoder() {
        let ptr = unsafe { TFB_CreateSoundSample(null_mut(), 4, null_mut()) };
        assert!(!ptr.is_null());
        unsafe { TFB_DestroySoundSample(ptr) };
    }

    #[test]
    #[ignore = "P20: TFB_DestroySoundSample stub"]
    fn test_destroy_sound_sample_null_ptr() {
        unsafe { TFB_DestroySoundSample(null_mut()) }; // should not panic
    }

    #[test]
    #[ignore = "P20: TFB_SetSoundSampleData stub"]
    fn test_set_sound_sample_data_null_ptr() {
        unsafe { TFB_SetSoundSampleData(null_mut(), null_mut()) }; // no-op
    }

    #[test]
    #[ignore = "P20: TFB_GetSoundSampleData stub"]
    fn test_get_sound_sample_data_null_ptr() {
        let result = unsafe { TFB_GetSoundSampleData(null_mut()) };
        assert!(result.is_null());
    }

    // REQ-CROSS-FFI-03: Error translation
    #[test]
    #[ignore = "P20: InitStreamDecoder stub"]
    fn test_init_stream_decoder_return_code() {
        let code = InitStreamDecoder();
        assert!(code == 0 || code == -1);
    }

    #[test]
    #[ignore = "P20: PlayingStream stub"]
    fn test_playing_stream_returns_int() {
        let result = PlayingStream(0);
        assert!(result == 0 || result == 1);
    }

    #[test]
    #[ignore = "P20: PlayingTrack stub"]
    fn test_playing_track_returns_int() {
        let result = PlayingTrack();
        assert!(result >= 0);
    }

    #[test]
    #[ignore = "P20: SoundPlaying stub"]
    fn test_sound_playing_returns_int() {
        let result = SoundPlaying();
        assert!(result == 0 || result == 1);
    }

    #[test]
    #[ignore = "P20: LoadSoundFile stub"]
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

    #[test]
    fn test_utf16_ptr_to_option_null() {
        let result = unsafe { utf16_ptr_to_option(ptr::null()) };
        assert!(result.is_none());
    }

    #[test]
    fn test_utf16_ptr_to_option_valid() {
        let data: Vec<u16> = "hello".encode_utf16().chain(std::iter::once(0)).collect();
        let result = unsafe { utf16_ptr_to_option(data.as_ptr()) };
        assert_eq!(result.as_deref(), Some("hello"));
    }

    #[test]
    fn test_c_str_to_option_null() {
        let result = unsafe { c_str_to_option(ptr::null()) };
        assert!(result.is_none());
    }

    #[test]
    fn test_c_str_to_option_valid() {
        let s = CString::new("test").unwrap();
        let result = unsafe { c_str_to_option(s.as_ptr()) };
        assert_eq!(result, Some("test"));
    }
}
