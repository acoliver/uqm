# Phase 10a: Setup Sync & Notifications — Verification

## Phase ID
`PLAN-20260314-NETPLAY.P10a`

## Prerequisites
- Required: Phase 10 (Setup Sync & Notifications) completed
- Expected: notify module fully implemented with tests passing

## Verification Tasks

### Per-Connection Notifications
- [ ] Every notification function validates the correct connection state
- [ ] Setup notifications reject calls when `handshake.local_ok` is true
- [ ] Battle notifications reject calls outside battle-active states
- [ ] SeedRandom enforces discriminant check
- [ ] All notifications produce correctly serialized packets via P06 codec
- [ ] All setup-notification signatures use canonical `FleetSide`, `FleetSlot`, and `ShipId` widths

### Broadcast
- [ ] Broadcast iterates ALL player slots (`0..NUM_PLAYERS`)
- [ ] Broadcast skips `None` entries
- [ ] Broadcast skips disconnected entries
- [ ] Broadcast skips entries in wrong state
- [ ] Individual errors do not prevent remaining broadcasts

### Integration Readiness
- [ ] Notification API is sufficient for SuperMelee to:
  - send fleet changes
  - send team name changes
  - send full fleet bootstrap
  - send battle input
  - send checksums
  - send ship selections
- [ ] Notification layer is transport-only and does not duplicate the deterministic setup conflict-resolution policy owned by P12 integration

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::notify
```

## Gate Decision
- [ ] PASS: proceed to Phase 12
- [ ] FAIL: return to Phase 10 and fix issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P10a.md`
