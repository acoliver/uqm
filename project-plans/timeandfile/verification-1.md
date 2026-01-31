# Phase 1 Verification - File module C→Rust swap

## Purpose
Confirm that the C executable calls Rust replacements for `copyFile`, `fileExists`, and `fileExists2` and logs each invocation.

## Qualitative Checks
- `USE_RUST_FILE` is enabled in `sc2/build/unix/build.config` and adds `-DUSE_RUST_FILE`.
- `USE_RUST_FILE` is exported for Makeinfo evaluation.
- `files.c` is excluded when `USE_RUST_FILE=1` (no object file for it).
- Rust exports `copyFile`, `fileExists`, `fileExists2` symbols.

## Pass/Fail Commands
1) `cd rust && cargo build --release && cd ..`
2) `cd sc2 && ./build.sh uqm config` (enable `USE_RUST_FILE`).
3) `cd sc2 && ./build.sh uqm`
4) Symbol resolution: `nm sc2/build/uqm | rg "T fileExists|T fileExists2|T copyFile"`
5) Exclusion check: `test ! -f sc2/build/obj*/libs/file/files.o`
6) Call-site check (Linux): `objdump -d sc2/build/uqm | rg "call.*fileExists"`
7) Call-site check (macOS fallback): `otool -tV sc2/build/uqm | rg "fileExists"`
8) Run the binary briefly to trigger file checks.
9) Log markers:
   - `rg -n "RUST_FILE_EXISTS_CALLED" rust-bridge.log`
   - `rg -n "RUST_FILE_EXISTS2_CALLED" rust-bridge.log`
   - `rg -n "RUST_COPY_FILE_CALLED" rust-bridge.log`

## Stronger Functional Test (Behavior Divergence)
- Temporarily hardcode Rust `fileExists("/tmp/uqm_probe")` to return `true` and log `RUST_FILE_PROBE_TRUE`.
- Temporarily hardcode C `fileExists` to return `false` for the same path (or rely on C exclusion).
- Run the game once and verify:
  - `rg -n "RUST_FILE_PROBE_TRUE" rust-bridge.log`
- Revert the hardcoded divergence after verification.

## Expected Results
- All commands succeed.
- Rust symbols defined in binary.
- `files.c` not compiled.
- Log markers present.

## Failure Conditions
- Build fails, symbols undefined, or log markers missing.

## User Action Needed
- If file operations only trigger after entering menus, note exact steps.
