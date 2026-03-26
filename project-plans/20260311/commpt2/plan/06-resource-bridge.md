# Phase 06: Resource Bridge

## Phase ID
`PLAN-20260326-COMMPT2.P06`

## Prerequisites
- Required: Phase 05a (C Rendering Verification) completed
- Existing destroy bridges exist in rust_comm.c (lines 809–836)
- C resource management functions (LoadGraphic, LoadFont, CaptureDrawable, etc.) are available
- C graphics context functions (CreateContext, SetContext, etc.) are available
- C SIS drawing functions (DrawSISFrame, etc.) are available
- All existing comm tests pass

## Requirements Implemented (Expanded)

### REQ-HL-002: Load encounter resources
**Requirement text**: The encounter loop SHALL load alien portrait (AlienFrame), font (AlienFont), colormap (AlienColorMap), song (AlienSong with alt-song fallback), and conversation phrases (ConversationPhrases) using C bridge resource functions.

Behavior contract:
- GIVEN: An encounter is being initialized in `hail_alien()`
- WHEN: The resource loading phase executes
- THEN: C bridge wrappers `c_LoadGraphic`, `c_LoadFont`, `c_LoadColorMap`, `c_LoadMusic`, `c_LoadStringTable` are called to load all required resources, and `c_CaptureDrawable`, `c_CaptureColorMap`, `c_CaptureStringTable` are called to capture reference-counted resources

Why it matters:
- These resource bridges are the foundation for HailAlien — without them, P07 cannot load any encounter data

### REQ-HL-003: Manage graphics contexts
**Requirement text**: The encounter loop SHALL create and manage the AnimContext and TextCacheContext graphics contexts matching C HailAlien behavior.

Behavior contract:
- GIVEN: Resources have been loaded
- WHEN: Context setup phase executes
- THEN: `c_CreateContext`, `c_SetContext`, `c_SetContextFGFrame`, `c_SetContextClipRect`, `c_SetContextBackGroundColor` are called to set up TextCacheContext and AnimContext exactly as C does

Why it matters:
- Graphics contexts control where drawing happens — wrong context = wrong rendering target

### REQ-HL-005: Clean up resources on exit
**Requirement text**: The encounter loop SHALL clean up all loaded resources (destroy drawables, fonts, colormaps, music, string tables, contexts) on exit regardless of exit path (normal, abort, load).

Behavior contract:
- GIVEN: Resources were loaded during encounter initialization
- WHEN: The encounter exits (normal, abort, or load)
- THEN: All resources are destroyed in reverse order via `c_DestroyDrawable`, `c_DestroyFont`, `c_DestroyColorMap`, `c_DestroyMusic`, `c_DestroyStringTable`, `c_DestroyContext`

Why it matters:
- Resource leaks cause memory exhaustion and graphical corruption

## Implementation Tasks

### Files to modify

#### `sc2/src/uqm/rust_comm.c`

All new bridge functions added before the `#endif /* USE_RUST_COMM */` at end of file.

- **Resource Load bridges**
  - `c_LoadGraphic(unsigned int res) -> uintptr_t` — calls `LoadGraphic((RESOURCE)res)`
  - `c_LoadFont(unsigned int res) -> uintptr_t` — calls `LoadFont((RESOURCE)res)`
  - `c_LoadColorMap(unsigned int res) -> uintptr_t` — calls `LoadColorMap((RESOURCE)res)`
  - `c_LoadMusic(unsigned int res) -> uintptr_t` — calls `LoadMusic((RESOURCE)res)`
  - `c_LoadStringTable(unsigned int res) -> uintptr_t` — calls `LoadStringTable((RESOURCE)res)`
  - marker: `@plan PLAN-20260326-COMMPT2.P06`
  - marker: `@requirement REQ-HL-002`

- **Capture/Release bridges**
  - `c_CaptureDrawable(uintptr_t handle) -> uintptr_t` — calls `CaptureDrawable((DRAWABLE)handle)`
  - `c_CaptureColorMap(uintptr_t handle) -> uintptr_t` — calls `CaptureColorMap((COLORMAP)handle)`
  - `c_CaptureStringTable(uintptr_t handle) -> uintptr_t` — calls `CaptureStringTable((STRING_TABLE)handle)`
  - `c_ReleaseDrawable(uintptr_t handle) -> uintptr_t` — calls `ReleaseDrawable((FRAME)handle)`
  - `c_ReleaseColorMap(uintptr_t handle) -> uintptr_t` — calls `ReleaseColorMap((COLORMAP)handle)`
  - `c_ReleaseStringTable(uintptr_t handle) -> uintptr_t` — calls `ReleaseStringTable((STRING_TABLE)handle)`
  - marker: `@plan PLAN-20260326-COMMPT2.P06`
  - marker: `@requirement REQ-HL-002`

- **Context Management bridges**
  - `c_CreateContext(const char *name) -> uintptr_t` — calls `CreateContext(name)`
  - `c_DestroyContext(uintptr_t ctx)` — calls `DestroyContext((CONTEXT)ctx)`
  - `c_SetContext(uintptr_t ctx) -> uintptr_t` — calls `SetContext((CONTEXT)ctx)`, returns old context
  - `c_SetContextFGFrame(uintptr_t frame)` — calls `SetContextFGFrame((FRAME)frame)`
  - `c_SetContextClipRect(int x, int y, int w, int h)` — builds RECT, calls `SetContextClipRect(&r)`
  - `c_ClearContextClipRect()` — calls `SetContextClipRect(NULL)` to clear
  - `c_SetContextBackGroundColor(int r, int g, int b)` — calls with `BUILD_COLOR(MAKE_RGB15(r,g,b), 0x00)`
  - `c_SetContextFont(uintptr_t font) -> uintptr_t` — calls `SetContextFont((FONT)font)`, returns old
  - marker: `@plan PLAN-20260326-COMMPT2.P06`
  - marker: `@requirement REQ-HL-003`

- **Drawable Management bridges**
  - `c_CreateDrawable(unsigned int type, int w, int h, int num_frames) -> uintptr_t`
  - `c_SetFrameTransparentColor(uintptr_t frame, int r, int g, int b)`
  - `c_ClearDrawable()` — calls `ClearDrawable()`
  - `c_GetFrameRect(uintptr_t frame, int *x, int *y, int *w, int *h)` — calls `GetFrameRect` and extracts fields
  - marker: `@plan PLAN-20260326-COMMPT2.P06`
  - marker: `@requirement REQ-HL-003`

- **Graphics Batching bridges**
  - `c_BatchGraphics()` — calls `BatchGraphics()`
  - `c_UnbatchGraphics()` — calls `UnbatchGraphics()`
  - marker: `@plan PLAN-20260326-COMMPT2.P06`

- **Transition bridges**
  - `c_SetTransitionSource(uintptr_t src)` — calls `SetTransitionSource((FRAME)src)` (0 = NULL)
  - `c_ScreenTransition(int num_frames, uintptr_t rect_ptr)` — calls `ScreenTransition(num_frames, (RECT*)rect_ptr)`
  - marker: `@plan PLAN-20260326-COMMPT2.P06`

- **SIS Drawing bridges**
  - `c_DrawSISFrame()` — calls `DrawSISFrame()`
  - `c_DrawSISMessage(const char *msg)` — calls `DrawSISMessage((UNICODE*)msg)` (NULL-safe)
  - `c_DrawSISTitle(const char *title)` — calls `DrawSISTitle((UNICODE*)title)`
  - marker: `@plan PLAN-20260326-COMMPT2.P06`
  - marker: `@requirement REQ-HL-007`

- **DoInput bridge**
  - `c_DoInput(void *state, int exclusive)` — calls `DoInput(state, (BOOLEAN)exclusive)`
  - marker: `@plan PLAN-20260326-COMMPT2.P06`
  - marker: `@requirement REQ-DI-001`

- **Accessor bridges for HailAlien state**
  - `c_GetScreen() -> uintptr_t` — returns `(uintptr_t)Screen`
  - `c_GetSpaceContext() -> uintptr_t` — returns `(uintptr_t)SpaceContext`
  - `c_SetLastActivityCheckLoad()` — sets `LastActivity |= CHECK_LOAD`
  - `c_GetCommDataAlienFrameRes() -> unsigned int` — returns `CommData.AlienFrameRes`
  - `c_GetCommDataAlienFontRes() -> unsigned int` — returns `CommData.AlienFontRes`
  - `c_GetCommDataAlienColorMapRes() -> unsigned int` — returns `CommData.AlienColorMapRes`
  - `c_GetCommDataAlienSongRes() -> unsigned int` — returns `CommData.AlienSongRes`
  - `c_GetCommDataAlienAltSongRes() -> unsigned int` — returns `CommData.AlienAltSongRes`
  - `c_GetCommDataAlienSongFlags() -> unsigned int` — returns `CommData.AlienSongFlags`
  - `c_GetCommDataConversationPhrasesRes() -> unsigned int` — returns `CommData.ConversationPhrasesRes`
  - `c_SetCommDataAlienFrame(uintptr_t frame)` — sets `CommData.AlienFrame = (FRAME)frame`
  - `c_SetCommDataAlienFont(uintptr_t font)` — sets `CommData.AlienFont = (FONT)font`
  - `c_SetCommDataAlienColorMap(uintptr_t cmap)` — sets `CommData.AlienColorMap = (COLORMAP)cmap`
  - `c_SetCommDataAlienSong(uintptr_t song)` — sets `CommData.AlienSong = (MUSIC_REF)song`
  - `c_SetCommDataConversationPhrases(uintptr_t phrases)` — sets `CommData.ConversationPhrases = (STRING)phrases`
  - `c_ClearCommDataConversationPhrasesRes()` — sets `CommData.ConversationPhrasesRes = 0`
  - `c_ClearCommDataConversationPhrases()` — sets `CommData.ConversationPhrases = 0`
  - `c_SetCurInputState(void *state)` — sets `pCurInputState = state`
  - `c_SetTalkingFinished(int finished)` — sets `TalkingFinished = finished`
  - `c_CallInitEncounterFunc()` — calls `(*CommData.init_encounter_func)()`
  - `c_CallPostEncounterFunc()` — calls `(*CommData.post_encounter_func)()`
  - `c_CallUninitEncounterFunc()` — calls `(*CommData.uninit_encounter_func)()`
  - `c_IsStarbaseConversation() -> int` — checks `GET_GAME_STATE(GLOBAL_FLAGS_AND_DATA) == (BYTE)~0 && GET_GAME_STATE(STARBASE_AVAILABLE)`
  - `c_GetGameString(int base, int offset) -> const char *` — returns `GAME_STRING(base + offset)`
  - `c_GetPlanetName() -> const char *` — returns `GLOBAL_SIS(PlanetName)`
  - `c_CheckLoad() -> int` — returns `(GLOBAL(CurrentActivity) & CHECK_LOAD) ? 1 : 0`
  - `c_GetSISScreenWidth() -> int` — returns `SIS_SCREEN_WIDTH`
  - `c_GetSISScreenHeight() -> int` — returns `SIS_SCREEN_HEIGHT`
  - `c_GetSliderY() -> int` — returns `SLIDER_Y`
  - `c_GetSliderHeight() -> int` — returns `SLIDER_HEIGHT`
  - `c_GetSISOrigin(int *x, int *y)` — returns `SIS_ORG_X`, `SIS_ORG_Y`
  - `c_GetPlayerFontRes() -> unsigned int` — returns `PLAYER_FONT` resource ID
  - `c_GetWantPixmap() -> unsigned int` — returns `WANT_PIXMAP`
  - `c_SetupSubtitleTextFromCommData()` — sets `SubtitleText.baseline` and `.align` from CommData
  - `c_ClearPhraseBuf()` — sets `pCurInputState->phrase_buf[0] = '\0'`
  - marker: `@plan PLAN-20260326-COMMPT2.P06`

#### `sc2/src/uqm/rust_comm.h`

- **Add declarations** for all new bridge functions
  - Group by category (load, capture/release, context, drawable, batching, transition, SIS, DoInput, accessors)
  - marker: `@plan PLAN-20260326-COMMPT2.P06`

### Pseudocode traceability
- Uses pseudocode lines: E01–E104 (Resource Bridge Wrappers)

## Verification Commands

```bash
# C compilation with USE_RUST_COMM=on
# (project-specific build command)

# C compilation with USE_RUST_COMM=off (no regressions)
# (project-specific build command)

# Verify all declared functions exist
grep "^[a-z_]*uintptr_t\|^void\|^int\|^unsigned\|^const" sc2/src/uqm/rust_comm.c | wc -l
# Should be significantly more than before

# Verify declarations in header
grep "c_Load\|c_Capture\|c_Release\|c_Create\|c_Destroy\|c_Set\|c_Get\|c_Draw\|c_DoInput\|c_Batch\|c_Unbatch" sc2/src/uqm/rust_comm.h | wc -l

# Rust tests still pass
cargo test --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# No duplicate symbols
# (verify at link time via build)
```

## Structural Verification Checklist
- [ ] All Load bridges created (LoadGraphic, LoadFont, LoadColorMap, LoadMusic, LoadStringTable)
- [ ] All Capture bridges created (CaptureDrawable, CaptureColorMap, CaptureStringTable)
- [ ] All Release bridges created (ReleaseDrawable, ReleaseColorMap, ReleaseStringTable)
- [ ] Context management bridges created (Create, Destroy, Set, SetFGFrame, SetClipRect, SetBGColor, SetFont)
- [ ] Drawable management bridges created (Create, SetFrameTransparentColor, Clear, GetFrameRect)
- [ ] Batching bridges created (BatchGraphics, UnbatchGraphics)
- [ ] Transition bridges created (SetTransitionSource, ScreenTransition)
- [ ] SIS drawing bridges created (DrawSISFrame, DrawSISMessage, DrawSISTitle)
- [ ] DoInput bridge created
- [ ] All CommData accessor bridges created
- [ ] All encounter function call bridges created
- [ ] All declarations present in `rust_comm.h`
- [ ] `@plan` and `@requirement` markers present
- [ ] Both build modes compile and link
- [ ] No duplicate symbols at link time

## Semantic Verification Checklist (Mandatory)
- [ ] Load bridges return valid handles (non-zero for valid resources)
- [ ] Capture bridges properly reference-count resources
- [ ] Release bridges properly dereference resources
- [ ] Context bridges use correct C type casts (CONTEXT, FRAME, FONT, etc.)
- [ ] SetContextClipRect correctly builds RECT from individual x,y,w,h parameters
- [ ] SetContextBackGroundColor correctly builds color from r,g,b via BUILD_COLOR/MAKE_RGB15
- [ ] GetFrameRect correctly extracts all four fields into output parameters
- [ ] DoInput passes state pointer and exclusive flag correctly
- [ ] CommData accessors read from the correct fields
- [ ] CommData setters write to the correct fields
- [ ] Encounter function callers dereference function pointers correctly
- [ ] All bridges are within `#ifdef USE_RUST_COMM` guard
- [ ] No memory ownership confusion (Rust does not free C-allocated memory directly)

## Deferred Implementation Detection (Mandatory)

```bash
# Reject if these appear in new bridge functions:
grep -n "Stub\|TODO\|FIXME\|HACK\|placeholder\|for now" sc2/src/uqm/rust_comm.c
# Should be 0 matches in new bridge code
```

## Success Criteria
- [ ] All ~40 bridge functions compiled and linked
- [ ] Both build modes work
- [ ] All existing tests pass
- [ ] No duplicate/undefined symbols
- [ ] Header declarations match implementations

## Failure Recovery
- Rollback: `git checkout -- sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h`
- If C types not accessible: add necessary `#include` directives
- If function name conflicts: use unique prefixed names (but prefer `c_LoadMusic` for consistency)
- If global variables not accessible: add `extern` declarations or accessor wrappers
- Blocking: all C APIs must be available; check includes

## Phase Completion Marker
Create: `project-plans/20260311/commpt2/.completed/P06.md`

Contents:
- Phase ID: `PLAN-20260326-COMMPT2.P06`
- Timestamp
- Files changed: `sc2/src/uqm/rust_comm.c`, `sc2/src/uqm/rust_comm.h`
- Tests added/updated: (none — C bridges, verified via build)
- Verification outputs
- Semantic verification summary
