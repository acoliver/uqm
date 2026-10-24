# Sa-Matra automation and ending observations

## Connected campaign completed

`campaign-s13-connected-ending` starts from `connected-samatra-remnants.sav`, earned by defeating the six remnant guards after the Talking Pet distraction. Its parent chain includes the Mycon lure, Sun Device, Chmmr bomb conversion and paid final fleet. The producer `pt-p12-30` and independent full-state reload `pt-p12-32` passed native-window acceptance. The checkpoint retains the actual losses: five escorts, flagship crew 50, fuel 4663 and RU 961616 on 14 June 2158.

Normal Cyborg mode skips the Sa-Matra battle in the original encounter rules. The remnant producer disables it through ordinary Game settings before saving. The ending scenario then uses normal menu, ship-selection, flight and weapon inputs. Avatar steering corrects velocity even while a generator is in range and fires at nearby hazards before resuming its generator approach.

Native run `pt-p12-33` passed both suite and native-window acceptance, exiting 0. Its runtime trace records all eight distinct generator deaths, the open gate, hazard cleanup, escort retreat, bomb selection and actual gate collision. It then records victory, natural final-presentation completion, post-outtake credit control readiness and natural credit-roll completion. The trace ends with `success` at 669231 ms. Ordinary Cancel skips outtake speech after the final presentation; Down accelerates the credit roll only after its controls are ready. Neither input replaces a completion predicate.

`samatra-dev-fixture-v1` remains development-only and is not part of this campaign ancestry. The fixture observations below describe earlier controller development, not additional campaign proof.

## Development fixture

The `samatra_dev_fixture` start scene requires a debug build or the `debug-process` feature used by the native runner. It prepares twelve Pkunk escorts, a crewed weaponless bomb flagship with engines/jets, the converted-bomb story state and a Sa-Matra star identity. These grants are fixture setup, not story-earned progress. Normal last-battle dispatch then starts the Talking Pet conversation and real boss battle.

The controller never grants damage, removes generators, selects ships through a callback, changes activity, or marks victory. Only the fixture initializer writes setup state. Runtime observations remain read-only.

Run from the lane checkout through the shared native-window launcher:

```sh
ROOT=/Volumes/XS1000/acoliver/projects/uqm
bash "$ROOT/tmp/pt-runs/iterm-run.sh" end-fixture-current samatra-dev-fixture-v1 "$ROOT/tmp/wt/closing"
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

## Verification and evidence limits

The current local changes pass formatting, workspace clippy with `-D warnings`, workspace tests (6058 library tests and 43 gameplay-proof tests, with 7 library tests ignored), and the linked debug build. These checks do not establish combat reliability or campaign ancestry.

Native fixture run 7 recorded eight generator deaths, an open gate, escort retreat, bomb selection, actual gate collision, victory and presentation start. It did not prove natural presentation or credits completion. The outtake path exposed a communication-lock deadlock; the current runtime releases that lock during input-driven speech.

Runs 8, 10 and 11 exhausted escorts with three, eight and two generators left respectively. Run 9 failed to compile the velocity observer's original output-pointer types; the corrected observer copies the descriptor and uses the ABI's `i16` outputs. Run 12 destroyed all eight generators but the bomb died before gate entry. It was stopped after that loss and finalized as a signal-9 failure, not a successful native scenario.

Run 13 uses twelve fixture Pkunk escorts, retained targets, velocity-aware steering and post-generator hazard cleanup. Its trace records all eight deaths, open gate, no remaining hazards before bomb selection, actual bomb-gate collision, `ending:won=1` and `ending:presentation_started=1`. It also recorded natural `ending:presentation_completed=1` and `ending:credits_started=1`. It reached the 840-second wall budget during outtakes without recording credits completion, so native acceptance failed with `wall_timeout`. Artifacts are under `tmp/pt-runs/end-fixture-13` in the repository root. This is one fixture run and does not establish repeatability.

Image-comparer inspected original OS screenshots 0007 and 0009 from run 12. Both contain the Sa-Matra battle and HUD without clear image corruption. It could not confidently identify the generators or gate from these stills. The visual review does not replace the runtime gate/death observations, and these battle images do not establish an ending. Image-comparer separately inspected run 13 OS screenshots 0015, 0017 and 0019: 0015 showed battle art; 0017 and 0019 showed the illustrated “Thank you for playing The Ur-Quan Masters.” panel. No clear rendering defect was identified. Those stills did not show scrolling credit names or prove roll completion.

PR 421 includes the steering, hazard cleanup and credits-readiness follow-ups. Andrew authorized publication and merge for this landing task. The development fixture remains excluded from campaign proof; S12–S14 are pending.

Run 14 keeps the same gameplay code and completion predicates. After the credits-start capture, it holds normal Cancel input to skip outtake speech, releases it, then holds normal Down input to accelerate the player-controlled credit roll. It never cancels the final slideshow or calls a credit-completion callback. The final wait still requires the credits owner's natural bottom-of-roll event. It recorded victory and natural presentation completion, but again reached `wall_timeout` without credit completion. Its fixed Down interval happened during outtakes, before the credit-roll input loop could consume it.

Run 15 replaces that fixed speech interval with the runtime-owned `ending:credits_controls_ready` observation, recorded after outtakes return. Cancel remains held only until that boundary. Down then accelerates the credit roll through normal controls. The same wall budget and natural-completion predicate remain in force. Its first launch (run 15) was rejected by source admission because the additional bounded wait exceeded the declared input-callback total. The script now declares 110000 input callbacks and 220000 presentations to cover its static sum; the 840-second wall budget is unchanged. Run 16 passed: `suite-index.json` and `native-acceptance.json` both contain `passed: true`; the child exited 0 without TERM or KILL. Its runtime trace ended with `success` at 630738 ms, 19702 admitted input callbacks and 31155 presentations. It recorded all eight deaths, an open gate, hazard cleanup, ordinary escort retreat and bomb selection, actual gate entry, victory, natural presentation completion, post-outtake control readiness and natural credit-roll completion. Artifacts are under `tmp/pt-runs/end-fixture-16`.

Image-comparer inspected original run 16 OS screenshots 0017, 0026 and 0027. Screenshot 0017 shows the illustrated thank-you panel. The two roll-completion checkpoints show a near-black starfield with small green edge marks, not readable credit names. The stills cannot establish whether those marks or dark frames are intended. Runtime completion and native-window acceptance passed, but visual confirmation of scrolling credit names remains unproven.

Runs 13, 14 and 16 each reached bomb-gate entry with the current controller, but only run 16 completed the full scenario. This does not establish fleet-independent reliability or independence from the Pkunk's normal randomized resurrection. No campaign segment is promoted by these fixture results.
