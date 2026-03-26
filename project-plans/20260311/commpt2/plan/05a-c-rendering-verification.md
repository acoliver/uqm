# Phase 05a: C Rendering Bridges Verification

## Phase ID
`PLAN-20260326-COMMPT2.P05a`

## Prerequisites
- Required: Phase 05 (C Rendering Bridges) completed
- Phase completion marker exists: `project-plans/20260311/commpt2/.completed/P05.md`

## Structural Verification Checklist

- [ ] `c_FeedbackPlayerPhrase` body contains drawing calls (not `(void)text`)
- [ ] `c_RefreshResponses` body contains drawing calls (not `(void)top; (void)num_responses; (void)cur_response`)
- [ ] `c_SelectConversationSummary` body contains drawing calls (not empty)
- [ ] No `P11: Stub` markers remain in the three functions
- [ ] `rust_comm.h` declarations are consistent with implementations
- [ ] `@plan PLAN-20260326-COMMPT2.P05` markers present
- [ ] `@requirement REQ-RB-*` markers present
- [ ] Both `USE_RUST_COMM=on` and `USE_RUST_COMM=off` compile

## Semantic Verification Checklist

- [ ] `c_FeedbackPlayerPhrase` uses SetContext/SetContextFont for proper rendering state
- [ ] `c_FeedbackPlayerPhrase` clears the text area before drawing new text
- [ ] `c_RefreshResponses` iterates through response entries correctly
- [ ] `c_RefreshResponses` uses distinct colors for current vs. non-current responses
- [ ] `c_SelectConversationSummary` uses BatchGraphics/UnbatchGraphics
- [ ] All three functions save and restore graphics context (no state corruption)
- [ ] Functions handle edge cases: no responses, empty text, etc.
- [ ] Integration: Rust code in talk_segue.rs calls these functions via the c_bridge extern block

## Behavioral Parity Verification Checklist

- [ ] `c_RefreshResponses` selection highlight uses correct colors matching C (COMM_PLAYER_TEXT_NORMAL_COLOR vs. COMM_PLAYER_TEXT_HIGHLIGHT_COLOR)
- [ ] `c_RefreshResponses` sets up proper clipping rect for the response area (comm window bounds)
- [ ] `c_RefreshResponses` handles top-of-window offset correctly (responses may scroll)
- [ ] `c_SelectConversationSummary` invocation path matches C (called from summary view flow)
- [ ] `c_FeedbackPlayerPhrase` text position matches C subtitle area placement
- [ ] Drawing text uses the correct font (PlayerFont for responses, AlienFont for NPC text)

## Verification Commands

```bash
# C build verification
# (project-specific build with USE_RUST_COMM=on)
# (project-specific build with USE_RUST_COMM=off)

# No stubs remain
grep -c "P11: Stub" sc2/src/uqm/rust_comm.c
# Expected: 0

# Verify real implementation exists
grep -c "SetContext\|DrawText\|DrawFilledRectangle\|font_DrawText" sc2/src/uqm/rust_comm.c
# Expected: > 0 (actual drawing calls in the rendering functions)

# Rust tests still pass
cargo test --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Deferred implementation check
grep -n "Stub\|TODO\|FIXME\|placeholder" sc2/src/uqm/rust_comm.c
# Expected: 0 matches in rendering functions
```

## Pass/Fail Gate Criteria

**PASS if**:
- All structural checks pass
- All semantic checks pass
- Both build modes compile and link
- No `P11: Stub` markers remain
- Rust tests still pass (no regressions)

**FAIL if**:
- Any rendering function is still a stub (no drawing calls)
- Build fails in either mode
- Graphics context is corrupted (save/restore missing)
- Any test regression
