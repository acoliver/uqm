# Phase 22: Colormap + C Guards — Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P22`

## Prerequisites
- Required: Phase P21a completed
- Expected: ~29 drawing-pipeline C files newly guarded in P21 (total ~31 including 2 pre-existing)
- Expected: Colormap FFI exports compiled and linked
- Expected: Build succeeds with and without `USE_RUST_GFX`

## Requirements Verified

### REQ-GUARD-010–050: All C File Guards
Verification:
- Enumerate all 41 C files in `sc2/src/libs/graphics/`
- Confirm 31 have `USE_RUST_GFX` guards (2 pre-existing + 29 from P21)
- Confirm 10 unguarded files are accounted for:
  - 5 widget-dependent (context.c, drawable.c, frame.c, font.c, widgets.c) → deferred to P23
  - 5 loaders (gfxload.c, resgfx.c, filegfx.c, loaddisp.c, png2sdl.c) → deferred indefinitely

### REQ-CMAP-010–030: Colormap FFI Correctness
Verification:
- Run colormap unit tests in `cmap_ffi.rs`
- Verify fade step produces correct fade_amount values
- Verify colormap set/get round-trip

### REQ-COMPAT-010: Backward Compatibility
**Requirement text**: When `USE_RUST_GFX` is not defined, the C graphics
path shall compile and function identically to the pre-port state.

Verification:
- Build without `USE_RUST_GFX`, run game, verify identical behavior
- No C compilation errors from guard placement

### REQ-COMPAT-020: Link Completeness
**Requirement text**: When `USE_RUST_GFX` is defined, all symbols
referenced by the remaining (unguarded) C code shall be provided by the
Rust FFI exports.

Verification:
- Build with `USE_RUST_GFX=1`
- Check for undefined symbol errors at link time
- All `rust_*` symbols resolved by `libuqm_rust.a`

## Verification Tasks

### Task 1: Guard Inventory Audit

```bash
# List all C files and their guard status
for f in $(find sc2/src/libs/graphics -name '*.c' | sort); do
  if grep -q 'USE_RUST_GFX' "$f"; then
    echo "[GUARDED] $f"
  else
    echo "[UNGUARDED] $f"
  fi
done
```

Expected output: ~31 GUARDED, ~10 UNGUARDED (5 loaders + sdl_common.c + 4 widget-dependent files deferred to P23)

### Task 2: Build Verification — Rust Path

```bash
# Full build with Rust GFX (USE_RUST_GFX=1 in build.vars)
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | tee /tmp/build_rust_gfx.log

# Verify no undefined symbols (check exit code first, then grep)
echo "Build exit code: $?"
grep -c 'undefined reference\|undefined symbol' /tmp/build_rust_gfx.log
# Expected: 0

# Note: Guarded files may still appear in build logs as empty TUs.
# The authoritative check is that no C drawing-pipeline object code is linked.
# Verify via nm on the final binary (see Task 5).
```

### Task 3: Build Verification — C Path

```bash
# Full build without Rust GFX (set USE_RUST_GFX='0' in build.vars first)
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | tee /tmp/build_c_gfx.log

# Verify build succeeded (exit code is authoritative, not grep)
echo "Build exit code: $?"
```

### Task 4: Rust Test Suite

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Colormap-specific tests
cd rust && cargo test --lib -- cmap_ffi::tests --nocapture
```

### Task 5: Symbol Verification

```bash
# Verify all expected Rust symbols are exported
cd rust && cargo build --release
nm -gU target/release/libuqm_rust.a 2>/dev/null | grep -E 'rust_(gfx|dcq|canvas|cmap)_' | sort
# Expected: >= 50 symbols (17 gfx + 15 dcq + 10 canvas + 8 cmap)
```

### Task 6: Functional Smoke Test

```bash
# Run game with USE_RUST_GFX, verify startup sequence
# (requires display server — may need to be run manually)
./uqm --logfile /tmp/uqm_rust_gfx.log 2>&1
# Check log for: "Rust GFX init" message, no crash within 10 seconds
```

## Structural Verification Checklist
- [ ] 31 C files have `USE_RUST_GFX` guards (2 pre-existing + 29 new in P21)
- [ ] 10 unguarded files accounted for (5 widget-dependent → P23, 5 loaders → deferred)
- [ ] Build succeeds with `USE_RUST_GFX=1`
- [ ] Build succeeds without `USE_RUST_GFX`
- [ ] No undefined symbol errors in either build path
- [ ] All Rust tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] Guarded files produce zero object code when `USE_RUST_GFX` is defined
- [ ] Colormap fade_amount values in [0, 511] range
- [ ] Colormap set/get round-trip preserves data
- [ ] No C-side regressions: game runs identically on C path
- [ ] Rust path links and loads without missing symbols

## Success Criteria
- [ ] ~31/41 C files guarded (widget-dependent files deferred to P23, loaders unguarded)
- [ ] Both build paths compile without errors
- [ ] Link succeeds with Rust providing all replaced symbols
- [ ] Colormap FFI tests pass
- [ ] All cargo gates pass

## Failure Recovery
- rollback: `git stash` (revert all guard additions)
- blocking issues: missing Rust FFI symbols at link time — identify which C symbols lack Rust replacements and implement them (do NOT add empty stubs)

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P22.md`

Contents:
- phase ID: P22
- timestamp
- guard audit: 31 guarded (2 pre-existing + 29 new) / 10 unguarded (5 widget→P23, 5 loaders→deferred)
- build verification: both paths successful
- symbol count: N total Rust FFI exports
- test results: all pass
- functional: smoke test result (if available)
