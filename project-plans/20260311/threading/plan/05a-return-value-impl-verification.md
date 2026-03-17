# Phase 05a: Return Value Propagation — Implementation Verification

## Phase ID
`PLAN-20260314-THREADING.P05a`

## Prerequisites
- Required: Phase 05 completed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P05.md`

## ABI Consistency Check

Verify all three files declare the same signature for `rust_thread_join`:

```bash
# Rust side
grep -n "rust_thread_join" /Users/acoliver/projects/uqm/rust/src/threading/mod.rs

# C header
grep -n "rust_thread_join" /Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_threads.h

# C adapter forward declaration
grep -n "rust_thread_join" /Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c
```

- [ ] All three have matching two-parameter signature: `(RustThread*, int* out_status) -> int`

## Rust Test Gate

```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --workspace --all-features 2>&1
```
- [ ] All existing 1547+ tests pass
- [ ] All 4 P04 tests pass (`test_thread_c_int_return_*`)
- [ ] No regressions

## Rust Lint Gate

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
- [ ] Format clean
- [ ] Clippy clean

## C Build Gate

```bash
cd /Users/acoliver/projects/uqm/sc2
make -f Makefile.build
```
- [ ] C project compiles without errors
- [ ] No warnings related to `rust_thread_join` signature

## Behavioral Verification

### Return value data flow
- [ ] `spawn_c_thread` closure captures `func(data)` as `c_int` (not discarded)
- [ ] `Thread<c_int>` wraps `JoinHandle<c_int>` containing the return value
- [ ] `rust_thread_join` extracts `c_int` via `join()` → `Ok(status)`
- [ ] `rust_thread_join` writes `status` to `*out_status` when non-null
- [ ] `WaitThread` reads `out_status` and writes it to caller's `*status`

### Edge cases
- [ ] `rust_thread_join(NULL, &out_status)` → returns 0, writes 0 to out_status
- [ ] `rust_thread_join(thread, NULL)` → returns 1, no write attempted
- [ ] `WaitThread(t, NULL)` → no crash, no write
- [ ] `ProcessThreadLifecycles` calls `WaitThread(t, NULL)` → still works

## Gate Decision
- [ ] PASS: proceed to Phase 06
- [ ] FAIL: fix issues before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P05a.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
