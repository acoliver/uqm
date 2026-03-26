# Communication Subsystem Completion (Part 2) — Specification

Plan ID: `PLAN-20260326-COMMPT2`

## 1. Problem Statement

The first comm plan (PLAN-20260314-COMM, phases P03–P12) built Rust modules
for animation, track management, encounter lifecycle, response handling,
subtitle tracking, speech graphics, and the talk-segue state machine.  It also
wired C-side `#ifndef USE_RUST_COMM` guards and created bridge wrappers.

However, four critical integration gaps remain:

1. **`rust_HailAlien` is an empty stub** — the function that replaces C's
   `HailAlien()` does nothing.  When `USE_RUST_COMM=on`, conversations are
   silently skipped.

2. **Input bridge functions return hardcoded `false`** — `check_select_input`,
   `check_cancel_input`, `check_up/down/left/right_input` all return `false`
   in non-test mode, making the dialogue loop non-interactive.

3. **`rust_NPCPhrase_cb` and `rust_NPCPhrase_splice` are empty stubs** — NPC
   speech emission doesn't work through the Rust path.

4. **C-side rendering bridges are stubs** — `c_FeedbackPlayerPhrase`,
   `c_RefreshResponses`, `c_SelectConversationSummary` do nothing.

This plan implements all four gaps and verifies end-to-end functionality.

## 2. Architecture Overview

### 2.1 Current Module Structure (from comm pt1)

```
rust/src/comm/
  mod.rs              — module declarations
  state.rs            — CommState + COMM_STATE LazyLock<RwLock>
  types.rs            — CommError, enums
  ffi.rs              — 74 FFI exports (70 real, 4 stubs)
  talk_segue.rs       — dialogue state machine (1361 lines)
  animation.rs        — CommAnimState engine (950 lines)
  encounter.rs        — lifecycle management
  response.rs         — ResponseSystem
  track.rs            — TrackManager + CTrackBridge
  subtitle.rs         — SubtitleTracker
  oscilloscope.rs     — waveform model
  summary.rs          — ConversationSummary + SummaryView
  speech_graphics.rs  — SliderState + SpeechGraphics
  response_ui.rs      — VisibleRange + ResponseUI
  subtitle_display.rs — change detection
  glue.rs             — C glue functions
  locdata.rs          — LOCDATA access
  phrase_state.rs     — PhraseState
  segue.rs            — Segue state
```

### 2.2 C-side Bridge (from comm pt1)

```
sc2/src/uqm/rust_comm.c  — 839 lines, ~80 bridge wrappers
sc2/src/uqm/rust_comm.h  — declarations for Rust FFI + C bridges
sc2/src/uqm/comm.c       — guards around lines 420–1308, HailAlien routing
sc2/src/uqm/commglue.h   — PHRASE_ENABLED/DISABLE_PHRASE routed
```

### 2.3 What This Plan Changes

| Component | Change |
|---|---|
| `rust/src/comm/ffi.rs` | Implement `rust_HailAlien`, `rust_NPCPhrase_cb`, `rust_NPCPhrase_splice` |
| `rust/src/comm/talk_segue.rs` | Replace hardcoded-false input functions with C bridge calls |
| `rust/src/comm/talk_segue.rs` | Implement `has_transition_anim` via C bridge |
| `sc2/src/uqm/rust_comm.c` | Implement rendering bridge stubs (`c_FeedbackPlayerPhrase`, `c_RefreshResponses`, `c_SelectConversationSummary`) |
| `sc2/src/uqm/rust_comm.h` | Add any missing declarations |
| New: `rust/src/comm/hail.rs` | HailAlien encounter orchestration (resource load, context setup, loop, cleanup) |

### 2.4 What This Plan Does NOT Change

- 27 race scripts (C, untouched)
- `InitCommunication` / `RaceCommunication` (remain in C)
- `init_race` switch (remains in C)
- Animation engine internals (already implemented)
- Track/subtitle/response system internals (already implemented)

## 3. Integration Points

### 3.1 C → Rust Call Path (existing, needs implementation)

```
InitCommunication()                    [C, stays in C]
  └─ if (status == HAIL)
       └─ rust_HailAlien()             [Rust FFI export, currently stub]
            └─ hail_alien()            [New Rust function]
                 ├─ load resources via c_* bridges
                 ├─ setup contexts via c_* bridges
                 ├─ call init_encounter_func via c_locdata_get_init_func
                 ├─ run encounter loop (DoCommunication equivalent)
                 │    ├─ check_*_input via c_GetPulsedMenuKey
                 │    ├─ talk_segue / alien_talk_segue (existing Rust)
                 │    ├─ player_response_input (existing Rust)
                 │    └─ c_DoInput for frame dispatch
                 ├─ call post_encounter_func (if not aborted)
                 ├─ call uninit_encounter_func
                 └─ free resources via c_Destroy* bridges
```

### 3.2 Rust → C Bridges (existing wrappers, need to be called)

Already in rust_comm.c/h:
- `c_GetPulsedMenuKey(key_index)` — input polling
- `c_PlayingTrack()` / `c_SpliceTrack()` / `c_PlayTrack()` etc. — trackplayer
- `c_ClearSubtitles()` / `c_CheckSubtitles()` / `c_RedrawSubtitles()` — subtitle rendering
- `c_InitSpeechGraphics()` / `c_UpdateSpeechGraphics()` — slider/oscilloscope
- `c_SetMenuSounds()` — menu sound effects
- `c_DrawAlienFrame()` — portrait rendering
- `c_SetColorMap()` — colormap application

Need to be added/completed:
- `c_LoadGraphic(res)` / `c_LoadFont(res)` / `c_LoadColorMap(res)` / `c_LoadMusic(res)` — resource loading
- `c_DestroyDrawable(handle)` / `c_DestroyFont(handle)` / `c_DestroyColorMap(handle)` / `c_DestroyMusic(handle)` — resource cleanup
- `c_CreateContext(name)` / `c_DestroyContext(ctx)` — graphics context management
- `c_SetContext(ctx)` / `c_SetContextFGFrame(frame)` / `c_SetContextClipRect(rect)` — context configuration
- `c_BatchGraphics()` / `c_UnbatchGraphics()` — draw batching
- `c_DrawSISFrame()` / `c_DrawSISMessage(msg)` / `c_DrawSISTitle(title)` — SIS display
- `c_DoInput(state)` — frame-driven input loop
- `c_SetTransitionSource(src)` / `c_ScreenTransition(num_frames, rect)` — screen transitions
- `c_FadeMusic(vol, duration)` / `c_StopMusic()` / `c_StopSound()` — already exist

### 3.3 Resource Lifecycle

HailAlien loads 7 resources and must free all of them on every exit path:

| Resource | Load | Free |
|---|---|---|
| AlienFrame | `CaptureDrawable(LoadGraphic(res))` | `DestroyDrawable(ReleaseDrawable(frame))` |
| AlienFont | `LoadFont(res)` | `DestroyFont(font)` |
| AlienColorMap | `CaptureColorMap(LoadColorMap(res))` | `DestroyColorMap(ReleaseColorMap(cmap))` |
| AlienSong | `LoadMusic(res)` | `DestroyMusic(song)` |
| ConversationPhrases | `CaptureStringTable(LoadStringTable(res))` | `DestroyStringTable(ReleaseStringTable(table))` |
| PlayerFont | `LoadFont(PLAYER_FONT)` | `DestroyFont(font)` |
| TextCacheFrame | `CaptureDrawable(CreateDrawable(...))` | `DestroyDrawable(ReleaseDrawable(frame))` |

Plus 2 contexts: AnimContext and TextCacheContext (created and destroyed).

## 4. C Reference: HailAlien (comm.c lines 1170–1296)

The C HailAlien function does the following in order:

1. Initialize ENCOUNTER_STATE, set TalkingFinished=FALSE, InputFunc=DoCommunication
2. Load PlayerFont
3. Load AlienFrame, AlienFont, AlienColorMap, AlienSong (with alt fallback), ConversationPhrases
4. Set SubtitleText baseline and alignment from CommData
5. Create TextCacheContext + TextCacheFrame, set background color, clear, set transparent
6. Set phrase_buf to empty
7. Set SpaceContext, set PlayerFont
8. Create AnimContext, set to Screen, get frame rect, configure CommWndRect
9. SetTransitionSource(NULL), BatchGraphics
10. Draw SIS frame/message/title (conditional on WON_LAST_BATTLE)
11. DrawSISComWindow
12. Set LastActivity |= CHECK_LOAD
13. Call init_encounter_func
14. DoInput(&ES, FALSE) — the main encounter loop
15. If not aborted: call post_encounter_func
16. Call uninit_encounter_func
17. Restore SpaceContext and font
18. Destroy all resources in reverse order
19. Clear CommData.ConversationPhrasesRes and ConversationPhrases
20. Clear pCurInputState

## 5. Key Constants

These values come from the second `enum` in `sc2/src/uqm/controls.h`, where
KEY_PAUSE=0 … KEY_FULLSCREEN=4 precede the KEY_MENU_* entries:

| Constant | Value | Used For |
|---|---|---|
| KEY_MENU_UP | 5 | Navigate responses |
| KEY_MENU_DOWN | 6 | Navigate responses |
| KEY_MENU_LEFT | 7 | Replay / reverse |
| KEY_MENU_RIGHT | 8 | Fast-forward |
| KEY_MENU_SELECT | 9 | Confirm response |
| KEY_MENU_CANCEL | 10 | Summary / skip |
| SLIDER_Y | varies | Slider position |
| SLIDER_HEIGHT | varies | Slider height |
| SIS_SCREEN_WIDTH | varies | Screen dimensions |
| SIS_SCREEN_HEIGHT | varies | Screen dimensions |
| COMM_ANIM_RATE | ONE_SECOND/40 | Animation frame rate |
| OSCILLOSCOPE_RATE | ONE_SECOND/32 | Oscilloscope update rate |

## 6. Phase Summary

| Phase | Description | ~LoC |
|---|---|---|
| P00.5 | Preflight: verify existing bridges, C functions, key indices | — |
| P01 | Analysis: map every stub to its resolution, trace call paths | — |
| P01a | Analysis verification | — |
| P02 | Pseudocode: HailAlien orchestration, input bridge, NPCPhrase | — |
| P02a | Pseudocode verification | — |
| P03 | Input bridge: wire check_*_input to c_GetPulsedMenuKey | ~100 |
| P03a | Input bridge verification | — |
| P04 | NPCPhrase: implement rust_NPCPhrase_cb/splice | ~150 |
| P04a | NPCPhrase verification | — |
| P05 | C rendering bridges: implement c_FeedbackPlayerPhrase, c_RefreshResponses, c_SelectConversationSummary | ~200 |
| P05a | C rendering verification | — |
| P06 | Resource bridge: add C wrappers for Load/Destroy/Context ops | ~300 |
| P06a | Resource bridge verification | — |
| P07 | HailAlien: implement full encounter orchestration | ~500 |
| P07a | HailAlien verification | — |
| P08 | Integration test + deferred implementation sweep | ~100 |
| P08a | Final E2E verification | — |

Estimated total: ~1,350 new/modified Rust LoC + ~300 C LoC.
