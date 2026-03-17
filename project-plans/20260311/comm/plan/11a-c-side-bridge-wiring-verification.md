# Phase 11a: C-Side Bridge Wiring Verification

## Phase ID
`PLAN-20260314-COMM.P11a`

## Prerequisites
- Required: Phase 11 completed

## Verification Commands

```bash
# Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C — both build modes
cd /Users/acoliver/projects/uqm/sc2

# Build with USE_RUST_COMM (normal mode)
make clean && make 2>&1 | tail -30

# Build without USE_RUST_COMM (C fallback — comment out config_unix.h:86-87)
# (manual verification step — restore after)
```

## Structural Verification Checklist
- [ ] Every guarded section in comm.c has matching `#ifdef USE_RUST_COMM` Rust call-through
- [ ] commglue.c conditionally routes all script API functions
- [ ] commglue.h macros route PHRASE_ENABLED/DISABLE_PHRASE correctly
- [ ] commanim.c fully guarded with Rust stubs
- [ ] rust_comm.c has wrapper for every high-level comm function
- [ ] rust_comm.h declares every Rust FFI export used by C
- [ ] No `#ifdef USE_RUST_COMM` block is missing its `#else` fallback

## Semantic Verification Checklist

### Build Validation
- [ ] `test_rust_build_succeeds` — `cargo build` clean
- [ ] `test_c_build_rust_mode` — full C build with USE_RUST_COMM succeeds
- [ ] `test_c_build_c_mode` — full C build without USE_RUST_COMM succeeds
- [ ] `test_no_link_errors` — no undefined symbols at link time
- [ ] `test_no_duplicate_symbols` — no multiply-defined symbols

### Race Script Compilation
- [ ] All 27 race directories produce .o files
- [ ] Spot-check: arilouc.o, starbas.o, orzc.o, zoqfotc.o, melnormc.o
- [ ] No modification to any file under `sc2/src/uqm/comm/*/`

### API Routing
- [ ] `test_race_communication_routes_to_rust` — `RaceCommunication` → `rust_RaceCommunication`
- [ ] `test_init_communication_routes_to_rust` — `InitCommunication` → `rust_InitCommunication`
- [ ] `test_npc_phrase_routes_to_rust` — NPCPhrase → rust_NPCPhrase_cb
- [ ] `test_response_routes_to_rust` — DoResponsePhrase → rust_DoResponsePhrase
- [ ] `test_segue_routes_to_rust` — setSegue → rust_SetSegue
- [ ] `test_phrase_enable_routes_to_rust` — PHRASE_ENABLED → rust_PhraseEnabled
- [ ] `test_animation_routes_to_rust` — ProcessCommAnimations → rust_ProcessCommAnimations

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h sc2/src/uqm/comm.c sc2/src/uqm/commglue.c sc2/src/uqm/commglue.h sc2/src/uqm/commanim.c
```

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P11a.md`
