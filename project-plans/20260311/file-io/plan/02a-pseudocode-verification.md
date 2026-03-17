# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-FILE-IO.P02a`

## Prerequisites
- Required: Phase 02 completed
- Expected artifacts: Pseudocode components PC-01 through PC-13

## Structural Verification
- [ ] Every gap G1–G20 has a corresponding pseudocode component or explicit branch note
- [ ] Pseudocode uses numbered lines for traceability
- [ ] Validation points are present (null checks, flag validation, mode-string validation)
- [ ] Error handling is explicit (errno setting, return values, partial-allocation cleanup)
- [ ] Ordering constraints documented (which operations must happen first)
- [ ] Conditional branches from P00a are carried into pseudocode, not left implicit
- [ ] Panic-containment strategy is explicit at the FFI boundary

## Semantic Verification
- [ ] PC-01 covers `uio_feof`, `uio_ferror`, `uio_clearerr`, stream-close cleanup, and ABI-layout carry-forward if required
- [ ] PC-02 clarifies that any helper used for `va_list` formatting is internal-only and does not reintroduce exported-symbol shim dependency
- [ ] PC-04 handles `.`, `..`, repeated slashes, root clamping, empty path = directory handle location, and host-root confinement
- [ ] PC-05 covers errno helpers plus panic containment and fallback return behavior
- [ ] PC-06 handles TOP/BOTTOM/ABOVE/BELOW with relative-handle validation, errno-bearing failure, and AutoMount branch behavior
- [ ] PC-08 covers read-only enforcement, cross-mount EXDEV, parent visibility rules, and `ENOTDIR` shadowing
- [ ] PC-09 does not overclaim POSIX ERE parity beyond the audited engine decision
- [ ] PC-10 requires deterministic dedup ordering, non-null empty results, and `uio_DirList` ABI-preserving allocation ownership
- [ ] PC-11 matches real FileBlock ABI: `openFileBlock2(handle, offset, size)`, `accessFileBlock(..., char **buffer) -> ssize_t`, and `uio_clearFileBlockBuffers`
- [ ] PC-12 resolves stdio access using actual object-boundary rules, not only successful `uio_getFileLocation`
- [ ] PC-13 preserves safe cleanup after mount removal and names concrete concurrency race classes to audit

## Gate Decision
- [ ] PASS: proceed to Phase 03
- [ ] FAIL: revise pseudocode

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P02a.md` summarizing:
- pseudocode verification results
- any remaining ABI/signature mismatches
- branch assumptions approved for implementation
