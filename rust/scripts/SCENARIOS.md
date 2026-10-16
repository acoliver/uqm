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

- `battle-v1` proves battle frames run, not a battle outcome. No typed
  assertion reads the battle result (ship destroyed, melee exit); one would
  need a new reader of the battle state and has not been added.
- `load-save-v1` captures the save and load screens, asserts the loaded game is
  in hyperspace with 1000 fuel / 0 RU, but never compares the loaded position
  with the saved one. `assert_hyperspace_displacement` only checks a minimum
  distance since `record_hyperspace_position`; an "unchanged position"
  assertion is missing.
- `linked-playable-v1` is registered for boot menu and battle only (it walks the
  main menu and runs Super Melee). The cross-domain journey is
  `real-sol-probe-to-starbase`.

## Scenarios changed after the second audit

These edits have not run in a real window yet.

- `probe-encounter-v1`, `-v2`, `-v5`, `-v6`, `-v7` keep their names and now meet
  the probe first: `navigate_to_planet 2`, `assert_dispatch 24/18`,
  `wait_for_communication_end` (1 completion) and `assert_activity 4`, before
  the hyperspace flight. A rename would touch gates, MATRIX, tests and docs for
  scripts whose flights are pinned by `probe_encounter_flight_input`.
  `probe-encounter-v1` now exceeds 300 s and joins the 900 s outer deadline list.
- `real-sol-commander-completion` selects Commander responses with typed
  `select_communication_response` (index 0 twice, as in
  `real-sol-planetside-collision-proof`) after
  `wait_for_communication_responses 3`, then waits for the conversation to end
  (`wait_for_communication_end`, which counts completions since the probe wait
  consumed its own) and asserts `assert_activity 4`. The earlier blind
  cancel/select taps never ended the conversation in a real window
  (`communication_responses_active:count=3`, then a 334 s timeout): the first
  list is 3 responses and the next is 2, as in the `comm-encounter-v1` and
  `real-sol-starbase-to-mars` traces.
- `real-sol-rust-planetside-smoke` follows the Mars sequence of
  `real-sol-starbase-to-mars`: orbit, `assert_mode orbit`, scan, dispatch,
  landing site, `wait_for_planet_side_start`, thrust and weapon, special tap,
  `wait_for_planet_side_end returned`, `assert_mode orbit`.
- `real-sol-mercury-radioactives` (navigation, orbit and scan, planet-side
  results; #226) handles the probe broadcast first (`wait_for_communication_end`
  then `assert_activity 4`, as in `real-sol-probe-to-starbase`), flies to
  Mercury (Sol planet index 0, accepted by `navigate_to_planet` and
  `navigate_to_orbit` for 0..=8; the navigation snapshot only rejects an index
  at or above the system's planet count), then orbit, `assert_mode orbit`,
  scan, dispatch, landing site and `wait_for_planet_side_start`. On the surface
  `collect_planet_minerals` drives the lander with player input until the hold
  carries one radioactive unit, a Special tap launches it,
  `wait_for_planet_side_end returned` ends the session, and
  `assert_ship_element radioactive >= 1` proves the unit reached `GLOBAL_SIS`
  after the lander docked. Captures: `mercury-surface`,
  `mercury-radioactive-collected`, `mercury-orbit-with-radioactives`. It never
  visits the Starbase, so it does not depend on the Commander chain below.
  Budgets: 50000 ticks, 600 s wall, so it joins the 900 s outer deadline list.
  Not played live: whether the lander reaches a radioactive deposit within
  3000 ticks of the default landing site on Mercury is unconfirmed.
- `real-sol-starbase-alliance` (new game, communication choices, navigation,
  planet-side results, battle outcomes, Starbase; #226) plays the complete C
  chain to an allied Starbase in one run, so it can be played in a real window.
  It handles the probe broadcast, talks to the Commander (visit 1, picks `[0]`
  then `[0]`), lands on Mercury and collects a radioactive unit exactly as
  `real-sol-mercury-radioactives` does, takes the Luna moonbase with the
  `probe-encounter-v3` landing (19 tick Up hold, six Cancels), then revisits the
  Commander (picks `we_will_transfer_now`, `we_are_vindicator0`,
  `our_mission_was_secret`, `we_are_here_to_help`, `base_was_abandoned`, all
  index 0, each after `wait_for_communication_responses`). The Ilwrath
  conversation (`wait_for_communication_responses` 4, pick 0) starts the
  battle, `assert_mode battle`, `fight_battle` fights it to a win, and the
  Commander's `VERY_IMPRESSIVE` talk (3 options, then the 4 alliance names)
  sets `STARBASE_AVAILABLE`. The same visit continues into time passage and the
  allied Commander talk (2 options, pick 1), then `assert_mode starbase` proves
  the menu. Captures: `commander-first-visit-done`, the three Mercury captures,
  `luna-moonbase-collected`, `commander-radioactives-offer`, `ilwrath-battle`,
  `ilwrath-defeated`, `allied-starbase-menu`. It asserts no resource numbers,
  since the refuel and 14 day passage values have not been observed live.
  Budgets: 160000 ticks, 900 s wall (outer deadline pin 900000 ms). Not played
  live: every step after the Luna return, and whether `fight_battle` wins the
  Ilwrath fight within 6000 ticks.
- `quit-v1` selects Quit and ends with `expect_quit`. The restart menu sets
  `CHECK_ABORT` directly for Quit (`restart.c`; `DoConfirmExit` is not on this
  path) and the game exits before the next input callback, so the scripted
  `assert_mode quit` never ran and finalization recorded an unlabelled
  `semantic_mismatch` at input 146 (real-window run: `mode_verified:main_menu`
  at 145, one input tick at 146, then `run_end semantic_mismatch`, teardown
  `game_status 0`). `expect_quit` is a typed terminal step: it must directly
  follow a `tap_menu_key select` and directly precede `finish`. The restart
  menu reports the Quit selection (`quit_selected:activity=0x4000`) and
  finalization turns it into `quit_verified:activity=0x4000` and a `success`
  run end only when the script declares `expect_quit` at the step the run
  stopped on. An exit without the report, or a reported quit in a script that
  does not declare it, is still a `semantic_mismatch` with its own label
  (`quit_expected:...`, `quit_unexpected:...`). The step needs no OS
  checkpoint (the window is gone); the native acceptance instead requires the
  `quit_verified` record in the trace of any script that declares it.
- `load-save-v1` asserts ship resources after the load.
- `real-sol-probe-to-starbase` is now the composed journey (new game,
  communication, navigation, planet side, starbase); `linked-playable-v1`
  narrows to boot menu and battle.

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
  menu. The live runs at `798528ddf` reached `navigation_reached:planet=3`
  (input tick 4631) and never `orbit_reached` in 6000 ticks, with or without the
  Cancel press, so Cancel was not the cause. The checkpoint screenshot taken at
  `navigation_reached` shows the flagship inside Mars's inner system at its
  entry point, flying (the STARMAP/MANIFEST/GAME/NAVIGATE list is the
  unhighlighted flight panel, not an open menu). The cause was the navigation
  target: `navigation_snapshot` took `PLANET_DESC::image.origin` for Mars, but
  inside the planet's own inner system that field still holds the outer-view
  position (`ValidateOrbit` writes it only for the outer view and moons;
  `EnterPlanetOrbit` says so), while the planet is drawn at the view centre. The
  flagship steered at a point outside the inner view and left it. Runs that
  entered Mars from the Starbase happened to head that way. The snapshot now
  reports the view centre once the ship is in the target planet's inner system,
  and `navigate_to_orbit` uses the velocity controller that `navigate_to_moon`
  uses, because nose-only pursuit of an inertial flagship circles its target.
  The script keeps no Cancel and lets the broadcast end by itself; a script test
  rejects a Cancel before the orbit flight.
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
  six times, a 600 tick wait and `wait_for_planet_side_end returned`. The second
  live run still failed: `798528ddf` dropped the Up hold (19 ticks) that
  `probe-encounter-v3` and `real-sol-starbase-to-mars` use before the landing
  site, and the lander touched down at `position=728,148` instead of their
  `728,68`, where the Cancel presses never launched it. The Up hold is back,
  as in `probe-encounter-v3`, and a script test requires it. After the
  capture of the moon base report the script asserts `assert_mode solar`: the
  third live run showed `mode_mismatch:expected=orbit:observed=solar` 150 ticks
  after the return, because the moon base pickup leaves Luna orbit for the solar
  system view (a Mars landing still reads as orbit). The capture is labelled
  `earth-moon-solar-after-base`. The script proves the pickup through the
  Commander: the second visit pays 2500 RU only with `MOONBASE_ON_SHIP` set, so
  the run ends with `assert_mode starbase` and `assert_ship_resources` 970 /
  2500. Budgets: 65363 ticks, 900 s wall.
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
| v5 | 4 x 100 | 3, three right angles | 1000 raw |
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

Whether the square re-enters Sol depends on how closely it closes, and the
simulation is deterministic for the fixed new-game seed. v5 at 4 x 50 ticks
(real-window run after the probe meeting) ended its last leg at 27967,164221
against the start 27987,164141 while travelling along the line y = x +
136254, which passes 100 units above Sol's point on that diagonal (about 71
perpendicular), so the ship coasted past Sol and `assert_activity 4` failed.
Before the probe meeting the same legs happened to close. v5 now flies v1's
flight section exactly (40 ticks before `hyperspace-start`, 4 x 100 thrust
ticks, settle 5): in the real-window v1 run the last leg is the line through
Sol (27171,163325 to 27935,164089, y = x + 136154, Sol at 27987,164141) and
ends 52 units short of it on each axis, so the coasting ship enters Sol. v2
(80 ticks, 27275,163429 to 27923,164077) closes the same way. The v5 return is
that recorded v1 trajectory rather than a fresh tiny square.

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

## #226: the Starbase no longer becomes available after the first Commander talk

`rust_visit_starbase` used to set `STARBASE_AVAILABLE` and `STARBASE_VISITED`
after the unallied Commander conversation. C never did. The shortcut was added
(8f10ba814) while `CleanupAfterStarBase` was not ported: the visit returned
with `START_ENCOUNTER` and `GLOBAL_FLAGS_AND_DATA` still set, so the game kept
re-entering the visit. `cleanup_after_visit` now ports that function, and the
shortcut is gone (`starbase::finish_unallied_commander_talk`).

Scripts that reach `assert_mode starbase` through the shortcut:
`probe-encounter-v3`, `probe-encounter-v4`, `real-sol-earth-moon-base`,
`real-sol-probe-to-starbase`, `real-sol-starbase-to-mars`, `starbase-visit-v1`
(visit 2 in each, steps 38-45 in v3). Expect them to stop at the second visit (not played live): the
Starbase is still unallied, so visit 2 is the unallied Commander talk again
(`comandr.c` `Intro`: `STARBASE_VISITED` set, `RADIOACTIVES_PROVIDED` 0, so
"do you have radioactives this time"), never the menu. `repro-comm-menu-initial`
and the `StarbaseCommander` start scene set `STARBASE_AVAILABLE` themselves and
are unaffected.

The real C sequence the scripts must play (none of it is scriptable by the
existing prep, and none of it has been played live):

1. First Commander talk (sets `STARBASE_VISITED`).
2. Fetch radioactives: land on a world with radioactive elements (the
   Commander points to Mercury), pick up at least one unit with the lander
   (`select_planet_menu`/`wait_for_planet_side_*` already drive a landing, but
   there is no typed action to steer the lander onto a mineral deposit), return,
   `we_will_transfer_now` sets `RADIOACTIVES_PROVIDED` and refuels.
3. Revisit: `TellMoonBase`, then Luna. Luna's base conversation or fight sets
   `MOONBASE_DESTROYED`; `MOONBASE_ON_SHIP` is set by the Luna pickup.
4. Revisit: `BaseDestroyed` offers `base_was_abandoned` / `we_fought_them`
   (-> `oh_yes_big_fight` / `i_lied_it_was_abandoned` / `i_cant_talk_about_it`).
   `ByeBye` sets `PROBE_ILWRATH_ENCOUNTER`; the visit goes on.
5. `VisitStarBase` clones one Ilwrath Avenger into the NPC queue
   (`starbase.c:462`) and runs `ILWRATH_CONVERSATION`, which starts a one ship
   battle against the flagship (the Ilwrath cloak and flame; the flagship is
   the only player ship). Winning needs flight and fire input in a battle that
   has no typed action yet. `wait_for_battle_ship_loss` can end the wait, but
   nothing steers or aims. Losing ends the game (`CrewEnlisted == ~0`).
6. After the battle the Commander talks again (`PROBE_ILWRATH_ENCOUNTER` still
   1): `cook_their_butts` / `overthrow_evil_aliens` / `annihilate_those_monsters`
   reset `PROBE_ILWRATH_ENCOUNTER` to 0 and set `STARBASE_AVAILABLE` to 1
   (`comandr.c:159`). The next visit is the allied one with the menu.

Plan: add a battle driving action (hold thrust/fire toward the Ilwrath until
`wait_for_battle_ship_loss` for the enemy) and a lander mineral-pickup action,
then replace the visit-2 block of the six scripts with steps 2-6, keeping every
existing assertion (`assert_mode starbase`, 970 raw fuel / 2500 RU, Outfit
purchases). Expected values change: the radioactives trade refuels the ship,
and the 14-day time passage and 2500 RU arrive on the visit after
`STARBASE_AVAILABLE` is set. Re-derive the `assert_ship_resources` numbers from
a real-window run. Until then these scripts are expected to stop at visit 2.

### #226 research: the C chain, verified against the sources

All `comandr.c` lines are `sc2/src/uqm/comm/comandr/comandr.c`. Response indices
are the zero-based order in which the conversation offers its options, so they
are the `select_communication_response` indices. The lander-loss and fuel
options appear first when they apply (`NumLanders == 0`, fuel under two tanks);
the indices below assume a fresh game where neither does.

Flags: `STARBASE_VISITED` (set at `comandr.c:636`), `RADIOACTIVES_PROVIDED`
(`comandr.c:566`), `MOONBASE_DESTROYED` + `MOONBASE_ON_SHIP` (both set by the
Luna pickup, `planets/generate/gensol.c:594-595`), `WILL_DESTROY_BASE`
(`comandr.c:108`, informational only), `PROBE_ILWRATH_ENCOUNTER`
(`comandr.c:117/124/131`), `STARBASE_AVAILABLE` (`comandr.c:159`).

1. **Visit 1** (unallied, `Intro` first-visit branch, `comandr.c:630-648`).
   Options: `[0]` `no_but_well_help0`, `[1]` `yes_this_is_supply_ship`,
   `[2]` `what_slave_planet`. Pick `[0]` -> `NoRadioactives` (`comandr.c:197`).
   Options: `[0]` `ok_i_will_get_radios` (ends, `THANKS_FOR_HELPING`),
   `[1]` `where_can_i_get_radios` (`RADIOS_ON_MERCURY`, then the same two
   again). Pick `[0]` to end. This matches the live run ("We will leave now to
   find the elements you require").
2. **Radioactives**: `RADIOACTIVES_PROVIDED` is only checked against
   `GLOBAL_SIS (ElementAmounts[RADIOACTIVE])` being nonzero
   (`comandr.c:203/256/625`), so any one unit of the RADIOACTIVE element
   category is enough. `RADIOACTIVE` is element category 6
   (`planets/elemdata.h:37`), worth 8 RU per unit
   (`globdata.c:381`, `ElementWorth[RADIOACTIVE]`). Mercury is `METAL_WORLD`
   (`planets/generate/gensol.c:129`), and `METAL_WORLD` in `plandata.c:839-856`
   lists `{URANIUM, HEAVY_USEFUL}` (uranium is in the radioactive category);
   `HEAVY_USEFUL` is `MINERAL_DEPOSIT (MODERATE, HEAVY)`
   (`plandata.c:720`). Mineral generation is
   `GenerateDefault_generateMinerals` (`gensol.c:61`), so the deposit
   positions come from the system seed. Io (Jupiter moon 0) is also a
   `RADIOACTIVE_WORLD` (`gensol.c:221`) but the Commander points at Mercury
   (`comandr.c:216`). The commander does not take the units: nothing in
   `GiveRadios` subtracts `ElementAmounts`, it only sets
   `RADIOACTIVES_PROVIDED` and plays the refuel speech.
   Rust side: `rust/src/planet_side/cargo.rs` `MineralCargo` accumulates per
   category (`categories()[6]`), the lander cargo is returned in
   `ShipDelta::element_amounts` (`session.rs:211`) and added to the ship on
   return, so the amount is readable after the trip from the ship resources.
3. **Luna moonbase**: the lander landing at 728,68 in `probe-encounter-v3`
   (a 19 tick Up hold) is the moonbase energy node
   (`gensol.c:545-596`: node at `MAP_WIDTH*3/4, MAP_HEIGHT/4`), not a mineral.
   Its pickup sets `MOONBASE_DESTROYED` and `MOONBASE_ON_SHIP` regardless of
   `WILL_DESTROY_BASE`. So the existing Luna landing already does the moonbase
   mission; it gives no radioactives. It must happen before the turn-in talk
   below, because `RevealSelf` routes `we_are_here_to_help` to `BaseDestroyed`
   only when `MOONBASE_DESTROYED` is set (`comandr.c:552-555`); otherwise to
   `TellMoonBase`, which never sets `PROBE_ILWRATH_ENCOUNTER`.
4. **Visit 2** (radioactives aboard, Luna done). `Intro` (`comandr.c:606-627`)
   says `DO_YOU_HAVE_RADIO_THIS_TIME`, then `GiveRadios (0)`
   (`comandr.c:585-592`): `[0]` `we_will_transfer_now` (sets
   `RADIOACTIVES_PROVIDED`, plays `FUEL_UP0/1`, goes to `RevealSelf (0)`),
   `[1]` `what_will_you_give_us`, `[2]` `before_radios_we_need_info`.
   `RevealSelf` (`comandr.c:466-558`) then offers `[0]` `we_are_vindicator0`,
   `[1]` `first_give_info`. Pick `[0]` each time (the `first_give_info` family
   flips the stack and re-orders the list): after `we_are_vindicator0` the
   list is `[0]` `our_mission_was_secret`, `[1]` `first_give_info`; after that
   it is `[0]` `we_are_here_to_help`, `[1]` `first_give_info`. So the picks are
   `[0], [0], [0], [0]`: transfer, vindicator, secret, here_to_help.
   `we_are_here_to_help` -> `BaseDestroyed` (`comandr.c:349`, Luna done):
   `[0]` `base_was_abandoned` (-> `ByeBye`, `PROBE_ILWRATH_ENCOUNTER` = 1,
   `HERE_COMES_ILWRATH`, conversation ends) or `[1]` `we_fought_them`
   (-> `[0]` `oh_yes_big_fight`, `[1]` `i_lied_it_was_abandoned`,
   `[2]` `i_cant_talk_about_it`, each also sets the flag). Pick `[0]`.
   If radioactives are NOT aboard on this visit: `AskAfterRadios`
   (`comandr.c:286`): `[0]` `well_go_get_them_now`, `[1]` `where_get_radios`.
   The live run showed one option ("Where can we acquire radioactives?")
   and the leave line; reconcile against the Rust comm option list before
   scripting indices for that visit.
5. **Ilwrath fight** (`starbase.c:446-476`): with the flag set,
   `VisitStarBase` clones one `ILWRATH_SHIP` into `npc_built_ship_q`
   (`starbase.c:463`) and runs `ILWRATH_CONVERSATION`. The Ilwrath comm
   (`comm/ilwrath/ilwrathc.c:544-552`) offers four options
   `[0..3]` (`where_you_come_from`, `it_will_be_a_pleasure`, `surrender`,
   `be_reasonable`), all `CombatIsInevitable` (`ilwrathc.c:104`,
   `Segue_hostile`), so any index starts the battle; the comm init picks
   `Segue_peace` only when the flag is clear (`ilwrathc.c:634-646`), so with
   the flag set it is hostile. The battle is the flagship against one Ilwrath
   Avenger; the battle ends when one side has no ship. Losing sets
   `CrewEnlisted == ~0` and the game ends (`starbase.c:472-474`). There is no
   scripted outcome. Winning reliably is not established: the flagship needs
   flight and fire input against the cloaking Avenger, which no action
   provides.
6. **After the battle** (`starbase.c:477-483`): `Intro` sees
   `PROBE_ILWRATH_ENCOUNTER` and says `VERY_IMPRESSIVE` with `[0]`
   `cook_their_butts`, `[1]` `overthrow_evil_aliens`,
   `[2]` `annihilate_those_monsters` (`comandr.c:598-605`); any of them resets
   the flag and sets `STARBASE_AVAILABLE` (`comandr.c:137,159`), then offers
   the alliance-name list `[0]` `name_1`, `[1]` `name_2`, `[2]` `name_3`,
   `[3]` `name_40` (`comandr.c:150-157`); any pick ends it with
   `STARBASE_WILL_BE_READY`. `commglue.c:390-394` now routes the Commander to
   the allied `init_starbase_comm`.
7. **Allied visit start** (`starbase.c:488-507`): `MOONBASE_ON_SHIP` is still
   set, so `DoTimePassage` (14 days) runs and the allied Commander talks
   (`starbas.c:1644`, `STARBASE_IS_READY_A`; `post_starbase_enc` clears
   `MOONBASE_ON_SHIP`, `starbas.c:1932`), then `DoStarBase` shows the menu
   with Outfit. So the same visit that ends the Ilwrath fight is the one that
   shows the allied menu; no extra visit is required.

What the automation lacks, in order: (a) the lander mineral pickup for
Mercury (radioactive category 6), (b) driving the Ilwrath battle to a win,
(c) the script rewrite of visit 2 using the indices above. `rust/src/planet_side`
currently publishes lander position and crew to the automation (telemetry),
but not facing, per-category cargo, or deposit positions.

### #226 battle: `pick_battle_ship`

After the Ilwrath hail the full game enters `Battle ()` and `selectAllShips`
(`battle.c:387`) calls `GetInitialStarShips`, which asks `GetEncounterStarShip`
for each side (`pickship.c`). The NPC side takes the head of its fleet with no
input. For the human side (`pickship.c`, full game, `RPG_PLAYER_NUM`) the C
code returns the flagship without any screen only when this is the first pick
and `battle_counter[0] == 1`, that is, the player has no escorts. Otherwise it
runs `GetArmadaStarShip`, whose `DoPickBattleShip` loop waits for
`KEY_MENU_SELECT` (the flagship column is preselected). After the Commander
visit the player owns escorts, so the picker is shown and the battle never
starts without a Select. The real-window run of 77b6c8f16 stalled there: the
NPC assets loaded, then nothing until the step timed out. There is no Rust
port of `selectAllShips` or the picker, so no Rust/C divergence.

`pick_battle_ship {max_ticks}` waits until the picker has run its first
iteration, then presses the real menu Select once and records
`battle_ship_picked:generation=N`. It fails as a semantic mismatch when no
picker appears within `max_ticks`, so it also fails if the player has no
escorts. The observation is `rust_automation_observe_ship_picker` in
`ui_observation.rs`, called from `pickship.c` (a read-only `1` after the
picker initialises, a `0` after `DoInput` returns; this is the only C touched,
and only as observation). Each entry starts a new generation, so one pick
cannot be reused for a later picker.

### #226 battle: `fight_battle`

`fight_battle {max_ticks}` fights a full-game battle with the player-one keys
until the opposing side has lost a ship and the player's side has not. It is
for the Starbase fight (step 5 above): the flagship against one Ilwrath
Avenger. Nothing in the game is written; the action only holds
thrust, left, right and weapon.

**How the battle reads player input.** `DoBattle` runs the ship input of each
side through `PlayerControls[playerNr]`. In the full game side 0 is the human
player (`RPG_PLAYER_NUM`) and side 1 the NPC (`NPC_PLAYER_NUM`,
`races.h:STARSHIP.playerNr`, `element.h:ELEMENT.playerNr`). The automation key
setter writes `ImmediateInputState.key[PlayerControls[0]][index]`
(`gameinp.c:c_automation_set_immediate_player_key`), so the same
`inject_player_key` used for flight drives the battle ship: `thrust` (0),
`left` (2), `right` (3), `weapon` (4), `special` (5). The battle loop is
already entered through the input hook every frame, which is how the
`set_player_key` holds of `battle-v1` work.

**What exposes the ships.** The battle is still C-owned, so the reader borrows
the same queues the hyperspace facing reader borrows. `disp_q` holds every
live `ELEMENT`; the ship elements carry `PLAYER_SHIP`, `playerNr`,
`current.location` (world units) and the live crew in `crew_level`, and
`pParent` is the owning `STARSHIP`, whose `ShipFacing` is the facing 0..15
(0 is north, clockwise, `ShipFacing` is kept current by `ship.c`). Ship
counts are `battle_counter[2]` (`battle.c`, `pickship.c`, `tactrans.c`: a ship
death does `battle_counter[deadStarShip->playerNr]--`), declared once in
`c_bindings::live_queues`. `battle_counter[0]` is the player side in a full-game
battle as well, because `Battle ()` sets it from `CountLinks (&race_q[0])` and
`race_q[RPG_PLAYER_NUM]` is the human fleet; the `battle-v1` Super Melee run
(11,9 then 10,9 when the player's own ship died) shows the same ordering. The
walk of `disp_q` is the existing `savegame::runtime::queues::walk`, which
checks every link against the pool before it is followed.
The cloak flag (`OBJECT_CLOAKED`) lives in `DisplayArray[PrimIndex]`, which the
generated bindings do not expose; the fight does not need it, because a
cloaked Avenger's element and `current.location` are still in `disp_q`.

**The arena.** The battle space is a torus of `LOG_SPACE_WIDTH` by
`LOG_SPACE_HEIGHT` world units (`units.h`: `DISPLAY_TO_WORLD (SPACE) <<
MAX_REDUCTION`, so 32 world units per space pixel). The space is the screen
less the 64 pixel status panel, so the arena depends on `ScreenWidth` and
`ScreenHeight`: 8192 by 7680 at 320x240. The `battle_types` constants assume
8192 by 8192, which is only right for a square space, so the action takes the
size from the live screen (`battle_fight::arena_size`) and wraps deltas with
`shortest_path_delta`, the same rule as `WRAP_DELTA_X/Y`.

**The ships.** The flagship's primary weapon is the ion blaster
(`sis_ship.c`): `BLASTER_SPEED` 24 display units (96 world units) for
`BLASTER_LIFE` 12 frames, so about 1150 world units, forward only (a tracker
module only bends it), `WEAPON_WAIT` 6, one energy per shot, energy regeneration
1 per `ENERGY_WAIT` 10 frames. Holding fire all the time drains energy, so the
fight fires only when aimed and in range. Maximum thrust 10, `TURN_WAIT` 17.
The Avenger (`ilwrath.c`) has a short hellfire (`MISSILE_SPEED` 25,
`MISSILE_LIFE` 8, `ILWRATH_OFFSET` 29: about 230 world units), maximum thrust
25, `TURN_WAIT` 2, and a cloak on `special` (`SPECIAL_ENERGY_COST` 3,
`SPECIAL_WAIT` 13). Its intelligence cloaks when it has no target in
reach and no flame going, and decloaks to flame when the flagship is within
about six turns of its aim. It out-runs and out-turns the flagship, so the
flagship cannot kite it; the only way to win is to out-shoot it at range while
not sitting in front of its flame.

**Strategy** (`rust/src/automation/battle_fight.rs`, `fight`): each tick, take
the wrapped delta to the enemy and the facing it points along (the same
`arctan`/`angle_to_facing` rule as the lander steering).
- Turn along the shorter way round until the heading is exactly the bearing.
- Fire when within one facing of the bearing and within 1000 world units.
- Close in (thrust) when farther than 800 units and roughly aimed (within two
  facings).
- Inside 400 units (well inside hellfire reach plus the Avenger's speed) thrust
  only while facing away (5..=11 facings off), so the flagship backs off and
  turns back to aim rather than driving into the flame.
- Otherwise coast and keep aiming.
- If either ship element is missing (before the ships spawn) hold every key; a
  cloaked enemy still has an element and a location, so the aim is unchanged.

**Outcome.** The first observation in a battle records both `battle_counter`
values. The fight is won as soon as the enemy side is below its starting count
and the player's side is not, and records
`battle_won:player_left=N:enemy_left=M:ticks=T`. It fails as a semantic
mismatch with `battle_fight_failed:reason=player_ship_lost` (the player's side
is below its start, even if the enemy side fell too), `reason=timeout`
(`max_ticks` observations passed) or `reason=not_in_battle` (no battle at the
start, or it ended with neither side losing a ship). Every key is released on
completion and on failure. `max_ticks` must be at least 1.

**Steps from the Ilwrath conversation to the won fight** (indices from step 5;
any of the four picks starts the battle, `[0]` is `where_you_come_from`):

```json
{"action": "wait_for_communication_responses", "minimum": 4, "max_ticks": 2000},
{"action": "select_communication_response", "index": 0, "max_ticks": 5000},
{"action": "pick_battle_ship", "max_ticks": 5000},
{"action": "wait_for_battle_frames", "minimum": 30, "max_ticks": 3000},
{"action": "assert_mode", "mode": "battle"},
{"action": "fight_battle", "max_ticks": 6000},
{"action": "wait_for_communication_responses", "minimum": 3, "max_ticks": 6000}
```

The last step is the Commander's `VERY_IMPRESSIVE` reply of step 6 (three
options). The scenario budget `max_input_ticks` must cover the 6000 ticks.
This was not run against the live game here; it is verified by unit tests of
the decision, tracker, script, scheduler, predicate and checkpoint.
