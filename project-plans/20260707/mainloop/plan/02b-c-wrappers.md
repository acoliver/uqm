# Phase 02b: C Wrapper Functions for Static/Internal-Linkage Symbols (Revised)

## Phase ID
`PLAN-20260707-MAINLOOP.P02b`

## Prerequisites
- Phase 02a pseudocode verification PASS

## Purpose
C functions the Rust game loop needs that have `static` (internal)
linkage cannot be called from a separate `.c` file. Wrappers MUST be
added **inside the same translation unit** where the static function
lives. Also adds activity/global accessor functions.

**Revision (iteration 3)**: Wrappers go in `starcon.c` (not a new file).
No startup wrapper needed — C `main()` owns startup.

---

## Static Function Wrappers (in starcon.c)

| Static Function | Location | Wrapper (added to starcon.c) |
|-----------------|----------|------------------------------|
| `on_battle_frame` | starcon.c:80 | `uqm_battle_with_frame_callback()` — calls `Battle(&on_battle_frame)` |
| `BackgroundInitKernel` | starcon.c:92 | `uqm_splash_with_bg_init_kernel()` — calls `SplashScreen(BackgroundInitKernel)` |

**Why they must be in starcon.c**: `static` functions have internal
linkage. A new `.c` file cannot reference them. The wrapper functions
are non-static (extern) and are added to `starcon.c` alongside the
static functions they call.

## Macro/Global Wrappers (in rust_bridge_mainloop.c)

Several C constructs used by the game loop are macros, not linkable
symbols, or have ABI issues. These need C wrappers:

| C Construct | Type | Wrapper | Why |
|-------------|------|---------|-----|
| `ZeroVelocityComponents(&GLOBAL(velocity))` | Macro (velocity.h:38) | `uqm_zero_global_velocity()` | Macro = no linkable symbol; needs `&GLOBAL(velocity)` |
| `SetPlayerInputAll()` failure | C99 `bool` + `explode()` | `uqm_set_player_input_all_or_explode()` | Returns C `bool` (not `BOOLEAN`); C calls `explode()` on failure |
| `SetFlashRect(NULL)` | Function taking pointer | `uqm_set_flash_rect_null()` | Correct NULL pointer type |
| `GLOBAL_SIS(CrewEnlisted)` | Macro global | `uqm_get_crew_enlisted()` → u16 | Avoid raw struct access |

## Activity/Global Accessors (in rust_bridge_mainloop.c)

These access C globals via function calls (not raw memory offset from Rust).
They CAN go in a new file because they reference `extern` globals, not
static functions:

| Accessor | C Global | Header |
|----------|----------|--------|
| `get_current_activity()` → u16 | `GlobData.Game_state.CurrentActivity` | globdata.h:930 |
| `set_current_activity(u16)` | same | |
| `get_next_activity()` → u16 | `NextActivity` | save.h:66 |
| `set_next_activity(u16)` | same | |
| `get_last_activity()` → u16 | `LastActivity` | setup.h:60 |
| `set_last_activity(u16)` | same | |
| `set_main_exited(BOOLEAN)` | `MainExited` | |

## Named Game-State Accessors (in rust_bridge_mainloop.c)

Using `GET_GAME_STATE` / `SET_GAME_STATE` macros (bit-packed, not byte offsets):

| Game State | Getter | Setter |
|-----------|--------|--------|
| `CHMMR_BOMB_STATE` | `uqm_get_chmmr_bomb_state()` | `uqm_set_chmmr_bomb_state(BYTE)` |
| `STARBASE_AVAILABLE` | `uqm_get_starbase_available()` | — |
| `GLOBAL_FLAGS_AND_DATA` | `uqm_get_global_flags_and_data()` | — |
| `KOHR_AH_KILLED_ALL` | `uqm_get_kohr_ah_killed_all()` | — |
| `CrewEnlisted` | `uqm_get_crew_enlisted()` → u16 | — | (death detection, starcon.c:295) |

---

## Build System Integration

**Critical**: `rust_bridge_mainloop.c` is a NEW C file and will NOT be
automatically linked into the UQM binary. It must be added to the build:

1. Add to the source list in `sc2/build.vars.in` template (follow the
   pattern used for `rust_bridge_ships.c`, `rust_comm.c`, etc.)
2. The file compiles under all configurations (accessors reference
   `extern` globals, not static functions)
3. Static-function wrappers in `starcon.c` are compiled automatically
   (starcon.c is already in the build)

**Verification after build-system change:**
```bash
cd sc2 && ./build.sh uqm
# Verify ALL wrapper symbols are present
nm sc2/uqm | grep -E 'get_current_activity|get_next_activity|get_last_activity|uqm_get_chmmr_bomb_state|uqm_get_crew_enlisted|uqm_splash_with_bg_init_kernel|uqm_battle_with_frame_callback'
```

### Files to modify (add wrappers to existing C files):
- `sc2/src/uqm/starcon.c` — add `uqm_splash_with_bg_init_kernel()` and
  `uqm_battle_with_frame_callback()` near the static functions they wrap
  - marker: `@plan PLAN-20260707-MAINLOOP.P02b`

### Files to create (accessors — reference extern globals, safe in new file):
- `sc2/src/uqm/rust_bridge_mainloop.c` — all accessor implementations
  - marker: `@plan PLAN-20260707-MAINLOOP.P02b`
- `sc2/src/uqm/rust_bridge_mainloop.h` — prototypes for ALL wrappers

### Example: static-function wrappers in starcon.c:

```c
// === ADDED to starcon.c (near on_battle_frame and BackgroundInitKernel) ===

// Exported wrapper for Rust FFI — calls static BackgroundInitKernel
void
uqm_splash_with_bg_init_kernel (void)
{
    SplashScreen (BackgroundInitKernel);
}

// Exported wrapper for Rust FFI — calls static on_battle_frame
void
uqm_battle_with_frame_callback (void)
{
    Battle (&on_battle_frame);
}
```

### Example: accessors in rust_bridge_mainloop.c:

```c
#include "starcon.h"
#include "globdata.h"
#include "save.h"   // for NextActivity
#include "setup.h"  // for LastActivity

UWORD get_current_activity (void)  { return GLOBAL (CurrentActivity); }
void  set_current_activity (UWORD v) { GLOBAL (CurrentActivity) = v; }

ACTIVITY get_next_activity (void)  { return NextActivity; }
void  set_next_activity (ACTIVITY v) { NextActivity = v; }

ACTIVITY get_last_activity (void)  { return LastActivity; }
void  set_last_activity (ACTIVITY v) { LastActivity = v; }

BYTE uqm_get_chmmr_bomb_state (void)  { return GET_GAME_STATE (CHMMR_BOMB_STATE); }
void uqm_set_chmmr_bomb_state (BYTE v) { SET_GAME_STATE (CHMMR_BOMB_STATE, v); }
BYTE uqm_get_starbase_available (void) { return GET_GAME_STATE (STARBASE_AVAILABLE); }
BYTE uqm_get_global_flags_and_data (void) { return GET_GAME_STATE (GLOBAL_FLAGS_AND_DATA); }
BYTE uqm_get_kohr_ah_killed_all (void) { return GET_GAME_STATE (KOHR_AH_KILLED_ALL); }
```

---

## Verification

```bash
cd sc2 && ./build.sh uqm

# Verify all wrapper symbols are exported
nm sc2/uqm | grep -E 'get_current_activity|set_current_activity|get_next_activity|get_last_activity|set_last_activity|uqm_get_chmmr_bomb_state|uqm_splash_with_bg_init_kernel|uqm_battle_with_frame_callback'
```

All symbols must appear (not absent = static/missing).

## Semantic Verification Checklist
- [ ] Static-function wrappers compile inside starcon.c
- [ ] Activity/global accessors compile in rust_bridge_mainloop.c
- [ ] All 4 named game-state accessors compile
- [ ] NextActivity accessor exists (for load/restart path)
- [ ] No `uqm_rust_safe_startup` (removed — C main() owns startup)
- [ ] `_Static_assert(sizeof(UWORD) == 2)` in header

## Success Criteria
- [ ] All wrapper symbols present in `nm sc2/uqm`
- [ ] Binary builds and runs with original Starcon2Main (USE_RUST_MAINLOOP=0)

## Failure Recovery
- `git restore sc2/src/uqm/starcon.c`
- `rm sc2/src/uqm/rust_bridge_mainloop.c sc2/src/uqm/rust_bridge_mainloop.h`

## Phase Completion Marker
Create: `project-plans/20260707/mainloop/.completed/P02b.md`
