# Communication Subsystem Production Parity (Part 3) — Domain Model

Plan ID: `PLAN-20260325-COMMPT3`

## 1. Entities

### 1.1 CommState (Rust — `rust/src/comm/state.rs`)

Central dialogue state machine stored in `COMM_STATE: LazyLock<RwLock<CommState>>`.

Owns:
- `talking_finished: bool` — phase tracking
- `first_talk_call: bool` — whether intro sequence has run
- `responses: ResponseSystem` — up to 8 response slots
- `phrase_state: PhraseState` — disabled-phrase tracking
- `segue: Segue` — encounter outcome (Peace/Hostile/Victory/Defeat)
- `top_response: Option<u8>` — scroll position for response list
- `track: TrackManager` — Rust-side track model (test only in production)
- `subtitle_display: SubtitleDisplay` — Rust-side subtitle change tracking (test only in production)
- `summary: ConversationSummary` — Rust-side summary model (test only in production)

### 1.2 CommData (C static — `sc2/src/uqm/comm.c` via `commglue.h`)

Per-encounter descriptor populated by `init_*_comm()` race scripts. Contains:
- Resource handles: `AlienFrame`, `AlienFont`, `AlienColorMap`, `AlienSong`, `ConversationPhrases`
- Callback pointers: `init_encounter_func`, `post_encounter_func`, `uninit_encounter_func`
- Layout metadata: `AlienTextBaseline`, `AlienTextAlign`, text width
- Animation descriptors: `AlienTalkDesc`, `AlienTransitionDesc`, `AlienAmbientArray`

### 1.3 SubtitleText (C static — `comm.c:103`)

C-side subtitle rendering state:
- `pStr: *const UNICODE` — current subtitle text from trackplayer
- `baseline: POINT` — screen position (from CommData)
- `align: TEXT_ALIGN` — text alignment
- `CharCount: COUNT` — character count for rendering
- Associated: `clear_subtitles: BOOLEAN` (`comm.c:102`), `last_subtitle: *const UNICODE` (`comm.c:104`)

Flow: `GetTrackSubtitle() → CheckSubtitles() → SubtitleText updated → RedrawSubtitles() → add_text(1, &t) → font_DrawText()`

### 1.4 Trackplayer (C — `libs/sound/trackplayer.c`)

Authoritative audio playback engine. Owns:
- Phrase queue (`SpliceTrack`/`SpliceMultiTrack`)
- Playback state (`PlayTrack`/`StopTrack`/`PlayingTrack`)
- Subtitle history (`GetFirstTrackSubtitle`/`GetNextTrackSubtitle`)
- Current subtitle pointer (`GetTrackSubtitle`)

## 2. State Transitions

### 2.1 DoCommunication State Machine (per frame)

```
┌─────────────────────────────────────────────────────────┐
│                    rust_DoCommunication                   │
│                    (one frame per call)                   │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  [CHECK_ABORT or CHECK_LOAD?] ──yes──► return 0 (Done)   │
│       │no                                                 │
│       ▼                                                   │
│  [talking_finished == false?] ──yes──► alien_talk_segue   │
│       │                                     │              │
│       │                               return 1 (Continue) │
│       │no                                                 │
│       ▼                                                   │
│  [responses.count() > 0?] ──yes──► do_communication      │
│       │                                  │                │
│       │                           ┌──────┴──────┐        │
│       │                      Continue       Selected     │
│       │                        │                │        │
│       │                  return 1     select_response    │
│       │                               extract (fn,ref)  │
│       │                               DROP lock         │
│       │                               fn(ref)           │
│       │                               return 1          │
│       │no                                                │
│       ▼                                                   │
│  run_last_replay ──► return 0 (Done)                      │
└─────────────────────────────────────────────────────────┘
```

### 2.2 AlienTalkSegue First-Call Sequence

```
first_talk_call == false
  │
  ├─ init_speech_graphics()   → c_InitSpeechGraphics()
  ├─ set_colormap()           → c_SetColorMapFromCommData() [NEW]
  │                             └─ SetColorMap(GetColorMapAddress(CommData.AlienColorMap))
  ├─ draw_alien_frame()       → c_DrawAlienFrame()
  ├─ update_speech_graphics() → c_UpdateSpeechGraphics()
  ├─ comm_intro_transition()  → c_CommIntroTransition()
  ├─ play_alien_music()       → c_PlayAlienMusic() [NEW]
  │                             └─ PlayMusic(CommData.AlienSong, TRUE, 1)
  ├─ set_music_background_vol → c_FadeMusic(BACKGROUND_VOL, 0)
  ├─ init_comm_animations()   → c_InitCommAnimations()
  ├─ clear_load_flag()        → c_ClearLastActivityLoadFlag()
  │
  └─ first_talk_call = true
```

### 2.3 Response Callback Dispatch Sequence

```
do_communication() returns CommunicationResult::Continue
  player_response_input() returned PlayerInputResult::Selected
    │
    ├─ [holding COMM_STATE write lock]
    │    ├─ select_response(&mut state) called
    │    │    ├─ get selected response entry
    │    │    ├─ feedback_player_phrase(text)  → c_FeedbackPlayerPhrase(text)
    │    │    ├─ stop_track()                  → c_StopTrack()
    │    │    ├─ clear_subtitles()             → c_ClearSubtitles()
    │    │    ├─ set_slider_image(Play)
    │    │    ├─ fade_music_to_background()    → c_FadeMusic(BACKGROUND, ONE_SECOND)
    │    │    ├─ set talking_finished = false
    │    │    ├─ clear responses
    │    │    └─ return Some((callback_fn, response_ref))
    │    └─ extract (fn, ref)
    │
    ├─ [COMM_STATE lock dropped — explicit `drop(state)`]
    │
    └─ callback_fn(response_ref)    → C race script
         ├─ rust_NPCPhrase_cb(idx, cb)      → acquires COMM_STATE.write()
         ├─ rust_DoResponsePhrase(ref, t, f) → acquires COMM_STATE.write()
         ├─ rust_SetSegue(s)                 → acquires COMM_STATE.write()
         └─ rust_DisablePhrase(p)            → acquires COMM_STATE.write()
```

### 2.4 Subtitle Update per Frame

```
DoCommunication frame (talking phase via do_talk_segue)
  │
  ├─ check_subtitles()
  │    └─ c_CheckSubtitles() [rust_comm.c]
  │         └─ comm_CheckSubtitles() [comm.c, #ifdef USE_RUST_COMM]
  │              └─ pStr = GetTrackSubtitle()
  │                   if pStr != SubtitleText.pStr → clear_subtitles = TRUE
  │                   → update SubtitleText
  │
  ├─ update_animations(seeking)
  │    └─ c_UpdateAnimations(seeking) [rust_comm.c]
  │         └─ change = ProcessCommAnimations(clear_subtitles, FALSE)
  │              if change || clear_subtitles → RedrawSubtitles
  │                  └─ comm_RedrawSubtitles() [comm.c]
  │                       └─ add_text(1, &SubtitleText) → font_DrawText
  │
  └─ update_speech_graphics()
       └─ c_UpdateSpeechGraphics() [rust_comm.c]
            └─ DrawOscilloscope() + DrawSlider()
```

## 3. Edge and Error Map

| Edge Case | Handling |
|---|---|
| `CommData.AlienColorMap == 0` | `c_SetColorMapFromCommData` checks handle; if 0, skip `SetColorMap` call |
| `CommData.AlienSong == 0` | `c_PlayAlienMusic` checks handle; if 0, skip `PlayMusic` call |
| No subtitle text (`pStr == NULL`) | `comm_CheckSubtitles` sets `CharCount=0`; `comm_RedrawSubtitles` skips (pStr is NULL) |
| Callback function pointer is `NULL` | `select_response` returns `None` via the `resp.response_func?` early return |
| COMM_STATE lock contention during callback | Not possible: lock is dropped before callback; callback acquires its own separate lock |
| CHECK_ABORT during response callback | Next `rust_DoCommunication` frame detects abort via `do_communication` and returns 0 |
| Response overflow (>8) | `ResponseSystem` rejects excess (existing behavior, unchanged) |
| Summary with 0 subtitle entries | `c_SelectConversationSummary` shows empty page, returns on any key |
| `c_PlayMusic` / `c_SetColorMap` removed from talk_segue extern block | No caller loss — these were only used by `play_alien_music` and `set_colormap` which are replaced |

## 4. Integration Touchpoints

### 4.1 Files Modified

| File | Changes | Requirements |
|---|---|---|
| `rust/src/comm/talk_segue.rs` | Fix `set_colormap()` and `play_alien_music()` to use new zero-arg bridges; add new extern declarations; remove `c_SetColorMap` and `c_PlayMusic` from extern block (or keep for other callers — verify no other callers first) | REQ-CM-001/002, REQ-MU-001/002, REQ-SM-001 |
| `rust/src/comm/ffi.rs` | Rewrite `rust_DoCommunication` for clean lock discipline; guard `rust_ShowConversationSummary` production path | REQ-RL-001/002/003, REQ-DC-001-005, REQ-CS-002, REQ-SM-001 |
| `sc2/src/uqm/rust_comm.c` | Add `c_SetColorMapFromCommData()`, `c_PlayAlienMusic()`; fix subtitle bridges to call `comm.c` implementations | REQ-CM-002, REQ-MU-002, REQ-SD-001 |
| `sc2/src/uqm/rust_comm.h` | Declare new bridge functions (`c_SetColorMapFromCommData`, `c_PlayAlienMusic`, `comm_ClearSubtitles`, `comm_CheckSubtitles`, `comm_RedrawSubtitles`) | — |
| `sc2/src/uqm/comm.c` | Add `comm_ClearSubtitles`, `comm_CheckSubtitles`, `comm_RedrawSubtitles` in `#ifdef USE_RUST_COMM` block | REQ-SD-002/003/004 |

### 4.2 Files NOT Modified

| File | Reason |
|---|---|
| `rust/src/comm/hail.rs` | Orchestration is correct; the bridges it calls are what need fixing |
| `rust/src/comm/state.rs` | State machine structure is correct |
| `rust/src/comm/response.rs` | Response system works correctly |
| `sc2/src/uqm/commglue.c` | NPCPhrase routing is correct |
| `sc2/src/uqm/commglue.h` | Macro routing is correct |
| 27 race scripts | Untouched |

## 5. Dependency Analysis

### 5.1 Build-time Dependencies

- `c_SetColorMapFromCommData` depends on `GetColorMapAddress` (declared in
  `libs/graphics/gfx_common.h`) and `SetColorMap` (declared in
  `libs/graphics/gfxlib.h`)
- `c_PlayAlienMusic` depends on `PlayMusic` (declared in `libs/sound/sound.h`)
- `comm_CheckSubtitles` depends on `GetTrackSubtitle` (declared in
  `libs/sound/trackplayer.h`) and `SubtitleText`, `clear_subtitles`,
  `last_subtitle` (static in `comm.c` — must live in `comm.c`)
- `comm_RedrawSubtitles` depends on `add_text` (static in `comm.c` — must
  live in `comm.c`)

### 5.2 Implementation Order Dependencies

1. **Colormap + Music bridges** (REQ-CM, REQ-MU) — independent of other fixes
2. **Subtitle display fix** (REQ-SD) — independent of colormap/music
3. **DoCommunication rewrite** (REQ-RL, REQ-DC) — independent of subtitle fix
4. **Summary guard + stale markers** (REQ-CS, REQ-SM) — depends on DoCommunication rewrite (same file)
5. **Integration verification** (REQ-E2E) — depends on all above

Phases 1-3 are parallelizable in principle but will be executed sequentially
per PLAN.md rules.
