# Phase 03a: Core Types & State Machine — Stub Verification

## Phase ID
`PLAN-20260314-NETPLAY.P03a`

## Prerequisites
- Required: Phase 03 (Core Types Stub) completed
- Expected artifacts: all files listed in P03

## Verification Tasks

### Structural
- [ ] All 14 new files exist at correct paths
- [ ] `lib.rs` modified with feature-gated module declaration
- [ ] `Cargo.toml` has `netplay` feature defined
- [ ] `cargo build --all-features` succeeds
- [ ] `cargo build` (without netplay) succeeds

### Type Correctness
- [ ] `NetState` variant count matches C `netstate.h` (10 variants)
- [ ] `NetState::name()` returns correct debug strings for all variants
- [ ] `AbortReason` variant values can be mapped to C `packet.h:47-55` integers
- [ ] `ResetReason` covers manual and sync-loss cases
- [ ] `NetplayError` variants cover all error paths from analysis
- [ ] `StateFlags` field layout matches C `netconnection.h:95-143` semantics
- [ ] `PeerOptions` defaults match C `netoptions.c:19-37`
- [ ] All constants match C `netplay.h:24-55`

### Feature Gating
- [ ] `#[cfg(feature = "netplay")]` on the module declaration in `lib.rs`
- [ ] Module is completely absent from compilation without the feature
- [ ] No leakage of netplay types into non-netplay builds

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: return to Phase 03 and fix issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P03a.md`
