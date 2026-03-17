# Phase 03.5a: FFI Boundary & Ownership Contract Verification

## Phase ID
`PLAN-20260314-SHIPS.P03.5a`

## Prerequisites
- Required: Phase 03.5 (FFI Boundary & Ownership Contract) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `ffi_contract.rs` exists and is exported from `ships/mod.rs`
- [ ] All catalog, queue, loader, runtime, lifecycle, and writeback FFI entrypoints have exact typed signatures documented
- [ ] Ownership/lifetime notes exist for every returned pointer/handle
- [ ] Shared-layout vs opaque-handle choice is explicit per ABI-facing type

## Semantic Verification Checklist
- [ ] Canonical ownership remains aligned with the spec boundary: external systems still control queue creation/enqueue/selection timing
- [ ] No primary FFI signature uses ambiguous `usize` / `*const c_void` in place of a concrete ABI type
- [ ] Catalog pointer validity window is defined and safe
- [ ] Spawn/writeback contracts are expressed in terms of C-owned queue/runtime structures
- [ ] SIS/module-state prerequisite accessors are identified before race implementation phases
- [ ] Mixed-path smoke-test contract exists for early integration verification

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: return to Phase 03.5 and fix contract gaps
