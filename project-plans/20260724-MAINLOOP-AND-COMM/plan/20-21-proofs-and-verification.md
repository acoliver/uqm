# P20-P21: Automation proof scripts and final verification

## P20: Create and run automation proof scripts

### Scripts to create

1. **`state-sync-v1.json`** — Tests game state ownership (P09)
   - Start game, assert activity flags read correctly from Rust
   - Verify state sync between C reads and Rust writes

2. **`comm-encounter-v1.json`** — Tests comm dispatch (P11)
   - Start new game, wait for hyperspace
   - Wait for encounter, assert IN_ENCOUNTER
   - Capture the comm screen, finish

3. **`explore-planet-v1.json`** — Tests ExploreSolarSys (P16)
   - Navigate to planet, assert IN_INTERPLANETARY, capture, finish

4. **`starbase-visit-v1.json`** — Tests VisitStarBase (P17)
   - Navigate to starbase, assert IN_STARBASE, capture, finish

5. **`battle-v1.json`** — Tests Battle dispatch (P19)
   - Encounter hostile, choose attack, assert IN_BATTLE, capture, finish

### Verification criteria per proof
- Exit code 0
- Teardown receipt with correct terminal class
- Trace.jsonl with expected number of records
- PNG captures (where capture steps exist)
- Activity assertions pass

## P21: Final verification

### Acceptance criteria
1. All 4 quality gates pass:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets --features audio_heart -- -D warnings`
   - `cargo test --lib --features audio_heart -- --test-threads=1`
   - `cargo build --bin uqm --release --features audio_heart,linked_c_archive`

2. All automation proofs pass against real binary:
   - main-menu-v1, watchdog-v1, inactive-smoke, hard-hang (existing)
   - state-sync-v1, comm-encounter-v1, explore-planet-v1, starbase-visit-v1, battle-v1 (new)

3. Game is playable interactively (launch without automation, play normally)

4. Bridge surface reduced: rust_comm.c LOCDATA accessors eliminated,
   game state dual-ownership eliminated

5. No regressions in existing functionality

### Final commit
- Commit all ported code
- Push to origin/main
- Verify repository status is clean