# Phase 08: TODO/Stub Cleanup

## Phase ID
`PLAN-20260314-THREADING.P08`

## Prerequisites
- Required: Phase 07a (Detached Thread Documentation Verification) completed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P07a.md`
- All tests pass

## Requirements Implemented (Expanded)

### Active-code TODO/stub cleanup within scoped target locations
This phase addresses the four stale TODO/stub markers identified during preflight and analysis.

Gaps addressed:
- **G5** (Medium): Stale TODO markers in active code
- **G6** (Low): Lifecycle processing stub with TODO

### G5 — Stale TODOs in mod.rs

Four TODOs exist in the scoped target locations in active code:

1. **Line ~596-603**: `task.state()` — `// TODO: Implement state retrieval`
   - **Analysis**: The method IS implemented. It loads from `AtomicU32` and maps to `TaskState`. The TODO is stale.
   - **Fix**: Remove the TODO comment.

2. **Line ~611-612**: `task.set_state()` — `// TODO: Implement state setting`
   - **Analysis**: The method IS implemented. It stores to `AtomicU32`. The TODO is stale.
   - **Fix**: Remove the TODO comment.

3. **Line ~681-683**: `process_thread_lifecycles()` — `// TODO: Implement lifecycle processing`
   - **Analysis**: This function is intentionally a no-op. Lifecycle processing is C-owned (spec §2.4).
   - **Fix**: Replace TODO with documentation explaining the intentional no-op.

4. **Line ~696-699**: `hibernate_thread()` — `// TODO: Implement thread hibernation`
   - **Analysis**: The function IS implemented — it calls `thread::sleep(duration)`.
   - **Fix**: Remove the TODO comment.

## Implementation Tasks

### Files to modify

#### `rust/src/threading/mod.rs` — Remove/replace 4 scoped stale TODOs

1. **`state()` method** (around line 596-603): remove stale TODO
2. **`set_state()` method** (around line 611-612): remove stale TODO
3. **`process_thread_lifecycles()` function** (around line 676-683): replace TODO with intentional no-op documentation
4. **`hibernate_thread()` function** (around line 696-699): remove stale TODO and document current implementation if helpful

### Pseudocode traceability
- No pseudocode lines — this is cleanup, not algorithm implementation

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] TODO at `state()` removed
- [ ] TODO at `set_state()` removed
- [ ] TODO at `process_thread_lifecycles()` replaced with intentional no-op doc
- [ ] TODO at `hibernate_thread()` removed
- [ ] No functional behavior changes

## Semantic Verification Checklist (Mandatory)
- [ ] `process_thread_lifecycles()` is still a no-op (not accidentally given behavior)
- [ ] `hibernate_thread()` still calls `thread::sleep(duration)`
- [ ] `state()` still reads from `AtomicU32`
- [ ] `set_state()` still writes to `AtomicU32`
- [ ] All tests pass

## Deferred Implementation Detection (Mandatory)

```bash
# Verify the four targeted stale markers are gone
grep -n -E "TODO|FIXME|HACK|placeholder|for now|will be implemented" \
  /Users/acoliver/projects/uqm/rust/src/threading/mod.rs
```

- [ ] The targeted stale markers at the four scoped functions are gone
- [ ] Any remaining TODO-like strings elsewhere in `mod.rs` are reviewed individually and do not fail this phase unless they are newly introduced by this work

## Success Criteria
- [ ] All 4 scoped stale TODOs resolved
- [ ] `process_thread_lifecycles()` documented as intentional no-op
- [ ] `hibernate_thread()` documented as complete implementation if documentation was updated
- [ ] No functional regressions
- [ ] All tests pass

## Failure Recovery
- rollback: `git checkout -- rust/src/threading/mod.rs`

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P08.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
