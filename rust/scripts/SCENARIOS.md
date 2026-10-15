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
- Save/Load (from Outfit and from the Shipyard) and the Shipyard ship spin are
  not ported and end the game loop with a typed `Unported` error. No script
  selects them. Only `starbase-visit-v1` opens the Shipyard, for crew changes.
- `repro-comm-menu-initial` starts in the Commander conversation and finishes
  inside it, so it never reaches the Starbase menu. It is unchanged.

## Starbase visit

`starbase-visit-v1` is a real Starbase visit. It reuses the fuel-preparation
route of `probe-encounter-v3` unchanged up to the Starbase menu (probe
broadcast, Luna landing, first and second Commander conversation), so the
Commander has paid 2500 RU and the ship has paid 30 raw fuel for the Luna
landing. The visit then runs, using only menu input:

1. `assert_mode starbase` and `assert_ship_resources` 970 raw fuel / 2500 RU at
   the Starbase menu, capture `starbase-menu-arrival-commander-highlighted`.
2. Down, Select opens Outfit. `assert_mode starbase` (the docked observation
   holds while Outfit is the active input loop) and `assert_ship_resources`
   1000 / 2500: Outfit rounds 970 up to the whole unit when it opens.
3. Select enters fuel adjust, one Up buys 100 raw fuel for 20 RU:
   `assert_ship_resources` 1100 / 2480. A second Up: 1200 / 2460. Select leaves
   fuel adjust (unchanged), Cancel returns to the Starbase menu, and
   `assert_mode starbase` plus 1200 / 2460 hold again.
4. Down, Select opens the Shipyard, Select opens the hangar, Select edits the
   flagship crew. A new ship has a full crew pod (50 of 50), so a hire is
   refused until a member is dismissed. Down dismisses one crew member and
   refunds `CrewCost` (3 RU): `assert_ship_resources` 1200 / 2463. Up hires the
   member back for 3 RU: 1200 / 2460. No existing assertion reads
   `CrewEnlisted`, so the crew count itself is proved only through the RU it
   moves. Select, Cancel and Cancel leave the editor, the hangar and the
   Shipyard; `assert_mode starbase` and 1200 / 2460 hold.
5. Down, Select departs. After 60 ticks `assert_activity` (mask 255, equals 4)
   proves interplanetary flight and `assert_ship_resources` 1200 / 2460
   proves the purchases survived the departure.

Budgets: `max_input_ticks` 65752 (the validator's static bound for this step
list, the minimum it accepts), `max_presentations` 131504 (twice the ticks, as
in `probe-encounter-v3`), `max_wallclock_seconds` 720. The script is pinned with
the approved 900000 ms outer deadline like the other scripts above 300 s. Like
the preparation it extends, the visit has not run live; the first live run
decides whether the Outfit and Shipyard expectations hold.

## Scenario audit limits

A scenario passes when its assertions hold, so a name or domain that claims
more than the assertions prove is a gap. Known gaps that need a new typed
action or a live run before they can be closed:

- `quit-v1` highlights Quit and asserts the main menu mode. It never confirms
  the quit (the Quit confirmation loop bypasses the `DoInput` service).
- `battle-v1` proves battle frames run, not a battle outcome.
- `load-save-v1` captures the save and load screens but never compares the
  loaded state with the saved one.
- `probe-encounter-v1`, `-v2`, `-v5`, `-v6`, `-v7` never meet the probe.
- `real-sol-commander-completion` never asserts that the conversation ended.
- `real-sol-rust-planetside-smoke` asserts no planet-side state.
- `linked-playable-v1` is registered for boot menu, new game, communication,
  navigation and battle but only walks the main menu and runs Super Melee.

## Scenarios that prove their claim

The scenario audit found six scenarios whose name or domain claimed more than
their assertions proved. Each now carries the assertion that matches the claim.
The first real-window run at `e810b17c0` passed `sol-probe-scene-v1` and
`real-sol-probe-to-starbase` and exposed four scripts that failed or proved too
little; the entries below describe the corrected scripts, which have not run
live since. `audited_scenarios_assert_what_they_claim` in `script.rs` and
`explore_flies_to_mars_orbit_and_scans_with_real_menu_input` in `scheduler.rs`
pin the shape of the scripts.

- `explore-planet-v1` (navigation, orbit and scan) no longer flies in
  hyperspace. After the probe broadcast it flies to Mars (planet 3) and enters
  orbit with `navigate_to_planet` and `navigate_to_orbit`, then
  `select_planet_menu orbit` and `assert_mode orbit` (the orbit menu is the
  active planet menu phase). `select_planet_menu auto_scan` runs the scan and
  `select_planet_menu dispatch` only completes once the scan has finished and
  the Dispatch menu is up; Cancel backs out of the landing site picker without
  launching the lander, and a second `assert_mode orbit` holds in the scan
  menu. The first live run reached `navigation_reached:planet=3` but never
  `orbit_reached` in 6000 ticks. The script had skipped the probe speech with
  Cancel, which ended the broadcast 123 input ticks after `dispatch_verified`
  where the uninterrupted broadcast in `real-sol-starbase-to-mars` takes 2546,
  while `real-sol-starbase-to-mars`, which lets the broadcast run, orbits Mars.
  The trace records no menu state, so the flagship
  menu opened by a stray Cancel (`DoIpFlight`) is the suspected mechanism, not
  a recorded fact. The script now pulses no Cancel and waits for the broadcast
  to end by itself. A script test rejects a Cancel before the orbit flight.
  Budgets: 24710 input ticks (the validator minimum), 600 s wall with the
  approved 900000 ms outer deadline.
- `real-sol-probe-to-starbase` (starbase) takes the `probe-encounter-v3`
  route: probe broadcast, Luna landing, first and second Commander
  conversation, then `assert_mode starbase` and `assert_ship_resources`
  970 raw fuel / 2500 RU (30 fuel for the Luna landing, 2500 RU from the
  Commander once the moon base is on board). The first Commander dispatch and
  its capture are kept. Budgets: 65215 ticks, 720 s wall.
- `real-sol-starbase-to-mars` (starbase and navigation) takes the same route to
  the Starbase menu, asserts `starbase` mode and 970 / 2500, departs with three
  Down presses and Select, asserts interplanetary flight and unchanged
  resources, then flies to Mars as before. The takeoff is proved the way
  `real-sol-planetside-collision-proof` ends its session: a Special tap, then
  `wait_for_planet_side_end returned`, and only then `assert_mode orbit`.
  Orbit mode already reads true while the lander is on the ground (the planet
  menu phase stays active under it), so the old Escape tap followed by an orbit
  assert proved nothing. Budgets: 83375 ticks, 900 s wall.
- `real-sol-earth-moon-base` (navigation, orbit and scan) follows the
  `probe-encounter-v3` order: probe broadcast, Luna landing, then the two
  Commander conversations. The first live run landed on Luna but the lander
  never left (`planet_side_wait_unsatisfied ... active=true phase=Active` for
  6000 ticks) because the six presses were Select; the working route uses Cancel
  six times, a 600 tick wait and `wait_for_planet_side_end returned`. After the
  capture of the moon base report the script asserts `assert_mode orbit`, and
  proves the pickup through the Commander: the second visit pays 2500 RU only
  with `MOONBASE_ON_SHIP` set, so the run ends with `assert_mode starbase` and
  `assert_ship_resources` 970 / 2500. Budgets: 64762 ticks, 900 s wall.
- `sol-probe-scene-v1` (communication choices) skips the Urquan speech with
  Cancel, then `wait_for_communication_responses` (minimum 1) and
  `assert_communication_responses` (minimum 1), the same list that
  `repro-sol-probe-comm-ui` asserts. It needs 500 input ticks and a 60 s wall
  budget (was 192 and 30).
- `comm-encounter-v1` (communication choices) cannot prove a choice with the
  probe: its broadcast is one way. The first live run ended with
  `communication_responses_wait_expired:minimum=1:active=false:completions=1`.
  The script now lets the broadcast end, flies to the Commander on Luna
  (`navigate_to_moon` 2/0, `wait_for_dispatch` 2/2), waits for and asserts the
  response list (minimum 2), selects the first response twice and waits for the
  conversation to end. Budgets: 12000 input ticks, 400 s wall with the approved
  900000 ms outer deadline.

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
script that asserts hyperspace (`probe-encounter-v1` to
`v7`, `load-save-v1`) first selects New Game, asserts
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
