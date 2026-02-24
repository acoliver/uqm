# Phase 08a: Screen Compositing — Implementation Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P08a`

## Prerequisites
- Required: Phase P08 completed
- Expected artifacts: Full ScreenLayer unscaled implementation

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_screen` has no `todo!()` or `unimplemented!()`
- [ ] All unsafe blocks have `// SAFETY:` comments
- [ ] `texture_creator` is local (not stored in state)
- [ ] `texture` is local (dropped at end of scope)
- [ ] `canvas.copy` uses `(src_rect, dst_rect)` where src == dst for unscaled
- [ ] Blend mode set before canvas.copy
- [ ] Alpha mod set when alpha < 255

## Semantic Verification Checklist (Mandatory)

```bash
# Verify no deferred patterns in screen function
sed -n '/fn rust_gfx_screen/,/^pub extern "C" fn\|^#\[no_mangle\]/p' rust/src/graphics/ffi.rs | grep -c "TODO\|FIXME\|todo!\|unimplemented!" | xargs test 0 -eq && echo "CLEAN" || echo "FAIL"

# Verify SAFETY comments present
grep -c "// SAFETY:" rust/src/graphics/ffi.rs  # Should be > 0
```

- [ ] No deferred implementation patterns in rust_gfx_screen
- [ ] SAFETY comments present on unsafe blocks
- [ ] All tests pass
- [ ] Behavior: unscaled ScreenLayer composites surfaces correctly

## Success Criteria
- [ ] All structural checks pass
- [ ] All semantic checks pass
- [ ] All cargo gates pass
- [ ] Game displays visible content (manual runtime verification recommended)

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P08a.md`
