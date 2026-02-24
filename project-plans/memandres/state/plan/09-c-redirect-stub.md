# Phase 09: C Redirect — Stub

## Phase ID
`PLAN-20260224-STATE-SWAP.P09`

## Prerequisites
- Required: Phase P08a (Deadlock Fix Verification) completed
- Both Rust blockers (seek-past-end and copy deadlock) are fixed
- All Rust tests pass

## Requirements Implemented (Expanded)

### REQ-SF-006: C Redirect Correctness
**Requirement text**: When `USE_RUST_STATE` is defined, each of the 7 state file functions in `state.c` shall redirect to its Rust FFI equivalent.

Behavior contract:
- GIVEN: `USE_RUST_STATE` is defined in `config_unix.h`
- WHEN: C code calls `OpenStateFile(0, "wb")`
- THEN: The call is forwarded to `rust_open_state_file(0, "wb")` via FFI
- AND: The return value is translated from Rust success/failure to `GAME_STATE_FILE*`

### REQ-SF-007: Opaque Pointer Preservation
**Requirement text**: The `GAME_STATE_FILE*` returned by the redirect layer must be usable by all existing callers.

Behavior contract:
- GIVEN: Caller receives `GAME_STATE_FILE*` from `OpenStateFile`
- WHEN: Caller passes it to `CloseStateFile`, `ReadStateFile`, etc.
- THEN: The redirect layer recovers the file index from the pointer and calls the Rust FFI

### REQ-SF-009: Feature Flag Isolation
**Requirement text**: `USE_RUST_STATE` in `config_unix.h` controls whether Rust or C state file I/O is used. Default: disabled.

Behavior contract:
- GIVEN: `USE_RUST_STATE` is NOT defined
- WHEN: `OpenStateFile` is called
- THEN: Original C implementation is used (no Rust involvement)

## Implementation Tasks

### Files to modify

1. **`sc2/config_unix.h`**
   - Add `/* #define USE_RUST_STATE */` (commented out — disabled by default)
   - marker: `@plan PLAN-20260224-STATE-SWAP.P09`

2. **`sc2/src/uqm/state.c`**
   - Add `#ifdef USE_RUST_STATE` block at the top of the file (after includes)
   - Inside the block: include Rust FFI header, define redirect functions
   - The `#ifdef` wraps replacement implementations of all 7 functions
   - The original C implementations are in the `#else` block
   - marker: `@plan PLAN-20260224-STATE-SWAP.P09`
   - marker: `@requirement REQ-SF-006, REQ-SF-007`

3. **Create `sc2/src/uqm/rust_state_ffi.h`** (or use existing FFI header)
   - Declare `extern` Rust FFI functions:
     - `int rust_open_state_file(int file_index, const char *mode);`
     - `void rust_close_state_file(int file_index);`
     - `void rust_delete_state_file(int file_index);`
     - `size_t rust_length_state_file(int file_index);`
     - `size_t rust_read_state_file(int file_index, uint8_t *buf, size_t size, size_t count);`
     - `size_t rust_write_state_file(int file_index, const uint8_t *buf, size_t size, size_t count);`
     - `int rust_seek_state_file(int file_index, int64_t offset, int whence);`
   - marker: `@plan PLAN-20260224-STATE-SWAP.P09`

### Redirect approach (from pseudocode lines 1–48)

The key challenge is pointer ↔ index translation:
- `OpenStateFile` returns `&state_files[stateFile]` (actual pointer to static array entry)
- All other functions receive `GAME_STATE_FILE *fp` and compute `index = (int)(fp - state_files)`
- This works because `state_files` is a static array and all pointers point into it
- The static `state_files` array still exists (for pointer arithmetic) even when Rust handles the data

```c
#ifdef USE_RUST_STATE
#include "rust_state_ffi.h"

GAME_STATE_FILE *
OpenStateFile (int stateFile, const char *mode)
{
    if (stateFile < 0 || stateFile >= NUM_STATE_FILES)
        return NULL;
    if (!rust_open_state_file(stateFile, mode))
        return NULL;
    return &state_files[stateFile];
}

void
CloseStateFile (GAME_STATE_FILE *fp)
{
    int index = (int)(fp - state_files);
    rust_close_state_file(index);
}

// ... similar for Read, Write, Seek, Length, Delete
#else
// ... existing C implementations
#endif
```

### Stub phase deliverable
- `config_unix.h` has commented-out `USE_RUST_STATE`
- `state.c` has `#ifdef USE_RUST_STATE` block with all 7 redirects
- `rust_state_ffi.h` declares the Rust FFI functions
- Feature is DISABLED by default — no behavior change

## Verification Commands

```bash
# Build WITHOUT USE_RUST_STATE (default) — must succeed
cd sc2 && make clean && make
# Verify the new header exists
ls -la sc2/src/uqm/rust_state_ffi.h
# Verify state.c has the ifdef
grep -c "USE_RUST_STATE" sc2/src/uqm/state.c
```

## Structural Verification Checklist
- [ ] `config_unix.h` contains `/* #define USE_RUST_STATE */` (commented out)
- [ ] `state.c` has `#ifdef USE_RUST_STATE` / `#else` / `#endif` structure
- [ ] All 7 functions have redirect implementations in the `#ifdef` block
- [ ] `rust_state_ffi.h` declares all 7 Rust FFI functions
- [ ] Pointer-to-index translation uses `(int)(fp - state_files)`
- [ ] Build succeeds with `USE_RUST_STATE` NOT defined

## Semantic Verification Checklist (Mandatory)
- [ ] With `USE_RUST_STATE` undefined: original C behavior unchanged
- [ ] The static `state_files` array is available in both `#ifdef` branches
- [ ] `OpenStateFile` redirect returns actual `GAME_STATE_FILE*` (pointer to static entry)
- [ ] Other redirects compute index from pointer arithmetic
- [ ] `DeleteStateFile` redirect passes `stateFile` directly (already index-based)
- [ ] Build passes with existing C path (no regressions)

## Success Criteria
- [ ] Feature flag added (disabled)
- [ ] Redirect scaffolding in place
- [ ] Default build succeeds (C path)
- [ ] No behavior change when flag is off

## Failure Recovery
- rollback: `git checkout -- sc2/config_unix.h sc2/src/uqm/state.c`
- if new header causes issues: `rm sc2/src/uqm/rust_state_ffi.h`

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P09.md`

Contents:
- phase ID: P09
- files created: `sc2/src/uqm/rust_state_ffi.h`
- files modified: `sc2/config_unix.h`, `sc2/src/uqm/state.c`
- changes: USE_RUST_STATE flag (disabled), #ifdef redirect scaffolding, FFI header
- build: default build succeeds with C path
