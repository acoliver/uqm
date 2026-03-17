# Phase 10a: Crew Writeback & Ship Death Verification

## Phase ID
`PLAN-20260314-SHIPS.P10a`

## Prerequisites
- Required: Phase 10 (Crew Writeback & Death) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `writeback.rs` exports death/transition/writeback functions
- [ ] lifecycle.rs integrates with writeback

## Semantic Verification Checklist
- [ ] Death spawns explosions and crew scatter
- [ ] Transition frees descriptor, writes back, marks inactive
- [ ] Transition stops audio before replacement
- [ ] Fragment matching by queue order + species
- [ ] Double-update prevented
- [ ] Surviving ship crew written back correctly
- [ ] Floating crew accounted for
- [ ] Robustness: no panic on edge cases (no desc, absent hook, etc.)

## Gate Decision
- [ ] PASS: proceed to Phase 11
- [ ] FAIL: return to Phase 10 and fix issues
