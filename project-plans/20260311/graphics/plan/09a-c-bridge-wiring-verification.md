# Phase 09a: C-Side Bridge Wiring Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P09a`

## Prerequisites
- Required: Phase P09 completed

## Verification Commands

```bash
# Rust side
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C build
cd sc2 && ./build.sh uqm

# Symbol verification
nm -gU rust/target/release/libuqm_rust.a | grep "rust_dcq_push"
nm -gU rust/target/release/libuqm_rust.a | grep "rust_dcq_"
nm -gU rust/target/release/libuqm_rust.a | grep "rust_canvas_"
nm -gU rust/target/release/libuqm_rust.a | grep "rust_cmap_"
```

## Structural Verification Checklist
- [ ] All TFB_DrawScreen_* functions have USE_RUST_GFX guards
- [ ] TFB_FlushGraphics redirects to rust_dcq_flush
- [ ] Batch/unbatch/set_screen redirected
- [ ] Colormap lifecycle wired in init/uninit
- [ ] Deferred control-path ingress inventory exists and names exact file/function owners
- [ ] Full `rust_gfx.h` ↔ Rust export audit completed, not only DCQ push symbol counting
- [ ] Project builds and links successfully

## Semantic Verification Checklist (Mandatory)
- [ ] C calls reach Rust functions on the real migrated path, not merely by symbol presence
- [ ] Color packing matches unpacking
- [ ] Screen index mapping correct
- [ ] Batch and nested batch semantics hold through the actual C entry points
- [ ] Transition capture timing is revalidated through the actual bridge path
- [ ] Extra-screen workflow behavior is revalidated through the actual bridge path
- [ ] Context-driven draw state propagation is revalidated through the actual bridge path
- [ ] Flush completion synchronization remains compatible
- [ ] Idle/no-redraw behavior is revalidated on the migrated flush path
- [ ] ReinitVideo and system-box behaviors are revalidated through the actual orchestration path
- [ ] Image metadata synchronization obligations are revalidated at the ABI boundary
- [ ] No double-init or double-uninit of DCQ/colormap

## C-side Wiring Coverage

```bash
# Verify all TFB_DrawScreen functions have USE_RUST_GFX guards
grep -c "USE_RUST_GFX" sc2/src/libs/graphics/tfb_draw.c
# Expected: at least 11 (one per function)

grep -c "USE_RUST_GFX" sc2/src/libs/graphics/dcqueue.c
# Expected: at least 1 (flush redirect)
```
