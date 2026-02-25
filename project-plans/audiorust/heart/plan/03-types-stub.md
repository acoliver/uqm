# Phase 03: Shared Types Stub

## Phase ID
`PLAN-20260225-AUDIO-HEART.P03`

## Prerequisites
- Required: Phase P02a (Pseudocode Verification) passed
- Expected files: All 7 pseudocode files in `analysis/pseudocode/`

## Requirements Implemented (Expanded)

### REQ-CROSS-CONST-01 through REQ-CROSS-CONST-08: Constants
**Requirement text**: The system shall define MAX_VOLUME (255), NORMAL_VOLUME (160), NUM_SFX_CHANNELS (5), source indices (0-6), PAD_SCOPE_BYTES (256), ACCEL_SCROLL_SPEED (300), TEXT_SPEED (80), ONE_SECOND (840).

Behavior contract:
- GIVEN: No shared audio types exist yet
- WHEN: The types stub module is created
- THEN: All constants, enums, and struct shells are defined and compile

### REQ-CROSS-ERROR-01 through REQ-CROSS-ERROR-03: Error Handling
**Requirement text**: AudioError enum with 14 variants, Display/Error impls, From conversions.

Behavior contract:
- GIVEN: MixerError and DecodeError exist in the codebase
- WHEN: AudioError is defined
- THEN: From<MixerError> and From<DecodeError> conversions compile

### REQ-CROSS-GENERAL-01: parking_lot Mutex
**Requirement text**: All Mutex acquisitions shall use parking_lot::Mutex.

Behavior contract:
- GIVEN: parking_lot is already a dependency (used by mixer)
- WHEN: Shared types reference Mutex
- THEN: They use parking_lot::Mutex consistently

### REQ-CROSS-GENERAL-04: Send+Sync Bounds
**Requirement text**: SoundDecoder is Send; SoundSample is Send+Sync when wrapped in Arc<Mutex<>>.

Behavior contract:
- GIVEN: SoundDecoder: Send already
- WHEN: SoundSample is defined
- THEN: It satisfies Send when inner types are Send

### REQ-CROSS-GENERAL-05: Time FFI
**Requirement text**: GetTimeCounter via FFI, ONE_SECOND=840.

Behavior contract:
- GIVEN: C function GetTimeCounter exists
- WHEN: Time helper is defined
- THEN: get_time_counter() wraps the FFI call safely

### REQ-CROSS-GENERAL-07: Module Registration
**Requirement text**: All new modules added to sound::mod.rs.

Behavior contract:
- GIVEN: sound::mod.rs exists with existing module declarations
- WHEN: New modules are registered
- THEN: They compile as part of the workspace

## Implementation Tasks

### Files to create
- `rust/src/sound/types.rs` — AudioError, AudioResult, constants, SoundSample, SoundTag, SoundSource, SoundPosition, StreamCallbacks trait, helper functions (decode_all, get_decoder_time)
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P03`
  - marker: `@requirement REQ-CROSS-CONST-01..08, REQ-CROSS-ERROR-01..03, REQ-CROSS-GENERAL-01..08`

### Files to modify
- `rust/src/sound/mod.rs`
  - Add: `pub mod types;`
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P03`
  - marker: `@requirement REQ-CROSS-GENERAL-07`

### Stub contents of `types.rs`
1. `AudioError` enum with all 14 variants — full implementation (Display, Error, From impls)
2. `AudioResult<T>` type alias
3. All constants (NUM_SFX_CHANNELS through ONE_SECOND)
4. `SoundSample` struct shell (all fields as specified in spec §3.1.1)
5. `SoundTag` struct (`#[repr(C)]`)
6. `StreamCallbacks` trait with default no-op implementations
7. `SoundSource` struct shell (all fields as specified in spec §3.1.1)
8. `FadeState` struct
9. `SoundPosition` struct (`#[repr(C)]`, `Copy`, `Clone`)
10. `SoundBank` struct shell
11. `MusicRef` struct (`#[repr(transparent)]`)
12. `SubtitleRef` struct shell
13. `SoundChunk` struct shell (linked list node)
14. `fn get_time_counter() -> u32` — FFI wrapper
15. `fn quit_posted() -> bool` — FFI wrapper
16. `fn decode_all(decoder: &mut dyn SoundDecoder) -> DecodeResult<Vec<u8>>` — free function (may use `todo!()` in stub)
17. `fn get_decoder_time(decoder: &dyn SoundDecoder) -> f32` — free function (may use `todo!()` in stub)

### Allowed in stub phase
- `todo!()` for `decode_all` body
- `todo!()` for `get_decoder_time` body
- Struct constructors may be incomplete

### NOT allowed
- Fake success behavior
- Empty/trivial implementations pretending to be complete

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
```

## Structural Verification Checklist
- [ ] `rust/src/sound/types.rs` created
- [ ] `rust/src/sound/mod.rs` updated with `pub mod types;`
- [ ] `@plan` and `@requirement` markers present in types.rs
- [ ] `cargo check` passes (compiles without errors)
- [ ] `cargo fmt --all --check` passes

## Semantic Verification Checklist (Mandatory)
- [ ] AudioError has exactly 14 variants matching spec §2.1
- [ ] All From conversions compile (From<MixerError>, From<DecodeError>)
- [ ] All 8 constant groups defined (REQ-CROSS-CONST-01..08)
- [ ] SoundSample has all fields from spec §3.1.1
- [ ] SoundSource has all fields from spec §3.1.1
- [ ] StreamCallbacks trait has 5 methods with correct signatures
- [ ] SoundPosition is #[repr(C)] with positional, x, y fields
- [ ] MusicRef is #[repr(transparent)]
- [ ] parking_lot::Mutex used (not std::sync::Mutex)

## Deferred Implementation Detection (Mandatory)

```bash
# Only todo!() allowed in decode_all and get_decoder_time stubs
grep -n "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/types.rs
# Verify only expected stubs have todo!()
grep -n "todo!()" rust/src/sound/types.rs
```

## Success Criteria
- [ ] All types compile
- [ ] Module registered in mod.rs
- [ ] Constants accessible
- [ ] Error types have conversions

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs rust/src/sound/types.rs`
- blocking issues: If MixerError or DecodeError signatures have changed, update From impls

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P03.md`
