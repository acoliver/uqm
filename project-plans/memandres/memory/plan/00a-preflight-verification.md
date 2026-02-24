# Phase 00a: Preflight Verification

## Phase ID
`PLAN-20260224-MEM-SWAP.P00a`

## Purpose
Verify all assumptions about the codebase before implementation begins.

## Toolchain Verification
- [ ] `cargo --version` — Rust toolchain available
- [ ] `rustc --version` — compiler available
- [ ] `cargo clippy --version` — linter available
- [ ] C build toolchain works (`./build.sh uqm` or equivalent)

## Dependency Verification
- [ ] `libc` crate present in `rust/Cargo.toml`
- [ ] Rust static library (`libuqm_rust.a`) links into the C build

## Type/Interface Verification

### C Side
- [ ] `sc2/src/libs/memlib.h` exists and declares: `HMalloc`, `HFree`, `HCalloc`, `HRealloc`, `mem_init`, `mem_uninit`
- [ ] `sc2/src/libs/memory/w_memlib.c` exists and implements those 6 functions
- [ ] `sc2/src/libs/memory/Makeinfo` exists with `uqm_CFILES="w_memlib.c"`
- [ ] `sc2/config_unix.h` exists and contains other `USE_RUST_*` defines

### Rust Side
- [ ] `rust/src/memory.rs` exports: `rust_hmalloc`, `rust_hfree`, `rust_hcalloc`, `rust_hrealloc`, `rust_mem_init`, `rust_mem_uninit`
- [ ] All exports use `#[no_mangle] pub unsafe extern "C"`
- [ ] `rust/src/logging.rs` defines `LogLevel` enum with `User = 1`

### Pattern Verification
- [ ] Existing `USE_RUST_*` pattern confirmed in `config_unix.h` (e.g., `USE_RUST_FILE`, `USE_RUST_CLOCK`)
- [ ] `#error` guard pattern confirmed in at least one other `.c` file (e.g., `files.c`, `clock.c`)
- [ ] `Makeinfo` conditional pattern confirmed in at least one other module (e.g., `libs/file/Makeinfo`)
- [ ] Header macro redirect pattern confirmed (e.g., `rust_audiocore.h`, `rust_mixer.h`, `rust_vcontrol.h`)

### Call Site Safety
- [ ] No call site uses `&HMalloc` or passes `HMalloc` as a function pointer
- [ ] No call site uses `&HFree`, `&HCalloc`, or `&HRealloc` as function pointers
- [ ] All uses of `HMalloc`/`HFree`/`HCalloc`/`HRealloc` are direct call invocations `Name(args)`

## Test Infrastructure Verification
- [ ] `cargo test --workspace` passes (existing memory tests work)
- [ ] `rust/src/memory.rs` has `#[cfg(test)] mod tests` with at least 5 tests

## Verification Commands

```bash
# Toolchain
cargo --version
rustc --version
cargo clippy --version

# C files exist
test -f sc2/src/libs/memlib.h && echo "OK: memlib.h"
test -f sc2/src/libs/memory/w_memlib.c && echo "OK: w_memlib.c"
test -f sc2/src/libs/memory/Makeinfo && echo "OK: Makeinfo"
test -f sc2/config_unix.h && echo "OK: config_unix.h"

# Rust exports
grep -c 'pub unsafe extern "C" fn rust_hmalloc' rust/src/memory.rs
grep -c 'pub unsafe extern "C" fn rust_hfree' rust/src/memory.rs
grep -c 'pub unsafe extern "C" fn rust_hcalloc' rust/src/memory.rs
grep -c 'pub unsafe extern "C" fn rust_hrealloc' rust/src/memory.rs
grep -c 'pub unsafe extern "C" fn rust_mem_init' rust/src/memory.rs
grep -c 'pub unsafe extern "C" fn rust_mem_uninit' rust/src/memory.rs

# No function pointer usage
grep -rn '&HMalloc\|&HFree\|&HCalloc\|&HRealloc' sc2/src/ || echo "OK: no function pointer usage"

# Existing pattern references
grep -c '#error' sc2/src/libs/file/files.c
grep -c 'USE_RUST_FILE' sc2/src/libs/file/Makeinfo
grep -c 'USE_RUST_' sc2/config_unix.h

# Rust tests
cargo test --workspace --all-features 2>&1 | tail -5
```

## Blocking Issues
None expected — all patterns are already established in the codebase.

## Gate Decision
- [ ] PASS: proceed to Phase 01
- [ ] FAIL: revise plan — document specific blocker
