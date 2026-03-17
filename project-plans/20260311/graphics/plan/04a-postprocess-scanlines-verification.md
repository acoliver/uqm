# Phase 04a: Postprocess + Scanlines Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P04a`

## Prerequisites
- Required: Phase P04 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features -- ffi
```

## Structural Verification Checklist
- [ ] `rust_gfx_postprocess` body is < 30 lines (was ~120)
- [ ] No `create_texture`, `update_texture`, or `copy` calls in postprocess
- [ ] `apply_scanlines` helper function exists
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] Postprocess function flow: check state → scanlines (if flag) → present
- [ ] Scanline applies to alternating rows with semi-transparent black
- [ ] All existing ffi.rs tests pass unchanged
- [ ] No regression in screen compositing behavior

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ffi.rs | head -20
```
