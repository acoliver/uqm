# Phase 07a: Surface Generation TDD + Stub Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P07a`

## Prerequisites
- Required: Phase 07 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `surface_tests.rs` exists with determinism, algorithm, and fixture tests
- [ ] `surface.rs` has refined signatures with `SurfaceAlgorithm` enum
- [ ] `gentopo.rs` has refined `delta_topography` signature
- [ ] Fixture data present for at least 3 reference worlds

## Semantic Verification Checklist
- [ ] Tests call actual `surface.rs` and `gentopo.rs` public APIs
- [ ] Tests assert on byte-level topo_data content, not just size
- [ ] Tests cover gas giant, rocky, and cratered algorithm selection

## Gate Decision
- [ ] PASS: proceed to Phase 08
- [ ] FAIL: add missing test cases
