# Phase 10a: Dead Code Removal — Verification

## Phase ID
`PLAN-20260314-RESOURCE.P10a`

## Prerequisites
- Phase 10 complete

## Structural Verification Checklist
- [ ] prove-unused/build-dependency analysis for Rust and C-side dead-code targets is recorded
- [ ] `ffi.rs` removed or explicitly retained with documented evidence-based reason
- [ ] `resource_system.rs` removed or explicitly retained with documented evidence-based reason
- [ ] `loader.rs` removed or explicitly retained with documented evidence-based reason
- [ ] `cache.rs` removed or explicitly retained with documented evidence-based reason
- [ ] `index.rs` removed or explicitly retained with documented evidence-based reason
- [ ] `config_api.rs` removed or explicitly retained with documented evidence-based reason
- [ ] `mod.rs` updated — no references to removed modules
- [ ] No dangling imports in crate
- [ ] `rust_resource.h` / `rust_resource.c` disposition recorded with build evidence or deferred explicitly to Phase 11 confirmation

## Semantic Verification Checklist
- [ ] `cargo test --workspace --all-features` — all pass
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean
- [ ] `cargo fmt --all --check` — clean
- [ ] Any removal performed so far is proven safe against current build/linkage evidence

## Remaining Module Inventory
After removal, `rust/src/resource/` should contain only the authoritative or explicitly justified remaining modules:
- [ ] `mod.rs` — module declarations
- [ ] `ffi_bridge.rs` — C ABI replacement (authoritative)
- [ ] `dispatch.rs` — resource dispatch (authoritative)
- [ ] `type_registry.rs` — type handler registry (authoritative)
- [ ] `ffi_types.rs` — C-compatible type definitions (authoritative)
- [ ] `propfile.rs` — property file parser (authoritative)
- [ ] `resource_type.rs` — resource type definitions (authoritative)
- [ ] `stringbank.rs` — string pool helper (if still used)
- [ ] `tests.rs` — test suite

## Success Criteria
- [ ] Dead-code removal verification is evidence-based
- [ ] No premature cleanup claims are made about unresolved C-side wrappers

## Failure Recovery
- rollback steps: `git checkout -- project-plans/20260311/resource/plan/10a-dead-code-removal-verification.md`
- blocking issues to resolve before next phase: unresolved build dependency or missing evidence for a removal decision

## Gate Decision
- [ ] Phase 10 complete and verified
- [ ] Proceed to Phase 11
