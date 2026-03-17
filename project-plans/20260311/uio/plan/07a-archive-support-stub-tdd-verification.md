# Phase 07a: Archive Support — Stub/TDD Verification

## Phase ID
`PLAN-20260314-UIO.P07a`

## Prerequisites
- Required: Phase 07 completed

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/io/uio/mod.rs` exists and exports `archive`
- [ ] `rust/src/io/uio/archive.rs` has all structs: `ArchiveEntryInfo`, `MountedArchive`, `ArchiveFileHandle`
- [ ] `ARCHIVE_REGISTRY` static exists
- [ ] All 9 public functions have signatures
- [ ] `zip` crate compiles as dependency
- [ ] 15+ tests exist
- [ ] `create_test_zip` helper exists

## Semantic Verification Checklist
- [ ] Code compiles cleanly with `cargo check`
- [ ] Tests that exercise stubs either: (a) pass because stubs return expected error values, or (b) are marked `#[ignore]` until Phase 08 implementation
- [ ] Test names describe behavior being tested
- [ ] No test depends on another test's side effects

## Gate Decision
- [ ] PASS: proceed to Phase 08
- [ ] FAIL: fix compilation or structure issues

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P07a.md`
