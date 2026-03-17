# Phase 11a: Lifecycle & Cleanup Verification

## Phase ID
`PLAN-20260314-FILE-IO.P11a`

## Prerequisites
- Required: Phase 11 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification
- [ ] `uio_init` is not a log-only stub
- [ ] `uio_unInit` clears registries/state
- [ ] repository close unmounts and frees associated state
- [ ] cleanup paths remain independent of mount-active status
- [ ] `uio_fclose` frees buffer
- [ ] `uio_DirList_free` works without fragile side-channel dependency
- [ ] concrete concurrency race review note exists
- [ ] lifecycle/shutdown failure paths extend errno mapping where applicable

## Semantic Verification
- [ ] init/uninit/reinit cycle works
- [ ] cleanup after mount removal is safe for close/fclose/closeDir/releaseStdioAccess
- [ ] no leaks in stream lifecycle
- [ ] no leaks in DirList lifecycle
- [ ] no leaks in stdio temp-resource lifecycle
- [ ] concrete race classes are verified for mount registry iteration/mutation, repository close vs open handles, and returned allocation lifetimes
- [ ] Game starts and shuts down cleanly

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P11a.md` summarizing:
- lifecycle verification result
- post-unmount cleanup verification result
- concrete concurrency verification result
