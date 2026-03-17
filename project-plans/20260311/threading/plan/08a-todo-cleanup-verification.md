# Phase 08a: TODO/Stub Cleanup — Verification

## Phase ID
`PLAN-20260314-THREADING.P08a`

## Prerequisites
- Required: Phase 08 completed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P08.md`

## Scoped TODO Elimination Verification

```bash
grep -n -E "TODO|FIXME|HACK|placeholder|for now|will be implemented" \
  /Users/acoliver/projects/uqm/rust/src/threading/mod.rs
```

- [ ] The four targeted stale markers in the scoped functions are gone
- [ ] Any remaining TODO-like strings elsewhere in `mod.rs` are reviewed case-by-case rather than treated as automatic phase failure
- [ ] No new TODO/FIXME/HACK markers were introduced by Phase 08

## Function-by-Function Check

```bash
# Verify state() has no stale TODO
grep -B 1 -A 5 "pub fn state" /Users/acoliver/projects/uqm/rust/src/threading/mod.rs

# Verify set_state() has no stale TODO
grep -B 1 -A 3 "pub fn set_state" /Users/acoliver/projects/uqm/rust/src/threading/mod.rs

# Verify process_thread_lifecycles() documented
grep -B 5 -A 3 "pub fn process_thread_lifecycles" /Users/acoliver/projects/uqm/rust/src/threading/mod.rs

# Verify hibernate_thread() documented or clean
grep -B 4 -A 3 "pub fn hibernate_thread" /Users/acoliver/projects/uqm/rust/src/threading/mod.rs
```

- [ ] `state()` — no stale TODO
- [ ] `set_state()` — no stale TODO
- [ ] `process_thread_lifecycles()` — documented as intentional no-op (not TODO)
- [ ] `hibernate_thread()` — no stale TODO; implementation remains intact

## Rust Test Gate

```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --workspace --all-features 2>&1
```
- [ ] All tests pass

## Rust Lint Gate

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
- [ ] Format clean
- [ ] Clippy clean

## Behavioral Regression Check
- [ ] No functional changes beyond scoped stale-comment cleanup
- [ ] `process_thread_lifecycles()` is still an empty/no-op function
- [ ] `hibernate_thread()` still calls `thread::sleep`

## Gate Decision
- [ ] PASS: proceed to Phase 09
- [ ] FAIL: fix issues before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P08a.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
