# Phase 09a: Ship Spawn & Lifecycle Verification

## Phase ID
`PLAN-20260314-SHIPS.P09a`

## Prerequisites
- Required: Phase 09 (Spawn & Lifecycle) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `lifecycle.rs` exports spawn, init, uninit functions
- [ ] `game_init/init.rs` delegates correctly

## Semantic Verification Checklist
- [ ] Spawn loads descriptor at BattleReady tier
- [ ] Spawn patches crew from queue entry
- [ ] Spawn creates element with correct callbacks
- [ ] Spawn is idempotent (double-spawn returns early)
- [ ] Spawn failure cleans up all partial state
- [ ] init_ships loads shared assets
- [ ] uninit_ships frees all descriptors with teardown hooks
- [ ] uninit_ships writes back crew
- [ ] Round-trip init→uninit leaves clean state

## Gate Decision
- [ ] PASS: proceed to Phase 10
- [ ] FAIL: return to Phase 09 and fix issues
