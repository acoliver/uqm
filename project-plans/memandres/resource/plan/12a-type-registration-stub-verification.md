# Phase 12a: Type Registration — Stub Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P12a`

## Prerequisites
- Required: Phase 12 completed

## Verification Checklist
- [ ] `ffi_types.rs` compiles with `#[repr(C)]` types
- [ ] `type_registry.rs` stubs compile
- [ ] Function pointer type signatures match C reslib.h
- [ ] Existing tests pass

## Gate Decision
- [ ] PASS: proceed to P13
- [ ] FAIL: fix stubs
