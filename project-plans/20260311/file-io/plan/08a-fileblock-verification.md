# Phase 08a: FileBlock Verification

## Phase ID
`PLAN-20260314-FILE-IO.P08a`

## Prerequisites
- Required: Phase 08 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification
- [ ] `uio_FileBlock` struct has real fields (handle, range, cache)
- [ ] `uio_openFileBlock2` uses the correct `(handle, offset, size)` ABI
- [ ] `uio_accessFileBlock` uses the correct `char **buffer` out-parameter ABI and `ssize_t` return
- [ ] `uio_clearFileBlockBuffers` is implemented
- [ ] All public FileBlock functions are non-stub implementations
- [ ] FileBlock-specific failure paths extend errno mapping

## Semantic Verification
- [ ] Test: sequential access reads return correct bytes and counts
- [ ] Test: overlapping access reads refresh/cache correctly
- [ ] Test: ranged block boundaries are enforced
- [ ] Test: `uio_clearFileBlockBuffers` resets cache state safely
- [ ] Test: copy fills caller buffer correctly
- [ ] Test: EOF handling returns short counts without uninitialized memory
- [ ] Test: invalid arguments fail safely with expected errno behavior
- [ ] Test: close + open cycle doesn't leak

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P08a.md` summarizing:
- ABI signature verification
- FileBlock semantic test results
- FileBlock errno verification result
