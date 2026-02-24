# Phase 12: Integration Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P12`

## Prerequisites
- Required: Phase P11a (C Redirect Verification) completed
- USE_RUST_STATE enabled, game runs with Rust state file I/O
- All Rust tests pass, all quality gates pass

## Requirements Implemented (Expanded)

This phase verifies end-to-end integration across all requirements.

### All REQ-SF-* Requirements
All requirements from the specification are verified through integration tests.

## Integration Test Suite

### Test 1: Save/Load Round-Trip
**Purpose**: Verify that game state persists correctly through save and reload.

**Procedure**:
1. Launch game with USE_RUST_STATE enabled
2. Start a new game
3. Perform some gameplay actions (enter star system, scan a planet, encounter a group)
4. Save game (slot 00)
5. Note the game state: coordinates, fuel, crew, scanned planets, date
6. Quit the game
7. Relaunch and load slot 00
8. Verify all noted state matches

**Pass criteria**: All game state fields match after round-trip.

### Test 2: Multiple Save Slots
**Purpose**: Verify multiple state file instances work correctly.

**Procedure**:
1. Save to slot 00
2. Advance game state (move ship, scan planet)
3. Save to slot 01
4. Load slot 00 → verify earlier state
5. Load slot 01 → verify later state

**Pass criteria**: Each slot restores its own state independently.

### Test 3: State File Buffer Operations
**Purpose**: Verify STARINFO, RANDGRPINFO, and DEFGRPINFO all work.

**Procedure**:
1. Start new game → STARINFO initialized (InitPlanetInfo)
2. Enter a star system → STARINFO read/written (GetPlanetInfo/PutPlanetInfo)
3. Encounter random group in interplanetary space → RANDGRPINFO written
4. Encounter scripted group → DEFGRPINFO read
5. Save → all three buffers serialized
6. Load → all three buffers restored

**Pass criteria**: No crashes, no data corruption.

### Test 4: Seek-Past-End (grpinfo.c Integration)
**Purpose**: Verify that grpinfo.c's seek-past-end pattern works with Rust backend.

**Procedure**:
1. Start new game
2. Enter interplanetary space (triggers group info initialization)
3. `FlushGroupInfo` is called internally — this seeks to `LengthStateFile(fp)` and writes
4. Multiple encounters trigger group list updates with seek+write patterns

**Pass criteria**: No crashes, group encounters display correctly.

### Test 5: Copy Game State (Legacy Load Path)
**Purpose**: Verify that the legacy save loader doesn't deadlock.

**Procedure**:
1. If legacy save files are available: attempt to load one
2. If not: this test verifies the Rust unit tests (P07/P08) provide coverage

**Pass criteria**: No deadlock, legacy save loads correctly (if applicable).

### Test 6: Cargo Test Full Suite
**Purpose**: Verify all Rust tests pass with final implementation.

```bash
cd rust && cargo test --workspace --all-features
```

**Pass criteria**: 0 failures, 0 errors.

### Test 7: Full Quality Gate
**Purpose**: Verify code quality standards.

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

**Pass criteria**: All three commands succeed with exit code 0.

### Test 8: Build Both Configurations
**Purpose**: Verify C-only path still works (for fallback).

```bash
# C path
sed -i '' 's/^#define USE_RUST_STATE/\/\* #define USE_RUST_STATE \*\//' sc2/config_unix.h
cd sc2 && make clean && make
# Restore Rust path
sed -i '' 's/^\/\* #define USE_RUST_STATE \*\//#define USE_RUST_STATE/' sc2/config_unix.h
cd sc2 && make clean && make
```

**Pass criteria**: Both configurations build without errors.

## Verification Commands

```bash
# Full quality gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Full build
cd rust && cargo build --release
cd sc2 && make clean && make

# Deferred implementation check
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/state/ || echo "CLEAN"
```

## Structural Verification Checklist
- [ ] All plan phases P03–P11 completed
- [ ] No `todo!()`, `FIXME`, `HACK` in Rust state module
- [ ] USE_RUST_STATE enabled in config_unix.h
- [ ] Build succeeds
- [ ] All Rust tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] Save/load round-trip preserves all game state
- [ ] Multiple save slots work independently
- [ ] All three state files (STARINFO, RANDGRPINFO, DEFGRPINFO) work
- [ ] Seek-past-end pattern (grpinfo.c) works without crash
- [ ] Copy game state doesn't deadlock
- [ ] C-only build still works (fallback path)
- [ ] Game is playable end-to-end with Rust state I/O

## Integration Contract Verification

### Existing Callers
- [ ] `state.c` data ops (InitPlanetInfo, GetPlanetInfo, PutPlanetInfo) → Rust via redirect
- [ ] `grpinfo.c` (InitGroupInfo, GetGroupInfo, PutGroupInfo, FlushGroupInfo) → Rust via redirect
- [ ] `save.c` (SaveStarInfo, SaveGroups, SaveBattleGroup) → Rust via redirect
- [ ] `load.c` (LoadScanInfo, LoadGroupList, LoadBattleGroup) → Rust via redirect
- [ ] `load_legacy.c` (bulk writes to state files) → Rust via redirect

### Old Behavior Replaced
- [ ] C state file buffer management (malloc/realloc/memcpy) → Rust Vec
- [ ] C seek with no upper clamp → Rust seek with no upper clamp (both correct)
- [ ] C read against physical size → Rust read against data.len() (both correct)

### User Access Path
- [ ] New Game → state files initialized → planet/group data readable
- [ ] Save Game → state files serialized → save file written
- [ ] Load Game → save file read → state files reconstructed

### Data/State Migration
- [ ] No data migration needed — save files are binary compatible
- [ ] Rust state files produce byte-for-byte identical save output as C

## Success Criteria
- [ ] All 8 integration tests pass
- [ ] Full quality gate passes
- [ ] Game is playable with Rust state I/O
- [ ] Save files are backward compatible

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P12.md`

Contents:
- phase ID: P12
- integration tests: all 8 pass
- quality gate: fmt/clippy/test all clean
- runtime: game playable, save/load works
- backward compatibility: C build still works, save files compatible
- plan status: COMPLETE
