# Phase 06a: Mount Ordering & errno — Verification

## Phase ID
`PLAN-20260314-UIO.P06a`

## Prerequisites
- Required: Phase 06 completed

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `sort_mount_registry` function is gone
- [ ] `insert_mount_at_position` function exists and handles all 4 placement modes
- [ ] `MountInfo` has `position: usize` field
- [ ] `set_errno` is called in all non-stream API error paths listed in Phase 06
- [ ] 8+ new tests exist and pass

## Semantic Verification Checklist
- [ ] Mount TOP/BOTTOM/ABOVE/BELOW produce correct resolution order
- [ ] Unmounting does not corrupt remaining mount positions
- [ ] errno values match expected POSIX codes
- [ ] Null handle arguments produce `EINVAL` errno
- [ ] `sdluio.c` integration: `strerror(errno)` will produce meaningful error messages

## Integration Verification
- [ ] Build: `cd sc2 && make` — verify C+Rust build succeeds
- [ ] If build passes: `options.c` startup mounts with TOP/ABOVE flags behave correctly

## Gate Decision
- [ ] PASS: proceed to Phase 07
- [ ] FAIL: fix issues before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P06a.md`
