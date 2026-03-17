# Phase 05: Path Normalization & errno Setting

## Phase ID
`PLAN-20260314-FILE-IO.P05`

## Prerequisites
- Required: Phase 04a completed
- Expected: Stream output and direct fread export are functional

## Requirements Implemented (Expanded)

### REQ-FIO-PATH-NORM: Correct path normalization
**Requirement text**: When a path contains `.` components, the subsystem SHALL remove them. When a path contains `..` components, the subsystem SHALL resolve them logically. When `..` would traverse above root `/`, the subsystem SHALL clamp at `/`. When a path contains repeated slashes, the subsystem SHALL collapse them. When a path contains a trailing slash, the subsystem SHALL strip it. When an empty path string is passed, the subsystem SHALL treat it as referring to the directory handle's own location.

Behavior contract:
- GIVEN: Path `"/foo/./bar/../baz"` relative to root
- WHEN: Normalization is applied
- THEN: Result is `"/foo/baz"`

- GIVEN: Path `"/../../above_root"`
- WHEN: Normalization is applied
- THEN: Result is `"/above_root"` (clamped at root)

- GIVEN: Path `"//foo///bar//"`
- WHEN: Normalization is applied
- THEN: Result is `"/foo/bar"`

- GIVEN: Empty path relative to a non-root directory handle `/content/addons`
- WHEN: Normalization is applied
- THEN: Result is `/content/addons` (the handle's own location), not unconditionally `/`

### REQ-FIO-PATH-CONFINEMENT: Host resolution stays within mount physical root
**Requirement text**: When a path is resolved against a mounted subtree, `..` SHALL NOT escape above the mount's physical root on the host filesystem.

### REQ-FIO-ERRNO: Proper errno reporting and safe failure
**Requirement text**: When any public operation fails due to an underlying filesystem or path-resolution error, the subsystem SHALL preserve an error code that allows existing callers to diagnose the failure. When detectably invalid arguments or unsupported combinations are passed, the subsystem SHALL fail safely and SHALL NOT crash or silently degrade behavior.

Behavior contract:
- GIVEN: `uio_open` on a nonexistent file without `O_CREAT`
- WHEN: The operation fails
- THEN: `errno` is set to `ENOENT`

- GIVEN: `uio_fopen` with an unrecognized mode string
- WHEN: The operation fails
- THEN: `errno` is set to `EINVAL`

- GIVEN: Any operation that partially allocates internal resources before failing
- WHEN: The operation returns failure
- THEN: intermediate allocations are cleaned up and subsystem state remains consistent

### REQ-FIO-PANIC-SAFETY: No panic crosses the FFI boundary
**Requirement text**: No Rust panic may propagate across the FFI boundary. All exported `extern "C"` entry points SHALL catch panics and convert them to safe failure returns.

## Implementation Tasks

### Files to modify
- `rust/src/io/uio_bridge.rs`
  - **`resolve_path` / equivalent virtual path helper**: Replace simple `base.join(rel)` with full normalization
    - Handle `.`, `..`, repeated slashes, root clamping, trailing slashes, and empty path = handle location
    - marker: `@plan PLAN-20260314-FILE-IO.P05`
    - marker: `@requirement REQ-FIO-PATH-NORM`
  - **Host-path mapping helper**: add explicit mount-root confinement logic so host resolution cannot traverse above the mount physical root during logical normalization
    - marker: `@requirement REQ-FIO-PATH-CONFINEMENT`
  - **New helper**: `set_errno(code: c_int)` — platform-aware errno setter
    - marker: `@requirement REQ-FIO-ERRNO`
  - **FFI guard helper(s)**:
    - add a canonical panic-containment wrapper strategy for exported `extern "C"` entry points
    - assign audited fallback returns per ABI shape (`NULL`, `-1`, `EOF`, `0`, or void)
    - set errno on panic-contained failure using the audited mapping
    - marker: `@requirement REQ-FIO-PANIC-SAFETY`
  - **Validation helpers**: centralize invalid-argument checks for null pointers, invalid mode strings, bad seek values, unsupported flag combinations where possible
  - **Error paths**: ensure `uio_open`, `uio_stat`, `uio_access`, `uio_rename`, `uio_mkdir`, `uio_rmdir`, `uio_unlink`, `uio_fopen`, `uio_getFileLocation`, and related helpers set errno consistently and unwind partial allocations
  - **Guard application sweep**: apply the panic-containment wrapper strategy across existing exported entry points before later phases add more complex failure surfaces

### Cross-phase errno rule
This phase establishes shared errno/panic-containment helpers and performs the initial sweep, but `REQ-FIO-ERRNO` remains a mandatory cross-cutting requirement in Phases 06–11. Any later phase that adds new failure paths must extend errno mapping and panic-safe fallback behavior for those new paths rather than assuming Phase 05 completed the work globally.

### Pseudocode traceability
- Uses pseudocode lines: PC-04 lines 01–24, PC-05 lines 01–21

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Virtual path normalization handles `.` components
- [ ] Virtual path normalization handles `..` components with root clamping
- [ ] Virtual path normalization collapses repeated slashes
- [ ] Virtual path normalization strips trailing slashes
- [ ] Empty path resolves to the directory handle's own location
- [ ] Host-path mapping includes explicit mount-root confinement logic
- [ ] `set_errno` compiles on macOS (primary dev platform)
- [ ] Panic-containment helper(s) exist and are usable from exported entry points
- [ ] Error return paths call `set_errno` and unwind partial state consistently
- [ ] Existing exported `extern "C"` entry points are wrapped or otherwise proven panic-contained

## Semantic Verification Checklist (Mandatory)
- [ ] Test: `"/foo/./bar" → "/foo/bar"`
- [ ] Test: `"/foo/../bar" → "/bar"`
- [ ] Test: `"/../../x" → "/x"` (root clamping)
- [ ] Test: `"//a///b//" → "/a/b"`
- [ ] Test: `""` relative to non-root handle resolves to that handle location
- [ ] Test: path mapping with repeated `..` cannot escape above the mount physical root on host path resolution
- [ ] Test: `errno` is set to ENOENT after failed `uio_open` on nonexistent path
- [ ] Test: `errno` is set to EEXIST after `uio_mkdir` on existing directory
- [ ] Test: invalid `uio_fopen` mode string fails with `EINVAL`
- [ ] Test: unsupported/invalid flag combination fails explicitly with consistent errno
- [ ] Test: `uio_getFileLocation` failure on archive-backed, synthetic-directory, and merged-directory cases sets `ENOENT`
- [ ] Verification note: exported `extern "C"` entry points have a panic-containment strategy with audited fallback returns and errno behavior

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] Path normalization tests pass
- [ ] confinement tests pass
- [ ] errno/validation tests pass
- [ ] panic-containment sweep is documented for current exported entry points
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/io/uio_bridge.rs`

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P05.md` containing:
- path normalization test summary
- host-confinement verification summary
- errno/validation coverage notes
- panic-containment strategy summary
