# Phase 11: End-to-End Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P11`

## Prerequisites
- Required: Phase P10 completed
- Verify: All integration tests pass
- Verify: Project builds with USE_RUST_GFX enabled
- Expected: All planned gap closures (G1-G16) implemented and unit/integration tested

## Requirements Verified

This phase verifies the full set of planned and in-scope requirement mappings as an integrated whole. It does not implement new functionality. Deferred loader-parity items remain excluded exactly as documented in the overview.

The verification scope includes:

- REQ-RL-001 through REQ-RL-012 (Rendering lifecycle)
- REQ-DQ-001 through REQ-DQ-013 (Draw queue)
- REQ-CAN-001 through REQ-CAN-006 (Canvas)
- REQ-IMG-001 through REQ-IMG-008 (Image)
- REQ-FONT-001 through REQ-FONT-004 (Font)
- REQ-CMAP-001 through REQ-CMAP-005 (Colormap)
- REQ-FADE-001 through REQ-FADE-006 (Fade)
- REQ-SCAL-001 through REQ-SCAL-009 (Scaling/Presentation)
- REQ-TRANS-001 through REQ-TRANS-003 (Transition)
- REQ-ERR-001 through REQ-ERR-007 (Error handling)
- REQ-OWN-001 through REQ-OWN-007 (Ownership/lifecycle)
- REQ-INT-001 through REQ-INT-011 (Integration)
- REQ-INT-005 and REQ-INT-012 remain deferred per documented scope

## Verification Tasks

### Task 1: Full build verification

```bash
# Clean build from scratch
cd rust && cargo clean && cargo build --release
cd ../sc2 && make clean && ./build.sh uqm
```

Verify:
- [ ] Rust library compiles without warnings
- [ ] C project compiles without warnings
- [ ] Final binary links without unresolved symbols

### Task 2: Automated test suite

```bash
# All Rust tests
cargo test --workspace --all-features

# Count test coverage summary
cargo test --workspace --all-features 2>&1 | grep "test result:" | tail -1
```

Verify:
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] No test regressions from baseline

### Task 3: Static analysis

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Verify:
- [ ] No formatting issues
- [ ] No clippy warnings

### Task 4: FFI symbol completeness

```bash
# Extract all declared symbols from rust_gfx.h
grep -o 'rust_[a-z_]*' sc2/src/libs/graphics/sdl/rust_gfx.h | sort -u > /tmp/declared.txt

# Extract all exported symbols from the Rust library
nm -gU rust/target/release/libuqm_rust.a | grep ' T _rust_' | sed 's/.*_rust_/rust_/' | sort -u > /tmp/exported.txt

# Diff — all declared should be exported
diff /tmp/declared.txt /tmp/exported.txt
```

Verify:
- [ ] Every symbol declared in `rust_gfx.h` is exported by the Rust library
- [ ] No missing symbols

### Task 5: C wiring completeness

```bash
# Verify real draw/control entry points have USE_RUST_GFX guards
grep -n "USE_RUST_GFX" sc2/src/libs/graphics/tfb_draw.c sc2/src/libs/graphics/dcqueue.c sc2/src/libs/graphics/cmap.c sc2/src/libs/graphics/sdl/sdl_common.c
```

Verify:
- [ ] All required draw/control functions have Rust redirects
- [ ] Flush has Rust redirect
- [ ] Batch/unbatch/set_screen bridge points are present where required
- [ ] Deferred control-path ingress inventory from P09 is complete and matches actual call sites

### Task 6: Runtime smoke test (manual)

If a display environment is available:

1. Launch the game with USE_RUST_GFX enabled
2. Verify the main menu renders correctly
3. Verify screen transitions work (menu navigation)
4. Verify fade effects work (entering/exiting menus)
5. Verify text rendering works (menu labels)
6. Verify image rendering works (sprites, backgrounds)
7. Verify extra-screen workflows produce correct visible results where exercisable
8. Verify system-box visibility through fades/transitions
9. If scanlines enabled, verify scanline effect visually matches expectations
10. Verify no obvious regressions in input/render synchronization during normal rendering
11. Verify idle/no-redraw flush behavior does not visibly perturb output when no update is required

Document results in completion marker.

### Task 7: Semantic spot checks for requirements the review called out

These checks are mandatory even if unit/integration tests already pass:

- [ ] REQ-DQ-003 batch visibility verified on the migrated path
- [ ] REQ-DQ-004 nested batching verified on the migrated path
- [ ] REQ-OWN-006 deferred free ordering verified on the migrated path
- [ ] REQ-OWN-007 image synchronization obligations verified at the ABI boundary
- [ ] REQ-INT-006 transition capture timing/stability verified end-to-end
- [ ] REQ-INT-007 extra-screen workflow verified end-to-end
- [ ] REQ-INT-008 context-driven draw state propagation verified end-to-end
- [ ] REQ-SCAL-006 scanline output verified semantically (runtime/image-based), not just structurally
- [ ] REQ-RL-009 idle/no-redraw behavior verified explicitly, including no visible-output change
- [ ] REQ-RL-011 reinit failure/reversion behavior verified as far as safely testable
- [ ] REQ-RL-012 system-box compositing ordering verified on the real presentation path

### Task 8: Deferred implementation audit

```bash
# Final check for any remaining placeholders
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ sc2/src/libs/graphics/ --include="*.rs" --include="*.c" --include="*.h" | grep -v "#\[cfg(test)\]" | wc -l
```

Any remaining items must be:
- Documented as intentional
- Outside the scope of this plan
- Filed as follow-up issues if still relevant

## Definition of Done (from 00-overview.md)

- [ ] All unit tests pass (`cargo test --workspace --all-features`)
- [ ] All integration tests pass (`cargo test --test graphics_integration`)
- [ ] Code compiles without warnings (`cargo clippy --workspace --all-targets --all-features -- -D warnings`)
- [ ] Code is formatted (`cargo fmt --all --check`)
- [ ] Project builds with `USE_RUST_GFX` enabled
- [ ] All in-scope requirements from requirements.md are traced to implementations and verification
- [ ] No `.bak` files in `rust/src/graphics/`
- [ ] No deferred implementation patterns in production code

## Success Criteria
- [ ] Full build succeeds (clean build)
- [ ] All tests pass (unit + integration)
- [ ] Static analysis clean (fmt + clippy)
- [ ] All FFI symbols resolve
- [ ] All required C draw/control functions redirect to Rust under USE_RUST_GFX
- [ ] Runtime smoke test passes (if display available)
- [ ] Review-highlighted semantic requirements are explicitly revalidated
- [ ] No outstanding deferred implementation in production code

## Phase Completion Marker
Create: `project-plans/20260311/graphics/.completed/P11.md`

Contents:
- phase ID: PLAN-20260314-GRAPHICS.P11
- timestamp
- full test output
- build output
- symbol verification output
- smoke test results (if available)
- explicit results for the review-highlighted semantic checks
- list of any remaining items deferred to future plans
