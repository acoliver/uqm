# Phase 06: Mount Ordering & Access Semantics

## Phase ID
`PLAN-20260314-FILE-IO.P06`

## Prerequisites
- Required: Phase 05a completed
- Expected: Path normalization and errno setting are functional
- Carry-forward: P00a branch decisions for AutoMount and process temp-directory mounting must be reflected here if they affect mount APIs or topology

## Requirements Implemented (Expanded)

### REQ-FIO-MOUNT-ORDER: Correct mount placement semantics
**Requirement text**: When multiple mounts overlap in namespace, the subsystem SHALL honor the public mount placement semantics for top, bottom, above, and below placement. When a caller supplies a relative mount handle for above or below placement, the subsystem SHALL place the new mount relative to the referenced mount. When a required relative mount handle is omitted, the mount SHALL fail.

Behavior contract:
- GIVEN: Mounts A and B at `/content`
- WHEN: B is mounted with `uio_MOUNT_TOP`
- THEN: B takes priority over A for path resolution

- GIVEN: Mount A exists
- WHEN: Mount B is added with `uio_MOUNT_ABOVE` relative to A
- THEN: B has higher priority than A but lower than anything already above A

### REQ-FIO-ACCESS-MODE: Mode-aware access checks
**Requirement text**: When a caller requests an access check, the subsystem SHALL honor the requested access mode semantics rather than performing existence-only checks.

Behavior contract:
- GIVEN: A file on a read-only mount
- WHEN: `uio_access(dir, path, W_OK)` is called
- THEN: Returns -1 with `errno = EACCES`

### REQ-FIO-MUTATION: Overlay-aware mutation resolution
**Requirement text**: Mutation operations SHALL resolve through overlay precedence. The subsystem SHALL reject cross-mount renames with `EXDEV`, SHALL NOT fall through to lower writable layers when an upper read-only layer shadows a path, SHALL fail when parent visibility rules forbid the mutation, and SHALL report `ENOTDIR` when an upper visible component shadows a parent path with a non-directory.

### REQ-FIO-MOUNT-AUTOMOUNT: Conditional AutoMount parity branch
**Requirement text**: If P00a audit determines AutoMount is required for current parity, the plan must reserve concrete implementation/verification work rather than leaving it as an unresolved note.

## Implementation Tasks

### Files to modify
- `rust/src/io/uio_bridge.rs`
  - **`MountInfo` struct**: Add explicit ordering and read-only state; add AutoMount rule storage if P00a requires it
    - marker: `@plan PLAN-20260314-FILE-IO.P06`
    - marker: `@requirement REQ-FIO-MOUNT-ORDER`
  - **`register_mount`**:
    - validate `TOP`/`BOTTOM` require `relative == NULL`
    - validate `ABOVE`/`BELOW` require non-null `relative`
    - derive deterministic `position`
    - persist `read_only`
    - copy/store `autoMount` rules if AutoMount branch is active
    - extend errno mapping for invalid placement and argument combinations introduced here
  - **registry ordering helpers**: replace heuristic sorting with explicit position-based ordering
  - **resolution helpers**: iterate mounts in precedence order for reads and mutation-parent checks
  - **mount registry concurrency audit**:
    - identify the lock/snapshot rule used while iterating and mutating mount topology
    - ensure registration/unregistration does not expose partially updated ordering state to readers
  - **`uio_access`**:
    - validate mode bits
    - evaluate only the winning visible object
    - implement `F_OK`, `R_OK`, `W_OK`, `X_OK` per backing type and mount writability
    - extend errno mapping for invalid mode bits and new access failures introduced here
  - **write/mutation entry points** (`uio_open`, `uio_unlink`, `uio_rename`, `uio_mkdir`, `uio_rmdir`):
    - enforce topmost-visible-object semantics
    - reject shadowed read-only paths without falling through
    - reject missing writable parent exposure
    - detect `ENOTDIR` on upper-layer non-directory shadowing
    - reject cross-mount rename with `EXDEV`
    - extend errno mapping for the new mutation failure cases introduced here

### Conditional branch tasks
- If P00a determined AutoMount is required:
  - reserve the mount-registration support here and complete listing-trigger behavior in Phase 07
- If P00a determined process-level temp-directory mounting is required and implemented through regular mount APIs:
  - document the ordering/relative-placement semantics for that mount here as part of topology rules

### Pseudocode traceability
- Uses pseudocode lines: PC-06 lines 01–33, PC-07 lines 01–15, PC-08 lines 01–25

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `MountInfo` has deterministic ordering state and read-only state
- [ ] mount registration validates relative-handle requirements
- [ ] Mount ordering is position-based, not heuristic-based
- [ ] `uio_access` checks mode, not just existence
- [ ] Mutation operations use overlay-aware resolution helpers
- [ ] Parent-path checks include no-fallthrough and `ENOTDIR` shadowing rules
- [ ] AutoMount branch requirements are either wired for later use or explicitly recorded as not required
- [ ] Mount-registry iteration/mutation contract is documented for the shared topology state changed in this phase
- [ ] New access/mutation failure paths extend errno mapping rather than relying on Phase 05 alone

## Semantic Verification Checklist (Mandatory)
- [ ] Test: `MOUNT_TOP` gives highest priority
- [ ] Test: `MOUNT_BOTTOM` gives lowest priority
- [ ] Test: `MOUNT_ABOVE` inserts above the relative mount
- [ ] Test: `MOUNT_BELOW` inserts below the relative mount
- [ ] Test: invalid relative-handle combinations fail explicitly
- [ ] Test: `uio_access(W_OK)` on read-only mount returns -1
- [ ] Test: `uio_access(X_OK)` on directory returns 0
- [ ] Test: `uio_access` evaluates only the topmost visible object
- [ ] Test: `uio_open(O_WRONLY)` on read-only top layer fails without falling through
- [ ] Test: creating a file under a parent visible only in read-only layers fails
- [ ] Test: creating a file with same name visible in an upper read-only layer fails
- [ ] Test: upper-layer non-directory shadowing a parent path yields `ENOTDIR`
- [ ] Test: `uio_rename` across mounts fails with `EXDEV`
- [ ] Verification note: topology mutation vs resolution race class is reviewed for data-structure integrity
- [ ] Game startup mount sequence still works correctly

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] Mount ordering tests pass
- [ ] Access check tests pass
- [ ] Mutation-resolution edge-case tests pass
- [ ] Topology concurrency review for this phase is complete
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/io/uio_bridge.rs`
- Risk: mount ordering change may affect startup sequence — verify with full game boot

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P06.md` containing:
- ordering/access test summary
- mutation edge-case coverage summary
- topology-concurrency review note
- AutoMount branch status note
