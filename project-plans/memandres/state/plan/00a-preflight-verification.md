# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P00.5`

## Purpose
Verify assumptions about the codebase before implementation begins.

## Toolchain Verification
- [ ] `cargo --version` — Rust stable toolchain present
- [ ] `rustc --version` — Compiler present
- [ ] `cargo clippy --version` — Clippy available

## Dependency Verification
- [ ] `libc = "0.2"` present in `rust/Cargo.toml`
- [ ] `std::sync::Mutex` available (standard library, no crate needed)
- [ ] `std::ffi::CStr` available (standard library)

## Type/Interface Verification

### Rust types exist and match assumptions
- [ ] `GameState` struct exists in `rust/src/state/game_state.rs`
- [ ] `GameState::get_state(usize, usize) -> u8` method exists
- [ ] `GameState::set_state(usize, usize, u8)` method exists
- [ ] `GameState::copy_state(&mut self, usize, &GameState, usize, usize)` method exists
- [ ] `GameState::as_bytes() -> &[u8; NUM_GAME_STATE_BYTES]` method exists
- [ ] `GameState::from_bytes(&[u8; NUM_GAME_STATE_BYTES]) -> Self` method exists
- [ ] `StateFile` struct exists in `rust/src/state/state_file.rs`
- [ ] `StateFile::read(&mut self, &mut [u8]) -> Result<usize>` method exists
- [ ] `StateFile::write(&mut self, &[u8]) -> Result<()>` method exists
- [ ] `StateFile::seek(&mut self, i64, SeekWhence) -> Result<()>` method exists
- [ ] `StateFile::length(&self) -> usize` method exists
- [ ] `StateFileManager` struct exists with `open`, `close`, `delete` methods
- [ ] `GLOBAL_GAME_STATE: Mutex<Option<GameState>>` exists in `ffi.rs`
- [ ] `GLOBAL_STATE_FILES: Mutex<Option<StateFileManager>>` exists in `ffi.rs`

### Verify known blockers still present
- [ ] `StateFile::seek` clamps to `data.len()` — confirm the clamping code exists
- [ ] `rust_copy_game_state` double-locks `GLOBAL_GAME_STATE` — confirm the nested lock pattern exists
- [ ] `StateFile` has no separate `used` field — confirm `data.len()` is used for both logical and physical size
- [ ] `StateFile::open_count` is `u32` (should be `i32`) — confirm

### C types and functions exist
- [ ] `state.c` contains `GAME_STATE_FILE` struct definition
- [ ] `state.c` contains `state_files[3]` static array
- [ ] `state.c` contains `OpenStateFile` function definition
- [ ] `state.c` contains `CloseStateFile` function definition
- [ ] `state.c` contains `ReadStateFile` function definition
- [ ] `state.c` contains `WriteStateFile` function definition
- [ ] `state.c` contains `SeekStateFile` function definition
- [ ] `state.c` contains `LengthStateFile` function definition
- [ ] `state.c` contains `DeleteStateFile` function definition
- [ ] `config_unix.h` exists and contains other `USE_RUST_*` flags
- [ ] `config_unix.h` does NOT yet contain `USE_RUST_STATE`

### FFI signatures match
- [ ] `rust_open_state_file(c_int, *const c_char) -> c_int` in `ffi.rs`
- [ ] `rust_close_state_file(c_int)` in `ffi.rs`
- [ ] `rust_read_state_file(c_int, *mut u8, usize, usize) -> usize` in `ffi.rs`
- [ ] `rust_write_state_file(c_int, *const u8, usize, usize) -> usize` in `ffi.rs`
- [ ] `rust_seek_state_file(c_int, i64, c_int) -> c_int` in `ffi.rs`
- [ ] `rust_length_state_file(c_int) -> usize` in `ffi.rs`
- [ ] `rust_delete_state_file(c_int)` in `ffi.rs`
- [ ] `rust_copy_game_state(c_int, c_int, c_int)` in `ffi.rs`

## Test Infrastructure Verification
- [ ] `rust/src/state/state_file.rs` has `#[cfg(test)] mod tests` section with passing tests
- [ ] `rust/src/state/game_state.rs` has `#[cfg(test)] mod tests` section with passing tests
- [ ] `rust/src/state/ffi.rs` has `#[cfg(test)] mod tests` section with passing tests
- [ ] `cargo test --workspace --all-features` passes (all existing tests)

## Call-Path Feasibility
- [ ] `state.c` `OpenStateFile` returns `GAME_STATE_FILE*` — verify pointer arithmetic `fp - state_files` will produce valid index
- [ ] `state.h` `sread_32` calls `ReadStateFile` — confirm inline function calls through function pointer
- [ ] `grpinfo.c` uses `SeekStateFile` with offsets that may exceed current data length — confirm pattern exists
- [ ] `load_legacy.c` calls `copyGameState` on local arrays (not global) — confirm legacy loader independence

## Blocking Issues
- [ ] If `cargo test` fails on existing tests, must fix before proceeding
- [ ] If `state.c` structure doesn't match assumptions (e.g., `state_files` not static), plan needs revision

## Gate Decision
- [ ] PASS: proceed
- [ ] FAIL: revise plan
