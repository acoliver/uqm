# Video System Implementation Summary

## Files Modified

### Core Video Implementation Files
- `/Users/acoliver/projects/uqm/rust/src/video/ffi.rs` - Added `rust_present_video_to_window()` and `rust_play_video_direct_window()` functions
- `/Users/acoliver/projects/uqm/rust/src/video/scaler.rs` - Added `LanczosVideoScaler` for window-aware scaling with aspect preservation
- `/Users/acolover/projects/uqm/rust/src/video/player.rs` - Added direct window presentation mode and Lanczos scaler support
- `/Users/acoliver/projects/uqm/sc2/src/libs/video/rust_video.c` - Updated C wrapper to use direct window presentation path automatically

## Changes Made

### 1. Direct Window Presentation Function
- Added `rust_present_video_to_window()` FFI function that renders RGBA video frames directly to SDL window
- Bypasses the 320x240 SDL_Screen surface completely
- UsesSDL2 renderer for proper scaling and presentation
- Maintains aspect ratio with letterboxing/pillarboxing

### 2. Window-Aware Lanczos Scaler
- Created `LanczosVideoScaler` that scales video frames to actual window dimensions
- Preserves aspect ratio while maximizing usage of window space
- Uses fast_image_resize crate for SIMD-accelerated Lanczos3 interpolation

### 3. VideoPlayer Enhancements
- Added `direct_window_mode` flag (enabled by default)
- Added `lanczos_scaler` support for window-aware scaling
- Modified `process_frame()` to route frames directly to window when in direct mode
- Added fallback handling if Lanczos scaling fails

### 4. C Wrapper Updates
- Modified `TFB_PlayVideo()` to detect actual window size via `ScreenWidthActual`/`ScreenHeightActual`
- Automatically chooses direct window presentation when window > 320x240
- Added `rust_play_video_direct_window()` extern declaration
- Preserves existing C VIDPLAYER API compatibility

### 5. xBRZ/hq2x Bypass
- Video frames in direct window mode completely bypass the graphics scaler pipeline
- No xBRZ or hq2x scaling is applied to video frames
- Maintains video quality by preventing scaling artifacts from game scalers

## How Video is Now Presented/Scaled

1. **Legacy Path (320x240 windows)**:
   - Video frames are decoded to RGBA
   - Scaled with existing VideoScaler if configured
   - Blitted to the 320x240 SDL_Screen surface
   - Goes through normal graphics pipeline (potential xBRZ/hq2x)

2. **Direct Window Path (actual window size > 320x240)**:
   - Video frames are decoded to RGBA
   - Scaled with `LanczosVideoScaler` to actual window dimensions
   - Presented directly via `rust_present_video_to_window()` to SDL renderer
   - Bypasses all graphics scalers completely

3. **Aspect Ratio Preservation**:
   - Direct window mode maintains video aspect ratio
   - Automatic letterboxing (black bars) if video/window aspect differ
   - Centered presentation within window

## Logging Added

Video presentation now logs to `rust-bridge.log`:
- Frame dimensions and destination size
- Success/failure status for each presentation
- When bypassing xBRZ/hq2x scalers
- Window detection and path selection

## Follow-up Items

1. **Testing**: Verify video playback at various window sizes
2. **Performance**: Monitor CPU usage with Lanczos scaling vs. previous path
3. **Fallback**: Ensure seamless fallback when window < 320x240
4. **Integration**: Confirm no conflicts with existing graphics API

## Compatibility Notes

- **C API**: Existing `TFB_PlayVideo()` API unchanged for external callers
- **Configuration**: Direct window mode is automatic based on window size detection
- **Scalers**: Video uses Lanczos, game continues to use xBRZ/hq2x for sprites
- **Memory**: No additional memory usage beyond frame buffers
- **Thread-safety**: All modifications maintain existing thread-safety guarantees

## Assumptions

- `ScreenWidthActual`/`ScreenHeightActual` variables are correctly maintained by the graphics subsystem
- SDL	renderer is already initialized by the time video playback begins
- Video decoder maintains RGBA format with R component in bits 0-7