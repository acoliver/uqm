//! C FFI bindings for the communication system
//!
//! Provides C-compatible functions for the communication system.

use std::ffi::{c_char, c_int, c_uint, c_void, CStr};

use super::response::ResponseEntry;
use super::state::COMM_STATE;
use super::track::SoundChunk;
use super::types::{CommData, CommIntroMode};

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the communication system
#[no_mangle]
pub extern "C" fn rust_InitCommunication() -> c_int {
    match COMM_STATE.write().init() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Uninitialize the communication system
#[no_mangle]
pub extern "C" fn rust_UninitCommunication() {
    COMM_STATE.write().uninit();
}

/// Check if communication is initialized
#[no_mangle]
pub extern "C" fn rust_IsCommInitialized() -> c_int {
    if COMM_STATE.read().is_initialized() {
        1
    } else {
        0
    }
}

/// Clear communication state
#[no_mangle]
pub extern "C" fn rust_ClearCommunication() {
    COMM_STATE.write().clear();
}

// ============================================================================
// Track Management
// ============================================================================

/// Start the speech track
#[no_mangle]
pub extern "C" fn rust_StartTrack() -> c_int {
    match COMM_STATE.write().start_track() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Stop the speech track
#[no_mangle]
pub extern "C" fn rust_StopTrack() {
    COMM_STATE.write().stop_track();
}

/// Rewind the track to the beginning
#[no_mangle]
pub extern "C" fn rust_RewindTrack() {
    COMM_STATE.write().track_mut().rewind();
}

/// Jump within the track by offset seconds
#[no_mangle]
pub extern "C" fn rust_JumpTrack(offset: f32) {
    COMM_STATE.write().track_mut().jump(offset);
}

/// Seek to absolute position in track
#[no_mangle]
pub extern "C" fn rust_SeekTrack(position: f32) {
    COMM_STATE.write().track_mut().seek(position);
}

/// Commit track position (for save/restore)
#[no_mangle]
pub extern "C" fn rust_CommitTrack() -> f32 {
    COMM_STATE.write().track_mut().commit()
}

/// Wait for track to finish (returns 1 when done)
#[no_mangle]
pub extern "C" fn rust_WaitTrack() -> c_int {
    if COMM_STATE.read().wait_track() {
        1
    } else {
        0
    }
}

/// Get track position
#[no_mangle]
pub extern "C" fn rust_GetTrackPosition() -> f32 {
    COMM_STATE.read().track().position()
}

/// Get track length
#[no_mangle]
pub extern "C" fn rust_GetTrackLength() -> f32 {
    COMM_STATE.read().track().length()
}

/// Add a speech chunk to the track
#[no_mangle]
pub extern "C" fn rust_SpliceTrack(
    audio_handle: c_uint,
    text: *const c_char,
    start_time: f32,
    duration: f32,
) {
    let subtitle = if text.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(text).to_str().ok() }
    };

    COMM_STATE
        .write()
        .track_mut()
        .splice_track(audio_handle, subtitle, start_time, duration);
}

/// Add text-only subtitle to the track
#[no_mangle]
pub extern "C" fn rust_SpliceTrackText(text: *const c_char, start_time: f32, duration: f32) {
    if text.is_null() {
        return;
    }

    let text_str = unsafe {
        match CStr::from_ptr(text).to_str() {
            Ok(s) => s,
            Err(_) => return,
        }
    };

    COMM_STATE
        .write()
        .track_mut()
        .splice_text(text_str, start_time, duration);
}

/// Clear the track
#[no_mangle]
pub extern "C" fn rust_ClearTrack() {
    COMM_STATE.write().track_mut().clear();
}

// ============================================================================
// Subtitle Management
// ============================================================================

/// Get current subtitle (returns null if none)
#[no_mangle]
pub extern "C" fn rust_GetSubtitle() -> *const c_char {
    // Note: This returns a pointer to internal data which is only valid
    // while the lock is held. C code must copy the string immediately.
    let state = COMM_STATE.read();
    match state.current_subtitle() {
        Some(s) => s.as_ptr() as *const c_char,
        None => std::ptr::null(),
    }
}

/// Enable/disable subtitles
#[no_mangle]
pub extern "C" fn rust_SetSubtitlesEnabled(enabled: c_int) {
    COMM_STATE
        .write()
        .subtitles_mut()
        .set_enabled(enabled != 0);
}

/// Check if subtitles are enabled
#[no_mangle]
pub extern "C" fn rust_AreSubtitlesEnabled() -> c_int {
    if COMM_STATE.read().subtitles().is_enabled() {
        1
    } else {
        0
    }
}

// ============================================================================
// Response System
// ============================================================================

/// Add a response option
#[no_mangle]
pub extern "C" fn rust_DoResponsePhrase(
    response_ref: c_uint,
    text: *const c_char,
    func: Option<extern "C" fn()>,
) -> c_int {
    if text.is_null() {
        return 0;
    }

    let text_str = unsafe {
        match CStr::from_ptr(text).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    let func_addr = func.map(|f| f as usize);

    if COMM_STATE
        .write()
        .add_response(response_ref, text_str, func_addr)
    {
        1
    } else {
        0
    }
}

/// Display response choices
#[no_mangle]
pub extern "C" fn rust_DisplayResponses() {
    COMM_STATE.write().display_responses();
}

/// Clear all responses
#[no_mangle]
pub extern "C" fn rust_ClearResponses() {
    COMM_STATE.write().clear_responses();
}

/// Select next response
#[no_mangle]
pub extern "C" fn rust_SelectNextResponse() -> c_int {
    if COMM_STATE.write().select_next_response() {
        1
    } else {
        0
    }
}

/// Select previous response
#[no_mangle]
pub extern "C" fn rust_SelectPrevResponse() -> c_int {
    if COMM_STATE.write().select_prev_response() {
        1
    } else {
        0
    }
}

/// Get selected response index
#[no_mangle]
pub extern "C" fn rust_GetSelectedResponse() -> c_int {
    COMM_STATE.read().selected_response()
}

/// Get number of responses
#[no_mangle]
pub extern "C" fn rust_GetResponseCount() -> c_int {
    COMM_STATE.read().responses().count() as c_int
}

/// Execute selected response callback
#[no_mangle]
pub extern "C" fn rust_ExecuteResponse() -> c_uint {
    let state = COMM_STATE.read();
    let response = match state.responses().get_selected() {
        Some(r) => r,
        None => return 0,
    };

    let func_addr = match response.response_func {
        Some(addr) => addr,
        None => return response.response_ref,
    };

    let response_ref = response.response_ref;

    // Drop the lock before calling the callback
    drop(state);

    // Call the callback
    unsafe {
        let func: extern "C" fn() = std::mem::transmute(func_addr);
        func();
    }

    response_ref
}

// ============================================================================
// Animation Management
// ============================================================================

/// Start an animation
#[no_mangle]
pub extern "C" fn rust_StartCommAnimation(index: c_uint) {
    COMM_STATE.write().animations_mut().start(index as usize);
}

/// Stop an animation
#[no_mangle]
pub extern "C" fn rust_StopCommAnimation(index: c_uint) {
    COMM_STATE.write().animations_mut().stop(index as usize);
}

/// Start all animations
#[no_mangle]
pub extern "C" fn rust_StartAllCommAnimations() {
    COMM_STATE.write().animations_mut().start_all();
}

/// Stop all animations
#[no_mangle]
pub extern "C" fn rust_StopAllCommAnimations() {
    COMM_STATE.write().animations_mut().stop_all();
}

/// Pause all animations
#[no_mangle]
pub extern "C" fn rust_PauseCommAnimations() {
    COMM_STATE.write().animations_mut().pause();
}

/// Resume all animations
#[no_mangle]
pub extern "C" fn rust_ResumeCommAnimations() {
    COMM_STATE.write().animations_mut().resume();
}

/// Get current frame for an animation
#[no_mangle]
pub extern "C" fn rust_GetCommAnimationFrame(index: c_uint) -> c_uint {
    COMM_STATE
        .read()
        .animations()
        .get(index as usize)
        .map(|a| a.frame_index())
        .unwrap_or(0)
}

// ============================================================================
// Oscilloscope
// ============================================================================

/// Add samples to the oscilloscope
#[no_mangle]
pub extern "C" fn rust_AddOscilloscopeSamples(samples: *const i16, count: c_uint) {
    if samples.is_null() || count == 0 {
        return;
    }

    let samples_slice = unsafe { std::slice::from_raw_parts(samples, count as usize) };
    COMM_STATE.write().add_oscilloscope_samples(samples_slice);
}

/// Update the oscilloscope display
#[no_mangle]
pub extern "C" fn rust_UpdateOscilloscope() {
    COMM_STATE.write().oscilloscope_mut().update();
}

/// Get oscilloscope Y value at position
#[no_mangle]
pub extern "C" fn rust_GetOscilloscopeY(x: c_uint) -> u8 {
    COMM_STATE.read().oscilloscope().get_y(x as usize)
}

/// Clear the oscilloscope
#[no_mangle]
pub extern "C" fn rust_ClearOscilloscope() {
    COMM_STATE.write().oscilloscope_mut().clear();
}

// ============================================================================
// State Queries
// ============================================================================

/// Check if alien is talking
#[no_mangle]
pub extern "C" fn rust_IsTalking() -> c_int {
    if COMM_STATE.read().is_talking() {
        1
    } else {
        0
    }
}

/// Check if talking has finished
#[no_mangle]
pub extern "C" fn rust_IsTalkingFinished() -> c_int {
    if COMM_STATE.read().is_talking_finished() {
        1
    } else {
        0
    }
}

/// Set talking finished flag
#[no_mangle]
pub extern "C" fn rust_SetTalkingFinished(finished: c_int) {
    COMM_STATE.write().set_talking_finished(finished != 0);
}

/// Get intro mode
#[no_mangle]
pub extern "C" fn rust_GetCommIntroMode() -> c_uint {
    COMM_STATE.read().intro_mode() as c_uint
}

/// Set intro mode
#[no_mangle]
pub extern "C" fn rust_SetCommIntroMode(mode: c_uint) {
    COMM_STATE.write().set_intro_mode(CommIntroMode::from(mode));
}

/// Get fade time
#[no_mangle]
pub extern "C" fn rust_GetCommFadeTime() -> c_uint {
    COMM_STATE.read().fade_time()
}

/// Set fade time
#[no_mangle]
pub extern "C" fn rust_SetCommFadeTime(time: c_uint) {
    COMM_STATE.write().set_fade_time(time);
}

/// Check if input is paused
#[no_mangle]
pub extern "C" fn rust_IsCommInputPaused() -> c_int {
    if COMM_STATE.read().is_input_paused() {
        1
    } else {
        0
    }
}

/// Set input paused
#[no_mangle]
pub extern "C" fn rust_SetCommInputPaused(paused: c_int) {
    COMM_STATE.write().set_input_paused(paused != 0);
}

/// Update communication state
#[no_mangle]
pub extern "C" fn rust_UpdateCommunication(delta_time: f32) {
    COMM_STATE.write().update(delta_time);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::ffi::CString;

    fn reset_state() {
        rust_UninitCommunication();
    }

    #[test]
    #[serial]
    fn test_ffi_init_uninit() {
        reset_state();

        assert_eq!(rust_InitCommunication(), 1);
        assert_eq!(rust_IsCommInitialized(), 1);

        rust_UninitCommunication();
        assert_eq!(rust_IsCommInitialized(), 0);
    }

    #[test]
    #[serial]
    fn test_ffi_track_management() {
        reset_state();
        rust_InitCommunication();

        let text = CString::new("Hello world").unwrap();
        rust_SpliceTrack(1, text.as_ptr(), 0.0, 2.0);

        assert_eq!(rust_StartTrack(), 1);
        assert_eq!(rust_IsTalking(), 1);

        rust_StopTrack();
        assert_eq!(rust_IsTalking(), 0);

        rust_UninitCommunication();
    }

    #[test]
    #[serial]
    fn test_ffi_response_system() {
        reset_state();
        rust_InitCommunication();

        let text1 = CString::new("Option A").unwrap();
        let text2 = CString::new("Option B").unwrap();

        assert_eq!(rust_DoResponsePhrase(1, text1.as_ptr(), None), 1);
        assert_eq!(rust_DoResponsePhrase(2, text2.as_ptr(), None), 1);

        assert_eq!(rust_GetResponseCount(), 2);

        rust_DisplayResponses();
        assert_eq!(rust_GetSelectedResponse(), 0);

        rust_SelectNextResponse();
        assert_eq!(rust_GetSelectedResponse(), 1);

        rust_ClearResponses();
        assert_eq!(rust_GetResponseCount(), 0);

        rust_UninitCommunication();
    }

    #[test]
    #[serial]
    fn test_ffi_subtitles() {
        reset_state();
        rust_InitCommunication();

        assert_eq!(rust_AreSubtitlesEnabled(), 1);

        rust_SetSubtitlesEnabled(0);
        assert_eq!(rust_AreSubtitlesEnabled(), 0);

        rust_SetSubtitlesEnabled(1);
        assert_eq!(rust_AreSubtitlesEnabled(), 1);

        rust_UninitCommunication();
    }

    #[test]
    #[serial]
    fn test_ffi_oscilloscope() {
        reset_state();
        rust_InitCommunication();

        let samples: [i16; 4] = [100, 200, 300, 400];
        rust_AddOscilloscopeSamples(samples.as_ptr(), 4);
        rust_UpdateOscilloscope();

        // Should return a value in range
        let y = rust_GetOscilloscopeY(0);
        assert!(y <= 255);

        rust_ClearOscilloscope();

        rust_UninitCommunication();
    }

    #[test]
    #[serial]
    fn test_ffi_state_queries() {
        reset_state();
        rust_InitCommunication();

        assert_eq!(rust_IsTalkingFinished(), 0);
        rust_SetTalkingFinished(1);
        assert_eq!(rust_IsTalkingFinished(), 1);

        rust_SetCommIntroMode(2);
        assert_eq!(rust_GetCommIntroMode(), 2);

        rust_SetCommFadeTime(30);
        assert_eq!(rust_GetCommFadeTime(), 30);

        assert_eq!(rust_IsCommInputPaused(), 0);
        rust_SetCommInputPaused(1);
        assert_eq!(rust_IsCommInputPaused(), 1);

        rust_UninitCommunication();
    }

    #[test]
    #[serial]
    fn test_ffi_track_text_only() {
        reset_state();
        rust_InitCommunication();

        let text = CString::new("Subtitle only").unwrap();
        rust_SpliceTrackText(text.as_ptr(), 0.0, 2.0);

        // Track should have a chunk
        assert!(rust_GetTrackLength() > 0.0);

        rust_UninitCommunication();
    }

    #[test]
    #[serial]
    fn test_ffi_track_position() {
        reset_state();
        rust_InitCommunication();

        let text = CString::new("Test").unwrap();
        rust_SpliceTrack(1, text.as_ptr(), 0.0, 5.0);

        rust_StartTrack();
        rust_SeekTrack(2.5);
        assert!((rust_GetTrackPosition() - 2.5).abs() < 0.01);

        rust_JumpTrack(1.0);
        assert!((rust_GetTrackPosition() - 3.5).abs() < 0.01);

        rust_RewindTrack();
        assert!((rust_GetTrackPosition() - 0.0).abs() < 0.01);

        rust_UninitCommunication();
    }

    #[test]
    #[serial]
    fn test_ffi_update() {
        reset_state();
        rust_InitCommunication();

        let text = CString::new("Test").unwrap();
        rust_SpliceTrack(1, text.as_ptr(), 0.0, 1.0);

        rust_StartTrack();
        rust_UpdateCommunication(0.5);
        assert!((rust_GetTrackPosition() - 0.5).abs() < 0.01);

        rust_UpdateCommunication(1.0);
        assert_eq!(rust_IsTalkingFinished(), 1);

        rust_UninitCommunication();
    }

    #[test]
    #[serial]
    fn test_ffi_null_safety() {
        reset_state();
        rust_InitCommunication();

        // Should handle null pointers gracefully
        rust_SpliceTrack(1, std::ptr::null(), 0.0, 1.0);
        rust_SpliceTrackText(std::ptr::null(), 0.0, 1.0);
        assert_eq!(rust_DoResponsePhrase(1, std::ptr::null(), None), 0);
        rust_AddOscilloscopeSamples(std::ptr::null(), 0);

        rust_UninitCommunication();
    }
}
