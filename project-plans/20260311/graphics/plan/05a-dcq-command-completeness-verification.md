# Phase 05a: DCQ Command Completeness Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P05a`

## Prerequisites
- Required: Phase P05 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features -- dcq
```

## Structural Verification Checklist
- [ ] `DrawCommand` enum has `SetPalette` variant
- [ ] 5 new FFI push functions exist: filledimage, fontchar, setmipmap, deletedata, callback
- [ ] `rust_dcq_push_drawimage` signature includes scale, scale_mode, colormap_index, draw_mode
- [ ] `rust_dcq_push_setpalette` pushes `SetPalette` command (not `Callback`)
- [ ] C header `rust_gfx.h` declares all new functions
- [ ] 10+ new tests added

## Semantic Verification Checklist (Mandatory)
- [ ] All 16 spec command types have push functions
- [ ] All push functions: null/uninitialized → -1, valid → 0, correct queue length
- [ ] SetPalette command handler updates render context colormap state
- [ ] FontChar push function handles null glyph data safely
- [ ] All existing DCQ tests pass without modification

## FFI Symbol Cross-Check

Verify every function declared in `rust_gfx.h` has a corresponding Rust export:

```bash
grep 'rust_dcq_push_' sc2/src/libs/graphics/sdl/rust_gfx.h | sort
grep '#\[no_mangle\]' rust/src/graphics/dcq_ffi.rs -A1 | grep 'pub.*extern.*fn rust_dcq' | sort
# Both lists should match
```
