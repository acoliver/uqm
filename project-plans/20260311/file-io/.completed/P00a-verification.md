# P00a Preflight Verification — Verdict

**Date:** 2026-03-14
**Verdict:** ACCEPT

## Summary

The preflight report (P00a.md) is thorough, well-evidenced, and its claims hold up under spot-check. It correctly identifies the critical ABI mismatches, stubbed functionality, and missing exports that later implementation phases must resolve. The FAIL gate is appropriate for a preflight — it documents what the implementation must fix, not a flaw in the plan itself.

## Spot-Check Results

### Claim #1: `uio_openFileBlock2` ABI mismatch — [OK] CONFIRMED

- **C header** (`fileblock.h:74-75`): `uio_FileBlock *uio_openFileBlock2(uio_Handle *handle, off_t offset, size_t size);` — 3 params: handle, offset, size.
- **Rust** (`uio_bridge.rs:919-922`): `fn uio_openFileBlock2(_handle: *mut uio_Handle, _flags: c_int)` — 2 params: handle, flags.
- The preflight correctly identifies this as an ABI mismatch: wrong parameter count, wrong parameter types, wrong parameter semantics.

### Claim #2: `uio_accessFileBlock` ABI mismatch — [OK] CONFIRMED

- **C header** (`fileblock.h:76-77`): `ssize_t uio_accessFileBlock(uio_FileBlock *block, off_t offset, size_t length, char **buffer);` — 4th param is `char **buffer` (output pointer).
- **Rust** (`uio_bridge.rs:938-942`): 4th param is `_flags: c_int` — completely wrong type and semantics.
- The preflight correctly identifies this mismatch.

### Claim #3: `uio_getStdioAccess` ABI mismatch — [OK] CONFIRMED

- **C header** (`utils.h:34-35`): `uio_StdioAccessHandle *uio_getStdioAccess(uio_DirHandle *dir, const char *path, int flags, uio_DirHandle *tempDir);` — 4 params.
- **Rust** (`uio_bridge.rs:1194-1198`): Only 3 params — missing `tempDir: *mut uio_DirHandle`.
- The preflight correctly identifies this as a 3-vs-4 parameter mismatch.

### Claim #4: `uio_vasprintf` / `uio_asprintf` not exported — [OK] CONFIRMED

- Grep of `uio_bridge.rs` finds zero matches for `uio_vasprintf` or `uio_asprintf`.
- C header `utils.h:39-40` declares both as public API.
- The preflight correctly identifies this gap.

### Claim #5: `uio_clearFileBlockBuffers` not exported — [OK] CONFIRMED

- Grep of `uio_bridge.rs` finds zero matches for `uio_clearFileBlockBuffers`.
- C header `fileblock.h:84` declares it as public ABI.
- The preflight report mentions this in the header audit (PASS on C header side) and documents FileBlock stubs as blockers, but did not separately call out this missing export as its own blocker. Minor omission but the overall FileBlock blocker (#6) covers it.

## Additional Finding Not in Report

### `uio_setFileBlockUsageHint` ABI mismatch — MISSED by preflight

- **C header** (`fileblock.h:82-83`): `void uio_setFileBlockUsageHint(uio_FileBlock *block, int usage, size_t readAheadBufSize);` — 3rd param is `size_t readAheadBufSize`.
- **Rust** (`uio_bridge.rs:960-963`): 3rd param is `_flags: c_int` — wrong type (`c_int` vs `size_t`) and wrong name/semantics.
- This is another ABI mismatch in the FileBlock family. It doesn't change the overall verdict (FileBlock is already a known blocker) but should be noted for implementation.

## Assessment of Blocker Items

| # | Blocker | Verified | Notes |
|---|---------|----------|-------|
| 1 | `uio_openFileBlock2` ABI wrong | [OK] Confirmed | 2 params vs 3, wrong types |
| 2 | `uio_accessFileBlock` ABI wrong | [OK] Confirmed | `c_int` vs `char **buffer` |
| 3 | `uio_getStdioAccess` ABI wrong | [OK] Confirmed | 3 params vs 4 |
| 4 | `uio_vfprintf` is stub | Accepted on report evidence | Netplay callers exist |
| 5 | `uio_vasprintf`/`uio_asprintf` missing | [OK] Confirmed | Zero exports in Rust |
| 6 | FileBlock stubs vs ZIP callers | Accepted on report evidence | All stubs verified |
| 7 | Build command invalid | **FALSE ALARM** | Correct build is `cd sc2 && ./build.sh uqm`, not `make`. Proven working in prior sessions. |
| 8 | Temp-dir mount required | Accepted on report evidence | Engine startup depends on it |

## Verdict Rationale

**ACCEPT** — The preflight report is accepted because:

1. **Thoroughness:** It systematically covers all checklist items from the plan. Every section of `00a-preflight-verification.md` is addressed with specific file/line evidence.

2. **Accuracy:** All spot-checked claims (ABI mismatches, missing exports) are confirmed correct against actual source code.

3. **Appropriate conservatism:** Open questions (Q1-Q3) are resolved with clear evidence and carry-forward requirements. The regex and ZIP/FileBlock dependency decisions are well-reasoned.

4. **One false alarm (blocker #7):** The build command blocker is invalid — the project uses `./build.sh uqm`, not `make`. This was proven in prior sessions. This does not invalidate the report; it's a minor environmental knowledge gap. Later phases should use the correct build command.

5. **One minor omission:** `uio_setFileBlockUsageHint` has a type mismatch (`c_int` vs `size_t` for the 3rd parameter) that wasn't called out separately. This is subsumed under the general FileBlock blocker.

6. **FAIL gate is correct for its purpose:** The preflight is supposed to find problems before implementation begins. Finding 8 blockers means the implementation phases have clear work items. The plan itself anticipated this — Phase 08 (FileBlock), Phase 10 (StdioAccess), etc. exist precisely to address these gaps.

The implementation phases can proceed using this preflight as their ground truth, with the noted corrections (build command, `uio_setFileBlockUsageHint` type mismatch).
