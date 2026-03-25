# Phase 06a: C Bridge Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P06a`

## Prerequisites
- Required: Phase 06 (C Bridge) completed
- Expected artifacts: `c_bridge.rs`, updated `ffi.rs`, updated `process.c`

## Structural Verification Checklist
- [ ] `c_bridge.rs` exists with all 44 bridge wrappers
- [ ] `ffi.rs` has `rust_battle_redraw_queue` export (with `catch_unwind`)
- [ ] `mod.rs` declares `pub mod c_bridge;`
- [ ] `collision_bridge()` removed from `process_loop.rs`
- [ ] process.c has `#ifdef USE_RUST_BATTLE_LOOP` guard structure
- [ ] Pre-P13 build/link map artifact exists

## Semantic Verification Checklist (Mandatory — Most Important)

### Bridge wrapper completeness
- [ ] **GraphicsIntegration**: all 17 wrappers present and extern "C" declarations match C header signatures
- [ ] **AudioIntegration**: all 11 wrappers present
- [ ] **ThreadingIntegration**: all 3 wrappers present
- [ ] **InputIntegration**: all 4 wrappers present
- [ ] **ResourceIntegration**: all 5 wrappers present
- [ ] **ShipRaceIntegration**: all 4+ wrappers present
- [ ] Total: 44 deferred bridge operations verified

### FFI safety (spec §10)
- [ ] **Pointer-family classification**: each wrapper's pointer args classified per spec §10.3 (always-nonnull / nullable / invalidation-sensitive)
- [ ] **Null checks**: nullable pointers checked at wrapper boundary before dereference
- [ ] **Panic containment**: C→Rust entry points (ffi.rs exports) use `catch_unwind`; Rust→C bridge calls don't need it
- [ ] **Thread affinity**: all bridge wrappers document main-thread/DoInput-thread affinity (spec §10.4)
- [ ] **Stable identity model**: no raw pointer cached across callback/re-entrant boundary (spec §10.5)

### Callback-slot migration matrix (spec §8.1)
- [ ] Matrix produced covering all 4 element callback families (preprocess, postprocess, collision, death)
- [ ] Matrix covers 2 handler/vtable families (frameInput, battleEndReady)
- [ ] Each entry: slot owner, C-only target, Rust-owned target, earliest live replacement phase
- [ ] Installation rules documented per spec §8.2

### C-side guard verification
- [ ] process.c: `#ifdef USE_RUST_BATTLE_LOOP` guards wrap function bodies correctly
- [ ] process.c: original behavior preserved when `USE_RUST_BATTLE_LOOP` is NOT defined
- [ ] process.c: `extern` declarations for Rust replacements present inside guarded blocks
- [ ] C-only build compiles and runs identically to pre-P06

### collision_bridge migration
- [ ] `process_loop.rs` calls `c_bridge::drawables_intersect()` (not a local function)
- [ ] ProcessCollisions behavior unchanged after migration
- [ ] Tests still pass after migration

### ffi.rs extension
- [ ] `rust_battle_redraw_queue` export present
- [ ] Uses `catch_unwind` for panic containment
- [ ] Calls `process_loop::redraw_queue()` correctly
- [ ] All 17 Phase 1 FFI adapters remain unchanged

## Branch-Parity Verification
- [ ] `USE_RUST_BATTLE_LOOP` guard in process.c: verified that guards are dark-code (not enabled) and do not alter C-only compilation path

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/c_bridge.rs rust/src/battle/ffi.rs
```

## Pass/Fail Gate Criteria
- **PASS:** All 44 bridge wrappers present with correct signatures. FFI safety enforced (null checks, panic containment, thread affinity documented). Callback-slot matrix produced. C-only build unchanged. collision_bridge migration clean. No TODO/FIXME/HACK.
- **FAIL:** Any bridge wrapper missing. Any pointer argument lacks appropriate safety check. C-only build broken. Phase 1 FFI adapters modified. Callback-slot matrix missing.
