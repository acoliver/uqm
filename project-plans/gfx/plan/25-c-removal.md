# Phase 25: C Code Removal

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P25`

## Prerequisites
- Required: Phase P24a (Integration Verification) completed
- Expected: Game runs correctly on Rust GFX path
- Expected: Visual equivalence confirmed
- Expected: Performance acceptable

## Requirements Implemented (Expanded)

### REQ-GUARD-070: Remove C Fallback Path
**Requirement text**: After integration verification confirms the Rust path
is functionally equivalent, the `#ifndef USE_RUST_GFX` fallback code in
guarded C files shall be removed.

Behavior contract:
- GIVEN: The Rust path is verified working
- WHEN: C fallback code is removed
- THEN: `USE_RUST_GFX` is always defined; C graphics code is deleted

### REQ-GUARD-080: Unconditional Rust Path
**Requirement text**: After C code removal, `USE_RUST_GFX` shall be
defined unconditionally (or the guards removed entirely), making the Rust
path the only graphics implementation.

Behavior contract:
- GIVEN: All C graphics fallback code is removed
- WHEN: The project is built
- THEN: Only Rust graphics code compiles; no C graphics objects produced

## Implementation Tasks

### Step 1: Make USE_RUST_GFX unconditional

Modify the build system to always define `USE_RUST_GFX`:

```bash
# In Makefile / build.sh / CMakeLists.txt:
# Remove: -DUSE_RUST_GFX (optional flag)
# Add: -DUSE_RUST_GFX=1 (always defined)
# Or: remove all #ifdef USE_RUST_GFX guards entirely
```

### Step 2: Remove guarded C code

For each of the 41 guarded C files, remove the `#ifndef USE_RUST_GFX` block:

**Before:**
```c
#ifdef USE_RUST_GFX
/* Replaced by Rust implementation */
#else
/* ... hundreds of lines of C code ... */
#endif
```

**After:**
```c
/* This file's functionality is now provided by the Rust GFX backend.
 * See rust/src/graphics/ for the implementation.
 * 
 * Original C implementation removed in PLAN-20260223-GFX-FULL-PORT.P25.
 */
```

### Step 3: Remove or minimize C files

For files that become empty stubs after guard removal:
- **Option A**: Delete the file entirely and remove from build system
- **Option B**: Keep a tombstone comment (as shown above) for git history

Recommended: Option A for most files, Option B for `sdl_common.c` (which
still has vtable wiring) and any file that still has unguarded code.

### C files to process (41 total)

**Delete entirely (pure C graphics, fully replaced):**
- `dcqueue.c` — replaced by `dcqueue.rs` + `dcq_ffi.rs`
- `tfb_draw.c` — replaced by `tfb_draw.rs` + `canvas_ffi.rs`
- `tfb_prim.c` — replaced by `tfb_draw.rs`
- `clipline.c` — replaced by `tfb_draw.rs`
- `boxint.c` — replaced by `tfb_draw.rs`
- `bbox.c` — replaced by `tfb_draw.rs`
- `cmap.c` — replaced by `cmap.rs` + `cmap_ffi.rs`
- `context.c` — replaced by `context.rs`
- `drawable.c` — replaced by `drawable.rs`
- `frame.c` — replaced by `frame.rs`
- `pixmap.c` — replaced by `pixmap.rs`
- `intersec.c` — replaced by Rust intersection logic
- `gfx_common.c` — replaced by `gfx_common.rs`
- `sdl/canvas.c` — replaced by `canvas_ffi.rs`
- `sdl/primitives.c` — replaced by `tfb_draw.rs`
- `sdl/2xscalers.c` — replaced by `scaling.rs`
- `sdl/2xscalers_mmx.c` — replaced by `scaling.rs`
- `sdl/2xscalers_sse.c` — replaced by `scaling.rs`
- `sdl/2xscalers_3dnow.c` — replaced by `scaling.rs`
- `sdl/bilinear2x.c` — replaced by `scaling.rs`
- `sdl/biadv2x.c` — replaced by `scaling.rs`
- `sdl/hq2x.c` — replaced by `scaling.rs`
- `sdl/nearest2x.c` — replaced by `scaling.rs`
- `sdl/triscan2x.c` — replaced by `scaling.rs`
- `sdl/rotozoom.c` — replaced by `scaling.rs`
- `sdl/sdl2_pure.c` — replaced by `ffi.rs`
- `sdl/sdl1_common.c` — dead code
- `sdl/pure.c` — replaced by `ffi.rs`
- `gfxload.c` — replaced by `gfxload_ffi.rs`
- `resgfx.c` — replaced by Rust resource management
- `filegfx.c` — replaced by Rust file loading
- `loaddisp.c` — replaced by Rust display loading
- `sdl/png2sdl.c` — replaced by Rust PNG loading
- `font.c` — replaced by Rust font system
- `widgets.c` — bridged or ported in P23

**Keep but simplify (still have non-graphics code or vtable wiring):**
- `sdl/sdl_common.c` — vtable wiring (thin shim calling Rust)
- `sdl/scalers.c` — may have non-graphics scaler selection logic
- `sdl/sdl2_common.c` — may have shared SDL2 utilities
- `sdl/opengl.c` — may be needed for future GL support
- `sdl/sdluio.c` — UIO integration (may have non-graphics parts)
- `sdl/palette.c` — may have shared palette utilities

### Build system changes

Remove deleted files from:
- `Makefile` / `CMakeLists.txt` / `Makefile.build` (depending on build system)
- Any `SOURCES` or `OBJS` lists

Update link order to ensure `libuqm_rust.a` provides all symbols.

## Verification Commands

```bash
# Build without any C graphics files
cd sc2 && make clean && make 2>&1 | tee /tmp/build_no_c_gfx.log
grep -c 'error:\|undefined' /tmp/build_no_c_gfx.log
# Expected: 0

# Verify no C graphics objects
find sc2/build -name '*.o' | while read f; do
  if nm "$f" 2>/dev/null | grep -q 'TFB_DrawScreen_\|TFB_DrawCanvas_'; then
    echo "FOUND C GFX: $f"
  fi
done
# Expected: no output

# Verify Rust provides all symbols
cd rust && cargo build --release
nm -gU target/release/libuqm_rust.a 2>/dev/null | grep -E 'rust_(gfx|dcq|canvas|cmap|gfxload)_' | wc -l
# Expected: >= 55

# Full test suite
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Game smoke test
./uqm --logfile /tmp/uqm_no_c_gfx.log 2>&1 &
sleep 15 && kill %1
grep -ic 'error\|panic\|crash' /tmp/uqm_no_c_gfx.log
# Expected: 0
```

## Structural Verification Checklist
- [ ] All target C files deleted or emptied
- [ ] Build system updated (removed deleted files from source lists)
- [ ] Build succeeds without deleted C files
- [ ] No undefined symbol errors at link time
- [ ] `USE_RUST_GFX` defined unconditionally (or guards removed)
- [ ] All cargo gates pass

## Semantic Verification Checklist (Mandatory)
- [ ] Game starts and reaches main menu
- [ ] No C graphics code compiled into final binary
- [ ] All rendering handled by Rust path
- [ ] No regressions from P24 integration testing
- [ ] Build is simpler (fewer files to compile)
- [ ] Git history preserves C code for reference

## Deferred Implementation Detection (Mandatory)

```bash
# No deferred patterns in any Rust FFI file
for f in rust/src/graphics/*_ffi.rs; do
  echo "=== $f ==="
  grep -n "todo!\|TODO\|FIXME\|HACK\|placeholder" "$f" && echo "FAIL" || echo "CLEAN"
done
```

## Success Criteria
- [ ] Build succeeds with zero C graphics object files
- [ ] Game runs correctly
- [ ] All tests pass
- [ ] C code cleanly removed (not just commented out)
- [ ] Build system updated

## Failure Recovery
- rollback: `git revert` the removal commit
- blocking issues: hidden dependencies on deleted C functions

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P25.md`

Contents:
- phase ID: P25
- timestamp
- files deleted: list of removed C files
- files modified: build system files
- build verification: success
- game verification: runs correctly
- C graphics objects: 0
