# Phase 06a: P06 Verification

## Phase ID
`PLAN-20260707-MAINLOOP.P06a`

## Verifies
P06 (Rust Entry Point + Game Loop)

## Requirements Verified
REQ-ML-001, REQ-ML-007

## Verification Gate
```bash
cargo test --workspace -- mainloop::game_loop
cargo clippy --workspace --all-targets -- -D warnings
grep -RIn "TODO\|FIXME\|HACK" rust/src/mainloop/game_loop.rs
```

## Checklist
- [ ] `test_rust_game_loop_returns_exit_code` PASS
- [ ] `test_game_loop_structure` PASS
- [ ] `rust_game_loop` is `#[no_mangle] pub extern "C" fn` returning `c_int`
- [ ] Outer loop governed by `StartGame()` return value
- [ ] Inner loop governed by `CHECK_ABORT` flag
- [ ] Win/loss/death conditions match starcon.c:275-290
- [ ] Zero deferred-implementation hits

## Decision
- [ ] PASS → proceed to P07
- [ ] FAIL → remediate P06 before continuing
