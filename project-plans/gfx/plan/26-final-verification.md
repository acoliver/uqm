# Phase 26: Final Verification — Rust GFX Path Complete

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P26`

## Prerequisites
- Required: Phase P25a (C Code Guarding Verification) completed
- Expected: All 41 C graphics files guarded with USE_RUST_GFX
- Expected: Both Rust and C paths build and run
- Expected: Game runs on Rust-only graphics path

## Requirements Verified

### REQ-COMPAT-060: Complete Port Verification
**Requirement text**: The Rust GFX backend shall handle all graphics
operations that were previously performed by C code when `USE_RUST_GFX=1`.

Verification:
- Binary analysis confirms Rust provides all graphics symbols
- All rendering paths exercised
- Game fully playable

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
nm -gU rust/target/release/libuqm_rust.a 2>/dev/null | grep -E 'rust_(gfx|dcq|canvas|cmap|gfxload)_' | wc -l
# Expected: >= 55

# Verify NO C graphics objects compiled
find sc2/obj -name '*.o' 2>/dev/null | while read f; do
  base=$(basename "$f" .o)
  case "$base" in
    dcqueue|tfb_draw|tfb_prim|canvas|primitives|cmap|context|drawable|frame)
      echo "C GFX OBJECT FOUND: $f" ;;
  esac
done
# Expected: no output
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
# Expected: >= 55

echo "=== Test count ==="
grep -r '#\[test\]' rust/src/graphics/ | wc -l
# Expected: >= 80
```

## Structural Verification Checklist
- [ ] >= 55 Rust FFI exports
- [ ] Zero C graphics object files when USE_RUST_GFX=1
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

The Full Rust GFX Port is **COMPLETE** when all of the following are true:

1. **All C files guarded**: 41/41 C graphics files behind `#ifndef USE_RUST_GFX`
2. **All tests pass**: `cargo test` + build gates green
3. **Game playable**: Manual scene walkthrough completed on Rust path
4. **C fallback works**: Toggling `USE_RUST_GFX=0` still builds and runs
5. **No deferred patterns**: Zero `todo!`/`FIXME`/`HACK` in graphics code
6. **Clean build**: No warnings from `cargo clippy` or C compiler
7. **All FFI bridges complete**: vtable + DCQ + canvas + colormap + gfxload = ~55 exports

**Note**: C code is NOT deleted. It remains in the repository behind
`#ifdef` guards for reference and fallback. Deletion is a future decision
once the Rust path has proven stable in production.

## Failure Recovery
- rollback: restore `build.vars` to `USE_RUST_GFX='0'`
- partial rollback: remove specific file guards to re-enable C code
  for individual subsystems

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P26.md`

Contents:
- phase ID: P26
- timestamp: completion date
- guard count: 41/41 C files guarded
- test count: total across all modules
- scene walkthrough: all items checked
- code metrics: Rust LoC, FFI export count
- C fallback: confirmed working
- PLAN STATUS: **COMPLETE**
- next steps: optimization, advanced features (scanlines, GL backend),
  eventual C code removal when Rust path is proven
