> **NOTE**: This file's name (`26-final-verification.md`) is a historical
> artifact from a phase reorder. Canonical phase: **P27** (Final Verification).

# Phase 27: Final Verification — Rust GFX Path Complete

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P27`

## Prerequisites
- Required: Phase P26a (C Code Guarding Verification) completed
- Expected: All ~37 C drawing-pipeline files guarded with USE_RUST_GFX
  (4 loader files intentionally unguarded — see 00-overview.md)
- Expected: Both Rust and C paths build and run
- Expected: Game runs on Rust-only graphics path

## Requirements Verified

### REQ-COMPAT-060: Drawing-Pipeline Port Verification
**Requirement text**: The Rust GFX backend shall handle all drawing-pipeline
operations that were previously performed by C code when `USE_RUST_GFX=1`.
Resource-loading code (gfxload.c, filegfx.c, resgfx.c, loaddisp.c)
remains in C and compiles in both modes.

Verification:
- Binary analysis confirms Rust provides all drawing-pipeline symbols
- All rendering paths exercised
- Game fully playable
- Loader files compile and function in both modes

### REQ-COMPAT-070: Regression-Free
**Requirement text**: The Rust GFX port shall not introduce any visual
regressions, crashes, or performance degradation compared to the original
C implementation.

Verification:
- Side-by-side visual comparison (Rust vs C path)
- Performance benchmarking

## Verification Tasks

### Task 1: Symbol Verification

```bash
# Build with USE_RUST_GFX=1
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm

# Verify Rust provides all graphics symbols
nm -gU rust/target/release/libuqm_rust.a 2>/dev/null | grep -E 'rust_(gfx|dcq|canvas|cmap|frame|context|drawable|font)_' | wc -l
# Expected: >= 50

# Verify NO C drawing-pipeline objects compiled (loaders are expected)
find sc2/obj -name '*.o' 2>/dev/null | while read f; do
  base=$(basename "$f" .o)
  case "$base" in
    dcqueue|tfb_draw|tfb_prim|canvas|primitives|cmap|context|drawable|frame|font|widgets|pixmap|gfx_common|clipline|boxint|bbox|intersec)
      echo "C DRAWING-PIPELINE OBJECT FOUND: $f" ;;
  esac
done
# Expected: no output
# Note: gfxload.o, filegfx.o, resgfx.o, loaddisp.o are EXPECTED (loader files)
```

### Task 2: Full Cargo Verification

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

### Task 3: Comprehensive Scene Testing

Manual verification — play through these scenarios:

**Startup sequence:**
- [ ] Splash screen renders
- [ ] Main menu renders with all items
- [ ] Menu navigation (up/down/select) works
- [ ] Settings menu renders

**Gameplay scenes:**
- [ ] New game → intro sequence plays
- [ ] Star map renders with stars, lines, labels
- [ ] Hyperspace travel animation works
- [ ] Planet approach shows correct planet sprite
- [ ] Orbit view shows rotating planet
- [ ] Surface scan shows minerals, bio readings
- [ ] Landing on planet works, surface exploration renders

**Combat:**
- [ ] Space combat renders both ships
- [ ] Projectiles visible and correctly animated
- [ ] Explosions render
- [ ] Ship rotation smooth
- [ ] Combat UI (health bars, etc.) renders

**Dialogue:**
- [ ] Alien portraits render correctly
- [ ] Text renders correctly (no garbled characters)
- [ ] Response options visible and selectable
- [ ] Dialogue transitions work

**Effects:**
- [ ] Fade to black works
- [ ] Fade to white works
- [ ] Crossfade transitions work
- [ ] Screen flash effects work

**Resolution/Window:**
- [ ] Windowed mode works
- [ ] Fullscreen toggle works
- [ ] Correct aspect ratio maintained

### Task 4: C Path Comparison

```bash
# Build C path for comparison
sed -i '' "s/USE_RUST_GFX='1'/USE_RUST_GFX='0'/" sc2/build.vars
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm

# Run C path, note visual behavior
./uqm >/tmp/uqm_c_final.log 2>&1 &
sleep 15 && kill %1

# Restore Rust path
sed -i '' "s/USE_RUST_GFX='0'/USE_RUST_GFX='1'/" sc2/build.vars

# Compare: both logs should show similar resource loading and no errors
diff <(grep -E 'loaded|error|panic' /tmp/uqm_c_final.log) \
     <(grep -E 'loaded|error|panic' /tmp/uqm_rust_final.log)
```

### Task 5: Deferred Pattern Audit

```bash
echo "=== Deferred pattern audit ==="
for f in rust/src/graphics/*.rs; do
  hits=$(grep -cn "todo!\|TODO\|FIXME\|HACK\|placeholder\|unimplemented!" "$f" 2>/dev/null)
  if [ "$hits" -gt 0 ]; then
    echo "FAIL: $f ($hits deferred patterns)"
    grep -n "todo!\|TODO\|FIXME\|HACK\|placeholder\|unimplemented!" "$f"
  fi
done
echo "=== Audit complete ==="
```

### Task 6: Code Metrics

```bash
echo "=== Rust GFX code size ==="
wc -l rust/src/graphics/*.rs 2>/dev/null | tail -1

echo "=== FFI exports ==="
grep -r '#\[no_mangle\]' rust/src/graphics/ | wc -l
# Expected: >= 50

echo "=== Test count ==="
grep -r '#\[test\]' rust/src/graphics/ | wc -l
# Expected: >= 80
```

## Structural Verification Checklist
- [ ] >= 50 Rust FFI exports (drawing-pipeline only; loaders stay in C)
- [ ] Zero C drawing-pipeline implementations active when USE_RUST_GFX=1 (36 files guarded, 5 loaders compile in both modes)
- [ ] Loader .o files (gfxload, filegfx, resgfx, loaddisp) present in both modes
- [ ] All cargo gates pass (fmt, clippy, test)
- [ ] >= 80 total tests across graphics modules
- [ ] Zero deferred patterns in Rust graphics code
- [ ] C path still builds with USE_RUST_GFX=0

## Semantic Verification Checklist (Mandatory)
- [ ] Game is fully playable from start to any game scene
- [ ] All visual effects work (fade, transition, flash)
- [ ] All UI elements render (menus, widgets, text)
- [ ] Combat is functional and visual
- [ ] Dialogue screens work
- [ ] Performance is acceptable
- [ ] Resolution/fullscreen management works
- [ ] C fallback path still works when toggled

## Success Criteria — Definition of Done

The Full Rust GFX Drawing-Pipeline Port is **COMPLETE** when all of the
following are true:

1. **All drawing-pipeline C files guarded**: ~37 C files behind `#ifndef USE_RUST_GFX`
2. **Loader files compile in both modes**: gfxload.c, filegfx.c, resgfx.c, loaddisp.c unguarded
3. **All tests pass**: `cargo test` + build gates green
4. **Game playable**: Manual scene walkthrough completed on Rust path
5. **C fallback works**: Toggling `USE_RUST_GFX=0` still builds and runs
6. **No deferred patterns**: Zero `todo!`/`FIXME`/`HACK` in graphics code
7. **Clean build**: No warnings from `cargo clippy` or C compiler
8. **All drawing-pipeline FFI bridges complete**: vtable (17) + DCQ (~12) + canvas (~10) + colormap (~8) + frame/context/drawable (~28) = ~75 exports (loader bridges deferred)

**Note**: Drawing-pipeline C code is NOT deleted. It remains in the
repository behind `#ifdef` guards for reference and fallback. Loader code
stays active. Deletion is a future decision once the Rust path has proven
stable in production.

## Failure Recovery
- rollback: restore `build.vars` to `USE_RUST_GFX='0'`
- partial rollback: remove specific file guards to re-enable C code
  for individual subsystems

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P27.md`

Contents:
- phase ID: P27
- timestamp: completion date
- guard count: ~37 drawing-pipeline C files guarded, 4 loader files unguarded
- test count: total across all modules
- scene walkthrough: all items checked
- code metrics: Rust LoC, FFI export count
- C fallback: confirmed working
- PLAN STATUS: **COMPLETE**
- next steps: optimization, advanced features (scanlines, GL backend),
  eventual C code removal when Rust path is proven
