# Phase 06: Subtitle Display Bridge — Stub

## Phase ID
`PLAN-20260325-COMMPT3.P06`

## Prerequisites
- Required: Phase P05a (Colormap + Music Impl Verification) completed
- Expected: colormap/music bridges fully implemented and verified

## Requirements Addressed
- REQ-SD-001..005 (stubs only — behavior in P08)

## Purpose
Create compile-safe skeletons for the subtitle bridge functions in `comm.c`,
wire `rust_comm.c` to forward to them instead of routing back to Rust.

## Stub Tasks

### C-side stubs (comm.c, inside `#ifdef USE_RUST_COMM`)
- Add empty `comm_ClearSubtitles()` function (body: `/* stub — impl in P08 */`)
- Add empty `comm_CheckSubtitles()` function (body: `/* stub — impl in P08 */`)
- Add empty `comm_RedrawSubtitles()` function (body: `/* stub — impl in P08 */`)

### Declarations (rust_comm.h)
- Add declarations for `comm_ClearSubtitles()`, `comm_CheckSubtitles()`, `comm_RedrawSubtitles()`

### Bridge rewiring (rust_comm.c)
- Replace `c_ClearSubtitles` body: remove `rust_ClearSubtitles()` call →
  call `comm_ClearSubtitles()` instead
- Replace `c_CheckSubtitles` body: remove `rust_CheckSubtitles()` call →
  call `comm_CheckSubtitles()` instead
- Replace `c_RedrawSubtitles` body: remove `rust_RedrawSubtitles()` call →
  call `comm_RedrawSubtitles()` instead

### Allowed
- Empty C function bodies
- `/* stub */` comments

### Not Allowed
- Fake success behavior or return values
- Any subtitle rendering logic

## Pseudocode Traceability
- Bridge forwarding stubs: pseudocode `002-subtitle-display-fix.md` lines 39-48 (routing only)
- Function signatures: pseudocode `002-subtitle-display-fix.md` lines 50-54

## Traceability Markers (in code)
```c
/* @plan PLAN-20260325-COMMPT3.P06 */
/* @requirement REQ-SD-001 */
/* @pseudocode 002-subtitle-display-fix lines 39-54 */
```

## Implementation Tasks

### Files to modify
- `sc2/src/uqm/comm.c` — add 3 empty `comm_*` subtitle functions inside `#ifdef USE_RUST_COMM`
- `sc2/src/uqm/rust_comm.c` — rewire `c_*` subtitle bodies to call `comm_*` (not `rust_*`)
- `sc2/src/uqm/rust_comm.h` — add declarations for 3 `comm_*` functions

### Files to create
- None

## Verification Commands

```bash
# Build gate
cd rust && cargo check --workspace --all-features
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify circular routing broken
grep -n "rust_ClearSubtitles\|rust_CheckSubtitles\|rust_RedrawSubtitles" sc2/src/uqm/rust_comm.c | grep -v "extern\|declare\|//\|proto"
# Expected: zero matches in c_* function bodies

# Verify new functions in comm.c
grep -n "comm_ClearSubtitles\|comm_CheckSubtitles\|comm_RedrawSubtitles" sc2/src/uqm/comm.c

# Verify forwarding in rust_comm.c
grep -A3 "c_ClearSubtitles\|c_CheckSubtitles\|c_RedrawSubtitles" sc2/src/uqm/rust_comm.c | grep "comm_"

# Verify declarations
grep "comm_ClearSubtitles\|comm_CheckSubtitles\|comm_RedrawSubtitles" sc2/src/uqm/rust_comm.h
```

## Structural Verification Checklist
- [ ] `comm_ClearSubtitles()` exists in `comm.c` inside `#ifdef USE_RUST_COMM` (empty body)
- [ ] `comm_CheckSubtitles()` exists in `comm.c` inside `#ifdef USE_RUST_COMM` (empty body)
- [ ] `comm_RedrawSubtitles()` exists in `comm.c` inside `#ifdef USE_RUST_COMM` (empty body)
- [ ] All three declared in `rust_comm.h`
- [ ] `c_ClearSubtitles` in `rust_comm.c` calls `comm_ClearSubtitles()` (not `rust_ClearSubtitles`)
- [ ] `c_CheckSubtitles` in `rust_comm.c` calls `comm_CheckSubtitles()` (not `rust_CheckSubtitles`)
- [ ] `c_RedrawSubtitles` in `rust_comm.c` calls `comm_RedrawSubtitles()` (not `rust_RedrawSubtitles`)
- [ ] Both build modes compile and link

## Success Criteria
- [ ] Both build modes compile and link
- [ ] All existing tests pass (268+)
- [ ] Circular routing broken (rust_comm.c → comm.c, NOT back to Rust)
- [ ] No functional behavior in stubs

## Failure Recovery
- rollback: `git restore sc2/src/uqm/comm.c sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h`

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P06.md`
