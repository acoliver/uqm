# Phase 08: FileBlock Implementation

## Phase ID
`PLAN-20260314-FILE-IO.P08`

## Prerequisites
- Required: Phase 07a completed
- Expected: Mount resolution and path normalization are fully functional
- Dependency note: complete public FileBlock ABI coverage regardless of whether Phase 09 ultimately uses FileBlock internally or a direct Rust ZIP reader

## Requirements Implemented (Expanded)

### REQ-FIO-FILEBLOCK: Functional FileBlock operations
**Requirement text**: When the public ABI exposes FileBlock operations, the subsystem SHALL provide functional implementations for opening, closing, clearing buffers, and accessing file blocks. When a caller accesses a file block, the subsystem SHALL return readable bytes for the requested range through the public ABI contract. When the requested range extends beyond available content, the subsystem SHALL return only available bytes or fail, but SHALL NOT expose uninitialized memory.

Behavior contract:
- GIVEN: An open `uio_Handle` for a readable file
- WHEN: `uio_openFileBlock(handle)` is called
- THEN: Returns a non-null `uio_FileBlock` pointer

- GIVEN: A valid `uio_FileBlock`
- WHEN: `uio_openFileBlock2(handle, offset, size)` is called
- THEN: Returns a block covering the specified file region

- GIVEN: A valid `uio_FileBlock`
- WHEN: `uio_accessFileBlock(block, offset, length, &buffer)` is called
- THEN: Returns the number of bytes available; `buffer` points to stable internal data valid until next access on that block or close

- GIVEN: A valid `uio_FileBlock`
- WHEN: `uio_clearFileBlockBuffers(block)` is called
- THEN: Internal cached buffers are dropped without invalidating the block itself

- GIVEN: A valid `uio_FileBlock`
- WHEN: `uio_copyFileBlock(block, offset, buffer, length)` is called
- THEN: Copies bytes into caller buffer; returns 0 on success

Why it matters:
- FileBlock is part of the public ABI and must be completed even if ZIP reading later chooses a direct Rust implementation path.

## Implementation Tasks

### Files to modify
- `rust/src/io/uio_bridge.rs`
  - **`uio_FileBlock` backing struct**: Replace stub type with a real block state structure storing handle, base offset, size bounds, and cache state
    - marker: `@plan PLAN-20260314-FILE-IO.P08`
    - marker: `@requirement REQ-FIO-FILEBLOCK`
  - **`uio_openFileBlock`**: Create a block covering the entire file
  - **`uio_openFileBlock2`**:
    - use the real public signature `(handle, offset, size)`
    - validate offset/size against file bounds and fail safely on invalid input
    - extend errno mapping for range/argument failures introduced here
  - **`uio_accessFileBlock`**:
    - use the real public signature `(block, offset, length, char **buffer) -> ssize_t`
    - seek/read within the block region
    - cache returned data in internal storage
    - set `*buffer` to the internal buffer pointer
    - return available-byte count (short at EOF allowed)
    - extend errno mapping for bad block state, invalid pointers, and range failures introduced here
  - **`uio_clearFileBlockBuffers`**:
    - clear cache buffers and related bookkeeping
    - keep the FileBlock itself usable for future accesses
  - **`uio_copyFileBlock`**:
    - validate args
    - copy bytes into caller-provided buffer
    - return 0 on success, -1 on error
    - extend errno mapping for copy/range failures introduced here
  - **`uio_closeFileBlock`**: free all FileBlock resources
  - **`uio_setFileBlockUsageHint`**: accept hints without breaking correctness; no-op allowed if documented

### Pseudocode traceability
- Uses pseudocode lines: PC-11 lines 01–39

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `uio_FileBlock` is a real struct with file-range and cache state
- [ ] `uio_openFileBlock` creates a valid whole-file block
- [ ] `uio_openFileBlock2` uses the correct `(handle, offset, size)` ABI
- [ ] `uio_accessFileBlock` uses the correct `char **buffer` out-parameter ABI and returns `ssize_t`
- [ ] `uio_clearFileBlockBuffers` exists and is non-stub
- [ ] `uio_copyFileBlock` reads data into caller buffer
- [ ] `uio_closeFileBlock` frees all resources
- [ ] errno mapping is extended for FileBlock-specific failure paths
- [ ] No uninitialized memory exposed

## Semantic Verification Checklist (Mandatory)
- [ ] Test: open file, create whole-file block, access bytes at offset 0 → correct data and byte count
- [ ] Test: open ranged block with `uio_openFileBlock2(handle, offset, size)` → access is constrained to that region
- [ ] Test: `uio_accessFileBlock` returns short count at EOF without exposing garbage
- [ ] Test: repeated access returns stable pointer until next access/close
- [ ] Test: `uio_clearFileBlockBuffers` clears cache and next access repopulates correctly
- [ ] Test: copy bytes into caller buffer → buffer matches file content
- [ ] Test: invalid args fail safely with defined errno behavior
- [ ] Test: close block → no leak

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] FileBlock tests pass
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/io/uio_bridge.rs`

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P08.md` containing:
- FileBlock ABI coverage checklist
- cache/EOF verification summary
- FileBlock errno coverage summary
