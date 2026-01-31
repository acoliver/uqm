# Phase 0 Verification - Baseline harness + logging channel

## Purpose
Verify that the C build links Rust code and that the executable calls into Rust to emit a unique log marker.

## Binary Paths
- Expected binary: `sc2/build/uqm` (relative to repo root)
- If build uses a different output path, capture it from build logs.

## Qualitative Checks
- `rust/Cargo.toml` defines `staticlib` with `name = "uqm_rust"`.
- `sc2/build/unix/build.config` includes a `USE_RUST_BRIDGE` toggle and appends `-DUSE_RUST_BRIDGE` to `CFLAGS`.
- `USE_RUST_BRIDGE` is exported for Makeinfo evaluation (check `export USE_RUST_BRIDGE` or equivalent in build scripts).
- C code includes `sc2/src/rust_bridge.h` and calls `rust_bridge_init()` under `#ifdef USE_RUST_BRIDGE`.
- C build log shows `-L../rust/target/release -luqm_rust` (or equivalent).

## Pass/Fail Commands
1) `cd rust && cargo build --release && cd ..`
2) `cd sc2 && ./build.sh uqm config`
3) `cd sc2 && ./build.sh uqm`
4) Symbol resolution: `nm sc2/build/uqm | rg "rust_bridge_init"` (must show `T rust_bridge_init`, not `U`).
5) Call-site verification (Linux): `objdump -d sc2/build/uqm | rg "call.*rust_bridge_init"`
6) Call-site verification (macOS fallback): `otool -tV sc2/build/uqm | rg "rust_bridge_init"`
7) Source verification: `rg -n "rust_bridge_init\(" sc2/src`
8) Run binary (manual if needed) and exit after main menu appears.
9) `rg -n "RUST_BRIDGE_PHASE0_OK" rust-bridge.log`

## Expected Results
- All commands succeed.
- `nm` shows Rust symbol defined.
- `objdump` or `otool` shows a call or reference to the Rust symbol.
- Log contains `RUST_BRIDGE_PHASE0_OK`.

## Failure Conditions
- Build fails, symbol undefined, or log marker missing.

## User Action Needed
- If the game requires manual exit, document the exit steps used.
