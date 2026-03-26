# Phase 03a: Input Bridge Verification

## Phase ID
`PLAN-20260326-COMMPT2.P03a`

## Prerequisites
- Required: Phase 03 (Input Bridge) completed
- Phase completion marker exists: `project-plans/20260311/commpt2/.completed/P03.md`

## Structural Verification Checklist

- [ ] `c_GetPulsedMenuKey` is in the `c_bridge` extern block in `talk_segue.rs`
- [ ] `c_HasTransitionAnim` is in the `c_bridge` extern block (or equivalent C bridge exists)
- [ ] Key constants are defined: UP=5, DOWN=6, LEFT=7, RIGHT=8, SELECT=9, CANCEL=10
- [ ] `check_select_input` `#[cfg(not(test))]` body calls `c_GetPulsedMenuKey(9)`
- [ ] `check_cancel_input` `#[cfg(not(test))]` body calls `c_GetPulsedMenuKey(10)`
- [ ] `check_up_input` `#[cfg(not(test))]` body calls `c_GetPulsedMenuKey(5)`
- [ ] `check_down_input` `#[cfg(not(test))]` body calls `c_GetPulsedMenuKey(6)`
- [ ] `check_left_input` `#[cfg(not(test))]` body calls `c_GetPulsedMenuKey(7)`
- [ ] `check_right_input` `#[cfg(not(test))]` body calls `c_GetPulsedMenuKey(8)`
- [ ] `has_transition_anim` `#[cfg(not(test))]` body calls C bridge (not `false`)
- [ ] All `#[cfg(test)]` bodies remain functional
- [ ] `@plan PLAN-20260326-COMMPT2.P03` markers present
- [ ] `@requirement REQ-IP-*` markers present on relevant functions

## Semantic Verification Checklist

- [ ] No production input function returns hardcoded `false`
- [ ] Input functions are called from `player_response_input()` (line ~362+)
- [ ] Input functions are called from `do_talk_segue()` (line ~167+)
- [ ] Key index values verified against C `controls.h`
- [ ] `has_transition_anim` is called from `alien_talk_segue()` (line ~220)
- [ ] Test suite exercises input paths (confirm existing tests still work)
- [ ] No new unsafe blocks without `// SAFETY:` documentation

## Verification Commands

```bash
# All tests pass
cargo test --workspace --all-features

# Lint gates
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Verify wiring
grep -c "c_GetPulsedMenuKey" rust/src/comm/talk_segue.rs
# Expected: at least 7 (1 declaration + 6 calls)

# Verify no remaining hardcoded false in production input
grep -B2 -A4 "cfg(not(test))" rust/src/comm/talk_segue.rs | grep -c "false"
# Should be 0 for input functions (won_last_battle may still have test false)

# Deferred implementation check
grep -n "P11: Stub\|hardcoded\|placeholder\|for now" rust/src/comm/talk_segue.rs
# Must not match in check_*_input or has_transition_anim functions

# C build
# (project-specific build with USE_RUST_COMM=on)
```

## Pass/Fail Gate Criteria

**PASS if**:
- All structural checks pass
- All semantic checks pass
- All 267+ comm tests pass
- `cargo fmt`, `cargo clippy`, `cargo test` all green
- No hardcoded `false` in production input paths
- Both build modes compile

**FAIL if**:
- Any input function still returns `false` in production
- Key constants don't match C values
- Any existing test breaks
- Lint or format errors
- `has_transition_anim` still returns `false` in production
