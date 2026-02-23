# Phase 03a: Preprocess Fix + Postprocess Refactor — Stub Verification

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P03a`

## Prerequisites
- Required: Phase P03 completed
- Expected artifacts: Modified `rust/src/graphics/ffi.rs`

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_preprocess` contains `set_blend_mode(BlendMode::None)`
- [ ] `rust_gfx_postprocess` body is ~5 lines (guard + present)
- [ ] No `texture_creator` in postprocess
- [ ] No `create_texture_streaming` in postprocess
- [ ] No `canvas.copy` in postprocess
- [ ] No `texture.update` in postprocess
- [ ] No pixel format conversion code in postprocess
- [ ] No scaling code in postprocess

## Semantic Verification Checklist (Mandatory)

```bash
# Verify postprocess has no texture/upload/copy operations
grep -n "texture_creator\|create_texture_streaming\|canvas.copy\|texture.update\|from_raw_parts\|scaled_buffers\|scale_rgba\|hq2x" rust/src/graphics/ffi.rs | grep -A0 "postprocess" || echo "CLEAN: no upload/render in postprocess"

# Verify preprocess has BlendMode::None
grep -A5 "fn rust_gfx_preprocess" rust/src/graphics/ffi.rs | grep "BlendMode::None" && echo "PASS" || echo "FAIL"
```

- [ ] Postprocess contains zero texture/surface operations
- [ ] Preprocess sets blend mode to None
- [ ] All cargo gates pass

## Success Criteria
- [ ] All structural checks pass
- [ ] All semantic checks pass
- [ ] All verification commands pass

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P03a.md`
