# Phase 03: Colormap + Music Bridge — Stub

## Phase ID
`PLAN-20260325-COMMPT3.P03`

## Prerequisites
- Required: Phase P02a (Pseudocode Verification) completed
- Expected files from previous phase: all 5 pseudocode components verified

## Requirements Addressed
- REQ-CM-001, REQ-CM-002, REQ-MU-001, REQ-MU-002 (stubs only — behavior in P05)

## Purpose
Create compile-safe skeletons for the colormap and music bridge functions in
both C and Rust. Wire call sites to use the new stubs. No functional behavior.

## Stub Tasks

### C-side stubs (rust_comm.c / rust_comm.h)
- Add `c_SetColorMapFromCommData()` as an empty C function in `rust_comm.c`
  (body: `/* stub — impl in P05 */` comment only, no behavior)
- Add `c_PlayAlienMusic()` as an empty C function in `rust_comm.c`
  (body: `/* stub — impl in P05 */` comment only, no behavior)
- Add declarations for both in `rust_comm.h`

### Rust-side wiring (talk_segue.rs)
- Add extern declarations: `fn c_SetColorMapFromCommData()` and `fn c_PlayAlienMusic()`
- Replace `c_SetColorMap(std::ptr::null_mut())` → `c_SetColorMapFromCommData()` in `set_colormap()`
- Replace `c_PlayMusic(std::ptr::null_mut(), 1, 1)` → `c_PlayAlienMusic()` in `play_alien_music()`
- Remove old extern declarations (`c_SetColorMap`, `c_PlayMusic`) if no other
  callers exist (verify by grep first)

### Allowed
- Empty C function bodies
- `todo!()` / `unimplemented!()` in Rust if needed for compilation

### Not Allowed
- Fake success behavior
- Any rendering, audio, or resource-management logic

## Pseudocode Traceability
- `c_SetColorMapFromCommData` stub: pseudocode `001-colormap-music-bridges.md` lines 01-08 (structure only)
- `c_PlayAlienMusic` stub: pseudocode `001-colormap-music-bridges.md` lines 09-15 (structure only)
- Rust caller wiring: pseudocode `001-colormap-music-bridges.md` lines 16-31 (call replacement only)

## Traceability Markers (in code)
```rust
/// @plan PLAN-20260325-COMMPT3.P03
/// @requirement REQ-CM-001, REQ-MU-001
/// @pseudocode 001-colormap-music-bridges lines 01-31
```

## Implementation Tasks

### Files to modify
- `sc2/src/uqm/rust_comm.c` — add empty `c_SetColorMapFromCommData()` and `c_PlayAlienMusic()`
- `sc2/src/uqm/rust_comm.h` — add declarations
- `rust/src/comm/talk_segue.rs` — extern block update + call site rewiring

### Files to create
- None

## Verification Commands

```bash
# Gate: both build modes compile and link
cd rust && cargo check --workspace --all-features
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings

# Verify new extern declarations exist
grep "c_SetColorMapFromCommData\|c_PlayAlienMusic" rust/src/comm/talk_segue.rs

# Verify C stubs exist
grep -n "c_SetColorMapFromCommData\|c_PlayAlienMusic" sc2/src/uqm/rust_comm.c

# Verify old null_mut calls replaced
grep -n "null_mut" rust/src/comm/talk_segue.rs | grep -v test | grep -v "cfg(test)" | grep -v "///"
# Expected: zero matches in set_colormap/play_alien_music production paths

# Existing tests should still pass (stubs are no-ops, matching previous null behavior)
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `c_SetColorMapFromCommData()` exists in `rust_comm.c` (empty body)
- [ ] `c_PlayAlienMusic()` exists in `rust_comm.c` (empty body)
- [ ] Both declared in `rust_comm.h`
- [ ] Rust extern declarations for both new functions present
- [ ] `set_colormap()` calls `c_SetColorMapFromCommData()` (not `c_SetColorMap`)
- [ ] `play_alien_music()` calls `c_PlayAlienMusic()` (not `c_PlayMusic`)
- [ ] Project compiles with both `USE_RUST_COMM=on` and `=off`

## Success Criteria
- [ ] Both build modes compile and link
- [ ] All existing tests pass (268+)
- [ ] No functional behavior in stubs

## Failure Recovery
- rollback: `git restore rust/src/comm/talk_segue.rs sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h`

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P03.md`
