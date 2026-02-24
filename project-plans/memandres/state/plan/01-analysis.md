# Phase 01: Analysis

## Phase ID
`PLAN-20260224-STATE-SWAP.P01`

## Prerequisites
- Required: Phase P00.5 (Preflight Verification) completed
- All preflight checks passed
- No blocking issues identified

## Requirements Implemented (Expanded)

This phase produces analysis artifacts — no code changes.

### REQ-SF-001: Seek-Past-End Allowed
**Requirement text**: `StateFile::seek` shall allow the cursor to be set to any non-negative value without upper-bound clamping.

Behavior contract:
- GIVEN: A state file with 100 bytes of data
- WHEN: `seek(1000000, SEEK_SET)` is called
- THEN: Cursor is set to 1000000; no error, no clamp

Why it matters:
- `grpinfo.c` computes offsets that may temporarily exceed buffer size before writes extend it.

### REQ-SF-005: Separate Used and Physical Size Tracking
**Requirement text**: `StateFile` shall track `used` (logical high-water mark) and physical allocation size separately.

Behavior contract:
- GIVEN: A state file opened with "wb" mode
- WHEN: Buffer is allocated to size_hint but nothing written
- THEN: `length()` returns 0 (used), but reads can access up to size_hint bytes (physical)

Why it matters:
- C `ReadStateFile` checks against physical size (`fp->size`), not logical size (`fp->used`).
- C `LengthStateFile` returns `fp->used`, not `fp->size`.
- Current Rust conflates both as `data.len()`.

### REQ-SF-004: Copy Deadlock Prevention
**Requirement text**: `rust_copy_game_state` shall not deadlock when source and destination are the same global state.

Behavior contract:
- GIVEN: `GLOBAL_GAME_STATE` contains a `GameState` behind a `Mutex`
- WHEN: `rust_copy_game_state` is called
- THEN: The function completes without blocking; mutex is acquired exactly once

Why it matters:
- `load_legacy.c` calls `copyGameState(dest, target, src, begin, end)` where `dest == src == GLOBAL(GameState)`.
- The Rust FFI wrapper calls through to the global, which deadlocks on double lock.

## Implementation Tasks

### Files to create
- `project-plans/memandres/state/analysis/domain-model.md` — entity/state analysis
  - marker: `@plan PLAN-20260224-STATE-SWAP.P01`
  - marker: `@requirement REQ-SF-001, REQ-SF-004, REQ-SF-005`

### Analysis outputs
1. **Entity/state model**: StateFile fields, lifecycle, used-vs-physical distinction
2. **Edge/error handling map**: seek negative, read EOF, write allocation failure, open_count underflow
3. **Integration touchpoints**: state.c 7 functions, sread_*/swrite_* helpers, grpinfo.c patterns
4. **Old code to replace/remove**: state.c function bodies (guarded, not deleted), StateFile::seek clamping, ffi.rs double-lock pattern

## Verification Commands

```bash
# Analysis is documentation-only — verify files exist
ls -la project-plans/memandres/state/analysis/domain-model.md
```

## Structural Verification Checklist
- [ ] `analysis/domain-model.md` created
- [ ] Domain model covers all 3 state file types
- [ ] Domain model documents used-vs-physical size distinction
- [ ] Domain model documents seek-past-end behavior
- [ ] Domain model documents save/load flow
- [ ] Domain model documents copy deadlock analysis
- [ ] Domain model documents open_count type issue

## Semantic Verification Checklist (Mandatory)
- [ ] All requirements (REQ-SF-001 through REQ-SF-009) represented in analysis
- [ ] Integration touchpoints (grpinfo.c, save.c, load.c, load_legacy.c) documented
- [ ] Known blockers (seek clamp, copy deadlock) analyzed with root cause

## Success Criteria
- [ ] Analysis artifacts complete and accurate
- [ ] All requirements traceable to analysis

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P01.md`

Contents:
- phase ID: P01
- files created: `analysis/domain-model.md`
- analysis covers: 3 state file types, buffer model, seek-past-end, save/load, deadlock, open_count
