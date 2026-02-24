# Phase 22a: Level 0 Guards — Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P22a`

## Prerequisites
- Required: Phase P22 (Level 0 Guards) completed
- Expected: ~15 Level 0 C files newly guarded (total ~17 including 2 pre-existing)
- Expected: Build succeeds with and without `USE_RUST_GFX`

## Requirements Verified

### REQ-GUARD-030: Scaler Guards
Verification:
- All 10 scaler files have `USE_RUST_GFX` guards
- Scaler symbols resolve from Rust when `USE_RUST_GFX=1`

### REQ-GUARD-010 (partial): Primitives + Geometry Guards
Verification:
- All 5 primitives/geometry files have `USE_RUST_GFX` guards
- Symbols resolve correctly in both build modes

### REQ-COMPAT-010: Backward Compatibility
Verification:
- Build without `USE_RUST_GFX`, run game, verify identical behavior
- No C compilation errors from guard placement

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

Expected: ~17 GUARDED, ~24 UNGUARDED (14 Level 1-2 → P23, 5 widget → P24, 5 loaders)

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

## Structural Verification Checklist
- [ ] 17 C files have `USE_RUST_GFX` guards (2 pre-existing + 15 Level 0)
- [ ] Build succeeds with `USE_RUST_GFX=1`
- [ ] Build succeeds without `USE_RUST_GFX`
- [ ] No undefined symbol errors in either build path
- [ ] All Rust tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] Guarded files produce zero object code when `USE_RUST_GFX` is defined
- [ ] No C-side regressions: game runs identically on C path
- [ ] Rust path links without missing symbols

## Success Criteria
- [ ] ~17/41 C files guarded (Level 0 complete)
- [ ] Both build paths compile without errors
- [ ] All cargo gates pass

## Failure Recovery
- rollback: `git stash`
- blocking issues: missing Rust FFI symbols at link time

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P22a.md`

Contents:
- phase ID: P22a
- timestamp
- guard audit: 17 guarded / 24 unguarded
- build verification: both paths successful
