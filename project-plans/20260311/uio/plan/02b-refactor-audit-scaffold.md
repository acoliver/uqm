# Phase 02b: Refactor & Audit Scaffold

## Phase ID
`PLAN-20260314-UIO.P02b`

## Prerequisites
- Required: Phase 02a completed
- Pseudocode approved for mount, archive, stream, dirlist, and lifecycle work

## Purpose
Create an explicit low-risk scaffold for decomposing `rust/src/io/uio_bridge.rs` and require a durable exported-surface audit artifact before feature-heavy implementation begins.

## Requirements Implemented (Planning Deliverable)
- REQ-UIO-ERR-012
- REQ-UIO-FFI-001 through REQ-UIO-FFI-004
- REQ-UIO-INT-004
- REQ-UIO-INT-005
- REQ-UIO-MEM-005

## Implementation Tasks

### Files to modify

#### `rust/src/io/uio_bridge.rs`
- Introduce only the minimum wrapper-preserving module split needed to reduce later implementation risk.
- Keep all `#[no_mangle] extern "C"` exports thin and behavior-preserving in this phase.
- Move shared type definitions and constants first into `rust/src/io/uio/types.rs`.
- Introduce `rust/src/io/uio/mod.rs` plus empty or near-empty compile-passing modules for:
  - `mount.rs`
  - `archive.rs`
  - `stream.rs`
  - `dirlist.rs`
  - `diagnostics.rs`
- If extraction of `fileblock.rs` or `stdio_access.rs` is deferred, leave an explicit note in the audit artifact and keep behavior unchanged in this phase.
- Do not combine module extraction with archive behavior changes, mount ordering changes, or stream-state fixes in this phase.

#### `project-plans/20260311/uio/exported-surface-audit.md`
- Create a durable audit artifact with one row per exported `uio_*` symbol and each FFI-visible shared struct.
- For each exported function record:
  - symbol name
  - source file / function location
  - return type
  - success sentinel
  - failure sentinel
  - null-input behavior
  - panic-containment status
  - current implementation status (`implemented`, `clean-failure stub`, `fake-success stub`, `needs audit`)
  - required follow-up phase
- For each FFI-visible struct record:
  - struct name
  - canonical C header location
  - Rust location
  - layout status
  - unresolved ABI risks
- Explicitly include `uio_DirList`, `uio_Stream`, `uio_DirHandle`, `uio_MountHandle`, and any other public structs shared with C or Rust FFI consumers.
- Explicitly mark remaining fake-success stubs that must be removed in later phases.

### Deliverable constraints
- This phase is scaffolding and audit only.
- No intentional API behavior changes beyond wrapper-preserving refactor safety work.
- If any extraction cannot be done without changing semantics, stop short and record the blocker in the audit artifact instead of forcing the refactor.

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `project-plans/20260311/uio/exported-surface-audit.md` exists
- [ ] Audit artifact has one row per exported `uio_*` symbol
- [ ] Audit artifact includes all FFI-visible shared structs
- [ ] `rust/src/io/uio/mod.rs` exists
- [ ] `types.rs`, `mount.rs`, `archive.rs`, `stream.rs`, `dirlist.rs`, and `diagnostics.rs` exist as compile-passing scaffolds or extracted modules
- [ ] `uio_bridge.rs` remains the FFI entry surface with thin wrappers
- [ ] No feature-behavior changes are mixed into this phase

## Semantic Verification Checklist
- [ ] Build/test behavior is unchanged aside from wrapper-preserving refactor movement
- [ ] Audit artifact identifies every remaining fake-success stub or unresolved ABI risk
- [ ] Later phases can reference exact modules and audit rows rather than vague monolith locations
- [ ] Module extraction reduces, rather than increases, later archive/dirlist/stream integration risk

## Success Criteria
- [ ] Durable exported-surface audit artifact created
- [ ] Early module scaffold established safely
- [ ] No ABI regressions introduced
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/io/uio_bridge.rs rust/src/io/uio`
- blocking issues: extraction changes semantics, unresolved ABI coupling requires narrower scaffold

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P02b.md`
