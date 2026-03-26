# Phase 08a: Final E2E Verification

## Phase ID
`PLAN-20260326-COMMPT2.P08a`

## Prerequisites
- Required: Phase 08 (Integration Sweep) completed
- Phase completion marker exists: `project-plans/20260311/commpt2/.completed/P08.md`
- All previous phases (P03–P08) completed and verified

## Purpose

Final end-to-end verification matching the Definition of Done from the original
comm plan P12. This is the last gate before the plan is considered complete.

---

## Structural Verification Checklist

### Code Completeness
- [ ] `rust/src/comm/hail.rs` exists with complete `hail_alien()` implementation
- [ ] `rust/src/comm/mod.rs` declares `pub mod hail`
- [ ] `rust_HailAlien` in `ffi.rs` calls `hail::hail_alien()` (no stub)
- [ ] `rust_NPCPhrase_cb` in `ffi.rs` resolves phrases and splices tracks (no stub)
- [ ] `rust_NPCPhrase_splice` in `ffi.rs` delegates to cb variant (no stub)
- [ ] All 6 `check_*_input` functions in `talk_segue.rs` call `c_GetPulsedMenuKey`
- [ ] `has_transition_anim` in `talk_segue.rs` calls C bridge (not hardcoded false)
- [ ] `c_FeedbackPlayerPhrase` in `rust_comm.c` renders text (not stub)
- [ ] `c_RefreshResponses` in `rust_comm.c` renders response list (not stub)
- [ ] `c_SelectConversationSummary` in `rust_comm.c` shows overlay (not stub)
- [ ] All resource bridge functions exist in `rust_comm.c` (P06 bridges)
- [ ] All declarations present in `rust_comm.h`

### Marker Cleanliness
- [ ] Zero `P11: Stub` markers in `rust/src/comm/ffi.rs`
- [ ] Zero `P11: Stub` markers in `sc2/src/uqm/rust_comm.c`
- [ ] Zero `TODO`/`FIXME`/`HACK`/`placeholder`/`for now` in production Rust comm code
- [ ] Zero `TODO`/`FIXME`/`Stub` in production C bridge code
- [ ] Plan markers (`@plan`, `@requirement`) present in implementation code

### Build Health
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `USE_RUST_COMM=on` build compiles and links
- [ ] `USE_RUST_COMM=off` build compiles and links
- [ ] No duplicate symbols
- [ ] No undefined symbols

## Semantic Verification Checklist

### REQ-HL: HailAlien Loop
- [ ] REQ-HL-001: `rust_HailAlien` executes full encounter loop
- [ ] REQ-HL-002: All 7 resources loaded via C bridges
- [ ] REQ-HL-003: AnimContext and TextCacheContext created and managed
- [ ] REQ-HL-004: init/post/uninit encounter funcs called in correct order
- [ ] REQ-HL-005: All resources cleaned up on exit (all paths)
- [ ] REQ-HL-006: CHECK_LOAD flag set before init_encounter_func
- [ ] REQ-HL-007: SIS frame/message/title/comm window drawn correctly

### REQ-IP: Input Polling
- [ ] REQ-IP-001: check_select_input polls real KEY_MENU_SELECT (0)
- [ ] REQ-IP-002: check_cancel_input polls real KEY_MENU_CANCEL (3)
- [ ] REQ-IP-003: check_up_input polls real KEY_MENU_UP (1)
- [ ] REQ-IP-004: check_down_input polls real KEY_MENU_DOWN (2)
- [ ] REQ-IP-005: check_left_input polls real KEY_MENU_LEFT (4)
- [ ] REQ-IP-006: check_right_input polls real KEY_MENU_RIGHT (5)
- [ ] REQ-IP-007: All use c_GetPulsedMenuKey wrapper
- [ ] REQ-IP-008: Test mode uses simulated input (not C bridge)

### REQ-NP: NPC Phrase
- [ ] REQ-NP-001: rust_NPCPhrase_cb resolves and splices with callback
- [ ] REQ-NP-002: rust_NPCPhrase_splice works without callback
- [ ] REQ-NP-003: Uses c_get_conversation_phrase for resolution
- [ ] REQ-NP-004: Updates conversation summary

### REQ-RB: Rendering Bridges
- [ ] REQ-RB-001: c_FeedbackPlayerPhrase renders player response text
- [ ] REQ-RB-002: c_RefreshResponses renders response list with highlight
- [ ] REQ-RB-003: c_SelectConversationSummary shows summary overlay
- [ ] REQ-RB-004: Uses correct graphics contexts, fonts, colors

### REQ-AT: Animation/Transition
- [ ] REQ-AT-001: has_transition_anim checks actual LOCDATA
- [ ] REQ-AT-002: Animation processing in encounter loop
- [ ] REQ-AT-003: Intro transition plays on conversation entry

### REQ-DI: DoInput Integration
- [ ] REQ-DI-001: Integrates with C DoInput framework
- [ ] REQ-DI-002: Frame-driven dispatch (batch, callback, sleep)
- [ ] REQ-DI-003: Respects CHECK_ABORT and CHECK_LOAD flags
- [ ] REQ-DI-004: Frame timing matches COMM_ANIM_RATE

### REQ-CS: C-Side Stub Completion
- [ ] REQ-CS-001: All P11 Stub markers replaced
- [ ] REQ-CS-002: Rendering delegates to C drawing logic
- [ ] REQ-CS-003: No deferred markers in production code

### REQ-E2E: End-to-End
- [ ] REQ-E2E-001: Conversation displays portrait, audio, subtitles, responses
- [ ] REQ-E2E-002: Response selection invokes callback and advances conversation
- [ ] REQ-E2E-003: Clean resource cleanup on exit
- [ ] REQ-E2E-004: All 27 race encounters work (identical to C mode)
- [ ] REQ-E2E-005: Both build modes compile, link, and run
- [ ] REQ-E2E-006: No regression in 267+ comm tests

## Verification Commands

```bash
# ===== COMPREHENSIVE TEST SUITE =====
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Count total comm tests
cargo test --workspace --all-features -- comm 2>&1 | grep "test result"
# MUST show >= 267 tests passing, 0 failures

# ===== DEFERRED IMPLEMENTATION FINAL SCAN =====
echo "=== Rust production code markers ==="
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|P11: Stub\|P11: Track" rust/src/comm/ --include="*.rs" | grep -v "test\|Test\|TEST\|#\[cfg(test)\]"

echo "=== C bridge markers ==="
grep -In "TODO\|FIXME\|HACK\|placeholder\|for now\|P11: Stub" sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h

echo "=== Hardcoded false in production input ==="
for fn in check_select_input check_cancel_input check_up_input check_down_input check_left_input check_right_input has_transition_anim; do
  echo "--- $fn ---"
  grep -A10 "fn $fn" rust/src/comm/talk_segue.rs | grep -B1 "false"
done

# ===== BUILD VERIFICATION =====
# (project-specific USE_RUST_COMM=on build)
# (project-specific USE_RUST_COMM=off build)

# ===== INTEGRATION CHAIN VERIFICATION =====
echo "=== Call chain check ==="
echo "1. C → Rust:"
grep -c "rust_HailAlien" sc2/src/uqm/comm.c
echo "2. FFI → hail:"
grep -c "hail::hail_alien\|hail_alien" rust/src/comm/ffi.rs
echo "3. hail → DoInput:"
grep -c "c_DoInput" rust/src/comm/hail.rs
echo "4. Input → C bridge:"
grep -c "c_GetPulsedMenuKey" rust/src/comm/talk_segue.rs
echo "5. NPCPhrase → resolve+splice:"
grep -c "c_get_conversation_phrase\|c_SpliceTrack" rust/src/comm/ffi.rs
echo "6. Rendering → real code:"
grep -A1 "c_FeedbackPlayerPhrase\|c_RefreshResponses\|c_SelectConversationSummary" sc2/src/uqm/rust_comm.c | grep -cv "void\|Stub\|^\-\-$"

# ===== FILE INVENTORY =====
echo "=== Comm module files ==="
ls -la rust/src/comm/*.rs | wc -l
echo "=== Expected: 20+ files (including hail.rs) ==="
```

## Pass/Fail Gate Criteria

### PASS — Plan Complete
ALL of the following must be true:
- All structural checks pass
- All semantic checks pass (every REQ-* verified)
- All 267+ comm tests pass with zero failures
- Both build modes compile and link
- Zero deferred-implementation markers in production code
- Complete integration chain verified
- `cargo fmt`, `cargo clippy`, `cargo test` all green

### FAIL — Remediation Required
If ANY of the following:
- Any REQ-* is not satisfied
- Any deferred marker remains in production code
- Any test fails
- Either build mode fails
- Integration chain is broken at any point
- Any hardcoded `false` remains in production input functions

## Remediation Process (if FAIL)

1. Identify the specific failure
2. Trace back to the responsible phase (P03–P08)
3. Fix the issue in the appropriate module
4. Re-run all verification commands
5. Re-check all affected semantic items
6. Do NOT mark as PASS until ALL checks are green

## Plan Completion

When all checks pass:

1. Create final completion marker: `project-plans/20260311/commpt2/.completed/P08a.md`
2. Update execution tracker in `00-overview.md` — all phases [OK]
3. Create summary document listing:
   - All files created/modified
   - Total LoC changed (Rust + C)
   - All tests passing
   - Both build modes verified
   - All requirements satisfied

### Completion Marker Contents
- Phase ID: `PLAN-20260326-COMMPT2.P08a`
- Timestamp
- Final test count: (number) passing, 0 failures
- Build verification: both modes green
- Deferred marker scan: 0 matches
- All 38 requirements (REQ-HL through REQ-E2E) verified
- Plan status: COMPLETE
