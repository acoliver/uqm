# Phase 03: CommData & LOCDATA FFI

## Phase ID
`PLAN-20260314-COMM.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed
- Verify previous phase markers/artifacts exist

## Requirements Implemented (Expanded)

### EC-REQ-003: Race-specific communication data initialization
**Requirement text**: When an encounter is initialized, the communication subsystem shall initialize race-specific communication data before any dialogue presentation or player response interaction begins.

Behavior contract:
- GIVEN: An encounter with a known CONVERSATION enum variant
- WHEN: C-owned `init_race(comm_id)` dispatch is invoked through a thin wrapper
- THEN: Rust-owned CommData is populated with all LOCDATA fields

Why it matters: Every encounter depends on CommData being fully populated before scripts run.

### DS-REQ-004: Encounter data retention
**Requirement text**: When a race-specific script returns encounter communication data, the communication subsystem shall copy or retain that data in a form that remains valid for the duration of the encounter.

Behavior contract:
- GIVEN: `init_*_comm()` returns a `LOCDATA*` pointer
- WHEN: The Rust subsystem reads the LOCDATA fields
- THEN: All field values are copied into Rust-owned storage, with borrowed pointers (callbacks, number speech) stored as FFI-safe types

### SC-REQ-003: No binary layout requirement
**Requirement text**: The communication subsystem shall not require binary layout compatibility with the C `LOCDATA` structure. Logical field compatibility obtained through FFI accessors is sufficient.

### DS-REQ-001, DS-REQ-002: Race dispatch
**Requirement text**: The communication subsystem shall dispatch to the correct race-specific script entrypoint, including state-dependent dispatch.

## Implementation Tasks

### Files to create

- `rust/src/comm/locdata.rs` — LOCDATA FFI accessor functions and CommData expansion
  - marker: `@plan PLAN-20260314-COMM.P03`
  - marker: `@requirement EC-REQ-003, DS-REQ-004, SC-REQ-003`
  - Contains: `read_locdata_from_c()`, `AnimationDescData`, `NumberSpeechRef`
  - Contains: FFI extern declarations for LOCDATA field accessors

### Files to modify

- `rust/src/comm/types.rs`
  - Expand `CommData` struct with all LOCDATA fields per spec §3.1
  - Add: resource ID fields (u32): `alien_frame_res`, `alien_font_res`, `alien_colormap_res`, `alien_song_res`, `alien_alt_song_res`, `conversation_phrases_res`
  - Add: text layout fields: `alien_text_fcolor`, `alien_text_bcolor` (Color), `alien_text_baseline` (Point), `alien_text_width`, `alien_text_align`, `alien_text_valign`
  - Add: animation descriptors: `alien_ambient_array: [AnimationDescData; 20]`, `alien_transition_desc`, `alien_talk_desc`
  - Add: loaded handles: `alien_frame`, `alien_font`, `alien_color_map`, `alien_song`, `conversation_phrases` (opaque u32 handles)
  - Add: `alien_number_speech: *const c_void` (borrowed C pointer, valid for encounter lifetime)
  - Add: `alien_song_flags: u32`
  - marker: `@plan PLAN-20260314-COMM.P03`

- `rust/src/comm/mod.rs`
  - Add `pub mod locdata;`
  - Add re-exports for new types
  - marker: `@plan PLAN-20260314-COMM.P03`

- `rust/src/comm/ffi.rs`
  - Add `rust_ReadLocdata(locdata_ptr: *const c_void) -> c_int` FFI export if useful for testing/introspection only
  - Do **not** add a public Rust-owned `rust_InitRace` dispatch export; Rust should consume the existing C-owned dispatch via `c_init_race(comm_id) -> LOCDATA*`
  - marker: `@plan PLAN-20260314-COMM.P03`

### C-side files to modify (minimal, for FFI accessors)

- `sc2/src/uqm/rust_comm.h`
  - Add declaration for `rust_ReadLocdata` if that helper is retained
  - Add declaration for `c_init_race(comm_id) -> LOCDATA*` as a C helper callable from Rust via FFI

- `sc2/src/uqm/rust_comm.c`
  - Add LOCDATA field accessor functions callable from Rust:
    - `c_locdata_get_init_func(LOCDATA*) -> void*`
    - `c_locdata_get_post_func(LOCDATA*) -> void*`
    - `c_locdata_get_uninit_func(LOCDATA*) -> void*`
    - `c_locdata_get_alien_frame_res(LOCDATA*) -> unsigned int`
    - (one accessor per field, approximately 20 accessors)
    - `c_locdata_get_anim_desc(LOCDATA*, index) -> AnimDescData` (for ambient array)
    - `c_init_race(comm_id) -> LOCDATA*` (wrapper around existing C `init_race` switch)

### Design note: init_race ownership boundary

This phase adopts a single coherent layering model:
- `init_race` dispatch remains **C-owned** for source compatibility with the existing 27 script entrypoints.
- Rust does **not** export a public `rust_InitRace` for scripts or C callers.
- Rust calls `c_init_race(comm_id) -> LOCDATA*`, then copies LOCDATA into Rust-owned `CommData`.

This keeps dispatch ownership simple and avoids duplicating the 27-way switch in Rust.

### AnimationDescData struct (shared C/Rust)

```rust
#[repr(C)]
pub struct AnimationDescData {
    pub start_index: u32,
    pub num_frames: u32,
    pub anim_flags: u32,
    pub base_frame_rate: u32,
    pub random_frame_rate: u32,
    pub base_restart_rate: u32,
    pub random_restart_rate: u32,
    pub block_mask: u32,
}
```

### Pseudocode traceability
- Uses pseudocode lines: 01-26 (read_locdata_from_c), 30-35 (load_commdata_for_race)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/comm/locdata.rs` created
- [ ] `CommData` expanded with all LOCDATA fields
- [ ] `AnimationDescData` repr(C) struct created
- [ ] FFI accessors declared for all LOCDATA fields
- [ ] `c_init_race(comm_id) -> LOCDATA*` helper present and used by Rust
- [ ] No public `rust_InitRace` export remains in the design
- [ ] Plan/requirement traceability present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] `CommData` has logical field parity with C `LOCDATA` (all 26+ fields)
- [ ] LOCDATA accessor functions are callable from Rust (extern "C" declarations match C implementations)
- [ ] `read_locdata_from_c` correctly copies all scalar fields
- [ ] Borrowed pointer fields (callbacks, number speech) are stored as raw FFI-safe types
- [ ] Test: create CommData from mock LOCDATA data, verify all fields populated
- [ ] Test: CommData fields survive across encounter lifecycle (set, read back, clear)
- [ ] Test: `c_init_race` returns the same `LOCDATA*` the existing C dispatch would return for representative races and game-state-dependent cases
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/locdata.rs rust/src/comm/types.rs
```

## Success Criteria
- [ ] Full LOCDATA field parity in Rust CommData
- [ ] FFI accessors tested
- [ ] `init_race` layering is coherent and documented
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git restore rust/src/comm/types.rs rust/src/comm/mod.rs rust/src/comm/ffi.rs`
- blocking: LOCDATA struct layout must be verified in preflight

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P03.md`
