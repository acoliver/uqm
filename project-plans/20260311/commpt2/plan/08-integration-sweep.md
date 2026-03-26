# Phase 08: Integration Sweep

## Phase ID
`PLAN-20260326-COMMPT2.P08`

## Prerequisites
- Required: Phase 07a (HailAlien Verification) completed
- All previous phases (P03–P07) completed and verified
- Input bridges wired (P03)
- NPCPhrase implemented (P04)
- C rendering bridges implemented (P05)
- Resource bridges created (P06)
- HailAlien orchestration implemented (P07)
- All existing comm tests pass
- Both build modes compile

## Requirements Implemented (Expanded)

### REQ-CS-003: No production markers remain
**Requirement text**: No `TODO`, `FIXME`, `Stub`, `placeholder`, or `for now` markers SHALL remain in production code paths after completion.

Behavior contract:
- GIVEN: All implementation phases (P03–P07) are complete
- WHEN: A comprehensive grep is run across all comm-related files
- THEN: Zero deferred-implementation markers are found in production code paths (test comments excluded)

Why it matters:
- Deferred markers indicate incomplete work — the entire point of this plan is to close ALL gaps

### REQ-E2E-001: Encounter displays correctly
**Requirement text**: With `USE_RUST_COMM=on`, entering a conversation SHALL display the alien portrait, play speech audio, show subtitles, and present response options.

Behavior contract:
- GIVEN: The game is built with `USE_RUST_COMM=on`
- WHEN: The player enters a conversation with an alien
- THEN: The alien portrait is visible, speech audio plays, subtitles appear below the portrait, and response options are displayed in the comm window

Why it matters:
- This is the primary user-facing behavior — conversations must work

### REQ-E2E-002: Response selection advances conversation
**Requirement text**: Selecting a response SHALL invoke the response callback and advance the conversation.

Behavior contract:
- GIVEN: The player is viewing response options
- WHEN: The player navigates to a response and presses select
- THEN: The selected response's callback is invoked, the conversation advances to the next NPC phrase

Why it matters:
- Without response callbacks, conversations are stuck on the first NPC phrase

### REQ-E2E-003: Clean resource cleanup
**Requirement text**: The conversation SHALL complete normally with proper resource cleanup.

Behavior contract:
- GIVEN: A conversation has been completed or aborted
- WHEN: The encounter exits
- THEN: All loaded resources (fonts, drawables, colormaps, music, string tables, contexts) are freed

Why it matters:
- Resource leaks cause crashes after multiple conversations

### REQ-E2E-004: All race encounters work
**Requirement text**: All 27 race encounters SHALL work identically to C-only mode.

Behavior contract:
- GIVEN: `USE_RUST_COMM=on` and the player encounters any of 27 alien races
- WHEN: The conversation plays through
- THEN: The conversation flow, options, and outcomes are identical to C-only mode

Why it matters:
- Each race script exercises different code paths — all must work

### REQ-E2E-005: Both build modes work
**Requirement text**: Both `USE_RUST_COMM=on` and `USE_RUST_COMM=off` builds SHALL compile, link, and run correctly.

Behavior contract:
- GIVEN: The codebase after all phases
- WHEN: Compiled with either build flag
- THEN: Compilation succeeds, linking succeeds, and the game runs correctly

Why it matters:
- The C path must remain functional as fallback

### REQ-E2E-006: No test regression
**Requirement text**: No regression in the existing 267+ comm tests.

Behavior contract:
- GIVEN: 267+ comm tests existed before this plan
- WHEN: All tests are run after all phases
- THEN: All 267+ tests pass (no regressions, possibly more tests added)

Why it matters:
- Existing tests validate the comm subsystem's correctness

## Implementation Tasks

### Deferred Implementation Sweep

#### Rust source files
- **Scan ALL comm module files** for deferred markers:
  - `rust/src/comm/ffi.rs`
  - `rust/src/comm/talk_segue.rs`
  - `rust/src/comm/hail.rs`
  - `rust/src/comm/state.rs`
  - `rust/src/comm/animation.rs`
  - `rust/src/comm/encounter.rs`
  - `rust/src/comm/response.rs`
  - `rust/src/comm/track.rs`
  - `rust/src/comm/subtitle.rs`
  - `rust/src/comm/summary.rs`
  - `rust/src/comm/oscilloscope.rs`
  - `rust/src/comm/speech_graphics.rs`
  - `rust/src/comm/response_ui.rs`
  - `rust/src/comm/subtitle_display.rs`
  - `rust/src/comm/glue.rs`
  - `rust/src/comm/locdata.rs`
  - `rust/src/comm/phrase_state.rs`
  - `rust/src/comm/segue.rs`
  - `rust/src/comm/types.rs`
  - `rust/src/comm/mod.rs`
  - marker: `@plan PLAN-20260326-COMMPT2.P08`
  - marker: `@requirement REQ-CS-003`

#### C source files
- **Scan C bridge and comm-adjacent files** for deferred markers:
  - `sc2/src/uqm/rust_comm.c`
  - `sc2/src/uqm/rust_comm.h`
  - `sc2/src/uqm/commglue.h`
  - marker: `@plan PLAN-20260326-COMMPT2.P08`
  - marker: `@requirement REQ-CS-001`

#### Resolution of any remaining markers
- For each marker found in production code:
  - If it's in `#[cfg(test)]` or a comment about future enhancement: acceptable
  - If it's in production code path: MUST be resolved in this phase
  - If it's a "P11: Stub" marker: MUST be replaced with implementation or removed if function is now implemented
  - marker: `@plan PLAN-20260326-COMMPT2.P08`

### Files to modify
- Any files with remaining deferred markers (identified by grep scan)
  - Remove or resolve each marker
  - marker: `@plan PLAN-20260326-COMMPT2.P08`
  - marker: `@requirement REQ-CS-003`

### Pseudocode traceability
- Uses pseudocode lines: G01–G18 (Integration Sweep)

## Verification Commands

```bash
# ===== Deferred Implementation Detection =====
# CRITICAL: This must find zero matches in production code

# Rust comm files — comprehensive pattern list
grep -RIn 'TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|P11: Stub\|P11: Track\|unimplemented!\|panic!("stub")\|todo!\|TBD\|XXX\|stubbed\|not yet implemented' rust/src/comm/
# Must be zero in production code (test-only markers acceptable)

# C bridge files — expanded scope to include commglue.h
grep -In 'TODO\|FIXME\|HACK\|placeholder\|for now\|P11: Stub\|P11: Track\|TBD\|XXX\|stubbed\|not yet implemented' sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h sc2/src/uqm/commglue.h
# Must be zero

# ===== Test Verification =====
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features 2>&1 | tail -20

# Count comm tests
cargo test --workspace --all-features -- comm 2>&1 | grep "test result"
# Must show 267+ tests passing

# ===== Build Verification =====
# USE_RUST_COMM=on build
# (project-specific build command)

# USE_RUST_COMM=off build
# (project-specific build command)

# ===== Symbol Verification =====
# No duplicate symbols
# No undefined symbols
# (verified by successful build and link in both modes)

# ===== Integration Path Verification =====
# Verify the complete call chain exists:
grep "rust_HailAlien" sc2/src/uqm/comm.c          # C calls Rust
grep "hail_alien" rust/src/comm/ffi.rs              # FFI calls hail module
grep "pub fn hail_alien" rust/src/comm/hail.rs      # hail module exists
grep "c_DoInput" rust/src/comm/hail.rs              # hail calls DoInput
grep "c_GetPulsedMenuKey" rust/src/comm/talk_segue.rs  # input is wired
grep "c_get_conversation_phrase" rust/src/comm/ffi.rs   # phrase resolution wired
grep "c_FeedbackPlayerPhrase" sc2/src/uqm/rust_comm.c  # rendering not a stub
```

## Structural Verification Checklist
- [ ] All Rust comm module files scanned for deferred markers
- [ ] All C bridge files scanned for deferred markers
- [ ] Zero deferred markers remain in production code
- [ ] All 267+ comm tests pass
- [ ] Both `USE_RUST_COMM=on` and `USE_RUST_COMM=off` compile and link
- [ ] No duplicate symbols at link time
- [ ] No undefined symbols at link time
- [ ] `@plan` markers present on any changes made
- [ ] `@requirement` markers present for resolved markers

## Semantic Verification Checklist (Mandatory)
- [ ] Complete call chain verified: comm.c → rust_HailAlien → hail_alien → c_DoInput
- [ ] Input bridge verified: check_*_input → c_GetPulsedMenuKey (not false)
- [ ] NPC phrase verified: rust_NPCPhrase_cb → c_get_conversation_phrase + c_SpliceTrack
- [ ] Rendering verified: c_FeedbackPlayerPhrase/c_RefreshResponses/c_SelectConversationSummary are not stubs
- [ ] Resource lifecycle verified: all 7 resources loaded and freed
- [ ] has_transition_anim verified: calls C bridge (not false)
- [ ] Test paths still use simulated input (no C bridge in tests)
- [ ] No regressions in any module

## Deferred Implementation Detection (Mandatory)

```bash
# FINAL comprehensive check — this is the plan's last line of defense
# Scan ALL comm-related files with expanded patterns:
echo "=== Rust comm modules ==="
grep -RIn 'TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|P11:\|stub\|unimplemented!\|panic!("stub")\|todo!\|TBD\|XXX\|stubbed\|not yet implemented' rust/src/comm/ --include="*.rs" | grep -v "#\[cfg(test)\]" | grep -v "// Test"

echo "=== C bridge files (including commglue.h) ==="
grep -In 'TODO\|FIXME\|HACK\|placeholder\|for now\|P11:\|Stub\|TBD\|XXX\|stubbed\|not yet implemented' sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h sc2/src/uqm/commglue.h

echo "=== Hardcoded false in production ==="
grep -B2 -A2 "cfg(not(test))" rust/src/comm/talk_segue.rs | grep "false"
# Should only match won_last_battle test helper, not input functions
```

## Success Criteria
- [ ] Zero deferred-implementation markers in production comm code
- [ ] All 267+ comm tests pass
- [ ] Both build modes compile and link
- [ ] No duplicate/undefined symbols
- [ ] Complete integration chain verified
- [ ] All requirements (REQ-HL through REQ-E2E) are satisfied

### Behavioral Parity Checks
- [ ] Alternate song fallback: if `LDASF_USE_ALTERNATE` flag set and `AlienAltSongRes` non-zero, try alt song first, fall back to primary on failure
- [ ] Starbase title/message branch: when `GLOBAL_FLAGS_AND_DATA == (BYTE)~0 && STARBASE_AVAILABLE`, use "Starbase Commander" / "Starbase" instead of default planet name
- [ ] CHECK_ABORT/CHECK_LOAD post-encounter skip: `post_encounter_func` is NOT called when either flag is set in `CurrentActivity`
- [ ] Replay-mode path: when `num_responses == 0` and not aborting, FadeMusic and DoLastReplay allow player to review alien's last phrases before exit

## Failure Recovery
- If deferred markers found: resolve each one individually, re-run verification
- If tests fail: diagnose and fix; do not suppress failures
- If build fails: check for missing/duplicate symbols, fix in affected files
- If integration chain broken: trace from comm.c through each layer to find the break
- Blocking: all previous phases must be verified before this phase can pass

## Phase Completion Marker
Create: `project-plans/20260311/commpt2/.completed/P08.md`

Contents:
- Phase ID: `PLAN-20260326-COMMPT2.P08`
- Timestamp
- Files changed: (list any files modified to resolve markers)
- Tests added/updated: (list)
- Verification outputs: full test run results, deferred marker scan results
- Semantic verification summary
- Final E2E assessment
