# Phase 06: Loader Consolidation — Implementation

## Phase ID
`PLAN-20260314-AUDIO-HEART.P06`

## Prerequisites
- Required: Phase P05a completed
- Expected files from previous phase: shared loader module/file with seam-based tests

## Requirements Implemented (Expanded)

### Canonical loading
**Requirement text**: The final audio-heart subsystem shall consolidate resource-loading logic so that there is a single canonical loading implementation per resource type, with all entry points routing through it.

Behavior contract:
- GIVEN: `load_music_canonical("music/track01.ogg")` called
- WHEN: UIO can open the file
- THEN: Returns MusicRef with decoder attached, sample.length set

- GIVEN: `load_sound_bank_canonical("comm/orz/orz.snd")` called
- WHEN: Bank file lists 5 WAV files, all loadable
- THEN: Returns SoundBank with 5 samples, each with mixer buffer containing decoded PCM

### Music loading / Sound bank loading
These requirements are satisfied here at the implementation level, but behavioral confidence must be tied to the seam-based tests from P05 plus explicit integration verification, not validation-only tests alone.

Why it matters:
- Closes the most critical gap: internal loaders are non-functional stubs
- Enables the resource system to load audio through Rust at all entry points
- Eliminates duplicate loading code between FFI and internal paths

## Implementation Tasks

### Files to modify

#### Shared loader module/file — implement canonical loaders
- Implement `load_music_canonical()`:
  1. Validate filename non-empty
  2. Extract extension, select decoder constructor
  3. Read file via the UIO seam/helper
  4. Create decoder from bytes via the decoder seam/helper
  5. Query decoder length
  6. Create SoundSample with decoder and NUM_BUFFERS_PER_SOURCE buffers
  7. Set sample.length
  8. Wrap in MusicRef and return
- Implement `load_sound_bank_canonical()`:
  1. Validate filename non-empty
  2. Read bank listing file via the UIO seam/helper
  3. Parse listing (one filename per line)
  4. Resolve relative entry paths from the bank file directory
  5. For each entry: read file, create decoder, decode all audio, create sample, upload PCM through the mixer upload seam/helper
  6. Collect all samples into SoundBank
  7. Return bank
- Implement supporting helpers using the seams established in P05
- marker: `@plan PLAN-20260314-AUDIO-HEART.P06`

#### `rust/src/sound/music.rs` — replace stub
- Replace `get_music_data()` body: call the shared canonical music loader
- Replace `check_music_res_name()`: delegate to shared validation logic if appropriate
- marker: `@plan PLAN-20260314-AUDIO-HEART.P06`

#### `rust/src/sound/sfx.rs` — replace stub
- Replace `get_sound_bank_data()` body: call the shared canonical bank loader
- marker: `@plan PLAN-20260314-AUDIO-HEART.P06`

#### `rust/src/sound/fileinst.rs` — verify routing
- `load_music_file()` already calls `music::get_music_data()` which now routes to canonical
- `load_sound_file()` already calls `sfx::get_sound_bank_data()` which now routes to canonical
- Verify the guard pattern still works correctly
- marker: `@plan PLAN-20260314-AUDIO-HEART.P06`

#### `rust/src/sound/heart_ffi.rs` — refactor FFI loaders
- `LoadMusicFile`: replace inline loading logic with call to `fileinst::load_music_file()` (which goes through canonical loader). Keep only the C-handle construction (MusicRef -> opaque pointer).
- `LoadSoundFile`: replace inline loading logic with call to `fileinst::load_sound_file()` (which goes through canonical loader). Keep only the STRING_TABLE construction.
- marker: `@plan PLAN-20260314-AUDIO-HEART.P06`

### Integration verification requirement
This phase must not claim full loader confidence from unit tests alone. It must do one of the following and record which path was completed:

- **Implemented now:** at least one integration-style fixture test for music loading and one for sound-bank loading using controlled resources; or
- **Deferred explicitly:** document named fixture scenarios that P06a/P12a will execute against the built subsystem and mark the current confidence as seam-level + structural until those checks pass.

Minimum deferred fixture scenarios if integration tests are not implemented in P06:
1. Music loader opens a real fixture resource through UIO and returns a playable MusicRef
2. Sound-bank loader parses a real `.snd` fixture with relative paths and uploads at least one sample to the mixer path
3. Both FFI and internal entry points hit the same canonical path for the same fixture resource

### Pseudocode traceability
- Implements PC-01 lines 01-18 (canonical music loader)
- Implements PC-02 lines 01-26 (canonical bank loader)
- Implements PC-03 lines 01-22 (routing consolidation)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Integration: verify both paths use the same loader
grep -n 'load_music_canonical\|load_sound_bank_canonical' rust/src/sound/

# Verify old inline loader code is removed from heart_ffi.rs
grep -c 'uio_fopen\|decode_all\|mixer_buffer_data' rust/src/sound/heart_ffi.rs
```

## Structural Verification Checklist
- [ ] Shared loader module/file has complete implementations (no `todo!()` remaining)
- [ ] `music::get_music_data` delegates to the canonical music loader
- [ ] `sfx::get_sound_bank_data` delegates to the canonical bank loader
- [ ] `heart_ffi::LoadMusicFile` calls through `fileinst::load_music_file`
- [ ] `heart_ffi::LoadSoundFile` calls through `fileinst::load_sound_file`
- [ ] Only ONE implementation of music loading exists
- [ ] Only ONE implementation of bank loading exists

## Semantic Verification Checklist
- [ ] All seam-level tests from P05 pass with real implementations
- [ ] `load_music_canonical` returns MusicRef with decoder attached
- [ ] `load_sound_bank_canonical` returns SoundBank with populated samples
- [ ] Bank loading handles relative-path resolution correctly
- [ ] Error paths return correct AudioError variants
- [ ] Integration verification path is either executed now or explicitly deferred with named fixture scenarios
- [ ] No placeholder/deferred implementation patterns remain in loader code

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/
```

## Success Criteria
- [ ] Single canonical loader per resource type
- [ ] All entry points converge
- [ ] Seam-level tests pass
- [ ] Integration verification path is explicit and realistic

## Failure Recovery
- rollback: restore only files touched by the chosen loader implementation path
- blocking issues: UIO FFI may need additional declarations; decoder constructors may need adjustment for byte-buffer input vs file-handle input

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P06.md`
