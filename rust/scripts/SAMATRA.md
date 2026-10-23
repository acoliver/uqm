# Sa-Matra automation and ending observations

## Campaign acceptance is pending

`samatra-dev-fixture-v1` is development-only. It does not prove a campaign playthrough, the Talking Pet distraction, the six remnant guard victories, or an earned bomb/fleet. S12–S14 remain pending until the campaign lane supplies a connected pre-boss checkpoint.

The save search covered `rust/scripts/saves`, `docs/evidence`, and `tmp/retained-proof`: 101 save files, 39 distinct payloads. The committed checkpoints reached the Burvixese caster; the retained saves were opening/save-load proofs. No pre-Sa-Matra checkpoint was identified. Save-summary activity numbers are not live activity numbers and were not used to infer boss readiness.

## Development fixture

The `samatra_dev_fixture` start scene requires a debug build or the `debug-process` feature used by the native runner. It prepares twelve Chmmr escorts, a crewed weaponless bomb flagship with engines/jets, the converted-bomb story state and a Sa-Matra star identity. These grants are fixture setup, not story-earned progress. Normal last-battle dispatch then starts the Talking Pet conversation and real boss battle.

The controller never grants damage, removes generators, selects ships through a callback, changes activity, or marks victory. Only the fixture initializer writes setup state. Runtime observations remain read-only.

Run from the lane checkout through the shared native-window launcher:

```sh
ROOT=/Volumes/XS1000/acoliver/projects/uqm
bash "$ROOT/tmp/pt-runs/iterm-run.sh" end-fixture-current samatra-dev-fixture-v1 "$ROOT/tmp/wt/closing"
```

## Input controller

`fight_samatra` accepts a positive `max_ticks` and an optional `until`: `ready`, `gate_open`, or `gate_entered` (default). Consecutive phases preserve the controller's proof within the same boss generation while giving each phase its own tick bound. Splitting phases allows native OS screenshots of eight live generators and the opened gate before bomb entry.

The ship picker chooses escorts while generators remain. The controller steers toward living generators using actual thrust/turn/fire inputs. Pkunk escorts also use their energy-restoring special. After all eight generator deaths and the actual open-gate state, an escort retreats through ordinary Escape input. The next picker selects the bomb flagship, which is steered toward the gate. Completion requires all eight distinct runtime death events, an observed open gate, escort and bomb participation, and the boss runtime's actual bomb-gate collision.

The snapshot copies current generator IDs/positions/hit points, generator count, gate position/open state, selected species, bomb-carrier status and player position/facing/crew. Generator IDs are registered by the owning spawn callback. Their address values are identity keys only and are never dereferenced after observation. Battle view scrolling translates world positions, so neither position nor survivor order identifies a generator. The registry is cleared when a new boss generation begins.

## State predicates

Existing `assert_state` and `wait_for_state` accept these additional keys:

| Key | Meaning |
|---|---|
| `samatra:remaining` | Live generator counter, 0–8 |
| `samatra:destroyed` | Distinct generator deaths retained for this boss generation, 0–8 |
| `samatra:gate_open` | Live gate primitive has changed to its open state |
| `samatra:gate_entered` | Sticky actual bomb-carrier gate collision |
| `samatra:bomb_carrier` | Selected player ship is the bomb flagship |
| `samatra:selected_species` | Selected ship species ID, not fleet index |
| `ending:won` | Final encounter owner reached `WON_LAST_BATTLE` |
| `ending:presentation_started` | Final-presentation owner entered its slideshow |
| `ending:presentation_completed` | Final slideshow reached the end of its operation list, not Cancel |
| `ending:credits_started` | Victory credits initialized |
| `ending:credits_completed` | Victory credits reached their natural scroll completion |

Ending observations are sticky for the process and originate in Rust gameplay/presentation owners. The interpreter only reads them. Victory alone does not prove that either presentation or credits completed. Unavailable live boss fields do not compare as zero. The native predicate validator rechecks phase-specific completion labels, including all eight deaths and bomb entry for the terminal phase.
