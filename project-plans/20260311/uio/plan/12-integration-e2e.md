# Phase 12: Integration & End-to-End Verification

## Phase ID
`PLAN-20260314-UIO.P12`

## Prerequisites
- Required: Phase 11a completed
- All implementation phases complete
- All unit and module tests passing
- Full project builds successfully

## Requirements Implemented (Expanded)

### REQ-UIO-INT-001: Startup orchestration compatibility
**Requirement text**: Startup code continues to mount through the established UIO API without UIO taking ownership of startup policy.

### REQ-UIO-INT-002: External ownership model compatibility
**Requirement text**: Engine globals continue to own stored UIO objects.

### REQ-UIO-INT-003: SDL_RWops stream semantics
**Requirement text**: The `sdluio.c` adapter must correctly distinguish success, EOF, and I/O error.

### REQ-UIO-INT-004 / REQ-UIO-INT-006: ABI/header compatibility
**Requirement text**: C and Rust consumers continue to see the expected public ABI and header-driven behavior.

### REQ-UIO-INT-005: No fake-success behavior on enabled build surface
**Requirement text**: Rust UIO must not pretend to succeed on unsupported public behavior.

### REQ-UIO-INT-007: Diagnostic entry points callable
**Requirement text**: Memory/debug tooling entry points remain available and safe.

### REQ-UIO-ARCHIVE-ACCEPT: End-to-end archive acceptance
**Requirement text**: After mounting, consumers discover, open, read, seek, query archive assets through standard APIs without special-case behavior.

### REQ-UIO-CONC-001 through REQ-UIO-CONC-004: Concurrency verification
### REQ-UIO-LIFE-001 through REQ-UIO-LIFE-005: Lifecycle safety verification
### REQ-UIO-FFI-001 through REQ-UIO-FFI-004: FFI/ABI verification
### REQ-UIO-BOUND-001 through REQ-UIO-BOUND-003: Boundary preservation verification

## Implementation Tasks

This phase is verification-only. No new production code is written. The purpose is to confirm that all preceding phases integrate correctly end-to-end.

### End-to-End Test Scenarios

#### Scenario 1: Full game startup path
```
1. Build full project: cd sc2 && make
2. Run game with USE_RUST_UIO enabled
3. Verify: repository opens
4. Verify: content directory mounts (STDIO)
5. Verify: .uqm/.zip archives discovered and mounted (ZIP)
6. Verify: .rmp files discovered via getDirList
7. Verify: resource indices loaded
8. Verify: game reaches main menu
9. Verify: startup policy remains in options.c with no Rust-side policy takeover
```

#### Scenario 2: Archive content reading
```
1. Mount a real .uqm package archive
2. Enumerate contents via uio_getDirList
3. Open a known asset (e.g., a music file) via uio_fopen
4. Read full contents via uio_fread
5. Verify seek/tell consistency (seek to 0, re-read, compare)
6. Verify uio_fstat reports correct size
7. Verify uio_feof returns non-zero after full read
8. Verify uio_ferror returns 0 after successful read
9. Verify uio_fgetc / uio_fgets / uio_ungetc behave correctly on archive-backed streams
```

#### Scenario 3: SDL_RWops error propagation
```
1. Open a file via uio_fopen
2. Read it through the sdluio.c adapter path
3. At EOF: verify ferror returns 0, feof returns non-zero
4. Simulate error: verify ferror returns non-zero
5. Verify strerror(errno) produces meaningful message
```

#### Scenario 4: Cross-mount directory merge
```
1. Mount STDIO directory with files A, B
2. Mount ZIP archive with files B, C (higher precedence)
3. Get directory listing
4. Verify: A (from STDIO), B (from ZIP), C (from ZIP) — no duplicate B
5. Verify: regex filter works for .rmp pattern
6. Verify public DirList layout and free path remain ABI-safe
```

#### Scenario 5: Mount precedence
```
1. Mount base content at BOTTOM
2. Mount addon content at TOP
3. Both contain same file
4. Open file — verify addon version is returned
5. Unmount addon — verify base version now returned
6. Exercise overlapping-prefix case and verify the provisional rule: placement first, then longer prefix, then recency
```

#### Scenario 6: Transplant and shadow content
```
1. Mount base content
2. Mount addon shadow-content directory
3. Transplant shadow above base with uio_transplantDir
4. Verify shadow files override base files at new location
5. Verify transplanted mount is independently unmountable
```

#### Scenario 7: StdioAccess for archive content
```
1. Mount ZIP archive
2. Request stdio access for archive file
3. Verify temp copy created
4. Verify path is readable
5. Release — verify temp cleaned up
6. Force a temp-copy failure path and verify partial resources are cleaned up
```

#### Scenario 8: Memory and lifecycle safety
```
1. Open multiple streams and directory handles
2. Read, seek, read again
3. Unmount backing content while some objects remain live
4. Close in both normal and intentionally bad order
5. Verify no memory leaks (valgrind/ASan if available)
6. Verify no crash / UB floor holds for live directory/file/stream objects
```

#### Scenario 9: Concurrency
```
1. Open separate handles/streams on multiple threads
2. Read concurrently from independent objects
3. Mutate mounts in a controlled synchronized test while readers resolve paths
4. Verify no torn-state observations, no crashes, and valid results only
5. Exercise same-handle synchronization expectations explicitly
```

#### Scenario 10: FFI / ABI audit
```
1. Enumerate exported uio_* symbols and verify expected presence
2. Spot-check null-input behavior across representative pointer-taking APIs
3. Verify panic containment wrappers prevent unwinding across the FFI boundary
4. Verify public struct layouts consumed by C/Rust callers remain ABI-compatible
5. Verify unsupported exported APIs fail with correct sentinels and ENOTSUP, not dummy success
```

### Verification Matrix

| Requirement | Test scenario | Pass criteria |
|---|---|---|
| REQ-UIO-ARCHIVE-ACCEPT | 1, 2 | Game starts; archive assets readable |
| REQ-UIO-INT-001 | 1 | Startup orchestration works without moving policy into UIO |
| REQ-UIO-INT-002 | 1, 8 | External ownership model remains valid |
| REQ-UIO-INT-003 | 3 | SDL_RWops error propagation correct |
| REQ-UIO-INT-004/006 | 4, 10 | ABI/layout/header compatibility preserved |
| REQ-UIO-INT-005 | 10 | Unsupported surface does not fake success |
| REQ-UIO-INT-007 | 10 | Diagnostic entry points callable |
| REQ-UIO-LIST-002/003 | 4 | Merged listing deduplicated |
| REQ-UIO-LIST-016/017 | 4 | .rmp discovery works |
| REQ-UIO-MOUNT-002/003 | 5 | Mount precedence correct, including overlap rule |
| REQ-UIO-MOUNT-008 | 6 | Transplant works with distinct mount identity |
| REQ-UIO-STDIO-001/002/003/004/006 | 7 | StdioAccess works and cleans up on success/failure |
| REQ-UIO-LIFE-001 through 005 | 8 | Lifecycle safety floors preserved |
| REQ-UIO-CONC-001 through 004 | 9 | Concurrency safety requirements met |
| REQ-UIO-FFI-001 through 004 | 10 | Null handling, panic containment, ABI layout verified |
| REQ-UIO-BOUND-001 through 003 | 1, 3 | Boundaries preserved |
| REQ-UIO-MEM-001/004/006/007 | 8 | No leaks; null-safe cleanup works |

## Verification Commands

```bash
# Rust unit tests
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Full project build
cd sc2 && make

# Game startup test (manual)
# ./uqm --help (or equivalent to verify binary runs)

# Memory check (if available)
# valgrind ./uqm --startskip
# or: RUSTFLAGS="-Zsanitizer=address" cargo test
```

## Structural Verification Checklist
- [ ] All phases P03 through P11 completed
- [ ] All `.completed/` markers exist: P03.md through P11a.md
- [ ] Full project builds without warnings or errors
- [ ] No remaining C shims for UIO functions unless a shim is explicitly retained without changing public behavior
- [ ] Exported-surface audit artifact exists and covers all public `uio_*` symbols

## Semantic Verification Checklist
- [ ] Archive content is discoverable and readable through standard APIs
- [ ] SDL_RWops error propagation distinguishes EOF from error
- [ ] Cross-mount directory listings are merged and deduplicated
- [ ] Mount precedence respects placement flags and overlap rule
- [ ] Stream state machine (EOF/error/clear/seek-reset) works correctly
- [ ] errno set on all failure paths that require it
- [ ] No memory leaks on stream/dirlist/handle close
- [ ] No panics across FFI boundary
- [ ] Null arguments handled safely
- [ ] Public struct layouts remain ABI-compatible
- [ ] Independent-handle concurrency is safe
- [ ] Same-handle integrity expectations are met
- [ ] Live objects remain within the documented post-unmount/shutdown safety floor
- [ ] Startup-policy and adapter boundaries remain intact
- [ ] Unsupported APIs do not return fake success objects

## Final Audit

```bash
# Check for remaining stubs/placeholders
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|stub\|dummy\|hardcoded\|always returns\|intentionally" rust/src/io/

# Check for unwrap() calls in FFI functions (should use guarded handling)
grep -n "\.unwrap()" rust/src/io/uio_bridge.rs | grep -v "test"

# Check all exported functions are no_mangle extern "C"
grep -c "no_mangle.*extern.*C" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] All test scenarios pass
- [ ] Full project builds and links
- [ ] Game starts with Rust UIO
- [ ] No regressions from pre-plan state
- [ ] All requirement families verified explicitly

## Complete Requirement Traceability

| Requirement ID | Phase Implemented | Test Coverage |
|---|---|---|
| REQ-UIO-INIT-001/002/003 | P11 | test_uio_init_idempotent, test_uio_uninit |
| REQ-UIO-REPO-001/002/003 | Pre-existing + P06/P11 | pre-existing tests + repository lifecycle tests |
| REQ-UIO-MOUNT-001 through 005, 009 | P06 | test_mount_top/bottom/above/below, overlap-order tests |
| REQ-UIO-MOUNT-006/007/010 | P06, P10 | mount ordering + invalid mount-request tests |
| REQ-UIO-MOUNT-008 | P10 | test_transplant_dir_*, distinct-mount-identity tests |
| REQ-UIO-PATH-001 through 004 | P06, P08 | path normalization + archive resolution tests |
| REQ-UIO-PATH-005/006 | P10 | test_get_file_location_* |
| REQ-UIO-DIR-001 through 007 | Pre-existing + P10 | pre-existing + live-dir-after-unmount tests |
| REQ-UIO-LIST-001 through 013, 016-017, startup-acceptance slice of 015 | P09 | test_merged_dir_list_*, test_matches_pattern_*, ABI-safe DirList tests |
| REQ-UIO-FILE-001 through 014 | P06, P08, P10 | pre-existing + archive + access tests |
| REQ-UIO-FILE-015/016 | P10 | mkdir/rmdir constraint tests |
| REQ-UIO-STREAM-001 through 019 | P03-P05, P08, P10, P11 | test_feof/ferror/clearerr/fseek/fclose/fflush_*, archive stream tests, ungetc, vfprintf/clean-failure tests |
| REQ-UIO-ARCHIVE-001 through ACCEPT | P07-P08 | test_mount_archive_*, test_uio_*_archive_*, e2e |
| REQ-UIO-FB-001 through 007 | P10 | test_*_file_block_* |
| REQ-UIO-STDIO-001 through 006 | P10 | test_stdio_access_* |
| REQ-UIO-MEM-001 through 007 | P05, P09, P10, P11 | stream leak tests, dirlist free tests, cleanup-path tests |
| REQ-UIO-ERR-001 through 012 | P02b, P05, P06, P10, P11 | test_errno_*, exported-surface audit artifact, clean-unsupported-surface tests, FFI-failure tests |
| REQ-UIO-CONC-001 through 005 | P06, P08, P10, P12 | concurrent independent reads, same-handle integrity, mount-mutation serialization tests |
| REQ-UIO-LIFE-001 through 005 | P06, P08, P10, P12 | repository/unmount lifecycle tests, live-object post-unmount tests, shutdown-order tests |
| REQ-UIO-INT-001 through 007 | P06, P08, P10, P11, P12 | startup, SDL, ABI, diagnostics, unsupported-surface tests |
| REQ-UIO-FFI-001 through 004 | P00a, P02b, P09, P11, P12 | null-safety tests, exported-surface audit artifact, panic containment audit, ABI layout checks |
| REQ-UIO-BOUND-001 through 003 | P01, P12 | startup/adapter boundary verification |
| REQ-UIO-LOG-001/002 | P11 | test_print_mounts, test_dir_handle_print |

## Failure Recovery
- If game startup fails: check mount ordering, archive parsing, errno values
- If specific test fails: isolate the phase, revert only that phase's changes
- If build fails: check Makeinfo changes, shim removal, symbol exports

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P12.md`

Contents:
- Plan ID: PLAN-20260314-UIO
- Final verification timestamp
- All files changed across all phases
- All tests added/updated
- Full verification output
- Semantic verification summary
- Explicit PASS/FAIL decision
