# Phase 02: Pseudocode & Module Design

## Plan ID
`PLAN-20260707-RESTARTMENU.P02`

## Module Structure

```
rust/src/mainloop/restart_menu/
├── mod.rs              — module declarations, public API, MainLoopError reuse
├── types.rs            — RestartMenuItem enum, MenuInput struct, SelectionResult enum
├── menu_logic.rs       — pure functions: navigate_up, navigate_down, apply_selection
├── c_extern.rs         — FFI extern declarations for all C-side calls
├── restart_ops.rs      — RestartMenuOps trait (testability abstraction)
├── do_restart.rs       — do_restart_frame impl (InputFunc callback logic)
├── restart_menu.rs     — restart_menu_impl (RestartMenu orchestration)
├── try_start_game.rs   — try_start_game_impl (TryStartGame loop)
├── start_game.rs       — start_game_impl + rust_start_game entry point
└── ffi_bridge.rs       — #[cfg(not(test))] CffiOps impl + C callback export
```

## Domain Types

```rust
/// The five items on the main menu, matching restart.c:45-52.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartMenuItem {
    NewGame = 0,
    LoadGame = 1,
    SuperMelee = 2,
    Setup = 3,
    Quit = 4,
}

impl RestartMenuItem {
    pub const COUNT: u8 = 5;
}

/// Result of processing a menu selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionResult {
    /// Player chose to start/load a game — exit menu, proceed.
    StartGame { new_game: bool },
    /// Player chose Super Melee — exit menu, run Melee.
    SuperMelee,
    /// Player chose Setup — stay in menu after setup completes.
    StayInMenu,
    /// Player chose Quit — exit menu, set CHECK_ABORT.
    Quit,
}

/// Input state snapshot read from C each frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MenuInputState {
    pub select: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub mouse_down: bool,
}
```

## Pure Logic Functions (menu_logic.rs)

### `navigate_up(current: RestartMenuItem) -> RestartMenuItem`

```
MATCH current:
  NewGame => Quit      // wrap from 0 to 4
  other   => decrement // 1→0, 2→1, 3→2, 4→3
```

### `navigate_down(current: RestartMenuItem) -> RestartMenuItem`

```
MATCH current:
  Quit    => NewGame   // wrap from 4 to 0
  other   => increment // 0→1, 1→2, 2→3, 3→4
```

### `apply_selection(item: RestartMenuItem) -> SelectionResult`

```
MATCH item:
  NewGame     => SelectionResult::StartGame { new_game: true }
  LoadGame    => SelectionResult::StartGame { new_game: false }
  SuperMelee  => SelectionResult::SuperMelee
  Setup       => SelectionResult::StayInMenu
  Quit        => SelectionResult::Quit
```

### `check_timeout(now: TimeCount, last_input: TimeCount, timeout: TimeCount) -> bool`

```
// CRITICAL: Use wrapping subtraction — C TimeCount is unsigned and wraps.
// Rust subtraction panics on underflow in debug mode.
RETURN now.wrapping_sub(last_input) > timeout
```

## RestartMenuOps Trait (restart_ops.rs)

All C-side operations abstracted for testability:

```rust
pub trait RestartMenuOps {
    // --- Activity globals ---
    fn get_current_activity(&self) -> ActivityValue;
    fn set_current_activity(&self, activity: ActivityValue);
    fn get_last_activity(&self) -> ActivityValue;
    fn set_last_activity(&self, activity: ActivityValue);
    fn set_next_activity(&self, activity: ActivityValue);

    // --- Input ---
    fn get_menu_input(&self) -> MenuInputState;
    fn get_time_counter(&self) -> u32;

    // --- Game state ---
    fn get_game_state(&self, offset: usize) -> u8;
    fn set_game_state(&self, offset: usize, value: u8);
    fn get_crew_enlisted(&self) -> u16;

    // --- Init/teardown per menu session ---
    fn init_menu_graphics(&self, menu_frame: ...);
    fn draw_menu_graphic(&self);
    fn draw_menu_state(&self, state: u8);
    fn set_menu_sounds(&self, ...);
    fn set_default_menu_repeat_delay(&self);
    fn run_do_input(&self);  // calls C DoInput with Rust callback
    fn cleanup_menu(&self);  // stop music, destroy flash, destroy drawable

    // --- Per-frame operations ---
    fn flash_process(&self);
    fn load_music(&self, ref: u32) -> MusicHandle;
    fn play_music(&self, handle: MusicHandle, loop_: bool, volume: u8);
    fn stop_music(&self);
    fn destroy_music(&self, handle: MusicHandle);
    fn fade_music(&self, volume: u8, duration: u32);
    fn fade_screen(&self, mode: FadeMode, duration: u32) -> u32;
    fn sleep_thread_until(&self, time: u32);
    fn sleep_thread(&self, duration: u32);

    // --- Subsystem calls ---
    fn melee(&self);
    fn setup_menu(&self);
    fn free_game_data(&self);
    fn introduction(&self);
    fn credits(&self, victory: bool);
    fn victory(&self);
    fn do_popup_window(&self, msg: u32);
    fn reinit_race_queues(&self);
    fn seed_random(&self);

    // --- Player control ---
    fn set_player_control(&self, player: u8, control: u16);

    // --- Game paused ---
    fn set_game_paused(&self, val: bool);
}
```

## do_restart_frame Logic (do_restart.rs)

The InputFunc callback. Called once per frame by C's DoInput.

```
FUNCTION do_restart_frame(ops, state) -> bool:
  ops.set_game_paused(false)
  now = ops.get_time_counter()

  IF state.initialized:
    ops.flash_process()

  IF NOT state.initialized:
    // First call: initialize
    IF state.music != 0:
      ops.stop_music()
      ops.destroy_music(state.music)
    state.music = ops.load_music(MAINMENU_MUSIC)
    state.timeout = (state.music != 0 ? 120 : 20) * ONE_SECOND
    ops.create_flash_overlay()
    ops.draw_menu_state(state.item as u8)
    ops.flash_start()
    ops.play_music(state.music, true, 1)
    state.last_input = ops.get_time_counter()
    state.initialized = true
    ops.sleep_thread_until(ops.fade_screen(ToColor, ONE_SECOND/2))
    RETURN true

  ELIF current_activity has CHECK_ABORT:
    RETURN false  // quit

  ELIF input.select:
    result = apply_selection(state.item)
    MATCH result:
      StartGame{new_game}:
        // CRITICAL: LastActivity gets ONLY flag bits, NOT IN_INTERPLANETARY.
        // C: LastActivity = CHECK_LOAD; or LastActivity = CHECK_LOAD | CHECK_RESTART;
        // C: GLOBAL(CurrentActivity) = IN_INTERPLANETARY;  (separate global)
        IF new_game:
          ops.set_last_activity(CHECK_LOAD | CHECK_RESTART)
        ELSE:
          ops.set_last_activity(CHECK_LOAD)
        ops.set_current_activity(IN_INTERPLANETARY)
      SuperMelee:
        ops.set_current_activity(SUPER_MELEE)
      StayInMenu:
        ops.pause_flash()
        ops.fade_flash_in()
        ops.setup_menu()
        // CRITICAL: re-read CurrentActivity after SetupMenu (C can mutate it)
        ops.set_menu_sounds(UP|DOWN, SELECT)
        state.last_input = ops.get_time_counter()
        ops.redraw_menu()
        ops.continue_flash()
        RETURN true  // stay in menu
      Quit:
        ops.sleep_thread_until(ops.fade_screen(ToBlack, ONE_SECOND/2))
        ops.set_current_activity(CHECK_ABORT)
    IF result != StayInMenu:
      ops.pause_flash()
      RETURN false

  ELIF input.up OR input.down:
    new_item = IF input.up: navigate_up(state.item) ELSE navigate_down(state.item)
    IF new_item != state.item:
      ops.batch_graphics()
      ops.draw_menu_state(new_item as u8)
      ops.unbatch_graphics()
      state.item = new_item
    state.last_input = ops.get_time_counter()

  ELIF input.left OR input.right:
    state.last_input = ops.get_time_counter()

  ELIF input.mouse_down:
    ops.show_mouse_not_supported_popup()
    state.last_input = ops.get_time_counter()

  ELSE:
    // No input — check timeout
    IF check_timeout(now, state.last_input, state.timeout):
      ops.fade_out_music()
      ops.set_current_activity(~0)
      RETURN false

  ops.sleep_thread_until(now + ONE_SECOND/30)
  RETURN true
```

## restart_menu_impl Logic (restart_menu.rs)

```
FUNCTION restart_menu_impl(ops, state) -> bool:
  ops.reinit_race_queues()
  ops.set_screen_context()

  // Set CHECK_ABORT during setup
  activity = ops.get_current_activity()
  activity |= CHECK_ABORT
  ops.set_current_activity(activity)

  // Utwig bomb suicide special case
  IF ops.get_crew_enlisted() == 0xFFFF
     AND ops.get_game_state(UTWIG_BOMB_ON_SHIP)
     AND NOT ops.get_game_state(UTWIG_BOMB):
    ops.set_game_state(UTWIG_BOMB_ON_SHIP, 0)
    ops.white_flash_and_clear()
    timeout_dur = ONE_SECOND / 8
  ELSE:
    timeout_dur = ONE_SECOND / 2
    // Victory/credits if last battle was won
    IF ops.get_last_activity().kind() == WonLastBattle:
      ops.set_current_activity(WonLastBattle)
      ops.victory()
      ops.credits(true)
      ops.free_game_data()
      ops.set_current_activity(CHECK_ABORT)

  ops.set_last_activity(0)
  ops.set_next_activity(0)

  ops.sleep_thread_until(ops.fade_screen(ToBlack, timeout_dur))
  IF timeout_dur == ONE_SECOND/8:
    ops.sleep_thread(ONE_SECOND * 3)

  state.frame = ops.load_menu_graphic()
  ops.draw_menu_graphic()

  // Clear CHECK_ABORT
  activity = ops.get_current_activity()
  activity &= ~CHECK_ABORT
  ops.set_current_activity(activity)

  ops.set_menu_sounds(UP|DOWN, SELECT)
  ops.set_default_menu_repeat_delay()

  // ENTER DoInput loop — calls do_restart_frame repeatedly
  ops.run_do_input()

  // Cleanup
  ops.stop_music()
  IF state.music != 0: ops.destroy_music(state.music)
  ops.terminate_flash()
  ops.destroy_menu_graphic()

  // Check exit conditions
  IF ops.get_current_activity() == 0xFFFF: RETURN false  // timeout
  IF ops.get_current_activity() has CHECK_ABORT: RETURN false  // quit

  ops.sleep_thread_until(ops.fade_screen(ToBlack, ONE_SECOND/2))
  ops.flush_color_xforms()
  ops.seed_random()

  RETURN ops.get_current_activity().kind() != SuperMelee
```

## try_start_game_impl Logic (try_start_game.rs)

```
FUNCTION try_start_game_impl(ops) -> bool:
  ops.set_last_activity(ops.get_current_activity())
  ops.set_current_activity(0)

  state = RestartMenuState::new()  // item = NewGame, initialized = false

  WHILE NOT restart_menu_impl(ops, state):
    activity = ops.get_current_activity()
    IF activity.kind() == SuperMelee AND NOT activity.has(CHECK_ABORT):
      ops.free_game_data()
      ops.melee()
      state.initialized = false  // reinit menu for next pass
    ELIF activity == 0xFFFF:
      ops.sleep_thread_until(ops.fade_screen(ToBlack, ONE_SECOND/2))
      RETURN false  // timeout
    ELIF activity.has(CHECK_ABORT):
      RETURN false  // quit

  RETURN true
```

## start_game_impl Logic (start_game.rs)

```
FUNCTION start_game_impl(ops) -> bool:
  DO:
    WHILE NOT try_start_game_impl(ops):
      activity = ops.get_current_activity()
      IF activity == 0xFFFF:  // timeout
        ops.set_current_activity(0)
        ops.splash_screen()
        ops.credits(false)
      IF activity.has(CHECK_ABORT):
        RETURN false  // quit

    IF ops.get_last_activity().has(CHECK_RESTART):
      ops.introduction()

  WHILE ops.get_current_activity().has(CHECK_ABORT)

  ops.assign_global_arrays()
  ops.set_player_control(0, HUMAN_CONTROL | STANDARD_RATING)
  ops.set_player_control(1, COMPUTER_CONTROL | AWESOME_RATING)

  RETURN true
```

## C Bridge Pattern

### C side (rust_bridge_restart.c)

New C wrapper functions for game-state accessors and operations not
already exposed by rust_bridge_mainloop.c:

```c
// Game state byte accessors
BYTE uqm_get_game_state(uint16_t offset);
void uqm_set_game_state(uint16_t offset, BYTE value);
COUNT uqm_get_crew_enlisted(void);

// LastActivity/NextActivity accessors
UWORD uqm_get_last_activity(void);
void uqm_set_last_activity(UWORD val);
void uqm_set_next_activity(UWORD val);

// PlayerControl
void uqm_set_player_control(uint8_t player, COUNT control);

// Input
BOOLEAN uqm_get_pulsed_key(uint8_t key_index);
BOOLEAN uqm_get_mouse_button_down(void);
TimeCount uqm_get_time_counter(void);
void uqm_set_game_paused(BOOLEAN val);

// ... (full list in phase specs)
```

### Rust callback export

The Rust `do_restart_frame` must be callable from C's `DoInput` via the
`InputFunc` function pointer. Two approaches:

**Approach A (chosen): Callback bridge.**
C calls `DoInput(pMS, TRUE)` where `pMS->InputFunc` is set to a C wrapper
function that calls into Rust. The Rust side holds the menu state.

```c
// In rust_bridge_restart.c:
static BOOLEAN rust_restart_input_func(MENU_STATE *pMS) {
    return rust_do_restart_frame();
}

void uqm_run_restart_menu(void) {
    // C sets up MENU_STATE with InputFunc = rust_restart_input_func
    // then calls DoInput, which repeatedly calls back into Rust.
}
```

**Approach B (rejected): Full Rust loop.**
Rust implements its own input pump, bypassing DoInput entirely. Rejected
because DoInput handles sound effects, input flushing, and task yielding
that would all need separate porting.
