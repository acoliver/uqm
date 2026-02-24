# Phase 07: C Header Redirect — TDD

## Phase ID
`PLAN-20260224-MEM-SWAP.P07`

## Prerequisites
- Required: Phase 06a (Stub Verification) completed
- All C-side changes in place, build succeeds with flag off
- Expected files from previous phase: modified `memlib.h`, `w_memlib.c`, `Makeinfo`, `config_unix.h`

## Requirements Implemented (Expanded)

### REQ-MEM-007: Build Both Paths (Test)
**Requirement text**: Verify that both the C path (flag off) and Rust path (flag on) compile and link successfully.

Behavior contract — C path:
- GIVEN: `USE_RUST_MEM` is NOT defined (commented out in `config_unix.h`)
- WHEN: Full build is executed
- THEN: Build succeeds, `w_memlib.c` is compiled, C `HMalloc` etc. are linked

Behavior contract — Rust path:
- GIVEN: `USE_RUST_MEM` IS defined (uncommented in `config_unix.h`)
- WHEN: Full build is executed
- THEN: Build succeeds, `w_memlib.c` is NOT compiled, Rust `rust_hmalloc` etc. are linked via macros

Why it matters:
- Confirms the swap is reversible
- Catches linker errors before the final enable

### REQ-MEM-002: C Source Guard (Test)
**Requirement text**: Verify the `#error` in `w_memlib.c` fires when the flag is defined and `w_memlib.c` is forcibly compiled.

Behavior contract:
- GIVEN: `USE_RUST_MEM` is defined
- WHEN: `w_memlib.c` is compiled directly (bypassing Makeinfo)
- THEN: Compilation fails with error "w_memlib.c should not be compiled when USE_RUST_MEM is enabled"

Why it matters:
- Safety net against build system misconfiguration

## Implementation Tasks

### Build Tests (manual verification — no automated test framework for C builds)

1. **Test C path**: Build with `USE_RUST_MEM` commented out
   - Clean build: `cd sc2 && ./build.sh uqm`
   - Verify success

2. **Test Rust path**: Temporarily uncomment `USE_RUST_MEM` in `config_unix.h`, build
   - Edit `config_unix.h`: uncomment `#define USE_RUST_MEM`
   - Clean build: `cd sc2 && ./build.sh uqm`
   - Verify success
   - Re-comment the flag (leave it off for this phase — enable permanently in P08)

3. **Test #error guard**: With `USE_RUST_MEM` defined, try to compile `w_memlib.c` directly
   - `gcc -DUSE_RUST_MEM -I sc2/src -c sc2/src/libs/memory/w_memlib.c -o /dev/null`
   - Verify compiler error mentioning "should not be compiled"

### Pseudocode traceability
- Uses pseudocode lines: 01-26 (header redirect — tested via build), 30-34 (source guard — tested via direct compile), 40-45 (Makeinfo — tested via build)

## Verification Commands

```bash
# Test 1: C path (flag off)
cd sc2 && ./build.sh uqm
echo "C path build: $?"

# Test 2: Rust path (flag on) — temporarily uncomment flag
# (manually uncomment USE_RUST_MEM in config_unix.h first)
cd sc2 && ./build.sh uqm
echo "Rust path build: $?"
# (re-comment USE_RUST_MEM after test)

# Test 3: #error guard
gcc -DUSE_RUST_MEM -I sc2/src -c sc2/src/libs/memory/w_memlib.c -o /dev/null 2>&1 | grep -i "error"

# Rust tests still pass
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] C path build tested and succeeds
- [ ] Rust path build tested and succeeds
- [ ] `#error` guard tested and fires correctly
- [ ] Flag re-commented after Rust path test

## Semantic Verification Checklist (Mandatory)
- [ ] C path produces a working binary (game can launch)
- [ ] Rust path produces a working binary (game can launch)
- [ ] `#error` message is clear and helpful
- [ ] No linker errors in either path
- [ ] No warnings introduced in either path

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" sc2/src/libs/memlib.h sc2/src/libs/memory/w_memlib.c
```

## Success Criteria
- [ ] Both build paths succeed
- [ ] `#error` guard works
- [ ] Flag is re-commented (still off after this phase)

## Failure Recovery
- Rollback:
  ```bash
  git checkout sc2/config_unix.h  # restore commented-out flag
  ```
- Blocking issues:
  - Linker error on Rust path: check that `rust_hmalloc` etc. are exported from `libuqm_rust.a`
  - Build system doesn't pass `USE_RUST_MEM` to Makeinfo: check `build.sh` variable propagation

## Phase Completion Marker
Create: `project-plans/memandres/memory/.completed/P07.md`

Contents:
- phase ID
- timestamp
- files changed: none (or `config_unix.h` temporarily toggled)
- tests executed: 3 build tests
- verification outputs (build logs)
- semantic verification summary
