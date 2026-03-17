# Phase 09a: Final Integration Verification — Sign-off

## Phase ID
`PLAN-20260314-THREADING.P09a`

## Prerequisites
- Required: Phase 09 completed and all checks pass
- Expected previous artifact: `project-plans/20260311/threading/.completed/P09.md`

## Final Gate

### All phases completed
- [ ] P00a — Preflight verification
- [ ] P01 — Analysis
- [ ] P01a — Analysis verification
- [ ] P02 — Pseudocode
- [ ] P02a — Pseudocode verification
- [ ] P03 — Return value stub
- [ ] P03a — Return value stub verification
- [ ] P04 — Return value TDD
- [ ] P04a — Return value TDD verification
- [ ] P05 — Return value implementation
- [ ] P05a — Return value implementation verification
- [ ] P06 — SleepThreadUntil async pump
- [ ] P06a — SleepThreadUntil verification
- [ ] P07 — Detached thread docs
- [ ] P07a — Detached thread docs verification
- [ ] P08 — TODO cleanup
- [ ] P08a — TODO cleanup verification
- [ ] P09 — Final integration
- [ ] P09a — This sign-off

### Gap status accounting
- [ ] G1 (Critical): Thread return value propagation — IMPLEMENTED
- [ ] G2 (Critical): SleepThreadUntil async pumping — IMPLEMENTED
- [ ] G3 (High): StartThread_Core routing documented — DOCUMENTED / CLARIFIED
- [ ] G4 (High): Detached helper cleanup/documentation corrected — OPEN BLOCKER; detached-thread creation failure ABI mismatch remains unresolved
- [ ] G5 (Medium): Scoped stale TODOs removed — IMPLEMENTED
- [ ] G6 (Low): Lifecycle stub documented — IMPLEMENTED
- [ ] G7 (Low): Stale comment corrected — IMPLEMENTED

### Requirement-status summary
- [ ] Requirements implemented by this plan are listed separately from already-satisfied items and still-open blockers
- [ ] Detached-thread creation failure semantics are explicitly reported as not yet at full spec parity
- [ ] Plain-mutex recursion audit blocker remains visible in sign-off

### Final tests
```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --workspace --all-features 2>&1 | grep "test result"
```
- [ ] All tests pass
- [ ] Test count matches baseline recorded in P00a plus plan-added tests
- [ ] Zero failures, zero ignored (unless pre-existing)

### Final lint
```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings
```
- [ ] Clean

### Final C build
```bash
cd /Users/acoliver/projects/uqm/sc2
make -f Makefile.build
```
- [ ] Clean build

### Files changed (complete list)
- [ ] `rust/src/threading/mod.rs` — return type change, join signature, detached docs, TODO cleanup
- [ ] `rust/src/threading/tests.rs` — Rust-generic return value tests and any adapter-level test coverage added by P05
- [ ] `sc2/src/libs/threads/rust_thrcommon.c` — WaitThread, SleepThreadUntil, StartThread_Core comment, recursive mutex comment
- [ ] `sc2/src/libs/threads/rust_threads.h` — rust_thread_join declaration
- [ ] No other files modified

### Open items acknowledged (not plan failures, but still open)
- [ ] Spec §3.1 plain mutex recursion audit — deferred blocker
- [ ] Spec §2.1 stack size audit — deferred
- [ ] Spec §2.1 deferred creation audit — deferred
- [ ] Spec §2.1 thread naming audit — deferred
- [ ] Spec §2.5 detached-thread creation failure contract vs current detached ABI — documented open blocker

## Plan Sign-off

- [ ] **PLAN-20260314-THREADING: COMPLETE AS A TARGETED REMEDIATION PLAN** — Implemented slices are complete and verified, and unmet normative requirements are explicitly reported as open blockers/deferred audits rather than incorrectly claimed closed.
- [ ] **NOT FULL SUBSYSTEM SPEC-PARITY SIGN-OFF** — full parity still requires detached-thread failure-contract resolution plus the outstanding audit-blocked requirement decisions.

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P09a.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
