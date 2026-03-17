# Phase 05b: Trackplayer C Wrapper Seam

## Phase ID
`PLAN-20260314-COMM.P05b`

## Prerequisites
- Required: Phase 05a completed
- Expected: FFI signatures corrected and validated; preflight/analysis confirmed the authoritative trackplayer owner boundary and the concrete backing C symbols in `sc2/src/libs/sound/trackplayer.c`

## Purpose

Create the concrete C-side wrapper seam that Phase 06 consumes. This phase owns the bridge layer in `sc2/src/uqm/rust_comm.c` / `sc2/src/uqm/rust_comm.h` so Rust comm can call authoritative trackplayer behavior without depending on direct legacy symbol shapes or ad hoc wrapper creation inside later phases.

## Requirements Implemented (Expanded)

### TP-REQ-001 through TP-REQ-013 / SS-REQ-001 through SS-REQ-017 / IN-REQ-001 through IN-REQ-003
This phase does not implement trackplayer semantics itself; it makes those semantics callable at the comm integration seam by exposing a stable wrapper API backed by the authoritative existing C implementation.

## Implementation Tasks

### Files to modify

- `sc2/src/uqm/rust_comm.c`
  - marker: `@plan PLAN-20260314-COMM.P05b`
  - Add concrete wrapper functions backed by `sc2/src/libs/sound/trackplayer.c`:
    - `c_SpliceTrack`
    - `c_SpliceMultiTrack`
    - `c_PlayTrack`
    - `c_StopTrack`
    - `c_JumpTrack`
    - `c_PlayingTrack`
    - `c_GetTrackSubtitle`
    - `c_GetFirstTrackSubtitle`
    - `c_GetNextTrackSubtitle`
    - `c_GetTrackSubtitleText`
    - `c_FastForward_Page`
    - `c_FastForward_Smooth`
    - `c_FastReverse_Page`
    - `c_FastReverse_Smooth`
    - `c_PollPendingTrackCompletion`
    - `c_CommitTrackAdvancement`
    - `c_ReplayLastPhrase`
  - Keep wrappers thin: no new comm policy, only signature adaptation and stable bridge ownership

- `sc2/src/uqm/rust_comm.h`
  - marker: `@plan PLAN-20260314-COMM.P05b`
  - Declare the full wrapper surface consumed by `rust/src/comm/track.rs`
  - Ensure callback signatures and iterator/opaque-pointer types match the validated seam inventory

### Concrete ownership rule

- P05b is the only phase that owns first creation of the C-side trackplayer wrapper seam.
- P06 may consume these wrappers from Rust.
- P11 may extend the seam only for additional comm wiring not already created here, but must not re-own or defer the foundational wrapper surface.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd /Users/acoliver/projects/uqm/sc2 && make 2>&1 | tail -20
```

## Structural Verification Checklist
- [ ] `rust_comm.c` defines every trackplayer wrapper required by P06
- [ ] `rust_comm.h` declares every wrapper required by P06
- [ ] Each wrapper maps to an authoritative backing function in `sc2/src/libs/sound/trackplayer.c`
- [ ] No new comm policy logic introduced in the wrapper layer

## Semantic Verification Checklist (Mandatory)
- [ ] Wrapper callback signatures match the validated response/track callback ABI
- [ ] Wrapper surface covers queueing, playback control, subtitle access, history enumeration, pending completion, commit, replay, and fast-seek operations
- [ ] Full project build resolves the wrapper symbols before Phase 06 begins
- [ ] Phase text makes P06 a consumer of this seam rather than an implicit creator of it

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h
```

## Success Criteria
- [ ] The complete C-side trackplayer seam required by P06 exists and links
- [ ] Wrapper ownership/order is explicit and internally consistent

## Failure Recovery
- rollback: `git restore sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h`

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P05b.md`
