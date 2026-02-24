# Phase 09a: C Redirect — Stub Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P09a`

## Prerequisites
- Required: Phase P09 completed
- Expected: USE_RUST_STATE flag in config_unix.h, #ifdef in state.c, FFI header

## Structural Verification
- [ ] `config_unix.h` has `/* #define USE_RUST_STATE */` (commented out)
- [ ] `state.c` has `#ifdef USE_RUST_STATE` wrapping replacement functions
- [ ] `state.c` has `#else` section containing original C implementations
- [ ] `rust_state_ffi.h` exists with 7 function declarations
- [ ] All 7 functions redirected: Open, Close, Delete, Length, Read, Write, Seek

## Build Verification
```bash
# Default build (USE_RUST_STATE not defined) must succeed
cd sc2 && make clean && make
```
- [ ] Build succeeds with C path

## Semantic Verification
- [ ] OpenStateFile redirect returns `&state_files[stateFile]` (not NULL on success)
- [ ] CloseStateFile redirect computes `(int)(fp - state_files)`
- [ ] ReadStateFile redirect computes index and passes buf/size/count
- [ ] WriteStateFile redirect computes index and passes buf/size/count
- [ ] SeekStateFile redirect casts offset to int64_t
- [ ] LengthStateFile redirect casts return to DWORD
- [ ] DeleteStateFile redirect passes stateFile directly

## Gate Decision
- [ ] PASS: proceed to P10
- [ ] FAIL: fix redirect scaffolding
