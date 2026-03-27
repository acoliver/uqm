# Phase 16: Final Parity Sign-Off

## Phase ID
`PLAN-20260325-COMMPT3.P16`

## Prerequisites
- Required: Phase P15a (Integration Build Verification) completed
- Expected: all builds pass, all tests pass, zero deferred markers

## Requirements Verified

### REQ-E2E-001: Full Encounter Visual Parity
### REQ-E2E-002: Conversation Summary Parity
### REQ-E2E-003: Response Selection Parity
### REQ-E2E-004: Replay Parity
### REQ-TS-001: AlienTalkSegue Intro Sequence
### REQ-TS-002: Per-Frame Talk Segue Operations
### REQ-TS-003: Talking Animation Control
### REQ-TS-004: Track Completion Detection

## Purpose
Final acceptance gate: E2E runtime verification and complete requirement sign-off.

## Implementation Tasks

### Files to create / modify
- None (sign-off only)

## Pseudocode Traceability
- Full encounter flow: pseudocode `005-end-to-end-integration.md` lines 01-71
- User trigger paths: pseudocode `005-end-to-end-integration.md` lines 87-98
- Deadlock-free criteria: pseudocode `005-end-to-end-integration.md` lines 72-81
- Build verification: pseudocode `005-end-to-end-integration.md` lines 82-86

## Traceability Marker Audits (MANDATORY — missing = FAIL)

### @plan marker audit
```bash
fail=0
for f in rust/src/comm/ffi.rs rust/src/comm/talk_segue.rs sc2/src/uqm/rust_comm.c sc2/src/uqm/comm.c sc2/src/uqm/rust_comm.h; do
  count=$(grep -c "@plan PLAN-20260325-COMMPT3" "$f" 2>/dev/null || echo 0)
  echo "$f: $count @plan markers"
  if [ "$count" -eq 0 ]; then echo "FAIL: no @plan marker in $f"; fail=1; fi
done
[ "$fail" -eq 0 ] && echo "PASS" || echo "FAIL: do NOT sign off"
```

### @requirement marker audit
```bash
fail=0
for f in rust/src/comm/ffi.rs rust/src/comm/talk_segue.rs sc2/src/uqm/rust_comm.c sc2/src/uqm/comm.c; do
  count=$(grep -c "@requirement" "$f" 2>/dev/null || echo 0)
  echo "$f: $count @requirement markers"
  if [ "$count" -eq 0 ]; then echo "FAIL: no @requirement marker in $f"; fail=1; fi
done
[ "$fail" -eq 0 ] && echo "PASS" || echo "FAIL: do NOT sign off"
```

### @pseudocode audit
```bash
fail=0
grep -q "@pseudocode 001" rust/src/comm/talk_segue.rs || { echo "FAIL: P03/P05 ref"; fail=1; }
grep -q "@pseudocode 001" sc2/src/uqm/rust_comm.c || { echo "FAIL: P03/P05 ref"; fail=1; }
grep -q "@pseudocode 002" sc2/src/uqm/comm.c || { echo "FAIL: P06/P08 ref"; fail=1; }
grep -q "@pseudocode 002" sc2/src/uqm/rust_comm.c || { echo "FAIL: P06/P08 ref"; fail=1; }
grep -q "@pseudocode 003" rust/src/comm/ffi.rs || { echo "FAIL: P09/P11 ref"; fail=1; }
grep -q "@pseudocode 003" rust/src/comm/talk_segue.rs || { echo "FAIL: P09/P11 ref"; fail=1; }
grep -q "@pseudocode 004" rust/src/comm/ffi.rs || { echo "FAIL: P12/P14 ref"; fail=1; }
[ "$fail" -eq 0 ] && echo "PASS" || echo "FAIL: do NOT sign off"
```

### Stub→TDD→Impl completion audit
```bash
fail=0
for phase in P03 P04 P05 P06 P07 P08 P09 P10 P11 P12 P13 P14; do
  if [ -f "project-plans/20260311/commpt3/.completed/${phase}.md" ]; then
    echo "$phase: exists"
  else
    echo "FAIL: $phase missing"; fail=1
  fi
done
[ "$fail" -eq 0 ] && echo "PASS" || echo "FAIL: do NOT sign off"
```

## Manual Runtime Verification Checklist

### Encounter Entry (REQ-E2E-001, REQ-TS-001)
- [ ] Launch game with `USE_RUST_COMM=on`
- [ ] Encounter alien — portrait displays with correct colors (REQ-CM-001)
- [ ] Background music plays (REQ-MU-001)

### Subtitle Display (REQ-E2E-001, REQ-TS-002)
- [ ] Subtitles appear on screen during NPC speech
- [ ] Subtitles synchronized with audio
- [ ] Subtitles update as new phrases play

### Talking Animation (REQ-TS-003, REQ-TS-004)
- [ ] Talking animation plays during speech
- [ ] Animation stops when speech ends
- [ ] Music fades to foreground after speech

### Response Selection (REQ-E2E-003)
- [ ] Response options display after talking
- [ ] Up/Down navigation with wrapping
- [ ] Selection advances conversation (no deadlock — REQ-E2E-007)

### Conversation Summary (REQ-E2E-002)
- [ ] Cancel shows summary with paging
- [ ] Return from summary works

### Replay (REQ-E2E-004)
- [ ] Left key replays last phrase
- [ ] No callback re-fire

### Multi-Encounter Stability
- [ ] Multiple encounters without state leak
- [ ] Abort (ESC) exits cleanly
- [ ] Load exits cleanly
- [ ] No crashes across 5+ encounters

### C-Only Fallback (REQ-E2E-005)
- [ ] `USE_RUST_COMM=off` compiles and works

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
cd rust && cargo test --workspace --all-features -- comm 2>&1 | grep "test result"
```

## Deferred Implementation Detection (Mandatory — Final Sweep)

```bash
echo "=== FINAL PRODUCTION SWEEP ==="
fail=0
for f in rust/src/comm/*.rs; do
  grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|not yet wired\|not yet implemented\|todo!\|unimplemented!' "$f" 2>/dev/null | while IFS= read -r line; do
    lineno=$(echo "$line" | cut -d: -f1)
    content=$(sed -n "${lineno}p" "$f")
    if echo "$content" | grep -q 'stubs in commanim'; then echo "EXEMPT (C ref): $f:$line"
    elif echo "$content" | grep -q 'not yet disabled this encounter'; then echo "EXEMPT (design): $f:$line"
    elif echo "$content" | grep -q 'not yet initialized'; then echo "EXEMPT (sentinel): $f:$line"
    elif echo "$content" | grep -q '^ *///'; then echo "EXEMPT (doc): $f:$line"
    elif echo "$content" | grep -q 'cfg(test)'; then echo "EXEMPT (test): $f:$line"
    else echo "FAIL: $f:$line"; fail=1; fi
  done
done
grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|not yet\|stub' sc2/src/uqm/rust_comm.c 2>/dev/null && fail=1 || true
awk '/#ifdef USE_RUST_COMM/,/#endif/' sc2/src/uqm/comm.c | grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|not yet' && fail=1 || true
[ "$fail" -eq 0 ] && echo "PASS: all clean" || echo "FAIL: do NOT sign off"
```

## Structural Verification Checklist
- [ ] All automated tests pass (268+)
- [ ] Both build modes compile
- [ ] Zero deferred markers
- [ ] All 12 implementation phase completion markers present

## Semantic Verification Checklist — Full Requirement Coverage

- [ ] REQ-CM-001..003: Colormap applied from CommData — **VERIFIED**
- [ ] REQ-MU-001..003: Music plays from CommData — **VERIFIED**
- [ ] REQ-SD-001..005: Subtitles render on screen — **VERIFIED**
- [ ] REQ-CS-002..003: Summary delegates to C — **VERIFIED**
- [ ] REQ-RL-001..004: No deadlock during response selection — **VERIFIED**
- [ ] REQ-DC-001..005: State machine correct — **VERIFIED**
- [ ] REQ-TS-001..004: Intro + per-frame + animations + completion — **VERIFIED**
- [ ] REQ-SM-001..002: Zero stale markers — **VERIFIED**
- [ ] REQ-E2E-001..007: Full parity — **VERIFIED**

## Final Gate Decision
- [ ] **PASS**: Plan PLAN-20260325-COMMPT3 complete. `USE_RUST_COMM=on` production-ready.
- [ ] **FAIL**: Document failures and required remediation.

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P16.md`

Contents:
- phase ID: PLAN-20260325-COMMPT3.P16
- FINAL DECISION: PASS / FAIL
- requirement coverage matrix (all 35 requirements)
- manual test results
- deferred-implementation sweep results
