# Pseudocode 005: End-to-End Integration Verification

Plan ID: `PLAN-20260325-COMMPT3`
Requirements: REQ-E2E-001 through REQ-E2E-007, REQ-TS-001 through REQ-TS-004
Implementation Phase: P07, P08

## Full Encounter Flow (Production Path)

```
01: InitCommunication(which_comm)                          [C, comm.c:1429]
02:   init_race(comm_id) → LOCDATA* copied to CommData     [C, commglue.c]
03:   InitEncounter → HAIL or ATTACK                       [C, comm.c]
04:   IF HAIL:
05:     rust_HailAlien()                                    [Rust FFI → hail.rs:174]
06:       load resources (Frame, Font, ColorMap, Song, Phrases, PlayerFont)
07:       create TextCacheContext + AnimContext
08:       c_CallInitEncounterFunc() → race script init      [C → C race script]
09:         race script calls:
10:           NPCPhrase(idx) → rust_NPCPhrase_cb(idx, NULL) → c_SpliceTrack
11:           Response(R, fn) → DoResponsePhrase(R, fn, 0) → rust_DoResponsePhrase
12:           setSegue(s) → rust_SetSegue(s)
13:           DISABLE_PHRASE(p) → rust_DisablePhrase(p)
14:
15:       c_RunEncounterDoInput()                            [C, runs DoInput loop]
16:         each frame: rust_do_communication_cb()
17:           → rust_DoCommunication()                       [Rust FFI → ffi.rs]
18:
19:         FRAME STATE MACHINE:
20:         IF NOT talking_finished:
21:           do_communication → alien_talk_segue(state, WAIT_TRACK_ALL)
22:             IF first call (first_talk_call == false):
23:               init_speech_graphics()       → c_InitSpeechGraphics()
24:               set_colormap()               → c_SetColorMapFromCommData() [FIXED]
25:               draw_alien_frame()           → c_DrawAlienFrame()
26:               update_speech_graphics()     → c_UpdateSpeechGraphics()
27:               comm_intro_transition()      → c_CommIntroTransition()
28:               play_alien_music()           → c_PlayAlienMusic()          [FIXED]
29:               set_music_background_vol()   → c_FadeMusic(BACKGROUND, 0)
30:               init_comm_animations()       → c_InitCommAnimations()
31:               clear_last_activity_load_flag → c_ClearLastActivityLoadFlag()
32:             END IF first call
33:             talk_segue(state, wait_track):
34:               play_track()                → c_PlayTrack()
35:               do_talk_segue() loop:
36:                 check_subtitles()          → c_CheckSubtitles()          [FIXED]
37:                   → comm_CheckSubtitles()   [comm.c, real subtitle update]
38:                 update_animations(seeking)  → c_UpdateAnimations(seeking)
39:                   → ProcessCommAnimations(clear_subtitles, FALSE)
40:                   → if change: comm_RedrawSubtitles()                    [FIXED]
41:                 update_speech_graphics()    → c_UpdateSpeechGraphics()
42:               IF track finished:
43:                 ts.ended = true
44:             IF finished:
45:               talking_finished = true
46:               fade_music_to_foreground()
47:           RETURN CommunicationResult::Talking → return 1 (continue)
48:
49:         ELSE IF responses > 0:
50:           do_communication → player_response_input(state)
51:             UP/DOWN: scroll/wrap responses → c_RefreshResponses
52:             CANCEL: c_SelectConversationSummary()                        [already correct]
53:             LEFT: replay last phrase → talk_segue(state, 0)
54:             SELECT:
55:               → CommunicationResult::Selected(fn, ref)
56:               → rust_DoCommunication: DROP lock, fn(ref)                [FIXED]
57:                 → race script → NPCPhrase → new phrases queued
58:           RETURN 1 (continue)
59:
60:         ELSE (no responses):
61:           do_communication → run_last_replay → CommunicationResult::Done
62:           RETURN 0 (end)
63:
64:       // DoInput loop exited
65:       AnimContext teardown, FlushColorXForms
66:       rust_ClearSubtitles()
67:       StopMusic, StopSound, StopTrack, FadeMusic(NORMAL_VOLUME, 0)
68:       IF NOT aborted: c_CallPostEncounterFunc()
69:       c_CallUninitEncounterFunc()
70:       Destroy all resources in reverse C order
71:       Clear CommData fields
```

## User Trigger Paths

| User Action | Trigger Path |
|---|---|
| Enter orbit near alien ship | `comm_encounter → InitCommunication → rust_HailAlien` |
| Watch NPC talk | `AlienTalkSegue: colormap [FIXED] → portrait → music [FIXED] → track → subtitles [FIXED]` |
| Read subtitles | `comm_CheckSubtitles → comm_RedrawSubtitles → add_text → font_DrawText` [FIXED] |
| Navigate responses | `player_response_input → UP/DOWN → c_RefreshResponses` |
| Select response | `player_response_input → SELECT → select_response → lock drop [FIXED] → callback` |
| View summary | `player_response_input → CANCEL → c_SelectConversationSummary` [already correct] |
| Replay last phrase | `player_response_input → LEFT → talk_segue(state, 0) → c_PlayTrack` |
| Skip phrase | `do_talk_segue → cancel → c_JumpTrack → ts.ended` |

## Requirement-to-Line Contracts

| Requirement | Pseudocode Lines | Contract |
|---|---|---|
| REQ-E2E-001 | 01-71 | Full encounter: colormap + music + subtitles + responses all work |
| REQ-E2E-002 | 52 | Cancel → `c_SelectConversationSummary()` |
| REQ-E2E-003 | 49-58 | Response navigation + selection + callback dispatch |
| REQ-E2E-004 | 53 | Replay via LEFT → `talk_segue(state, 0)` |
| REQ-E2E-005 | 82-83 | Both `USE_RUST_COMM=on` and `=off` builds compile and link |
| REQ-E2E-006 | 84 | 268+ tests pass |
| REQ-E2E-007 | 72-81 | No deadlock — lock dropped before callback, no nested write |
| REQ-TS-001 | 22-32 | First-call intro sequence with real bridge calls |
| REQ-TS-002 | 33-41 | Per-frame: check_subtitles + update_animations + update_speech |
| REQ-TS-003 | 33-41 | Talking animation start/stop (implicit in per-frame ops) |
| REQ-TS-004 | 42-46 | Track completion → `talking_finished = true` + fade music |

## Deadlock-Free Verification Criteria

```
72: INVARIANT: COMM_STATE write lock is never held when calling any extern "C"
73:   function that transitions to a C race script callback
74: PROOF:
75:   1. rust_DoCommunication acquires COMM_STATE.write() at line 42 of pseudocode 003
76:   2. If Selected: lock is dropped at line 56 before callback at line 57
77:   3. C callback calls rust_NPCPhrase_cb/rust_DoResponsePhrase/rust_SetSegue/
78:      rust_DisablePhrase — each acquires its own COMM_STATE.write()
79:   4. No nested write locking on the same thread
80:   5. All other C bridge calls (c_PlayTrack, c_CheckSubtitles, etc.) do not
81:      call back into Rust, so holding the lock during those is safe
```

## Build Verification

```
82: COMMAND: Build with USE_RUST_COMM=on → must compile + link
83: COMMAND: Build with USE_RUST_COMM=off → must compile + link
84: COMMAND: cargo test --workspace --all-features → 268+ tests pass
85: COMMAND: cargo clippy --workspace --all-targets --all-features -- -D warnings
86: COMMAND: cargo fmt --all --check
```

## Manual Runtime Verification Checklist

```
87: [ ] Launch game with USE_RUST_COMM=on
88: [ ] Encounter any alien (e.g., Pkunk, Orz, Starbase)
89: [ ] Alien portrait displays with correct colors (colormap applied)
90: [ ] Background music plays during conversation
91: [ ] Subtitle text appears synchronized with speech audio
92: [ ] Response options display correctly
93: [ ] Selecting a response advances the conversation
94: [ ] Cancel key shows conversation summary with page indicators
95: [ ] Left key replays last phrase
96: [ ] Conversation completes normally
97: [ ] No deadlock or crash during any interaction
98: [ ] Multiple consecutive encounters work without state leak
```
