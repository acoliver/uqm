# Phase 16a: Final Parity Sign-Off Verification

## Phase ID
`PLAN-20260325-COMMPT3.P16a`

## Prerequisites
- Required: Phase P16 completed
- Expected artifacts: All manual runtime test results documented

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
cd rust && cargo test --workspace --all-features -- comm 2>&1 | grep "test result"

# Final sweep
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
    else echo "FAIL: $f:$line"; fi
  done
done
grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|not yet\|stub' sc2/src/uqm/rust_comm.c && echo "FAIL" || echo "CLEAN"
```

## Structural Verification Checklist
- [ ] All phases P03 through P15a have completion markers
- [ ] All automated tests pass
- [ ] Both build modes compile
- [ ] Zero deferred markers (per-match classification)
- [ ] `@plan` markers in all 5 modified files
- [ ] `@requirement` markers in all 4 implementation files
- [ ] `@pseudocode` cross-references all present
- [ ] All pseudocode line references verified against actual files

## Semantic Verification Checklist — Full Requirement Sign-Off

- [ ] REQ-CM-001: `c_SetColorMapFromCommData()` implemented and called — **VERIFIED**
- [ ] REQ-CM-002: Null guard on `AlienColorMap == 0` — **VERIFIED**
- [ ] REQ-CM-003: Reads current `CommData.AlienColorMap` — **VERIFIED**
- [ ] REQ-MU-001: `c_PlayAlienMusic()` implemented and called — **VERIFIED**
- [ ] REQ-MU-002: Null guard on `AlienSong == 0` — **VERIFIED**
- [ ] REQ-MU-003: Music before first AlienTalkSegue — **VERIFIED**
- [ ] REQ-SD-001: Subtitle bridges route to C — **VERIFIED**
- [ ] REQ-SD-002: `comm_ClearSubtitles` matches reference — **VERIFIED**
- [ ] REQ-SD-003: `comm_CheckSubtitles` matches reference — **VERIFIED**
- [ ] REQ-SD-004: `comm_RedrawSubtitles` matches reference — **VERIFIED**
- [ ] REQ-SD-005: Rust subtitle model test-only — **VERIFIED**
- [ ] REQ-CS-002: Production delegates to `c_SelectConversationSummary` — **VERIFIED**
- [ ] REQ-CS-003: No Rust SummaryView in production — **VERIFIED**
- [ ] REQ-RL-001: Lock released before callback — **VERIFIED**
- [ ] REQ-RL-002: Select→extract→drop→invoke — **VERIFIED**
- [ ] REQ-RL-003: No lock during callback — **VERIFIED**
- [ ] REQ-RL-004: Pre-callback work under lock — **VERIFIED**
- [ ] REQ-DC-001: Single frame iteration — **VERIFIED**
- [ ] REQ-DC-002: No response input during talking — **VERIFIED**
- [ ] REQ-DC-003: Single response input per frame — **VERIFIED**
- [ ] REQ-DC-004: Done when no responses — **VERIFIED**
- [ ] REQ-DC-005: Immediate exit on abort/load — **VERIFIED**
- [ ] REQ-TS-001: Intro sequence with real bridges — **VERIFIED**
- [ ] REQ-TS-002: Per-frame subtitle/animation/speech — **VERIFIED**
- [ ] REQ-TS-003: Talking animation start/stop — **VERIFIED**
- [ ] REQ-TS-004: Track completion — **VERIFIED**
- [ ] REQ-SM-001: Zero stale markers — **VERIFIED**
- [ ] REQ-SM-002: Exemptions documented — **VERIFIED**
- [ ] REQ-E2E-001: Full encounter parity — **VERIFIED**
- [ ] REQ-E2E-002: Conversation summary — **VERIFIED**
- [ ] REQ-E2E-003: Response selection — **VERIFIED**
- [ ] REQ-E2E-004: Replay — **VERIFIED**
- [ ] REQ-E2E-005: Dual build — **VERIFIED**
- [ ] REQ-E2E-006: 268+ test regression gate — **VERIFIED**
- [ ] REQ-E2E-007: No deadlock — **VERIFIED**

### Manual Runtime Test Results
- [ ] All P16 manual checklist items documented as PASS
- [ ] No known failures deferred

## Final Gate Decision
- [ ] **PASS**: Plan PLAN-20260325-COMMPT3 is complete.
- [ ] **FAIL**: Document failures and required remediation.

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P16a.md`

Contents:
- phase ID: PLAN-20260325-COMMPT3.P16a
- FINAL DECISION: PASS / FAIL
- requirement coverage matrix (all 35 requirements)
- any notes on exemptions or known limitations
