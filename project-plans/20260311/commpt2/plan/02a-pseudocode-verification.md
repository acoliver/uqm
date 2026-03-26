# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260326-COMMPT2.P02a`

## Prerequisites
- Required: Phase 02 (Pseudocode) completed
- Pseudocode document exists at `project-plans/20260311/commpt2/plan/02-pseudocode.md`

## Structural Verification Checklist

- [ ] Pseudocode A (Input Bridge) covers all 6 input functions: select, cancel, up, down, left, right
- [ ] Pseudocode A includes key constant definitions matching C controls.h
- [ ] Pseudocode A has both `#[cfg(not(test))]` and `#[cfg(test)]` branches
- [ ] Pseudocode B (Transition Animation) covers `has_transition_anim`
- [ ] Pseudocode C (NPC Phrase) covers `rust_NPCPhrase_cb` with callback parameter
- [ ] Pseudocode C covers `rust_NPCPhrase_splice` as delegation to cb variant
- [ ] Pseudocode C includes phrase text resolution via `c_get_conversation_phrase`
- [ ] Pseudocode C includes `c_SpliceTrack` call
- [ ] Pseudocode C includes conversation summary update
- [ ] Pseudocode D (C Rendering) covers `c_FeedbackPlayerPhrase`
- [ ] Pseudocode D covers `c_RefreshResponses`
- [ ] Pseudocode D covers `c_SelectConversationSummary`
- [ ] Pseudocode E (Resource Bridge) covers all Load functions (5: Graphic, Font, ColorMap, Music, StringTable)
- [ ] Pseudocode E covers all Capture functions (3: Drawable, ColorMap, StringTable)
- [ ] Pseudocode E covers all Release functions (3: Drawable, ColorMap, StringTable)
- [ ] Pseudocode E covers context management (Create, Destroy, Set, SetFGFrame, SetClipRect, SetBGColor)
- [ ] Pseudocode E covers drawing utilities (BatchGraphics, UnbatchGraphics, ClearDrawable)
- [ ] Pseudocode E covers SIS drawing (DrawSISFrame, DrawSISMessage, DrawSISTitle)
- [ ] Pseudocode E covers DoInput bridge
- [ ] Pseudocode F (HailAlien) follows C comm.c:1183–1308 step-by-step
- [ ] Pseudocode F covers all 7 resource loads
- [ ] Pseudocode F covers alt-song fallback logic
- [ ] Pseudocode F covers TextCacheContext setup (lines F43–F54)
- [ ] Pseudocode F covers AnimContext setup (lines F64–F73)
- [ ] Pseudocode F covers SIS drawing with WON_LAST_BATTLE branch (lines F79–F93)
- [ ] Pseudocode F covers CHECK_LOAD flag (line F96)
- [ ] Pseudocode F covers init/post/uninit encounter func calls (lines F99–F105)
- [ ] Pseudocode F covers resource cleanup in reverse order (lines F111–F119)
- [ ] Pseudocode F covers CommData field clearing (lines F121–F124)
- [ ] Pseudocode G (Integration Sweep) covers deferred implementation detection
- [ ] Pseudocode G covers test verification
- [ ] Pseudocode G covers dual build mode verification
- [ ] All pseudocode lines are numbered for traceability

## Semantic Verification Checklist

- [ ] Input bridge pseudocode matches C `PulsedInputState.menu[key_index]` semantics
- [ ] NPC Phrase pseudocode handles null/invalid phrase indices gracefully
- [ ] C Rendering pseudocode uses correct graphics contexts (SpaceContext)
- [ ] Resource bridge pseudocode uses correct C type casts (uintptr_t for handles)
- [ ] HailAlien pseudocode matches the exact sequence from C comm.c:1183–1308
- [ ] HailAlien pseudocode handles the starbase conversation special case
- [ ] HailAlien cleanup runs unconditionally (not guarded by abort/load checks)
- [ ] Test paths (`#[cfg(test)]`) maintain existing behavior (no regressions)

## Verification Commands

```bash
# Verify pseudocode references match actual C code
grep -n "CaptureDrawable\|CaptureColorMap\|CaptureStringTable" sc2/src/uqm/comm.c
grep -n "CreateContext\|DestroyContext\|SetContext" sc2/src/uqm/comm.c
grep -n "DrawSISFrame\|DrawSISMessage\|DrawSISTitle" sc2/src/uqm/comm.c
grep -n "LoadGraphic\|LoadFont\|LoadColorMap\|LoadMusic\|LoadStringTable" sc2/src/uqm/comm.c
```

## Pass/Fail Gate Criteria

**PASS if**:
- All structural checks confirmed
- All semantic checks confirmed
- Pseudocode line numbers are sequential and referenceable
- Every implementation phase (P03–P08) has corresponding pseudocode

**FAIL if**:
- Any pseudocode section is missing
- HailAlien pseudocode deviates from C reference
- Resource cleanup order doesn't match C
- Test-mode branches are missing
