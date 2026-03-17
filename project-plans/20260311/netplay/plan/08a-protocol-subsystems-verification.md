# Phase 08a: Protocol Sub-systems — Verification

## Phase ID
`PLAN-20260314-NETPLAY.P08a`

## Prerequisites
- Required: Phase 08 (Protocol Sub-systems) completed
- Expected: ready, confirm, reset protocols implemented with tests passing

## Verification Tasks

### Ready Protocol
- [ ] Callback fires regardless of local-first vs remote-first ordering
- [ ] Ready flags are cleared after both_ready
- [ ] Stored callback is consumed (taken, not cloned)
- [ ] Wrong-state rejection works
- [ ] Duplicate local_ready rejection works
- [ ] Ready packet is only queued when `send_packet=true`

### Confirmation Protocol
- [ ] Full exchange works: Handshake0→Handshake0→Handshake1→Handshake1→complete
- [ ] Cancel→CancelAck works and allows re-confirmation
- [ ] Remote setup change during confirmation invalidates correctly
- [ ] handshake_complete() transitions InSetup→PreBattle
- [ ] Cannot confirm outside InSetup

### Reset Protocol
- [ ] Local-first reset: send Reset, wait for remote Reset, complete
- [ ] Remote-first reset: receive Reset, auto-send confirming Reset, complete
- [ ] Simultaneous reset: both send Reset, both complete
- [ ] Callback fires only when both flags AND callback are present
- [ ] set_reset_callback fires immediately if completion condition already met

### Integration Between Protocols
- [ ] Ready protocol can be used by other phases (not hardcoded to init)
- [ ] Confirmation uses the connection's packet queue (not its own socket)
- [ ] Reset uses the connection's packet queue

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::proto
```

## Gate Decision
- [ ] PASS: proceed to Phase 09
- [ ] FAIL: return to Phase 08 and fix issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P08a.md`
