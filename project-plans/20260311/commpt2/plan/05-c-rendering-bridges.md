# Phase 05: C Rendering Bridges

## Phase ID
`PLAN-20260326-COMMPT2.P05`

## Prerequisites
- Required: Phase 04a (NPC Phrase Verification) completed
- Rendering stubs exist in rust_comm.c (lines 665, 673, 684)
- C rendering functions (text drawing, filled rectangles) are available in the codebase
- Comm-related C drawing functions exist in comm.c (guarded behind `#ifndef USE_RUST_COMM`)
- All existing comm tests pass

## Requirements Implemented (Expanded)

### REQ-RB-001: c_FeedbackPlayerPhrase renders player text
**Requirement text**: `c_FeedbackPlayerPhrase(text)` SHALL render the player's selected response text in the subtitle area by calling the appropriate C drawing functions.

Behavior contract:
- GIVEN: A player has selected a response during an active encounter
- WHEN: `c_FeedbackPlayerPhrase(text)` is called from Rust via FFI
- THEN: The player's response text is displayed in the comm window subtitle area, replacing any previous text

Why it matters:
- Without this, the player's selected response is never shown on screen — the dialogue appears one-sided

### REQ-RB-002: c_RefreshResponses renders response list
**Requirement text**: `c_RefreshResponses(top, num_responses, cur_response)` SHALL render the response list in the SIS comm window using C's text rendering.

Behavior contract:
- GIVEN: The NPC has finished speaking and the response list is being shown
- WHEN: `c_RefreshResponses(top, num_responses, cur_response)` is called
- THEN: The visible response options are rendered in the comm window with the current selection highlighted

Why it matters:
- Without this, the player sees a blank response area and cannot choose what to say

### REQ-RB-003: c_SelectConversationSummary shows overlay
**Requirement text**: `c_SelectConversationSummary()` SHALL display the conversation summary overlay using C's drawing functions.

Behavior contract:
- GIVEN: The player presses the cancel key during a conversation
- WHEN: `c_SelectConversationSummary()` is called
- THEN: A summary of the conversation history is displayed as an overlay

Why it matters:
- The conversation summary is a key UX feature for reviewing what was said

### REQ-RB-004: Rendering uses correct graphics state
**Requirement text**: The rendering bridges SHALL use the same graphics contexts, fonts, and colors as C HailAlien sets up.

Behavior contract:
- GIVEN: Graphics contexts, fonts, and colors were set up by HailAlien initialization
- WHEN: Any rendering bridge function is called
- THEN: It uses the established SpaceContext, PlayerFont, and standard comm colors

Why it matters:
- Mismatched rendering state would cause visual corruption or crashes

### REQ-CS-001: P11 Stub markers removed
**Requirement text**: All `P11: Stub` markers in rust_comm.c SHALL be replaced with working implementations.

Behavior contract:
- GIVEN: rust_comm.c contains `P11: Stub` comments in rendering functions
- WHEN: Phase 05 is complete
- THEN: No `P11: Stub` markers remain in `c_FeedbackPlayerPhrase`, `c_RefreshResponses`, or `c_SelectConversationSummary`

Why it matters:
- Stub markers indicate incomplete code that must be resolved

### REQ-CS-002: Rendering delegates to C original logic
**Requirement text**: `c_FeedbackPlayerPhrase`, `c_RefreshResponses`, `c_SelectConversationSummary` SHALL delegate to the original C rendering logic (either by calling the C functions directly or by reimplementing the draw commands).

Behavior contract:
- GIVEN: The original C rendering code exists in comm.c behind `#ifndef USE_RUST_COMM` guards
- WHEN: The rendering bridge stubs are implemented
- THEN: They perform equivalent rendering operations using the same C drawing primitives

Why it matters:
- Visual parity between Rust and C modes is essential

### REQ-CS-003: No production markers remain
**Requirement text**: No `TODO`, `FIXME`, `Stub`, `placeholder`, or `for now` markers SHALL remain in production code paths after completion.

Behavior contract:
- GIVEN: Phase 05 modifies rust_comm.c rendering functions
- WHEN: Phase 05 is complete
- THEN: The modified functions contain no deferred-implementation markers

Why it matters:
- Markers indicate incomplete work that must be tracked

## Implementation Tasks

### Files to modify

#### `sc2/src/uqm/rust_comm.c`

- **Implement `c_FeedbackPlayerPhrase`** (replacing stub at lines 665–670)
  - Reproduce the behavior from C's `FeedbackPlayerPhrase` in comm.c
  - Save/restore graphics context (SpaceContext)
  - Clear the player text area below the slider
  - Render the player's phrase text using C text drawing functions
  - Handle NULL/empty text gracefully
  - marker: `@plan PLAN-20260326-COMMPT2.P05`
  - marker: `@requirement REQ-RB-001`

- **Implement `c_RefreshResponses`** (replacing stub at lines 673–681)
  - Reproduce the behavior from C's `RefreshResponses` in comm.c
  - Access the response entries from `pCurInputState` (ENCOUNTER_STATE)
  - Clear the response list area
  - Render each visible response, highlighting the current selection
  - Use proper fonts and colors from the encounter setup
  - marker: `@plan PLAN-20260326-COMMPT2.P05`
  - marker: `@requirement REQ-RB-002`

- **Implement `c_SelectConversationSummary`** (replacing stub at lines 684–687)
  - Reproduce the behavior from C's summary display logic
  - Draw the conversation history overlay
  - Use BatchGraphics/UnbatchGraphics for flicker-free rendering
  - marker: `@plan PLAN-20260326-COMMPT2.P05`
  - marker: `@requirement REQ-RB-003`

#### `sc2/src/uqm/rust_comm.h`

- **Verify declarations** for the three rendering functions match implementations
- Add any additional forward declarations needed for C rendering helpers
  - marker: `@plan PLAN-20260326-COMMPT2.P05`

### C Reference Code Locations

The original C implementations are in comm.c, guarded behind `#ifndef USE_RUST_COMM`:

- `FeedbackPlayerPhrase` — look for this function in comm.c (likely near lines 400–450)
- `RefreshResponses` — look for this function in comm.c (likely near lines 450–550)
- Conversation summary display — look for `SelectConversation`/summary-related code

These functions are `static` in comm.c, so the bridge must replicate their behavior rather
than calling them directly (they're not visible outside comm.c's compilation unit).

### Pseudocode traceability
- Uses pseudocode lines: D01–D40 (C Rendering Bridges)

## Verification Commands

```bash
# C compilation with USE_RUST_COMM=on
# (project-specific build command)

# Verify stubs are replaced
grep -n "P11: Stub" sc2/src/uqm/rust_comm.c
# Should return 0 matches

# Verify rendering functions have real code
grep -A5 "c_FeedbackPlayerPhrase" sc2/src/uqm/rust_comm.c | grep -v "void\|text\|{"
# Should show actual drawing calls, not just (void)text

grep -A5 "c_RefreshResponses" sc2/src/uqm/rust_comm.c | grep -v "void\|unsigned\|{"
# Should show actual rendering calls

grep -A5 "c_SelectConversationSummary" sc2/src/uqm/rust_comm.c | grep -v "void\|{"
# Should show actual overlay calls

# Rust-side tests still pass
cargo test --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `c_FeedbackPlayerPhrase` has a real implementation (not stub)
- [ ] `c_RefreshResponses` has a real implementation (not stub)
- [ ] `c_SelectConversationSummary` has a real implementation (not stub)
- [ ] No `P11: Stub` comments remain in the three functions
- [ ] `rust_comm.h` declarations match implementations
- [ ] `@plan` markers present in modified functions
- [ ] `@requirement` markers present
- [ ] C compiles with `USE_RUST_COMM=on`
- [ ] C compiles with `USE_RUST_COMM=off` (no regressions)

## Semantic Verification Checklist (Mandatory)
- [ ] `c_FeedbackPlayerPhrase` draws text in the correct screen area
- [ ] `c_FeedbackPlayerPhrase` clears the area before drawing (no visual artifacts)
- [ ] `c_FeedbackPlayerPhrase` handles NULL text gracefully
- [ ] `c_RefreshResponses` renders the correct number of visible responses
- [ ] `c_RefreshResponses` highlights the current selection
- [ ] `c_RefreshResponses` uses correct fonts and colors
- [ ] `c_SelectConversationSummary` displays conversation history
- [ ] All three functions use/restore graphics context properly (no state leaks)
- [ ] Visual output matches C-only mode behavior
- [ ] Functions are reachable from Rust: called via FFI from talk_segue.rs

## Deferred Implementation Detection (Mandatory)

```bash
# Reject if these appear in the rendering functions:
grep -n "Stub\|TODO\|FIXME\|HACK\|placeholder\|for now" sc2/src/uqm/rust_comm.c
# Filter to only rendering function lines (665-690) to check
```

## Success Criteria
- [ ] All three rendering bridge stubs are replaced with working implementations
- [ ] No `P11: Stub` markers remain in the three functions
- [ ] Both build modes compile and link
- [ ] All 267+ comm tests pass
- [ ] Visual rendering matches C-only mode

## Failure Recovery
- Rollback: `git checkout -- sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h`
- If original C functions are inaccessible (static): replicate their drawing logic using available C APIs
- If rendering state (fonts/colors) is not accessible: add accessor bridges in P06
- Blocking: must verify C drawing APIs are available before implementation

## Phase Completion Marker
Create: `project-plans/20260311/commpt2/.completed/P05.md`

Contents:
- Phase ID: `PLAN-20260326-COMMPT2.P05`
- Timestamp
- Files changed: `sc2/src/uqm/rust_comm.c`, possibly `sc2/src/uqm/rust_comm.h`
- Tests added/updated: (none — C code, tested via build + manual verification)
- Verification outputs
- Semantic verification summary
