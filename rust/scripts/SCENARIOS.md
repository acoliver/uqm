# Runtime automation paths

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
  opening 1000 raw fuel. Whether their flights fit that tank is an estimate,
  not a measurement, and no live run has shown it either way. They keep their
  original holds and turns; nothing was shortened to make them fit.
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
