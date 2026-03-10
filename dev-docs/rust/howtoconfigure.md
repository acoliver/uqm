# How Configuration and the Rust/C Boundary Work

> **Audience:** A fresh LLM entering a new context to help with the UQM Rust port.
> This doc explains the coexistence model, boundary conventions, and configuration
> system. Read `howtorun.md` first for build/run basics.

---

## The Core Coexistence Model

Every ported subsystem has a C preprocessor define (`USE_RUST_*`). When set:

1. The C file's body compiles to nothing (via `#ifndef` / `#ifdef` guards).
2. The linker resolves those symbols from the Rust static library (`libuqm_rust.a`).
3. Rust exposes `#[no_mangle] pub extern "C" fn` entry points that C calls directly.

**C always owns `main()`.** The binary is `sc2/uqm`. Rust is a statically linked
library, never the executable. (`rust/src/main.rs` exists but is dead code — see
`howtorun.md`.)

---

## Two Toggle Layers

### Layer 1: Rust Bridge (All-or-Nothing)

`./build.sh uqm config` -> "Rust bridge -> enabled" sets **all** subsystem
defines at once. There is no per-subsystem build menu. The single line in
`build.config` (~line 472):

```sh
CCOMMONFLAGS="$CCOMMONFLAGS -DUSE_RUST_BRIDGE -DUSE_RUST_FILE -DUSE_RUST_CLOCK \
  -DUSE_RUST_UIO -DUSE_RUST_OGG -DUSE_RUST_AUDIO -DUSE_RUST_COMM \
  -DUSE_RUST_INPUT -DUSE_RUST_VIDEO -DUSE_RUST_VIDPLAYER -DUSE_RUST_GFX \
  -DUSE_RUST_RESOURCE -DUSE_RUST_MOD -DUSE_RUST_WAV -DUSE_RUST_THREADS \
  -DUSE_RUST_MIXER -DUSE_RUST_MEM"
```

Also adds `-L.../rust/target/release -luqm_rust` and macOS frameworks
(CoreAudio, AudioToolbox, CoreFoundation).

### Layer 2: Audio-Heart (Opt-In Extra)

`USE_RUST_AUDIO_HEART` is the **only** independently toggled define:

```sh
USE_RUST_AUDIO_HEART=1 ./build.sh uqm
```

On the Cargo side, this maps to `--features audio_heart`, gating `heart_ffi.rs`.

### Defines Not Yet Wired into the Build Menu

| Define | C File | Status |
|---|---|---|
| `USE_RUST_DUKAUD` | `decoder.c` | Rust DukAud decoder — guards exist, not set by build menu |
| `USE_RUST_AIFF` | `decoder.c` | Rust AIFF decoder — guards exist, not set by build menu |
| `USE_RUST_STATE` | `uqm/state.c` | Rust game-state bitfield — guards exist, not set by build menu |

To test these, manually add `-DUSE_RUST_FOO` to CCOMMONFLAGS in `build.config`.

---

## Complete Subsystem Map

| C Define | Rust Module(s) | C Code Replaced | Plan Directory |
|---|---|---|---|
| `USE_RUST_BRIDGE` | `bridge_log`, `c_bindings` | Bridge init, logging | — |
| `USE_RUST_FILE` | `io::{dirs,files,temp}` | `files.c`, `dirs.c`, `temp.c` | `timeandfile/` |
| `USE_RUST_CLOCK` | `time::clock_bridge` | Game clock, date arithmetic | `timeandfile/` |
| `USE_RUST_UIO` | `io::uio_bridge` | `uio_open`/`uio_read`/`uio_close`/`uio_fstat` | `timeandfile/` |
| `USE_RUST_OGG` | `sound::ogg` | OGG Vorbis decoder | `audiorust/decoder/` |
| `USE_RUST_WAV` | `sound::wav` | WAV decoder | `audiorust/decoder/` |
| `USE_RUST_MOD` | `sound::mod_decoder` | MOD/tracker decoder | `audiorust/decoder/` |
| `USE_RUST_AUDIO` | `sound::mixer`, `audiocore_rust.c` | Audio backend (cpal/rodio) | `sound/` |
| `USE_RUST_MIXER` | `sound::mixer::*` | Mixer buffer/source ops | `sound/` |
| `USE_RUST_THREADS` | `threading` | Thread, mutex, semaphore, condvar | `vid_thread_resource/` |
| `USE_RUST_MEM` | `memory` | `HMalloc`/`HFree`/`HCalloc`/`HRealloc` | `memandres/memory/` |
| `USE_RUST_COMM` | `comm` | Alien dialogue, animation, subtitles, oscilloscope | — |
| `USE_RUST_INPUT` | `input` | VControl, keyboard/joystick mapping | — |
| `USE_RUST_VIDEO` | `video::decoder` | DukVid (.duk) video decoder | `vid_thread_resource/` |
| `USE_RUST_VIDPLAYER` | `video::player` | Video playback integration | `vid_thread_resource/` |
| `USE_RUST_GFX` | `graphics` | Drawable, frame, pixmap, font, DCQ, scaling, render | `gfx/` |
| `USE_RUST_RESOURCE` | `resource` | Resource index, loader, cache, string bank, prop files | `memandres/resource/` |
| `USE_RUST_AUDIO_HEART` | `heart_ffi`, `stream`, `trackplayer`, `music`, `sfx` | `stream.c`, `trackplayer.c`, `music.c`, `sfx.c`, `sound.c`, `fileinst.c` | `audiorust/heart/` |

---

## Where Configuration Lives

| File | Role | Hand-edit? |
|---|---|---|
| `sc2/build/unix/build.config` | Build menu, `rust_bridge_enabled_action()` | **Yes** — source of truth |
| `sc2/config_unix.h.in` | Template with `@SYMBOL_USE_RUST_*_DEF@` placeholders | **Yes** — add new defines here |
| `sc2/config_unix.h` | **Generated** C header — `#define` or `/* #undef */` per flag | No |
| `sc2/build.vars` | **Generated** build variables — compiler/linker flags | No |
| `rust/Cargo.toml` | `[features]` section — currently only `audio_heart` | **Yes** |

### Sync Mechanism: `uqm_pre_build()` (~line 700 in build.config)

Before compiling C, the build system runs Cargo with the correct features:

```sh
uqm_pre_build() {
    if [ "$USE_RUST_BRIDGE" = "1" ]; then
        RUST_FEATURES=""
        if printf "%s" "$TARGET_CFLAGS" | grep -q -- '-DUSE_RUST_AUDIO_HEART'; then
            RUST_FEATURES="--features audio_heart"
        fi
        (cd "$TOPDIR/../rust" && cargo build --release $RUST_FEATURES)
    fi
}
```

This keeps the C defines and Cargo features in sync automatically.

---

## C Guard Patterns — Know Which One Before Editing

### Pattern 1: `#ifndef` — Entire C File Excluded When Rust Active

```c
// stream.c, trackplayer.c, music.c, sfx.c, fileinst.c, sound.c
#ifndef USE_RUST_AUDIO_HEART
// ... entire C implementation ...
#endif
```

Same pattern: `thrcommon.c` (`USE_RUST_THREADS`), `getres.c`/`resinit.c`
(`USE_RUST_RESOURCE`), `comm.c` (`USE_RUST_COMM`), `vidplayer.c`
(`USE_RUST_VIDEO`).

### Pattern 2: `#ifdef` — C File Is a Shim That Calls Rust

```c
// rust_thrcommon.c
#ifdef USE_RUST_THREADS
extern int rust_init_thread_system(void);
// ... wrappers that call rust_* functions ...
#endif
```

Same pattern: `audiocore_rust.c`, `rust_resource.c`, `rust_vcontrol_impl.c`,
`rust_video.c`, `rust_comm.c`, `clock_rust.c`.

### Pattern 3: Inline Switching Within One File

```c
// decoder.c — per-format switching
#ifdef USE_RUST_OGG
extern TFB_DecoderVtbl rust_ova_DecoderVtbl;
#endif
...
#ifdef USE_RUST_OGG
  { "ogg", &rust_ova_DecoderVtbl },
#else
  { "ogg", &ogg_DecoderVtbl },
#endif
```

Also: `sdl_common.c` (`USE_RUST_GFX`), `mixer.c` (`USE_RUST_MIXER`),
`w_memlib.c` (`USE_RUST_MEM`), `files.c` (`USE_RUST_FILE`).

---

## FFI Naming Convention

| Subsystem | FFI Prefix | Example |
|---|---|---|
| Mixer | `rust_mixer_*` | `rust_mixer_Init`, `rust_mixer_GenSources` |
| Threads | `rust_thread_*`, `rust_mutex_*`, `rust_semaphore_*`, `rust_condvar_*` | `rust_thread_spawn` |
| Memory | `rust_hmalloc`, `rust_hfree`, `rust_hcalloc`, `rust_hrealloc` | — |
| Resource | `rust_resource_*`, `rust_cache_*` | `rust_resource_load` |
| Input | `rust_VControl_*` | `rust_VControl_HandleEvent` |
| Comm | `rust_*Communication` | `rust_InitCommunication` |
| Clock | `rust_clock_*` | `rust_clock_init`, `rust_clock_tick` |
| Video | `rust_play_video`, `rust_stop_video` | — |
| Bridge | `rust_bridge_init`, `rust_bridge_log_msg` | — |
| Decoders | `rust_{ova,wav,mod,duka,aiff}_DecoderVtbl` | Static vtable exports |
| Audio-heart | Uses **original C function names** | `PLRPlaySong`, `InitSound`, `LoadMusicFile` |

Audio-heart is unique: its FFI functions keep the **original C names** so
calling code doesn't change. All other subsystems use `rust_*` prefixed names
and require a C shim file.

---

## The `audiocore_rust.c` Bridge

When `USE_RUST_AUDIO` is defined, `sc2/src/libs/sound/audiocore_rust.c` provides
the `audio_*` API by calling Rust `rust_mixer_*` FFI functions. It translates:

- `audio_Object` (C `uintptr_t`) <-> `mixer_Object` (Rust `intptr_t`)
- `audio_*` enum values -> `MIX_*` constants via an `EnumLookup[]` table

Active in both mixer-only and full audio-heart modes.

---

## Dependency Diagram

```
                       USE_RUST_BRIDGE
                            |
          +-----------------+-----------------------------------+
          |                 |                                   |
    USE_RUST_FILE     USE_RUST_AUDIO                     USE_RUST_THREADS
    USE_RUST_CLOCK    USE_RUST_MIXER                     USE_RUST_MEM
    USE_RUST_UIO      USE_RUST_OGG                       USE_RUST_INPUT
    USE_RUST_VIDEO    USE_RUST_MOD                       USE_RUST_COMM
    USE_RUST_GFX      USE_RUST_WAV                       USE_RUST_RESOURCE
    USE_RUST_VIDPLAYER                                   (USE_RUST_DUKAUD)*
                                                         (USE_RUST_AIFF)*
                                                         (USE_RUST_STATE)*
                            |
                            |  opt-in extra
                            v
                   USE_RUST_AUDIO_HEART
                   Cargo feature: audio_heart

    * = guards exist in C code but not yet wired into build menu
```

All flags in the top group are set/cleared together. Audio-heart is the only
independent toggle.

---

## C-Side File Map (Key Files with Guards)

| C File | Guard(s) | Guard Pattern |
|---|---|---|
| `src/uqm.c` | `USE_RUST_BRIDGE` | Inline (#ifdef for bridge init) |
| `src/libs/sound/audiocore_rust.c` | `USE_RUST_AUDIO` | Pattern 2 (shim) |
| `src/libs/sound/stream.c` | `USE_RUST_AUDIO_HEART` | Pattern 1 (#ifndef) |
| `src/libs/sound/trackplayer.c` | `USE_RUST_AUDIO_HEART` | Pattern 1 |
| `src/libs/sound/music.c` | `USE_RUST_AUDIO_HEART` | Pattern 1 |
| `src/libs/sound/sfx.c` | `USE_RUST_AUDIO_HEART` | Pattern 1 |
| `src/libs/sound/sound.c` | `USE_RUST_AUDIO_HEART` | Pattern 1 |
| `src/libs/sound/fileinst.c` | `USE_RUST_AUDIO_HEART` | Pattern 1 |
| `src/libs/sound/mixer/mixer.c` | `USE_RUST_MIXER` | Pattern 3 (inline) |
| `src/libs/sound/decoders/decoder.c` | per-format `USE_RUST_{OGG,WAV,MOD,...}` | Pattern 3 |
| `src/libs/threads/thrcommon.c` | `USE_RUST_THREADS` | Pattern 1 |
| `src/libs/threads/rust_thrcommon.c` | `USE_RUST_THREADS` | Pattern 2 (shim) |
| `src/libs/resource/getres.c` | `USE_RUST_RESOURCE` | Pattern 1 |
| `src/libs/resource/resinit.c` | `USE_RUST_RESOURCE` | Pattern 1 |
| `src/libs/resource/loadres.c` | `USE_RUST_RESOURCE` | Pattern 1 |
| `src/libs/resource/propfile.c` | `USE_RUST_RESOURCE` | Pattern 1 |
| `src/libs/resource/rust_resource.c` | `USE_RUST_RESOURCE` | Pattern 2 (shim) |
| `src/libs/input/sdl/vcontrol.c` | `USE_RUST_INPUT` | Pattern 3 |
| `src/libs/input/sdl/rust_vcontrol_impl.c` | `USE_RUST_INPUT` | Pattern 2 (shim) |
| `src/libs/video/vidplayer.c` | `USE_RUST_VIDEO` | Pattern 1 |
| `src/libs/video/rust_video.c` | `USE_RUST_VIDEO` | Pattern 2 (shim) |
| `src/libs/graphics/sdl/sdl_common.c` | `USE_RUST_GFX` | Pattern 3 |
| `src/libs/file/files.c` | `USE_RUST_FILE` | Pattern 3 |
| `src/libs/memory/w_memlib.c` | `USE_RUST_MEM` | Pattern 3 |
| `src/libs/uio/io.c` | `USE_RUST_UIO` | Pattern 3 |
| `src/uqm/comm.c` | `USE_RUST_COMM` | Pattern 1 |
| `src/uqm/rust_comm.c` | `USE_RUST_COMM` | Pattern 2 (shim) |
| `src/uqm/clock.c` | `USE_RUST_CLOCK` | Pattern 3 |
| `src/uqm/clock_rust.c` | `USE_RUST_CLOCK` | Pattern 2 (shim) |
| `src/uqm/state.c` | `USE_RUST_STATE` | Pattern 3 (not yet wired) |

---

## How to Reason About This for New Work

### "Where does X happen?"

1. Find the C define (`USE_RUST_*`) for the subsystem.
2. `grep USE_RUST_FOO sc2/src/ -r` to find C guard sites.
3. Check `rust/src/lib.rs` for the corresponding Rust module.
4. Check `project-plans/` for the canonical spec and plan.

### "Is the Rust version active?"

```sh
grep 'USE_RUST_FOO' sc2/config_unix.h
```

If it says `#define USE_RUST_FOO`, Rust is active. If `/* #undef ... */`, C is active.

### "How do I add a new FFI function?"

1. Define `#[no_mangle] pub extern "C" fn rust_foo()` in the Rust module.
2. Declare `extern` in the C shim file or header.
3. Wrap calls in `#ifdef USE_RUST_FOO`.
4. Run full build to verify linkage.

---

## Common Configuration Tasks

```sh
# Enable everything including audio-heart:
USE_RUST_AUDIO_HEART=1 ./build.sh uqm

# Enable Rust bridge, not audio-heart:
./build.sh uqm config    # "Rust bridge -> enabled"
./build.sh uqm

# Disable all Rust:
./build.sh uqm config    # "Rust bridge -> disabled"
./build.sh uqm

# Reprocess config without interactive menu:
./build.sh uqm reprocess_config
```

---

## Troubleshooting

| Symptom | Likely Cause |
|---|---|
| Link error: undefined `PLRPlaySong`, `LoadMusicFile` | `audio_heart` Cargo feature not enabled but `USE_RUST_AUDIO_HEART` is set in C |
| Link error: duplicate symbols | Both C and Rust providing same function — config out of sync |
| Link error: undefined `rust_mixer_*` | Rust bridge disabled but C expects Rust mixer |
| No sound | Mixer pump thread failed to start — check stderr for `[mixer_pump]` |
| Crash in `HMalloc` | `USE_RUST_MEM` mismatch — C calling Rust but lib not linked |
| Keyboard not working | `USE_RUST_INPUT` active but VControl init failed |
| `config_unix.h` has `#define` but feature still off | Some defines are set only via `-D` CFLAGS, not in `config_unix.h.in` |
