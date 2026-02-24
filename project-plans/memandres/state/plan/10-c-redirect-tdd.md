# Phase 10: C Redirect — TDD (Dual Build Verification)

## Phase ID
`PLAN-20260224-STATE-SWAP.P10`

## Prerequisites
- Required: Phase P09a (Stub Verification) completed
- Expected: C redirect scaffolding in place, default build works

## Requirements Implemented (Expanded)

### REQ-SF-006: C Redirect Correctness
**Requirement text**: Both build paths (C and Rust) must compile and produce correct symbols.

Behavior contract:
- GIVEN: `USE_RUST_STATE` is NOT defined
- WHEN: `make` is run
- THEN: Build succeeds using original C state file implementations

- GIVEN: `USE_RUST_STATE` IS defined
- WHEN: `make` is run
- THEN: Build succeeds using Rust FFI redirects (linking to Rust static library)

### REQ-SF-009: Feature Flag Isolation
**Requirement text**: Toggling `USE_RUST_STATE` switches between C and Rust implementations.

## Implementation Tasks

### Build test procedure

This phase is a build-system verification phase, not a unit test phase. The "tests" are:

1. **Build with C path (USE_RUST_STATE off)**:
   ```bash
   # Ensure USE_RUST_STATE is commented out in config_unix.h
   cd sc2 && make clean && make
   ```
   Expected: Build succeeds. All symbols resolved from C code.

2. **Build with Rust path (USE_RUST_STATE on)**:
   ```bash
   # Uncomment USE_RUST_STATE in config_unix.h
   cd rust && cargo build --release
   cd sc2 && make clean && make
   ```
   Expected: Build succeeds. State file symbols redirect to Rust library.
   If link errors occur: identify missing symbols and fix FFI declarations.

3. **Verify Rust cargo tests still pass**:
   ```bash
   cd rust && cargo test --workspace --all-features
   ```

4. **Verify the binary launches** (smoke test):
   ```bash
   # With USE_RUST_STATE off: launch game, verify title screen appears
   # With USE_RUST_STATE on: launch game, verify title screen appears
   ```

### Files to modify
- `sc2/config_unix.h` — temporarily uncomment `USE_RUST_STATE` for build test
  - marker: `@plan PLAN-20260224-STATE-SWAP.P10`

### Link verification
- The Rust static library (`libuqm_rust.a` or equivalent) must export all 7 `rust_*` symbols
- Verify with: `nm -g rust/target/release/libuqm_rust.a | grep rust_.*state_file`

## Verification Commands

```bash
# C path build
grep -q "^/\* #define USE_RUST_STATE" sc2/config_unix.h && echo "C path"
cd sc2 && make clean && make

# Rust path build
# (temporarily enable USE_RUST_STATE)
cd rust && cargo build --release
# Check symbols exported
nm -g rust/target/release/libuqm_rust.a 2>/dev/null | grep "rust_.*state" || echo "Check lib path"

# Rust tests
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] C path builds successfully (USE_RUST_STATE off)
- [ ] Rust library compiles successfully
- [ ] Rust library exports all 7 FFI symbols
- [ ] Rust tests all pass

## Semantic Verification Checklist (Mandatory)
- [ ] C path: build produces working binary (at minimum, compiles and links)
- [ ] Rust path: build compiles and links (FFI symbols resolve)
- [ ] No symbol conflicts between C and Rust implementations
- [ ] No undefined references when USE_RUST_STATE is enabled

## Success Criteria
- [ ] Both build configurations compile and link
- [ ] Rust FFI symbols verified in static library
- [ ] No link errors in either configuration
- [ ] Config flag properly controls which implementation is used

## Failure Recovery
- rollback: `git checkout -- sc2/config_unix.h`
- Link errors: check `rust_state_ffi.h` declarations match Rust `#[no_mangle]` signatures
- Missing symbols: verify `ffi.rs` exports are not behind `#[cfg(test)]`

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P10.md`

Contents:
- phase ID: P10
- build: both C and Rust paths compile and link
- symbols: all 7 rust_*_state_file functions exported
- rust tests: all pass
- note: USE_RUST_STATE left DISABLED after this phase
