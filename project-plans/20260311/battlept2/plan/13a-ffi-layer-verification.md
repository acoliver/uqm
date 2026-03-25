# Phase 13a: FFI Layer Phase 3 Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P13a`

## Prerequisites
- Required: Phase 13 (FFI Layer Phase 3) completed
- Expected artifacts: `rust_battle_wrappers.c`, updated `ffi.rs`, guards on 5+ C files

## Structural Verification Checklist
- [ ] `rust_battle_wrappers.c` exists with all wrapper functions
- [ ] `ffi.rs` has all Phase 3 exports alongside Phase 1 exports
- [ ] All C files have `USE_RUST_BATTLE_LOOP` guards
- [ ] Retained boundary functions (11) NOT guarded
- [ ] Build system includes wrapper file
- [ ] Both build modes compile (C-only and Rust-enabled)

## Semantic Verification Checklist (Mandatory — Most Important)

### Build-mode coexistence
- [ ] **C-only build** (no USE_RUST_BATTLE_LOOP): all original C function bodies compile and execute unchanged; no Rust code involvement
- [ ] **Rust-enabled build** (USE_RUST_BATTLE_LOOP defined): ported function bodies guarded out; wrappers delegate to Rust; retained boundaries still in C

### DoBattle thin-shell (spec §4)
- [ ] battle.c: `#ifdef USE_RUST_BATTLE_LOOP` wraps DoBattle body
- [ ] Thin-shell body: calls `rust_battle_frame()` with appropriate args
- [ ] Returns rust_battle_frame() result to DoInput unchanged
- [ ] C-only mode: original DoBattle body active
- [ ] No duplicate frame logic, rendering logic, or lifecycle branching in shell

### rust_battle_frame per-frame sequence (battle.c DoBattle:258-354)
- [ ] SetMenuSounds(MENU_SOUND_NONE, MENU_SOUND_NONE)
- [ ] NETPLAY_CHECKSUM: CRC computation via Phase 1 netplay.rs functions
- [ ] ProcessInput (calls lifecycle::process_input)
- [ ] BatchGraphics / frame callback / UnbatchGraphics sequence
- [ ] RedrawQueue(TRUE) — calls process_loop::redraw_queue
- [ ] ScreenTransition (first-time only)
- [ ] Battle speed timing: nth_frame HIBYTE → SleepThreadUntil
- [ ] Activity flag mutation
- [ ] Max-speed rendering skip (simulation runs, rendering skipped)

### Symbol-provider matrix (spec §5.2)
- [ ] `Battle()`: provided by `rust_battle_wrappers.c` → `rust_battle_entry()`
- [ ] `computer_intelligence()`: provided by `rust_battle_wrappers.c` → `rust_computer_intelligence()`
- [ ] `InitShips()` / `UninitShips()`: provided by wrappers
- [ ] `InitSpace()` / `UninitSpace()`: provided by wrappers
- [ ] `BattleSong()` / `FreeBattleSong()`: provided by wrappers
- [ ] `GetPlayerOrder()`: provided by wrappers
- [ ] Internal-only symbols (ProcessInput, etc.): Rust-owned, no wrapper needed
- [ ] Retained boundaries (11): C-owned, not wrapped

### FFI safety
- [ ] Every `#[no_mangle] pub extern "C"` export has `catch_unwind`
- [ ] Panic → deterministic error return (not UB)
- [ ] Pointer arguments validated in exports

### C guard verification per file
- [ ] **process.c**: 7 ported functions guarded; dark-code guards from P06 now complete
- [ ] **battle.c**: DoBattle thin-shell + Battle/ProcessInput/selectAllShips/etc. guarded
- [ ] **ship.c**: 6 ported functions guarded (separate from existing USE_RUST_SHIPS guards)
- [ ] **tactrans.c**: ported functions guarded; retained boundaries NOT guarded
- [ ] **intel.c**: computer_intelligence guarded
- [ ] **init.c**: InitShips/UninitShips/InitSpace/UninitSpace/CountCrewElements guarded; load_animation/free_image/BuildSIS NOT guarded

### CRC-32 determinism (spec §Cross-language frame determinism)
- [ ] For a given input sequence: C path and Rust path produce identical CRC-32 per frame
- [ ] Element processing order identical
- [ ] Integer arithmetic only (no float substitution)
- [ ] RNG call order identical

## Branch-Parity Verification
- [ ] `USE_RUST_BATTLE_LOOP`: master toggle correctly gates all ported function bodies
- [ ] `NETPLAY_CHECKSUM`: CRC computation in rust_battle_frame matches C DoBattle
- [ ] Max-speed rendering skip: simulation-always, rendering-conditional

## Verification Commands

```bash
# Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C (both modes)
# C-only: make clean && make
# Rust-enabled: make clean && CFLAGS=-DUSE_RUST_BATTLE_LOOP make

# Deferred impl detection
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/ffi.rs sc2/src/uqm/rust_battle_wrappers.c
```

## Pass/Fail Gate Criteria
- **PASS:** Both build modes compile. DoBattle thin shell delegates to rust_battle_frame correctly. All FFI exports have catch_unwind. Symbol-provider matrix complete. CRC-32 determinism verified. All 11 retained boundaries accessible. No TODO/FIXME/HACK.
- **FAIL:** Either build mode fails to compile. DoBattle shell has logic beyond delegation. Any FFI export missing catch_unwind. Symbol missing provider. CRC mismatch. Retained boundary guarded out.
