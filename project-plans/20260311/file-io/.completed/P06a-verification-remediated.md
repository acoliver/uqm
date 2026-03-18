# P06a Verification (Remediated): Mount Ordering and Access Semantics

## Verdict
ACCEPT

## Reviewed Inputs
1. `/Users/acoliver/projects/uqm/project-plans/20260311/file-io/.completed/P06.md`
2. `/Users/acoliver/projects/uqm/project-plans/20260311/file-io/plan/06-mount-ordering-access.md`
3. `/Users/acoliver/projects/uqm/project-plans/20260311/file-io/.completed/P06a-verification.md`
4. `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs`
5. Requested verification command:
   - `cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features 2>&1 | tail -5`

## Scope of Re-verification
Re-checked the three rejection items from the prior P06a verification against actual code:
1. overlay-aware mutation helper presence
2. cross-mount rename returning `EXDEV`
3. read-only shadow protection in mutation entry points
4. mount position hardening against underflow/collision risk

## Findings

### 1. Overlay-aware mutation helper: now present
Verified in `rust/src/io/uio_bridge.rs`:
- `MountResolution` struct exists at lines 714-717
- `resolve_mount_for_path()` exists at lines 728-753
- It iterates active mounts in registry order and resolves the topmost covering mount to a host path

Assessment:
- This directly addresses the previous rejection that no overlay-aware resolution helper existed for mutation entry points.
- The helper is actually used by `uio_rename`, `uio_open`, `uio_unlink`, `uio_mkdir`, and `uio_rmdir`.

### 2. Cross-mount rename EXDEV check: now present
Verified in `uio_rename()` at lines 168-193:
- Both source and destination are resolved with `resolve_mount_for_path()`
- Mount IDs are compared
- When mount IDs differ, the function returns `fail_errno(libc::EXDEV, -1)`

Assessment:
- This directly addresses the prior rejection item for missing cross-mount rename protection.
- The implementation is explicit and correctly uses `EXDEV`.

### 3. Read-only shadow protection in mutation entry points: now present
Verified in actual code:
- `uio_open()` write path checks `resolution.mount.read_only` and returns `EACCES` without falling through (lines 2247-2257)
- `uio_unlink()` checks `resolution.mount.read_only` and returns `EACCES` (lines 2440-2448)
- `uio_mkdir()` checks `resolution.mount.read_only` and returns `EACCES` (lines 395-403)
- `uio_rmdir()` checks `resolution.mount.read_only` and returns `EACCES` (lines 450-458)
- `uio_rename()` checks both old and new mount resolutions for `read_only` and returns `EACCES` (lines 171-186)

Assessment:
- The prior rejection stated these operations still worked directly on resolved host paths without overlay-aware read-only enforcement. That is no longer true.
- The write-mode branch in `uio_open()` explicitly fails on the topmost read-only mount and does not fall through to lower writable layers.

### 4. Mount position hardening: now present
Verified in `register_mount()` at lines 624-673:
- `DEFAULT_START_POSITION` introduced
- `saturating_sub(1)` used for TOP and ABOVE placement
- `saturating_add(1)` used for BOTTOM and BELOW placement

Assessment:
- This addresses the prior underflow concern around `usize` arithmetic.
- The implementation is materially safer than the previous `min - 1` / `relative_pos - 1` approach.

## Residual note
A helper for parent non-directory shadow detection exists:
- `is_parent_shadowed_by_file()` at lines 760-774

It appears preparatory and is not part of the three remediated rejection items requested for this re-verification. The requested re-verification items are addressed in code.

## Verification command result
```text
test threading::tests::test_condvar_broadcast ... ok
test threading::tests::test_hibernate_thread ... ok

test result: ok. 1523 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 0.10s
```

## Final Decision
ACCEPT

## Acceptance Basis
The three concrete rejection items from the prior verification are now addressed in actual code:
- overlay-aware mount resolution helper exists and is used by mutation operations
- cross-mount rename fails with `EXDEV`
- read-only protection is enforced in `uio_open`, `uio_unlink`, `uio_mkdir`, `uio_rmdir`, and `uio_rename`
- mount position arithmetic is hardened with saturating operations
