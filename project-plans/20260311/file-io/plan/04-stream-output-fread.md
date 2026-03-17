# Phase 04: Stream Output & Direct `uio_fread` Export

## Phase ID
`PLAN-20260314-FILE-IO.P04`

## Prerequisites
- Required: Phase 03a completed
- Expected: Stream status tracking is functional
- Carry-forward: P00a/P01 ABI audit decision for `uio_Stream` remains in force while editing stream code

## Requirements Implemented (Expanded)

### REQ-FIO-STREAM-WRITE: Functional formatted output
**Requirement text**: When formatted output APIs are part of the public ABI, the subsystem SHALL implement their externally visible behavior rather than returning a permanent stub error.

Behavior contract:
- GIVEN: An open writable stream
- WHEN: `uio_vfprintf(stream, "%s=%d", args)` is called
- THEN: The formatted string is written to the stream and the character count is returned

- GIVEN: An open writable stream
- WHEN: `uio_fprintf(stream, "hello %s", "world")` is called
- THEN: "hello world" is written to the stream

Why it matters:
- Netplay debug logging (`packetq.c`, `netsend.c`, `netrcv.c`, `crc.h`) uses `uio_fprintf` to write debug output

### REQ-FIO-BUILD-BOUNDARY: All symbols exported from Rust
**Requirement text**: All `uio_*` symbols are exported directly from the Rust static library. No exported C shim files are required for any exported symbol.

Behavior contract:
- GIVEN: The `USE_RUST_UIO` build configuration
- WHEN: The build system links the final binary
- THEN: The `uio_fread` symbol is resolved from the Rust static library, not from `uio_fread_shim.c`

## Implementation Tasks

### Files to modify
- `rust/src/io/uio_bridge.rs`
  - **`uio_vfprintf`**: Implement formatted output with correct `va_list` behavior
    - marker: `@plan PLAN-20260314-FILE-IO.P04`
    - marker: `@requirement REQ-FIO-STREAM-WRITE`
  - **`rust_uio_fread` → `uio_fread`**: Rename/export the public symbol directly from Rust
    - marker: `@requirement REQ-FIO-BUILD-BOUNDARY`

### Files to modify (C/build side)
- `sc2/src/libs/uio/Makeinfo`: Remove `uio_fread_shim.c` from `uqm_CFILES` in Rust mode
- `sc2/src/libs/uio/uiostream.h`: Align declarations with direct Rust export
- `sc2/src/libs/uio/uio_fread_shim.c`: mark for deletion or leave inert/unbuilt

### Pseudocode traceability
- Uses pseudocode lines: PC-02 lines 01–10, PC-03 lines 01–07

## Technical Notes

### `va_list` handling strategy
Rust's stable ABI does not natively expose a full portable `va_list` formatting story. If an internal helper is required, the plan permits an **internal-only** helper for formatting support, but it must not violate the architectural/build-boundary requirement for exported `uio_*` symbols.

Allowed:
1. Internal helper compiled via `build.rs` that is **not** itself an exported `uio_*` ABI symbol and exists only to perform formatting.
2. A pure-Rust implementation if feasible and ABI-correct.

Not allowed:
1. Reintroducing a C shim whose purpose is to provide the exported `uio_vfprintf` or `uio_fread` symbol.
2. Leaving `uio_vfprintf` behaviorally stubbed.

Document the chosen strategy explicitly in the implementation notes when this phase is executed.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification Checklist
- [ ] `uio_fread` exported directly from Rust (no `rust_` prefix)
- [ ] `uio_fread_shim.c` removed from active Rust-mode build inputs
- [ ] `uio_vfprintf` no longer returns `-1` unconditionally
- [ ] `uio_fprintf` works for common format strings
- [ ] Any helper used for formatting is internal-only and does not provide exported `uio_*` ABI symbols
- [ ] Build succeeds without relying on `uio_fread_shim.c`

## Semantic Verification Checklist (Mandatory)
- [ ] `uio_fread` works identically to before (same behavior, different symbol source)
- [ ] `uio_vfprintf` produces correct formatted output
- [ ] `uio_fprintf` produces correct formatted output for `%s`, `%d`, `%x` patterns
- [ ] Invalid format/stream argument handling fails safely
- [ ] No link errors in full build
- [ ] Game boots and loads content correctly

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] `uio_fread` resolves from Rust library
- [ ] `uio_vfprintf` produces correct output
- [ ] Full build succeeds
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/io/uio_bridge.rs sc2/src/libs/uio/Makeinfo sc2/src/libs/uio/uiostream.h`

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P04.md` containing:
- chosen `va_list` strategy
- direct-export verification result
