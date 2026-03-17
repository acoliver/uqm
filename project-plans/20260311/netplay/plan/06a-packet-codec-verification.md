# Phase 06a: Packet Codec & Wire Format — Verification

## Phase ID
`PLAN-20260314-NETPLAY.P06a`

## Prerequisites
- Required: Phase 06 (Packet Codec) completed
- Expected: all packet module files implemented with tests passing

## Verification Tasks

### Wire Compatibility Audit
- [ ] Manually construct the Init packet C would produce and compare byte-for-byte
- [ ] Manually construct a Fleet packet with 2 ships and compare
- [ ] Verify TeamName "Test\0" produces correct padded output
- [ ] Verify BattleInput(0xFF) produces correct 8-byte packet
- [ ] Verify Ready packet is exactly 4 bytes (header only)

### Round-Trip Coverage
- [ ] Every one of the 18 packet types has a serialize→deserialize round-trip test
- [ ] Tests use `assert_eq!` on the deserialized value, not just "no panic"

### Edge Cases
- [ ] Empty fleet (0 ships) serializes correctly
- [ ] Maximum-length team name (25 chars) serializes correctly
- [ ] SelectShip with `0xFFFF` (random) serializes correctly
- [ ] Buffer with exactly one complete packet extracts it
- [ ] Buffer with one complete + one incomplete packet extracts one, retains remainder

### Error Handling
- [ ] Truncated header (< 4 bytes) → error, not panic
- [ ] Unknown packet type → error
- [ ] Packet length < minimum for type → error
- [ ] Packet length not multiple of 4 → error (if enforced)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# Specific packet tests
cargo test --all-features -- netplay::packet
```

## Gate Decision
- [ ] PASS: proceed to Phase 07
- [ ] FAIL: return to Phase 06 and fix issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P06a.md`
