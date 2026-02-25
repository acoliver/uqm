# Phase 21: Integration

## Phase ID
`PLAN-20260225-AUDIO-HEART.P21`

## Prerequisites
- Required: Phase P20a (FFI Implementation Verification) passed
- Expected: All 7 Rust modules fully implemented, all tests passing, all FFI symbols exported

## Requirements Implemented (Expanded)

### REQ-CROSS-GENERAL-06: Build Flag Integration
**Requirement text**: The USE_RUST_AUDIO_HEART build flag shall conditionally include Rust audio heart modules or C equivalents.

Behavior contract:
- GIVEN: Both Rust and C implementations exist
- WHEN: `USE_RUST_AUDIO_HEART` is defined
- THEN: Rust FFI functions are linked instead of C functions; C files excluded from compilation
- WHEN: `USE_RUST_AUDIO_HEART` is not defined
- THEN: C files compile and link as before (backward compatible)

Why it matters:
- Allows gradual rollout and easy rollback if issues found

### REQ-CROSS-GENERAL-02: Log Crate Integration
**Requirement text**: All diagnostic output via log crate.

Behavior contract:
- GIVEN: Rust code uses log::warn!, log::error!, etc.
- WHEN: Integrated into C build
- THEN: Log output appears in console/logfile

### REQ-CROSS-GENERAL-07: Module Registration
**Requirement text**: All modules registered in sound::mod.rs.

### REQ-CROSS-THREAD-01, REQ-CROSS-THREAD-02, REQ-CROSS-THREAD-03, REQ-CROSS-THREAD-04: Threading Integration
**Requirement text**: Decoder thread runs correctly when called from C game loop.

## Integration Contract

### Existing Callers (C code that calls audio functions)
- `sc2/src/libs/sound/sound.c` → `InitSound()`, `UninitSound()`, `StopSound()`, `SoundPlaying()`
- `sc2/src/libs/sound/stream.c` → `InitStreamDecoder()`, `UninitStreamDecoder()`, `PlayStream()`, etc.
- `sc2/src/libs/sound/music.c` → `PLRPlaySong()`, `PLRStop()`, `FadeMusic()`, etc.
- `sc2/src/libs/sound/sfx.c` → `PlayChannel()`, `StopChannel()`, etc.
- `sc2/src/libs/sound/trackplayer.c` → `SpliceTrack()`, `PlayTrack()`, `GetTrackSubtitle()`, etc.
- `sc2/src/libs/sound/fileinst.c` → `LoadSoundFile()`, `LoadMusicFile()`, etc.
- Various game code files (comm screens, melee, starmap, etc.)

### Existing Code Replaced/Removed (conditional)
Under `USE_RUST_AUDIO_HEART`:
- `sc2/src/libs/sound/stream.c` — excluded from build
- `sc2/src/libs/sound/trackplayer.c` — excluded from build
- `sc2/src/libs/sound/music.c` — excluded from build
- `sc2/src/libs/sound/sfx.c` — excluded from build
- `sc2/src/libs/sound/sound.c` — excluded from build
- `sc2/src/libs/sound/fileinst.c` — excluded from build

### User Access Path
- Launch game with `USE_RUST_AUDIO_HEART=1` build flag
- All audio (music, SFX, speech, comm screen oscilloscope) driven by Rust

### Data/State Migration
- No persistent state migration needed (all audio state is runtime-only)
- GetTimeCounter() and QuitPointed() FFI still points to C implementation

### End-to-End Verification
- `cargo test --lib --all-features` (all Rust tests)
- `./build.sh uqm` (C+Rust build succeeds)
- Manual: game launches, music plays, SFX fires, speech renders with subtitles, oscilloscope animates

## Implementation Tasks

### Files to create
- `sc2/src/libs/sound/audio_heart_rust.h` — C header declaring all Rust FFI function prototypes
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P21`
  - marker: `@requirement REQ-CROSS-GENERAL-06`

### Files to modify

#### Build system
- `sc2/src/config_unix.h` (or equivalent config header)
  - Add: `/* #define USE_RUST_AUDIO_HEART 1 */` (commented out by default)
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P21`

#### C header
- `sc2/src/libs/sound/audio_heart_rust.h`
  - Declare all 60+ Rust FFI function prototypes
  - **Must include standard include guards:**
    ```c
    #ifndef AUDIO_HEART_RUST_H_
    #define AUDIO_HEART_RUST_H_
    #ifdef __cplusplus
    extern "C" {
    #endif
    // ... all declarations ...
    #ifdef __cplusplus
    }
    #endif
    #endif /* AUDIO_HEART_RUST_H_ */
    ```
  - Include type definitions for `TFB_SoundSample`, `TFB_SoundTag`, `TFB_SoundCallbacks`, etc.

#### Build file modifications
- `sc2/Makefile` or `sc2/build/unix/build_functions` (or equivalent)
  - **IMPORTANT:** Verify the actual build system structure during implementation. The UQM build system uses `build.sh` → `build_functions` → Makefiles; the exact Makefile path and variable names (`SOUND_SRCS`) are speculative and must be confirmed by reading the actual build files before modifying them.
  - **MANDATORY FIRST STEP:** Before modifying any build file, run:
    ```bash
    # 1. Find the actual build system files that reference sound .c files
    grep -rn "stream\.c\|trackplayer\.c\|music\.c\|sfx\.c\|sound\.c\|fileinst\.c" sc2/build/ sc2/Makefile* sc2/CMakeLists* 2>/dev/null
    # 2. Identify the exact variable names and build file structure
    grep -rn "SOUND\|AUDIO\|SRC.*sound" sc2/build/ sc2/Makefile* 2>/dev/null
    # 3. Verify the Rust static library link flags are already present
    grep -rn "rust\|cargo\|libuqm_rust" sc2/build/ sc2/Makefile* 2>/dev/null
    ```
    Record the results and adapt the conditional compilation to the actual build system. Do NOT assume `SOUND_SRCS` or `filter-out` will work — the build system may use file lists, CMake, or another mechanism.
  - **DUPLICATE SYMBOL PREVENTION:** When `USE_RUST_AUDIO_HEART` is defined, the 6 C files MUST be excluded from compilation AND the Rust static library MUST be linked. If both the C files and the Rust library are linked, duplicate symbol errors will occur for all 60+ audio function names. Verify with: `nm -g librust_uqm.a | grep -E "PLRPlaySong|PlayChannel|InitSound"` to confirm symbol export.
  - Add conditional: when `USE_RUST_AUDIO_HEART` is defined, exclude the 6 C files from the sound source list
  - Example (adapt variable names to actual build system):
    ```makefile
    ifdef USE_RUST_AUDIO_HEART
      SOUND_SRCS := $(filter-out stream.c trackplayer.c music.c sfx.c sound.c fileinst.c, $(SOUND_SRCS))
    endif
    ```

#### Sound header guards
- `sc2/src/libs/sound/stream.h` — Add `#ifndef USE_RUST_AUDIO_HEART` guard around C prototypes, with `#else #include "audio_heart_rust.h"` for Rust prototypes
- `sc2/src/libs/sound/trackplayer.h` — Same pattern
- `sc2/src/libs/sound/music.h` — Same pattern
- `sc2/src/libs/sound/sfx.h` — Same pattern
- `sc2/src/libs/sound/sound.h` — Same pattern
- `sc2/src/libs/sound/fileinst.h` — Same pattern

### Conditional compilation pattern

Each of the 6 sound headers needs `#ifdef USE_RUST_AUDIO_HEART` guards around the C function declarations. This ensures that when the Rust implementation is enabled, C code sees the Rust FFI prototypes (from `audio_heart_rust.h`) instead of the C prototypes. When the flag is not set, the existing C declarations remain untouched.

**All 6 headers requiring guards:**

1. **`music.h`** — Guards around: `PLRPlaySong`, `PLRStop`, `PLRPlaying`, `PLRSeek`, `PLRPause`, `PLRResume`, `snd_PlaySpeech`, `snd_StopSpeech`, `SetMusicVolume`, `FadeMusic`, `CheckMusicResName`, `DestroyMusic`, `GetMusicData`, `ReleaseMusicData`
2. **`sfx.h`** — Guards around: `PlayChannel`, `StopChannel`, `ChannelPlaying`, `SetChannelVolume`, `CheckFinishedChannels`, `UpdateSoundPosition`, `GetPositionalObject`, `SetPositionalObject`, `DestroySound`, `GetSoundBankData`, `ReleaseSoundBankData`
3. **`sound.h`** — Guards around: `InitSound`, `UninitSound`, `StopSource`, `CleanSource`, `StopSound`, `SoundPlaying`, `WaitForSoundEnd`, `SetSFXVolume`, `SetSpeechVolume`
4. **`stream.h`** — Guards around: `InitStreamDecoder`, `UninitStreamDecoder`, `PlayStream`, `StopStream`, `PauseStream`, `ResumeStream`, `SeekStream`, `PlayingStream`, `SetMusicStreamFade`, `GraphForegroundStream`, `TFB_CreateSoundSample`, `TFB_DestroySoundSample`, etc.
5. **`trackplayer.h`** — Guards around: `SpliceTrack`, `SpliceMultiTrack`, `PlayTrack`, `StopTrack`, `JumpTrack`, `PauseTrack`, `ResumeTrack`, `PlayingTrack`, `GetTrackPosition`, `GetTrackSubtitle`, etc.
6. **`fileinst.h`** — Guards around: `LoadSoundFile`, `LoadMusicFile`

**Pattern for each header:**

```c
/* In each sound header, e.g., music.h */
#ifndef USE_RUST_AUDIO_HEART

/* Existing C function declarations */
void PLRPlaySong(MUSIC_REF MusicRef, BOOLEAN Continuous, BYTE Priority);
void PLRStop(MUSIC_REF MusicRef);
BOOLEAN PLRPlaying(MUSIC_REF MusicRef);
/* ... all other C prototypes for this header ... */

#else /* USE_RUST_AUDIO_HEART */

/* Rust FFI declarations — identical signatures, implemented in Rust */
#include "audio_heart_rust.h"

#endif /* USE_RUST_AUDIO_HEART */
```

**Important**: The `#ifdef` guard must wrap ALL function declarations that are replaced by Rust, not just some. Partial guards would cause duplicate symbol errors at link time. The `audio_heart_rust.h` header provides all prototypes in one place, so the `#else` branch is simply an include.

### Rust module registration verification
- `rust/src/sound/mod.rs` must declare all 7 new modules:
  ```rust
  pub mod types;
  pub mod stream;
  pub mod trackplayer;
  pub mod music;
  pub mod sfx;
  pub mod control;
  pub mod fileinst;
  pub mod heart_ffi;
  ```

## Verification Commands

```bash
# Rust verification
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings

# C build verification (without Rust heart — regression check)
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm

# C build verification (with Rust heart)
cd /Users/acoliver/projects/uqm/sc2 && USE_RUST_AUDIO_HEART=1 ./build.sh uqm

# Symbol verification
nm rust/target/debug/libuqm_rust.a | grep -c "T.*InitStreamDecoder\|T.*PLRPlaySong\|T.*PlayChannel"
```

## End-to-End Manual Verification

### Step 1: Build with flag
```bash
cd /Users/acoliver/projects/uqm/sc2
USE_RUST_AUDIO_HEART=1 ./build.sh uqm
```
Expected: Build succeeds, no link errors for missing audio symbols.

### Step 2: Launch game
```bash
./uqm
```
Expected: Game launches to title screen.

### Step 3: Music playback
- Navigate to title screen → background music should play
- Enter menus → menu music should play
- Start new game → intro music should play

### Step 4: SFX playback
- In melee/combat → weapon sounds should fire
- In menus → navigation click sounds

### Step 5: Speech/track player
- Enter communication screen (any alien dialogue)
- Speech audio plays
- Subtitles appear and scroll
- Oscilloscope waveform animates

### Step 6: Volume control
- Adjust music volume in options → volume changes
- Adjust SFX volume in options → volume changes
- Adjust speech volume in options → volume changes

### Step 7: Fade
- When transitioning between screens, music should fade

### Step 8: Seeking
- In communication screen, use seek controls → speech seeks correctly

### Step 9: Build without flag (regression)
```bash
cd /Users/acoliver/projects/uqm/sc2
./build.sh uqm
```
Expected: Build succeeds with C audio (no Rust). Game works identically.

## Structural Verification Checklist
- [ ] `audio_heart_rust.h` created with all FFI prototypes
- [ ] `config_unix.h` has USE_RUST_AUDIO_HEART define (commented out)
- [ ] Build system conditionally excludes C files
- [ ] Sound headers have `#ifndef USE_RUST_AUDIO_HEART` guards
- [ ] `mod.rs` declares all 7 modules
- [ ] Build succeeds with flag
- [ ] Build succeeds without flag (regression)

## Semantic Verification Checklist (Mandatory)
- [ ] Music plays from Rust path
- [ ] SFX plays from Rust path
- [ ] Speech plays with subtitles from Rust path
- [ ] Oscilloscope renders from Rust path
- [ ] Volume controls work from Rust path
- [ ] Fade works from Rust path
- [ ] No audio regressions when flag is off
- [ ] No link errors in either build mode

## Deferred Implementation Detection (Mandatory)

```bash
# Check all Rust sound modules
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" \
  rust/src/sound/types.rs \
  rust/src/sound/stream.rs \
  rust/src/sound/trackplayer.rs \
  rust/src/sound/music.rs \
  rust/src/sound/sfx.rs \
  rust/src/sound/control.rs \
  rust/src/sound/fileinst.rs \
  rust/src/sound/heart_ffi.rs
# Must return 0 results
```

## Success Criteria
- [ ] Build succeeds with USE_RUST_AUDIO_HEART
- [ ] Build succeeds without USE_RUST_AUDIO_HEART
- [ ] All Rust tests pass
- [ ] Manual verification of all 9 steps passes
- [ ] Zero deferred implementations

## Failure Recovery
- rollback: Remove USE_RUST_AUDIO_HEART define, restore original headers
- `git checkout -- sc2/src/libs/sound/*.h sc2/src/config_unix.h`
- blocking issues: If link errors occur, check symbol names match between C header and Rust `#[no_mangle]` exports

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P21.md`

Contents:
- phase ID: P21
- timestamp
- files changed: audio_heart_rust.h, config_unix.h, sound headers, build files, mod.rs
- tests: all Rust tests (130+ across all modules)
- verification outputs: cargo test, cargo clippy, build.sh results
- semantic verification: manual testing summary for all 9 steps
- explicit pass/fail: PASS if all checks green
