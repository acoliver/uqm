# Phase 02: Pseudocode (Revised — iteration 3)

## Phase ID
`PLAN-20260707-MAINLOOP.P02`

## Revision notes (iteration 3)
- **Architectural simplification**: Rust replaces only Starcon2Main body.
  C main() owns startup and subsystem shutdown. No startup wrapper.
- **Rust shutdown = only starcon.c:313-318** (game-kernel cleanup).
- **Added NextActivity** (save.h:66) for load/restart path.
- **Encounter-only post-dispatch**: flag mutation only for encounter branch.
- **Combined win/loss/death condition**: one `if` for WON_LAST_BATTLE OR death.
- **Fixed LastActivity**: standalone global (setup.h:60), not GlobData.

---

## Component A: `rust_game_loop` — Rust Game Loop Body (REQ-ML-001, REQ-ML-007)

Replaces the `Starcon2Main()` body. Runs on the Starcon2Main thread.
C `main()` has already completed startup before calling this.

```text
 1: FUNCTION rust_game_loop() -> c_int
 2:   LET result = run_game_lifecycle()
 3:   MATCH result
 4:     Ok(_)  => RETURN 0
 5:     Err(e) => log_fatal(e); RETURN EXIT_FAILURE
 6:   END MATCH
 7: END FUNCTION
 8:
 9: FUNCTION run_game_lifecycle() -> Result<(), MainLoopError>
10:   // --- INIT (Starcon2Main-specific init only; C main() already did startup) ---
11:   init_audio()?                        // FFI: initAudio(snddriver, soundflags)
12:   IF NOT load_kernel()                 // FFI: LoadKernel(0, 0) -> CBoolean
13:     set_main_exited(true)              // tell C main() to start shutdown
14:     RETURN Err(LoadKernelFailed)
15:   END IF
16:   // CRITICAL: clear CurrentActivity before splash (starcon.c:205)
17:   // BackgroundInitKernel loop checks CurrentActivity & CHECK_ABORT
18:   set_current_activity(ActivityValue(0))
19:   // SplashScreen uses static callback — wrapper is IN starcon.c
20:   uqm_splash_with_bg_init_kernel()     // FFI: wrapper in starcon.c (P02b)
21:
22:   // Outer loop: new game / load game
23:   WHILE start_game()                   // FFI: StartGame() -> CBoolean
24:     // CRITICAL: SetPlayerInputAll failure calls explode() (does not return)
25:     set_player_input_all()             // FFI: SetPlayerInputAll()
26:     init_game_structures()             // FFI: InitGameStructures()
27:     init_game_clock()                  // FFI: InitGameClock()
28:     add_initial_game_events()          // FFI: AddInitialGameEvents()
29:
30:     // Inner loop: activity state machine
31:     LOOP
32:       // CRITICAL: SetStatusMessageMode BEFORE load/velocity (starcon.c:235)
33:       set_status_message_mode(DEFAULT) // FFI: SetStatusMessageMode(SMM_DEFAULT)
34:
35:
35a:       // DEBUG hook (starcon.c:223-233): #ifdef DEBUG, if debugHook != NULL,
35a:       // call it and `continue` (skip normal dispatch for this tick).
35a:       // Scope: USE_RUST_MAINLOOP is NOT enabled for DEBUG builds until
35a:       // debugHook is ported. This is a known limitation.
35:
35:       LET activity = get_current_activity()  // FFI accessor
36:
37:       // Load path: check CurrentActivity | NextActivity for CHECK_LOAD
38:       IF NOT ((activity | get_next_activity()).has_flag(CHECK_LOAD))
39:         zero_velocity_components()      // FFI: ZeroVelocityComponents
40:       ELSE IF activity.has_flag(CHECK_LOAD)
41:         // Replace CurrentActivity with NextActivity
42:         set_current_activity(get_next_activity())
43:         activity = get_current_activity()    // RE-READ
44:       END IF

44:       // Evaluate activity and dispatch to C
45:       LET decision = ActivityStateMachine::evaluate(activity)  // Component B
46:       LET was_encounter = decision.is_encounter_branch()
47:       execute_activity(decision)        // dispatches to C via FFI (Component C)
48:
45:
44:       // CRITICAL: re-read CurrentActivity — C dispatch mutated it
45:       activity = get_current_activity()
46:
47:       // Post-dispatch flag mutation — ENCOUNTER BRANCH ONLY (starcon.c:263-268)
48:       IF was_encounter AND NOT activity.has_flag(CHECK_ABORT | CHECK_LOAD)
49:         activity.clear_flag(START_ENCOUNTER)
50:         IF activity.kind() == InInterplanetary
51:           activity.set_flag(START_INTERPLANETARY)
52:         END IF
53:         set_current_activity(activity)
54:       END IF
55:
56:       set_flash_rect(null)             // FFI: SetFlashRect(NULL)
57:
58:       // CRITICAL: set LastActivity from re-read value (setup.h:60 global)
59:       // Re-read in case post-dispatch mutation changed it
60:       set_last_activity(get_current_activity())
61:
62:       // Win/loss/death check — COMBINED condition (starcon.c:292-303)
63:       IF should_stop_loop()             // Component D — re-reads activity internally
64:         BREAK
65:       END IF
66:     END LOOP until get_current_activity().has_flag(CHECK_ABORT)
67:
68:     stop_sound()                       // FFI: StopSound()
69:     uninit_game_clock()                // FFI: UninitGameClock()
70:     uninit_game_structures()           // FFI: UninitGameStructures()
71:     clear_player_input_all()           // FFI: ClearPlayerInputAll()
72:   END WHILE
73:
74:   // --- GAME-KERNEL CLEANUP ONLY (starcon.c:313-318) ---
75:   // Subsystem teardown is done by C main() after MainExited (uqm.c:479-507)
76:   shutdown_game_kernel()               // Component E, lines 86-91
77:
78:   RETURN Ok(())
79: END FUNCTION
```

## Component B: `ActivityStateMachine::evaluate` — Activity Dispatch (REQ-ML-004)

```text
70: FUNCTION ActivityStateMachine::evaluate(activity: ActivityValue) -> ActivityDecision
71:   // Named accessors, NOT byte offsets — game state is bit-packed
72:   LET bomb_state = uqm_get_chmmr_bomb_state()     // FFI: named C wrapper
73:   LET starbase_avail = uqm_get_starbase_available()
74:   LET global_flags = uqm_get_global_flags_and_data()
75:
76:   IF activity.has_flag(START_ENCOUNTER) OR bomb_state == 2
77:     IF bomb_state == 2 AND NOT starbase_avail
78:       RETURN Decision::InstallBombAtEarth
79:     ELSE IF global_flags == 0xFF OR bomb_state == 2
80:       RETURN Decision::VisitStarBase
81:     ELSE
82:       RETURN Decision::RaceCommunication
83:     END IF
84:   ELSE IF activity.has_flag(START_INTERPLANETARY)
85:     RETURN Decision::ExploreSolarSystem
86:   ELSE
87:     RETURN Decision::Battle
88:   END IF
89: END FUNCTION
90:
91: FUNCTION ActivityDecision::is_encounter_branch() -> bool
92:   MATCH self
93:     InstallBombAtEarth | VisitStarBase | RaceCommunication => true
94:     _ => false
95:   END MATCH
96: END FUNCTION
```

## Component C: `execute_activity` — Dispatch to C (via wrappers for callbacks)

```text
100: FUNCTION execute_activity(decision: ActivityDecision)
101:   MATCH decision
102:     InstallBombAtEarth => install_bomb_at_earth()     // FFI
103:     VisitStarBase =>
104:       // CRITICAL: C sets START_ENCOUNTER before VisitStarBase (starcon.c:254)
105:       set_current_activity(get_current_activity().set_flag(START_ENCOUNTER))
106:       visit_starbase()             // FFI
107:     RaceCommunication =>
108:       // CRITICAL: C sets START_ENCOUNTER before RaceCommunication (starcon.c:259)
109:       set_current_activity(get_current_activity().set_flag(START_ENCOUNTER))
110:       race_communication()         // FFI
111:     ExploreSolarSystem =>
106:       set_current_activity(make_word(IN_INTERPLANETARY, 0))
107:       draw_autopilot_message(true)
108:       set_game_clock_rate(INTERPLANETARY_CLOCK_RATE)
109:       explore_solar_sys()                              // FFI
110:     Battle =>
111:       set_current_activity(make_word(IN_HYPERSPACE, 0))
112:       draw_autopilot_message(true)
113:       set_game_clock_rate(HYPERSPACE_CLOCK_RATE)
114:       // Battle uses static callback — wrapper is IN starcon.c
115:       uqm_battle_with_frame_callback()                 // FFI: wrapper in starcon.c
116:   END MATCH
117: END FUNCTION
```

## Component D: `should_stop_loop` — Combined Win/Loss/Death Check

Matches starcon.c:292-303 exactly: one combined condition for
WON_LAST_BATTLE OR player death.

```text
120: FUNCTION should_stop_loop() -> bool
121:   LET activity = get_current_activity()   // fresh read
122:   IF NOT activity.has_flag(CHECK_ABORT | CHECK_LOAD)
123:     IF activity.kind() == WON_LAST_BATTLE OR crew_enlisted() == 0xFFFF
124:       IF uqm_get_kohr_ah_killed_all()
125:         init_communication(BLACKURQ_CONVERSATION)    // FFI
126:       ELSE IF activity.has_flag(CHECK_RESTART)
127:         activity.clear_flag(CHECK_RESTART)
128:         set_current_activity(activity)
129:       END IF
130:       RETURN true
131:     END IF
132:   END IF
133:   RETURN false
134: END FUNCTION
```

## Component E: Game-Kernel Cleanup ONLY (starcon.c:313-318) (REQ-ML-008)

```text
86: FUNCTION shutdown_game_kernel()
87:   uninit_game_kernel()         // FFI: UninitGameKernel
88:   free_master_ship_list()      // FFI: FreeMasterShipList
89:   free_kernel()                // FFI: FreeKernel
90:   log_show_box(false, false)   // FFI: log_showBox
91:   set_main_exited(true)        // FFI: MainExited = TRUE
92: END FUNCTION
93: // NOTE: C main() does the rest (uqm.c:479-507) after seeing MainExited.
94: // Rust does NOT call subsystem teardown — that would double-free.
```

---

## Pseudocode Verification Points

| Lines | Requirement | What it proves |
|-------|-------------|----------------|
| 1-7 | REQ-ML-001 | `rust_game_loop` entry, returns c_int |
| 9-79 | REQ-ML-007 | Outer (StartGame) + inner (loop-until-CHECK_ABORT) |
| 11-17 | — | Starcon2Main-specific init only (C main() did startup) |
| 28-37 | — | NextActivity load path (starcon.c:237-241) |
| 40-42 | REQ-ML-004 | State machine evaluates dispatch |
| 41 | — | was_encounter flag for encounter-only post-dispatch |
| 44-45 | REQ-ML-003 | CurrentActivity re-read after dispatch |
| 48-54 | — | Encounter-only post-dispatch mutation (starcon.c:263-268) |
| 60 | — | LastActivity from fresh CurrentActivity (setup.h:60) |
| 63-65 | — | Combined win/loss/death check (starcon.c:292-303) |
| 66 | — | Inner loop condition re-reads CurrentActivity |
| 72-78 | — | Game-kernel cleanup only; C main() does subsystem shutdown |
| 86-94 | REQ-ML-008 | Game-kernel cleanup = starcon.c:313-318 only |
