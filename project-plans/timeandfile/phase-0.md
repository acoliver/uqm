# Phase 0 - Baseline harness + logging channel

## Objective
Establish a repeatable build/run harness and a Rust logging channel that can be surfaced from the C build. This phase creates the shared infrastructure needed to prove that later Rust modules are actually invoked by the C binary.

## Test-First Methodology
1. Define verification commands and expected outputs first.
2. Implement minimal Rust/C build wiring.
3. Require symbol resolution + call-site verification + runtime log marker.

## Project Context (Self-Contained)
- Project root: `/Users/acoliver/projects/uqm`
- Rust crate: `rust/` (`rust/Cargo.toml`)
- C build entry: `sc2/build.sh`
- C source roots: `sc2/src/` and `sc2/src/uqm/`
- Build config file: `sc2/build/unix/build.config`
- Build traversal: `sc2/build/unix/recurse` (sources `Makeinfo` files)
- Expected Rust staticlib: `rust/target/release/libuqm_rust.a`
- Expected log file: `rust-bridge.log` in project root

## Scope
- Add a Rust staticlib target in `rust/Cargo.toml` (`name = "uqm_rust"`).
- Add a small Rust logging helper with C ABI.
- Add a C header for Rust exports.
- Wire the Rust staticlib into the C build system (linker + pre-build step).
- Provide a runtime call from C into Rust to write a unique log marker.

## Expected Deliverables
- Rust static library built by Cargo and linked into the C binary.
- C build uses `USE_RUST_BRIDGE` macro to call Rust log init.
- Log file contains unique marker: `RUST_BRIDGE_PHASE0_OK`.
- Symbol resolution shows Rust symbols defined in the C binary.
- Call-site verification shows C calls into Rust symbols.

## Subagent Prompt (Implementation)
You are implementing Phase 0 of a bottom-up Rust integration. Do the following:

1) Update `rust/Cargo.toml` to include:
```
[lib]
name = "uqm_rust"
crate-type = ["staticlib"]
```

2) Add Rust C-ABI functions in `rust/src/bridge_log.rs`:
- `#[no_mangle] pub extern "C" fn rust_bridge_init() -> libc::c_int` (truncate/create `rust-bridge.log`).
- `#[no_mangle] pub extern "C" fn rust_bridge_log(message: *const libc::c_char) -> libc::c_int`.
- Write `RUST_BRIDGE_PHASE0_OK` on init or first log call.

3) Add C header `sc2/src/rust_bridge.h` declaring:
```
int rust_bridge_init(void);
int rust_bridge_log(const char *message);
```

4) Update C call site (choose one early entry point such as `sc2/src/uqm/uqm.c` or `sc2/src/uqm/starcon.c`) to call `rust_bridge_init()` under `#ifdef USE_RUST_BRIDGE`.

5) Build integration:
- Add a menu option in `sc2/build/unix/build.config` under `uqm_prepare_config()` to toggle `USE_RUST_BRIDGE` and append `-DUSE_RUST_BRIDGE` to `CFLAGS`.
- Ensure `cargo build --release` runs before linking (hook in Makeinfo or build script).
- Add linker flags to include the Rust staticlib: `-L../rust/target/release -luqm_rust` (adjust relative path to project root as needed).

6) Ensure `rust-bridge.log` is truncated on each run.

Constraints:
- Keep changes minimal and reversible.
- Use `extern "C"` + `#[no_mangle]` symbols.
- Log file path must be deterministic (`./rust-bridge.log`).

## Verification Prompt
You are verifying Phase 0. Run commands and confirm symbol resolution, call-site linkage, and runtime marker. If any step fails, report failure.

Qualitative checks:
- `sc2/build/unix/build.config` has a `USE_RUST_BRIDGE` toggle and adds `-DUSE_RUST_BRIDGE`.
- C code includes `rust_bridge.h` and calls `rust_bridge_init()`.

Pass/Fail commands:
1) `cd rust && cargo build --release && cd ..`
2) `cd sc2 && ./build.sh uqm config`
3) `cd sc2 && ./build.sh uqm`
4) Symbol resolution: `nm sc2/build/uqm | rg "rust_bridge_init"` (must show `T rust_bridge_init`, not `U`).
5) Call-site verification: `objdump -d sc2/build/uqm | rg "call.*rust_bridge_init"`.
6) Run binary (manual if needed) and exit after main menu appears.
7) `rg -n "RUST_BRIDGE_PHASE0_OK" rust-bridge.log`

Success criteria:
- Build succeeds.
- `nm` shows Rust symbol defined in binary.
- `objdump` shows call site to Rust symbol.
- `rust-bridge.log` contains `RUST_BRIDGE_PHASE0_OK`.

User action required:
- If the game needs manual exit, document how to exit so verification can proceed.
