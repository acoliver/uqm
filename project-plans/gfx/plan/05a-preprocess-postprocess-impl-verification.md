# Phase 05a: Preprocess Fix + Postprocess Refactor — Implementation Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P05a`

## Prerequisites
- Required: Phase P05 completed
- Expected artifacts: Final preprocess + postprocess implementation

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_preprocess` contains exactly: set_blend_mode, set_draw_color, clear
- [ ] `rust_gfx_postprocess` contains exactly: present
- [ ] All tests pass
- [ ] No clippy warnings

## Semantic Verification Checklist (Mandatory)

```bash
# Verify postprocess is present-only
grep -c "canvas.present" rust/src/graphics/ffi.rs  # Should find it in postprocess
grep -c "texture_creator\|create_texture_streaming\|canvas.copy" rust/src/graphics/ffi.rs  # Should NOT find in postprocess
```

- [ ] Postprocess function body is ~3-5 lines
- [ ] No texture/surface code in postprocess
- [ ] Preprocess matches C reference behavior

## Deferred Implementation Detection (Mandatory)

```bash
# Check only the preprocess and postprocess functions
sed -n '/fn rust_gfx_preprocess/,/^}/p' rust/src/graphics/ffi.rs | grep "TODO\|FIXME\|HACK" || echo "CLEAN"
sed -n '/fn rust_gfx_postprocess/,/^}/p' rust/src/graphics/ffi.rs | grep "TODO\|FIXME\|HACK" || echo "CLEAN"
```

## Success Criteria
- [ ] All checks pass
- [ ] Implementation is final (no stubs remain in these two functions)

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P05a.md`
