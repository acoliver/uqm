# Phase 06: C Header Redirect — Stub

## Phase ID
`PLAN-20260224-MEM-SWAP.P06`

## Prerequisites
- Required: Phase 05a (Rust Fixes Verification) completed
- Rust side is ready: `LogLevel::Fatal` alias exists, `memory.rs` uses it
- Expected files from previous phase: modified `rust/src/logging.rs`, `rust/src/memory.rs`

## Requirements Implemented (Expanded)

### REQ-MEM-001: Header Macro Redirect (Stub)
**Requirement text**: Add `#ifdef USE_RUST_MEM` block to `memlib.h` with `extern` declarations for `rust_*` functions and `#define` macros redirecting `HMalloc`→`rust_hmalloc`, etc.

Behavior contract:
- GIVEN: `memlib.h` currently has plain `extern` declarations for C functions
- WHEN: `#ifdef USE_RUST_MEM` block is added
- THEN: When `USE_RUST_MEM` is defined, macros redirect all 6 functions to Rust; when undefined, original declarations remain

Why it matters:
- This is the core mechanism — 322+ call sites are redirected without modification

### REQ-MEM-002: C Source Guard (Stub)
**Requirement text**: Add `#ifdef USE_RUST_MEM` / `#error` at the top of `w_memlib.c`.

Behavior contract:
- GIVEN: `w_memlib.c` currently compiles unconditionally
- WHEN: `#ifdef USE_RUST_MEM` / `#error` is added at the top
- THEN: If `w_memlib.c` is accidentally compiled with `USE_RUST_MEM` defined, the build fails with a clear error message

Why it matters:
- Prevents accidental double-linking of C and Rust memory functions
- Follows established pattern (`files.c`, `clock.c`, `io.c`, `vcontrol.c`)

### REQ-MEM-003: Build System Conditional (Stub)
**Requirement text**: Update `Makeinfo` to conditionally exclude `w_memlib.c` when `USE_RUST_MEM` is set.

Behavior contract:
- GIVEN: `Makeinfo` currently always sets `uqm_CFILES="w_memlib.c"`
- WHEN: Conditional is added checking `USE_RUST_MEM` or `uqm_USE_RUST_MEM`
- THEN: When flag is set, `uqm_CFILES=""` (C file excluded); when unset, original behavior preserved

Why it matters:
- Without this, the `#error` in `w_memlib.c` would fire even when we want to use Rust

### REQ-MEM-004: Config Flag (Stub — initially commented out)
**Requirement text**: Add `USE_RUST_MEM` to `config_unix.h`, initially commented out so the C path remains active during stub phase.

Behavior contract:
- GIVEN: `config_unix.h` has other `USE_RUST_*` flags all defined (uncommented)
- WHEN: `/* #define USE_RUST_MEM */` is added
- THEN: The flag exists but is inactive; C path compiles as before

Why it matters:
- Stub phase must not break the existing build
- Flag is enabled in Phase 08 (Implementation)

## Implementation Tasks

### Files to modify

1. **`sc2/src/libs/memlib.h`**
   - Wrap existing declarations in `#ifdef USE_RUST_MEM` / `#else` / `#endif`
   - Add `extern` declarations for `rust_hmalloc`, `rust_hfree`, `rust_hcalloc`, `rust_hrealloc`, `rust_mem_init`, `rust_mem_uninit` in the `#ifdef` block
   - Add `#define` macros: `HMalloc(s)`, `HFree(p)`, `HCalloc(s)`, `HRealloc(p, s)`, `mem_init()`, `mem_uninit()`
   - Keep original declarations in `#else` block
   - marker: `@plan PLAN-20260224-MEM-SWAP.P06`
   - marker: `@requirement REQ-MEM-001`

2. **`sc2/src/libs/memory/w_memlib.c`**
   - Add at top (after copyright, before includes):
     ```c
     #ifdef USE_RUST_MEM
     #error "w_memlib.c should not be compiled when USE_RUST_MEM is enabled"
     #endif
     ```
   - marker: `@plan PLAN-20260224-MEM-SWAP.P06`
   - marker: `@requirement REQ-MEM-002`

3. **`sc2/src/libs/memory/Makeinfo`**
   - Replace unconditional `uqm_CFILES="w_memlib.c"` with conditional:
     ```sh
     if [ "$USE_RUST_MEM" = "1" ] || [ "$uqm_USE_RUST_MEM" = "1" ]; then
         uqm_CFILES=""
     else
         uqm_CFILES="w_memlib.c"
     fi
     ```
   - marker: `@plan PLAN-20260224-MEM-SWAP.P06`
   - marker: `@requirement REQ-MEM-003`

4. **`sc2/config_unix.h`**
   - Add commented-out flag after existing `USE_RUST_*` block:
     ```c
     /* Defined if using Rust memory allocator */
     /* #define USE_RUST_MEM */
     ```
   - marker: `@plan PLAN-20260224-MEM-SWAP.P06`
   - marker: `@requirement REQ-MEM-004`

5. **`sc2/build.vars.in`**
   - Add `USE_RUST_MEM` entries following the exact pattern of `USE_RUST_FILE`:
     - In the `uqm_USE_RUST_*` block (after `uqm_USE_RUST_MIXER`): add `uqm_USE_RUST_MEM='@USE_RUST_MEM@'`
     - In the `USE_RUST_*` block (after `USE_RUST_MIXER`): add `USE_RUST_MEM='@USE_RUST_MEM@'`
     - In the `uqm_USE_RUST_*` export line: append `uqm_USE_RUST_MEM`
     - In the `USE_RUST_*` export line: append `USE_RUST_MEM`
     - In the `uqm_SYMBOL_*_DEF` block (after `uqm_SYMBOL_USE_RUST_MIXER_DEF`): add `uqm_SYMBOL_USE_RUST_MEM_DEF='@SYMBOL_USE_RUST_MEM_DEF@'`
     - In the `SYMBOL_*_DEF` block (after `SYMBOL_USE_RUST_MIXER_DEF`): add `SYMBOL_USE_RUST_MEM_DEF='@SYMBOL_USE_RUST_MEM_DEF@'`
     - In the `uqm_SYMBOL_*_DEF` export line: append `uqm_SYMBOL_USE_RUST_MEM_DEF`
     - In the `SYMBOL_*_DEF` export line: append `SYMBOL_USE_RUST_MEM_DEF`
   - marker: `@plan PLAN-20260224-MEM-SWAP.P06`
   - marker: `@requirement REQ-MEM-004`

6. **`sc2/src/config_unix.h.in`**
   - Add `@SYMBOL_USE_RUST_MEM_DEF@` line after the existing `USE_RUST_*` block (after `@SYMBOL_USE_RUST_MIXER_DEF@`):
     ```c
     /* Defined if using Rust memory allocator */
     @SYMBOL_USE_RUST_MEM_DEF@
     ```
   - marker: `@plan PLAN-20260224-MEM-SWAP.P06`
   - marker: `@requirement REQ-MEM-004`

### Pseudocode traceability
- Uses pseudocode lines: 01-26 (header redirect), 30-34 (source guard), 40-45 (Makeinfo), 50-53 (config flag)

## Verification Commands

```bash
# C build with USE_RUST_MEM commented out (C path — should succeed)
cd sc2 && ./build.sh uqm

# Rust side still passes
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Structural checks
grep 'USE_RUST_MEM' sc2/src/libs/memlib.h
grep 'USE_RUST_MEM' sc2/src/libs/memory/w_memlib.c
grep 'USE_RUST_MEM' sc2/src/libs/memory/Makeinfo
grep 'USE_RUST_MEM' sc2/config_unix.h
grep 'USE_RUST_MEM' sc2/build.vars.in
grep 'USE_RUST_MEM' sc2/src/config_unix.h.in
```

## Structural Verification Checklist
- [ ] `memlib.h` has `#ifdef USE_RUST_MEM` block with 6 extern declarations + 6 macros + `#else` block with original declarations
- [ ] `w_memlib.c` has `#ifdef USE_RUST_MEM` / `#error` at top
- [ ] `Makeinfo` has conditional checking `USE_RUST_MEM` / `uqm_USE_RUST_MEM`
- [ ] `config_unix.h` has commented-out `USE_RUST_MEM` define
- [ ] `build.vars.in` has `USE_RUST_MEM`/`uqm_USE_RUST_MEM`/`SYMBOL_USE_RUST_MEM_DEF` entries + export lines
- [ ] `config_unix.h.in` has `@SYMBOL_USE_RUST_MEM_DEF@` after existing `USE_RUST_*` block
- [ ] Build succeeds with the flag commented out (C path active)
- [ ] Plan/requirement traceability present in all modified files

## Semantic Verification Checklist (Mandatory)
- [ ] C path still works — build compiles and links with C memory functions
- [ ] No behavioral change — game would run identically (flag is off)
- [ ] `#ifdef` / `#else` / `#endif` structure is correct in `memlib.h`
- [ ] `#error` message is clear and follows pattern of other guarded files
- [ ] Makeinfo conditional matches pattern from `libs/file/Makeinfo`

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" sc2/src/libs/memlib.h sc2/src/libs/memory/w_memlib.c sc2/src/libs/memory/Makeinfo sc2/build.vars.in sc2/src/config_unix.h.in
```

## Success Criteria
- [ ] All 6 files modified correctly
- [ ] Build succeeds with flag off (C path)
- [ ] All verification commands pass

## Failure Recovery
- Rollback:
  ```bash
  git checkout sc2/src/libs/memlib.h sc2/src/libs/memory/w_memlib.c sc2/src/libs/memory/Makeinfo sc2/config_unix.h sc2/build.vars.in sc2/src/config_unix.h.in
  ```
- Blocking issues: if build fails with flag off, the `#else` branch has an error

## Phase Completion Marker
Create: `project-plans/memandres/memory/.completed/P06.md`

Contents:
- phase ID
- timestamp
- files changed: `memlib.h`, `w_memlib.c`, `Makeinfo`, `config_unix.h`, `build.vars.in`, `config_unix.h.in`
- tests added/updated: none (C-side changes)
- verification outputs (build log)
- semantic verification summary
