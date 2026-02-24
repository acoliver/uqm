# Phase 06a: C Header Redirect — Stub Verification

## Phase ID
`PLAN-20260224-MEM-SWAP.P06a`

## Prerequisites
- Required: Phase 06 completed
- All 6 C-side/build-system files modified

## Verification Checks

### Structural
- [ ] `memlib.h` has `#ifdef USE_RUST_MEM` with 6 `extern` + 6 `#define` directives
- [ ] `memlib.h` has `#else` with original 6 `extern` declarations
- [ ] `memlib.h` has `#endif` closing the block
- [ ] `w_memlib.c` has `#ifdef USE_RUST_MEM` / `#error` before any code
- [ ] `Makeinfo` has conditional shell logic for `USE_RUST_MEM` / `uqm_USE_RUST_MEM`
- [ ] `config_unix.h` has `/* #define USE_RUST_MEM */` (commented out)
- [ ] `build.vars.in` has `USE_RUST_MEM`/`uqm_USE_RUST_MEM`/`SYMBOL_USE_RUST_MEM_DEF` entries + export lines
- [ ] `config_unix.h.in` has `@SYMBOL_USE_RUST_MEM_DEF@` after existing `USE_RUST_*` block

### Semantic — C Path (flag OFF)
- [ ] Full build succeeds: `cd sc2 && ./build.sh uqm`
- [ ] `w_memlib.c` is compiled (appears in build output or object files)
- [ ] No warnings introduced by the header changes

### Semantic — Rust Path Readiness
- [ ] `cargo test --workspace --all-features` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --all --check` passes

### Pattern Conformance
- [ ] `memlib.h` `#ifdef` block matches pattern in `rust_audiocore.h` or `rust_mixer.h`
- [ ] `w_memlib.c` `#error` matches pattern in `files.c` or `clock.c`
- [ ] `Makeinfo` conditional matches pattern in `libs/file/Makeinfo`

## Verification Commands

```bash
# Build with flag off
cd sc2 && ./build.sh uqm

# Check file contents
grep -A 20 'USE_RUST_MEM' sc2/src/libs/memlib.h
grep 'USE_RUST_MEM' sc2/src/libs/memory/w_memlib.c
cat sc2/src/libs/memory/Makeinfo
grep 'USE_RUST_MEM' sc2/config_unix.h
grep 'USE_RUST_MEM' sc2/build.vars.in
grep 'USE_RUST_MEM' sc2/src/config_unix.h.in

# Rust checks
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Gate Decision
- [ ] PASS: proceed to Phase 07
- [ ] FAIL: fix issues in Phase 06
