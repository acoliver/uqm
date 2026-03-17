# Phase 08a: Archive Support — Implementation Verification

## Phase ID
`PLAN-20260314-UIO.P08a`

## Prerequisites
- Required: Phase 08 completed

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `archive.rs` has no `todo!()` or `unimplemented!()` macros
- [ ] `HandleKind` enum exists with `Stdio` and `Archive` variants
- [ ] All `uio_read`/`uio_write`/`uio_lseek`/`uio_fstat`/`uio_close` dispatch on HandleKind
- [ ] `uio_mountDir` no longer sets `active_in_registry = false` for ZIP
- [ ] `uio_unmountDir` calls `unmount_archive` for ZIP mounts

## Semantic Verification Checklist
- [ ] All 15+ archive module tests pass
- [ ] All 12+ archive integration tests pass
- [ ] All pre-existing tests still pass (STDIO path not broken)
- [ ] End-to-end acceptance test passes:
  - Mount test ZIP → enumerate → open → read → seek → tell → fstat → feof → ferror
- [ ] Write operations on archive content correctly return -1 with EROFS errno
- [ ] Mixed mounts (STDIO + ZIP) resolve correctly by precedence

## Integration Verification
- [ ] Build: `cd sc2 && make` — C+Rust build succeeds
- [ ] Verify `options.c` ZIP mount path now creates active mounts
- [ ] If game can be launched: verify content loads from `.uqm` packages

## Risk Assessment
- [ ] Confirm `zip` crate handles all archive formats used by UQM (.uqm is standard ZIP)
- [ ] Confirm decompressed content matches what C ZIP implementation produces
- [ ] Confirm archive file handles are properly cleaned up on unmount

## Gate Decision
- [ ] PASS: proceed to Phase 09
- [ ] FAIL: fix archive implementation issues before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P08a.md`
