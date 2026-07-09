# Phase 01: Domain Analysis

## Plan ID
`PLAN-20260707-RESTARTMENU.P01`

## C Source Files Analyzed

### restart.c (413 lines)

Six functions, three call layers:

```
StartGame()          (line 373, exported, BOOLEAN return)
└─ TryStartGame()    (line 339, static)
   └─ RestartMenu()  (line 252, static)
      └─ DoInput()   (gameinp.c:361 — generic input pump)
         └─ DoRestart() (line 99, static — InputFunc callback)
```

### Function-by-function analysis

#### `StartGame()` (restart.c:373-412)

**Signature:** `BOOLEAN StartGame(void)`

**Behavior:**
1. Outer `do { } while (CurrentActivity & CHECK_ABORT)` loop
2. Inner `while (!TryStartGame())` loop:
   - If `CurrentActivity == (ACTIVITY)~0` (timeout): reset activity, call `SplashScreen(0)`, `Credits(FALSE)`
   - If `CHECK_ABORT`: return FALSE (quit)
3. After TryStartGame succeeds:
   - If `LastActivity & CHECK_RESTART`: call `Introduction()` (intro video)
4. After the do-while exits (no CHECK_ABORT):
   - Assign global arrays: `star_array = starmap_array`, `Elements = element_array`, `PlanData = planet_array`
   - Set `PlayerControl[0] = HUMAN_CONTROL | STANDARD_RATING`
   - Set `PlayerControl[1] = COMPUTER_CONTROL | AWESOME_RATING`
5. Return TRUE

**Global state read:** `CurrentActivity`, `LastActivity`
**Global state written:** `CurrentActivity`, `LastActivity`, `star_array`, `Elements`, `PlanData`, `PlayerControl[0]`, `PlayerControl[1]`

#### `TryStartGame()` (restart.c:339-371)

**Signature:** `static BOOLEAN TryStartGame(void)`

**Behavior:**
1. Save `LastActivity = CurrentActivity`, clear `CurrentActivity = 0`
2. Zero-initialize `MENU_STATE`, set `InputFunc = DoRestart`
3. `while (!RestartMenu(&MenuState))` loop:
   - If `CurrentActivity` low byte == `SUPER_MELEE` and not `CHECK_ABORT`: `FreeGameData()`, `Melee()`, reset `Initialized`
   - If `CurrentActivity == ~0`: fade to black, return FALSE (timeout)
   - If `CHECK_ABORT`: return FALSE (quit)
4. Return TRUE

**Global state read:** `CurrentActivity`, `LastActivity`
**Global state written:** `LastActivity`, `CurrentActivity`
**C functions called:** `RestartMenu`, `FreeGameData`, `Melee`, `FadeScreen`

#### `RestartMenu()` (restart.c:252-337)

**Signature:** `static BOOLEAN RestartMenu(MENU_STATE *pMS)`

**Behavior:**
1. `ReinitQueue(&race_q[0])`, `ReinitQueue(&race_q[1])` — clear battle queues
2. `SetContext(ScreenContext)` — set graphics context
3. Set `CHECK_ABORT` on `CurrentActivity`
4. Special case: Utwig bomb suicide (`CrewEnlisted == ~0 && UTWIG_BOMB_ON_SHIP && !UTWIG_BOMB`):
   - Clear bomb state, white flash, clear drawable
   - `TimeOut = ONE_SECOND / 8`
5. Normal case: `TimeOut = ONE_SECOND / 2`
   - If `LastActivity` low byte == `WON_LAST_BATTLE`: `Victory()`, `Credits(TRUE)`, `FreeGameData()`, reset activity to `CHECK_ABORT`
6. `LastActivity = 0`, `NextActivity = 0`
7. Fade to black over `TimeOut`
8. Load restart menu graphic: `pMS->CurFrame = CaptureDrawable(LoadGraphic(RESTART_PMAP_ANIM))`
9. `DrawRestartMenuGraphic(pMS)` — draw the background
10. Clear `CHECK_ABORT` from `CurrentActivity`
11. `SetMenuSounds(MENU_SOUND_UP | MENU_SOUND_DOWN, MENU_SOUND_SELECT)`
12. `SetDefaultMenuRepeatDelay()`
13. **`DoInput(pMS, TRUE)`** — enters the input pump loop, which repeatedly calls `DoRestart` until it returns FALSE
14. Cleanup: StopMusic, DestroyMusic, Flash_terminate, DestroyDrawable
15. Return FALSE if timed out (`CurrentActivity == ~0`) or quit (`CHECK_ABORT`)
16. Return `LOBYTE(CurrentActivity) != SUPER_MELEE`

**Global state read:** `CurrentActivity`, `LastActivity`, `CrewEnlisted`, `UTWIG_BOMB_ON_SHIP`, `UTWIG_BOMB`
**Global state written:** `CurrentActivity`, `LastActivity`, `NextActivity`, `UTWIG_BOMB_ON_SHIP`
**C functions called:** `ReinitQueue`, `SetContext`, `FadeScreen`, `Victory`, `Credits`, `FreeGameData`, `CaptureDrawable`, `LoadGraphic`, `DrawRestartMenuGraphic`, `SetMenuSounds`, `SetDefaultMenuRepeatDelay`, `DoInput`, `StopMusic`, `DestroyMusic`, `Flash_terminate`, `DestroyDrawable`, `ReleaseDrawable`, `FlushColorXForms`, `SeedRandomNumbers`, `FadeScreen`

#### `DoRestart()` (restart.c:99-250) — THE InputFunc callback

**Signature:** `static BOOLEAN DoRestart(MENU_STATE *pMS)`

This is the per-frame handler called by `DoInput()`. Returns TRUE to continue the menu loop, FALSE to exit.

**Behavior (first call — initialization):**
1. `GamePaused = FALSE`
2. Load menu music: `pMS->hMusic = LoadMusic(MAINMENU_MUSIC)`
3. `InactTimeOut = (hMusic ? 120 : 20) * ONE_SECOND`
4. Create flash overlay with fade-in
5. `DrawRestartMenu(pMS, CurState, CurFrame)` — set initial menu frame
6. `Flash_start(flashContext)`, `PlayMusic(hMusic, TRUE, 1)`
7. `LastInputTime = GetTimeCounter()`
8. `pMS->Initialized = TRUE`
9. `SleepThreadUntil(FadeScreen(FadeAllToColor, ONE_SECOND/2))`

**Behavior (subsequent calls):**
- If `CHECK_ABORT`: return FALSE (quit)
- If `KEY_MENU_SELECT`: switch on `CurState`:
  - `LOAD_SAVED_GAME`: `LastActivity = CHECK_LOAD`, `CurrentActivity = IN_INTERPLANETARY`
  - `START_NEW_GAME`: `LastActivity = CHECK_LOAD | CHECK_RESTART`, `CurrentActivity = IN_INTERPLANETARY`
  - `PLAY_SUPER_MELEE`: `CurrentActivity = SUPER_MELEE`
  - `SETUP_GAME`: call `SetupMenu()`, redraw, return TRUE (stay in menu)
  - `QUIT_GAME`: fade to black, `CurrentActivity = CHECK_ABORT`
  - After any non-SETUP selection: `Flash_pause()`, return FALSE
- If `KEY_MENU_UP`: wrap from `START_NEW_GAME(0)` → `QUIT_GAME(4)`, otherwise decrement
- If `KEY_MENU_DOWN`: wrap from `QUIT_GAME(4)` → `START_NEW_GAME(0)`, otherwise increment
- If `KEY_MENU_LEFT/RIGHT`: update `LastInputTime` only (no-op)
- If `MouseButtonDown`: show "mouse not supported" popup
- No input: check timeout (`GetTimeCounter() - LastInputTime > InactTimeOut`)
  - Timeout: fade music, `CurrentActivity = ~0`, return FALSE
- Always: `SleepThreadUntil(TimeIn + ONE_SECOND/30)`, return TRUE

**Menu state enum (restart.c:45-52):**
```c
START_NEW_GAME = 0,
LOAD_SAVED_GAME = 1,
PLAY_SUPER_MELEE = 2,
SETUP_GAME = 3,
QUIT_GAME = 4
```

**Global state read:** `CurrentActivity`, `PulsedInputState`, `MouseButtonDown`, `GamePaused`
**Global state written:** `CurrentActivity`, `LastActivity`, `GamePaused`
**Static variables:** `LastInputTime`, `InactTimeOut` (persist across calls)

### menu.c (603 lines)

Generic menu navigation functions used game-wide. NOT restart-specific.

**Functions:**
- `NextMenuState(BaseState, CurState)` — advance to next enabled menu item
- `PreviousMenuState(BaseState, CurState)` — go to previous enabled item
- `GetEndMenuState(BaseState)` — find last enabled item
- `GetBeginMenuState(BaseState)` — find first enabled item
- `FixMenuState(BadState)` — remap disabled states to enabled alternatives
- `GetAlternateMenu(BaseState, CurState)` — check for alternate menu set
- `ConvertAlternateMenu(BaseState, NewState)` — convert to alternate menu numbering
- `DrawPCMenu(...)`, `DrawPCMenuFrame(...)` — PC-style menu rendering

**Decision:** These are game-wide utilities, not restart-specific. **Deferred** to a future plan. The restart menu (`DoRestart`) implements its own simple up/down wrap-around and does NOT use `menu.c` functions.

### DoInput() (gameinp.c:361-412)

**Signature:** `void DoInput(void *pInputState, BOOLEAN resetInput)`

Generic input pump. Repeatedly:
1. `Async_process()`, `TaskSwitch()` — yield to other threads
2. `UpdateInputState()` — read SDL input into `PulsedInputState`/`ImmediateInputState`
3. `MenuKeysToSoundFlags()` — play menu navigation sounds
4. Call `pInputState->InputFunc(pInputState)` — the per-frame callback
5. Continue while InputFunc returns TRUE

**Decision:** Keep `DoInput` in C. It handles thread yielding, input flushing,
sound effects, and pause/exit side effects. Specifically:
- `Async_process()` — processes deferred command queue (graphics)
- `TaskSwitch()` — cooperative thread yield
- `UpdateInputState()` — reads SDL input into `PulsedInputState`/`ImmediateInputState`
- `MenuKeysToSoundFlags()` — plays menu navigation sounds based on input
- `inputCallback()` — optional callback (set by some menus)
- Loop continues while `InputFunc(pInputState)` returns TRUE
- After loop: `FlushInput()` if `resetInput` was true

The Rust port provides the `InputFunc` callback via a C-callable wrapper
that reads `MENU_STATE.privData` to access Rust state.

## Global State Crossings

| Global | Type | Read by | Written by | Rust representation |
|--------|------|---------|------------|---------------------|
| `CurrentActivity` | `ACTIVITY` (u16) | All | All | `ActivityValue` (existing in mainloop) |
| `LastActivity` | `ACTIVITY` (u16) | StartGame, TryStartGame | DoRestart, TryStartGame, RestartMenu | FFI accessor |
| `NextActivity` | `ACTIVITY` (u16) | (indirectly) | RestartMenu | FFI accessor |
| `GamePaused` | `BOOLEAN` | (game-wide) | DoRestart | FFI accessor |
| `PulsedInputState` | struct | DoRestart | DoInput | FFI accessor (key state) |
| `MouseButtonDown` | `BOOLEAN` | DoRestart | SDL | FFI accessor |
| `PlayerControl[0..1]` | `COUNT` | (game-wide) | StartGame | FFI accessor |
| `CrewEnlisted` | `COUNT` | RestartMenu | (game) | FFI accessor |
| Game state bytes | `BYTE` | RestartMenu | RestartMenu | FFI accessor (GET/SET_GAME_STATE) |

## Pure Logic vs FFI Calls

### Pure logic (extractable to testable Rust functions):
1. **Menu navigation wrap-around:** up/down between START_NEW_GAME(0) and QUIT_GAME(4)
2. **Selection dispatch:** switch on CurState to produce a SelectionResult
3. **Timeout check:** `now.wrapping_sub(last_input) > timeout` (unsigned wraparound)

### Extractable reducer decisions (orchestration behind traits, NOT "pure"):
4. **TryStartGame loop logic:** while loop branching on CurrentActivity — depends
   on post-call re-reads of C globals (CurrentActivity may change after Melee)
5. **StartGame outer loop logic:** do-while branching — depends on post-call
   re-reads of CurrentActivity and LastActivity

### FFI calls (must stay as C bridges):
1. Graphics: `SetContext`, `BatchGraphics`, `ClearDrawable`, `DrawStamp`, `font_DrawText`, `ScreenTransition`, `FadeScreen`
2. Flash: `Flash_createOverlay`, `Flash_process`, `Flash_pause`, `Flash_continue`, `Flash_start`, `Flash_terminate`, `Flash_setMergeFactors`, `Flash_setSpeed`, `Flash_setFrameTime`, `Flash_setState`, `Flash_setOverlay`
3. Music: `LoadMusic`, `PlayMusic`, `StopMusic`, `DestroyMusic`, `FadeMusic`
4. Resources: `CaptureDrawable`, `LoadGraphic`, `DestroyDrawable`, `ReleaseDrawable`, `SetAbsFrameIndex`
5. Input: `PulsedInputState`, `MouseButtonDown`, `FlushInput`, `SetMenuSounds`, `SetDefaultMenuRepeatDelay`
6. Game state: `GET_GAME_STATE`, `SET_GAME_STATE`, `GLOBAL(CurrentActivity)`, `GLOBAL_SIS(CrewEnlisted)`
7. Other: `Melee`, `SetupMenu`, `FreeGameData`, `Introduction`, `Credits`, `Victory`, `DoPopupWindow`, `SleepThreadUntil`, `SleepThread`, `SeedRandomNumbers`, `ReinitQueue`, `SetContext`, `SetTransitionSource`
