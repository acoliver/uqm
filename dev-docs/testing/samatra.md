# Sa-Matra automation and ending observations

## Connected campaign completed

`campaign-s13-connected-ending` starts from `connected-samatra-remnants.sav`, earned by defeating the six remnant guards after the Talking Pet distraction. Its parent chain includes the Mycon lure, Sun Device, Chmmr bomb conversion and paid final fleet. The producer `pt-p12-30` and independent full-state reload `pt-p12-32` passed native-window acceptance. The checkpoint retains the actual losses: five escorts, flagship crew 50, fuel 4663 and RU 961616 on 14 June 2158.

Normal Cyborg mode skips the Sa-Matra battle in the original encounter rules. The remnant producer disables it through ordinary Game settings before saving. The ending scenario then uses normal menu, ship-selection, flight and weapon inputs. Avatar steering corrects velocity even while a generator is in range and fires at nearby hazards before resuming its generator approach.

Native run `pt-p12-33` passed both suite and native-window acceptance, exiting 0. Its runtime trace records all eight distinct generator deaths, the open gate, hazard cleanup, escort retreat, bomb selection and actual gate collision. It then records victory, natural final-presentation completion, post-outtake credit control readiness and natural credit-roll completion. The trace ends with `success` at 669231 ms. Ordinary Cancel skips outtake speech after the final presentation; Down accelerates the credit roll only after its controls are ready. Neither input replaces a completion predicate.

`samatra-dev-fixture-v1` remains development-only and is not part of this campaign ancestry. The fixture observations below describe earlier controller development, not additional campaign proof.

## Development fixture

The `samatra_dev_fixture` start scene requires a debug build or the `debug-process` feature used by the native runner. It prepares twelve Pkunk escorts, a crewed weaponless bomb flagship with engines/jets, the converted-bomb story state and a Sa-Matra star identity. These grants are fixture setup, not story-earned progress. Normal last-battle dispatch then starts the Talking Pet conversation and real boss battle.

The controller never grants damage, removes generators, selects ships through a callback, changes activity, or marks victory. Only the fixture initializer writes setup state. Runtime observations remain read-only.

Run from the repository root on a macOS desktop:

```sh
cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run \
  --scenario samatra-dev-fixture-v1 --artifacts tmp/samatra-fixture \
  --content sc2/content/packages
```

## Input controller

`fight_samatra` accepts a positive `max_ticks` and an optional `until`: `ready`, `gate_open`, or `gate_entered` (default). Consecutive phases preserve the controller's proof within the same boss generation while giving each phase its own tick bound. Splitting phases allows native OS screenshots of eight live generators and the opened gate before bomb entry.

The ship picker chooses escorts while generators or hazards remain. The controller retains a living generator target, routes around the boss exterior, and corrects velocity with actual thrust/turn/fire inputs. Pkunk aiming accounts for inherited missile velocity; their special restores energy through ordinary input. Once the gate opens, escorts destroy surviving comets and sentinels before retreating through ordinary Escape input. Leaving these hazards alive killed the one-crew bomb flagship in fixture run 12. The next picker selects the bomb flagship, which is steered toward the gate. Completion requires all eight distinct runtime death events, an observed open gate, escort and bomb participation, and the boss runtime's actual bomb-gate collision.

The snapshot copies current generator IDs/positions/hit points, generator count, gate position/open state, selected species, bomb-carrier status, player position/facing/crew, one-frame velocity and live hazard positions/velocities. Generator IDs are registered by the owning spawn callback. Their address values are identity keys only and are never dereferenced after observation. Battle view scrolling translates world positions, so neither position nor survivor order identifies a generator. The registry is cleared when a new boss generation begins.

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
| `ending:credits_controls_ready` | Outtakes returned and the victory credit-roll input loop is ready |
| `ending:credits_completed` | Victory credits reached their natural scroll completion |

Ending observations are sticky for the process and originate in Rust gameplay/presentation owners. The interpreter only reads them. Victory alone does not prove that either presentation or credits completed. Unavailable live boss fields do not compare as zero. The native predicate validator rechecks phase-specific completion labels, including all eight deaths and bomb entry for the terminal phase.
