# Phase 06a: Mount Ordering & Access Verification

## Phase ID
`PLAN-20260314-FILE-IO.P06a`

## Prerequisites
- Required: Phase 06 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
# Boot game to verify startup mount sequence
```

## Structural Verification
- [ ] `MountInfo` has position-based ordering and read-only state
- [ ] No reliance on heuristic sorting for mount precedence
- [ ] mount registration validates relative-handle requirements
- [ ] `uio_access` has mode-specific branches
- [ ] mutation resolution uses overlay-aware helpers rather than direct host-path operations
- [ ] registry iteration/mutation contract for mount topology is documented
- [ ] new failure paths added in this phase extend errno mapping

## Semantic Verification
- [ ] Mount ordering tests: TOP/BOTTOM/ABOVE/BELOW all produce correct precedence
- [ ] Invalid `relative` combinations are rejected explicitly
- [ ] Access mode tests: F_OK, R_OK, W_OK, X_OK all follow spec §5.4
- [ ] Topmost-visible-object rule is verified for access and write-open
- [ ] Read-only shadowing prevents lower-layer create/write fallthrough
- [ ] Parent visibility rules and `ENOTDIR` shadowing cases are covered
- [ ] Cross-mount rename detection: `EXDEV`
- [ ] Topology mutation vs resolution integrity review is complete for shared mount state
- [ ] Game boots and runs through startup (mounts config, content, packages)

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P06a.md` summarizing:
- ordering/access verification result
- mutation edge-case verification result
- topology-concurrency verification result
