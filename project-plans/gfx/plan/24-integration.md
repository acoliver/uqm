# Phase 24: Integration — End-to-End Testing

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P24`

## Prerequisites
- Required: Phase P23a (Widget + GfxLoad Verification) completed
- Expected: All ~35 drawing-pipeline C files guarded (5 loader files + sdl_common.c unguarded)
- Expected: All drawing-pipeline FFI bridges implemented (vtable, DCQ, canvas, colormap, frame/context/drawable)
- Expected: Build succeeds with `USE_RUST_GFX=1`

## Requirements Implemented (Expanded)

### REQ-COMPAT-030: Visual Equivalence
**Requirement text**: The Rust GFX backend, when compiled with
`USE_RUST_GFX=1`, shall produce visually equivalent output to the C
backend for all game scenes.

Behavior contract:
- GIVEN: Same game state (save file, input sequence)
- WHEN: Rendered by Rust path vs C path
- THEN: Frame buffers are pixel-identical (or within tolerance for
  floating-point differences in scaling/blending)

### REQ-COMPAT-040: Performance Equivalence
**Requirement text**: The Rust GFX backend shall maintain at least the
same frame rate as the C backend under identical conditions.

Behavior contract:
- GIVEN: Same hardware, same game scene
- WHEN: FPS measured over 60-second interval
- THEN: Rust FPS >= 0.95 × C FPS (within 5% tolerance)

### REQ-SEQ-010: Full Call Sequence (End-to-End)
**Requirement text**: The complete rendering pipeline shall work:
C game logic → DCQ enqueue → DCQ flush → canvas draw → surface pixels →
vtable compositing → SDL present.

Behavior contract:
- GIVEN: Game is running with `USE_RUST_GFX=1`
- WHEN: A frame is rendered
- THEN: All stages execute correctly, frame is visible

### REQ-COMPAT-050: Scene Coverage
**Requirement text**: The following game scenes shall render correctly:
1. Main menu with all menu items visible
2. Star map with constellation lines
3. Planet orbit with rotating planet
4. Surface scan with mineral dots
5. Space combat with ship sprites and projectiles
6. Dialogue screen with alien portrait and text
7. Screen transitions (fade to black, crossfade)

## Verification Tasks

### Task 1: Build Verification

```bash
# Clean build with full Rust GFX
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | tee /tmp/build_integration.log
grep -c 'error:\|undefined' /tmp/build_integration.log
# Expected: 0

# Verify no C graphics object files compiled
find sc2/build -name '*.o' | xargs nm 2>/dev/null | grep -c 'TFB_DrawScreen_Line\|TFB_DrawCanvas_Line'
# Expected: 0 (or only from non-guarded files)
```

### Task 2: Symbol Completeness

```bash
# All Rust FFI symbols
cd rust && cargo build --release
nm -gU target/release/libuqm_rust.a 2>/dev/null | grep -E 'rust_(gfx|dcq|canvas|cmap|frame|context|drawable|font)_' | sort | tee /tmp/rust_symbols.txt
wc -l /tmp/rust_symbols.txt
# Expected: >= 50 drawing-pipeline symbols (loaders excluded)

# Cross-reference with C declarations
grep 'rust_' sc2/src/libs/graphics/sdl/rust_gfx.h | grep -v '//' | wc -l
# Expected: matches symbol count
```

### Task 3: Rendering Pipeline Test

```bash
# Launch game with Rust GFX, capture startup
./uqm --logfile /tmp/uqm_integration.log 2>&1 &
UQM_PID=$!
sleep 15
kill $UQM_PID 2>/dev/null

# Check log for successful init
grep -c 'Rust GFX init\|rust_gfx_init.*success' /tmp/uqm_integration.log
# Expected: >= 1

# Check for errors
grep -ic 'error\|panic\|crash\|segfault' /tmp/uqm_integration.log
# Expected: 0
```

### Task 4: Frame Buffer Verification

Run with `SDL_VIDEODRIVER=dummy` (headless) if available, or use
screenshot comparison:

```bash
# Visual equivalence is verified MANUALLY by running both paths
# and comparing the same game scene side-by-side.
# UQM does not currently have --screenshot or --quit-after-frame flags.
#
# Procedure:
# 1. Build C path (USE_RUST_GFX=0), run, navigate to test scene, take OS screenshot
# 2. Build Rust path (USE_RUST_GFX=1), run same scene, take OS screenshot
# 3. Compare visually (or with imagemagick if screenshots captured)
#
# If automated visual testing is needed, add a screenshot harness first.
```

### Task 5: Scene Walkthrough

Manual verification checklist (requires display):
- [ ] Main menu renders all items
- [ ] Star map shows constellation lines and labels
- [ ] Planet orbit shows rotating planet sprite
- [ ] Combat shows ships, projectiles, explosions
- [ ] Dialogue shows alien portrait, text, response options
- [ ] Fade transitions work (black fade, white flash)
- [ ] Menu navigation (keyboard/mouse) responds correctly
- [ ] Resolution/fullscreen toggle works

### Task 6: Performance Comparison

```bash
# UQM does not currently have a --benchmark flag.
# Performance comparison is done manually:
# 1. Build C path, run, observe SHOWFPS output in log (if enabled)
# 2. Build Rust path, run same scene, observe SHOWFPS output
# 3. Compare FPS values; Rust should be within 5% of C
#
# If automated benchmarking is needed, add a benchmark harness first.
```

### Task 7: Cargo Gates

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Build with `USE_RUST_GFX=1` succeeds with zero errors
- [ ] All expected Rust symbols exported (>= 50 drawing-pipeline)
- [ ] C header declarations match Rust exports
- [ ] No C graphics object code in `USE_RUST_GFX=1` build
- [ ] All cargo gates pass

## Semantic Verification Checklist (Mandatory)
- [ ] Game starts and reaches main menu on Rust path
- [ ] Rendering pipeline executes: DCQ → canvas → surface → vtable → present
- [ ] Fade effects work (fade to black, fade to white)
- [ ] Screen transitions work (crossfade between screens)
- [ ] Sprites render correctly (ships, portraits, UI elements)
- [ ] Text renders correctly (menus, dialogue, labels)
- [ ] No visual glitches, tearing, or black frames
- [ ] Performance within 5% of C path

## Success Criteria
- [ ] Game runs on Rust GFX path without crashes
- [ ] Visual output is equivalent to C path
- [ ] Performance is acceptable
- [ ] All build and test gates pass
- [ ] Scene walkthrough checklist completed

## Failure Recovery
- rollback: switch to C path (`rm -rf obj/release/src/libs/graphics && ./build.sh uqm`)
- blocking issues: visual differences → compare frame buffers pixel-by-pixel
  to identify which rendering stage diverges

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P24.md`

Contents:
- phase ID: P24
- timestamp
- build verification: success
- symbol count: N
- scene walkthrough: results
- performance: Rust FPS vs C FPS
- visual equivalence: pass/fail with notes
- verification: cargo suite output
