# Phase 11a: Port-Completeness & Cleanup — Verification

## Phase ID
`PLAN-20260314-UIO.P11a`

## Prerequisites
- Required: Phase 11 completed

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `uio_fread` exported directly from Rust (no `rust_uio_fread` alias)
- [ ] `uio_fread_shim.c` removed from build
- [ ] No `DIR_LIST_BUFFER_SIZES` or similar side-channel globals remain
- [ ] `uio_printMounts` produces diagnostic output
- [ ] `uio_init` / `uio_unInit` functional
- [ ] No stub APIs return dummy non-null handles

## Semantic Verification Checklist
- [ ] All 7+ new tests pass
- [ ] All pre-existing tests pass
- [ ] Full project builds: `cd sc2 && make`
- [ ] No linker errors for `uio_fread` symbol

## Final Stub Audit

```bash
# Verify no remaining stubs return fake success
grep -n "dummy\|0xDEAD\|0x1\b\|1 as \*mut" rust/src/io/uio_bridge.rs
```

Expected: no hits for functions that should return null on failure.

## Gate Decision
- [ ] PASS: proceed to Phase 12
- [ ] FAIL: fix issues before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P11a.md`
