# Phase 00: Requirements Lock

## Phase ID
`PLAN-20260325-COMMPT3.P00`

## Purpose
Confirm that the EARS-format requirements in `requirements.md` are complete,
testable, and accurately reflect the runtime parity gaps discovered in the
codebase. Lock requirements so no changes occur after this phase.

## Prerequisites
- Plan directory structure created
- `requirements.md`, `specification.md`, `analysis/domain-model.md` exist
- Access to `rust/src/comm/` and `sc2/src/uqm/` source trees

## Tasks

### 1. Validate each requirement against source code

For each requirement family, verify the defect exists at the stated location:

| Family | Validation Command |
|---|---|
| CM (Colormap) | `grep -n "null_mut" rust/src/comm/talk_segue.rs` — confirm line ~1003 passes null to `c_SetColorMap` |
| MU (Music) | `grep -n "null_mut" rust/src/comm/talk_segue.rs` — confirm line ~945 passes null to `c_PlayMusic` |
| SD (Subtitle) | `grep -n "rust_ClearSubtitles\|rust_CheckSubtitles\|rust_RedrawSubtitles" sc2/src/uqm/rust_comm.c` — confirm circular routing at lines 562-576 |
| CS (Summary) | `grep -n "not yet wired\|not yet implemented" rust/src/comm/ffi.rs` — confirm stale comments at lines 879-881 |
| RL (Lock) | Read `ffi.rs:715-752` — confirm double `player_response_input` call pattern |
| SM (Markers) | `grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|not yet" rust/src/comm/*.rs \| grep -v cfg(test)` — count production markers |
| DC (State machine) | Read `ffi.rs:721-752` — confirm `do_communication` followed by second `player_response_input` |
| E2E | `cd rust && cargo test --lib -- comm` — confirm 268 tests pass |

### 2. Confirm requirement testability

Each requirement must have at least one verifiable criterion:
- Build verification (compiles + links)
- Unit test (Rust `#[cfg(test)]`)
- Automated sweep (grep)
- Runtime manual test (documented observation)

### 3. Confirm no missing requirements

Cross-check against:
- The 5 gaps listed in `specification.md` §1
- All stale markers found by grep
- Integration touchpoints in `domain-model.md` §4

### 4. Freeze requirements

After this phase, `requirements.md` is locked. No new requirements may be
added without restarting from P00.

## Verification Commands

```bash
# Verify all defect locations
grep -n "null_mut" rust/src/comm/talk_segue.rs | grep -v test
grep -n "rust_ClearSubtitles\|rust_CheckSubtitles\|rust_RedrawSubtitles" sc2/src/uqm/rust_comm.c
grep -n "not yet wired\|not yet implemented\|for now" rust/src/comm/ffi.rs rust/src/comm/talk_segue.rs
cd rust && cargo test --lib -- comm 2>&1 | tail -5
```

## Success Criteria
- [ ] Every requirement in `requirements.md` maps to a specific code location verified by grep/read
- [ ] Every requirement has a testable verification criterion
- [ ] No missing gaps discovered that are not covered by existing requirements
- [ ] Requirements document is frozen (no changes after this phase)

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P00.md`
