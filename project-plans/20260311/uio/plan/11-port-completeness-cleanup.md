# Phase 11: Port-Completeness & Cleanup

## Phase ID
`PLAN-20260314-UIO.P11`

## Prerequisites
- Required: Phase 10a completed
- All engine-critical and compatibility-complete APIs are functional

## Requirements Implemented (Expanded)

### REQ-UIO-STREAM-018: Direct `uio_fread` export
**Requirement text**: The subsystem should export `uio_fread` directly from Rust so the C forwarding shim is not required.

Behavior contract:
- GIVEN: Rust crate is compiled
- WHEN: C code calls `uio_fread`
- THEN: the call goes directly to Rust without C shim intermediate

Why it matters:
- Eliminates `uio_fread_shim.c` from the build
- Simplifies the build dependency chain
- Port-completeness goal

### REQ-UIO-STREAM-019: `uio_vfprintf` implementation
**Requirement text**: Formatted output through the stream API shall be implemented or provide a compatible path.

Behavior contract:
- GIVEN: a writable stream and a format string with arguments
- WHEN: `uio_vfprintf(stream, fmt, args)` is called
- THEN: formatted output is written to the stream

Note: This is compatibility-complete, not engine-critical.

### REQ-UIO-MEM-005 / REQ-UIO-MEM-006 / REQ-UIO-MEM-007: Cleanup and null-safe deallocation
### REQ-UIO-ERR-007 / REQ-UIO-ERR-012: Unsupported APIs fail cleanly
### REQ-UIO-LOG-001 / REQ-UIO-LOG-002: Diagnostics
### REQ-UIO-INIT-001 / REQ-UIO-INIT-002 / REQ-UIO-INIT-003: Init/uninit review

## Implementation Tasks

### Files to modify

#### `rust/src/io/uio_bridge.rs`

- **Export `uio_fread` directly**
  - marker: `@plan PLAN-20260314-UIO.P11`
  - marker: `@requirement REQ-UIO-STREAM-018`
  - Rename `rust_uio_fread` to `uio_fread` in the `#[no_mangle] extern "C"` declaration
  - Keep the same implementation
  - This makes the C shim unnecessary

- **Implement `uio_vfprintf` or explicit clean failure**
  - marker: `@plan PLAN-20260314-UIO.P11`
  - marker: `@requirement REQ-UIO-STREAM-019, REQ-UIO-ERR-007, REQ-UIO-ERR-012`
  - Preferred: use `libc::vsnprintf` to format into a buffer, then write to the stream
  - If stable-Rust/C-variadic constraints prevent implementation, do **not** leave an ambiguous stub:
    - return the documented failure sentinel immediately
    - set `errno = ENOTSUP`
    - document the clean-failure behavior explicitly

- **Remove DirList buffer-size side-channel registry**
  - marker: `@plan PLAN-20260314-UIO.P11`
  - marker: `@requirement REQ-UIO-MEM-005`
  - Remove `DIR_LIST_BUFFER_SIZES` HashMap and any related helpers
  - `uio_DirList_free` must rely only on the ABI-safe allocation strategy introduced in Phase 09

- **Implement `uio_printMounts`**
  - marker: `@plan PLAN-20260314-UIO.P11`
  - marker: `@requirement REQ-UIO-LOG-001`
  - Lock mount registry, iterate mounts, print: id, mount_point, mounted_root, fs_type, active, placement metadata
  - Use `rust_bridge_log_msg` for output

- **Implement `uio_DirHandle_print`**
  - marker: `@plan PLAN-20260314-UIO.P11`
  - marker: `@requirement REQ-UIO-LOG-002`
  - Print dir handle path, repository pointer, refcount / ownership context
  - Use `rust_bridge_log_msg` for output

- **Review `uio_init` / `uio_unInit`**
  - marker: `@plan PLAN-20260314-UIO.P11`
  - marker: `@requirement REQ-UIO-INIT-001, REQ-UIO-INIT-002, REQ-UIO-INIT-003`
  - `uio_init`: ensure mount registry is initialized, archive registry is initialized
  - `uio_unInit`: clear mount registry, clear archive registry, release any global state
  - Make `uio_init` idempotent (check if already initialized)
  - Ensure cleanup paths free mount-handle allocations and tolerate valid null-no-op inputs safely

- **Finish exported-surface unsupported/stub audit**
  - marker: `@plan PLAN-20260314-UIO.P11`
  - marker: `@requirement REQ-UIO-ERR-007, REQ-UIO-ERR-012`
  - Update `project-plans/20260311/uio/exported-surface-audit.md` to final status rather than treating the audit as an implicit review step
  - Enumerate every exported `uio_*` symbol still not functionally implemented
  - Replace dummy returns with proper null / -1 / 0-item failure sentinels plus `errno = ENOTSUP`
  - Ensure no caller receives fake success handles from GPDir/GPFile/PRoot/GPRoot or any other remaining stubbed surface
  - For each remaining clean-failure unsupported API, record the verified sentinel and errno behavior in the audit artifact

#### `sc2/src/libs/uio/uio_fread_shim.c`
- **Remove or guard out** — no longer needed once Rust exports `uio_fread` directly
- marker: `@plan PLAN-20260314-UIO.P11`

#### `sc2/src/libs/uio/Makeinfo`
- **Remove `uio_fread_shim.c` from the Rust-mode file list**
- marker: `@plan PLAN-20260314-UIO.P11`

### Tests to add

- **`test_uio_fread_direct_export`**
  - Verify that `uio_fread` symbol exists and works (open file, fread from it)
  - marker: `@requirement REQ-UIO-STREAM-018`

- **`test_uio_print_mounts_no_crash`**
  - Call `uio_printMounts` with mounts registered
  - Assert no crash (output is diagnostic only)

- **`test_uio_dir_handle_print_no_crash`**
  - Call `uio_DirHandle_print` with a valid dir handle
  - Assert no crash

- **`test_uio_init_idempotent`**
  - Call `uio_init()` twice
  - Assert no crash or double-initialization error

- **`test_uio_uninit_clears_state`**
  - Init, mount, uninit
  - Assert mount registry is empty

- **`test_dirlist_free_no_side_channel`**
  - Get a DirList, free it, verify no HashMap lookup occurs
  - Structural expectation: HashMap code is removed

- **`test_remaining_stub_apis_fail_cleanly`**
  - Call any remaining intentionally unsupported exports
  - Assert they return public failure sentinels, not dummy handles
  - Assert `errno == ENOTSUP`

- **`test_null_safe_cleanup_entry_points`**
  - Call null-safe close/free APIs with null where the public contract permits it
  - Assert no crash

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_uio_fread` renamed to `uio_fread`
- [ ] `uio_fread_shim.c` removed or guarded out
- [ ] `Makeinfo` no longer lists `uio_fread_shim.c`
- [ ] `DIR_LIST_BUFFER_SIZES` HashMap removed
- [ ] `uio_printMounts` prints real mount info
- [ ] `uio_DirHandle_print` prints real handle info
- [ ] `uio_init`/`uio_unInit` are functional
- [ ] remaining stub APIs return public failure sentinels with `errno = ENOTSUP`
- [ ] 8+ new tests

## Semantic Verification Checklist
- [ ] `uio_fread` works without C shim
- [ ] `uio_printMounts` output is human-readable
- [ ] `uio_init` is safe to call multiple times
- [ ] `uio_unInit` releases all global state and mount-related allocations
- [ ] no side-channel registries remain
- [ ] unsupported exported APIs fail immediately and diagnostically
- [ ] all tests pass

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|dummy\|intentionally leak\|side.channel\|buffer_sizes" rust/src/io/uio_bridge.rs rust/src/io/uio/
```

Expected: no hits except intentionally documented clean-failure paths that set `ENOTSUP`.

## Success Criteria
- [ ] C shim eliminated
- [ ] cleanup items addressed
- [ ] unsupported-surface behavior is clean and explicit
- [ ] port is fully Rust-owned for all exported symbols
- [ ] verification commands pass

## Failure Recovery
- rollback: `git stash`
- blocking issues: C build breaks without shim (linker error), `vfprintf` ABI incompatibility

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P11.md`
