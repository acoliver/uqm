# P18-P19: Automation proof scripts and final verification

## P18: Create and run automation proof scripts

### Scripts to create

1. **`comm-encounter-v1.json`** — Tests comm dispatch (P09)
   ```
   wait 10 ticks (let game load)
   tap select (start NewGame)
   wait 30 ticks (let game enter hyperspace)
   assert_activity mask=0x00FF equals=0x0003 (IN_HYPERSPACE)
   wait 20 ticks (let encounter happen)
   assert_activity mask=0x00FF equals=0x0002 (IN_ENCOUNTER)
   capture "comm-encounter"
   tap select (choose to talk)
   wait 10 ticks
   capture "comm-dialogue"
   finish
   ```

2. **`explore-planet-v1.json`** — Tests ExploreSolarSys (P14)
   ```
   wait 10 ticks
   tap select (start NewGame)
   wait 30 ticks
   assert_activity mask=0x00FF equals=0x0003 (IN_HYPERSPACE)
   [navigate to planet - may need additional key actions]
   assert_activity mask=0x00FF equals=0x0004 (IN_INTERPLANETARY)
   capture "planet-explore"
   finish
   ```

3. **`starbase-visit-v1.json`** — Tests VisitStarBase (P15)
   ```
   wait 10 ticks
   tap select (start NewGame)
   wait 30 ticks
   assert_activity mask=0x00FF equals=0x0003 (IN_HYPERSPACE)
   [navigate to starbase]
   assert_activity mask=0x00FF equals=0x0008 (IN_STARBASE)
   capture "starbase"
   finish
   ```

4. **`battle-v1.json`** — Tests Battle dispatch (P17)
   ```
   wait 10 ticks
   tap select (start NewGame)
   wait 30 ticks
   assert_activity mask=0x00FF equals=0x0003 (IN_HYPERSPACE)
   wait 20 ticks (encounter)
   assert_activity mask=0x00FF equals=0x0002 (IN_ENCOUNTER)
   [choose attack]
   assert_activity mask=0xFF00 equals=0x0200 (IN_BATTLE flag)
   capture "battle"
   finish
   ```

### Verification criteria per proof
- Exit code 0
- Teardown receipt with correct terminal class
- Trace.jsonl with expected number of records
- PNG captures (where capture steps exist)
- Activity assertions pass

## P19: Final verification

### Acceptance criteria
1. All 4 quality gates pass:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets --features audio_heart -- -D warnings`
   - `cargo test --lib --features audio_heart -- --test-threads=1`
   - `cargo build --bin uqm --release --features audio_heart,linked_c_archive`

2. All automation proofs pass against real binary:
   - main-menu-v1 (existing, must still pass)
   - watchdog-v1 (existing, must still pass)
   - inactive-smoke (existing, must still pass)
   - hard-hang (existing, must still pass)
   - comm-encounter-v1 (new)
   - explore-planet-v1 (new)
   - starbase-visit-v1 (new)
   - battle-v1 (new)

3. Game is playable interactively (launch without automation, play normally)

4. No regressions in existing functionality

### Final commit
- Commit all ported code
- Push to origin/main
- Verify repository status is clean