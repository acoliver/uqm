# P04a Verification — PLAN-20260314-FILE-IO.P04

## Verdict
ACCEPT

## Basis

### 1. Completion report vs. plan
- Reviewed `/Users/acoliver/projects/uqm/project-plans/20260311/file-io/.completed/P04.md`
- Reviewed `/Users/acoliver/projects/uqm/project-plans/20260311/file-io/plan/04-stream-output-fread.md`
- The implemented changes match the requested scope for Phase P04:
  - functional `uio_vfprintf`
  - direct Rust export of `uio_fread`
  - Rust-mode build no longer relying on `uio_fread_shim.c`
  - clean `uiostream.h` declaration for `uio_fread`

### 2. `rust/src/io/uio_bridge.rs`
Verified in `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs`:

#### `uio_vfprintf`
- Located at line 791.
- It no longer returns `-1` unconditionally.
- It:
  - validates `stream` and `format`
  - sets `errno` to `EINVAL` on invalid arguments
  - calls internal helper `uio_vfprintf_format_helper`
  - writes the formatted buffer using `uio_fwrite`
  - frees the allocated buffer
  - returns `len as c_int` on success
  - returns `-1` only on actual error paths

This satisfies the structural requirement that `uio_vfprintf` is no longer a permanent stub.

#### `uio_fread`
- Located at line 1962.
- Exported directly as:
  - `#[no_mangle]`
  - `pub unsafe extern "C" fn uio_fread(...) -> size_t`
- Search found no `rust_uio_fread` symbol in the file.
- Internal call sites use `uio_fread(...)` directly.

This satisfies the direct-export requirement.

### 3. `sc2/src/libs/uio/Makeinfo`
Verified in `/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo`:
- In the `USE_RUST_UIO` block, `uqm_CFILES` is:
  - `charhashtable.c paths.c uioutils.c`
- `uio_fread_shim.c` is not present.
- Search confirmed no reference to `uio_fread_shim.c` remains in this file.

This satisfies the build-boundary requirement for Rust-mode UIO.

### 4. `sc2/src/libs/uio/uiostream.h`
Verified in `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h`:
- Declaration is clean and unconditional:
  - `size_t uio_fread(void *buf, size_t size, size_t nmemb, uio_Stream *stream);`
- No `USE_RUST_UIO` conditional wrapper remains around this declaration.

This satisfies the header cleanup requirement.

### 5. Requested test command
Executed:

    cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features 2>&1 | tail -5

Result:
- `test result: ok. 1479 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 0.10s`

## Notes
- This verification confirms the specific P04 acceptance points requested.
- The full plan’s broader semantic checklist items like full build success and game boot were not re-verified here because they were not part of the requested command set.

## Final Decision
ACCEPT
