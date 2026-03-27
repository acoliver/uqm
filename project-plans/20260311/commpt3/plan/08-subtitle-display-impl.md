# Phase 08: Subtitle Display Bridge — Implementation

## Phase ID
`PLAN-20260325-COMMPT3.P08`

## Prerequisites
- Required: Phase P07a (Subtitle Display TDD Verification) completed
- Expected: tests written, expected failures documented against stubs

## Requirements Implemented

### REQ-SD-001: C-Side Subtitle Rendering
The `c_ClearSubtitles`, `c_CheckSubtitles`, and `c_RedrawSubtitles` bridge
functions in `rust_comm.c` SHALL delegate to C-side drawing primitives; they
SHALL NOT route back to Rust FFI functions.

### REQ-SD-002: ClearSubtitles Behavior
WHEN `c_ClearSubtitles` is called, it SHALL set `clear_subtitles = TRUE`,
`last_subtitle = NULL`, `SubtitleText.pStr = NULL`, `SubtitleText.CharCount = 0`.

### REQ-SD-003: CheckSubtitles Behavior
WHEN `c_CheckSubtitles` is called, it SHALL read the current subtitle from
`GetTrackSubtitle()`, compare against `SubtitleText`, and update accordingly.

### REQ-SD-004: RedrawSubtitles Behavior
WHEN `c_RedrawSubtitles` is called, it SHALL draw `SubtitleText` using
`add_text(1, &t)` if `optSubtitles` is true and `SubtitleText.pStr` is non-null.

### REQ-SD-005: Rust SubtitleDisplay Test-Only
The Rust `SubtitleDisplay` model SHALL NOT independently render subtitle text.
Rust FFI subtitle functions remain for test use only.

## Implementation Tasks

### Files to modify
- `sc2/src/uqm/comm.c`
  - Implement `comm_ClearSubtitles()`: set `clear_subtitles=TRUE`, `last_subtitle=NULL`,
    `SubtitleText.pStr=NULL`, `SubtitleText.CharCount=0`
  - Implement `comm_CheckSubtitles()`: call `GetTrackSubtitle()`, compare to current,
    update `SubtitleText` fields from `CommData` if changed, log_Warning for out-of-sync
  - Implement `comm_RedrawSubtitles()`: if `optSubtitles && SubtitleText.pStr`,
    copy `SubtitleText` to local, call `add_text(1, &t)`
  - marker: `@plan PLAN-20260325-COMMPT3.P08`
  - marker: `@requirement REQ-SD-002, REQ-SD-003, REQ-SD-004`
  - marker: `@pseudocode 002-subtitle-display-fix lines 01-37`

### Files to create
- None

## Pseudocode Traceability
- `comm_ClearSubtitles`: pseudocode `002-subtitle-display-fix.md` lines 01-06
  - Contract: REQ-SD-002 (clear_subtitles=TRUE, last_subtitle=NULL, pStr=NULL, CharCount=0)
- `comm_CheckSubtitles`: pseudocode `002-subtitle-display-fix.md` lines 07-29
  - Contract: REQ-SD-003 (GetTrackSubtitle comparison, SubtitleText update, log_Warning)
- `comm_RedrawSubtitles`: pseudocode `002-subtitle-display-fix.md` lines 30-37
  - Contract: REQ-SD-004 (optSubtitles guard, add_text(1, &t) rendering)

## Traceability Markers (in code)
```c
/* @plan PLAN-20260325-COMMPT3.P08 */
/* @requirement REQ-SD-002, REQ-SD-003, REQ-SD-004 */
/* @pseudocode 002-subtitle-display-fix lines 01-37 */
```

## Verification Commands

```bash
# Full quality gates
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify implementations exist and are non-empty
grep -A15 "comm_ClearSubtitles" sc2/src/uqm/comm.c | head -20
grep -A30 "comm_CheckSubtitles" sc2/src/uqm/comm.c | head -35
grep -A15 "comm_RedrawSubtitles" sc2/src/uqm/comm.c | head -20

# Verify key behaviors
grep -A15 "comm_ClearSubtitles" sc2/src/uqm/comm.c | grep "clear_subtitles.*TRUE"
grep -A15 "comm_ClearSubtitles" sc2/src/uqm/comm.c | grep "last_subtitle.*NULL"
grep -A30 "comm_CheckSubtitles" sc2/src/uqm/comm.c | grep "GetTrackSubtitle"
grep -A30 "comm_CheckSubtitles" sc2/src/uqm/comm.c | grep "CommData.AlienText"
grep -A15 "comm_RedrawSubtitles" sc2/src/uqm/comm.c | grep "optSubtitles"
grep -A15 "comm_RedrawSubtitles" sc2/src/uqm/comm.c | grep "add_text"
```

## Structural Verification Checklist
- [ ] `comm_ClearSubtitles()` has implemented body (not stub)
- [ ] `comm_CheckSubtitles()` has implemented body (not stub)
- [ ] `comm_RedrawSubtitles()` has implemented body (not stub)
- [ ] No `todo!()`, `unimplemented!()`, or placeholder markers in implementations
- [ ] Plan/requirement/pseudocode markers present

## Semantic Verification Checklist (Mandatory)

### comm_ClearSubtitles (matches comm.c:1661-1667)
- [ ] Sets `clear_subtitles = TRUE`
- [ ] Sets `last_subtitle = NULL`
- [ ] Sets `SubtitleText.pStr = NULL`
- [ ] Sets `SubtitleText.CharCount = 0`

### comm_CheckSubtitles (matches comm.c:1670-1701)
- [ ] Calls `GetTrackSubtitle()` for current subtitle
- [ ] Reads `CommData.AlienTextBaseline` and `CommData.AlienTextAlign`
- [ ] Compares pStr, baseline, align to detect change
- [ ] Sets `clear_subtitles = TRUE` on change
- [ ] Includes `log_add(log_Warning, ...)` for out-of-sync
- [ ] Sets `CharCount = ~0` for non-null pStr, `0` for null

### comm_RedrawSubtitles (matches comm.c:1646-1657)
- [ ] Early return if `!optSubtitles`
- [ ] Early return if `SubtitleText.pStr == NULL`
- [ ] Copies `SubtitleText` to local before `add_text`
- [ ] Calls `add_text(1, &t)`

### Integration
- [ ] All P07 TDD behavioral tests now PASS (previously failing against stubs)
- [ ] Rust FFI subtitle exports unchanged (test use)
- [ ] All 268+ comm tests pass
- [ ] Both `USE_RUST_COMM=on` and `=off` builds compile

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Negative proof — clear**: Temporarily remove `last_subtitle = NULL` from
  `comm_ClearSubtitles` → test 2 (which greps for `last_subtitle.*NULL`) fails.
  Revert after confirming.
- [ ] **Negative proof — check**: Temporarily remove `GetTrackSubtitle` call from
  `comm_CheckSubtitles` → test 3 fails. Revert after confirming.
- [ ] **Negative proof — redraw**: Temporarily remove `optSubtitles` check from
  `comm_RedrawSubtitles` → test 5 fails. Revert after confirming.

## Deferred Implementation Detection (Mandatory)

```bash
echo "=== comm.c USE_RUST_COMM block ==="
awk '/#ifdef USE_RUST_COMM/,/#endif/' sc2/src/uqm/comm.c | grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|not yet' && echo "FAIL" || echo "CLEAN"

echo "=== rust_comm.c ==="
grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|not yet' sc2/src/uqm/rust_comm.c && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] All P07 TDD tests now pass
- [ ] Subtitle functions match C reference behavior (comm.c:1646-1701)
- [ ] All verification commands pass

## Failure Recovery
- rollback: `git restore sc2/src/uqm/comm.c sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h`

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P08.md`

Contents:
- phase ID: PLAN-20260325-COMMPT3.P08
- files changed: `comm.c`
- tests that now pass (were failing in P07)
- negative-proof results
- verification outputs
