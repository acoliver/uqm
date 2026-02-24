# Phase 21: C Bridge Wiring

## Phase ID
`PLAN-20260224-RES-SWAP.P21`

## Prerequisites
- Required: Phase 20a (Init/Index/UIO Implementation Verification) completed
- Expected: All 38 Rust `extern "C"` functions implemented and tested

## Requirements Implemented (Expanded)

### REQ-RES-R014: Compiled as Part of libuqm_rust.a
### Integration Contract: Wire C Build to Use Rust Resource System

## Implementation Tasks

### Integration Contract

#### Existing Callers
- `uqm/setup.c` → `InitResourceSystem()`
- `uqm/uqm.c` → `LoadResourceIndex()`, `res_GetInteger()`, etc.
- `uqm/setupmenu.c` → `res_PutInteger()`, `SaveResourceIndex()`, etc. (44 calls)
- `libs/input/sdl/input.c` → `LoadResourceIndex()`, `res_GetString()`, etc. (11 calls)
- `libs/graphics/resgfx.c` → `res_GetResource()`, `res_DetachResource()`
- `libs/sound/resinst.c` → `res_GetResource()`, `res_DetachResource()`
- `libs/strings/sresins.c` → `res_GetResource()`, `res_DetachResource()`
- `libs/video/vresins.c` → `res_GetResource()`, `res_DetachResource()`
- `uqm/dummy.c` → `res_GetResource()`, `res_DetachResource()`
- **200+ call sites total** — all continue calling the same function names

#### Existing Code Replaced/Removed
When `USE_RUST_RESOURCE` is defined:
- `libs/resource/resinit.c` — ALL functions guarded (init, types, config get/put, save)
- `libs/resource/getres.c` — ALL functions guarded (get, free, detach, load)
- `libs/resource/propfile.c` — ALL functions guarded (parser)
- `libs/resource/loadres.c` — ALL functions guarded (GetResourceData)
- `libs/resource/filecntl.c` — ALL functions guarded (file I/O wrappers)
- `libs/resource/stringbank.c` — Remains in C (used by string table loaders)
- `libs/resource/direct.c` — Remains in C (directory scanning)

#### User Access Path
- Game startup: `main()` → `uqm_init()` → `InitResourceSystem()` → Rust
- Config load: `LoadResourceIndex(configDir, "uqm.cfg", "config.")` → Rust
- Content load: `loadIndices(contentDir)` → Rust
- Settings menu: `res_PutInteger("config.sfxvol", 20)` → Rust
- Settings save: `SaveResourceIndex(configDir, "uqm.cfg", "config.", TRUE)` → Rust
- Asset load: `LoadGraphic(res)` → `res_GetResource(res)` → Rust → C loadFun

### Files to modify

#### C files: Add `#ifndef USE_RUST_RESOURCE` guards

1. `sc2/src/libs/resource/resinit.c`
   - Guard ALL function bodies with `#ifndef USE_RUST_RESOURCE`
   - Keep includes and forward declarations unconditional
   - The `process_resource_desc` callback, `UseDescriptorAsRes`,
     `DescriptorToInt`, `DescriptorToBoolean`, `DescriptorToColor`,
     `RawDescriptor`, `IntToString`, `BooleanToString`, `ColorToString`
     — all guarded (Rust provides equivalents)
   - `curResourceIndex` static — guarded (Rust owns global state)

2. `sc2/src/libs/resource/getres.c`
   - Guard: `res_GetResource`, `res_FreeResource`, `res_DetachResource`,
     `lookupResourceDesc`, `loadResourceDesc`, `LoadResourceFromPath`,
     `_cur_resfile_name`

3. `sc2/src/libs/resource/propfile.c`
   - Guard: `PropFile_from_string`, `PropFile_from_file`,
     `PropFile_from_filename`

4. `sc2/src/libs/resource/loadres.c`
   - Guard: `GetResourceData`, `FreeResourceData`

5. `sc2/src/libs/resource/filecntl.c`
   - Guard: `res_OpenResFile`, `res_CloseResFile`, `ReadResFile`,
     `WriteResFile`, `GetResFileChar`, `PutResFileChar`, `PutResFileNewline`,
     `SeekResFile`, `TellResFile`, `LengthResFile`, `DeleteResFile`

#### Pattern for guards:
```c
#ifndef USE_RUST_RESOURCE
// ... entire original function body ...
#endif /* !USE_RUST_RESOURCE */
```

#### Build system
- `sc2/config_unix.h` — `USE_RUST_RESOURCE` already defined
- Verify `Makefile` / `build.sh` links `libuqm_rust.a`
- Verify no duplicate symbol errors (C functions guarded out when Rust provides them)

#### Update existing rust_resource.c
- `sc2/src/libs/resource/rust_resource.c`
  - Guard with `#ifdef USE_RUST_RESOURCE`
  - Remove old sidecar cache functions (RustResourceInit etc.)
  - This file is no longer needed since Rust exports are now the
    actual API functions (not a sidecar)
  - Alternatively: leave the file but have it compile to nothing
    when USE_RUST_RESOURCE is defined (the Rust exports take over)

### Verification procedure

1. **Build with USE_RUST_RESOURCE=1 (default)**:
   - C resource files compile to empty (guarded out)
   - Rust provides all 38 symbols
   - No duplicate symbol errors
   - No undefined symbol errors

2. **Build with USE_RUST_RESOURCE=0**:
   - C resource files compile normally
   - Rust symbols not used
   - Original behavior preserved

3. **Run with USE_RUST_RESOURCE=1**:
   - Game starts (InitResourceSystem succeeds)
   - Config loaded (uqm.cfg parsed)
   - Content loaded (uqm.rmp and all .rmp files parsed)
   - Menu displayed (fonts, graphics loaded via type dispatch)
   - Settings menu works (get/put roundtrip)

## Verification Commands

```bash
# Build with Rust resource system
cd sc2 && ./build.sh uqm 2>&1 | tail -20

# Check for undefined symbols
cd sc2 && ./build.sh uqm 2>&1 | grep -i "undefined"
# Expected: 0

# Check for duplicate symbols
cd sc2 && ./build.sh uqm 2>&1 | grep -i "duplicate\|multiple definition"
# Expected: 0

# Rust quality gates
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] resinit.c fully guarded
- [ ] getres.c fully guarded
- [ ] propfile.c fully guarded
- [ ] loadres.c fully guarded
- [ ] filecntl.c fully guarded
- [ ] stringbank.c NOT guarded (still needed by C)
- [ ] direct.c NOT guarded (still needed by C)
- [ ] Build succeeds with USE_RUST_RESOURCE=1
- [ ] Build succeeds with USE_RUST_RESOURCE=0

## Semantic Verification Checklist
- [ ] Game starts with Rust resource system
- [ ] Config values loaded correctly (test a few known keys)
- [ ] Content resources loaded (graphics appear on screen)
- [ ] Settings menu works (change a setting, save, reload)
- [ ] Addon loading works (if addons present)
- [ ] No regressions with USE_RUST_RESOURCE=0

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK" rust/src/resource/ffi_bridge.rs
# Expected: 0
```

## Success Criteria
- [ ] Both build configurations succeed
- [ ] Game runs with Rust resource system
- [ ] No symbol conflicts

## Failure Recovery
- rollback C guards: `git checkout -- sc2/src/libs/resource/`
- rollback Rust: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P21.md`
