# Phase 07: HailAlien

## Phase ID
`PLAN-20260326-COMMPT2.P07`

## Prerequisites
- Required: Phase 06a (Resource Bridge Verification) completed
- All resource bridge wrappers from P06 exist and compile
- All input bridges from P03 are wired
- NPCPhrase from P04 is implemented
- C rendering bridges from P05 are functional
- All existing comm tests pass
- Both build modes compile

## Requirements Implemented (Expanded)

### REQ-HL-001: Full encounter loop
**Requirement text**: `rust_HailAlien` SHALL execute the full encounter loop: load resources, set up animation/text contexts, call init_encounter_func, run DoInput-equivalent loop, call post/uninit_encounter_func, free resources.

Behavior contract:
- GIVEN: The player has chosen to hail an alien and `USE_RUST_COMM=on`
- WHEN: `rust_HailAlien()` is called from `comm.c:1458`
- THEN: The full encounter sequence executes: resource loading → context setup → init_encounter_func → DoInput loop → post_encounter_func (if not aborted) → uninit_encounter_func → resource cleanup

Why it matters:
- This is THE critical function — without it, all alien conversations are skipped

### REQ-HL-002: Load 7 resources via C bridges
**Requirement text**: The encounter loop SHALL load alien portrait (AlienFrame), font (AlienFont), colormap (AlienColorMap), song (AlienSong with alt-song fallback), and conversation phrases (ConversationPhrases) using C bridge resource functions.

Behavior contract:
- GIVEN: LOCDATA has been initialized with alien-specific resource IDs
- WHEN: Resource loading phase of hail_alien() executes
- THEN: PlayerFont, AlienFrame (captured), AlienFont, AlienColorMap (captured), AlienSong (with alt fallback), ConversationPhrases (captured) are loaded via C bridges

Why it matters:
- Every encounter needs these resources — missing any causes crashes or blank screens

### REQ-HL-003: Create and manage contexts
**Requirement text**: The encounter loop SHALL create and manage the AnimContext and TextCacheContext graphics contexts matching C HailAlien behavior.

Behavior contract:
- GIVEN: Resources have been loaded
- WHEN: Context setup executes
- THEN: TextCacheContext is created with a pixmap drawable, background color key, and transparent color. AnimContext is created and configured with Screen as FG frame.

Why it matters:
- TextCacheContext caches subtitle text rendering. AnimContext controls alien portrait animation rendering.

### REQ-HL-004: Call init/post/uninit encounter funcs
**Requirement text**: The encounter loop SHALL invoke the LOCDATA init_encounter_func, then enter the dialogue input loop, then call post_encounter_func (if not aborted), then call uninit_encounter_func unconditionally.

Behavior contract:
- GIVEN: Contexts are set up and CHECK_LOAD has been set
- WHEN: The encounter execution phase runs
- THEN: init_encounter_func() is called, followed by DoInput loop, then post_encounter_func() (only if CHECK_ABORT and CHECK_LOAD are not set in CurrentActivity), then uninit_encounter_func() unconditionally

Why it matters:
- Race scripts depend on this exact call sequence for state machine correctness

### REQ-HL-005: Clean up all resources on exit
**Requirement text**: The encounter loop SHALL clean up all loaded resources (destroy drawables, fonts, colormaps, music, string tables, contexts) on exit regardless of exit path (normal, abort, load).

Behavior contract:
- GIVEN: The encounter loop has run (regardless of how it exited)
- WHEN: The cleanup phase executes
- THEN: All 7 loaded resources + 2 contexts are destroyed in reverse order, CommData fields are cleared, and pCurInputState is set to NULL

Why it matters:
- Resource leaks accumulate across conversations and eventually crash the game

### REQ-HL-006: Set CHECK_LOAD flag
**Requirement text**: The encounter loop SHALL set `LastActivity |= CHECK_LOAD` before calling init_encounter_func to prevent spurious input.

Behavior contract:
- GIVEN: All contexts have been set up
- WHEN: Just before calling init_encounter_func
- THEN: `c_SetLastActivityCheckLoad()` is called

Why it matters:
- Without this flag, queued input events from before the encounter would be processed prematurely

### REQ-HL-007: Draw SIS UI
**Requirement text**: The encounter loop SHALL draw the SIS frame, SIS message, SIS title, and SIS comm window matching C behavior for both WON_LAST_BATTLE and normal encounter contexts.

Behavior contract:
- GIVEN: AnimContext is set up with Screen as FG frame
- WHEN: The SIS drawing phase executes
- THEN: If WON_LAST_BATTLE, only clip rect is set. Otherwise: clip rect is set to SIS_ORG, DrawSISFrame() is called, DrawSISMessage/DrawSISTitle are called (starbase-specific or default), and DrawSISComWindow() is called.

Why it matters:
- The SIS frame provides the visual container for the entire conversation UI

### REQ-DI-001: Integrate with DoInput framework
**Requirement text**: The Rust encounter loop SHALL integrate with C's `DoInput` framework or provide an equivalent frame-driven input loop.

Behavior contract:
- GIVEN: init_encounter_func has been called
- WHEN: The main encounter loop begins
- THEN: `c_DoInput(&ES, FALSE)` is called, which runs the C frame-driven loop with the DoCommunication callback

Why it matters:
- DoInput provides frame timing, graphics batching, and callback dispatch

### REQ-DI-002: DoInput frame dispatch
**Requirement text**: The input loop SHALL call `c_DoInput(encounter_state)` or implement equivalent per-frame dispatch: graphics batch, callback invocation, sleep-thread timing.

Behavior contract:
- GIVEN: The encounter state has InputFunc = DoCommunication
- WHEN: c_DoInput runs
- THEN: Per-frame: BatchGraphics, invoke InputFunc (DoCommunication), UnbatchGraphics, SleepThread timing

Why it matters:
- Frame-driven dispatch is how the game engine processes input and renders

### REQ-DI-003: Respect activity flags
**Requirement text**: The loop SHALL respect `CHECK_ABORT` and `CHECK_LOAD` activity flags to exit cleanly.

Behavior contract:
- GIVEN: The DoInput loop is running
- WHEN: CHECK_ABORT or CHECK_LOAD is set in CurrentActivity
- THEN: The loop exits and post_encounter_func is skipped (uninit still called)

Why it matters:
- These flags indicate the player wants to abort or a save/load operation is in progress

### REQ-DI-004: Frame timing
**Requirement text**: Frame timing SHALL match C's `ONE_SECOND / COMM_ANIM_RATE` cadence.

Behavior contract:
- GIVEN: DoInput is running the encounter loop
- WHEN: Each frame completes
- THEN: SleepThread ensures timing matches COMM_ANIM_RATE (40 fps = 25ms per frame)

Why it matters:
- Wrong frame timing causes animations to play too fast or too slow

### REQ-AT-002: Animation processing in loop
**Requirement text**: Animation processing during the encounter loop SHALL call `rust_ProcessCommAnimations` which delegates to the existing Rust animation engine.

Behavior contract:
- GIVEN: The encounter loop is running via DoCommunication
- WHEN: Each frame of the encounter loop executes
- THEN: Animation processing occurs (via C's UpdateAnimations or Rust's animation engine)

Why it matters:
- Alien portrait animations must play during conversations

### REQ-AT-003: Intro transition animation
**Requirement text**: The intro/transition animation sequence SHALL play when entering a conversation, matching C CommIntroTransition behavior.

Behavior contract:
- GIVEN: An encounter is being initialized
- WHEN: The first frames of the encounter render
- THEN: CommIntroTransition plays the animated transition from space view to alien portrait

Why it matters:
- The transition animation provides visual feedback that a conversation is starting

## Implementation Tasks

### Files to create

#### `rust/src/comm/hail.rs`
- **New file**: Encounter orchestration module
  - `hail_alien()` function implementing the full encounter sequence
  - C bridge extern declarations for all P06 bridge functions used
  - Helper functions for resource loading, context setup, drawing, cleanup
  - Follow C comm.c:1183–1308 step by step
  - marker: `@plan PLAN-20260326-COMMPT2.P07`
  - marker: `@requirement REQ-HL-001`

### Files to modify

#### `rust/src/comm/ffi.rs`
- **Replace `rust_HailAlien` stub** (lines 869–875)
  - Change from empty body to calling `super::hail::hail_alien()`
  - Remove P11 stub comments
  - marker: `@plan PLAN-20260326-COMMPT2.P07`
  - marker: `@requirement REQ-HL-001`

#### `rust/src/comm/mod.rs`
- **Add `pub mod hail;`** to module declarations
  - marker: `@plan PLAN-20260326-COMMPT2.P07`

### Implementation Sequence in `hail.rs`

Following C `HailAlien()` (comm.c:1183–1310) step-by-step, plus DoCommunication
exit handling from comm.c:1100–1138. All 20 operations are listed below.

#### Setup Phase (Steps 1–11)

1. **Encounter state initialization** (C lines 1185–1190)
   - Allocate/zero ENCOUNTER_STATE via C bridge (`memset` to 0)
   - Set `pCurInputState = &ES`
   - Set `TalkingFinished = FALSE`
   - Set `ES.InputFunc = DoCommunication`

2. **Load PlayerFont** (C line 1196)
   - `PlayerFont = LoadFont(PLAYER_FONT)`

3. **Load and set alien resources** (C lines 1198–1212)
   - `CommData.AlienFrame = CaptureDrawable(LoadGraphic(CommData.AlienFrameRes))`
   - `CommData.AlienFont = LoadFont(CommData.AlienFontRes)`
   - `CommData.AlienColorMap = CaptureColorMap(LoadColorMap(CommData.AlienColorMapRes))`
   - Alt-song fallback: if `(AlienSongFlags & LDASF_USE_ALTERNATE) && AlienAltSongRes`, try `LoadMusic(AlienAltSongRes)` first; if non-zero use it, else `LoadMusic(AlienSongRes)`
   - `CommData.ConversationPhrases = CaptureStringTable(LoadStringTable(CommData.ConversationPhrasesRes))`

4. **Subtitle text setup** (C lines 1214–1215)
   - `SubtitleText.baseline = CommData.AlienTextBaseline`
   - `SubtitleText.align = CommData.AlienTextAlign`

5. **TextCacheContext setup** (C lines 1218–1229)
   - `TextCacheContext = CreateContext("TextCacheContext")`
   - `TextCacheFrame = CaptureDrawable(CreateDrawable(WANT_PIXMAP, SIS_SCREEN_WIDTH, SIS_SCREEN_HEIGHT - SLIDER_Y - SLIDER_HEIGHT + 2, 1))`
   - `SetContext(TextCacheContext)`
   - `SetContextFGFrame(TextCacheFrame)`
   - `TextBack = BUILD_COLOR(MAKE_RGB15(0x00, 0x00, 0x10), 0x00)` (color key)
   - `SetContextBackGroundColor(TextBack)`
   - `ClearDrawable()`
   - `SetFrameTransparentColor(TextCacheFrame, TextBack)`

6. **Clear phrase buffer** (C line 1231)
   - `ES.phrase_buf[0] = '\0'`

7. **Set SpaceContext and save old font** (C lines 1233–1234)
   - `SetContext(SpaceContext)`
   - `OldFont = SetContextFont(PlayerFont)` — must save OldFont for restore in step 17

8. **Create AnimContext and configure** (C lines 1236–1247)
   - `AnimContext = CreateContext("AnimContext")`
   - `SetContext(AnimContext)`
   - `SetContextFGFrame(Screen)`
   - `GetFrameRect(CommData.AlienFrame, &r)` for dimensions
   - `r.extent.width = SIS_SCREEN_WIDTH`
   - `CommWndRect.extent = r.extent`

9. **Transition and batching** (C lines 1249–1250)
   - `SetTransitionSource(NULL)`
   - `BatchGraphics()`

10. **Draw SIS UI** (C lines 1251–1276)
    - **WON_LAST_BATTLE branch**: `r.corner = CommWndRect.corner; SetContextClipRect(&r)`
    - **Normal branch**:
      - `r.corner.x = SIS_ORG_X; r.corner.y = SIS_ORG_Y`
      - `SetContextClipRect(&r)`
      - `CommWndRect.corner = r.corner`
      - `DrawSISFrame()`
      - **Starbase check**: if `GET_GAME_STATE(GLOBAL_FLAGS_AND_DATA) == (BYTE)~0 && GET_GAME_STATE(STARBASE_AVAILABLE)`: `DrawSISMessage("Starbase Commander")`, `DrawSISTitle("Starbase")`
      - **Default**: `DrawSISMessage(NULL)`, `DrawSISTitle(GLOBAL_SIS(PlanetName))`
    - `DrawSISComWindow()` (unconditional, C line 1278)

11. **Set CHECK_LOAD and call encounter funcs** (C lines 1282–1287)
    - `LastActivity |= CHECK_LOAD` (prevents spurious input)
    - `(*CommData.init_encounter_func)()`
    - `DoInput(&ES, FALSE)` — main encounter loop

#### DoCommunication Exit Handling (Steps 12–15)

When `DoCommunication` returns FALSE (encounter loop exits), these operations
run inside the final DoCommunication iteration (comm.c:1100–1138):

12. **Finish pending tracks** (C lines 1101–1105)
    - If `!TalkingFinished`: `AlienTalkSegue(WAIT_TRACK_ALL)`, return TRUE (loop continues)

13. **Handle zero-response replay** (C lines 1107–1119)
    - If `CHECK_ABORT`: skip
    - Else if `num_responses == 0`: FadeMusic, DoLastReplay (review alien phrases)
    - Else: `PlayerResponseInput(pES)`, return TRUE (loop continues)

14. **Tear down AnimContext** (C lines 1121–1123)
    - `SetContext(SpaceContext)`
    - `DestroyContext(AnimContext)` ; `AnimContext = NULL`

15. **Stop audio/video** (C lines 1125–1131)
    - `FlushColorXForms()`
    - `ClearSubtitles()`
    - `StopMusic()` ; `StopSound()` ; `StopTrack()`
    - `SleepThreadUntil(FadeMusic(NORMAL_VOLUME, 0) + ONE_SECOND/60)`

#### Post-encounter Phase (Steps 16)

16. **Call post/uninit encounter funcs** (C lines 1285–1287)
    - If `!(CurrentActivity & (CHECK_ABORT | CHECK_LOAD))`: `(*CommData.post_encounter_func)()`
    - Unconditionally: `(*CommData.uninit_encounter_func)()`

#### Cleanup Phase (Steps 17–20)

17. **Restore context and font** (C lines 1289–1290)
    - `SetContext(SpaceContext)`
    - `SetContextFont(OldFont)` — restore the font saved in step 7

18. **Destroy all resources in exact C order** (C lines 1292–1302)
    - `DestroyStringTable(ReleaseStringTable(CommData.ConversationPhrases))` — captured, requires Release
    - `DestroyMusic(CommData.AlienSong)` — not captured, direct Destroy
    - `DestroyColorMap(ReleaseColorMap(CommData.AlienColorMap))` — captured, requires Release
    - `DestroyFont(CommData.AlienFont)` — not captured, direct Destroy
    - `DestroyDrawable(ReleaseDrawable(CommData.AlienFrame))` — captured, requires Release
    - `DestroyContext(TextCacheContext)`
    - `DestroyDrawable(ReleaseDrawable(TextCacheFrame))` — captured, requires Release
    - `DestroyFont(PlayerFont)` — not captured, direct Destroy

19. **Clear CommData fields** (C lines 1305–1306)
    - `CommData.ConversationPhrasesRes = 0`
    - `CommData.ConversationPhrases = 0`

20. **Clear input state** (C line 1307)
    - `pCurInputState = 0`

### C Bridge Extern Declarations in `hail.rs`

```rust
#[cfg(not(test))]
mod c_bridge {
    use std::ffi::{c_char, c_int, c_uint, c_void};

    extern "C" {
        // Resource loading
        pub fn c_LoadGraphic(res: c_uint) -> usize;
        pub fn c_LoadFont(res: c_uint) -> usize;
        pub fn c_LoadColorMap(res: c_uint) -> usize;
        pub fn c_LoadMusic(res: c_uint) -> usize;
        pub fn c_LoadStringTable(res: c_uint) -> usize;
        pub fn c_CaptureDrawable(handle: usize) -> usize;
        pub fn c_CaptureColorMap(handle: usize) -> usize;
        pub fn c_CaptureStringTable(handle: usize) -> usize;

        // Context management
        pub fn c_CreateContext(name: *const c_char) -> usize;
        pub fn c_DestroyContext(ctx: usize);
        pub fn c_SetContext(ctx: usize) -> usize;
        pub fn c_SetContextFGFrame(frame: usize);
        pub fn c_SetContextClipRect(x: c_int, y: c_int, w: c_int, h: c_int);
        pub fn c_SetContextBackGroundColor(r: c_int, g: c_int, b: c_int);
        pub fn c_SetContextFont(font: usize) -> usize;

        // Drawable management
        pub fn c_CreateDrawable(dtype: c_uint, w: c_int, h: c_int, nframes: c_int) -> usize;
        pub fn c_SetFrameTransparentColor(frame: usize, r: c_int, g: c_int, b: c_int);
        pub fn c_ClearDrawable();
        pub fn c_GetFrameRect(frame: usize, x: *mut c_int, y: *mut c_int,
                              w: *mut c_int, h: *mut c_int);

        // Release (for captured resources — MUST call before Destroy)
        pub fn c_ReleaseDrawable(handle: usize) -> usize;
        pub fn c_ReleaseColorMap(handle: usize) -> usize;
        pub fn c_ReleaseStringTable(handle: usize) -> usize;

        // Resource destruction
        pub fn c_DestroyDrawable(handle: usize);
        pub fn c_DestroyFont(handle: usize);
        pub fn c_DestroyColorMap(handle: usize);
        pub fn c_DestroyMusic(handle: usize);
        pub fn c_DestroyStringTable(handle: usize);

        // Graphics batching
        pub fn c_BatchGraphics();
        pub fn c_UnbatchGraphics();

        // Transitions
        pub fn c_SetTransitionSource(src: usize);

        // SIS drawing
        pub fn c_DrawSISFrame();
        pub fn c_DrawSISMessage(msg: *const c_char);
        pub fn c_DrawSISTitle(title: *const c_char);
        pub fn c_DrawSISComWindow();

        // DoCommunication teardown (called during exit handling)
        pub fn c_FlushColorXForms();
        pub fn c_ClearSubtitles();
        pub fn c_StopMusic();
        pub fn c_StopSound();
        pub fn c_StopTrack();
        pub fn c_FadeMusic(vol: c_int, duration: c_int) -> c_int;
        pub fn c_SleepThreadUntil(time: c_int);

        // DoInput
        pub fn c_DoInput(state: *mut c_void, exclusive: c_int);

        // Accessors
        pub fn c_GetScreen() -> usize;
        pub fn c_GetSpaceContext() -> usize;
        pub fn c_SetLastActivityCheckLoad();
        pub fn c_WonLastBattle() -> c_int;
        pub fn c_CheckAbort() -> c_int;
        pub fn c_CheckLoad() -> c_int;

        // CommData accessors
        pub fn c_GetCommDataAlienFrameRes() -> c_uint;
        pub fn c_GetCommDataAlienFontRes() -> c_uint;
        pub fn c_GetCommDataAlienColorMapRes() -> c_uint;
        pub fn c_GetCommDataAlienSongRes() -> c_uint;
        pub fn c_GetCommDataAlienAltSongRes() -> c_uint;
        pub fn c_GetCommDataAlienSongFlags() -> c_uint;
        pub fn c_GetCommDataConversationPhrasesRes() -> c_uint;
        pub fn c_SetCommDataAlienFrame(frame: usize);
        pub fn c_SetCommDataAlienFont(font: usize);
        pub fn c_SetCommDataAlienColorMap(cmap: usize);
        pub fn c_SetCommDataAlienSong(song: usize);
        pub fn c_SetCommDataConversationPhrases(phrases: usize);
        pub fn c_ClearCommDataConversationPhrasesRes();
        pub fn c_ClearCommDataConversationPhrases();
        pub fn c_SetCurInputState(state: *mut c_void);
        pub fn c_SetTalkingFinished(finished: c_int);

        // Encounter functions
        pub fn c_CallInitEncounterFunc();
        pub fn c_CallPostEncounterFunc();
        pub fn c_CallUninitEncounterFunc();

        // UI state
        pub fn c_IsStarbaseConversation() -> c_int;
        pub fn c_GetPlanetName() -> *const c_char;
        pub fn c_GetGameString(base: c_int, offset: c_int) -> *const c_char;
        pub fn c_SetupSubtitleTextFromCommData();
        pub fn c_ClearPhraseBuf();

        // Dimension constants
        pub fn c_GetSISScreenWidth() -> c_int;
        pub fn c_GetSISScreenHeight() -> c_int;
        pub fn c_GetSliderY() -> c_int;
        pub fn c_GetSliderHeight() -> c_int;
        pub fn c_GetSISOrigin(x: *mut c_int, y: *mut c_int);
        pub fn c_GetPlayerFontRes() -> c_uint;
        pub fn c_GetWantPixmap() -> c_uint;
    }
}
```

### Pseudocode traceability
- Uses pseudocode lines: F01–F124 (HailAlien Orchestration)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Verify hail module exists
test -f rust/src/comm/hail.rs && echo "hail.rs exists"

# Verify module declaration
grep "pub mod hail" rust/src/comm/mod.rs

# Verify rust_HailAlien calls hail_alien
grep -A3 "rust_HailAlien" rust/src/comm/ffi.rs | grep "hail"

# Verify no stub remains
grep "P11: Stub" rust/src/comm/ffi.rs
# Should not match at rust_HailAlien

# C build verification
# (project-specific USE_RUST_COMM=on build)
```

## Structural Verification Checklist
- [ ] `rust/src/comm/hail.rs` created with `hail_alien()` function
- [ ] `rust/src/comm/mod.rs` declares `pub mod hail`
- [ ] `rust_HailAlien` in ffi.rs calls `hail::hail_alien()` (not stub)
- [ ] C bridge extern block in hail.rs declares all needed functions (including Release bridges)
- [ ] `hail_alien()` follows the 20-step sequence from C HailAlien
- [ ] Resource loading covers all 7 resources (PlayerFont, AlienFrame, AlienFont, AlienColorMap, AlienSong, ConversationPhrases, TextCacheFrame)
- [ ] TextCacheContext and AnimContext are created
- [ ] SIS drawing has WON_LAST_BATTLE branch
- [ ] Starbase conversation special case is handled
- [ ] CHECK_LOAD flag is set before init_encounter_func
- [ ] DoInput is called
- [ ] DoCommunication exit handling implemented (finish tracks, flush, clear subtitles, stop audio)
- [ ] Zero-response replay mode path is handled
- [ ] Post/uninit encounter funcs are called correctly
- [ ] Context+font restore happens BEFORE resource destruction (step 17)
- [ ] Cleanup uses Release-before-Destroy for captured resources: `DestroyX(ReleaseX(handle))`
- [ ] Cleanup uses direct Destroy for non-captured resources (AlienFont, AlienSong, PlayerFont)
- [ ] Cleanup order matches C exactly: ConversationPhrases → AlienSong → AlienColorMap → AlienFont → AlienFrame → TextCacheContext → TextCacheFrame → PlayerFont
- [ ] CommData.ConversationPhrasesRes and ConversationPhrases cleared to 0
- [ ] pCurInputState cleared to NULL
- [ ] `@plan` and `@requirement` markers present
- [ ] All existing tests compile and pass

## Semantic Verification Checklist (Mandatory)

### Setup correctness
- [ ] Resource loading order matches C (PlayerFont first, then alien resources)
- [ ] Alt-song fallback logic matches C (check `LDASF_USE_ALTERNATE` flag AND `AlienAltSongRes`, fallback to primary)
- [ ] TextCacheContext setup matches C (color key = `BUILD_COLOR(MAKE_RGB15(0x00,0x00,0x10), 0x00)`)
- [ ] TextCacheFrame height = `SIS_SCREEN_HEIGHT - SLIDER_Y - SLIDER_HEIGHT + 2`
- [ ] AnimContext setup matches C (FG frame = Screen, get frame rect)
- [ ] CommWndRect is set correctly from AlienFrame dimensions
- [ ] Clip rect is set correctly for both WON_LAST_BATTLE and normal paths
- [ ] SIS drawing matches C (frame, message, title, comm window)
- [ ] Starbase check matches C (`GLOBAL_FLAGS_AND_DATA == (BYTE)~0 && STARBASE_AVAILABLE`)

### Encounter execution
- [ ] init_encounter_func called AFTER CHECK_LOAD flag is set
- [ ] DoInput called with exclusive=FALSE
- [ ] post_encounter_func called ONLY if `!(CurrentActivity & (CHECK_ABORT | CHECK_LOAD))`
- [ ] uninit_encounter_func called UNCONDITIONALLY

### DoCommunication exit handling
- [ ] TalkingFinished check: if not finished, call AlienTalkSegue(WAIT_TRACK_ALL) and continue loop
- [ ] CHECK_ABORT branch: skip replay/response, proceed to teardown
- [ ] Zero-response replay: FadeMusic(0, 3s), DoLastReplay, then teardown
- [ ] AnimContext destruction occurs inside DoCommunication exit (before HailAlien cleanup)
- [ ] FlushColorXForms, ClearSubtitles called
- [ ] StopMusic, StopSound, StopTrack called in sequence
- [ ] SleepThreadUntil(FadeMusic(NORMAL_VOLUME, 0) + ONE_SECOND/60) for fade completion

### Cleanup correctness
- [ ] Context restored to SpaceContext BEFORE resource destruction
- [ ] Font restored to saved OldFont BEFORE resource destruction
- [ ] Captured resources use Release-before-Destroy: `DestroyStringTable(ReleaseStringTable(…))`, `DestroyColorMap(ReleaseColorMap(…))`, `DestroyDrawable(ReleaseDrawable(…))`
- [ ] Non-captured resources use direct Destroy: `DestroyMusic(…)`, `DestroyFont(…)`
- [ ] Cleanup order matches C exactly: ConversationPhrases → AlienSong → AlienColorMap → AlienFont → AlienFrame → TextCacheContext → TextCacheFrame → PlayerFont
- [ ] Cleanup is unconditional (runs on all exit paths: normal, abort, load)
- [ ] CommData.ConversationPhrasesRes cleared to 0
- [ ] CommData.ConversationPhrases cleared to 0
- [ ] pCurInputState cleared to NULL
- [ ] Feature is reachable: comm.c:1458 → rust_HailAlien → hail_alien

## Deferred Implementation Detection (Mandatory)

```bash
# Reject if these appear in hail.rs:
grep -n "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|stub" rust/src/comm/hail.rs
# Must be 0 matches

# Reject if stub remains in ffi.rs for rust_HailAlien:
grep -A5 "rust_HailAlien" rust/src/comm/ffi.rs | grep -ic "stub\|empty\|P11"
# Must be 0
```

## Success Criteria
- [ ] `hail_alien()` implements all 20 steps from C HailAlien + DoCommunication exit
- [ ] `rust_HailAlien` FFI export calls `hail_alien()`
- [ ] All 7 resources loaded and all resources freed on exit
- [ ] Both contexts created and managed correctly
- [ ] SIS UI drawn correctly for both WON_LAST_BATTLE and normal paths
- [ ] Encounter function call sequence matches C (init → DoInput → post → uninit)
- [ ] DoCommunication exit handling: flush, clear subtitles, stop music/sound/track, fade/sleep
- [ ] Cleanup uses correct Release-before-Destroy pattern for captured resources
- [ ] Cleanup order matches C exactly (ConversationPhrases first, PlayerFont last)
- [ ] Context+font restored before resource destruction
- [ ] CommData fields and pCurInputState cleared after destruction
- [ ] All 267+ comm tests pass
- [ ] Both build modes compile and link
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/comm/ffi.rs rust/src/comm/mod.rs && rm -f rust/src/comm/hail.rs`
- If bridge functions from P06 have wrong signatures: fix P06 first
- If DoInput doesn't work as expected: investigate C ENCOUNTER_STATE structure
- If encounter functions crash: verify LOCDATA is properly initialized
- Blocking: P06 bridges must all exist and have correct signatures

## Phase Completion Marker
Create: `project-plans/20260311/commpt2/.completed/P07.md`

Contents:
- Phase ID: `PLAN-20260326-COMMPT2.P07`
- Timestamp
- Files changed: new `rust/src/comm/hail.rs`, modified `ffi.rs`, `mod.rs`
- Tests added/updated: (list)
- Verification outputs
- Semantic verification summary
