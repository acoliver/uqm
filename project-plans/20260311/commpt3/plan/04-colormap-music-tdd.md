# Phase 04: Colormap + Music Bridge — TDD

## Phase ID
`PLAN-20260325-COMMPT3.P04`

## Prerequisites
- Required: Phase P03a (Colormap + Music Stub Verification) completed
- Expected: stubs compile, call sites rewired, no functional behavior

## Requirements Tested
- REQ-CM-001: Colormap bridge call during AlienTalkSegue
- REQ-CM-002: Colormap bridge reads CommData.AlienColorMap, no-op on zero
- REQ-CM-003: Colormap reflects current CommData value (not cached)
- REQ-MU-001: Music bridge call during encounter startup
- REQ-MU-002: Music bridge reads CommData.AlienSong, no-op on zero
- REQ-MU-003: Music playing before first AlienTalkSegue
- REQ-SM-001: "for now" marker removed from set_colormap path

## Purpose
Write behavior-driven tests that define expected colormap/music bridge behavior.
Tests MUST fail against the current stubs (proving they test real behavior, not stubs).

## Test Tasks

### Rust `#[cfg(test)]` tests (talk_segue.rs or dedicated test module)
1. **test_set_colormap_calls_bridge**: verify `set_colormap()` calls `c_SetColorMapFromCommData()`
   (not `null_mut`) — should already pass since P03 rewired the call site
2. **test_play_alien_music_calls_bridge**: verify `play_alien_music()` calls `c_PlayAlienMusic()`
   (not `null_mut`) — should already pass since P03 rewired the call site
3. **test_for_now_marker_removed**: grep-based verification that "for now" no longer appears
   in `set_colormap` production code — structural test

### C build-verification tests (grep-based structural tests)
4. **verify_c_bridge_functions_exist**: `c_SetColorMapFromCommData` and `c_PlayAlienMusic`
   exist in `rust_comm.c` with non-empty bodies (will fail against stubs until P05 impl)
5. **verify_c_bridge_reads_commdata**: `c_SetColorMapFromCommData` body contains
   `CommData.AlienColorMap` reference (will fail against stubs until P05 impl)
6. **verify_c_bridge_null_guard**: `c_SetColorMapFromCommData` body contains null/zero guard
   (will fail against stubs until P05 impl)
7. **verify_c_music_reads_commdata**: `c_PlayAlienMusic` body contains `CommData.AlienSong`
   reference (will fail against stubs until P05 impl)

### Expected failures against stubs (MUST be documented)
Tests 4-7 MUST fail against the current stubs — this proves they test real behavior.
Document expected failures:
- `c_SetColorMapFromCommData` has empty body → tests 5, 6 fail
- `c_PlayAlienMusic` has empty body → test 7 fails

## Pseudocode Traceability
- Tests trace to pseudocode `001-colormap-music-bridges.md`:
  - Lines 01-08: `c_SetColorMapFromCommData` behavior (tests 4-6)
  - Lines 09-15: `c_PlayAlienMusic` behavior (test 7)
  - Lines 16-21: `set_colormap` caller fix (test 1, 3)
  - Lines 22-27: `play_alien_music` caller fix (test 2)

## Traceability Markers (in test code)
```rust
/// @plan PLAN-20260325-COMMPT3.P04
/// @requirement REQ-CM-001, REQ-CM-002, REQ-MU-001, REQ-MU-002
/// @pseudocode 001-colormap-music-bridges lines 01-31
```

## Verification Commands

```bash
# Tests compile
cd rust && cargo test --workspace --all-features --no-run

# Run tests — document which pass and which fail against stubs
cd rust && cargo test --workspace --all-features 2>&1 | tee /tmp/tdd-results.txt
grep -E "FAIL|ok|FAILED" /tmp/tdd-results.txt

# C structural tests (grep-based — expected to fail against stubs)
echo "=== C bridge body verification (expected FAIL against stubs) ==="
grep -A5 "c_SetColorMapFromCommData" sc2/src/uqm/rust_comm.c | grep "CommData.AlienColorMap" && echo "PASS" || echo "EXPECTED FAIL: empty stub body"
grep -A5 "c_SetColorMapFromCommData" sc2/src/uqm/rust_comm.c | grep -E "if.*==.*0|if.*!|NULL" && echo "PASS" || echo "EXPECTED FAIL: no null guard in stub"
grep -A5 "c_PlayAlienMusic" sc2/src/uqm/rust_comm.c | grep "CommData.AlienSong" && echo "PASS" || echo "EXPECTED FAIL: empty stub body"
```

## Structural Verification Checklist
- [ ] All new tests compile (`cargo test --no-run`)
- [ ] Tests for Rust call-site rewiring pass (tests 1-3)
- [ ] Tests for C bridge behavior are written and expected to fail against stubs (tests 4-7)
- [ ] Expected failures documented with rationale

## Semantic Verification Checklist (Mandatory)
- [ ] Tests assert real behavior (CommData reads, null guards), not stubs
- [ ] Tests 4-7 genuinely fail against current stubs — not vacuously passing
- [ ] No tests that pass with non-functional implementation

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Negative proof**: Tests 4-7 fail against the current P03 stubs, proving they
  will only pass when real implementation exists in P05
- [ ] **Negative proof**: If `c_SetColorMapFromCommData` body were changed to
  `SetColorMap(0)` (wrong — no CommData read), test 5 would still fail
- [ ] **Negative proof**: If `c_PlayAlienMusic` body were changed to `PlayMusic(NULL, 0, 0)`
  (wrong args), test 7 would still fail

## Success Criteria
- [ ] All tests compile
- [ ] Rust call-site tests pass (confirming P03 wiring)
- [ ] C behavioral tests fail as expected (confirming need for P05)
- [ ] Expected failures are documented

## Failure Recovery
- rollback: `git restore` test files
- If Rust tests fail unexpectedly, review P03 stub wiring

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P04.md`

Contents:
- Tests written with expected failure documentation
- Pass/fail matrix for all tests against stubs
