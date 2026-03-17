# Phase 12a: Integration Verification

## Phase ID
`PLAN-20260314-FILE-IO.P12a`

## Prerequisites
- Required: Phase 12 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification
- [ ] Full build links cleanly
- [ ] No unresolved `uio_*` symbols remain in the final binary
- [ ] `uio_fread_shim.c` is not compiled into the active Rust-mode build
- [ ] linkage verification used full relevant output review, not truncated samples

## Semantic Verification
- [ ] startup mount sequence works, including package discovery and archive mounting
- [ ] SDL image loading works from ZIP-backed streams
- [ ] save/load cycle completes with overlay semantics intact
- [ ] StdioAccess resource loading works via correct direct-path/temp-copy decisions
- [ ] cleanup after mount removal/repository topology change is safe
- [ ] no regressions in existing tests

## End-to-End Gate
- [ ] Game is fully playable through at least the intro sequence
- [ ] Clean shutdown with no error output

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P12a.md` summarizing:
- linkage verification result
- end-to-end semantic verification result
