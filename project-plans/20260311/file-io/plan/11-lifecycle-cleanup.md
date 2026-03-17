# Phase 11: Lifecycle, Init/Uninit & Resource Cleanup

## Phase ID
`PLAN-20260314-FILE-IO.P11`

## Prerequisites
- Required: Phase 10a completed
- Expected: All major features are functional

## Requirements Implemented (Expanded)

### REQ-FIO-LIFECYCLE: Proper initialization and shutdown
**Requirement text**: When the subsystem is initialized, the subsystem SHALL establish any global state required for subsequent operations. When the subsystem is uninitialized, the subsystem SHALL release remaining subsystem-owned resources and leave the process ready for future clean initialization. Repeated initialization and shutdown sequences SHALL leave the subsystem in a valid state and SHALL NOT leak.

### REQ-FIO-RESOURCE-MGMT: Resource cleanup without leaks
**Requirement text**: The subsystem SHALL manage ownership of repositories, mounts, directory handles, descriptors, streams, directory lists, stdio temp resources, and file blocks without leaks or hidden free-path dependencies.

### REQ-FIO-POST-UNMOUNT-CLEANUP: Cleanup remains safe after mount removal
**Requirement text**: When a mount is unmounted, `uio_close`, `uio_fclose`, `uio_closeDir`, and `uio_releaseStdioAccess` SHALL remain well-defined and safe to call even though further I/O on the invalidated objects is not guaranteed.

### REQ-FIO-THREAD-SAFETY: Lifecycle and topology integrity under concurrency contract
**Requirement text**: Independent-handle operations remain safe; repository topology mutation and shutdown preserve internal data-structure integrity according to the documented concurrency contract.

## Implementation Tasks

### Files to modify
- `rust/src/io/uio_bridge.rs`
  - **`uio_init`**:
    - replace log-only stub with real initialization of global registries/flags
    - marker: `@plan PLAN-20260314-FILE-IO.P11`
    - marker: `@requirement REQ-FIO-LIFECYCLE`
  - **`uio_unInit`**:
    - clear subsystem-owned registries/state
    - leave subsystem ready for clean re-init under caller contract
  - **`uio_closeRepository` / unmount paths**:
    - verify repository close unmounts all mounts and frees associated state
    - preserve safe cleanup behavior for already-open but now-invalid objects
  - **cleanup paths**:
    - review `uio_close`, `uio_fclose`, `uio_closeDir`, `uio_releaseStdioAccess`
    - ensure they do not require mount-topology presence to free owned resources safely
  - **`uio_fclose`**: confirm Phase 03 buffer cleanup is complete
  - **`uio_DirList_free`**:
    - eliminate fragile side-channel free dependency, or prove the final allocation strategy is self-contained and leak-free
  - **concurrency/locking review points**:
    - confirm registry locking preserves data-structure integrity under concurrent independent operations and topology mutation
    - verify repository close vs open handles, open dirlists, and outstanding stdio-access handles
    - verify returned allocations remain valid until their documented free/release points
  - **errno follow-through**:
    - extend errno mapping for lifecycle/shutdown failures introduced here rather than assuming earlier phases already covered them

### Conditional branch tasks
- If P00a required process-level temp-directory mounting and Phase 10 created lifecycle hooks for it:
  - finalize mount/unmount integration here
  - verify cleanup during repository close / subsystem shutdown

### Pseudocode traceability
- Uses pseudocode lines: PC-13 lines 01–27, PC-01 lines 35–43

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification Checklist
- [ ] initialization flag/state exists and is used consistently
- [ ] `uio_init` is not a log-only stub
- [ ] `uio_unInit` clears subsystem-owned state
- [ ] repository close unmounts all directories and frees repository-owned state
- [ ] cleanup operations do not depend on mount still being active
- [ ] `uio_fclose` frees stream buffer
- [ ] `uio_DirList_free` uses a self-contained or otherwise safe free path
- [ ] locking/topology integrity review is documented
- [ ] lifecycle/shutdown failure paths extend errno mapping where applicable

## Semantic Verification Checklist (Mandatory)
- [ ] Test: init → use → uninit → reinit → use works correctly
- [ ] Test: uninit clears mount registry (no stale mounts)
- [ ] Test: `uio_close` remains safe after mount removal
- [ ] Test: `uio_fclose` remains safe after mount removal
- [ ] Test: `uio_closeDir` remains safe after mount removal
- [ ] Test: `uio_releaseStdioAccess` remains safe after mount removal
- [ ] Test: `uio_fclose` doesn't leak buffer
- [ ] Test: `uio_DirList_free` on non-empty list doesn't leak
- [ ] Test: `uio_DirList_free` on empty list doesn't crash
- [ ] Test: `uio_DirList_free(NULL)` is safe
- [ ] Test: closeRepository cleans up all mounts
- [ ] Verification note: concrete race classes are reviewed for mount registry iteration/mutation, repository close vs open handles, and returned allocation lifetimes
- [ ] If temp-mount branch active: shutdown/cleanup of temp mount is correct
- [ ] Game shutdown sequence completes without errors

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] Lifecycle tests pass
- [ ] post-unmount cleanup tests pass
- [ ] no memory leaks in stream/dirlist/stdio-temp paths
- [ ] concrete concurrency race review is complete
- [ ] verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/io/uio_bridge.rs`

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P11.md` containing:
- lifecycle verification summary
- post-unmount cleanup verification summary
- concrete concurrency race review note
