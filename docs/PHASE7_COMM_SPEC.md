# Phase 7: Alien Communication System Port to Rust

## Overview
Port `sc2/src/uqm/comm.c` (1649 lines) to Rust. The comm system handles alien dialogue, response selection, animation synchronization, and speech/subtitle playback.

## C Source Files
- `sc2/src/uqm/comm.c` - Main communication logic
- `sc2/src/uqm/comm.h` - Public interface
- `sc2/src/uqm/commanim.c` - Animation handling
- `sc2/src/uqm/commglue.c` - Glue code for alien-specific logic

## Key Data Structures

### ResponseEntry
```rust
pub struct ResponseEntry {
    pub response_ref: u32,           // Reference ID
    pub response_text: String,       // Display text
    pub response_func: Option<fn()>, // Callback function
}
```

### CommData (LOCDATA equivalent)
```rust
pub struct CommData {
    pub init_encounter_func: Option<fn()>,
    pub post_encounter_func: Option<fn()>,
    pub uninit_encounter_func: Option<fn()>,
    
    pub alien_frame: Option<Frame>,
    pub alien_font: Option<Font>,
    pub alien_color_map: Option<ColorMap>,
    
    pub alien_speech: Option<SoundSample>,
    pub num_animations: u32,
    pub anim_desc: Vec<AnimDesc>,
    
    pub ambient_flags: u32,
    pub transition_time: u32,
}
```

### SoundChunk (for speech/music)
```rust
pub struct SoundChunk {
    pub decoder: Box<dyn SoundDecoder>,
    pub start_time: f32,
    pub tag: Option<String>,      // Subtitle tag
    pub subtitle: Option<String>, // Actual subtitle text
    pub next: Option<Box<SoundChunk>>,
}
```

### CommState
```rust
pub struct CommState {
    pub track_count: i32,
    pub no_page_break: bool,
    pub sound_sample: Option<SoundSample>,
    pub tracks_length: u32,
    
    pub chunks_head: Option<Box<SoundChunk>>,
    pub chunks_tail: Option<*mut SoundChunk>,
    pub last_sub: Option<*mut SoundChunk>,
    pub cur_chunk: Option<*mut SoundChunk>,
    pub cur_sub_chunk: Option<*mut SoundChunk>,
    
    pub responses: Vec<ResponseEntry>,
    pub selected_response: i32,
    
    pub talking_finished: bool,
    pub intro_mode: CommIntroMode,
    pub fade_time: u32,
}
```

### CommIntroMode
```rust
pub enum CommIntroMode {
    Default,
    FadeIn,
    CrossFade,
    Immediate,
}
```

## Core Functions to Implement

### Initialization
- `InitCommunication()` - Start communication screen
- `UninitCommunication()` - Cleanup communication
- `HailAlien()` - Begin conversation

### Track Management
- `StartTrack()` - Start playing speech track
- `StopTrack()` - Stop current track
- `JumpTrack(offset)` - Jump within track
- `CommitTrack()` - Commit current track position
- `RewindTrack()` - Rewind to beginning

### Speech/Subtitle Handling
- `SpliceTrack(sample, text, start_time)` - Add speech chunk
- `SpliceTrackText(text, start_time)` - Add text-only chunk
- `GetSubtitle() -> &str` - Get current subtitle
- `WaitTrack() -> bool` - Wait for track to finish

### Response System
- `DoResponsePhrase(ref, text, func)` - Add response option
- `DisplayResponses()` - Show response choices
- `SelectResponse(index)` - Handle selection
- `ClearResponses()` - Clear response list

### Animation
- `UpdateCommAnimations()` - Update alien animations
- `StartCommAnimation(anim_id)` - Start specific animation
- `StopCommAnimation(anim_id)` - Stop specific animation

### Oscilloscope
- `UpdateOscilloscope()` - Update audio waveform display
- `DrawOscilloscope(context, rect)` - Render waveform

### Callbacks (for stream player)
- `OnStreamStart(sample) -> bool`
- `OnChunkEnd(sample, buffer) -> bool`
- `OnStreamEnd(sample)`
- `OnBufferTag(sample, tag)`

## Conversation Flow
```
1. HailAlien() called
2. InitCommunication() sets up screen
3. Alien-specific init function called
4. Loop:
   a. Play alien speech (track)
   b. Display subtitle
   c. Show response options
   d. Wait for player selection
   e. Call response function
   f. Response function may queue more speech
5. UninitCommunication() cleanup
```

## Subtitle Timing
```rust
struct SubtitleTiming {
    start_time: f32,
    end_time: f32,
    text: String,
}

fn get_current_subtitle(chunks: &[SoundChunk], current_time: f32) -> Option<&str> {
    for chunk in chunks {
        if current_time >= chunk.start_time 
           && current_time < chunk.start_time + chunk.decoder.length() {
            return chunk.subtitle.as_deref();
        }
    }
    None
}
```

## Animation Synchronization
The C code runs animations at 40 FPS (COMM_ANIM_RATE = ONE_SECOND / 40).
```rust
const COMM_ANIM_RATE: Duration = Duration::from_millis(25);  // 40 FPS

fn update_animations(anim_context: &mut AnimContext) {
    for anim in &mut anim_context.animations {
        if anim.running {
            anim.frame_time += delta_time;
            if anim.frame_time >= anim.frame_duration {
                anim.frame_time = Duration::ZERO;
                anim.current_frame = (anim.current_frame + 1) % anim.frame_count;
            }
        }
    }
}
```

## Thread Safety
The comm system is mostly single-threaded but stream callbacks come from the audio thread. Use `parking_lot::Mutex` for shared state.

## Test Plan (TDD)

### Unit Tests
1. `test_comm_init_uninit` - Initialize and cleanup
2. `test_add_response` - Add response entry
3. `test_clear_responses` - Clear all responses
4. `test_select_response` - Select response
5. `test_splice_track` - Add speech chunk
6. `test_subtitle_timing` - Get subtitle at time
7. `test_track_navigation` - Jump/rewind track
8. `test_oscilloscope_update` - Waveform calculation
9. `test_animation_frame_advance` - Animation timing
10. `test_fade_timing` - Fade in/out timing
11. `test_response_callback` - Response function invocation
12. `test_chunk_linking` - Linked list operations
13. `test_intro_modes` - Different intro modes
14. `test_empty_track` - Handle empty tracks
15. `test_multiple_responses` - Multiple response options

### Integration Tests
1. `test_comm_with_audio` - Full audio playback
2. `test_comm_with_animation` - Animation synchronization
3. `test_comm_with_subtitles` - Subtitle display
4. `test_full_conversation` - Complete conversation flow

## File Structure
```
rust/src/comm/
├── mod.rs              (public exports)
├── types.rs            (ResponseEntry, CommData, etc.)
├── state.rs            (CommState management)
├── track.rs            (audio track handling)
├── subtitle.rs         (subtitle timing)
├── response.rs         (response system)
├── animation.rs        (animation handling)
├── oscilloscope.rs     (waveform display)
└── ffi.rs              (C FFI bindings)
```

## FFI Functions to Export
```rust
#[no_mangle]
pub extern "C" fn rust_InitCommunication() -> c_int;

#[no_mangle]
pub extern "C" fn rust_UninitCommunication();

#[no_mangle]
pub extern "C" fn rust_DoResponsePhrase(
    ref_: c_uint,
    text: *const c_char,
    func: extern "C" fn()
);

#[no_mangle]
pub extern "C" fn rust_StartTrack();

#[no_mangle]
pub extern "C" fn rust_StopTrack();
// ... etc
```

## Dependencies
- Existing sound decoder system
- Existing mixer (from Phase 4)
- Graphics system for rendering
- `parking_lot` for synchronization

## Acceptance Criteria
1. All unit tests pass
2. Alien speech plays correctly
3. Subtitles sync with speech
4. Response selection works
5. Animations run at correct rate
6. Oscilloscope displays waveform
7. Fade transitions work
8. FFI bindings work with C code
9. No audio/visual desync
