# Phase 05a: Colormap + Music Bridge Implementation Verification

## Phase ID
`PLAN-20260325-COMMPT3.P05a`

## Prerequisites
- Required: Phase P05 completed
- Expected artifacts: Implemented `c_SetColorMapFromCommData()`, `c_PlayAlienMusic()`

## Verification Commands

```bash
# Full quality gates
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify implementation substance
grep -A10 "c_SetColorMapFromCommData" sc2/src/uqm/rust_comm.c | grep "CommData.AlienColorMap"
grep -A10 "c_SetColorMapFromCommData" sc2/src/uqm/rust_comm.c | grep "SetColorMap"
grep -A10 "c_PlayAlienMusic" sc2/src/uqm/rust_comm.c | grep "CommData.AlienSong"
grep -A10 "c_PlayAlienMusic" sc2/src/uqm/rust_comm.c | grep "PlayMusic"

# Verify null guards present
grep -A10 "c_SetColorMapFromCommData" sc2/src/uqm/rust_comm.c | grep -E "if.*==.*0|if.*!"
grep -A10 "c_PlayAlienMusic" sc2/src/uqm/rust_comm.c | grep -E "if.*==.*0|if.*!"

# Verify stale marker gone
grep -n "for now" rust/src/comm/talk_segue.rs && echo "FAIL" || echo "PASS"

# Verify no null_mut in production
grep -n "null_mut" rust/src/comm/talk_segue.rs | grep -v test | grep -v "cfg(test)" && echo "FAIL" || echo "PASS"

# No other callers of removed extern functions
grep -rn "c_SetColorMap\b" rust/src/comm/ | grep -v "FromCommData" | grep -v test
grep -rn "c_PlayMusic\b" rust/src/comm/ | grep -v "AlienMusic" | grep -v test
```

## Structural Verification Checklist
- [ ] `c_SetColorMapFromCommData()` has implemented body (not stub)
- [ ] `c_PlayAlienMusic()` has implemented body (not stub)
- [ ] Both declared in `rust_comm.h`
- [ ] `set_colormap()` calls `c_SetColorMapFromCommData()`
- [ ] `play_alien_music()` calls `c_PlayAlienMusic()`
- [ ] "pass null for now" comment gone
- [ ] No `todo!()` or placeholder markers in any modified file

## Semantic Verification Checklist (Mandatory)
- [ ] `c_SetColorMapFromCommData` reads `CommData.AlienColorMap` (not a parameter)
- [ ] `c_SetColorMapFromCommData` has zero-handle guard (no-op on zero)
- [ ] `c_SetColorMapFromCommData` calls `SetColorMap(GetColorMapAddress(...))`
- [ ] `c_PlayAlienMusic` reads `CommData.AlienSong` (not a parameter)
- [ ] `c_PlayAlienMusic` has zero-handle guard (no-op on zero)
- [ ] `c_PlayAlienMusic` calls `PlayMusic(song, TRUE, 1)`
- [ ] All P04 TDD behavioral tests now PASS
- [ ] All 268+ comm tests pass
- [ ] Both `USE_RUST_COMM=on` and `=off` builds compile

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: Breaking the CommData read (e.g., passing `0` instead of
  `CommData.AlienColorMap`) causes the TDD CommData-read test to fail
- [ ] **Confirmed**: Breaking the PlayMusic args (e.g., `FALSE` instead of `TRUE`)
  causes the TDD args test to fail
- [ ] **Confirmed**: Re-adding "for now" comment causes marker-absence test to fail

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P05a.md`
