# Runtime automation paths

The native UI suite that runs these scenarios is a local gate on a real Mac window (`native run/validate/report/gallery/replay`). Hosted GitHub runners do not run it.

Automation can either play through normal game state or, for tests that do not
need navigation coverage, activate a deterministic start scene.

`real-sol-earth-orbit` and `real-sol-pluto-orbit-scan` are unplayed New Game
inputs using existing navigation actions. Their exact claims, source-only
verification limits and commands for execution after lane025 is released are in
[the special-system input notes](../../docs/special-system-proof-inputs.md).
Pluto orbit/scan is a Sol path, not Spathi homeworld generator 5 coverage.

## Changing scenarios

Scenarios are plain JSON under `rust/scripts` and can be edited freely: nothing
pins their digests and CI does not run them. Keep assertion-class steps
(`assert_*`, `wait_for_*`, `capture`, `fight_battle`, ...) in place when editing,
because they are what makes a playthrough prove anything.

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
the renderer (`super-melee-battle-start` is taken right after
`assert_battle_frames` proves frames are being drawn, because a fixed delay
after Select sometimes still showed the team selection screen), and requires at
least 30 monotonic observations from the real
production battle loop. The observer counts completed battle-loop frames; it
does not claim combat elapsed time, animation frames, or unique pixel output.

The script then holds `weapon` and `left` (the ship turns while it fires) and
runs `wait_for_battle_ship_loss` (`minimum_ships` 1, `max_ticks` 3000). The
step reads `battle_counter[0]` and `battle_counter[1]`, the ships left on each
side that `tactrans.c` decrements when a ship dies and Super Melee never
raises. Its first sample must be taken inside a battle; the step succeeds
when a counter has dropped by `minimum_ships` since that sample, and fails
when `max_ticks` pass first or the first sample is outside a battle. A battle
that merely runs frames cannot pass, since no ship dies without a real
destroyed ship. The trace records
`battle_ship_loss_reached:lost=N:minimum=M:start=A,B:left=C,D`. The proof is
"a ship was destroyed", not a winner: ending a Super Melee needs one fleet to
lose every ship, which the scripted input and the computer opponent do not
reach deterministically inside the budget, so the script never claims which
side won. Which side lost the ship is visible in the `start`/`left` fields.
This has not run live; the first live run decides whether 3000 ticks is enough.

`super-melee-v1.json` plays a Super Melee match to its end. It enters the
fleet editor with the real menu keys, uses `delete` (the editor's
`KEY_MENU_DELETE`) to remove every ship but the first from each side, and
starts the battle. `assert_state` requires `battle:ships_p1` and
`battle:ships_p2` to both read 1 once combat frames are drawn, so a setup that
left extra ships fails. The human ship takes no further input, and
`wait_for_state` requires the final counters (`battle:ships_p1` 0,
`battle:ships_p2` 1). The result screen waits for a button, so the script waits
out the battle's ending, presses select, then requires the activity word to
return to the Super Melee setup value. The result screen presents no frames, so
it cannot be captured.

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
2. Since #226 the Starbase menu is only reachable through the complete C chain
   that `real-sol-starbase-alliance` plays (the earlier shortcut that allied the
   Starbase after the first Commander talk is gone): probe broadcast, Commander
   visit 1, a Mercury landing that carries a radioactive unit aboard, the Luna
   moon base landing (the lander picks the node up, which sets
   `MOONBASE_ON_SHIP`), Commander visit 2, the Ilwrath conversation and fight,
   the Commander's alliance talk and the allied talk ended with goodbye. The
   chain is copied verbatim from the alliance script.
3. With the moon base on board the allied Commander pays 2500 RU through
   `DeltaSISGauges` (`starbas.c:1659`; the Ilwrath fragment suppresses the
   salvage tally, `encount.c`, so the fight pays nothing; nothing is sold), and
   `assert_ship_resources` requires 896 raw fuel and 2500 RU once the Starbase
   menu is up. The 104 raw fuel are the two landings that precede it:
   `getLandingFuelNeeded` pays `min(SurfaceGravity << 1, 3 * FUEL_TANK_SCALE)`.
   Luna's gravity is 60 * 25 / 100 = 15 (30 raw, confirmed in a real window by
   the earlier 970 proof); Mercury's `gensol.c` overrides are density 98 and
   radius 38, gravity 98 * 38 / 100 = 37, so 74 raw. No hyperspace flight
   happens before this check, and the Commander gives no fuel on this path
   (`need_fuel_*` is never picked). The 896 is derived from the C code, not
   observed: it needs a real-window confirmation.
4. In the Outfit screen, 50 presses of Up each buy 100 raw fuel for 20 RU.
   `assert_ship_resources` requires 5900 raw fuel and 1500 RU. Entering Outfit
   first rounds the 896 raw fuel to the nearest whole unit, 900, so the 50
   purchases end at 900 + 50 * 100 = 5900 and 2500 - 50 * 20 = 1500. The
   tank can hold 6000, so the last 100 raw fuel are simply not bought.
5. Depart, then require the same state again before the flight starts.

`assert_ship_resources` reads `GLOBAL_SIS(FuelOnBoard)` and
`GLOBAL_SIS(ResUnits)`. It only reads: no script action grants fuel or RU, and
a mismatch ends the run with a `SemanticMismatch` trace record that names the
observed and the expected values.

Limits of this preparation:

- Only `probe-encounter-v3` and `probe-encounter-v4` carry the preparation.
  Both now declare 900 s wall budgets and the 900000 ms outer deadline pin.
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

`starbase-visit-v1` is a real Starbase visit. It plays the allied chain of
`real-sol-starbase-alliance` up to the Starbase menu (see "Fuel preparation"),
so the allied Commander has paid 2500 RU and the ship has paid 104 raw fuel for
the Mercury and Luna landings (derived from C, not yet observed live). The
visit then runs, using only menu input:

1. `assert_mode starbase` and `assert_ship_resources` 896 raw fuel / 2500 RU at
   the Starbase menu, capture `starbase-menu-arrival-commander-highlighted`.
2. Down, Select opens Outfit. `assert_mode starbase` (the docked observation
   holds while Outfit is the active input loop) and `assert_ship_resources`
   900 / 2500: Outfit rounds 896 to the nearest whole unit when it opens.
3. Select enters fuel adjust, one Up buys 100 raw fuel for 20 RU:
   `assert_ship_resources` 1000 / 2480. A second Up: 1100 / 2460. Select leaves
   fuel adjust (unchanged), Cancel returns to the Starbase menu, and
   `assert_mode starbase` plus 1100 / 2460 hold again.
4. Down, Select opens the Shipyard, Select opens the hangar, Select edits the
   flagship crew. A new ship has a full crew pod (50 of 50), so a hire is
   refused until a member is dismissed. Down dismisses one crew member and
   refunds `CrewCost` (3 RU): `assert_ship_resources` 1100 / 2463. Up hires the
   member back for 3 RU: 1100 / 2460. No existing assertion reads
   `CrewEnlisted`, so the crew count itself is proved only through the RU it
   moves. Select, Cancel and Cancel leave the editor, the hangar and the
   Shipyard; `assert_mode starbase` and 1100 / 2460 hold.
5. Down, Select departs. After 60 ticks `assert_activity` (mask 255, equals 4)
   proves interplanetary flight and `assert_ship_resources` 1100 / 2460
   proves the purchases survived the departure.

Budgets: `max_input_ticks` 161743 (the validator's static bound for this step
list, the minimum it accepts), `max_presentations` 323486 (twice the ticks),
`max_wallclock_seconds` 900. Its scenario entry carries the
900000 ms outer deadline. Neither the allied chain's tail nor the visit has run live; the
first live run decides whether the Outfit and Shipyard expectations hold.

## Scenario audit limits

A scenario passes when its assertions hold, so a name or domain that claims
more than the assertions prove is a gap. Known gaps that need a new typed
action or a live run before they can be closed:

- `linked-playable-v1` is registered for boot menu and battle only (it walks the
  main menu and runs Super Melee). The cross-domain journey is
  `real-sol-probe-to-starbase`.

## Scenarios changed after the second audit

These edits have not run in a real window yet.

- `probe-encounter-v1`, `-v2`, `-v5`, `-v6`, `-v7` keep their names and now meet
  the probe first: `navigate_to_planet 2`, `assert_dispatch 24/18`,
  `wait_for_communication_end` (1 completion) and `assert_activity 4`, before
  the hyperspace flight. A rename would touch gates, MATRIX, tests and docs for
  scripts whose flights are fixed by `probe_encounter_flight_input`.
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
  conversation (`wait_for_communication_responses` 4, pick 0,
  `wait_for_communication_end` so its completion is consumed before the
  post-battle response waits) starts the battle, `pick_battle_ship`,
  `assert_mode battle`, `fight_battle` fights it to a win, and the
  Commander's `VERY_IMPRESSIVE` talk (3 options, then the 4 alliance names)
  sets `STARBASE_AVAILABLE`. The same visit continues into time passage and the
  allied Commander talk (3 options `have_minerals`, `need_info`,
  `goodbye_commander`; pick 2 ends it, `starbas.c:1647-1736`, `ByeBye`
  `starbas.c:163`), then `assert_mode starbase` proves
  the menu. Captures: `commander-first-visit-done`, the three Mercury captures,
  `luna-moonbase-collected`, `commander-radioactives-offer`, `ilwrath-battle`,
  `ilwrath-defeated`, `allied-starbase-menu`. It asserts no resource numbers,
  since the refuel and 14 day passage values have not been observed live.
  Budgets: 164000 ticks, 900 s wall (outer deadline 900000 ms). Not played
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
- `load-save-v1` asserts ship resources after the load and, with
  `record_saved_hyperspace_position` / `assert_hyperspace_position_restored`,
  that the loaded hyperspace position equals the saved one (see below).
- `real-sol-starbase-alliance` is now the composed journey (new game,
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
  896 raw fuel / 2500 RU (104 fuel for the Mercury and Luna landings, 2500 RU
  from the allied Commander once the moon base is on board). Since #226 it
  plays the alliance chain verbatim (the probe captures `real-probe-broadcast`
  and `real-probe-returned-to-flight` are kept; `real-starbase-commander` went
  with the shortcut visit). Budgets: 161210 ticks, 900 s wall.
- `real-sol-starbase-to-mars` (starbase and navigation) plays the alliance
  chain to the Starbase menu (since #226), asserts `starbase` mode and
  896 / 2500 (derived from C, not yet observed), departs with three Down
  presses and Select, asserts interplanetary flight and unchanged resources,
  then flies to Mars as before. The takeoff is proved the way
  `real-sol-planetside-collision-proof` ends its session: a Special tap, then
  `wait_for_planet_side_end returned`, and only then `assert_mode orbit`.
  Orbit mode already reads true while the lander is on the ground (the planet
  menu phase stays active under it), so the old Escape tap followed by an orbit
  assert proved nothing. Budgets: 191435 ticks, 900 s wall.
- `real-sol-earth-moon-base` (navigation, orbit and scan) followed the
  `probe-encounter-v3` order (probe broadcast, Luna landing, then the two
  Commander conversations) until #226; it now plays the alliance chain (the
  Luna landing details below still apply to that chain's Luna step, but the
  `earth-moon-base-report` capture and the `assert_mode solar` /
  `earth-moon-solar-after-base` pair went with the old prefix; the chain's
  `assert_activity 4` after the Luna return covers the solar view). The first live run landed on Luna but the lander
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
  the run ends with `assert_mode starbase` and `assert_ship_resources` 896 /
  2500 (derived, not yet observed). Budgets: 161208 ticks, 900 s wall. This
  script is now the alliance scenario plus a two-step tail; it stays as a
  separate required scenario.
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

### Saved equals loaded

`record_saved_hyperspace_position` keeps `GLOBAL_SIS(log_x/log_y)` while the
Game menu is open (the game is paused, so the ship cannot drift) before the
first save. It is a separate slot from `record_hyperspace_position`, so a
displacement leg does not consume it.

The script navigates the flagship menu the way the game does (verified in real
windows for `probe-encounter-v6`/`v7`, and read from `gameopt.c`/`hyper.c`).
In hyperspace Cancel opens the flagship menu with the cursor on STARMAP; Cancel
inside it does not close it, and only selecting NAVIGATION (or STARMAP) returns
to flight. In PC menu mode Up from STARMAP wraps to NAVIGATION and Up twice
reaches GAME, so every Game-options visit is `cancel, up, up, select` (one-tick
select) followed by `assert_game_options`. A completed Save is not a Game-options
exit that stays open: `PickGame` returns success, `DoGameOptions` returns FALSE
and `GameOptions` hands control back to `DoHyperspaceMenu`, which redraws the
flagship menu with GAME still highlighted. The script therefore presses Select
after each save to reopen Game options and asserts it is open again before the
next save. After the second save it leaves with Cancel (back to the flagship
menu, GAME highlighted), Down (NAVIGATION) and Select (flight), asserts
hyperspace activity again, records a displacement baseline,
thrusts for 100 ticks and requires `assert_hyperspace_displacement` of 16
units, so the ship is verified to be somewhere else when the load starts. It
then reopens Game options with the same key sequence, asserts it, moves Down to
Load and selects it. The picker opens on the last used slot (slot 1, from the
second save), Up moves to slot 0 and Select loads it; a successful Load sets
`CHECK_LOAD`, which closes every menu on the way out.
`assert_hyperspace_position_restored` requires both coordinates to equal the
kept ones exactly (no tolerance). The ship cannot return to the saved
coordinates unless the game really loaded the save, because nothing moves it
while the menus are open. The trace records
`hyperspace_position_restored:x=:y=:saved_x=:saved_y=`, and the existing
`assert_ship_resources` (1000 fuel, 0 RU) follows. Game date and crew have no
Rust reader, so they are not compared. This has not run live: how many
`cancel` taps leave the Game submenu is decided by the first live run, and a
wrong count fails loudly at the displacement assertion, since a ship under a
menu does not move.

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

Done (this branch): the scripts below now play the alliance chain verbatim
from `navigate_to_planet 2` through `assert_mode starbase`, keep every
post-menu step, and expect 896 raw fuel (1000 minus the 74 raw Mercury and
30 raw Luna landings) and 2500 RU at the menu. The values are derived from the
C code and need one real-window run. The paragraph below describes the state
before the rewrite.

Scripts that reached `assert_mode starbase` through the shortcut:
`probe-encounter-v3`, `probe-encounter-v4`, `real-sol-starbase-to-mars`,
`starbase-visit-v1` (and the since deleted `real-sol-earth-moon-base` and
`real-sol-probe-to-starbase`)
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
until the battle has ended with the opposing side a ship down and the
player's flagship alive. It is
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
The Avenger (`ilwrath.c`) has a short hellfire: `MISSILE_SPEED` 25 world
units a frame (`initialize_missile` takes the speed in world units) for
`MISSILE_LIFE` 8 frames, from `ILWRATH_OFFSET` 29 display units (116 world
units) ahead of the ship, so a flame reaches about 290 world units, more
while the Avenger flies toward its target because the flame inherits its
velocity. `WEAPON_WAIT` is 0 and `ENERGY_REGENERATION` 4 every `ENERGY_WAIT` 4
frames pays the 1 energy of each flame, so it can flame every frame without
running dry; every flame does `MISSILE_DAMAGE` 1 once, so a flagship held in
the flame loses about one crew a frame. Its maximum thrust is 25, `TURN_WAIT`
2, and it has a cloak on `special` (`SPECIAL_ENERGY_COST` 3, `SPECIAL_WAIT`
13). Its intelligence (`ilwrath_intelligence` over `ship_intelligence`) always
PURSUEs, cloaks when it has no target within about six turns of its aim and no
flame going, and decloaks to flame when the flagship is within that reach.

**Strategy** (`rust/src/automation/battle_fight.rs`, `engage`).

*Can the flagship kite?* No. The starting flagship has thrust 10 world units a
frame against the Avenger's 25, and turns one facing per 18 frames
(`TURN_WAIT` 17 plus the turn itself) against one per 3 for the Avenger, so a
half turn costs the flagship 8 x 18 = 144 frames. The Avenger crosses the
bolt's 1152 world units in 46 frames and the 290 unit flame reach in 12 more.
Backing off cannot open a gap, and the old rule "thrust only while facing
away" spent those frames turning the flagship's back to the Avenger with the
gun pointing the wrong way.

*Damage race at contact.* The flagship has 51 crew. The Avenger of the Starbase
fight has 7: `starbase.c` builds it with `CloneShipFragment (ILWRATH_SHIP, ...,
7)`, not the 22 of a stock Avenger (the live battle shows `enemy_crew=7`). One
ion bolt does `BLASTER_DAMAGE` 2 once per `WEAPON_WAIT` 6 frames, so killing
the Avenger takes 4 hits, at least 24 frames of continuous accurate fire. In
the flame the flagship loses about one crew a frame, so 51 crew last about 51
frames of contact. The duel is therefore decided by getting 4 bolts home, and
a flagship that does not fire at all dies. What decides it is how many bolts
land before the Avenger gets inside flame reach.

*Policy.* Face the Avenger and shoot it while it comes; spend no frame on
anything else.
- Never thrust inside twice the bolt range (2300 world units). The Avenger
  closes 25 units a frame by itself, so every frame of its approach is a free
  shot; thrust adds speed the flagship cannot shed (there is no reverse
  thrust) and spoils the aim. Only an enemy farther than 2300 units is
  approached, and only when within two facings of the bearing. From 1100 to
  the Avenger's arrival that is about 40 frames, so about 6 bolts (12 damage,
  more than half its crew) before the first flame if they all land.
- Fire whenever the bolt can plausibly hit (`bolt_can_hit`). A miss costs one
  energy (regenerated at 1 per 10 frames); a withheld shot cost the first
  tuned rule the fight. The bolt fires when any of these holds:
  the facing's line passes within `HIT_RADIUS` 80 world units of the Avenger's
  centre with the Avenger ahead and within the bolt's 1152; or the bearing is
  within one facing of the heading and the Avenger is within 1152 (one facing
  off is a hit on a ship-sized target at close range, and the cheap shot is
  worth it further out); or the bearing is two facings off and the Avenger is
  under 250 units away (point blank).
- Turn along the shorter way round toward the bearing whenever the facing
  differs from it, and keep firing while turning when the rule above allows.
  Turning never stops until the facing matches, because the Avenger moves
  faster than the flagship turns.
- Enemy close behind or cloaked: a cloaked Avenger still has an element and a
  location, so nothing changes. Behind the flagship the bolt cannot hit, so it
  turns toward the Avenger by the shorter way, with no thrust and no fire.
- After the enemy side has lost its ship the flagship releases every key. The
  flames in flight cannot be outrun (25 units a frame against 10, turning 18
  frames a facing), so there is nothing to steer toward; it keeps observing
  until the battle ends and fails at once if it dies meanwhile.

`HIT_RADIUS` is `ENEMY_RADIUS` 64 (about 16 space pixels of hull) plus
`BOLT_RADIUS` 16 (about 4 pixels), at 4 world units a pixel. These are
conservative estimates: the frame sizes are in the content package, which the
Rust side does not read, so no measured extent is available.

*Real-window telemetry that tuned it* (flagship against the 7-crew Avenger
under the first geometric rule, `HIT_RADIUS` 48, which ignored the target's
size and never fired):

```
tick=1  distance=1067 facing_error=9  player_crew=51 enemy_crew=7 action=turn_left
tick=31 distance=424  facing_error=12 player_crew=51 enemy_crew=7 action=turn_left
tick=61 distance=164  facing_error=14 player_crew=27 enemy_crew=7 action=turn_left
tick=91 distance=147  facing_error=15 player_crew=13 enemy_crew=7 action=turn_left
battle_fight_failed:reason=player_ship_lost
```

The flagship turned the whole fight, was never within one facing at a range
where the narrow line test passed, and died. The earlier rule (fire at facing
error 15, 0 or 1 within 1000 units, commit `a82490f87`) won 2 of 3 runs in
109 to 166 ticks. Unit tests pin these four situations: (147, error 15) fires
and turns left, (424, 12) and (1067, 9) turn without firing, (300, 0) fires.

Every 30 ticks the action
writes `battle_fight_state:tick=T:distance=D:facing_error=E:player_crew=P:
enemy_crew=C:action=A` (`A` is `hold`, or `turn_left`, `turn_right`, `thrust`
and `fire` joined by `+`), at most one label per 30 ticks, so real runs give
the distance, aim and crew curves to tune it against.

**Outcome.** The battle is over when `IN_BATTLE` clears: the last ship's
explosion has finished (`cleanup_dead_ship`, `tactrans.c`) or
`rust_bridge_uninit_ships` ran. A ship death does `battle_counter[side]--` at
the moment it dies (`RecordShipDeath`), about 75 frames before the battle
ends, so the enemy's counter dropping is not the end: a run-C flagship died in
that window and the next step waited 6000 ticks for a Commander who never
came. The ship queue is freed with the battle, so the tracker keeps the
counters and the flagship's crew from the last tick the battle was running.
- The first observation records both `battle_counter` values.
- It fails as a semantic mismatch with
  `battle_fight_failed:reason=player_ship_lost` as soon as the player's
  counter is below its start, or the flagship's crew falls to 0 after it has
  been seen with crew (a ship element starts with crew 0 until its first
  preprocess, which is not a death). That is the same tick the C code
  registers the death, not the step timeout.
- When `IN_BATTLE` clears, the fight is won when the last running tick had the
  enemy side below its start and the flagship with crew. It records
  `battle_won:player_left=N:enemy_left=M:ticks=T:player_crew=C`, with `C` the
  flagship's crew at the last running tick, and the scenario predicate needs
  `player_crew` of at least 1.
- A battle that ends with neither side a ship down fails with
  `reason=not_in_battle`, as does no battle at the start; `reason=timeout`
  when `max_ticks` observations pass first. Every key is released on
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

## Start saves: the Starbase chain is played once

Six scripts used to replay the same eight-minute chain (new game, probe,
Commander, Mercury, Luna moon base, Ilwrath battle, alliance) before their own
steps. The chain is now played once, by `real-sol-starbase-alliance`, and later
scenarios start from the save it makes.

- `real-sol-earth-moon-base` and `real-sol-probe-to-starbase` were strict
  subsets of the alliance script (they differed only in two captures and the
  896 fuel / 2500 RU assertion at the menu). Those captures and the assertion
  moved into the alliance script and the two scripts are deleted.
- The Starbase menu has no Game options, so the alliance script departs the
  Starbase (Down x3, Select), asserts interplanetary activity and 896 fuel /
  2500 RU, then saves to slot 0 through the flight Game menu
  (`cancel, up, up, select`, `assert_game_options`, Select to open Save, Select
  for slot 0, Select to confirm). The save is therefore taken in interplanetary
  flight beside Earth. The Starbase is allied by then, so a visit goes straight
  to the Starbase menu with no conversation.
- A script declares `"start_save": {"path": "saves/allied-starbase.sav",
  "slot": 0}`. The path is relative to the script's directory and must be one
  `saves/<name>.sav` file. Before launch the harness copies it to
  `<configdir>/save/uqmsave.00` (`uqmsave.NN` for slot NN) and keeps a copy as
  `inputs/start-save.sav` in the run. The script then loads it through the real
  main menu: Down, Select (Load Game), Select (slot 0). The final config the run
  retains (`config-final/save/`) holds whatever saves the run wrote.
- Saves are not fingerprinted. The provenance file next
  to each save (`saves/allied-starbase.provenance.json`) documents how the save
  was produced: the scenario run, the script it came from and the resources it
  recorded. It is documentation, and admission does not compare it with anything.
- Regeneration rule: when `real-sol-starbase-alliance.json` changes the
  conversation chain or the purchases, regenerate the save. Run the alliance
  scenario in a real window, then `native save-extract --scenario real-sol-starbase-alliance --artifacts <suite bundle
  or its scenarios/NNNN dir> --slot 0 --output
  rust/scripts/saves/allied-starbase.sav`, and commit the save and its
  provenance together. `extract-save` refuses a run whose `terminal_reason` is not
  `success` or whose bundle holds anything but the alliance scenario.

### Scripts that start from the alliance save

`starbase-visit-v1`, `real-sol-starbase-to-mars`, `probe-encounter-v3` and
`probe-encounter-v4` declare the `start_save` and open with the main menu
sequence `wait_for_main_menu_ready`, Down (Load Game), Select (slot list), Select
(slot 0, the picker opens on slot 0 in a fresh process). They then assert
interplanetary activity and 896 fuel / 2500 RU (the values recorded in the
save's provenance), re-dock with `navigate_to_moon 2/0` (the allied Starbase
opens its menu with no conversation) and assert `starbase` mode and the same
resources again, followed by their own earlier steps unchanged. Their budgets
shrank to the validator minimum with a 300 s wall budget, so they no longer carry the
900000 ms outer deadline: `starbase-visit-v1` 2898 ticks,
`real-sol-starbase-to-mars` 32603, `probe-encounter-v3` 7656 and
`probe-encounter-v4` 7856 (presentations twice that). None of this has run live.

## Playability smoke

`native smoke` plays the smallest set of existing scenarios that together cover
the main menu, flight in a solar system and in hyperspace, orbit, landing, the
lander and takeoff back to orbit, a conversation with response selection, a
battle, and a save followed by a load. The set and what each scenario covers is
in `rust/xtask/smoke.json`; it is a subset of the scenarios in
`rust/scripts`, not a second inventory.

```sh
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- native smoke \
  --artifacts DIR --content PATH [--pr N]
```

Run it for any change to game runtime code, before saying the change works and
before merging. A tooling-only change (xtask, CI, docs, scripts) does not need
it. It uses the real Mac window and holds the display awake for the whole
smoke sequence through the same display-wake holder as `native run`.

Every scenario runs through the same path as `native run --scenario`, in order,
and the run continues after a failure so the receipt covers the whole set. The
command exits nonzero if any scenario failed.

The receipt is `DIR/smoke-receipt.json` and `DIR/smoke-receipt.md`: the commit
SHA and whether the tree was clean, the date, and for each scenario the result,
terminal reason, duration, capture count, the screen-check result (`passed` or
the exact failure) and the screenshots directory. The `.md` is a table for a PR
comment and lists the original OS screenshots (`checkpoint-NNNN.os.png`)
alongside the cropped client images (`checkpoint-NNNN.png`) a person or the
image-checking agent has to look at. `--pr N` records the number in the receipt;
the command posts nothing.

"Fixed" may only be claimed with a passing receipt for that commit whose listed
frames were looked at. A passing receipt for another commit, or one nobody
looked at, is not that evidence.

## Checkpoint saves: fix a late step without replaying the chain

The alliance chain (`real-sol-starbase-alliance`) takes three in-flight saves in
one run, each through the flight Game menu (`cancel, up, up, select`,
`assert_game_options`, Select to open Save, the slot, Select, Select):

| Slot | Taken | Save file | Used by |
| ---- | ----- | --------- | ------- |
| 1 | Flying beside Earth after the Mercury takeoff, radioactives aboard (chain steps 36-58) | `saves/alliance-after-mercury.sav` | `real-sol-alliance-from-mercury` |
| 2 | Back in the solar system after the Luna moonbase landing, before visit 2 (chain steps 82-101) | `saves/alliance-after-luna.sav` | `real-sol-alliance-from-luna` |
| 0 | Departing the allied Starbase, the last step of the chain | `saves/allied-starbase.sav` | `starbase-visit-v1`, `real-sol-starbase-to-mars`, `load-save-v1` family, `probe-encounter-v3`/`v4` |

Notes on how the saves are taken:

- The Mercury save is taken after `navigate_to_planet 2` leaves Mercury orbit. The
  orbit menu is a different menu from the flight Game menu and no script has
  opened Game options from orbit, so the checkpoint is placed in flight. The
  chain then resumes with its own `navigate_to_moon 2/1`.
- `PickGame` opens the picker on the last used slot (`lastUsedSlot` in
  `gameopt.c`). A fresh game starts on slot 0, so slot 1 is one Down from there,
  slot 2 is one Down from slot 1, and the final slot 0 save needs two Ups from
  slot 2. After each save the script presses Select (reopens Game options),
  asserts it, then Cancel, Down, Select to return to flight, the sequence
  `load-save-v1` uses, and asserts interplanetary activity.
- The two from-checkpoint scenarios declare `"start_save": {"slot": 0}`: the
  harness installs the file as `uqmsave.00` whichever slot it was extracted from,
  and the script loads it with the same main-menu sequence as
  `starbase-visit-v1`.
- `save-extract` decodes the selected slot and records its own campaign snapshot.
  It does not infer a save's state from the last resource assertion or a capture
  label. It refuses a missing slot, an unchanged installed parent slot, a failed
  run, a different producer script, or modified retained artifacts. The receipt
  includes the parent save digest, scenario, seed and linked build's source commit.

Regenerating all three from one chain run (a real window, passing run):

```
xtask native save-extract --scenario real-sol-starbase-alliance --artifacts <suite bundle> --slot 0 --output rust/scripts/saves/allied-starbase.sav
xtask native save-extract --scenario real-sol-starbase-alliance --artifacts <suite bundle> --slot 1 --output rust/scripts/saves/alliance-after-mercury.sav
xtask native save-extract --scenario real-sol-starbase-alliance --artifacts <suite bundle> --slot 2 --output rust/scripts/saves/alliance-after-luna.sav
```

Commit each save with its `.provenance.json`.

The rule: when fixing a late step, run the from-checkpoint scenario that starts
before it (`real-sol-alliance-from-luna` for visit 2, the Ilwrath fight, the
alliance and the Starbase menu; `real-sol-alliance-from-mercury` when the Luna
landing is also in question; `starbase-visit-v1` and the other slot 0 scenarios
after the Starbase menu). Do not replay the whole chain for that. Run the full
chain only to regenerate the saves, which is needed when a step before a
checkpoint changes (conversations, purchases, the Mercury or Luna routes).

The from-checkpoint scripts copy their steps from the chain verbatim (chain steps
36-95 for Mercury, 59-95 for Luna, original numbering). After the load they
assert interplanetary activity and at least one radioactive unit aboard. Their
tick budgets are the validator minimum. Their saves, `alliance-after-mercury.sav`
and `alliance-after-luna.sav`, were extracted with `xtask native extract-save`
from a passing run of the chain; each has a provenance file beside it. Only the
final save, `allied-starbase.sav`, records the chain's last ship-resource
assertion, because fuel and resource units change between checkpoints. None of
the from-checkpoint scripts has run live.

## Out of fuel

`out-of-fuel-v1` starts from `saves/allied-starbase.sav` (896 raw fuel, 2500 RU),
docks at the Starbase and sells fuel through Outfit: opening Outfit rounds to 900,
eight presses of Down sell 100 raw each for 20 RU, and `assert_ship_resources`
requires exactly 100 raw fuel and 2660 RU. It departs, `navigate_to_hyperspace`,
asserts hyperspace activity and starmap mode, holds thrust and
`wait_for_state` `sis:fuel eq 0`. With thrust still held and no fuel the flagship
decelerates (`sis_ship.c`), so after 400 settle ticks the script runs
`record_hyperspace_position`, waits 300 more ticks and requires
`assert_hyperspace_stationary` with `maximum_logical_units` 0 (the new inverse of
`assert_hyperspace_displacement`: it fails if the ship moved more than the
maximum). It then requires fuel 0 and 2660 RU, hyperspace activity and starmap
mode. The run fails if the tank never empties within 3000 ticks or the ship
keeps moving. No state is written and no save is edited.

Live run: fuel 896, 900, 100, 0; the ship stopped at hyperspace 26671,165445 and
moved 0 units over the 300 tick sample with thrust held.


### `fly-to-alpha-centauri-v1`

Loads `saves/allied-starbase.sav`, leaves Sol through ordinary flight and uses
`fly_to_star {target, max_ticks}` to reach Alpha Centauri (155.9, 99.3).
Targets accept a zero-based star catalog index or a case-insensitive full name
from the shipped game strings. Unknown names and out-of-range indices fail parsing.

The controller opens the hyperspace menu and starmap, steers using the live
cursor's screen projection, selects the destination, reads back autopilot and
closes the map. All player controls remain released during autopilot flight.
Arrival requires a live solar system, interplanetary activity and the exact
catalog index. The scenario independently asserts star 68 and fuel below the
loaded 896 raw points. It uses no position writes or debug movement.

Read-only campaign keys include `star:current` (catalog index), `hyper:x` and
`hyper:y` (SIS logical coordinates), and `starmap:cursor_x`/`starmap:cursor_y`
(universe coordinates, available only while the map is open). Unknown or
unavailable stars and cursors never read as zero. Encounters receive neutral
input; flight resumes if they return control. A blocking dialogue, an empty
tank or failure to arrive exhausts the bounded action with phase, location,
activity, star and fuel in its failure label.


## Phrase-based response selection and chained saves

```json
{"action":"select_response","phrase":"we_will_transfer_now","max_ticks":1000}
```

`phrase` is the name from the current race's conversation string table, not the
localized display text or an option index. Only an offered, callable response
can match. A fresh list is required after each choice. The action resolves its
current index and confirms with the ordinary Select key; it never calls the
story callback. Missing or ambiguous names fail rather than choosing another
option. `lost-battle-v1` uses this for the Commander's radioactives transfer.

Any passing native scenario can produce a checkpoint through the real Game/Save
menu. Select the producer and slot explicitly when extracting:

```sh
xtask native save-extract --scenario fly-to-alpha-centauri-v1 --artifacts <passing bundle> --slot 1 --output rust/scripts/saves/at-alpha-centauri.sav
```

`fly-to-alpha-centauri-v1` loads the allied Starbase save, flies to Alpha Centauri
and saves in slot 1. `load-alpha-centauri-v1` loads the extracted checkpoint and
asserts the star and raw fuel recorded in its adjacent provenance. Provenance
values come from decoding the selected saved file, including activity, catalog
star (when present), fuel, RU, crew, landers, date and opening campaign flags.
The producer commit comes from the retained linked-build receipt, so extracting
from another checkout does not change its attribution. A byte-identical rewrite
of the installed parent slot is refused because retained bytes alone cannot
prove that it was written again; save to a different slot in that case.

### Story devices and energy artifacts

`use_device {"device":"clear_spindle","max_ticks":200}` opens Devices with
menu keys, reads the live inventory, and selects the requested identity rather
than a fixed list index. Names are `clear_spindle`, `aqua_helix`, `rosy_sphere`,
`sun_device`, `talking_pet`, `umgah_caster`, `burvixese_caster`, and
`portal_spawner`. Start in ordinary flight, or in the initial orbit menu with
Scan highlighted. The unmodified bomb is deliberately excluded. Completion
requires a fresh invocation and its actual effect: consumed Ultron part and
condition increment, caster/encounter activation, Chmmr emergence, or the
Spawner's fuel debit and portal flag. A successful menu sound without the
required effect does not pass.

`collect_energy_node {"max_ticks":6000}` starts after lander deployment. It
steers toward a live energy node, dismisses its discovery report through normal
Cancel inputs, and requires both the node's collision-driven retrieval bit and
a newly acquired artifact flag in the same lander generation. Landing near the
node, an already-owned artifact, or a flag without retrieval cannot pass.
`collect-moon-energy-v1` exercises this from the unmodified Mercury checkpoint.

`campaign-s01-funded-sol` loads `allied-starbase.sav`, mines 19 radioactive units
on Mercury, sells all 38 carried radioactive units to Hayes for 304 RU, buys a
500-RU fusion thruster in drive slot 0, and saves flight state to slot 1 through
the Game menu. Its ledger requires 2500 -> 2804 -> 2304 RU, empty sold cargo,
800 raw fuel and positive crew. Slot 0 is retained as the predecessor.
The corresponding `load-funded-sol-v1` scenario verifies the extracted checkpoint.

### Whole-fleet encounter combat and Pkunk campaign segment

`fight_encounter {"max_ticks":16000}` uses the game's player-selectable Cyborg,
configured through Game -> Settings -> Cyborg at normal rendered speed.
`combat:cyborg == 1` and `combat:speed == 1` are required. The action can start
at an ordinary encounter's Hail/Attack menu or in its battle. It chooses Attack
through menu input, picks living escorts before the flagship, and handles a
fresh picker after an escort loss. It never writes crew, damage, fleet counters,
or control settings. Completion requires battle teardown, zero enemy ships,
a surviving player fleet and a living flagship. A single enemy loss does not
complete a larger encounter. Accelerated Cyborg and final-battle scenes are
rejected. `cyborg-encounter-v1` exercises the opening Ilwrath encounter with the
Earthling escort; tracker tests cover replacement picks and multi-ship results.
`fly_to_star` uses the same controller if an encounter interrupts autopilot,
then resumes its original destination instead of completing the flight action.

`escorts:count` reads the actual campaign roster. `escorts:species:N` counts
living escorts of race ID N (0 through 24); Pkunk Fury is 4 and Earthling Cruiser
is 2. These values are also decoded from extracted saves into their provenance.
`encounter:menu` reports the live encounter preview, not communication responses.

`campaign-s02-spindle-pkunk` starts at `funded-sol.sav`, buys 42 fuel units for
840 RU (800 -> 5000 raw fuel, 2304 -> 1464 RU), enables normal Cyborg, and uses
the catalog destination Gamma Krueger. Peaceful homeworld dialogue offers the
Clear Spindle and, after `we_are_vindicator0` / `we_need_help`, four Fury escorts.
The required roster is those Furies plus the existing Earthling Cruiser.
The living Pkunk homeworld grants the Spindle during contact and does not allow
ordinary landing. `collect_energy_node` is its alternative abandoned-ruins
pickup path; this segment does not remove the Pkunk to force that path.
The producer saves through the Game menu to slot 1 and leaves slot 0 unchanged.
`pkunk-refuel-v1` isolates its paid fuel and settings menu sequence.

For planetary travel, `encounter_response` on `navigate_to_planet` and
`navigate_to_orbit` explicitly names the patrol farewell to select after Hail.
The controller resumes steering after communication closes and fails if the
specified response is not offered. Homeworld contact is not dismissed.
`stop_at_encounter` on `navigate_to_orbit` can yield at a preview, recorded as
`contact_reached`, distinct from `orbit_reached`. A communication-response wait
uses Select to Hail a preview rather than waiting forever outside dialogue.

The committed `spindle-pkunk.sav` is the slot-1 output of the passing
`campaign-s02-spindle-pkunk` native producer `pt-p4-17`. It resumes in local
flight at Gamma Krueger, with Clear Spindle aboard, Pkunk manner 3, four living
Furies and the original Earthling Cruiser, crew 46, raw fuel 3457 and RU 1464.
The producer buys 42 fuel units for 840 RU before departure and preserves its
funded-Sol predecessor in slot 0. `load-spindle-pkunk-v1` checks these values
against the extracted save through the ordinary Load menu. The adjacent
provenance records the parent, producing script, seed, output slot and decoded
snapshot. Native screen checks retain an intermediate hyperspace capture so
Sol and Gamma Krueger solar views are not mistaken for a missing transition.

### Northern campaign preparation

`campaign-s03a-north-ready` continues from `spindle-pkunk.sav`, returns to
Sol through hyperspace and funds the northern trip through ordinary Starbase
menus. It sells one of the four gifted Furies (seven crew refunds plus the
hull: 2021 RU), the ion cannon (2000 RU) and the cargo bay (750 RU). The
crew pod in slot 2 remains installed. The ledger is 1464 -> 3485 -> 5485 ->
6235 RU, followed by three 500-RU tanks, two 500-RU thrusters and 181 fuel
units costing 3620 RU. The checkpoint has 20000 raw fuel, 115 RU, 46 flagship
crew, three Furies and the Earthling Cruiser. New tanks occupy module slots
0, 3 and 4; new thrusters occupy drive slots 1 and 2. The Clear Spindle stays
aboard. Normal rendered Cyborg is restored by loading the predecessor, so the
scenario asserts that setting rather than cycling it again.

Save extraction now includes the named story keys asserted or awaited by the
selected producer. Their values are decoded from the saved file, including
zeros; they are not copied from the script's expected values.

`campaign-s03b-broken-ultron` loads `north-ready.sav`, flies to the Supox home
system Beta Librae, and uses the peaceful `anyone_around_here`,
`whats_wrong_with_utwig`, `whats_ultron`, `what_do_i_do_now` dialogue branch.
It requires `ULTRON_CONDITION=1` and the retained Clear Spindle before saving
local flight to slot 1. Patrol contacts are dismissed with `bye_neutral`;
they do not substitute for the scripted homeworld conversation.

`campaign-s03c-thraddash-diverted` continues from `broken-ultron.sav` to
Delta Draconis. The conversation selects `what_about_you_1`,
`what_about_you_2`, `what_about_urquan_1`, `what_about_urquan_2`, then
`got_idea`. The game's event starts the mission; the producer waits for
`THRADD_MISSION` in 1..3 before saving. The extracted local-flight checkpoint
has mission 1, Ultron condition 1, the Clear Spindle, 5494 raw fuel, 115 RU
and 46 crew. `load-thraddash-diverted-v1` verifies the extracted state. The
Thraddash descriptor uses the shipped `comm.thraddash.*` assets; using the
source module abbreviation `comm.thradd.*` leaves contact without responses.

`campaign-s03d-north-artifacts` leaves the saved Thraddash contact with a
short left turn and thrust before the outward-flight controller. This avoids
re-entering the defended homeworld. It flies to Zeta Draconis, checks mission
1..3, lands on planet 0 and collects the Aqua Helix by an energy-node collision.
The lander returns with nine of its twelve crew. The producer saves local
flight to slot 1: Ultron condition 1, Helix aboard, Helix unprotected, mission 1,
Clear Spindle aboard, fuel 4896, RU 115 and flagship crew 43. The separate
`load-north-artifacts-v1` asserts those extracted values and catalog star 441.
The caster diversion and Ultron repair remain subsequent segments.

### Contacting a trader during the northern campaign

`navigate_to_contact {"race":12,"max_ticks":24000}` pursues a live
interplanetary ship group. Race IDs use the same zero-based numbering as
`escorts:species:N`; Melnorme is 12. The controller reads the group's current
inner/outer-system location, follows its planet when it is in another view,
then drives velocity toward the moving ship using normal thrust and turn
inputs. It stops only at an encounter preview or direct greeting with the requested
race in the actual encounter fleet. A wrong-race collision fails, and an absent group
cannot satisfy the bounded wait. The following communication wait hails the
ship through the normal menu. Entering a planet's orbit does not prove contact.

`campaign-s04a-north-refueled` loads `north-artifacts.sav`, orbits Epsilon
Draconis I to discover its rainbow-world location (`RAINBOW_WORLD0=32`), and
contacts the Melnorme at Alpha Apodis using the live-group controller. Selling
the newly discovered location earns 500 credits. Filling the four installed
tanks from 3383 to 21000 raw fuel costs 177 credits, leaving 323 (credit bytes
67 and 1). Crew remains 43, RU remains 115, and the Ultron, Spindle and Helix
remain aboard. The producer saves in local flight to slot 1; the corresponding
`load-north-refueled-v1` checks the extracted snapshot through ordinary Load.
The explicit discovery capture is taken after departure. Orbit-state assertion
checkpoints still retain OS images; paired rotating-planet captures have a
separately reported synchronization problem in issue #395.

`campaign-s04b-burvixese-caster` loads the refueled northern checkpoint and
flies to Arcturus. It approaches the first moon of planet 0, scans, dispatches
the lander and retrieves energy node 0 through a real collision. Both
`BURVIXESE_BROADCASTERS` and `BURV_BROADCASTERS_ON_SHIP` must become 1 before
saving. The retained checkpoint has 13382 raw fuel and 32 crew; natural surface
hazards cost 11 crew during this producing run. The Ultron remains at condition
1, with the Spindle and Helix still aboard. `load-burvixese-caster-v1` verifies
these exact saved values before the Ilwrath diversion.

### Ilwrath caster diversion

`campaign-s04c-ilwrath-approach` flies the earned Burvixese caster from Arcturus
to Alpha Tauri and saves ordinary local flight. Its reload retains 3722 fuel,
32 crew, the caster, both repair pieces and the broken Ultron. The separate
`campaign-s04d-caster-diversion` invokes the caster through Devices, selects
`go_kill_thraddash` and `bye_gods`, and saves `caster-diversion.sav` with
`ILWRATH_DECEIVED=1`, `BURV_BROADCASTERS_ON_SHIP=1` and `ULTRON_CONDITION=1`.
`load-caster-diversion-v1` independently reloads those values before barter.

Device opening reads the actual menu setting: PC menus require Manifest then
Devices; 3DO menus have a direct Devices entry. `device-menu-probe` exercises a
real Clear Spindle invocation from the earned caster checkpoint and requires
its consumption and Ultron condition 1 -> 2. This independent probe does not
replace or mutate the campaign checkpoint.

`campaign-s05a-south-refueled` flies from the earned Ilwrath diversion to
Alpha Centauri, contacts the Melnorme and spends previously earned rainbow
credits on fuel. The real flight leaves 732 fuel before `buy`, `buy_fuel`,
`fill_me_up`; the final checkpoint retains 21000 fuel, 32 crew, the caster,
Ilwrath diversion and broken Ultron. `load-south-refueled-v1` verifies the
extracted save independently. Returning Melnorme business dialogue plays its
intermediate speech through the unlocked Rust input loop, allowing communication
callbacks to access state without recursively locking the global write guard.
