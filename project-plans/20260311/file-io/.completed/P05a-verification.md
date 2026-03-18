# P05a Verification: Phase P05 (Path Normalization, errno, Panic Safety)

## Verdict
REJECT

## Basis
The implementation in `rust/src/io/uio_bridge.rs` satisfies several core P05 requirements, but it does not fully satisfy the plan's mandatory semantic checklist.

## Verified against code

### 1. Path normalization exists and covers required core behaviors
Verified in `normalize_virtual_path_full` at `rust/src/io/uio_bridge.rs:1471-1521`:
- empty path returns base handle location
- absolute paths ignore base
- `.` components are skipped
- `..` components are resolved
- root clamping is implemented by refusing to pop above the root component
- repeated slashes/trailing slashes are normalized via `Path::components()` traversal

Relevant tests exist at `rust/src/io/uio_bridge.rs:3254-3315`:
- dot removal
- dotdot resolution
- root clamping
- repeated slash collapse
- empty path => base location
- absolute path handling
- complex normalization case

### 2. Host-path confinement exists
Verified in `map_virtual_to_host_confined` at `rust/src/io/uio_bridge.rs:1527-1559`:
- `..` pops only within accumulated relative host components
- escape above mount root is clamped by refusing to pop when empty
- `.` is skipped
- final host path is rebuilt as `mount_root + confined components`

Relevant tests exist at `rust/src/io/uio_bridge.rs:3317-3343`:
- normal mapping
- escape prevention
- single `..` resolution

### 3. errno helpers exist
Verified at `rust/src/io/uio_bridge.rs:37-48`:
- `set_errno(code: c_int)`
- `fail_errno<T>(code, failure_return)`

### 4. Panic guard exists and is applied to exported functions
Verified at `rust/src/io/uio_bridge.rs:55-62`:
- `ffi_guard!` wraps function bodies with `catch_unwind`
- panic fallback sets `errno` to `EIO`

Observed application on exported functions from search results, including:
- `uio_rename`
- `uio_access`
- `uio_stat`
- `uio_mkdir`
- `uio_rmdir`
- `uio_getFileLocation`
- `uio_open`
- `uio_unlink`
- `uio_fopen`

### 5. `uio_fopen` validates mode strings and sets `EINVAL`
Verified in `rust/src/io/uio_bridge.rs:2042-2132`:
- null `dir`, `mode`, and invalid/null `path` cases set `EINVAL`
- invalid mode strings are rejected
- invalid mode test exists at `rust/src/io/uio_bridge.rs:3411-3438`

### 6. Requested test command
Command run:
`cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features 2>&1 | tail -5`

Result:
- `1479 passed`
- `0 failed`
- exit code `0`

### 7. Deferred implementation markers
Ran the required grep against `src/io/uio_bridge.rs`.
Result: no matches.

## Why this is REJECT instead of ACCEPT
The plan marks the semantic checklist as mandatory. The code and tests do not fully cover all mandatory items from `project-plans/20260311/file-io/plan/05-path-errno.md`.

Missing or unverified mandatory items:

1. **Unsupported/invalid flag combination fails explicitly with consistent errno**
   - I found no matching test or obvious dedicated validation path for this checklist item in `uio_bridge.rs`.

2. **`uio_getFileLocation` failure on archive-backed, synthetic-directory, and merged-directory cases sets `ENOENT`**
   - `uio_getFileLocation` does set `ENOENT` on general resolution failure (`rust/src/io/uio_bridge.rs:581-589`).
   - However, I found no tests covering the specific archive-backed, synthetic-directory, and merged-directory cases required by the plan.

Because the plan requires verification against both the structural and semantic checklists, and these mandatory semantic cases are not demonstrated by the actual code/tests I reviewed, Phase P05 cannot be accepted as fully verified.

## Summary
What passes:
- path normalization implementation and tests
- host confinement implementation and tests
- errno helpers present
- panic guard present and applied
- `uio_fopen` invalid mode handling with `EINVAL`
- requested cargo test command passes

What blocks acceptance:
- missing verification for invalid/unsupported flag-combination errno behavior
- missing verification for `uio_getFileLocation` ENOENT behavior in archive-backed, synthetic-directory, and merged-directory cases
