# Phase 4 - UIO core C→Rust swap

## Objective
Replace core `uio_*` file APIs with Rust implementations callable from C, and prove via runtime log + symbol resolution that the Rust versions are used across file/sound/video/string loaders.

## Test-First Methodology
1. Define exact C signatures, constants, and handle layouts.
2. Implement Rust externs exporting the exact C symbol names.
3. Exclude the original C implementations from the build when Rust is enabled.
4. Verify with symbol checks, call-site verification, and runtime behavior.

## Project Context (Self-Contained)
- C headers: `sc2/src/libs/uio/uio.h`, `sc2/src/libs/uio/uio_types.h`
- C implementations: `sc2/src/libs/uio/*.c` (notably `uio.c`, `stdio.c`, `uio_stdlib.c`)
- Build traversal: `sc2/build/unix/recurse` (sources `Makeinfo` files)
- Makeinfo file: `sc2/src/libs/uio/Makeinfo`
- Build config: `sc2/build/unix/build.config`
- Rust crate: `rust/`
- Log file: `rust-bridge.log` (runtime markers)

## Scope
- Rust `extern "C"` replacements that export the exact C symbol names:
  - `uio_open`, `uio_close`, `uio_read`, `uio_write`
  - `uio_fopen`, `uio_fclose`
  - `uio_fseek`, `uio_ftell`
  - `uio_fread`, `uio_fgets`
  - `uio_fstat`, `uio_unlink`
- Log markers per function (single line per call):
  - `RUST_UIO_OPEN`, `RUST_UIO_CLOSE`, `RUST_UIO_READ`, `RUST_UIO_WRITE`
  - `RUST_UIO_FOPEN`, `RUST_UIO_FCLOSE`
  - `RUST_UIO_FSEEK`, `RUST_UIO_FTELL`
  - `RUST_UIO_FREAD`, `RUST_UIO_FGETS`
  - `RUST_UIO_FSTAT`, `RUST_UIO_UNLINK`
- Exclude C UIO implementations when `USE_RUST_UIO` is enabled.

## Build System Integration (Concrete)
- Add `USE_RUST_UIO` toggle in `sc2/build/unix/build.config` (similar to Phase 1/2).
- Export `USE_RUST_UIO` so Makeinfo sees it.
- In `sc2/src/libs/uio/Makeinfo`, exclude C UIO sources when `USE_RUST_UIO=1`.
- Add a negative guard in key C sources (e.g., `uio.c`):
```
#ifdef USE_RUST_UIO
#error "uio.c should not be compiled when USE_RUST_UIO is enabled"
#endif
```

## Rust Implementation Notes
- Add module `rust/src/io/uio_bridge.rs` (or similar) implementing all externs.
- Map `uio_DirHandle`, `uio_Stream`, `uio_Handle` to opaque Rust structs.
- Use `libc` types (`c_int`, `c_char`, `c_long`, `size_t`) for ABI parity.
- Implement minimal internal backing using `std::fs::File` and `std::io` primitives.
- For `uio_fopen` vs `uio_open`, ensure consistent behavior with existing C semantics.
- Maintain `errno` on failure to match C call sites.

## Expected Deliverables
- Rust staticlib exports all `uio_*` symbols above.
- C `uio*.c` sources excluded when `USE_RUST_UIO` enabled.
- Runtime log contains `RUST_UIO_*` markers during normal game startup.
- Symbol resolution shows Rust symbols defined in the binary.

## Subagent Prompt (Implementation)
You are implementing Phase 4. Do the following:
1) Implement Rust externs for all listed `uio_*` functions with correct signatures.
2) Add logging markers for each function.
3) Add `USE_RUST_UIO` toggle to build config and export it.
4) Update `sc2/src/libs/uio/Makeinfo` to exclude C sources when enabled.
5) Add compile guards to C UIO sources to prevent accidental inclusion.
6) Rebuild and report any errors.

## Verification Prompt
You are verifying Phase 4. Run commands and confirm symbol resolution + runtime markers + behavior. If any step fails, report failure.

Pass/Fail commands:
1) `cd rust && cargo build --release && cd ..`
2) `cd sc2 && ./build.sh uqm config` (enable `USE_RUST_UIO`)
3) `cd sc2 && ./build.sh uqm`
4) Symbol resolution: `nm sc2/uqm-debug | rg "T uio_open|T uio_fopen|T uio_fread|T uio_fstat"`
5) Exclusion check: `test ! -f sc2/obj*/src/libs/uio/uio.c.o`
6) Run the game to load resources (menus, sounds, fonts).
7) Log markers:
   - `rg -n "RUST_UIO_" sc2/rust-bridge.log`

Success criteria:
- Build succeeds with `USE_RUST_UIO` enabled.
- Rust symbols are defined in binary.
- C UIO sources are not compiled.
- Log markers appear during runtime.

User action required:
- Run the game and exit cleanly after main menu appears.
