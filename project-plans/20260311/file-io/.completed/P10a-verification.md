# Phase P10 Verification: StdioAccess and uio_copyFile

## Verdict
REJECT

## Scope Verified
- Results document: `/Users/acoliver/projects/uqm/project-plans/20260311/file-io/.completed/P10.md`
- Plan document: `/Users/acoliver/projects/uqm/project-plans/20260311/file-io/plan/10-stdio-access-copyfile.md`
- Code: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs`
- Test command:
  - `cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features 2>&1 | tail -5`

## What matches the plan
- `StdioAccessHandleInner` exists with path/temp bookkeeping fields.
- `uio_getStdioAccess` has the required 4-parameter signature.
- `uio_StdioAccessHandle_getPath` returns the stored stable C string path.
- `uio_releaseStdioAccess` frees the handle and only deletes temp files for temp-copy handles.
- `uio_copyFile` exists, copies in 8KB chunks, and attempts partial-destination cleanup on copy failure.
- `uio_open` was updated to honor `O_CREAT | O_EXCL` with `create_new(true)`.
- The requested cargo test command passes:
  - `test result: ok. 1559 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 0.12s`

## Rejection reasons

### 1. Missing required temp-directory cleanup behavior
Plan/result contract says temp-copy release should delete the temp file and temp directory best-effort.

Actual code in `uio_releaseStdioAccess` only removes the temp file:
- `rust/src/io/uio_bridge.rs:2360-2368`

It does not attempt to remove the temp directory, so the implementation does not meet the documented/required release behavior.

### 2. Missing required semantic tests from the phase plan
The plan marks several semantic checks as mandatory, but the code/tests shown for P10 do not cover them:
- merged directory / synthetic archive directory boundary behavior
- handle/path usability after mount changes until release
- partial-failure cleanup test for `uio_copyFile`
- resource loading through `sc2/src/libs/resource/loadres.c`
- repository cleanup/topology race safety verification note backed by code/test evidence
- repository-visible temp area end-to-end behavior if conditional temp-mount branch were active

Existing tests cover only:
- direct stdio path
- ZIP temp copy
- directory/missing/tempdir-null/null-handle cases
- basic/existing-dest/missing-source/large copy

That is insufficient for the mandatory checklist in the plan.

### 3. The completion report overstates what was verified
`P10.md` claims success for items not evidenced by the implementation/tests inspected here, especially:
- temp directory deletion on release
- partial-failure cleanup test for `uio_copyFile`
- topology-change/lifetime safety validation
- full mandatory semantic checklist completion

## Code references
- `StdioAccessHandleInner`: `uio_bridge.rs:1959-1966`
- `uio_StdioAccessHandle_getPath`: `uio_bridge.rs:2154-2167`
- `uio_getStdioAccess`: `uio_bridge.rs:2177-2335`
- `uio_releaseStdioAccess`: `uio_bridge.rs:2345-2371`
- `uio_copyFile`: `uio_bridge.rs:2382-2472`
- `O_EXCL` handling in `uio_open`: `uio_bridge.rs:3018-3035`
- P10 tests reviewed: `uio_bridge.rs:6384-6826`

## Test command output
```text
test threading::tests::test_condvar_wait_signal ... ok
test threading::tests::test_hibernate_thread ... ok

test result: ok. 1559 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 0.12s
```

## Conclusion
Phase P10 is close, but it does not fully satisfy the plan as written. The implementation should not be accepted until the release behavior and mandatory verification gaps are resolved or the phase requirements are explicitly narrowed and re-approved.
