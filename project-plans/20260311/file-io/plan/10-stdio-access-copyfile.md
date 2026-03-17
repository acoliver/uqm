# Phase 10: StdioAccess & uio_copyFile

## Phase ID
`PLAN-20260314-FILE-IO.P10`

## Prerequisites
- Required: Phase 09a completed
- Expected: ZIP mounts are functional (StdioAccess must handle both stdio and archive-backed files)
- Carry-forward: Q3 audit result from P00a determines whether process-level temp-directory mounting work is added in this phase

## Requirements Implemented (Expanded)

### REQ-FIO-STDIO-ACCESS: Temporary stdio path for virtual files
**Requirement text**: `uio_getStdioAccess` returns a `uio_StdioAccessHandle` that provides a guaranteed stdio-filesystem path to the specified virtual file. If the file already resides on a stdio filesystem, the returned path points directly to it. If the file is archive-backed, the subsystem copies it to a temporary directory and returns a path to the copy. `uio_releaseStdioAccess` cleans up any temporary copies.

Behavior contract:
- GIVEN: A file at `/content/data.txt` backed by a stdio mount
- WHEN: `uio_getStdioAccess(contentDir, "data.txt", 0, NULL)` is called
- THEN: Returns a handle; `uio_StdioAccessHandle_getPath` returns the direct host path

- GIVEN: A file at `/content/packed.png` backed by a ZIP mount
- WHEN: `uio_getStdioAccess(contentDir, "packed.png", 0, tempDir)` is called
- THEN: Returns a handle; `uio_StdioAccessHandle_getPath` returns a path to a temp copy; the copy contains the decompressed content

- GIVEN: A valid StdioAccess handle for a temp copy
- WHEN: `uio_releaseStdioAccess(handle)` is called
- THEN: The temp file is deleted, the temp directory is removed (best-effort)

### REQ-FIO-COPY: File copying through the virtual namespace
**Requirement text**: `uio_copyFile` copies a file from one virtual location to another, resolving both paths through the mount system.

### REQ-FIO-MOUNT-TEMP: Conditional process-level temp-directory mounting
**Requirement text**: If audit determines current callers depend on repository-visible temp mounts, this phase must implement that behavior or explicitly wire it into lifecycle setup/teardown.

### REQ-FIO-UTILS-AUDIT: Rust-mode utils ABI audit
**Requirement text**: Public utils-surface symbols excluded from the C Rust-UIO build must be audited so the final plan does not overlook ABI-visible obligations.

## Implementation Tasks

### Files to modify
- `rust/src/io/uio_bridge.rs`
  - **`StdioAccessHandleInner`**: store path and temp-resource bookkeeping
  - **`uio_getStdioAccess`**:
    - keep the correct 4-parameter signature from `utils.h`
    - resolve against the winning visible object using actual mount/backing inspection
    - do not assume success of public `uio_getFileLocation` for archive-backed files
    - reject directories with `EISDIR`
    - return direct-path handle for stdio-backed single backing object
    - return temp-copy handle for archive-backed or otherwise non-stdio-backed file
    - preserve stable path until release
    - record the lifetime rule needed to keep returned path data valid until handle release even if topology changes later
    - marker: `@plan PLAN-20260314-FILE-IO.P10`
    - marker: `@requirement REQ-FIO-STDIO-ACCESS`
  - **`uio_StdioAccessHandle_getPath`**: return `handle.path`
  - **`uio_releaseStdioAccess`**:
    - free bookkeeping
    - delete temp file/dir best-effort for temp-copy handles only
    - never delete underlying host file for direct-path handles
  - **`uio_copyFile`**:
    - open source/destination through virtual namespace rules
    - copy in chunks
    - unwind partial destination on failure
    - preserve errno/state
    - extend errno mapping for copy/open/create/cleanup failures introduced here
    - marker: `@requirement REQ-FIO-COPY`
  - **stdio-access/thread-safety audit**:
    - confirm returned stdio-access handles remain releasable and their returned path strings remain valid until release even if the source mount or repository topology changes
    - confirm cleanup paths do not race unsafely with repository close or mount removal

### Conditional branch tasks
- If P00a determined process-level temp-directory mounting is required:
  - implement repository-visible temp mount creation or equivalent exposure here (or in coordinated lifecycle hooks with Phase 11)
  - define where it is mounted, how it is cleaned up, and how callers consume it
- If P00a determined it is not required:
  - record explicit deferral evidence in the phase completion marker and final summary

### Audit follow-up tasks
- Record the P00a/P01 audit result for `uio_asprintf` / `uio_vasprintf` and any other public utils ABI symbols so Phase 13 can prove the utils surface was not partially ignored

### Concrete caller touchpoint
- `sc2/src/libs/resource/loadres.c`: resource loading must continue to work through `uio_getStdioAccess`, including the direct-path vs temp-copy boundary

### Pseudocode traceability
- Uses pseudocode lines: PC-12 lines 01–30

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification Checklist
- [ ] `StdioAccessHandleInner` exists with path and temp-resource fields
- [ ] `uio_getStdioAccess` retains the correct 4-parameter signature
- [ ] stdio-access resolution uses actual object-boundary rules, not only `uio_getFileLocation` success/failure shortcuts
- [ ] `uio_getStdioAccess` handles stdio-backed files (direct path)
- [ ] `uio_getStdioAccess` handles ZIP-backed files (temp copy)
- [ ] `uio_getStdioAccess` rejects directories (`EISDIR`)
- [ ] `uio_releaseStdioAccess` only deletes temp artifacts for temp-copy handles
- [ ] `uio_copyFile` exists and cleans up partial failures
- [ ] errno mapping is extended for StdioAccess/copy failure cases introduced here
- [ ] stdio-access lifetime and cleanup behavior under topology changes is documented for shared state touched here
- [ ] conditional temp-mount work is implemented or explicitly deferred according to P00a
- [ ] utils ABI audit results are recorded

## Semantic Verification Checklist (Mandatory)
- [ ] Test: StdioAccess on stdio file → direct path, no temp copy
- [ ] Test: StdioAccess on ZIP file → temp copy with correct content
- [ ] Test: Release direct handle → no file deletion
- [ ] Test: Release temp handle → temp file and dir deleted best-effort
- [ ] Test: StdioAccess on directory → `NULL`, `errno = EISDIR`
- [ ] Test: StdioAccess on missing file → `NULL`, `errno = ENOENT`
- [ ] Test: StdioAccess on merged directory / synthetic archive directory follows file-location boundary rules correctly
- [ ] Test: `uio_getStdioAccess` remains usable after mount changes until handle release semantics are invoked
- [ ] Test: `uio_copyFile` copies content correctly
- [ ] Test: `uio_copyFile` to existing dest fails (`O_EXCL` semantics)
- [ ] Test: `uio_copyFile` partial-failure path removes partial destination and preserves state
- [ ] Verification note: stdio-access handle/path lifetime and release remain safe across topology changes and repository cleanup races
- [ ] If temp-mount branch active: repository-visible temp area behavior is tested end-to-end
- [ ] `sc2/src/libs/resource/loadres.c` resource loading works through `uio_getStdioAccess`

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] StdioAccess tests pass for both file types
- [ ] boundary tests pass for direct-path vs temp-copy decisions
- [ ] `uio_copyFile` tests pass
- [ ] stdio-access lifetime/concurrency review is complete
- [ ] conditional temp-mount work is resolved
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/io/uio_bridge.rs`

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P10.md` containing:
- stdio-access boundary verification summary
- `uio_copyFile` verification summary
- stdio-access lifetime/concurrency review note
- temp-mount branch result
- utils ABI audit note
