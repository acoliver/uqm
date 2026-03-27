# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260325-COMMPT3.P02a`

## Prerequisites
- Required: Phase P02 completed
- Expected artifacts: 5 pseudocode component files in `analysis/pseudocode/`

## Verification Commands

```bash
# Verify all pseudocode files exist and are non-trivial
for f in project-plans/20260311/commpt3/analysis/pseudocode/*.md; do
  lines=$(wc -l < "$f")
  echo "$(basename $f): $lines lines"
  if [ "$lines" -lt 20 ]; then echo "FAIL: too short"; fi
done

# Verify numbered format is used (should find lines starting with digits)
for f in project-plans/20260311/commpt3/analysis/pseudocode/*.md; do
  count=$(grep -cE '^ *[0-9]+:' "$f")
  echo "$(basename $f): $count numbered lines"
  if [ "$count" -lt 5 ]; then echo "FAIL: insufficient numbered pseudocode"; fi
done
```

## Structural Verification Checklist
- [ ] All 5 component files exist (001 through 005)
- [ ] Each uses numbered algorithmic format
- [ ] Each has validation points section
- [ ] Each has error handling section
- [ ] Each has ordering constraints section or integration boundaries
- [ ] Each has side effects section

## Semantic Verification Checklist (Mandatory)

### 001-colormap-music-bridges.md
- [ ] `c_SetColorMapFromCommData` pseudocode includes null-handle guard (CommData.AlienColorMap == 0)
- [ ] `c_PlayAlienMusic` pseudocode includes null-handle guard (CommData.AlienSong == 0)
- [ ] Rust `set_colormap()` fix replaces `c_SetColorMap(null_mut)` with `c_SetColorMapFromCommData()`
- [ ] Rust `play_alien_music()` fix replaces `c_PlayMusic(null_mut, ...)` with `c_PlayAlienMusic()`
- [ ] Extern block changes documented (remove old, add new declarations)
- [ ] REQ-MU-003 (music playing before first AlienTalkSegue) represented in ordering

### 002-subtitle-display-fix.md
- [ ] `comm_ClearSubtitles` matches `comm.c:1661-1667` behavior exactly
- [ ] `comm_CheckSubtitles` matches `comm.c:1670-1701` behavior exactly (including log_Warning)
- [ ] `comm_RedrawSubtitles` matches `comm.c:1646-1657` behavior exactly (optSubtitles check)
- [ ] Circular routing break documented (rust_comm.c forwards to comm.c, NOT to Rust FFI)
- [ ] REQ-SD-005 (Rust model test-only) is documented

### 003-do-communication-rewrite.md
- [ ] New `CommunicationResult` enum has 4 variants: Talking, ResponseContinue, Selected(fn,ref), Done
- [ ] `do_communication` returns rich result; `player_response_input` called exactly once
- [ ] `rust_DoCommunication` drops lock BEFORE callback invocation
- [ ] Lock discipline invariant explicitly stated (no nested write locking)
- [ ] REQ-DC-005 (CHECK_ABORT/CHECK_LOAD → return 0) documented
- [ ] REQ-DC-002 (talking phase does NOT process response input) documented

### 004-summary-guard-stale-markers.md
- [ ] `rust_ShowConversationSummary` production path delegates to `c_SelectConversationSummary`
- [ ] Rust SummaryView loop retained only under `#[cfg(test)]`
- [ ] All stale markers enumerated with disposition (remove vs. keep with justification)
- [ ] REQ-SM-002 exemptions (doc comments, test blocks, C stub references) documented
- [ ] Automated grep sweep command specified

### 005-end-to-end-integration.md
- [ ] Full encounter flow documented from `InitCommunication` through teardown
- [ ] All [FIXED] bridge calls annotated in the flow
- [ ] User trigger paths table covers: watch NPC, read subtitles, navigate/select responses, summary, replay
- [ ] Deadlock-free verification criteria documented
- [ ] Build verification commands for both `USE_RUST_COMM=on` and `=off`
- [ ] Manual runtime verification checklist present

## Success Criteria
- [ ] All structural checks pass
- [ ] All semantic checks pass

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P02a.md`
