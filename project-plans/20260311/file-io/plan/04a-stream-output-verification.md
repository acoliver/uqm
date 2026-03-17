# Phase 04a: Stream Output & `uio_fread` Verification

## Phase ID
`PLAN-20260314-FILE-IO.P04a`

## Prerequisites
- Required: Phase 04 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification
- [ ] `uio_fread` symbol exists in Rust library (not in C shim)
- [ ] `uio_fread_shim.c` not in Makeinfo Rust-mode source list
- [ ] `uio_vfprintf` body is non-trivial (not a stub)
- [ ] C trampoline file exists and is compiled by `build.rs` (if trampoline approach used)

## Semantic Verification
- [ ] Test: `uio_fread` reads correct data from an open stream
- [ ] Test: `uio_vfprintf` formats "%d" correctly
- [ ] Test: `uio_vfprintf` formats "%s" correctly
- [ ] Test: `uio_fprintf` with mixed format specifiers
- [ ] Full build links without undefined symbol errors
- [ ] Game boots successfully

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P04a.md`
