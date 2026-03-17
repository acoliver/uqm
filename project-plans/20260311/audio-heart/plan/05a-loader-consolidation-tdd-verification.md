# Phase 05a: Loader Consolidation TDD — Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P05a`

## Prerequisites
- Required: Phase P05 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features 2>&1 | grep -E 'test.*load|test.*bank|test.*decoder'
```

## Structural Verification Checklist
- [ ] At least 6 test functions in `loading.rs`
- [ ] Tests cover validation (empty filename, extension parsing)
- [ ] Tests cover decoder factory (known/unknown extensions)
- [ ] Tests cover error type specificity

## Semantic Verification Checklist
- [ ] Tests fail when behavior is broken (not just asserting mock internals)
- [ ] Error cases return specific AudioError variants, not generic errors
- [ ] Tests are independent (no shared mutable state leaking between tests)

## Success Criteria
- [ ] All validation-level tests pass
- [ ] Test infrastructure is ready for implementation phase

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P05a.md`
