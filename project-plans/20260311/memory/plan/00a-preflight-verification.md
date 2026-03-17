# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260314-MEMORY.P00.5`

## Purpose
Verify assumptions about the current codebase, toolchain, and integration state before implementing any gap-closure changes.

## Toolchain Verification
- [ ] `cargo --version`
- [ ] `rustc --version`
- [ ] `cargo clippy --version`

Coverage gate is not required for this plan — the memory subsystem is a thin wrapper and coverage tooling verification is not a blocking prerequisite.

## Dependency Verification
- [ ] `libc` crate is present in `rust/Cargo.toml`
- [ ] `std::ffi::CString` is available (standard library — always present)
- [ ] No additional crate dependencies are needed for the gap-closure work

## Type/Interface Verification
- [ ] `rust_hmalloc`, `rust_hfree`, `rust_hcalloc`, `rust_hrealloc` exist in `rust/src/memory.rs` with correct `extern "C"` signatures
- [ ] `rust_mem_init`, `rust_mem_uninit` exist in `rust/src/memory.rs` with `extern "C"` signatures returning `bool`
- [ ] `copy_argv_to_c` exists in `rust/src/memory.rs` as a `pub unsafe fn`
- [ ] `LogLevel::Fatal` and `LogLevel::Info` are available from `crate::logging`
- [ ] `log_add` function signature accepts `LogLevel` and `&str`

## Build/Test Baseline Verification
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace --all-features` passes
- [ ] All existing memory tests pass: `test_hmalloc_hfree`, `test_hcalloc`, `test_hrealloc`, `test_zero_size_allocations`, `test_copy_argv_to_c`

## Integration-Test Harness Verification
- [ ] `rust/Cargo.toml` library crate name is confirmed for integration-test imports
- [ ] `rust/src/lib.rs` publicly exposes `pub mod memory;`
- [ ] `rust/tests/` is an active integration-test directory for this crate layout
- [ ] The verified import path for integration tests is recorded (`uqm_rust::memory::*`, not package name by assumption)
- [ ] The verified invocation for the planned integration test is recorded (`cd rust && cargo test -p uqm --test memory_integration -- --nocapture` or equivalent confirmed command)
- [ ] No feature flag is required for `memory` module visibility in integration tests

## Integration State Verification
- [ ] `USE_RUST_MEM` is defined in `sc2/config_unix.h`
- [ ] `memlib.h` macro remapping is intact (lines 30-44)
- [ ] `w_memlib.c` has `#error` guard on line 1-2
- [ ] `rust/src/main.rs` calls `memory::rust_mem_init()` and `memory::rust_mem_uninit()`
- [ ] `rust/src/sound/heart_ffi.rs` calls `crate::memory::rust_hmalloc` and `crate::memory::rust_hfree`

## Verification Commands

```bash
# Toolchain
cargo --version
rustc --version
cargo clippy --version

# Build and test baseline
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Confirm USE_RUST_MEM is active
grep -n "USE_RUST_MEM" sc2/config_unix.h

# Confirm memlib.h remapping
grep -n "rust_hmalloc\|rust_hfree\|rust_hcalloc\|rust_hrealloc" sc2/src/libs/memlib.h

# Confirm w_memlib.c guard
head -3 sc2/src/libs/memory/w_memlib.c

# Confirm launcher integration
grep -n "rust_mem_init\|rust_mem_uninit" rust/src/main.rs

# Confirm heart_ffi integration
grep -n "crate::memory::rust_hmalloc\|crate::memory::rust_hfree" rust/src/sound/heart_ffi.rs

# Confirm integration-test harness assumptions
grep -n "^name = \"uqm\"\|^name = \"uqm_rust\"" rust/Cargo.toml
grep -n "pub mod memory;" rust/src/lib.rs
ls -la rust/tests
```

## Blocking Issues
None expected — the subsystem is already ported and passing tests.

## Gate Decision
- [ ] PASS: proceed to Phase 01
- [ ] FAIL: revise plan (list issues below)
