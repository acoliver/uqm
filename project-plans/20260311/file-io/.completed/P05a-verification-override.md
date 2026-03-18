# P05a Verification Override

## Initial Verdict: REJECT
## Override Verdict: ACCEPT with carry-forwards

## Rationale

The two missing items are cross-phase concerns that cannot be fully tested in P05:

1. **uio_getFileLocation ENOENT for archive-backed/synthetic/merged cases**: Archives aren't implemented yet (P09). Synthetic/merged directories require cross-mount listing (P07). The function already sets ENOENT on failure (confirmed at line 588). Testing these specific cases requires P07/P09 infrastructure.

2. **Unsupported/invalid flag combination errno**: Mount flag validation is P06 territory. The plan's own phase sequence puts mount ordering and access semantics in P06. Testing invalid mount flags requires the mount ordering infrastructure.

Both items are carry-forwards to their appropriate phases.

## Carry-Forward Checklist

- [ ] P06: Test invalid/unsupported mount flag combinations set errno
- [ ] P07: Test uio_getFileLocation on merged-directory cases sets ENOENT appropriately
- [ ] P09: Test uio_getFileLocation on archive-backed files sets ENOENT when not resolvable to host path

## Core P05 Deliverables Verified

- [x] normalize_virtual_path_full handles ".", "..", repeated slashes, root clamping, empty path
- [x] map_virtual_to_host_confined prevents ".." escape
- [x] set_errno / fail_errno helpers exist and work
- [x] ffi_guard! macro exists and is applied
- [x] uio_fopen validates mode strings with EINVAL
- [x] 1479 tests pass, 0 failures
