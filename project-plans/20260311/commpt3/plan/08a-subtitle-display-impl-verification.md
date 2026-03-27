# Phase 08a: Subtitle Display Bridge Implementation Verification

## Phase ID
`PLAN-20260325-COMMPT3.P08a`

## Prerequisites
- Required: Phase P08 completed
- Expected artifacts: Implemented subtitle functions in `comm.c`

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify circular routing still broken
grep -n "rust_ClearSubtitles\|rust_CheckSubtitles\|rust_RedrawSubtitles" sc2/src/uqm/rust_comm.c | grep -v "extern\|declare\|//\|proto"

# Verify implementations have substance
grep -A15 "comm_ClearSubtitles" sc2/src/uqm/comm.c | grep "clear_subtitles.*TRUE"
grep -A15 "comm_ClearSubtitles" sc2/src/uqm/comm.c | grep "last_subtitle.*NULL"
grep -A30 "comm_CheckSubtitles" sc2/src/uqm/comm.c | grep "GetTrackSubtitle"
grep -A30 "comm_CheckSubtitles" sc2/src/uqm/comm.c | grep "CommData.AlienText"
grep -A30 "comm_CheckSubtitles" sc2/src/uqm/comm.c | grep "log_Warning\|log_add"
grep -A15 "comm_RedrawSubtitles" sc2/src/uqm/comm.c | grep "optSubtitles"
grep -A15 "comm_RedrawSubtitles" sc2/src/uqm/comm.c | grep "add_text"
```

## Structural Verification Checklist
- [ ] Three `comm_*` functions have implemented bodies (not stubs)
- [ ] No placeholder markers in implementations
- [ ] Plan/requirement markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `comm_ClearSubtitles` matches comm.c:1661-1667 behavior exactly
- [ ] `comm_CheckSubtitles` matches comm.c:1670-1701 behavior exactly
- [ ] `comm_RedrawSubtitles` matches comm.c:1646-1657 behavior exactly
- [ ] All P07 TDD tests now PASS
- [ ] All 268+ comm tests pass
- [ ] Both build modes compile

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: Breaking `comm_ClearSubtitles` (removing `last_subtitle = NULL`) causes
  the TDD clear-vars test to fail
- [ ] **Confirmed**: Breaking `comm_CheckSubtitles` (removing `GetTrackSubtitle` call) causes
  the TDD check test to fail
- [ ] **Confirmed**: Breaking `comm_RedrawSubtitles` (removing `optSubtitles` guard) causes
  the TDD redraw-guard test to fail

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P08a.md`
