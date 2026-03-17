# Phase 03a: Return Value Propagation — Stub Verification

## Phase ID
`PLAN-20260314-THREADING.P03a`

## Prerequisites
- Required: Phase 03 completed

## Structural Verification
- [ ] `spawn_c_thread` returns `Result<Thread<c_int>>`
- [ ] Closure in `spawn_c_thread` captures `func(data)` return value (no `let _ =`)
- [ ] `rust_thread_spawn` works with `Thread<c_int>` handle
- [ ] `rust_thread_spawn_detached` works with `Thread<c_int>` handle
- [ ] `rust_thread_join` casts to `Thread<c_int>` internally
- [ ] `rust_thread_join` external signature UNCHANGED (`(thread: *mut RustThread) -> c_int`)

## Compilation Gate
```bash
cd /Users/acoliver/projects/uqm/rust
cargo build --lib --all-features 2>&1 | tail -5
```
- [ ] Compiles with zero errors

## Test Gate
```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --lib --all-features 2>&1
```
- [ ] All existing tests pass (1560+)
- [ ] No test regressions

Note: `--lib` is used instead of `--workspace` due to a pre-existing `input_integration_tests` linker failure established in P00a.

## Lint Gate
```bash
cd /Users/acoliver/projects/uqm/rust
cargo fmt --all --check
```
- [ ] Format clean

Clippy check scoped to threading module (crate-wide clippy has pre-existing FFI declaration conflicts in wav_ffi.rs/io/ffi.rs unrelated to this plan):
```bash
cd /Users/acoliver/projects/uqm/rust && cargo clippy --lib --all-features 2>&1 | grep "threading/mod.rs"
```
- [ ] No NEW clippy warnings in `threading/mod.rs` compared to baseline (4 pre-existing `missing_safety_doc` warnings at ~lines 945, 963, 1035, 1148 are known and predate this plan — verified by `git stash` comparison)

## Deferred Implementation Detection
```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/threading/mod.rs | grep -v "test"
```
- [ ] No NEW TODOs introduced (pre-existing TODOs at ~lines 587, 602, 672, 688 are expected and will be cleaned in P08)

## ABI Compatibility
- [ ] `rust_thread_join` signature matches `rust_threads.h:36` declaration
- [ ] No parameters added or removed at FFI boundary
- [ ] C project can still call `rust_thread_join(ptr)` with one argument

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: fix issues before proceeding
