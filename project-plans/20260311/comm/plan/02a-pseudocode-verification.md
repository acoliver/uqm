# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-COMM.P02a`

## Prerequisites
- Required: Phase 02 (Pseudocode) completed

## Pseudocode Completeness Checklist

### Coverage

- [ ] Component 1 (CommData/LOCDATA FFI): Covers all 26+ LOCDATA fields from spec §3.1
- [ ] Component 2 (init_race dispatch): Uses C-owned `c_init_race(comm_id) -> LOCDATA*` helper without introducing conflicting Rust-owned dispatch
- [ ] Component 3 (Phrase state): Covers PHRASE_ENABLED, DISABLE_PHRASE, reset per encounter
- [ ] Component 4 (NPCPhrase_cb): Covers special indices (GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME, negative), normal phrase resolution
- [ ] Component 5 (NPCPhrase_splice): Covers no-page-break semantics via trackplayer phrase append behavior
- [ ] Component 6 (NPCNumber): Covers number speech table decomposition, text-only fallback
- [ ] Component 7 (construct_response): Covers fragment concatenation from phrase table
- [ ] Component 8 (Segue): Covers all 4 segue values with correct side effects
- [ ] Component 9 (Animation): Covers all 4 anim types, BlockMask, WAIT_TALKING, one-shot
- [ ] Component 10 (Public entry points + encounter lifecycle): Covers `RaceCommunication()`, saved-game SIS update step, init, hail/attack, HailAlien, resource load/teardown, callback ordering
- [ ] Component 11 (Talk segue): Covers playback loop, seek, cancel, poll-loop completion dispatch, animation pause during seek
- [ ] Component 12 (Response UI): Covers rendering, scrolling, selection, scroll indicators
- [ ] Component 13 (Summary): Uses trackplayer subtitle enumeration as source of truth and covers pagination/page boundary carry-over
- [ ] Component 14 (Lock discipline): Covers release-and-reacquire pattern for all callback types

### Validation Points

- [ ] Every pseudocode component has explicit validation/error handling
- [ ] Null pointer checks present for all FFI boundary crossings
- [ ] Lock release before every C callback invocation
- [ ] Lock reacquire after every C callback return
- [ ] Phrase index bounds checking (index > 0 for normal phrases)
- [ ] Response count bounds checking (MAX_RESPONSES = 8)
- [ ] Animation index bounds checking (MAX_ANIMATIONS + 2)
- [ ] `RaceCommunication()` context resolution has explicit fallback/error behavior for unsupported states
- [ ] Saved-game SIS update path is ordered before further encounter setup work

### Ordering Constraints

- [ ] `RaceCommunication()` updates SIS display before `InitCommunication()` when a save was just loaded (lines 171-174)
- [ ] `init_encounter_func` called before dialogue loop (line 206)
- [ ] `post_encounter_func` called after dialogue loop on normal exit (line 212)
- [ ] `uninit_encounter_func` called after post_encounter_func (line 213)
- [ ] Resource teardown after callbacks (line 216)
- [ ] Phrase state reset before init_encounter_func (line 203)
- [ ] Response clear before callback dispatch (line 257-261)
- [ ] Track stop before response callback (line 259)
- [ ] Subtitle clear before response callback (line 260)
- [ ] Music fade before response callback (line 261)
- [ ] Pending track completion polled before next-frame advancement during talk loop (line 234)

### Integration Boundaries

- [ ] Trackplayer calls identified: SpliceTrack, SpliceMultiTrack, PlayTrack, StopTrack, JumpTrack, PlayingTrack, GetTrackSubtitle, GetFirst/NextTrackSubtitle, PollPendingTrackCompletion, CommitTrackAdvancement
- [ ] Graphics calls identified: DrawStamp, font_DrawText, CreateContext, SetContext, LoadGraphic, etc.
- [ ] Game state reads identified: GLOBAL(CurrentActivity), GLOBAL_SIS(), GET_GAME_STATE, saved-game-loaded indicator, `RaceCommunication()` context selection inputs
- [ ] Input reads identified: PulsedInputState.menu, CurrentInputState.menu
- [ ] Encounter flow calls identified: BuildBattle, EncounterBattle, InitEncounter

### Side Effects

- [ ] Phrase disable only affects PHRASE_ENABLED queries (line 44), not resolution (line 70)
- [ ] Segue_victory sets both BATTLE_SEGUE and instantVictory (lines 122-123)
- [ ] Segue_defeat sets crew sentinel and CHECK_RESTART (lines 125-126)
- [ ] Lock released before ALL C callback invocations (lines 205-206, 211-214, 262-264)
- [ ] Summary uses trackplayer history enumeration rather than comm-local shadow history (line 291)

## Implementation Phase Traceability

Each implementation phase must reference specific pseudocode lines:

| Phase | Pseudocode Lines |
|-------|-----------------|
| P03 (CommData/LOCDATA) | 01-26, 30-35 |
| P04 (Phrase/Glue) | 40-50, 55-112, 115-131 |
| P05 (FFI corrections) | Signature fixes — no pseudocode, API shape changes |
| P06 (Track/Trackplayer) | 59, 63, 67, 74, 83-86, 100, 234, 291 |
| P07 (Animation) | 135-167 |
| P08 (Encounter lifecycle / entry points) | 170-216 |
| P09 (Talk segue/main loop) | 220-266, 305-315 |
| P10 (Response UI/Summary/Speech gfx) | 270-300 |
| P11 (C-side wiring) | C-side changes, no Rust pseudocode |
| P12 (E2E) | Verification only |

## Success Criteria
- [ ] All pseudocode components verified complete
- [ ] All validation points verified present
- [ ] All ordering constraints verified correct
- [ ] All integration boundaries verified identified
- [ ] Phase-to-pseudocode traceability complete
