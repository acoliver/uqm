# Phase 03a: P03 Verification

## Phase ID
`PLAN-20260707-MAINLOOP.P03a`

## Verifies
P03 (Activity Types + CurrentActivity FFI Accessors)

## Requirements Verified
REQ-ML-003, REQ-ML-005 (foundation), REQ-ML-010

## Verification Gate
```bash
cargo test --workspace -- mainloop::bridge
cargo clippy --workspace --all-targets -- -D warnings
grep -RIn "TODO\|FIXME\|HACK" rust/src/mainloop/activity.rs rust/src/mainloop/bridge.rs rust/src/mainloop/c_extern.rs
```

## Checklist
- [ ] `test_current_activity_round_trip_rust_to_c` PASS
- [ ] `test_current_activity_round_trip_c_to_rust` PASS
- [ ] `test_game_state_byte_round_trip` PASS
- [ ] `test_activity_flags_decomposition` PASS
- [ ] Zero deferred-implementation hits
- [ ] No `unwrap()`/`expect()` in bridge.rs

## Decision
- [ ] PASS → proceed to P04
- [ ] FAIL → remediate P03 before continuing
