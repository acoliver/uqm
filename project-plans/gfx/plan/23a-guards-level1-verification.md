# Phase 23a: Level 1-2 Guards — Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P23a`

## Prerequisites
- Required: Phase P23 (Level 1-2 Guards) completed
- Expected: ~31 C files guarded total (2 pre-existing + 15 Level 0 + 14 Level 1-2)
- Expected: Build succeeds with and without `USE_RUST_GFX`

## Requirements Verified

### REQ-GUARD-010–050: All Non-Widget C File Guards
Verification:
- Enumerate all 41 C files in `sc2/src/libs/graphics/`
- Confirm ~31 have `USE_RUST_GFX` guards
- Confirm ~10 unguarded files are accounted for:
  - 5 widget-dependent (context.c, drawable.c, frame.c, font.c, widgets.c) → deferred to P24
  - 5 loaders (gfxload.c, resgfx.c, filegfx.c, loaddisp.c, png2sdl.c) → deferred indefinitely

### REQ-COMPAT-010: Backward Compatibility
Verification:
- Build without `USE_RUST_GFX`, verify C path still works
- No compilation errors from guard placement

### REQ-COMPAT-020: Link Completeness
Verification:
- Build with `USE_RUST_GFX=1`
- Check for undefined symbol errors at link time
- All `rust_*` symbols resolved by `libuqm_rust.a`

## Verification Tasks

### Task 1: Guard Inventory Audit

```bash
for f in $(find sc2/src/libs/graphics -name '*.c' | sort); do
  if grep -q 'USE_RUST_GFX' "$f"; then
    echo "[GUARDED] $f"
  else
    echo "[UNGUARDED] $f"
  fi
done
```

Expected: ~31 GUARDED, ~10 UNGUARDED (5 widget → P24, 5 loaders)

### Task 2: Build Verification — Rust Path

```bash
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | tee /tmp/build_rust_gfx.log
echo "Build exit code: $?"
grep -c 'undefined reference\|undefined symbol' /tmp/build_rust_gfx.log
# Expected: 0
```

### Task 3: Build Verification — C Path

```bash
cd sc2 && rm -rf obj/release/src/libs/graphics && ./build.sh uqm 2>&1 | tee /tmp/build_c_gfx.log
echo "Build exit code: $?"
```

### Task 4: Rust Test Suite

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

### Task 5: Symbol Verification

```bash
cd rust && cargo build --release
nm -gU target/release/libuqm_rust.a 2>/dev/null | grep -E 'rust_(gfx|dcq|canvas|cmap)_' | sort
# Expected: >= 50 symbols (17 gfx + 15 dcq + 10 canvas + 8 cmap)
```

## Structural Verification Checklist
- [ ] ~31 C files have `USE_RUST_GFX` guards
- [ ] 10 unguarded files accounted for (5 widget → P24, 5 loaders → deferred)
- [ ] Build succeeds with `USE_RUST_GFX=1`
- [ ] Build succeeds without `USE_RUST_GFX`
- [ ] No undefined symbol errors in either build path
- [ ] All Rust tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] Guarded files produce zero object code when `USE_RUST_GFX` is defined
- [ ] No C-side regressions: game runs identically on C path
- [ ] Rust path links and loads without missing symbols

## Success Criteria
- [ ] ~31/41 C files guarded (widget-dependent files deferred to P24, loaders unguarded)
- [ ] Both build paths compile without errors
- [ ] Link succeeds with Rust providing all replaced symbols
- [ ] All cargo gates pass

## Failure Recovery
- rollback: `git stash` (revert all guard additions)
- blocking issues: missing Rust FFI symbols at link time

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P23a.md`

Contents:
- phase ID: P23a
- timestamp
- guard audit: ~31 guarded / ~10 unguarded (5 widget→P24, 5 loaders→deferred)
- build verification: both paths successful
- symbol count: N total Rust FFI exports
- test results: all pass
