# Audio Heart / High-Level Sound Subsystem — Initial State

## Scope and purpose

The audio-heart subsystem is the high-level sound layer above the low-level audio backend/mixer and decoders. In C, it owns:

- streaming playback orchestration (`PlayStream`, `StopStream`, fade, scope/oscilloscope support),
- track/chunk assembly for comm speech + subtitles (`SpliceTrack`, `PlayTrack`, seek/page navigation),
- music and speech playback control (`PLR*`, `snd_PlaySpeech`),
- SFX channel playback and positional audio (`PlayChannel`, `UpdateSoundPosition`),
- top-level sound control/queries (`InitSound`, `SoundPlaying`, `WaitForSoundEnd`),
- file-based music/SFX resource loading (`LoadMusicFile`, `LoadSoundFile`).

Evidence:

- C replacement surface is explicitly declared in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/audio_heart_rust.h:6-8,27-139`.
- The shared C-side audio-heart data structures live in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sndintrn.h:29-75`.

## Current C structure

The original C subsystem is split across six high-level files, each now guarded by `#ifndef USE_RUST_AUDIO_HEART` around the high-level implementation:

- `/Users/acoliver/projects/uqm/sc2/src/libs/sound/stream.c:30-818`
- `/Users/acoliver/projects/uqm/sc2/src/libs/sound/trackplayer.c:29-884`
- `/Users/acoliver/projects/uqm/sc2/src/libs/sound/music.c:27-156`
- `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sfx.c:30-160` and `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sfx.c:300-316`
- `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sound.c:71-181`
- `/Users/acoliver/projects/uqm/sc2/src/libs/sound/fileinst.c:25-89`

The split is partial, not total. Several C-owned integration points remain outside the guard and therefore still compile even with `USE_RUST_AUDIO_HEART` enabled:

- `CleanSource` / `StopSource` and globals `musicVolume`, `musicVolumeScale`, `sfxVolumeScale`, `speechVolumeScale`, `soundSource[]` remain in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sound.c:26-69`.
- `CheckMusicResName`, `_GetMusicData`, `_ReleaseMusicData` remain in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/music.c:158-236`.
- `_GetSoundBankData`, `_ReleaseSoundBankData` remain in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sfx.c:162-298`.
- `DestroySound` / `GetSoundAddress` remain C-owned when the guard is not active for that tail section; the exported tail is still visible as a boundary marker in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sfx.c:300-316`.

That means the build currently switches the public high-level API surface to Rust, but some resource-loading and shared-state support code is still present in C.

## Current Rust structure

The Rust sound module exposes the audio-heart pieces under `rust/src/sound/`, with the FFI surface gated separately from the internal modules:

- module wiring: `/Users/acoliver/projects/uqm/rust/src/sound/mod.rs:30-41`
- shared types/constants: `/Users/acoliver/projects/uqm/rust/src/sound/types.rs:24-127,170-399`
- stream engine: `/Users/acoliver/projects/uqm/rust/src/sound/stream.rs:43-76,90-170`
- track player: `/Users/acoliver/projects/uqm/rust/src/sound/trackplayer.rs:42-67,120-182,321-760`
- music/speech control: `/Users/acoliver/projects/uqm/rust/src/sound/music.rs:23-47,53-173,180-265`
- SFX control/loading: `/Users/acoliver/projects/uqm/rust/src/sound/sfx.rs:41-62,68-120,122-204,210-299`
- top-level control: `/Users/acoliver/projects/uqm/rust/src/sound/control.rs:24-32,39-56,63-194`
- file-instance wrapper: `/Users/acoliver/projects/uqm/rust/src/sound/fileinst.rs:18-31,39-89`
- C ABI shim: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:37-52,393-1297`

Notably, the internal Rust modules are always compiled, but the actual C ABI export module is feature-gated:

- `/Users/acoliver/projects/uqm/rust/src/sound/mod.rs:33-34` only enables `pub mod heart_ffi;` under `#[cfg(feature = "audio_heart")]`.

## Build and config wiring

### C preprocessor wiring

The C build currently enables the subsystem switch globally in the generated Unix config header:

- `/Users/acoliver/projects/uqm/sc2/config_unix.h:80-81` defines `USE_RUST_AUDIO_HEART`.

That macro is what excludes the high-level C implementations listed above.

### Cargo feature wiring

The Rust crate defines a dedicated Cargo feature for the FFI export layer:

- `/Users/acoliver/projects/uqm/rust/Cargo.toml:5-7` defines the staticlib `uqm_rust`.
- `/Users/acoliver/projects/uqm/rust/Cargo.toml:37-39` defines `default = []` and `audio_heart = []`.
- `/Users/acoliver/projects/uqm/rust/src/sound/mod.rs:33-34` ties the actual export module to that feature.

Observed implication: the C-side switch and the Cargo feature are distinct mechanisms. Code under `rust/src/sound/{stream,trackplayer,music,sfx,control,fileinst}.rs` is not feature-gated, but the public Rust ABI that C needs is. If `USE_RUST_AUDIO_HEART` is enabled in C without building the Rust staticlib with Cargo feature `audio_heart`, the declarations in `audio_heart_rust.h` would not be backed by exported Rust symbols.

## C ↔ Rust integration points

### Public C header replacing the C implementations

`audio_heart_rust.h` is the contract surface replacing the C high-level subsystem:

- stream API: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/audio_heart_rust.h:23-40`
- sample/control API: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/audio_heart_rust.h:42-75`
- track-player API: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/audio_heart_rust.h:77-103`
- music/speech API: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/audio_heart_rust.h:105-117`
- SFX API: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/audio_heart_rust.h:119-130`
- file-loading API: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/audio_heart_rust.h:132-139`

### Rust exports implementing that contract

`heart_ffi.rs` exports the matching symbols:

- stream exports: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:393-606`
- track-player exports: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:608-759`
- music exports: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:761-883`
- SFX exports: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:885-985`
- top-level control exports: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:988-1045`
- file-loading exports: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1047-1297`

### C services still consumed by Rust

The Rust audio-heart implementation still depends on C-owned runtime services through FFI:

- UIO file access and `contentDir`: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:41-52,100-119`
- C string table allocation/free: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:50-52,82-98`
- Rust memory bridge functions used to match C allocation patterns: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:54-60`
- timing/quit state from C (`GetTimeCounter`, `QuitPosted`): `/Users/acoliver/projects/uqm/rust/src/sound/types.rs:136-152`

So the subsystem is not Rust-native end-to-end; it is Rust logic hosted inside the existing C runtime contract.

## What is already ported

### 1. Stream engine logic is materially ported

Rust has a real stream engine with global engine state, source table, sample lifecycle, play/stop/pause/resume/seek, tag handling, scope generation, and fade support:

- engine/source state: `/Users/acoliver/projects/uqm/rust/src/sound/stream.rs:43-76`
- sample creation/destruction: `/Users/acoliver/projects/uqm/rust/src/sound/stream.rs:82-143`
- play path and buffer prefill: `/Users/acoliver/projects/uqm/rust/src/sound/stream.rs:163-320`
- stop/pause/resume/seek/query: `/Users/acoliver/projects/uqm/rust/src/sound/stream.rs:323-423`
- scope/foreground graphing: `/Users/acoliver/projects/uqm/rust/src/sound/stream.rs:449-620`

This maps directly to the C stream responsibilities in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/stream.c:44-308` and the rest of the guarded file.

### 2. Track-player control path is ported

Rust owns track list state, track callbacks, splice logic, playback, and seek/page navigation:

- state and callback machinery: `/Users/acoliver/projects/uqm/rust/src/sound/trackplayer.rs:120-182,208-311`
- track assembly: `/Users/acoliver/projects/uqm/rust/src/sound/trackplayer.rs:321-520`
- playback and stop/jump/pause/resume: `/Users/acoliver/projects/uqm/rust/src/sound/trackplayer.rs:526-626`
- smooth/page seeking: `/Users/acoliver/projects/uqm/rust/src/sound/trackplayer.rs:632-742`
- position reporting: `/Users/acoliver/projects/uqm/rust/src/sound/trackplayer.rs:745-760`

The FFI side also explicitly recreates C’s per-page decoder-loading pattern in Rust before calling into the track-player core:

- `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:142-294`
- `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:616-645`

### 3. Music, speech, SFX, and top-level control APIs are ported

- music and speech playback wrappers: `/Users/acoliver/projects/uqm/rust/src/sound/music.rs:53-173`
- music volume/fade: `/Users/acoliver/projects/uqm/rust/src/sound/music.rs:230-265`
- SFX playback and positional control: `/Users/acoliver/projects/uqm/rust/src/sound/sfx.rs:68-252`
- top-level stop/query/wait and volume control: `/Users/acoliver/projects/uqm/rust/src/sound/control.rs:63-194`

### 4. Rust-side file loaders exist at the FFI boundary

The actual exported C-facing Rust loaders decode files and build C-compatible handles:

- `LoadSoundFile`: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1051-1206`
- `LoadMusicFile`: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1209-1283`
- `DestroyMusic`: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1285-1297`
- `DestroySound`: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:963-985`

These functions already bypass the older C `_GetMusicData` / `_GetSoundBankData` loaders for the direct `Load*File` path.

## What remains C-owned or C-dependent

### C-owned shared structs and globals remain authoritative at the ABI boundary

The original C struct layouts still define compatibility requirements:

- `TFB_SoundSample` and `TFB_SoundSource`: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sndintrn.h:40-72`

Rust mirrors behavior, but not by sharing the exact same in-memory structs. Instead, `heart_ffi.rs` translates to Rust-owned structs while manually reproducing C-facing layouts for `CStringTableEntry` and `CStringTable`:

- `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:82-98`

### C runtime services still host the subsystem

The Rust code still depends on:

- C UIO/content mounting: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:41-52,100-119`
- C string-table allocation/free: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:50-52,1181-1203,963-985`
- C time/quit globals: `/Users/acoliver/projects/uqm/rust/src/sound/types.rs:136-152`

### Some resource-loader support remains C-owned and still compiles

The resource-instantiation helpers are still present in C even when the high-level playback API is switched over:

- music resource helpers: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/music.c:158-236`
- sound bank resource helpers: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sfx.c:162-298`

That is a clear partial-port boundary.

## Partial-port boundaries, stubs, and parity markers

### Guard boundaries in C

The build split is visible and exact:

- `stream.c`: guard starts `/Users/acoliver/projects/uqm/sc2/src/libs/sound/stream.c:30`, ends `:818`
- `trackplayer.c`: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/trackplayer.c:29` to `:884`
- `music.c`: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/music.c:27` to `:156`
- `sound.c`: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sound.c:71` to `:181`
- `fileinst.c`: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/fileinst.c:25` to `:89`
- `sfx.c` high-level API: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sfx.c:30` to `:160`

### Rust “still in flux” markers

Multiple core Rust audio-heart modules are compiled with permissive warning suppression, which is a direct indicator that cleanup/integration is incomplete:

- `stream.rs`: `/Users/acoliver/projects/uqm/rust/src/sound/stream.rs:1-5`
- `trackplayer.rs`: `/Users/acoliver/projects/uqm/rust/src/sound/trackplayer.rs:1-5`
- `music.rs`: `/Users/acoliver/projects/uqm/rust/src/sound/music.rs:1-5`
- `sfx.rs`: `/Users/acoliver/projects/uqm/rust/src/sound/sfx.rs:1-5`
- `control.rs`: `/Users/acoliver/projects/uqm/rust/src/sound/control.rs:1-5`
- `fileinst.rs`: `/Users/acoliver/projects/uqm/rust/src/sound/fileinst.rs:1-3`
- `heart_ffi.rs`: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1-8`

### Concrete stub / non-parity behavior in Rust internals

Several Rust internals are still placeholders or weaker than the original C behavior.

#### `control.rs` init/uninit are effectively stubs

- `init_sound()` just returns `Ok(())`: `/Users/acoliver/projects/uqm/rust/src/sound/control.rs:57-61`
- `uninit_sound()` is a no-op comment: `/Users/acoliver/projects/uqm/rust/src/sound/control.rs:63-67`

C was also minimal here, but the Rust implementation is still just a placeholder lifecycle hook.

#### `music.rs` internal loader is not a real file loader

- `get_music_data()` only allocates an empty sample with 64 buffers and no decoder: `/Users/acoliver/projects/uqm/rust/src/sound/music.rs:180-189`
- `check_music_res_name()` only tests non-empty string: `/Users/acoliver/projects/uqm/rust/src/sound/music.rs:221-224`

By contrast, the old C helper really loaded and validated via `fileExists2` and `SoundDecoder_Load`:

- `/Users/acoliver/projects/uqm/sc2/src/libs/sound/music.c:158-205`

The direct FFI export `LoadMusicFile()` does implement real loading in Rust, but the internal helper remains a non-parity stub.

#### `sfx.rs` internal bank loading is explicitly placeholder behavior

- comment documents that full loading is deferred: `/Users/acoliver/projects/uqm/rust/src/sound/sfx.rs:270-281`
- implementation returns an empty `SoundBank` with just `source_file`: `/Users/acoliver/projects/uqm/rust/src/sound/sfx.rs:275-286`

The old C helper really parsed the bank file, loaded decoders, decoded all audio, and uploaded buffers:

- `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sfx.c:163-260`

Again, the exported FFI `LoadSoundFile()` does the real work, but the internal Rust helper is still a stub boundary.

#### `fileinst.rs` currently delegates to the stubby internal helpers

- sound path delegates to `sfx::get_sound_bank_data`: `/Users/acoliver/projects/uqm/rust/src/sound/fileinst.rs:66-68`
- music path delegates to `music::check_music_res_name` and `music::get_music_data`: `/Users/acoliver/projects/uqm/rust/src/sound/fileinst.rs:70-80`

So the `fileinst.rs` layer is not yet equivalent to the direct FFI loaders in `heart_ffi.rs`.

#### Track-player multi-track path is still reduced

`splice_multi_track()` in Rust says decoder loading is deferred and synthesizes placeholder chunks without decoders:

- `/Users/acoliver/projects/uqm/rust/src/sound/trackplayer.rs:486-505`

The original C `SpliceMultiTrack` actually loaded decoders for each track via `SoundDecoder_Load`:

- `/Users/acoliver/projects/uqm/sc2/src/libs/sound/trackplayer.c:384-391`

That is an important parity gap.

### Explicit parity-marker / debug instrumentation left in Rust

The Rust port still contains hardcoded debug output and parity markers, indicating active parity work rather than fully-settled production behavior:

- stream seek parity logging: `/Users/acoliver/projects/uqm/rust/src/sound/stream.rs:368-386`
- track seek parity logging: `/Users/acoliver/projects/uqm/rust/src/sound/trackplayer.rs:644-667`
- `SpliceTrack` debug output: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:625,771-785,900-903,993-1002,1063,1093,1145,1205,1221,1255`

These are evidence of active diagnostic scaffolding in the Rust path.

## Current state of file-loading split

There are effectively two Rust layers for resource/file loading:

1. **Internal Rust APIs** in `music.rs`, `sfx.rs`, `fileinst.rs` that are still partial/stubbed.
2. **FFI exports in `heart_ffi.rs`** that do real loading from C-visible file paths using C UIO and Rust decoders.

Evidence of that split:

- stub internal music loader: `/Users/acoliver/projects/uqm/rust/src/sound/music.rs:180-189`
- real FFI music loader: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1212-1283`
- stub internal SFX bank loader: `/Users/acoliver/projects/uqm/rust/src/sound/sfx.rs:275-286`
- real FFI sound-bank loader: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1054-1206`

This is the clearest evidence that the subsystem is only partially ported at the architectural level, even though the public C-facing API exists.

## Parity gaps

### 1. Internal Rust loader APIs do not match old C behavior

As noted above, `music::get_music_data()` and `sfx::get_sound_bank_data()` are not equivalent to the old C `_GetMusicData()` and `_GetSoundBankData()` helpers.

### 2. Rust `PLRPause` semantics are intentionally looser than C

The FFI shim documents a behavioral difference:

- Rust comment: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:835-837`

C only paused when the passed `MusicRef` matched `curMusicRef` or `~0`:

- `/Users/acoliver/projects/uqm/sc2/src/libs/sound/music.c:91-100`

Rust always pauses the current stream once the pointer is non-null.

### 3. Rust top-level constants differ from C defaults

Rust shared constants define `NORMAL_VOLUME = 160` in `types.rs`:

- `/Users/acoliver/projects/uqm/rust/src/sound/types.rs:108-124`

But `control.rs` redefines `NORMAL_VOLUME` as `MAX_VOLUME`:

- `/Users/acoliver/projects/uqm/rust/src/sound/control.rs:24-32`

That is an internal inconsistency inside the Rust port itself.

### 4. Rust `GraphForegroundStream` returns C-style success code, but via different implementation details

The Rust implementation reproduces source-selection and AGC behavior, but it is an independent implementation, not a shared algorithm with C:

- `/Users/acoliver/projects/uqm/rust/src/sound/stream.rs:482-620`

The original behavior lives in C `stream.c` within the guarded region beginning at `/Users/acoliver/projects/uqm/sc2/src/libs/sound/stream.c:30`.

This is not necessarily wrong, but it is a parity-sensitive area because it encodes UI-visible waveform behavior.

### 5. Rust trackplayer still contains ignored/stub-oriented tests

- `/Users/acoliver/projects/uqm/rust/src/sound/trackplayer.rs:1474` has `#[ignore = "P11: play_track stub"]`

That is a direct signal that at least some test coverage still treats portions of the implementation as not-final.

## Notable risks and unknowns

### Cargo-feature / C-macro mismatch risk

The subsystem requires both:

- C macro enablement: `/Users/acoliver/projects/uqm/sc2/config_unix.h:80-81`
- Rust feature enablement for FFI exports: `/Users/acoliver/projects/uqm/rust/Cargo.toml:37-39` and `/Users/acoliver/projects/uqm/rust/src/sound/mod.rs:33-34`

The code evidence here does not show the final build invocation that guarantees `--features audio_heart` whenever `USE_RUST_AUDIO_HEART` is defined. That wiring remains an integration risk.

### ABI/layout risk at the C boundary

Rust manually reproduces selected C struct layouts for `STRING_TABLE` and relies on pointer-slot conventions for `MUSIC_REF` and `SOUND_REF`:

- C string-table layout assumptions: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:82-98`
- `MUSIC_REF` double-pointer convention: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:348-365,1274-1297`
- sound-bank pointer-slot convention: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:889-915,1194-1203`

Those are parity-critical ABI assumptions and can drift if the C side changes.

### Runtime dependency risk on C UIO/resource environment

Rust file loading is not independent; it depends on C globals and mounted content setup:

- `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:41-52,100-119`

If UIO/content setup changes, the Rust audio-heart loader path changes with it.

### Duplicate ownership patterns increase maintenance risk

There are now overlapping implementations for file loading:

- old C resource helpers still present: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/music.c:166-236`, `/Users/acoliver/projects/uqm/sc2/src/libs/sound/sfx.c:162-298`
- partial internal Rust helpers: `/Users/acoliver/projects/uqm/rust/src/sound/music.rs:180-224`, `/Users/acoliver/projects/uqm/rust/src/sound/sfx.rs:270-299`
- real FFI Rust loaders: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1054-1297`

That duplication means parity fixes may need to be applied in more than one place or consolidated later.

## Bottom line

The audio-heart subsystem is **publicly switched to Rust at the high-level API boundary**, with Rust exports covering stream control, track-player behavior, music/speech control, SFX control, top-level sound control, and direct file loading. The evidence is the guarded-out C implementations in the six C source files and the matching Rust export surface in `audio_heart_rust.h` plus `heart_ffi.rs`.

However, it is still **partially ported** in three important ways:

1. **C support code remains compiled and authoritative** for some shared/resource behaviors (`sound.c`, `music.c`, `sfx.c` tails outside the guard).
2. **Rust internal helper layers are not yet fully parity-complete**, especially `music::get_music_data`, `sfx::get_sound_bank_data`, `fileinst.rs`, and `trackplayer::splice_multi_track`.
3. **The Rust path still depends on C runtime services** for file I/O, content mounting, timing, and string-table allocation.

So the current state is best described as: **Rust owns the high-level execution path, but the subsystem is still hybrid at the integration and resource-loading layers, with visible parity scaffolding and several internal stubs remaining.**