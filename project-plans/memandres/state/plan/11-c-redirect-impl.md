# Phase 11: C Redirect — Implementation (Enable USE_RUST_STATE)

## Phase ID
`PLAN-20260224-STATE-SWAP.P11`

## Prerequisites
- Required: Phase P10a (Dual Build Verification) completed
- Both build configurations compile and link
- All Rust blockers fixed (seek-past-end, copy deadlock)
- All Rust tests pass

## Requirements Implemented (Expanded)

### REQ-SF-006: C Redirect Correctness
**Requirement text**: Enable `USE_RUST_STATE` so Rust handles all state file I/O.

Behavior contract:
- GIVEN: `USE_RUST_STATE` defined in `config_unix.h`
- WHEN: Game calls any state file function (Open, Close, Read, Write, Seek, Length, Delete)
- THEN: Rust FFI handles the operation; C implementation is not compiled

### REQ-SF-009: Feature Flag Isolation
**Requirement text**: Enable `USE_RUST_STATE` by default.

## Implementation Tasks

### Files to modify

1. **`sc2/config_unix.h`**
   - Uncomment `#define USE_RUST_STATE`
   - marker: `@plan PLAN-20260224-STATE-SWAP.P11`
   - marker: `@requirement REQ-SF-009`

### Build and runtime verification

1. **Full build with Rust path**:
   ```bash
   cd rust && cargo build --release
   cd sc2 && make clean && make
   ```

2. **Game launch smoke test**:
   - Launch the game binary
   - Verify title screen appears
   - Start a new game
   - Save the game (slot 00)
   - Quit
   - Relaunch and load the save
   - Verify game state is restored correctly

3. **Legacy save compatibility** (if legacy saves are available):
   - Place a legacy save (`starcon2.00`) in the save directory
   - Launch game and load it
   - Verify it loads without crash

### FFI signature alignment check
Before enabling, verify that every Rust FFI function signature matches the C declaration:

| C Declaration (`rust_state_ffi.h`) | Rust Function (`ffi.rs`) |
|---|---|
| `int rust_open_state_file(int, const char*)` | `pub extern "C" fn rust_open_state_file(file_index: c_int, mode: *const c_char) -> c_int` |
| `void rust_close_state_file(int)` | `pub extern "C" fn rust_close_state_file(file_index: c_int)` |
| `void rust_delete_state_file(int)` | `pub extern "C" fn rust_delete_state_file(file_index: c_int)` |
| `size_t rust_length_state_file(int)` | `pub extern "C" fn rust_length_state_file(file_index: c_int) -> usize` |
| `size_t rust_read_state_file(int, uint8_t*, size_t, size_t)` | `pub extern "C" fn rust_read_state_file(file_index: c_int, buf: *mut u8, size: usize, count: usize) -> usize` |
| `size_t rust_write_state_file(int, const uint8_t*, size_t, size_t)` | `pub extern "C" fn rust_write_state_file(file_index: c_int, buf: *const u8, size: usize, count: usize) -> usize` |
| `int rust_seek_state_file(int, int64_t, int)` | `pub extern "C" fn rust_seek_state_file(file_index: c_int, offset: i64, whence: c_int) -> c_int` |

## Verification Commands

```bash
# Full build
cd rust && cargo build --release
cd sc2 && make clean && make

# Rust tests
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify USE_RUST_STATE is enabled
grep -q "^#define USE_RUST_STATE" sc2/config_unix.h && echo "ENABLED"
```

## Structural Verification Checklist
- [ ] `USE_RUST_STATE` uncommented/enabled in `config_unix.h`
- [ ] Build succeeds with Rust path
- [ ] All Rust tests pass
- [ ] `cargo fmt` and `cargo clippy` clean
- [ ] No undefined reference errors at link time

## Semantic Verification Checklist (Mandatory)
- [ ] Game launches without crash
- [ ] New game can be started (state files initialized)
- [ ] Game save works (state files read for serialization)
- [ ] Game load works (state files written from save data)
- [ ] `InitPlanetInfo` / `UninitPlanetInfo` work (STARINFO_FILE)
- [ ] `GetPlanetInfo` / `PutPlanetInfo` work (STARINFO_FILE seek/read/write)
- [ ] `GetGroupInfo` / `PutGroupInfo` work (RANDGRPINFO/DEFGRPINFO)
- [ ] State file data round-trips through save/load correctly

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/state/ || echo "CLEAN"
```

## Success Criteria
- [ ] `USE_RUST_STATE` is enabled
- [ ] Full build succeeds
- [ ] Game launches and runs with Rust state file I/O
- [ ] Save/load works correctly
- [ ] No deferred implementation markers

## Failure Recovery
- rollback: comment out `#define USE_RUST_STATE` in config_unix.h
- If game crashes: check `rust_open_state_file` return value handling
- If save/load fails: compare state file buffer contents (Rust vs C) for the same operations
- Debug: add logging to Rust FFI functions to trace calls

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P11.md`

Contents:
- phase ID: P11
- files modified: `sc2/config_unix.h`
- changes: USE_RUST_STATE enabled
- build: full build succeeds
- runtime: game launches, new game, save, load all work
- verification: cargo fmt/clippy/test all clean
