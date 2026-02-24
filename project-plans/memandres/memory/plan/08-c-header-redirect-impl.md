# Phase 08: C Header Redirect — Implementation

## Phase ID
`PLAN-20260224-MEM-SWAP.P08`

## Prerequisites
- Required: Phase 07a (TDD Verification) completed
- Both build paths tested and working
- Expected files: `memlib.h`, `w_memlib.c`, `Makeinfo`, `config_unix.h` all modified

## Requirements Implemented (Expanded)

### REQ-MEM-004: Config Flag (Enable)
**Requirement text**: Uncomment `#define USE_RUST_MEM` in `config_unix.h` to permanently enable the Rust memory path.

Behavior contract:
- GIVEN: `config_unix.h` has `/* #define USE_RUST_MEM */` (commented out)
- WHEN: The comment is removed, making it `#define USE_RUST_MEM`
- THEN: All subsequent builds use the Rust memory functions via macro redirect

Why it matters:
- This is the actual swap — the single-line change that routes all 322+ call sites to Rust

### REQ-MEM-001: Header Macro Redirect (Active)
**Requirement text**: With `USE_RUST_MEM` now defined, the `memlib.h` macros are active.

Behavior contract:
- GIVEN: `USE_RUST_MEM` is defined in `config_unix.h`
- WHEN: Any C file includes `memlib.h` and calls `HMalloc(size)`
- THEN: The preprocessor expands it to `rust_hmalloc(size)`, linking to the Rust function

### REQ-MEM-003: Build System Conditional (Active)
**Requirement text**: With `USE_RUST_MEM` defined, `Makeinfo` excludes `w_memlib.c`.

Behavior contract:
- GIVEN: `USE_RUST_MEM` or `uqm_USE_RUST_MEM` is set
- WHEN: The build system processes `libs/memory/Makeinfo`
- THEN: `uqm_CFILES=""` — `w_memlib.c` is not compiled

## Implementation Tasks

### Files to modify

1. **`sc2/config_unix.h`**
   - Uncomment `#define USE_RUST_MEM` (remove the `/* */` comment delimiters)
   - marker: `@plan PLAN-20260224-MEM-SWAP.P08`
   - marker: `@requirement REQ-MEM-004`

### Pseudocode traceability
- Uses pseudocode lines: 50-53 (config flag — now active)

## Verification Commands

```bash
# Verify flag is now active
grep 'USE_RUST_MEM' sc2/config_unix.h

# Full clean build with Rust memory
cd sc2 && ./build.sh uqm

# Verify w_memlib.c is NOT compiled
# (check build log for w_memlib.c — should be absent)

# Rust checks
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `config_unix.h` has `#define USE_RUST_MEM` (uncommented)
- [ ] Full build succeeds
- [ ] `w_memlib.c` is not compiled (not in build output)
- [ ] No linker errors
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist (Mandatory)
- [ ] Build produces a working binary
- [ ] Binary uses Rust memory functions (verify via log output from `rust_mem_init`)
- [ ] No new warnings
- [ ] No placeholder/deferred implementation patterns remain
- [ ] Integration is complete: all 322+ call sites now route to Rust

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" sc2/src/libs/memlib.h sc2/src/libs/memory/w_memlib.c sc2/config_unix.h
```

## Success Criteria
- [ ] `USE_RUST_MEM` is defined (uncommented)
- [ ] Full build succeeds
- [ ] All verification commands pass

## Failure Recovery
- Rollback: Comment out `#define USE_RUST_MEM` in `config_unix.h` → rebuild
  ```bash
  # In config_unix.h, change:
  #   #define USE_RUST_MEM
  # to:
  #   /* #define USE_RUST_MEM */
  cd sc2 && ./build.sh uqm
  ```
- Blocking issues:
  - Linker error: verify `rust_hmalloc` etc. are in `libuqm_rust.a` (`nm libuqm_rust.a | grep rust_hmalloc`)
  - Build system not propagating flag: check `build.sh` and `Makefile` for `USE_RUST_MEM` propagation

## Phase Completion Marker
Create: `project-plans/memandres/memory/.completed/P08.md`

Contents:
- phase ID
- timestamp
- files changed: `sc2/config_unix.h`
- tests added/updated: none
- verification outputs (build log, grep outputs)
- semantic verification summary
