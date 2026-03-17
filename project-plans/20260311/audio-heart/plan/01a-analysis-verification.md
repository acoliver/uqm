# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P01a`

## Prerequisites
- Required: Phase P01 completed

## Structural Verification Checklist
- [ ] All 13 gaps (G1-G13) documented with current-state file:line references
- [ ] All gaps mapped to spec section and/or REQ-* IDs
- [ ] Integration touchpoints table complete
- [ ] Old-code-to-replace inventory has exact file paths and line ranges
- [ ] Every gap from initialstate.md "Parity gaps" section is represented
- [ ] Every gap from initialstate.md "Concrete stub / non-parity behavior" section is represented

## Semantic Verification Checklist
- [ ] Gap severity ratings are justified
- [ ] Resolution strategies are feasible (no dependency on unavailable APIs)
- [ ] Integration touchpoints cover all cross-module boundaries
- [ ] No circular dependencies in the resolution order
- [ ] The old-code inventory covers all files mentioned in initialstate.md §"What remains C-owned or C-dependent"

## Verification Commands

```bash
# Verify all referenced files exist
ls -la rust/src/sound/{stream,trackplayer,music,sfx,control,fileinst,heart_ffi,types,mod}.rs
ls -la sc2/src/libs/sound/{stream,trackplayer,music,sfx,sound,fileinst}.c
ls -la sc2/src/libs/sound/audio_heart_rust.h

# Verify gap references are still accurate
grep -n 'get_music_data' rust/src/sound/music.rs
grep -n 'get_sound_bank_data' rust/src/sound/sfx.rs
grep -n 'splice_multi_track' rust/src/sound/trackplayer.rs
grep -n 'PLRPause' rust/src/sound/heart_ffi.rs
grep -n 'NORMAL_VOLUME' rust/src/sound/control.rs rust/src/sound/types.rs
```

## Success Criteria
- [ ] All gaps from initialstate.md are covered
- [ ] All spec §23.1 and §23.2 end-state requirements are addressed
- [ ] Resolution strategies are concrete (specific functions, specific modules)

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P01a.md`
