# Phase 06a: Screen Compositing — Stub Verification

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P06a`

## Prerequisites
- Required: Phase P06 completed
- Expected artifacts: ScreenLayer stub and convert_c_rect helper

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_screen` function exists with correct signature
- [ ] Guard: `get_gfx_state()` None check present
- [ ] Guard: `screen < 0 || screen >= TFB_GFX_NUMSCREENS` check present
- [ ] Guard: `screen == 1` early return present
- [ ] Guard: `src_surface.is_null()` check present
- [ ] `convert_c_rect` function exists and compiles

## Semantic Verification Checklist (Mandatory)
- [ ] Guards return void (no crash, no rendering)
- [ ] `todo!()` in body only (will be replaced in P08)
- [ ] No fake rendering behavior
- [ ] Existing tests still pass

## Success Criteria
- [ ] All cargo gates pass
- [ ] Stub structure matches pseudocode component-003 lines 1–13

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P06a.md`
