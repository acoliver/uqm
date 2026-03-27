# Phase 15: Integration Build Verification

## Phase ID
`PLAN-20260325-COMMPT3.P15`

## Prerequisites
- Required: Phase P14a (Summary Guard Impl Verification) completed
- Expected: all implementation phases (P03-P14) verified

## Requirements Verified
- REQ-E2E-005: Dual build compatibility
- REQ-E2E-006: Test regression gate
- REQ-E2E-007: No deadlock

## Purpose
Full cross-build verification confirming all implementation phases integrate
correctly. No new code — verification only.

## Implementation Tasks

### Files to create
- None (verification-only phase)

### Files to modify
- None

## Pseudocode Traceability
- Full encounter flow: pseudocode `005-end-to-end-integration.md` lines 01-71
- Build verification: pseudocode `005-end-to-end-integration.md` lines 82-86
- Deadlock-free criteria: pseudocode `005-end-to-end-integration.md` lines 72-81

## Traceability Marker Audits

### @plan marker audit
```bash
fail=0
for f in rust/src/comm/ffi.rs rust/src/comm/talk_segue.rs sc2/src/uqm/rust_comm.c sc2/src/uqm/comm.c sc2/src/uqm/rust_comm.h; do
  count=$(grep -c "@plan PLAN-20260325-COMMPT3" "$f" 2>/dev/null || echo 0)
  echo "$f: $count @plan markers"
  if [ "$count" -eq 0 ]; then echo "FAIL: missing @plan marker in $f"; fail=1; fi
done
[ "$fail" -eq 0 ] && echo "PASS" || echo "FAIL"
```

### @requirement marker audit
```bash
fail=0
for f in rust/src/comm/ffi.rs rust/src/comm/talk_segue.rs sc2/src/uqm/rust_comm.c sc2/src/uqm/comm.c; do
  count=$(grep -c "@requirement" "$f" 2>/dev/null || echo 0)
  echo "$f: $count @requirement markers"
  if [ "$count" -eq 0 ]; then echo "FAIL: missing @requirement marker in $f"; fail=1; fi
done
[ "$fail" -eq 0 ] && echo "PASS" || echo "FAIL"
```

### @pseudocode line-range cross-reference audit
```bash
echo "=== Pseudocode cross-references ==="
# P03/P05: 001-colormap-music-bridges
grep -n "@pseudocode 001" rust/src/comm/talk_segue.rs sc2/src/uqm/rust_comm.c || echo "WARN: P03/P05 ref missing"
# P06/P08: 002-subtitle-display-fix
grep -n "@pseudocode 002" sc2/src/uqm/comm.c sc2/src/uqm/rust_comm.c || echo "WARN: P06/P08 ref missing"
# P09/P11: 003-do-communication-rewrite
grep -n "@pseudocode 003" rust/src/comm/ffi.rs rust/src/comm/talk_segue.rs || echo "WARN: P09/P11 ref missing"
# P12/P14: 004-summary-guard-stale-markers
grep -n "@pseudocode 004" rust/src/comm/ffi.rs || echo "WARN: P12/P14 ref missing"
```

### Stub→TDD→Impl completion audit
```bash
echo "=== Stub→TDD→Impl completion audit ==="
for phase in P03 P04 P05 P06 P07 P08 P09 P10 P11 P12 P13 P14; do
  if [ -f "project-plans/20260311/commpt3/.completed/${phase}.md" ]; then
    echo "$phase: completion marker exists"
  else
    echo "FAIL: $phase completion marker missing"
  fi
done
```

## Integration Contract

### Existing Callers
- `sc2/src/uqm/comm.c:InitCommunication` → `rust_HailAlien()`
- `sc2/src/uqm/rust_comm.c:rust_do_communication_cb` → `rust_DoCommunication()`
- `sc2/src/uqm/rust_comm.c:c_ClearSubtitles` → `comm_ClearSubtitles()`
- `sc2/src/uqm/rust_comm.c:c_CheckSubtitles` → `comm_CheckSubtitles()`
- `sc2/src/uqm/rust_comm.c:c_RedrawSubtitles` → `comm_RedrawSubtitles()`
- `rust/src/comm/talk_segue.rs:set_colormap` → `c_SetColorMapFromCommData()`
- `rust/src/comm/talk_segue.rs:play_alien_music` → `c_PlayAlienMusic()`

### Code Replaced/Removed
- `talk_segue.rs`: `c_SetColorMap(null_mut)` → `c_SetColorMapFromCommData()`
- `talk_segue.rs`: `c_PlayMusic(null_mut, 1, 1)` → `c_PlayAlienMusic()`
- `rust_comm.c`: subtitle routing from `rust_*` → `comm_*`
- `ffi.rs`: double `player_response_input` → single-pass `do_communication`
- `ffi.rs`: convoluted lock-drop → clean match-based lifecycle
- `ffi.rs`: Rust SummaryView in production → `c_SelectConversationSummary`

## Verification Commands

```bash
# Full quality gates
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features 2>&1 | tail -10

# Test count
cd rust && cargo test --workspace --all-features -- comm 2>&1 | grep "test result"

# Lock discipline check
grep -c "COMM_STATE.write" rust/src/comm/ffi.rs
grep -B3 -A3 "callback_fn\|drop(state)" rust/src/comm/ffi.rs
```

## Structural Verification Checklist
- [ ] Rust build passes: fmt, clippy, test
- [ ] C build with `USE_RUST_COMM=on` compiles and links
- [ ] C build with `USE_RUST_COMM=off` compiles and links
- [ ] 268+ comm tests pass, 0 failures
- [ ] Zero deferred markers in production (per-match classification)
- [ ] All bridge functions exist and are called
- [ ] Completion markers exist for P03-P14 (12 phases)
- [ ] `@plan` markers in all 5 modified files
- [ ] `@requirement` markers in all 4 implementation files
- [ ] `@pseudocode` cross-references present

## Semantic Verification Checklist (Mandatory)
- [ ] Colormap bridge reads from CommData (not null)
- [ ] Music bridge reads from CommData (not null)
- [ ] Subtitle bridge routes to C drawing (not back to Rust)
- [ ] DoCommunication single-pass (no double input)
- [ ] Lock dropped before all C callbacks
- [ ] Summary production path delegates to C
- [ ] No stale markers in production
- [ ] No undefined symbols in either build mode

## Deferred Implementation Detection (Mandatory)

```bash
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
grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|not yet\|stub' sc2/src/uqm/rust_comm.c && fail=1 || true
awk '/#ifdef USE_RUST_COMM/,/#endif/' sc2/src/uqm/comm.c | grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|not yet' && fail=1 || true
[ "$fail" -eq 0 ] && echo "PASS" || echo "FAIL"
```

## Success Criteria
- [ ] Both build modes compile and link
- [ ] All tests pass
- [ ] Zero deferred markers
- [ ] All traceability audits pass

## Failure Recovery
- No code changes in this phase — fix regressions in the responsible phase

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P15.md`
