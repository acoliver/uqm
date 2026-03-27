# Phase 05: Colormap + Music Bridge — Implementation

## Phase ID
`PLAN-20260325-COMMPT3.P05`

## Prerequisites
- Required: Phase P04a (Colormap + Music TDD Verification) completed
- Expected: tests written, expected failures documented against stubs

## Requirements Implemented

### REQ-CM-001: Colormap Bridge Call During AlienTalkSegue
WHEN `set_colormap` is called during `AlienTalkSegue` intro setup,
the system SHALL call `c_SetColorMapFromCommData()` which executes
`SetColorMap(GetColorMapAddress(CommData.AlienColorMap))` directly.

### REQ-CM-002: Colormap Bridge Reads CommData.AlienColorMap
The bridge function SHALL obtain the handle from `CommData.AlienColorMap`
and SHALL be a no-op if the handle is zero.

### REQ-CM-003: Colormap Reflects Current CommData Value
The colormap applied SHALL reflect the current `CommData.AlienColorMap` value
each time the bridge is called (not cached).

### REQ-MU-001: Music Bridge Call During Encounter Startup
WHEN `play_alien_music` is called, the system SHALL call `c_PlayAlienMusic()`
which executes `PlayMusic(CommData.AlienSong, TRUE, 1)` directly.

### REQ-MU-002: Music Bridge Reads CommData.AlienSong
The bridge function SHALL obtain the song handle from `CommData.AlienSong`
and SHALL be a no-op if the handle is zero.

### REQ-MU-003: Music Playing Before First AlienTalkSegue
Music SHALL be playing at background volume when the first `AlienTalkSegue` executes.

### REQ-SM-001: "for now" Marker Removed
Remove "pass null for now" comment from `set_colormap` code path.

## Implementation Tasks

### Files to modify
- `sc2/src/uqm/rust_comm.c`
  - Implement `c_SetColorMapFromCommData()` body: reads `CommData.AlienColorMap`,
    guards against zero, calls `SetColorMap(GetColorMapAddress(handle))`
  - Implement `c_PlayAlienMusic()` body: reads `CommData.AlienSong`,
    guards against zero, calls `PlayMusic(song, TRUE, 1)`
  - marker: `@plan PLAN-20260325-COMMPT3.P05`
  - marker: `@requirement REQ-CM-001, REQ-CM-002, REQ-MU-001, REQ-MU-002`

- `rust/src/comm/talk_segue.rs`
  - Remove "pass null for now" comment (REQ-SM-001)
  - marker: `@plan PLAN-20260325-COMMPT3.P05`
  - marker: `@requirement REQ-SM-001`

### Files to create
- None

## Pseudocode Traceability
- `c_SetColorMapFromCommData`: pseudocode `001-colormap-music-bridges.md` lines 01-08
  - Contract: REQ-CM-001 (bridge call), REQ-CM-002 (null guard), REQ-CM-003 (reads current CommData)
- `c_PlayAlienMusic`: pseudocode `001-colormap-music-bridges.md` lines 09-15
  - Contract: REQ-MU-001 (bridge call), REQ-MU-002 (null guard)
- Marker removal: pseudocode `001-colormap-music-bridges.md` lines 16-21
  - Contract: REQ-SM-001 (marker removed)

## Traceability Markers (in code)
```c
/* @plan PLAN-20260325-COMMPT3.P05 */
/* @requirement REQ-CM-001, REQ-CM-002, REQ-MU-001, REQ-MU-002 */
/* @pseudocode 001-colormap-music-bridges lines 01-31 */
```

## Verification Commands

```bash
# All quality gates
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify implementation reads CommData
grep -A10 "c_SetColorMapFromCommData" sc2/src/uqm/rust_comm.c | grep "CommData.AlienColorMap"
grep -A10 "c_PlayAlienMusic" sc2/src/uqm/rust_comm.c | grep "CommData.AlienSong"

# Verify null guards
grep -A10 "c_SetColorMapFromCommData" sc2/src/uqm/rust_comm.c | grep -E "if.*==.*0|if.*!"
grep -A10 "c_PlayAlienMusic" sc2/src/uqm/rust_comm.c | grep -E "if.*==.*0|if.*!"

# Verify stale marker removed
grep -n "for now" rust/src/comm/talk_segue.rs && echo "FAIL: stale marker" || echo "PASS"

# Verify old null passthrough is gone
grep -n "null_mut" rust/src/comm/talk_segue.rs | grep -v test | grep -v "cfg(test)" && echo "FAIL" || echo "PASS"
```

## Structural Verification Checklist
- [ ] `c_SetColorMapFromCommData()` has non-empty implementation body
- [ ] `c_PlayAlienMusic()` has non-empty implementation body
- [ ] "pass null for now" comment removed
- [ ] No `todo!()`, `unimplemented!()`, or placeholder markers in implementation
- [ ] Plan/requirement/pseudocode markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `c_SetColorMapFromCommData` reads `CommData.AlienColorMap` (not a parameter)
- [ ] `c_SetColorMapFromCommData` is a no-op when `AlienColorMap == 0`
- [ ] `c_SetColorMapFromCommData` calls `SetColorMap(GetColorMapAddress(CommData.AlienColorMap))`
- [ ] `c_PlayAlienMusic` reads `CommData.AlienSong` (not a parameter)
- [ ] `c_PlayAlienMusic` is a no-op when `AlienSong == 0`
- [ ] `c_PlayAlienMusic` calls `PlayMusic(CommData.AlienSong, TRUE, 1)`
- [ ] All TDD tests from P04 now PASS (previously expected failures now succeed)
- [ ] All 268+ comm tests pass
- [ ] Both `USE_RUST_COMM=on` and `=off` builds compile and link

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Negative proof — colormap**: Temporarily change `CommData.AlienColorMap` reference
  to a hardcoded `0` in `c_SetColorMapFromCommData` → null-guard test passes but
  CommData-read test fails. Revert after confirming.
- [ ] **Negative proof — music**: Temporarily change `PlayMusic` args from `(song, TRUE, 1)`
  to `(song, FALSE, 0)` → behavioral test for correct args fails. Revert after confirming.
- [ ] **Negative proof — marker**: Re-add "for now" comment → marker-absence test fails.
  Revert after confirming.

## Deferred Implementation Detection (Mandatory)

```bash
grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|not yet' \
  rust/src/comm/talk_segue.rs sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h | \
  while IFS= read -r line; do
    lineno=$(echo "$line" | cut -d: -f2)
    file=$(echo "$line" | cut -d: -f1)
    content=$(sed -n "${lineno}p" "$file")
    if echo "$content" | grep -q '^ *///'; then echo "EXEMPT (doc): $line"
    elif echo "$content" | grep -q 'cfg(test)'; then echo "EXEMPT (test): $line"
    else echo "FAIL: production deferred marker: $line"; fi
  done || echo "CLEAN"
```

## Success Criteria
- [ ] All P04 TDD tests now pass (previously failing against stubs)
- [ ] Colormap bridge reads real handle from CommData
- [ ] Music bridge reads real handle from CommData
- [ ] No null pointers passed to rendering/audio C functions
- [ ] "for now" marker eliminated
- [ ] All verification commands pass

## Failure Recovery
- rollback: `git restore rust/src/comm/talk_segue.rs sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h`
- blocking issues: if `GetColorMapAddress` is not accessible from `rust_comm.c`, may need
  additional header include

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P05.md`

Contents:
- phase ID: PLAN-20260325-COMMPT3.P05
- files changed: `talk_segue.rs`, `rust_comm.c`
- tests that now pass (were failing in P04)
- negative-proof results
- verification outputs
