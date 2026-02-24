# Plan: State File I/O Migration (C → Rust)

Plan ID: `PLAN-20260224-STATE-SWAP`
Generated: 2026-02-24
Total Phases: 25 (P00.5 through P12, each with verification sub-phase)
Requirements: REQ-SF-001 through REQ-SF-009, REQ-SFILE-001 through REQ-SFILE-R008, REQ-STATE-R005

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. `unsafe` is explicitly approved for FFI boundary code

## Scope

### IN SCOPE
- Fix seek-past-end clamping in `StateFile::seek`
- Fix separate `used` vs physical size tracking in `StateFile`
- Fix copy deadlock in `rust_copy_game_state`
- Fix `open_count` type (u32 → i32) to match C semantics
- Add `USE_RUST_STATE` to `config_unix.h` (initially disabled)
- Add `#ifdef USE_RUST_STATE` redirects in `state.c`
- Verify both build paths
- Integration verification (save/load round-trip)

### OUT OF SCOPE
- Game state bits macro redirect (GET/SET_GAME_STATE) — 1,964 call sites
- `planet_info.rs` moon count bug
- `NUM_GAME_STATE_BITS` mismatch (2048 vs 1238)
- Save/load format changes

## Slices

| Slice | Name | Phases | Description |
|---|---|---|---|
| A | Analysis + Pseudocode | P01–P02 | Domain analysis and algorithmic pseudocode |
| B | Seek-Past-End Fix | P03–P05 | Fix StateFile::seek clamping and used/physical tracking |
| C | Copy Deadlock Fix | P06–P08 | Fix rust_copy_game_state single-lock pattern |
| D | C Redirect Wiring | P09–P11 | USE_RUST_STATE flag and state.c redirects |
| E | Integration Verification | P12 | End-to-end save/load round-trip |

## Phase Map

| Phase | Type | Slice | Description |
|---|---|---|---|
| P00.5 | Preflight | — | Toolchain, deps, types, test infra |
| P01 | Analysis | A | Domain model, flow analysis |
| P01a | Verification | A | Analysis verification |
| P02 | Pseudocode | A | Algorithmic pseudocode |
| P02a | Verification | A | Pseudocode verification |
| P03 | Stub | B | Seek-past-end stub (add `used` field, adjust signatures) |
| P03a | Verification | B | Stub verification |
| P04 | TDD | B | Tests for seek-past-end, read-after-seek, write-after-seek |
| P04a | Verification | B | TDD verification |
| P05 | Impl | B | Implement seek-past-end fix |
| P05a | Verification | B | Implementation verification |
| P06 | Stub | C | Deadlock fix stub (add `copy_state_self` signature) |
| P06a | Verification | C | Stub verification |
| P07 | TDD | C | Test: copy doesn't deadlock, self-copy correctness |
| P07a | Verification | C | TDD verification |
| P08 | Impl | C | Implement deadlock fix |
| P08a | Verification | C | Implementation verification |
| P09 | Stub | D | USE_RUST_STATE in config_unix.h, #ifdef scaffolding in state.c |
| P09a | Verification | D | Stub verification |
| P10 | TDD | D | Build with USE_RUST_STATE=0 and USE_RUST_STATE=1 |
| P10a | Verification | D | TDD verification |
| P11 | Impl | D | Enable USE_RUST_STATE, full build |
| P11a | Verification | D | Implementation verification |
| P12 | Integration | E | Save/load round-trip, cargo test, full build verification |
| P12a | Verification | E | Integration verification |

## Execution Tracker

| Phase | Status | Verified | Semantic Verified | Notes |
|------:|--------|----------|-------------------|-------|
| P00.5 | ⬜     | ⬜       | N/A               |       |
| P01   | ⬜     | ⬜       | ⬜                |       |
| P01a  | ⬜     | ⬜       | ⬜                |       |
| P02   | ⬜     | ⬜       | ⬜                |       |
| P02a  | ⬜     | ⬜       | ⬜                |       |
| P03   | ⬜     | ⬜       | ⬜                |       |
| P03a  | ⬜     | ⬜       | ⬜                |       |
| P04   | ⬜     | ⬜       | ⬜                |       |
| P04a  | ⬜     | ⬜       | ⬜                |       |
| P05   | ⬜     | ⬜       | ⬜                |       |
| P05a  | ⬜     | ⬜       | ⬜                |       |
| P06   | ⬜     | ⬜       | ⬜                |       |
| P06a  | ⬜     | ⬜       | ⬜                |       |
| P07   | ⬜     | ⬜       | ⬜                |       |
| P07a  | ⬜     | ⬜       | ⬜                |       |
| P08   | ⬜     | ⬜       | ⬜                |       |
| P08a  | ⬜     | ⬜       | ⬜                |       |
| P09   | ⬜     | ⬜       | ⬜                |       |
| P09a  | ⬜     | ⬜       | ⬜                |       |
| P10   | ⬜     | ⬜       | ⬜                |       |
| P10a  | ⬜     | ⬜       | ⬜                |       |
| P11   | ⬜     | ⬜       | ⬜                |       |
| P11a  | ⬜     | ⬜       | ⬜                |       |
| P12   | ⬜     | ⬜       | ⬜                |       |
| P12a  | ⬜     | ⬜       | ⬜                |       |

## Integration Contract

### Existing Callers (UNCHANGED)
- `state.c` data ops (InitPlanetInfo, GetPlanetInfo, PutPlanetInfo) → OpenStateFile, SeekStateFile, sread_*/swrite_*, CloseStateFile, DeleteStateFile
- `grpinfo.c` (InitGroupInfo, GetGroupInfo, PutGroupInfo, FlushGroupInfo) → all 7 functions
- `save.c` (SaveStarInfo, SaveBattleGroup, SaveGroups) → Open, Length, Seek, sread_*, Close
- `load.c` (LoadScanInfo, LoadGroupList, LoadBattleGroup) → Open, Length, Seek, swrite_*, Close
- `load_legacy.c` (LoadLegacyGame) → Open, WriteStateFile (direct), Close

### Existing Code Replaced/Removed
- `state.c` lines 53–226: 7 function bodies replaced by `#ifdef USE_RUST_STATE` blocks
- Original C implementations preserved in `#else` blocks

### User Access Path
- Start game → enter solar system → scan planet → state file accessed
- Save game → state files read for serialization
- Load game → state files written from deserialization

### Data/State Migration
- No migration needed. State file buffers are volatile (rebuilt from save data on each load).

### End-to-End Verification
- `cargo test --workspace --all-features` — Rust tests pass
- Build with `USE_RUST_STATE=0` — C path works (regression check)
- Build with `USE_RUST_STATE=1` — Rust path works
- Save game → quit → reload → verify gameplay state intact
