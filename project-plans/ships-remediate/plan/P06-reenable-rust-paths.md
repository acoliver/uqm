# Phase 6: Re-enable Rust Paths

## Purpose
After all 28 ships are ported, restore the USE_RUST_SHIPS guards in ship.c
and init.c so battle runs entirely through Rust.

## Files to modify

### sc2/src/uqm/ship.c
- Restore `#ifdef USE_RUST_SHIPS` guard in `ship_preprocess` → `rust_ships_preprocess`
- Restore `#ifdef USE_RUST_SHIPS` guard in `ship_postprocess` → `rust_ships_postprocess`
- Restore `#ifdef USE_RUST_SHIPS` guard in `spawn_ship` → `rust_ships_spawn`

### sc2/src/uqm/init.c
- Restore `#ifdef USE_RUST_SHIPS` guard in `InitShips` → `rust_ships_init`
- Restore `#ifdef USE_RUST_SHIPS` guard in `UninitShips` → `rust_ships_uninit`

### sc2/src/uqm/loadship.c
- Restore `#ifdef USE_RUST_SHIPS` guard in `load_ship` → `rust_ships_load`
- Restore `#ifdef USE_RUST_SHIPS` guard in `free_ship` → `rust_ships_free`

## Verification
- Full clean build with USE_RUST_SHIPS=on
- `cargo test` — all tests pass
- Super melee: pick any two ships, battle works
- All 28 ships: weapons fire, specials work, AI controls ships
- Multiple battles in sequence without crashes
- Battle cleanup works (no resource leaks)
- New Game → encounter → battle → return works

## Acceptance criteria
- Zero C ship code executing when USE_RUST_SHIPS is on
- All ship behavior is Rust
- C ship files are dead code (candidates for removal in future)
