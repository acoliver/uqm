# Phase 08a: Surface Generation & Rendering Implementation Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P08a`

## Prerequisites
- Required: Phase 08 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets::tests::surface_tests --all-features -- --nocapture
```

## Structural Verification Checklist
- [ ] `surface.rs`, `gentopo.rs`, `render.rs` — no `todo!()` or `unimplemented!()`
- [ ] All surface_tests pass
- [ ] Rendering functions compile and link against graphics subsystem APIs

## Semantic Verification Checklist
- [ ] Byte-level topo determinism verified for at least 3 reference worlds
- [ ] Algorithm selection covers all world types (gas giant, topo, cratered)
- [ ] Sphere rotation produces frame sequence suitable for animation
- [ ] Orbit drawing produces correct elliptical shapes
- [ ] Coordinate transforms match C `XFormIPLoc` behavior

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/surface.rs rust/src/planets/gentopo.rs rust/src/planets/render.rs
# Must return 0
```

## Gate Decision
- [ ] PASS: proceed to Phase 09
- [ ] FAIL: fix surface/rendering implementation
