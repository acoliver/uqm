# Phase 03: Stream State Fix — Scaffold

## Phase ID
`PLAN-20260314-UIO.P03`

## Prerequisites
- Required: Phase 02a completed
- Pseudocode for Component 001 (Stream State Machine) reviewed and approved

## Requirements Implemented (Expanded)

### REQ-UIO-STREAM-007: EOF indicator reflects actual state
**Requirement text**: When a caller queries the EOF indicator, the subsystem shall report the stream's actual EOF state rather than a constant or synthesized value unrelated to stream state.

Behavior contract:
- GIVEN: a stream that has not reached EOF
- WHEN: `uio_feof(stream)` is called
- THEN: returns 0

- GIVEN: a stream where a read has encountered end-of-file
- WHEN: `uio_feof(stream)` is called
- THEN: returns non-zero

Why it matters:
- `sdluio.c` uses EOF/error distinction after zero-byte reads

### REQ-UIO-STREAM-008: Error indicator reflects actual state
**Requirement text**: When a caller queries the error indicator, the subsystem shall report the stream's actual error state rather than a constant or synthesized value unrelated to stream state.

Behavior contract:
- GIVEN: a stream with no I/O errors
- WHEN: `uio_ferror(stream)` is called
- THEN: returns 0

- GIVEN: a stream where an I/O operation has failed
- WHEN: `uio_ferror(stream)` is called
- THEN: returns non-zero

Why it matters:
- `sdluio.c:92-100` calls `uio_ferror(stream)` after zero-byte reads to set `SDL_SetError`

### REQ-UIO-STREAM-009: Clear stream status
**Requirement text**: When a caller clears stream status, the subsystem shall clear both EOF and error indicators.

Behavior contract:
- GIVEN: a stream in EOF or error state
- WHEN: `uio_clearerr(stream)` is called
- THEN: `uio_feof(stream)` returns 0 AND `uio_ferror(stream)` returns 0

### REQ-UIO-STREAM-010: Seek clears EOF
**Requirement text**: When a caller seeks on a stream, the subsystem shall update the visible stream position and clear EOF status on successful repositioning.

Behavior contract:
- GIVEN: a stream in EOF state
- WHEN: `uio_fseek(stream, 0, SEEK_SET)` succeeds
- THEN: `uio_feof(stream)` returns 0

### REQ-UIO-MEM-004: No memory leaks on stream close
**Requirement text**: When a stream is closed, the subsystem shall release all stream-owned resources including the buffer.

Behavior contract:
- GIVEN: an open stream with allocated buffer
- WHEN: `uio_fclose(stream)` is called
- THEN: buffer is freed, handle is closed, stream struct is freed

## Implementation Tasks

### Files to modify

#### `rust/src/io/uio_bridge.rs`
- **Add errno-setting helper function**
  - marker: `@plan PLAN-20260314-UIO.P03`
  - Add `fn set_errno(code: c_int)` using `libc::__error()` on macOS or platform-appropriate mechanism
  - Keep the helper small and reusable so later phases can set errno without duplication

- **Introduce stream-status helper scaffolding only**
  - marker: `@plan PLAN-20260314-UIO.P03`
  - Add or normalize small internal helpers such as `set_stream_status(...)` / `set_stream_operation(...)` if they simplify the later implementation
  - Limit this phase to helper extraction, naming cleanup, and wrapper-preserving refactor work

- **Document exact production-call sites to be changed in Phase 05**
  - marker: `@plan PLAN-20260314-UIO.P03`
  - In comments or local plan markers near the target functions, identify the later Phase 05 touchpoints for:
    - `uio_feof`
    - `uio_ferror`
    - `uio_clearerr`
    - `uio_fseek`
    - `uio_fclose`
    - `uio_fflush`
    - `uio_fwrite`
    - `uio_fputc`
    - `uio_fputs`
  - Do not change observable behavior in those functions yet

### Pseudocode traceability
- Uses pseudocode lines: 01-62

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `set_errno` helper function exists
- [ ] Any shared stream-status helper functions needed by later phases exist
- [ ] Target production call sites for the Phase 05 stream behavior changes are clearly identified
- [ ] No skipped phases
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist
- [ ] Code compiles with `cargo check`
- [ ] Existing tests still pass
- [ ] This phase does not change observable stream behavior yet
- [ ] Phase 04 can drive the first real behavior changes without reworking the scaffold

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] Scaffold changes compile cleanly
- [ ] Existing tests pass
- [ ] Stream implementation can proceed in Phase 05 without additional refactor churn

## Failure Recovery
- rollback: `git checkout -- rust/src/io/uio_bridge.rs`
- blocking issues: stream struct layout mismatch, errno setter not available on platform

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P03.md`
