# P08a Verification — PLAN-20260314-RESOURCE

Verdict: ACCEPT

## Scope Verified
- `/Users/acoliver/projects/uqm/project-plans/20260311/resource/.completed/P08.md`
- `/Users/acoliver/projects/uqm/project-plans/20260311/resource/plan/08-res-openresfile-sentinel.md`
- `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs`
- `/Users/acoliver/projects/uqm/sc2/src/libs/resource/filecntl.c`

## Findings

### 1. `res_OpenResFile` uses `uio_stat` + directory check before `uio_fopen`
Verified in `rust/src/resource/ffi_bridge.rs:1046-1067`.

- Declares a `libc::stat` buffer
- Calls `uio_stat(dir, filename, &mut sb)` before attempting open
- Checks directory status with `(sb.st_mode & libc::S_IFMT) == libc::S_IFDIR`
- Returns `STREAM_SENTINEL` when the target is a directory
- Falls through to `uio_fopen(dir, filename, mode)` for non-directories and when `uio_stat` fails

This matches the C reference in `sc2/src/libs/resource/filecntl.c:33-43`, which does:
- `uio_stat(..., &sb) == 0 && S_ISDIR(sb.st_mode)`
- `return ((uio_Stream *) ~0);`
- otherwise `uio_fopen(...)`

### 2. Returns `STREAM_SENTINEL` for directories
Verified in `rust/src/resource/ffi_bridge.rs:1057-1062`.

### 3. Falls through to `uio_fopen` for non-dirs and stat failures
Verified in `rust/src/resource/ffi_bridge.rs:1057-1067`.
The sentinel path is only taken when `uio_stat(...) == 0` and the mode indicates directory. All other cases reach `uio_fopen`.

### 4. `LoadResourceFromPath` rejects `STREAM_SENTINEL`
Verified in `rust/src/resource/ffi_bridge.rs:1214-1219`.
It opens via `res_OpenResFile(...)` and immediately rejects both null and `STREAM_SENTINEL`.

### 5. `LoadResourceFromPath` closes handles on zero-length failure
Verified in `rust/src/resource/ffi_bridge.rs:1222-1227`.
When `LengthResFile(fp) == 0`, it calls `res_CloseResFile(fp)` before returning null.

### 6. `uio_stat` declared in extern block with compatible ABI
Verified in `rust/src/resource/ffi_bridge.rs:39-41`:

`fn uio_stat(dir: *mut c_void, path: *const c_char, stat_buf: *mut libc::stat) -> c_int;`

This is ABI-compatible with the C usage pattern in `filecntl.c`:
- dir handle pointer
- path C string
- `struct stat *`
- integer status return

## Additional Note
A duplicated `#[cfg(test)]` attribute appears above the local test stub for `uio_stat` at `ffi_bridge.rs:141-143`. It does not affect the verified P08 behavior.

## Test Run
Command run as requested:

`cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features 2>&1 | tail -5`

Result:
- `test result: ok. 1600 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 0.11s`

## Conclusion
Phase P08 satisfies the requested checks:
- directory sentinel behavior is implemented in `res_OpenResFile`
- non-directory/stat-failure fallback behavior is preserved
- `LoadResourceFromPath` rejects sentinel handles
- zero-length failure closes the opened handle
- `uio_stat` is declared with a compatible FFI signature
