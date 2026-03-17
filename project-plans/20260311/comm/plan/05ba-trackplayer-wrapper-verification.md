# Phase 05ba: Trackplayer Wrapper Verification

## Phase ID
`PLAN-20260314-COMM.P05ba`

## Prerequisites
- Required: Phase 05b completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd /Users/acoliver/projects/uqm/sc2 && make 2>&1 | tail -20
```

## Structural Verification Checklist
- [ ] `sc2/src/uqm/rust_comm.c` contains the full P06 wrapper surface
- [ ] `sc2/src/uqm/rust_comm.h` declares the same full wrapper surface
- [ ] No required P06 trackplayer wrapper remains first-owned by P11

## Semantic Verification Checklist
- [ ] Every P06 extern declaration has a corresponding C wrapper available before P06 starts
- [ ] Wrapper names/signatures are consistent across `rust_comm.h`, `rust_comm.c`, and the planned Rust extern block
- [ ] Full project build resolves the wrapper symbols successfully

## Gate Decision
- [ ] PASS: proceed to Phase 06
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P05ba.md`
