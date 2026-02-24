# Phase 09a: Color Layer — Stub Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P09a`

## Prerequisites
- Required: Phase P09 completed
- Expected artifacts: ColorLayer stub with guards

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_color` has uninitialized guard
- [ ] `rust_gfx_color` has negative rect guard
- [ ] Function compiles
- [ ] All existing tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] Guards return void for specified conditions
- [ ] `todo!()` present in body (stub phase)
- [ ] No fake rendering behavior

## Success Criteria
- [ ] All cargo gates pass
- [ ] Stub matches pseudocode structure

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P09a.md`
