# Phase 26: Final Verification — Zero C Graphics Code

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P26`

## Prerequisites
- Required: Phase P25a (C Code Removal Verification) completed
- Expected: All C graphics code removed from build
- Expected: Game runs on Rust-only graphics path
- Expected: Build succeeds with no C graphics objects

## Requirements Verified

### REQ-COMPAT-060: Complete Port Verification
**Requirement text**: The final Rust GFX backend shall handle all graphics
operations that were previously performed by C code, with zero C graphics
code compiled into the binary.

Verification:
- Binary analysis confirms no C graphics symbols
- All rendering paths exercised
- Game fully playable

### REQ-COMPAT-070: Regression-Free
**Requirement text**: The Rust GFX port shall not introduce any visual
regressions, crashes, or performance degradation compared to the original
C implementation.

Verification:
- Side-by-side visual comparison
- Automated frame buffer comparison (where feasible)
- Performance benchmarking

## Verification Tasks

### Task 1: Binary Analysis — Zero C Graphics

```bash
# Build final binary
cd sc2 && make clean && make 2>&1 | tee /tmp/build_final.log
grep -c 'error:' /tmp/build_final.log
# Expected: 0

# Check for ANY remaining C graphics symbols in the binary
nm -g ./uqm 2>/dev/null | grep -E 'TFB_Draw(Screen|Canvas)_|SDL_.*Canvas|gfx_scale_' | head -20
# Expected: NONE (zero matches)

# Verify only Rust graphics symbols
nm -g ./uqm 2>/dev/null | grep -E 'rust_(gfx|dcq|canvas|cmap|gfxload)_' | wc -l
# Expected: >= 55

# Verify C graphics source files are not in build
find sc2/build -name '*.o' 2>/dev/null | while read f; do
  base=$(basename "$f" .o)
  case "$base" in
    dcqueue|tfb_draw|tfb_prim|clipline|boxint|bbox|cmap|context|drawable|frame|pixmap|intersec|gfx_common)
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

# Total test count across all FFI modules
echo "=== Test counts ==="
echo "ffi.rs:       $(grep -c '#\[test\]' rust/src/graphics/ffi.rs)"
echo "dcq_ffi.rs:   $(grep -c '#\[test\]' rust/src/graphics/dcq_ffi.rs)"
echo "canvas_ffi.rs: $(grep -c '#\[test\]' rust/src/graphics/canvas_ffi.rs)"
echo "cmap_ffi.rs:  $(grep -c '#\[test\]' rust/src/graphics/cmap_ffi.rs)"
echo "gfxload_ffi.rs: $(grep -c '#\[test\]' rust/src/graphics/gfxload_ffi.rs)"
# Expected total: >= 80
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
- [ ] Warp/hyperspace visual effects work

**Resolution/Window:**
- [ ] Windowed mode works
- [ ] Fullscreen toggle works
- [ ] Window resize works (if supported)
- [ ] Correct aspect ratio maintained

### Task 4: Performance Benchmark

```bash
# Benchmark: 60 seconds of gameplay
./uqm --benchmark 60 2>&1 | tee /tmp/benchmark_final.txt
grep FPS /tmp/benchmark_final.txt
# Record final FPS and compare to pre-port baseline
```

### Task 5: Memory Leak Check (if valgrind available)

```bash
# Linux only — skip on macOS
valgrind --leak-check=full --show-leak-kinds=all ./uqm --quit-after-frame 500 2>&1 | tee /tmp/valgrind_final.txt
grep -E 'definitely lost|indirectly lost' /tmp/valgrind_final.txt
# Expected: 0 bytes lost from Rust code
```

### Task 6: Deferred Pattern Audit

```bash
# Final sweep: no deferred patterns in ANY Rust graphics file
echo "=== Deferred pattern audit ==="
for f in rust/src/graphics/*.rs rust/src/graphics/**/*.rs; do
  hits=$(grep -cn "todo!\|TODO\|FIXME\|HACK\|placeholder\|unimplemented!" "$f" 2>/dev/null)
  if [ "$hits" -gt 0 ]; then
    echo "FAIL: $f ($hits deferred patterns)"
    grep -n "todo!\|TODO\|FIXME\|HACK\|placeholder\|unimplemented!" "$f"
  fi
done
echo "=== Audit complete ==="
```

### Task 7: Code Metrics

```bash
# Lines of Rust graphics code
echo "=== Rust GFX code size ==="
wc -l rust/src/graphics/*.rs rust/src/graphics/**/*.rs 2>/dev/null | tail -1
# Record total

# Lines of C graphics code remaining
echo "=== C GFX code remaining ==="
wc -l sc2/src/libs/graphics/*.c sc2/src/libs/graphics/**/*.c 2>/dev/null | tail -1
# Expected: near zero (only shim files like sdl_common.c)

# FFI export count
echo "=== FFI exports ==="
grep -r '#\[no_mangle\]' rust/src/graphics/ | wc -l
# Expected: >= 55
```

## Structural Verification Checklist
- [ ] Zero C graphics symbols in final binary
- [ ] >= 55 Rust FFI exports in binary
- [ ] Zero C graphics object files in build
- [ ] All cargo gates pass (fmt, clippy, test)
- [ ] >= 80 total tests across all FFI modules
- [ ] Zero deferred patterns in Rust graphics code

## Semantic Verification Checklist (Mandatory)
- [ ] Game is fully playable from start to any game scene
- [ ] All visual effects work (fade, transition, flash)
- [ ] All UI elements render (menus, widgets, text)
- [ ] Combat is functional and visual
- [ ] Dialogue screens work
- [ ] Performance is acceptable (within 5% of C baseline)
- [ ] No memory leaks from Rust code
- [ ] Resolution/fullscreen management works

## Success Criteria — Definition of Done

The Full Rust GFX Port is **COMPLETE** when all of the following are true:

1. **Zero C graphics code compiled**: `nm` shows no `TFB_Draw*` symbols
2. **All tests pass**: `cargo test` + build gates green
3. **Game playable**: Manual scene walkthrough completed
4. **Performance acceptable**: Within 5% of C baseline
5. **No deferred patterns**: Zero `todo!`/`FIXME`/`HACK` in graphics code
6. **Clean build**: No warnings from `cargo clippy` or C compiler
7. **All 41 C files processed**: Deleted or reduced to thin shims
8. **All FFI bridges complete**: vtable (17) + DCQ (~15) + canvas (~10) +
   colormap (~8) + gfxload (~5) = ~55 exports

## Failure Recovery
- rollback: restore C files from git history
- partial rollback: re-enable specific C files by removing their
  `USE_RUST_GFX` guards

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P26.md`

Contents:
- phase ID: P26
- timestamp: completion date
- binary analysis: zero C graphics symbols confirmed
- test count: total across all modules
- scene walkthrough: all items checked
- performance: final FPS measurement
- code metrics: Rust LoC, C LoC remaining, FFI export count
- PLAN STATUS: **COMPLETE**
- next steps: optimization opportunities, advanced features (scanlines, GL backend)
