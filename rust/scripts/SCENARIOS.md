# Runtime automation paths

The native UI suite that runs these scenarios is a local gate on a real Mac window (`native run/validate/report/gallery/replay`). Hosted GitHub runners do not run it.

Automation can either play through normal game state or, for tests that do not
need navigation coverage, activate a deterministic start scene.

## Real Sol probe proof

`real-sol-autopilot.json` is the authoritative regression proof for the opening
Ur-Quan probe encounter. It does not inject a scene or force encounter state.
It:

1. Selects New Game through the normal main-menu input path.
2. Confirms interplanetary activity.
3. Uses `navigate_to_planet` to steer player one via production thrust/left/right controls.
4. Observes the real outer-to-inner transition for Earth (planet index 2).
5. Waits for the naturally spawned race-23 probe to intercept the flagship.
6. Uses `assert_dispatch` to require encounter conversation 24 (`URQUAN_DRONE`)
   and dialogue conversation 18 (`URQUAN`).
7. Captures the rendered conversation.

Build from a clean full-commit checkout, then run through the exact-child
supervisor from the repository root:

```sh
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- production
cargo run --locked --manifest-path rust/Cargo.toml \
  --no-default-features --features audio_heart,debug-process \
  --bin uqm-gameplay-proof -- run \
  . rust/target/production-artifacts.json \
  rust/scripts/real-sol-autopilot.json /tmp/uqm-real-sol-autopilot
cargo run --locked --manifest-path rust/Cargo.toml \
  --no-default-features --features audio_heart,debug-process \
  --bin uqm-gameplay-proof -- validate \
  /tmp/uqm-real-sol-autopilot/lcar-v1.json
```

The supervisor launches the exact production executable recorded by the clean
artifact manifest, drains bounded stdout/stderr concurrently, records the
PID/start/executable identity, reaps only that child, verifies no matching
orphan remains, and retains content-addressed LCAR evidence. The offline
validator rehashes the executable, script, content/config inputs, trace,
captures, logs, and teardown receipt.

`battle-v1.json` is the production battle proof. It enters Super Melee through
the normal menu path, asserts the full activity word has the `SUPER_MELEE` base
and `IN_BATTLE` flag, selects the human combatant through player-one weapon
input, waits for real combat, captures the exact composited frame committed by
the renderer, and requires at least 30 monotonic observations from the real
production battle loop. The observer counts completed battle-loop frames; it
does not claim combat elapsed time, animation frames, or unique pixel output.

Successful trace evidence includes:

```text
navigation_reached:planet=2
dispatch_verified:encounter=24:dialogue=18
```

The output capture is `captures/real-probe-encounter.png`.

## Fuel preparation for long hyperspace flights

`probe-encounter-v3` and `probe-encounter-v4` thrust for more hyperspace
distance than the opening tank (1000 raw fuel) can pay for, so they prepare the
ship before the original flight sequence, using only the inputs a player has:

1. `assert_ship_resources` requires the new-game state, 1000 raw fuel and 0 RU.
2. The opening probe broadcast, then a Luna landing on the moon base node. The
   lander picks the node up, which sets `MOONBASE_ON_SHIP`.
3. A first Starbase visit (the unallied Commander conversation), then a second
   visit. With the moon base on board the Commander pays 2500 RU through
   `DeltaSISGauges`, and `assert_ship_resources` requires 970 raw fuel and
   2500 RU once the Starbase menu is up. The 30 raw fuel are the one Luna
   landing that precedes it: `DispatchLander` pays
   `min(SurfaceGravity << 1, 3 * FUEL_TANK_SCALE)`, Luna's gravity is
   60 * 25 / 100 = 15, so the cost is 30. No hyperspace flight happens before
   this check.
   The second visit's dispatch only starts the conversation; the Commander's
   callback registers its response choices afterwards, while the greeting plays.
   `wait_for_communication_responses` (minimum 2, at most 1000 ticks) waits for a
   response list of the current conversation, newer than any list already
   selected, with that many choices. Only then does the unchanged
   `assert_communication_responses` (minimum 2) run. The wait never selects or
   skips speech, and a timeout ends the run with a trace record naming the
   observed state.
4. In the Outfit screen, 50 presses of Up each buy 100 raw fuel for 20 RU.
   `assert_ship_resources` requires 6000 raw fuel (reserve 1000 plus the
   5000 tank) and 1500 RU. Entering Outfit first rounds the 970 raw fuel to
   the nearest whole unit, 1000, so the 50 purchases end at
   1000 + 50 * 100 = 6000 and 2500 - 50 * 20 = 1500.
5. Depart, then require the same state again before the flight starts.

`assert_ship_resources` reads `GLOBAL_SIS(FuelOnBoard)` and
`GLOBAL_SIS(ResUnits)`. It only reads: no script action grants fuel or RU, and
a mismatch ends the run with a `SemanticMismatch` trace record that names the
observed and the expected values.

Limits of this preparation:

- Only `probe-encounter-v3` and `probe-encounter-v4` carry the preparation.
  `probe-encounter-v1`, `-v2`, `-v5`, `-v6` and `-v7` still start from the
  opening 1000 raw fuel, which pays for about 1100 thrust ticks (0.009 HUD
  fuel per tick, derived from one real-window run). Their total thrust is
  capped at 800 ticks, see "Hyperspace flight legs" below.
- The preparation has not run live. The Moon base pickup, the 2500 RU reward
  and the 50 Outfit purchases are checked only by unit tests of the pure
  transaction model and by the script invariants in
  `tests/probe_encounter_flight_input.rs`. The first live run decides whether
  the `assert_ship_resources` expectations hold.
- Shipyard, module installation and Save/Load from Outfit are not ported and
  end the game loop with a typed `Unported` error. The scripts never select
  them.
- `repro-comm-menu-initial` starts in the Commander conversation and finishes
  inside it, so it never reaches the Starbase menu. It is unchanged.

## Hyperspace flight legs

Every `probe-encounter-v1` to `v7` thrust leg is a `tap_player_key` thrust
bracketed by `record_hyperspace_position` immediately before it and
`assert_hyperspace_displacement` (minimum 16 logical units) immediately after
it. A leg that does not move the ship fails the run; no later leg can hide it.
A real-window run of v3 showed why: the old 2000-tick legs and multi-thousand
tick turn holds drove the ship into the universe edge (x=3, HUD 000.0), and the
remaining legs passed while the ship sat still. Without fuel the flagship also
cannot thrust (`sis_ship.c`), so a leg can fail for an empty tank.

Turns are verified, not timed. A fixed key hold cannot say how far the ship
turned: the first real-window run of v1 turned the flagship 112.5 degrees (five
of the sixteen facings) per 60 tick hold, so the tick-to-frame assumption was
wrong. Every turn is now a `turn_flagship` step:

```json
{"action": "turn_flagship", "direction": "right", "facings": 4, "max_ticks": 120}
```

It holds the turn key through the normal player input path (the same
`key[PlayerControls[0]]` writes as `navigate_to_hyperspace`) and releases it
when the live `STARSHIP.ShipFacing` of the hyperspace flagship has changed by
exactly `facings` (mod 16) in the stated direction. The facing is read with
`savegame::runtime::hyper_save::read_flagship_facing`, the same walk of
`race_q[RPG_PLAYER_NUM]` and `disp_q` that `SaveSisHyperState` uses, and
nothing is written. The step fails when the ship turns past `facings`
(`flagship_turn_overshot`), when the game is not in hyperspace or reports no
facing, and when `max_ticks` input ticks pass first. A right angle is four
facings. Success is recorded as
`flagship_turn_verified:direction=right:facings=4:from=A:to=B`.

Routes (leg = thrust ticks, every turn a verified four facing `right` `turn_flagship` after the leg):

| Script | Legs | Turns | Fuel at start |
| --- | --- | --- | --- |
| v1 | 4 x 100 | 3, three right angles | 1000 raw |
| v2 | 4 x 80 | 3, three right angles | 1000 raw |
| v3 | 4 x 500 | 3, three right angles | 6000 raw |
| v4 | 4 x 550 | 3, three right angles | 6000 raw |
| v5 | 4 x 50 | 3, three right angles | 1000 raw |
| v6 | 3 x 200 | 2, five flagship menu visits | 1000 raw |
| v7 | 3 x 200 | 2, five Game menu visits | 1000 raw |

The start heading is whatever the solar-system exit leaves. Sol is about
27 800 logical units from the nearest universe edges and the flagship covers
about 12 per thrust tick, so each script's total thrust is capped (800 ticks on
the opening tank, 2200 on the prepared tank) such that even a straight run
cannot reach an edge or empty the tank. `tests/probe_encounter_flight_input.rs`
enforces the bracketing, that every turn is a four facing `turn_flagship`
(bounded by 120 ticks) and the caps.

v1 to v5 fly one square: four legs of one length with three right turns
between them. The turns are proven; the closing of the square is geometry from
equal legs and exact right angles. A real-window run of v3 flew from Sol's
hyperspace point 27987,164141 through 23747,168357, 19475,164157 and
23655,159881 to 27911,164065, so the fourth leg ends back at Sol and the game
enters the Sol system. A fourth turn would run in the Solar mode and fail with
`flagship_turn_wrong_mode:observed=Solar`, so the scripts end on the arrival
instead: after the fourth leg's displacement proof each waits 100 input ticks
(there is no typed wait for an activity change; the transition took a few dozen
ticks in the trace), then `assert_activity` (mask 255, equals 4,
IN_INTERPLANETARY) proves they are back in the Sol system, then the
`returned-to-sol` capture, the script's final wait, `probe-encounter-final` and
`finish`. The arrival is the asserted end of these scripts, not a failure.
v6 and v7 fly three legs and never return to Sol.

### The flagship menu in v6 and v7

Cancel in hyperspace opens the flagship menu and nothing in it closes on
Cancel: `DoHyperspaceMenu` (`hyper.c`) handles only Select, and the menu is left
by selecting NAVIGATION (returns FALSE) or STARMAP (opens the star map, then
returns). The menu starts on STARMAP each time. With the default PC menu
(`optWhichMenu` is `OPT_PC`), `DoMenuChooser` (`menu.c`) maps the six items to
the four alternate entries Starmap, Manifest, Game, Navigation, and
`PreviousMenuState` wraps Up from the first entry to the last, so one Up from
STARMAP highlights NAVIGATION. Each v6 visit is therefore
`cancel, up, select` (captures of the open menu and of the NAVIGATION
highlight are kept on the first visit). Each v7 visit is
`cancel, up, up, select` to open Game options (the sequence of
`repro-game-menu-initial`), `assert_game_options`, `cancel` to leave Game
options (`DoGameOptions` returns FALSE on Cancel), then `down` from GAME_MENU to
NAVIGATION and `select`. The next `assert_hyperspace_displacement` proves flight
resumed; no step asserts the menu state directly. The old scripts tapped Cancel
twice and then thrust against an open menu, which moved the ship zero units.

The captures that named a direction (`fly-up-long-1`, `after-fly-right-2`, ...)
claimed a heading the scenario did not control. They are now `leg-N`/`turn-N`
(v3, v4) and `after-leg-N`/`after-turn-N` (v1, v2, v5, v6). The v6 captures
`after-cancel-1/2` claimed Cancel left the menu; they are now
`flagship-menu-open` and `flagship-menu-navigation-highlighted`.

## Player controls and navigation

Use `set_player_key` or `tap_player_key` for raw production gameplay controls.
Supported keys are `thrust`, `down`, `left`, `right`, `weapon`, `special`, and
`escape`. These differ from menu keys: menu direction actions do not move the
flagship.

`navigate_to_planet` is a feedback-driven action:

```json
{"action": "navigate_to_planet", "planet": 2, "max_ticks": 1800}
```

It samples read-only solar-system state and applies player controls on each
input callback. It succeeds only after the real game enters the target planet's
inner system. `max_ticks` contributes to script budget validation and the
script's global watchdog remains the terminal safety bound.

`navigate_to_hyperspace` is the departure counterpart:

```json
{"action": "navigate_to_hyperspace", "max_ticks": 1800}
```

It leaves any orbit through the menu, then flies player one directly away
from the centre of the current view with production thrust/left/right
controls. Leaving the screen at maximum zoom makes the game's own
`CheckShipLocation` end interplanetary flight. The step succeeds only after it
has itself sampled live interplanetary flight and then sees the solar system
torn down and the hyperspace activity dispatched. It never writes activity or
position. A fresh profile starts on the main menu with no save, so every
script that asserts hyperspace (`explore-planet-v1`, `probe-encounter-v1` to
`v7`, `starbase-visit-v1`, `load-save-v1`) first selects New Game, asserts
interplanetary activity and flies out with it (#213). `load-save-v1` then
saves twice through the hyperspace Game menu and loads slot 0 back through
the same menu, because the Quit confirmation loop bypasses the `DoInput`
automation service.

## Restart menu readiness and tap semantics

Every script that begins at the restart menu starts with
`wait_for_main_menu_ready`. The menu ignores input until its first frame is
initialized, and a fixed tick count raced that startup (#213). A
`tap_menu_key` is one press: the updates after its first one only hold the
key, and the production key-repeat pulse for it is discarded, so a slow host
cannot turn one tap into several.

Scripts with a `start_scene` begin at the restart menu too: the scene activates
only after New Game is chosen, so they wait for readiness before that first
Select (#213 amendment 7).

## Deterministic start scenes

Scripts may alternatively declare root-level `start_scene`. A scene is consumed
once after New Game initializes structures and events, before the first activity
dispatch. This is useful for targeted rendering/dialogue tests, but it is not a
substitute for real-path navigation proofs.

`sol-probe-scene-v1.json` demonstrates the mechanism. Its `assert_scene` action
requires encounter conversation 24 and dialogue conversation 18.

## Adding another start scene

1. Add a closed `AutomationScene` variant in `automation/scenario.rs`.
2. Add its pure `ScenePlan`, including expected encounter/dialogue evidence.
3. Add runtime setup in `scenario::activate`; keep it deterministic and let the
   normal game loop perform dispatch.
4. Add parsing, unknown-value, one-shot, safe-boundary, setup-plan, and wrong-
   dispatch tests.
5. Add a script containing both `assert_scene` and `capture`.
