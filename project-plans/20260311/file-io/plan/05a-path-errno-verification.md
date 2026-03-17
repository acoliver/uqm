# Phase 05a: Path Normalization & errno Verification

## Phase ID
`PLAN-20260314-FILE-IO.P05a`

## Prerequisites
- Required: Phase 05 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification
- [ ] Virtual path normalization has `.`/`..`/slash normalization logic
- [ ] Empty-path handling uses the directory handle location, not unconditional root
- [ ] Host-path mapping has explicit mount-root confinement logic
- [ ] `set_errno` helper exists with platform-specific implementation
- [ ] panic-containment helper exists for exported `extern "C"` entry points
- [ ] Error paths in public entry points call `set_errno` and unwind partial allocations/resources

## Semantic Verification
- [ ] Path normalization unit tests cover all required edge cases
- [ ] Host confinement tests prove host paths do not escape above mount physical roots through logical normalization
- [ ] errno value tests verify correct codes after failures
- [ ] validation tests cover invalid mode strings and invalid flag combinations
- [ ] `uio_getFileLocation` edge-case failures (archive, merged dir, synthetic dir, missing file) are tested
- [ ] Verification note or test evidence confirms no Rust panic escapes current exported FFI entry points
- [ ] Game startup still works (path resolution changes do not break existing mounts)

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P05a.md` summarizing:
- normalization/confinement pass results
- errno/validation pass results
- panic-containment verification result
