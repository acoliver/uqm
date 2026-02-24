> **NOTE**: This file's name (`25-c-removal.md`) is a historical
> artifact from a phase reorder. Canonical phase: **P26** (Guard Finalization).

# Phase 26: C Code Guarding — Complete USE_RUST_GFX Coverage

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P26`

## Prerequisites
- Required: Phase P25a (Integration Verification) completed
- Expected: Game runs correctly on Rust GFX path
- Expected: Visual equivalence confirmed
- Expected: Performance acceptable

## Requirements Implemented (Expanded)

### REQ-GUARD-070: All C Drawing-Pipeline Files Guarded
**Requirement text**: Every C drawing-pipeline source file shall be wrapped
in `#ifndef USE_RUST_GFX` / `#endif` guards so that setting
`USE_RUST_GFX=1` in `build.vars` compiles zero C drawing-pipeline code.

Loader files (gfxload.c, filegfx.c, resgfx.c, loaddisp.c) are explicitly
excluded — they compile in both modes. See `00-overview.md` "Deferred to
Future Phase" section.

Behavior contract:
- GIVEN: 36 of 41 C graphics files have guards (5 loaders excluded, see Canonical File Count Matrix)
- WHEN: `USE_RUST_GFX` is defined in `build.vars`
- THEN: No C drawing-pipeline implementations are active; all drawing symbols provided by Rust
- THEN: Loader files still compile and provide resource-loading symbols

### REQ-GUARD-080: C Fallback Preserved
**Requirement text**: The C graphics code shall remain in the repository,
guarded but not deleted. Setting `USE_RUST_GFX=0` in `build.vars` shall
compile the original C graphics path as a fallback.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is set to `'0'` in `build.vars`
- WHEN: The project is built
- THEN: The C graphics path compiles and the game runs on C graphics

Why it matters:
- Preserves ability to bisect regressions against C baseline
- No irreversible changes until the Rust path is proven in production

## Implementation Tasks

### Step 1: Verify all guards from P22/P23/P24

Confirm every C graphics file has the `#ifndef USE_RUST_GFX` guard
added in earlier phases. The 41 files to check:

**Core graphics — drawing pipeline (sc2/src/libs/graphics/):**
`dcqueue.c`, `tfb_draw.c`, `tfb_prim.c`, `cmap.c`, `context.c`,
`drawable.c`, `frame.c`, `font.c`, `gfx_common.c`, `pixmap.c`,
`intersec.c`, `boxint.c`, `bbox.c`, `clipline.c`, `widgets.c`

**Core graphics — loader files (NOT guarded, compile in both modes):**
`gfxload.c`, `resgfx.c`, `filegfx.c`, `loaddisp.c`

**SDL backend (sc2/src/libs/graphics/sdl/):**
`canvas.c`, `primitives.c`, `pure.c`, `sdl2_pure.c`, `sdl1_common.c`,
`sdl2_common.c`, `opengl.c`, `palette.c`, `png2sdl.c`, `sdluio.c`,
`2xscalers.c`, `2xscalers_mmx.c`, `2xscalers_sse.c`, `2xscalers_3dnow.c`,
`bilinear2x.c`, `biadv2x.c`, `hq2x.c`, `nearest2x.c`, `triscan2x.c`,
`rotozoom.c`, `scalers.c`

**Keep unguarded (still needed with Rust):**
- `sdl_common.c` — thin vtable shim that forwards to `rust_gfx_*`
- `gfxload.c` — resource loading, pure I/O
- `resgfx.c` — resource management
- `filegfx.c` — file loading helpers
- `loaddisp.c` — display loading
- `sdl/png2sdl.c` — PNG to SDL_Surface conversion (loader pipeline)

### Step 2: Build with USE_RUST_GFX=1

```bash
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | tee /tmp/build_rust_gfx.log
grep -c 'error:\|undefined' /tmp/build_rust_gfx.log
# Expected: 0
```

### Step 3: Build with USE_RUST_GFX=0

```bash
# Temporarily disable Rust GFX
sed -i '' "s/USE_RUST_GFX='1'/USE_RUST_GFX='0'/" sc2/build.vars
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | tee /tmp/build_c_gfx.log
grep -c 'error:\|undefined' /tmp/build_c_gfx.log
# Expected: 0

# Restore
sed -i '' "s/USE_RUST_GFX='0'/USE_RUST_GFX='1'/" sc2/build.vars
```

### Step 4: Verify both paths run

```bash
# Rust path
./uqm >/tmp/uqm_rust.log 2>&1 &
sleep 12 && kill %1
grep 'Using Rust graphics driver' /tmp/uqm_rust.log
# Expected: present

# C path (after toggling flag)
./uqm >/tmp/uqm_c.log 2>&1 &
sleep 12 && kill %1
grep -v 'Using Rust graphics driver' /tmp/uqm_c.log | grep -i 'graphics\|video' | head -5
# Expected: C graphics init messages
```

## Verification Commands

```bash
# Count guarded files
# Intentionally unguarded: sdl_common.c (vtable shim), loader files
UNGUARDED_OK="sdl_common.c gfxload.c filegfx.c resgfx.c loaddisp.c png2sdl.c"
echo "=== Guard audit ==="
for f in sc2/src/libs/graphics/*.c sc2/src/libs/graphics/sdl/*.c; do
  bn="$(basename $f)"
  if echo "$UNGUARDED_OK" | grep -qw "$bn"; then
    echo "  UNGUARDED (OK): $bn"
    continue
  fi
  if grep -q 'USE_RUST_GFX' "$f"; then
    echo "  GUARDED: $bn"
  else
    echo "  UNGUARDED: $bn *** NEEDS GUARD ***"
  fi
done

# Full test suite
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] 36 drawing-pipeline C files have USE_RUST_GFX guards (2 pre-existing + 15 from P22 + 14 from P23 + 5 from P24)
- [ ] sdl_common.c remains unguarded (vtable shim)
- [ ] 5 loader files remain unguarded (gfxload.c, resgfx.c, filegfx.c, loaddisp.c, sdl/png2sdl.c)
- [ ] Build succeeds with USE_RUST_GFX=1 (Rust path)
- [ ] Build succeeds with USE_RUST_GFX=0 (C fallback) — no undefined symbols
- [ ] No undefined symbol errors either way
- [ ] All cargo gates pass

## Dual-Path ABI Verification (Mandatory)
```bash
# Build with USE_RUST_GFX=0 (set in build.vars) and verify no undefined symbols
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | grep -c 'undefined'
# Expected: 0 (exit code is authoritative)
```

## Semantic Verification Checklist (Mandatory)
- [ ] Game starts and reaches main menu on Rust path
- [ ] Game starts and reaches main menu on C path
- [ ] Both paths play music and render correctly
- [ ] Toggle between paths is clean (just build.vars change + rebuild)
- [ ] No regressions from P24 integration testing

## Deferred Implementation Detection (Mandatory)

```bash
grep -rn "TODO\|FIXME\|HACK\|placeholder" rust/src/graphics/*_ffi.rs 2>/dev/null
# Expected: 0 matches
```

## Success Criteria
- [ ] Both build paths compile cleanly
- [ ] Both paths run the game
- [ ] All guards in place
- [ ] C code preserved but inactive when USE_RUST_GFX=1

## Failure Recovery
- rollback: `git restore sc2/src/libs/graphics/`
- blocking issues: missing Rust symbol exports

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P26.md`

Contents:
- phase ID: P26
- timestamp
- files guarded: 36 drawing-pipeline C files (list each)
- files intentionally unguarded: sdl_common.c (vtable shim), 5 loaders (gfxload.c, resgfx.c, filegfx.c, loaddisp.c, sdl/png2sdl.c)
- build verification: both paths compile
- game verification: both paths run
- guard audit: 36 guarded, 5 loaders unguarded, sdl_common.c unguarded
