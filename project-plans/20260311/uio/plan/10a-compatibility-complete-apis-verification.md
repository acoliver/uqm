# Phase 10a: Compatibility-Complete APIs — Verification

## Phase ID
`PLAN-20260314-UIO.P10a`

## Prerequisites
- Required: Phase 10 completed

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `fileblock.rs` has no stubs
- [ ] `stdio_access.rs` has no stubs
- [ ] All FileBlock/StdioAccess FFI exports in `uio_bridge.rs` delegate to modules
- [ ] `uio_transplantDir` uses positional insertion
- [ ] `uio_access` checks R_OK/W_OK/X_OK properly
- [ ] 17+ new tests exist and pass

## Semantic Verification Checklist
- [ ] FileBlock operations work on both STDIO and archive file handles
- [ ] StdioAccess creates temp copies for archive content
- [ ] StdioAccess cleans up temp resources on release
- [ ] Transplant with ABOVE/BELOW placement works correctly
- [ ] Access checks reflect mount read-only status
- [ ] All pre-existing tests pass

## Integration Verification
- [ ] Build: `cd sc2 && make`
- [ ] Verify `uio_transplantDir` call in `options.c:575-589` succeeds
- [ ] Verify addon shadow-content overlays work

## Gate Decision
- [ ] PASS: proceed to Phase 11
- [ ] FAIL: fix issues before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P10a.md`
