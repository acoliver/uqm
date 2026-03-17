# Phase 07a: Queue & Build Primitives Verification

## Phase ID
`PLAN-20260314-SHIPS.P07a`

## Prerequisites
- Required: Phase 07 (Queue & Build) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `queue.rs` exports all queue types and helper functions
- [ ] ShipQueue and FragmentQueue are fully implemented
- [ ] Global queues are properly guarded

## Semantic Verification Checklist
- [ ] Build creates zero-initialized entries with correct species
- [ ] Index lookup matches insertion order
- [ ] Fragment cloning preserves all metadata
- [ ] Escort helpers enforce limits
- [ ] Queue clear releases all entries

## Gate Decision
- [ ] PASS: proceed to Phase 08
- [ ] FAIL: return to Phase 07 and fix issues
