# Phase 03a: CommData & LOCDATA FFI Verification

## Phase ID
`PLAN-20260314-COMM.P03a`

## Prerequisites
- Required: Phase 03 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/comm/locdata.rs` exists
- [ ] `rust/src/comm/mod.rs` includes `pub mod locdata`
- [ ] `CommData` has all fields from spec §3.1
- [ ] `AnimationDescData` is `#[repr(C)]`
- [ ] FFI accessors have matching C implementations
- [ ] Unit tests for CommData construction and field access

## Semantic Verification Checklist
- [ ] `read_locdata_from_c` tested with mock data covering all field types
- [ ] AnimationDescData round-trips correctly between C and Rust
- [ ] Null LOCDATA pointer handled gracefully (returns error, does not panic)
- [ ] Empty animation array (NumAnimations=0) handled correctly
- [ ] Maximum animations (NumAnimations=20) handled correctly
- [ ] Callback function pointer fields stored as FFI-safe raw types
- [ ] Number speech pointer stored as `*const c_void` (opaque)

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/locdata.rs rust/src/comm/types.rs
```

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P03a.md`
