# Phase 1 - File module (libs/file) C→Rust swap

## Objective
Replace `libs/file` functions (`copyFile`, `fileExists`, `fileExists2`) with Rust implementations callable from C, and prove via runtime log + symbol resolution that the Rust versions are used.

## Test-First Methodology
1. Define exact C signatures and symbol bridging strategy.
2. Implement Rust externs with the *same symbol names* used by C.
3. Exclude the original C implementations from the build when Rust is enabled.
4. Verify with symbol checks, call-site verification, and runtime behavior.

## Project Context (Self-Contained)
- Header: `sc2/src/libs/file.h`
- C implementation: `sc2/src/libs/file/files.c`
- Build traversal: `sc2/build/unix/recurse` (sources Makeinfo).
- Build config: `sc2/build/unix/build.config`.
- Makeinfo file: `sc2/src/libs/file/Makeinfo`.
- Rust crate: `rust/`.
- Log file: `rust-bridge.log` at project root.

## Build System Integration (Concrete)
- `build.config` runs in a shell environment; variables must be exported to be visible in Makeinfo.
- Add `export USE_RUST_FILE` in `build.config` after menu selection.
- In `sc2/src/libs/file/Makeinfo`, use shell `if` to set `uqm_CFILES` based on `USE_RUST_FILE`.
  - This is valid because Makeinfo is `.` (sourced) by `recurse` and evaluated by `/bin/sh`.
- Example pattern:
```
if [ "$USE_RUST_FILE" = "1" ]; then
    uqm_CFILES="dirs.c"
else
    uqm_CFILES="dirs.c files.c"
fi
```

## C Function Signatures (from `sc2/src/libs/file.h`)
```
int copyFile(uio_DirHandle *srcDir, const char *srcName,
             uio_DirHandle *dstDir, const char *newName);
bool fileExists(const char *name);
bool fileExists2(uio_DirHandle *dir, const char *fileName);
```

## Scope
- Rust `extern "C"` replacements that export *the exact C symbol names*:
  - `copyFile`, `fileExists`, `fileExists2`.
- Log markers per function:
  - `RUST_FILE_EXISTS_CALLED`, `RUST_FILE_EXISTS2_CALLED`, `RUST_COPY_FILE_CALLED`.
- Exclude `files.c` from compilation when `USE_RUST_FILE` is enabled.

## Expected Deliverables
- Rust staticlib exports `copyFile`, `fileExists`, `fileExists2` symbols.
- `files.c` excluded from build under `USE_RUST_FILE`.
- Runtime log contains Rust markers.
- Symbol resolution shows Rust symbols defined in binary.

## Subagent Prompt (Implementation)
You are implementing Phase 1. Do the following:

1) Add Rust implementations in `rust/src/io/file_bridge.rs` (or similar):
```
#[no_mangle] pub extern "C" fn fileExists(name: *const c_char) -> bool
#[no_mangle] pub extern "C" fn fileExists2(dir: *mut uio_DirHandle, file_name: *const c_char) -> bool
#[no_mangle] pub extern "C" fn copyFile(src_dir: *mut uio_DirHandle, src_name: *const c_char,
                                        dst_dir: *mut uio_DirHandle, new_name: *const c_char) -> c_int
```
- Use `libc` types and match C return values (bool/int).
- Write log markers to `rust-bridge.log` in each function.

2) Ensure Rust functions export *exact C names* (no `rust_` prefix).

3) Exclude the C implementation when `USE_RUST_FILE` is enabled:
- In `sc2/src/libs/file/Makeinfo`, set `uqm_CFILES="dirs.c"` when `USE_RUST_FILE=1`.
- Otherwise use `uqm_CFILES="dirs.c files.c"`.

4) Add `USE_RUST_FILE` toggle in `sc2/build/unix/build.config` (similar to Phase 0 bridge), and **export** `USE_RUST_FILE` for Makeinfo visibility.

5) Add a negative guard in `files.c`:
```
#ifdef USE_RUST_FILE
#error "files.c should not be compiled when USE_RUST_FILE is enabled"
#endif
```

## Verification Prompt
You are verifying Phase 1. Run commands and confirm symbol resolution + runtime markers + behavior. If any step fails, report failure.

Qualitative checks:
- `files.c` excluded when `USE_RUST_FILE` enabled.
- `nm` shows `fileExists`, `fileExists2`, `copyFile` defined in binary (not undefined).

Pass/Fail commands:
1) `cd rust && cargo build --release && cd ..`
2) `cd sc2 && ./build.sh uqm config` (enable `USE_RUST_FILE`).
3) `cd sc2 && ./build.sh uqm`
4) Verify symbols: `nm sc2/build/uqm | rg "T fileExists|T fileExists2|T copyFile"`
5) Verify C file excluded: `test ! -f sc2/build/obj*/libs/file/files.o`
6) Run the binary briefly to trigger file operations.
7) `rg -n "RUST_FILE_EXISTS_CALLED" rust-bridge.log`
8) `rg -n "RUST_FILE_EXISTS2_CALLED" rust-bridge.log`
9) `rg -n "RUST_COPY_FILE_CALLED" rust-bridge.log`

Success criteria:
- Build succeeds with `USE_RUST_FILE` enabled.
- Rust symbols are defined in binary.
- `files.c` is not compiled.
- Log markers appear during runtime.

User action required:
- If file operations are only triggered in a menu, document the steps to reach that menu.
