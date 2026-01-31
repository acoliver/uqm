# Phase 4 Verification - UIO core C→Rust swap

## Purpose
Confirm that the C executable calls Rust replacements for core `uio_*` functions and logs each invocation.

## Qualitative Checks
- `USE_RUST_UIO` is enabled in `sc2/build/unix/build.config` and adds `-DUSE_RUST_UIO`.
- `USE_RUST_UIO` is exported for Makeinfo evaluation.
- C UIO sources excluded when `USE_RUST_UIO=1` (no object files for `uio.c`, `stdio.c`).
- Rust exports `uio_open`, `uio_fopen`, `uio_fread`, `uio_fstat` symbols.

## Pass/Fail Commands
1) `cd rust && cargo build --release && cd ..`
2) `cd sc2 && ./build.sh uqm config` (enable `USE_RUST_UIO`).
3) `cd sc2 && ./build.sh uqm`
4) Symbol resolution:
   - `nm sc2/uqm-debug | rg "T uio_open|T uio_fopen|T uio_fread|T uio_fstat"`
5) Exclusion checks:
   - `test ! -f sc2/obj*/src/libs/uio/uio.c.o`
   - `test ! -f sc2/obj*/src/libs/uio/stdio.c.o`
6) Run the game to load menus, fonts, and sounds.
7) Log markers:
   - `rg -n "RUST_UIO_" sc2/rust-bridge.log`

## Success Criteria
- Build succeeds with `USE_RUST_UIO` enabled.
- Rust symbols are defined in binary.
- C UIO sources are excluded.
- Log markers appear during runtime.

## Failure Conditions
- Build fails, symbols undefined, or log markers missing.

## User Action Needed
- Exit the game cleanly after the main menu appears.
