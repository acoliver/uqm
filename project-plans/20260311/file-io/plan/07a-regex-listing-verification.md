# Phase 07a: Regex & Cross-Mount Listing Verification

## Phase ID
`PLAN-20260314-FILE-IO.P07a`

## Prerequisites
- Required: Phase 07 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification
- [ ] regex compatibility layer is imported and used
- [ ] no hardcoded archive/index regex special cases remain
- [ ] `uio_getDirList` enumerates across multiple mounts
- [ ] dedup/output logic preserves deterministic precedence-sensitive ordering
- [ ] `uio_DirList` first-two-field ABI contract is preserved in the implementation changed here
- [ ] returned list ownership is self-contained until `uio_DirList_free`
- [ ] listing topology snapshot/locking rule exists for shared state
- [ ] AutoMount logic exists only if required by P00a

## Semantic Verification
- [ ] regex matching behavior matches the audited compatibility decision
- [ ] invalid regex is handled safely
- [ ] cross-mount listing produces correct union
- [ ] deduplication works correctly
- [ ] empty results produce non-null `uio_DirList` with `numNames == 0`
- [ ] `uio_DirList_free` remains correct for list results produced here
- [ ] returned directory-list lifetime remains safe until free even if topology changes after listing
- [ ] AutoMount behavior is validated if branch is active
- [ ] Game startup archive/index discovery still works

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P07a.md` summarizing:
- regex compatibility verification result
- listing ordering/dedup verification result
- `uio_DirList` ABI/ownership verification result
- AutoMount verification result if applicable
