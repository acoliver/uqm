# How to Build, Run, and Test UQM with Rust

> **Audience:** A fresh LLM entering a new context to help with the UQM Rust port.
> Read this doc first for build/run/test mechanics, then `howtoconfigure.md` for
> the C/Rust boundary model, then `howloggingworks.md` for observability.

---

## Start Here: What This Project Is

UQM (The Ur-Quan Masters) is an open-source port of Star Control II. This repo
is incrementally porting the C codebase to Rust, **subsystem by subsystem**. The
C and Rust code coexist at compile time:

- C preprocessor guards (`#ifdef USE_RUST_*`) select which implementation is active.
- Rust compiles as a **static library** (`libuqm_rust.a`) that the C binary links via FFI.
- The C `main()` in `sc2/src/uqm.c` is always the real entry point. Rust does
  **not** own `main`.

> **Correction note:** `rust/src/main.rs` exists in the repo but is **not part
> of the normal build**. `Cargo.toml` defines only `staticlib` and `rlib` crate
> types — there is no `[[bin]]` target. The file is a leftover from early
> prototyping. Do not reference it as the active launcher.

---

## Onboarding Checklist — Inspect These First

| Priority | What to Read | Why |
|---|---|---|
| 1 | This file | Build, run, test |
| 2 | `dev-docs/rust/howtoconfigure.md` | C/Rust boundary model, FFI conventions |
| 3 | `dev-docs/rust/howloggingworks.md` | Logging, observability, debugging |
| 4 | `dev-docs/PLAN.md` | How implementation plans are structured |
| 5 | `dev-docs/RULES.md` | Dev rules (TDD, lint, etc.) |
| 6 | `rust/src/lib.rs` | Top-level Rust module map |
| 7 | `sc2/build/unix/build.config` | Build menu + `USE_RUST_*` define wiring |
| 8 | `sc2/src/uqm.c` | C entry point, option parsing, init sequence |
| 9 | Relevant `project-plans/` subdirectory | Spec + plan for whatever subsystem you're working on |

---

## Canonical Specs and Plans

All implementation plans live under `project-plans/`. Each subsystem has its own
directory with a `specification.md`, analysis artifacts, and phased plan files.

| Directory | Subsystem(s) Covered |
|---|---|
| `project-plans/audiorust/decoder/` | OGG, WAV, MOD, AIFF, DukAud decoders |
| `project-plans/audiorust/heart/` | Audio-heart: streaming, music, SFX, trackplayer |
| `project-plans/gfx/` | Graphics: drawable, frame, font, DCQ, scaling, SDL render |
| `project-plans/memandres/memory/` | `HMalloc`/`HFree`/`HCalloc`/`HRealloc` |
| `project-plans/memandres/resource/` | Resource index, loader, cache, string bank, prop files |
| `project-plans/memandres/state/` | Game state bitfield save/load |
| `project-plans/timeandfile/` | File I/O, dirs, temp, game clock, UIO |
| `project-plans/vid_thread_resource/` | Video decoder/player, threading, resource integration |
| `project-plans/sound/` | Mixer engine (buffer, source, resample, rodio backend) |

> The top-level files `project-plans/audiorust/c-heart.md`, `rust-heart.md`,
> etc. are C↔Rust analysis documents for those subsystems. The `specification.md`
> inside each subdirectory is the canonical spec.

---

## Prerequisites

| Requirement | Notes |
|---|---|
| macOS (arm64) | Primary dev target |
| Rust toolchain | `rustup`, `cargo` (stable) |
| Homebrew deps | `sdl2`, `libpng` |
| UQM content packs | Game data — see main README |

---

## Build Modes

| Mode | Command | Result |
|---|---|---|
| C-only | `cd sc2 && ./build.sh uqm` | Original C, no Rust |
| Rust bridge | `cd sc2 && ./build.sh uqm config` → "Rust bridge → enabled", then `./build.sh uqm` | All Rust subsystems active except audio-heart |
| Rust bridge + audio-heart | `USE_RUST_AUDIO_HEART=1 ./build.sh uqm` | Everything Rust including streaming/music/SFX |

The build menu toggles **all** Rust subsystems as a single unit (details in
`howtoconfigure.md`). Audio-heart is the only independently opt-in subsystem.

---

## Running the Game

```sh
cd sc2
./uqm
```

### CLI Flags

| Flag | Purpose |
|---|---|
| `-l FILE` / `--logfile=FILE` | Redirect stderr (C + Rust) to file |
| `-n DIR` / `--contentdir=DIR` | Override content directory |
| `-M VOL` / `--musicvol=VOL` | Music volume 0–100 |
| `-S VOL` / `--sfxvol=VOL` | SFX volume 0–100 |
| `-T VOL` / `--speechvol=VOL` | Speech volume 0–100 |
| `-q QUALITY` / `--audioquality=QUALITY` | `high`, `medium`, `low` |
| `-r WxH` / `--res=WxH` | Resolution (e.g. `640x480`) |
| `--fullscreen` | Fullscreen mode |
| `--fps` | Show FPS counter |

CLI parsing is handled by the C `main()` in `sc2/src/uqm.c` using `getopt_long`.

### Capturing Logs

```sh
./uqm 2>tmp/uqm_full.log            # capture all stderr
./uqm --logfile=tmp/uqm_full.log    # same via C flag
./uqm 2>&1 | tee tmp/uqm_live.log   # live + file
```

---

## Verifying Active Subsystems

| Log Line | Meaning |
|---|---|
| `RUST_BRIDGE_PHASE0_OK` in `rust-bridge.log` | Rust bridge initialized |
| `initAudio: Using Rust mixer backend` | Rust mixer active |
| `[mixer_pump] started` | cpal output thread running |
| `[audio_heart] InitSound: success` | Audio-heart active |
| `Rust memory management initialized.` | `USE_RUST_MEM` active |

---

## Fast Iteration

### Rebuild Rust Only (Skip C Recompile)

```sh
cd rust
cargo build --release                         # without audio-heart
cargo build --release --features audio_heart  # with audio-heart

# Then re-link:
cd ../sc2
./build.sh uqm
```

### Unit Tests

```sh
cd rust
cargo test                          # all tests except audio_heart-gated
cargo test --features audio_heart   # includes heart_ffi tests
```

Most tests are unit-level with mocks/stubs. No content files or audio device required.

### Integration Tests

```sh
cd rust
cargo test --test sound_integration
cargo test --test phase2_integration_tests
cargo test --test input_integration_tests
cargo test --test sdl_driver_tests
```

### Quality Gates (Must Pass Before Merge)

```sh
cd rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

### Check What Is Configured

```sh
grep 'USE_RUST' sc2/config_unix.h       # C config header (authoritative)
grep 'USE_RUST' sc2/build.vars           # build vars
grep 'features' rust/Cargo.toml          # Cargo features
```

---

## Repo Layout — Key Areas for a New LLM

### Rust Side (`rust/`)

```
rust/
├── Cargo.toml              # deps, features (audio_heart), crate-type = staticlib+rlib
├── build.rs                # cc/bindgen — compiles mem_wrapper.c
├── tests/                  # integration tests
│   ├── sound_integration.rs
│   ├── phase2_integration_tests.rs
│   ├── input_integration_tests.rs
│   └── sdl_driver_tests.rs
└── src/
    ├── lib.rs              #  top-level module map + re-exports
    ├── main.rs             #  DEAD CODE — not compiled (no [[bin]] target)
    ├── bridge_log.rs       # rust-bridge.log file logging
    ├── c_bindings.rs       # FFI declarations for calling into C
    ├── cli.rs              # clap CLI parser (unused in normal build)
    ├── config.rs           # Options struct
    ├── logging.rs          # LogLevel enum + FFI to C log_add()
    ├── memory.rs           # HMalloc/HFree/HCalloc/HRealloc
    ├── comm/               # Alien communication (dialogue, anim, subtitles)
    ├── game_init/          # Game initialization
    ├── graphics/           # Drawable, frame, font, DCQ, scaling, SDL, cmap
    ├── input/              # VControl, keyboard, joystick, key names
    ├── io/                 # File I/O, dirs, temp, uio_bridge
    ├── resource/           # Resource index, loader, cache, stringbank, propfile
    ├── sound/              # Decoders, mixer, heart modules, rodio backend
    │   ├── mixer/          #   mixer engine (buffer, source, resample, mix, ffi)
    │   ├── heart_ffi.rs    #   audio-heart FFI shim (cfg(feature="audio_heart"))
    │   ├── stream.rs       #   decoder thread, buffer tags, fade
    │   ├── trackplayer.rs  #   track player (comm speech)
    │   ├── music.rs        #   music playback
    │   ├── sfx.rs          #   SFX playback
    │   ├── ogg.rs          #   OGG decoder
    │   ├── wav.rs          #   WAV decoder
    │   ├── mod_decoder.rs  #   MOD/tracker decoder
    │   ├── aiff.rs         #   AIFF decoder
    │   └── dukaud.rs       #   DukAud decoder
    ├── state/              # Game state (bitfield save/load)
    ├── threading/          # Thread, mutex, semaphore, condvar
    ├── time/               # GameClock, GameDate, clock_bridge
    └── video/              # DukVid decoder, player, scaler
```

### C Side (`sc2/`)

```
sc2/
├── build.sh                # main build script
├── build.vars              #  generated — do not hand-edit
├── config_unix.h           #  generated — USE_RUST_* defines
├── config_unix.h.in        # template (SYMBOL_* placeholders)
├── build/unix/
│   └── build.config        #  build menu + rust_bridge_enabled_action()
└── src/
    ├── uqm.c              #  C main() — the real entry point
    ├── mem_wrapper.c       # compiled into libuqm_core by Rust build.rs
    ├── darwin/SDLMain.m    # macOS SDL bootstrap → calls main() in uqm.c
    └── ...                 # C subsystem files with #ifdef guards (see howtoconfigure.md)
```

### Project Plans

```
project-plans/
├── audiorust/
│   ├── decoder/            # OGG, WAV, MOD, AIFF, DukAud decoders
│   └── heart/              # streaming, music, SFX, trackplayer
├── gfx/                    # graphics subsystem
├── memandres/
│   ├── memory/             # HMalloc/HFree
│   ├── resource/           # resource loader, cache, index
│   └── state/              # game state bitfield
├── timeandfile/            # file I/O, dirs, clock, UIO
├── vid_thread_resource/    # video, threading, resource integration
└── sound/                  # mixer engine
```

### Dev Docs

```
dev-docs/
├── PLAN.md                 # how to write implementation plans
├── RULES.md                # development rules (TDD, lint, etc.)
├── project-standards.md    # coding standards
├── COORDINATING.md         # subagent coordination for multi-phase plans
├── PLAN-TEMPLATE.md        # reusable plan template
└── rust/
    ├── howtorun.md         # ← you are here
    ├── howtoconfigure.md   # C/Rust boundary model
    └── howloggingworks.md  # logging and observability
```

---

## How the C/Rust Coexistence Model Works (Summary)

Full details in `howtoconfigure.md`. The essential mental model:

1. **C owns `main()`**. The binary is always a C program (`sc2/uqm`).
2. **Rust compiles to `libuqm_rust.a`** and links into the C binary.
3. **`#ifdef USE_RUST_*` guards** in C files switch between C and Rust implementations at compile time.
4. Rust exposes `#[no_mangle] pub extern "C" fn` entry points that C calls.
5. When Rust bridge is enabled, **all** subsystem defines are set together (no per-subsystem build menu). Audio-heart is the sole independent toggle.
6. The build system runs `cargo build --release` as a pre-build step, keeping C defines and Cargo features in sync.

---

## Adding a New Rust Subsystem — Checklist

1. [ ] Create module under `rust/src/`, add `pub mod` to `lib.rs`
2. [ ] Write `#[no_mangle] pub extern "C" fn rust_*()` entry points
3. [ ] Create C shim file (`rust_*.c`) with `#ifdef USE_RUST_FOO` guards
4. [ ] Wrap original C code in `#ifndef USE_RUST_FOO`
5. [ ] Add `-DUSE_RUST_FOO` to `build.config` → `rust_bridge_enabled_action()`
6. [ ] Clear it in `rust_bridge_disabled_action()`
7. [ ] Add `@SYMBOL_USE_RUST_FOO_DEF@` to `config_unix.h.in`
8. [ ] Add `SYMBOL_USE_RUST_FOO_DEF` set/clear in both action functions
9. [ ] If gated by Cargo feature: add to `[features]` in `Cargo.toml`, use `#[cfg(feature = "...")]`
10. [ ] Write spec in `project-plans/<feature>/specification.md`
11. [ ] Write tests before implementation (`RULES.md` requires TDD)
