# Phase 06a: DCQ Flush Parity Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P06a`

## Prerequisites
- Required: Phase P06 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features -- dcq
cargo test --workspace --all-features -- bbox
```

## Structural Verification Checklist
- [ ] `BoundingBox` struct with expand/reset methods
- [ ] Livelock detection has actual blocking mechanism (not just logging)
- [ ] Flush completion signaling exists (condvar or equivalent)
- [ ] Empty-queue path checks for active fade/transition
- [ ] 9+ new tests added

## Semantic Verification Checklist (Mandatory)
- [ ] Bbox accumulates correct union of affected regions
- [ ] Bbox only tracks Main screen commands
- [ ] Bbox resets after each flush cycle
- [ ] Livelock blocks producer pushes when threshold exceeded
- [ ] All existing tests pass unchanged
