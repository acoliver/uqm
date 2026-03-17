# Phase 01: Analysis

## Phase ID
`PLAN-20260314-AUDIO-HEART.P01`

## Prerequisites
- Required: Phase P00.5 completed (preflight verification passed)

## Purpose

Map every gap between current code and specification/requirements. Produce entity/flow analysis, integration touchpoints, explicit old-code-to-replace inventory, and a normalized traceability matrix that can be audited against the prose requirements source.

## Traceability Normalization

The supplied `requirements.md` is prose-only and does **not** define a stable standalone REQ-ID registry. Therefore this phase must normalize traceability as follows:

1. Every gap maps to one or more quoted requirement themes from `requirements.md`.
2. Every gap also maps to the most specific supporting `specification.md` section.
3. Any legacy pseudo-REQ labels used in earlier notes are treated as non-authoritative shorthand and must not be used as auditable proof of coverage.
4. The output of this phase must include a traceability matrix from G1-G13 and P03-P12 to requirement themes/spec sections.

## Gap Analysis

### G1: Internal Music Loader Stub (Critical)

**Current state:** `music::get_music_data()` (`rust/src/sound/music.rs:181-189`) allocates an empty `SoundSample` with 64 buffers and no decoder. Does not open any file, does not create a decoder.

**Required state (requirements + spec):**
- Requirements: Resource and UIO integration; Ownership and Lifecycle Obligations → Resource loading obligations; Loader consolidation
- Spec: §14.1, §14.4
- Single canonical implementation that:
  1. Acquires file-load guard
  2. Validates filename (non-empty)
  3. Opens file via UIO
  4. Creates appropriate decoder (format detection by extension)
  5. Creates SoundSample with decoder attached
  6. Returns MusicRef

**Real implementation exists at:** `heart_ffi.rs:1209-1283` (LoadMusicFile FFI export)

**Resolution:** Extract the real loading logic from `heart_ffi.rs` into a canonical function accessible from both `fileinst.rs` and `heart_ffi.rs`. P04 must compare ownership-boundary options before choosing final placement.

### G2: Internal SFX Bank Loader Stub (Critical)

**Current state:** `sfx::get_sound_bank_data()` (`rust/src/sound/sfx.rs:275-286`) returns an empty `SoundBank` with only `source_file` set.

**Required state (requirements + spec):**
- Requirements: Resource and UIO integration; Integration Obligations → Mixer integration; Resource loading obligations; Loader consolidation
- Spec: §14.2, §14.4
- Single canonical implementation that:
  1. Acquires file-load guard
  2. Parses bank file (listing of audio filenames)
  3. Loads decoder for each entry
  4. Pre-decodes all audio
  5. Uploads PCM to mixer buffers
  6. Returns SoundBank with fully-populated samples

**Real implementation exists at:** `heart_ffi.rs:1051-1206` (LoadSoundFile FFI export)

**Resolution:** Same pattern as G1 — extract into canonical function, with architecture decision justified in P04.

### G3: fileinst.rs Routes Through Stubs (Critical)

**Current state:** `fileinst.rs` delegates to `sfx::get_sound_bank_data` and `music::get_music_data` which are both stubs.

**Required state:**
- Requirements: Resource loading obligations; Loader consolidation
- Spec: §14.4
- `fileinst.rs` must route through the canonical loaders.

**Resolution:** After G1/G2 produce canonical loaders, update `fileinst.rs` to call them.

### G4: Multi-Track Decoder Loading Placeholder (Critical)

**Current state:** `trackplayer::splice_multi_track()` (`rust/src/sound/trackplayer.rs:486-505`) creates chunks with `decoder: None` and a comment "decoder loading deferred to FFI."

**C reference:** `trackplayer.c:384-391` loaded real decoders via `SoundDecoder_Load` for each track.

**FFI shim state:** `heart_ffi.rs:658-672` (SpliceMultiTrack FFI) calls `trackplayer::splice_multi_track` without loading decoders — it passes track name strings but the internal function doesn't load decoders.

**Required state:**
- Requirements: Multi-track assembly; Decoder integration
- Spec: §8.1
- Each non-null track gets a real decoder, `dec_offset` advances by decoder length.
- If there is no base track context, the request is silently ignored.

**Resolution:** The FFI shim for `SpliceMultiTrack` must load decoders (same pattern as `SpliceTrack` FFI which loads decoders at lines 142-294) before calling the internal function, OR the internal function must accept pre-loaded decoders.

### G5: PLRPause Semantics (High)

**Current state:** `heart_ffi.rs:831-838` — PLRPause ignores the music_ref argument and always pauses current stream. Comment acknowledges the deviation.

**Required state:**
- Requirements: Music behavior; Handle identity
- Spec: §10.4
- Pause only when the supplied reference matches `cur_music_ref` (by raw-handle identity) OR is the wildcard sentinel (`~0`).

**Resolution:** Add ref-matching check in PLRPause FFI shim. Need a `plr_pause_if_matching(music_ref)` function or modify the FFI to check before calling plr_pause.

### G6: NORMAL_VOLUME Conflict (High)

**Current state:**
- `types.rs:112`: `pub const NORMAL_VOLUME: i32 = 160;` [OK] (matches spec)
- `control.rs:29`: `pub const NORMAL_VOLUME: i32 = MAX_VOLUME;`  (255, conflicts)
- `control.rs:277-279`: test asserts `NORMAL_VOLUME == MAX_VOLUME`

**Required state:**
- Requirements: Volume behavior
- Spec: §6
- Single canonical value = 160.

**Resolution:** Remove `NORMAL_VOLUME` from `control.rs`, use `types::NORMAL_VOLUME` everywhere. Fix VolumeState::new() to use 160.

### G7: Pending-Completion State Machine (High)

**Current state:** No `PollPendingTrackCompletion` or `CommitTrackAdvancement` exists. The `on_tagged_buffer` callback in trackplayer updates `cur_sub_chunk` directly. No pending-completion slot.

**Required state:**
- Requirements: Subtitle synchronization; Comm integration; Concurrency expectations
- Spec: §8.3.1
- Provider-side state machine with:
  - Single-slot pending completion state
  - `PollPendingTrackCompletion()` — atomic claim-and-clear
  - `CommitTrackAdvancement()` — advance to next phrase
  - StopTrack clears pending without invoking
  - Seek defers if slot occupied

**Impact:** This is required by the comm subsystem contract (`comm/specification.md §6A.8`). Without it, phrase callbacks may run on the decoder thread instead of the main thread, causing race conditions.

**Resolution:** Split work into two explicit parts:
1. **Integration proof/adoption analysis:** locate the exact comm-side poll loop, determine where the consumer will call the provider operations, verify whether the ABI already has compatible hooks, and define the bridging representation expected across the ABI.
2. **Provider implementation:** only after the call path is confirmed, add pending-completion state, provider functions, and any necessary exports/header declarations.

### G8: WaitForSoundEnd Incomplete (Medium)

**Current state:** `control.rs:183-200` — accepts `Option<usize>` (None = all, Some = specific channel). Does not handle:
- Paused sources as active (spec §13.3: paused = still active)
- The WAIT_ALL_SOURCES sentinel properly (FFI converts 0xFFFF to None)
- Out-of-range values treated as all-sources (spec §13.3: default-branch behavior)
- Inactive-source rule (no sample or no mixer handle = inactive)
- Concurrent teardown / pre-quit timing cases are not yet explicitly verified

**Resolution:** Implement full spec §13.3 logic in `wait_for_sound_end`. Ensure FFI handles the correct sentinel, exit-on-quit-before-entry, and concurrent teardown safely.

### G9: No Pre-Init Guard (Medium)

**Current state:** FFI functions do not check whether `init_stream_decoder` has been called. If called before init, they may access uninitialized state.

**Required state:**
- Requirements: Pre-initialization behavior; Error handling; ABI and C integration
- Spec: §13.1, §19.3
- All FFI-exposed APIs return ABI-defined failure outcomes when called before init.

**Resolution:** Add an initialized flag (AtomicBool) set by `init_stream_decoder`, checked by FFI functions. Return null/0/void per the ABI failure mode map.

### G10: Diagnostic Scaffolding (Medium)

**Current state:** 83 eprintln calls across sound modules, including `[PARITY]` prefixed diagnostic output in stream seek, track seek, subtitle logging, mixer pump diagnostics, and splice debug output.

**Required state:**
- Requirements: Maintainability and cleanup
- Spec: §23.2, §24
- All `[PARITY]`-prefixed output and development-only debug logging removed or converted to conditional `log::trace!`/`log::debug!` calls behind the `log` crate.

**Resolution:** Replace `eprintln!("[PARITY]...")` with `log::trace!(...)`. Replace operational `eprintln!` in mixer pump with `log::debug!(...)`. Remove one-time debug output from FFI shims (PLRPlaySong, PlayChannel, LoadSoundFile, etc.).

### G11: Warning Suppression Attributes (Medium)

**Current state:** Seven modules have `#![allow(dead_code, unused_imports, unused_variables)]`:
- `stream.rs:5`
- `trackplayer.rs:5`
- `music.rs:5`
- `sfx.rs:5`
- `control.rs:4`
- `fileinst.rs:3`
- `heart_ffi.rs:3-8`

**Required state:**
- Requirements: Maintainability and cleanup
- Spec: §23.2
- All allow attributes removed. All code is used or explicitly cfg-gated.

**Resolution:** Remove allows, fix resulting warnings (remove truly dead code, add `_` prefixes for intentionally unused params, add cfg gates). Because earlier clippy gates are only partially meaningful while blanket suppression remains, P12 must explicitly describe those earlier gates as provisional.

### G12: Residual C Code (Low)

**Current state:** Even with `USE_RUST_AUDIO_HEART`, C code still compiles:
- `sound.c:26-69` — volume globals (`musicVolume`, `musicVolumeScale`, etc.), `CleanSource`, `StopSource`
- `music.c:158-236` — `CheckMusicResName`, `_GetMusicData`, `_ReleaseMusicData`
- `sfx.c:162-298` — `_GetSoundBankData`, `_ReleaseSoundBankData`

**Required state:**
- Requirements: Maintainability and cleanup
- Spec: §23.2
- These C implementations are fully replaced by Rust equivalents and guarded out.

**Resolution:** Extend `#ifndef USE_RUST_AUDIO_HEART` guards in C files to cover the residual tails. This is a C-side change, coordinated with the Rust loader consolidation.

### G13: InitSound Return Code (Low)

**Current state:** `heart_ffi.rs:993-1004` returns 1 for success, 0 for error. The C header declares `BOOLEAN InitSound(...)` — BOOLEAN 1=TRUE is the correct C convention. The spec §19.3 says "returns 0 for success, -1 for failure" but this conflicts with the actual C header `BOOLEAN` type.

**Required state:**
- Requirements: ABI and C integration; Contract hierarchy
- Spec: §2.1, §19.3
- The C header is the primary ABI contract.

**Resolution:** No code change needed — document the spec/header mismatch in analysis and preserve BOOLEAN semantics.

## Integration Touchpoints

| Touchpoint | Current State | Required Action |
|-----------|---------------|-----------------|
| `heart_ffi.rs` → `fileinst.rs` | FFI does real loading inline, bypasses fileinst | Consolidate: FFI calls canonical loader via fileinst |
| `fileinst.rs` → `music.rs` / `sfx.rs` | Routes to stubs | Route to canonical loaders |
| Loader ownership boundary | Not yet decided | Compare `loading.rs` vs expanding `fileinst.rs` vs lower-level helper placement; choose and justify one |
| `heart_ffi.rs` SpliceMultiTrack → `trackplayer.rs` | No decoder loading | Load decoders at FFI boundary |
| `heart_ffi.rs` PLRPause → `music.rs` | Ignores ref argument | Add ref-matching |
| `trackplayer.rs` / `heart_ffi.rs` → comm subsystem | No pending-completion provider, no proven consumer call path | Identify exact comm poll/adoption path before adding ABI hooks |
| `heart_ffi.rs` all functions → `stream.rs` init state | No pre-init check | Add initialized guard |
| C `sound.c`, `music.c`, `sfx.c` tails | Still compiled | Extend guards |

## Old Code to Replace/Remove

| File | Lines | What | Action |
|------|-------|------|--------|
| `rust/src/sound/music.rs` | 181-189 | Stub `get_music_data` | Replace with canonical loader call |
| `rust/src/sound/music.rs` | 222-224 | Trivial `check_music_res_name` | Replace with real validation |
| `rust/src/sound/sfx.rs` | 275-286 | Stub `get_sound_bank_data` | Replace with canonical loader call |
| `rust/src/sound/heart_ffi.rs` | 1051-1206 | Inline `LoadSoundFile` loader | Extract to canonical function |
| `rust/src/sound/heart_ffi.rs` | 1209-1283 | Inline `LoadMusicFile` loader | Extract to canonical function |
| `rust/src/sound/heart_ffi.rs` | 831-838 | PLRPause without ref-matching | Add ref check |
| `rust/src/sound/control.rs` | 29 | NORMAL_VOLUME = MAX_VOLUME | Remove, use types::NORMAL_VOLUME |
| `rust/src/sound/control.rs` | 277-279 | Test asserting wrong value | Fix assertion |
| `sc2/src/libs/sound/music.c` | 158-236 | C resource helpers | Extend USE_RUST_AUDIO_HEART guard |
| `sc2/src/libs/sound/sfx.c` | 162-298 | C resource helpers | Extend USE_RUST_AUDIO_HEART guard |
| `sc2/src/libs/sound/sound.c` | 26-69 | C volume globals | Extend USE_RUST_AUDIO_HEART guard |

## Required Output: Traceability Matrix

This phase must produce a matrix with at least these columns:
- Gap / Phase
- Requirement theme(s) from `requirements.md`
- Supporting `specification.md` section(s)
- Planned implementation phase(s)
- Planned verification artifact(s)

The matrix must explicitly cover:
- loader consolidation
- multi-track decoder behavior
- pending-completion provider + comm adoption
- wait-for-end semantics
- pre-init guards
- cleanup/end-state obligations

## Verification

- [ ] All gaps mapped to specific files and line ranges
- [ ] All gaps mapped to requirement themes and spec sections
- [ ] Integration touchpoints complete, including comm adoption path and loader ownership boundary decision inputs
- [ ] Old code inventory complete
- [ ] No gap left unaddressed from initialstate.md
- [ ] Traceability matrix added and auditable without invented REQ IDs

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P01.md`
