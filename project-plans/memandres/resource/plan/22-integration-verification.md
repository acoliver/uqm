# Phase 22: Integration Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P22`

## Prerequisites
- Required: Phase 21a (C Bridge Wiring Verification) completed
- Expected: Full resource system wired and functional

## Purpose

This is the final integration and verification phase. It confirms
end-to-end correctness of the Rust resource system replacement across
all major resource operations, config workflows, addon loading, and
error handling. This phase produces no new code — it is purely
verification.

## End-to-End Test Scenarios

### Scenario 1: Clean Startup
```
1. Delete ~/.uqm/uqm.cfg (force defaults)
2. Launch game with USE_RUST_RESOURCE=1
3. Verify: InitResourceSystem completes
4. Verify: Default config values loaded (from uqm.cfg template or defaults)
5. Verify: uqm.rmp parsed (963+ entries)
6. Verify: Menu key bindings loaded (menu.key)
7. Verify: Flight key bindings loaded (uqm.key / flight.cfg)
8. Verify: Main menu renders correctly
```

### Scenario 2: Config Read/Write Roundtrip
```
1. Launch game
2. Open settings menu
3. Change: SFX volume to 50, fullscreen toggle
4. Save settings
5. Quit game
6. Relaunch game
7. Verify: SFX volume is 50, fullscreen matches saved value
8. Verify: uqm.cfg file is readable and parseable
```

### Scenario 3: Resource Loading (Type Dispatch)
```
1. Start a new game
2. Verify: Graphics load (GFXRES via C _GetCelData)
3. Verify: Fonts load (FONTRES via C _GetFontData)
4. Verify: Music plays (MUSICRES via C _GetMusicData)
5. Verify: Sound effects play (SNDRES via C _GetSoundBankData)
6. Verify: String tables load (STRTAB via C _GetStringData)
7. Verify: Enter a conversation → CONVERSATION type loads
8. Verify: Ship combat → SHIP type loads
```

### Scenario 4: Addon Loading
```
1. Install 3domusic addon (if available)
2. Launch with --addon=3domusic
3. Verify: Music entries overridden (addon paths used)
4. Verify: Music plays from addon files
5. Verify: Non-overridden entries still work from base content
```

### Scenario 5: Key Binding Override
```
1. Launch game
2. Open key binding settings
3. Change a binding
4. Save
5. Verify: flight.cfg updated
6. Relaunch, verify binding persists
```

### Scenario 6: Error Handling
```
1. Try to access nonexistent resource key → NULL returned, warning logged
2. Try to free an unloaded resource → warning logged, no crash
3. Try to detach multi-referenced resource → NULL returned, warning logged
4. Missing .rmp file → silently skipped (no crash)
5. Malformed .rmp entry → warning logged, entry skipped
```

### Scenario 7: Behavioral Parity
```
1. Build with USE_RUST_RESOURCE=0 (C mode)
2. Run through scenarios 1-5, record behavior
3. Build with USE_RUST_RESOURCE=1 (Rust mode)
4. Run through scenarios 1-5, record behavior
5. Compare: Must be identical (config values, resource loading, gameplay)
```

## Verification Commands

```bash
# Full quality gate
cd rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Build verification (both modes)
cd sc2
# Mode 1: Rust resource system
./build.sh uqm 2>&1 | tail -5   # Should show success
# Mode 0: C resource system (edit config to unset USE_RUST_RESOURCE)
./build.sh uqm 2>&1 | tail -5   # Should show success
```

## Structural Verification Checklist
- [ ] All 38 extern "C" functions implemented
- [ ] All C files properly guarded
- [ ] No placeholder markers anywhere in resource code
- [ ] All plan phase markers present

## Semantic Verification Checklist
- [ ] Game starts and reaches main menu
- [ ] Config read/write roundtrip works
- [ ] All 9 resource types load correctly via type dispatch
- [ ] Addon override works
- [ ] Key bindings save/restore
- [ ] Error handling matches C behavior (warnings, not crashes)
- [ ] No memory leaks (valgrind or equivalent)
- [ ] Behavioral parity with C mode confirmed

## Plan Completion Criteria

The resource system migration is COMPLETE when:

1. **38 extern "C" functions** are implemented in Rust and pass all tests
2. **5 C files** (resinit.c, getres.c, propfile.c, loadres.c, filecntl.c)
   are fully guarded with `#ifndef USE_RUST_RESOURCE`
3. **200+ C call sites** work transparently without modification
4. **Config persistence** (save/load) produces compatible files
5. **Type-specific loaders** (GFXRES, FONTRES, etc.) are correctly
   dispatched from Rust to C function pointers
6. **UIO integration** works for all file I/O
7. **Addon loading** with key override semantics is preserved
8. **Both build modes** (USE_RUST_RESOURCE=0 and =1) compile and link
9. **All quality gates** pass (fmt, clippy, test)
10. **No regressions** compared to C-only build

## Files Changed (Full Plan)

### Rust files created
- `rust/src/resource/ffi_types.rs` — `#[repr(C)]` FFI types
- `rust/src/resource/ffi_bridge.rs` — 38 extern "C" function exports
- `rust/src/resource/type_registry.rs` — Type handler registry
- `rust/src/resource/dispatch.rs` — Resource get/free/detach/remove

### Rust files modified
- `rust/src/resource/mod.rs` — Module declarations
- `rust/src/resource/propfile.rs` — Fixed parser (TYPE:path, case-sensitive)
- `rust/src/resource/index.rs` — Fixed index (resource_type field, case-sensitive)
- `rust/src/resource/resource_type.rs` — Color parser (rgb/rgba/rgb15)
- `rust/src/resource/resource_system.rs` — Config Put API, SaveResourceIndex

### C files modified (guards only)
- `sc2/src/libs/resource/resinit.c` — `#ifndef USE_RUST_RESOURCE`
- `sc2/src/libs/resource/getres.c` — `#ifndef USE_RUST_RESOURCE`
- `sc2/src/libs/resource/propfile.c` — `#ifndef USE_RUST_RESOURCE`
- `sc2/src/libs/resource/loadres.c` — `#ifndef USE_RUST_RESOURCE`
- `sc2/src/libs/resource/filecntl.c` — `#ifndef USE_RUST_RESOURCE`

### C files unchanged
- `sc2/src/libs/resource/stringbank.c` — Still needed by C string table loaders
- `sc2/src/libs/resource/direct.c` — Still needed by C directory scanning
- `sc2/src/libs/resource/rust_resource.c` — Obsoleted (sidecar cache no longer needed)
- `sc2/src/libs/resource/index.h` — Struct definitions (still needed by C loaders)
- `sc2/src/libs/reslib.h` — Public API (unchanged, C callers use same names)

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P22.md`

Contents:
- Plan ID: PLAN-20260224-RES-SWAP
- Final phase
- All verification outputs
- Semantic verification summary
- PASS/FAIL decision
