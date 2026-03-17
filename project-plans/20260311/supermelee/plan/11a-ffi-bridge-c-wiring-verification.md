# Phase 11a: FFI Bridge & C Wiring Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P11a`

## Prerequisites
- Required: Phase 11 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Run the project's audited SuperMelee C/Rust integration build command as part of this gate once identified in the implementation phase.

## Structural Verification Checklist
- [ ] `c_bridge.rs` and `setup/ffi.rs` are implemented
- [ ] Scoped C-side SuperMelee files are wired to Rust where planned
- [ ] The verification phase checks FFI and C-wiring artifacts rather than unrelated E2E-only content

## Semantic Verification Checklist
- [ ] FFI signatures used as exact contracts were audited against actual headers
- [ ] Battle-facing combatant handoff remains compatible with the consuming boundary
- [ ] C-side redirection remains limited to SuperMelee-owned setup/load/save/pick entry points
- [ ] Netplay-related wiring remains at the documented SuperMelee boundary and does not claim transport ownership

## Gate Decision
- [ ] PASS: proceed to Phase 12
- [ ] FAIL: fix FFI/C wiring gaps

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P11a.md`
