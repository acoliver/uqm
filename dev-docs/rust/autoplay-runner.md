# Autoplay runner contract

Implementation for Issue #34 stays in PR #209, including its related controller-policy and provenance corrections. Source admission, a successful game child, validated native evidence and trusted CI acceptance are separate results. None implies the next.

## Deadlines

The script's `max_wallclock_seconds` is a watchdog ceiling, not a reservation or minimum execution window. `watchdog::reduce` makes a run terminal when its elapsed time reaches that ceiling. `ChildSession` independently supervises the whole native child with S4's 300,000 ms timeout and 5,000 ms kill grace. Startup, script execution and checkpoint observation occur inside that lifetime. Observer timeout values bound individual operations, not additional runtime after the script.

The admission condition that added the script ceiling, observer timeout and kill grace was incorrect. It rejected 15 valid pinned scripts before they could execute. Source preflight now validates every selected script's exact bytes, typed actions, action budgets and native runtime ordering before the linked build. It records the unchanged script budgets alongside the enclosing child timeout. No script budget, action, assertion, seed or pin is rewritten. All 36 pinned scripts, including the four declared fixtures, are checked in the regression; the gameplay selection remains the 32 nonfixtures.

The 3,600-second native step, 9,300-second complete S4 run, native child bounds and artifact limits are unchanged. Enclosing deadlines may fire before an inner watchdog. That is a failed execution with incomplete actions, never a passing shortened script. Successful suite evidence must account for the exact required selection and every action/checkpoint. Adding maximum watchdog values is not a predicted runtime and does not establish either feasibility or infeasibility of a successful run. Full-suite execution must demonstrate completion within the actual enclosing bounds; source admission makes no such claim.

This policy is chosen before new native gameplay results. It replaces the earlier planning assumption that each inner watchdog reserves its full duration. Fixed waits and input sequences remain intact. There is no CPU-time substitution, checkpoint timer reset, scenario skipping or distribution across tuples.

## Build provenance

`production` records one build. `prove` performs two clean builds and records their matching identities and artifact vectors. `verify` requires that proof and checks it against live inputs without rebuilding. Use `prove` then `verify` for production proof, or `package`, which performs both. A single-build manifest remains usable by the local gameplay runner but does not pass the stronger production verifier. See [reproducible-build.md](reproducible-build.md) for exact commands.

## Native images and execution

The native binary and the external xtask controller call the same Rust `automation::native_runner` implementation. The executable dispatches observer-helper arguments back into that implementation; the observer is the hashed controller executable, not a helper supplied by the game under test.

Each native screenshot retains the original OS PNG as well as the normalized client image. The `uqm-native-window-proof-v3` receipt and `uqm-native-window-acceptance-v3` manifest bind both paths, byte lengths and hashes, together with the selected checkpoint plan. Offline validation decodes within the existing byte and allocation bounds, reapplies the recorded OS/client geometry, and requires the derived PNG to match the retained normalized image. Removing the original, replacing its bytes, or substituting a different valid PNG and updating only its metadata fails validation. Internal automation captures remain supplementary renderer images.

The controller derives checkpoints from the exact selected script and its full resolved behavior, including seed and action order. The required 32-scenario selection has 132 capture actions; the full inventory including fixtures has 135. Assertions, semantic waits, navigation, semantic selections and fixture setup also create obligations. Ordinary input/presentation waits and key sequences remain unchanged. Stable/Playable presentation floors, accepted battle input, battle-frame floors and material scene change remain additional requirements for `linked-playable-v1`; arbitrary menu and short battle scripts do not inherit those floors.

Completing an obligated action holds scheduler advancement until its next committed presentation. Semantic predicates precede that presentation; the supplementary capture record follows it. A live, bounded trace journal is flushed before the corresponding native publication. The controller validates the exact action marker and successful predicate, captures the real OS window, verifies post-capture geometry and unchanged child publication, then acknowledges. Queued menu transitions and readiness remain pending behind this barrier. A missing image cannot be acknowledged as a completed checkpoint, including during final-publication handling. Script budgets and S4 ceilings are not reset or increased by the barrier.

Offline validation reconstructs the plan from retained source, rejects missing/duplicate/reordered obligations and stale consumed generations, and binds each marker to its first publication and exactly one OS screenshot. Script `expect_change` compares decoded RGB pixels against the preceding script capture, excluding intervening semantic checkpoints. A different PNG encoding, metadata or alpha channel alone does not satisfy it. Linked Stable/Playable images retain their stronger material-change threshold. Final semantic totals bind to the last acknowledged publication, which can follow the last required screenshot.

Behavioral tests cover these contracts, but the unchanged full gameplay suite has not executed successfully with the new checkpoints. Shared input retention and pre-write artifact reservations are wired through the suite and native runner as described below. The runner command surface, the retained-input replay and the checkpoint gallery are described under [Runner command surface](#runner-command-surface) below; none of them has been exercised against a successful native gameplay run. `suite-gallery.json`/`.html` remain the flat diagnostic catalog the suite adapter publishes.

## Runner command surface

One command drives the native runner, on the same xtask binary that acts as the trusted controller and observer:

```sh
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- native <command>
```

| Command | Result |
|---|---|
| `list [DOMAIN] [--json]` | The pinned scenario inventory with its matrix tags, declared seed, action and capture counts, and whether the required suite includes it. |
| `run --artifacts DIR --content DIR [--scenario NAME]... [--all] [--seed N] [--profile release] [--renderer sdl2-native-window] [--platform macos]` | Executes the selected suite through the existing `native-test` adapter. |
| `validate BUNDLE [--scenario NAME]... [--all]` | The existing offline suite validation. Not a replay. |
| `report BUNDLE [--json]` | Suite result, artifact index path, per-scenario state, the checkpoint obligation totals, each child's observed fault class and the teardown receipt. A failed suite is printed in full and then returned as a failure, so the exit status and the report agree. |
| `replay BUNDLE --output DIR` | Re-executes a recorded run from its retained inputs. |
| `gallery BUNDLE --output DIR [--json]` | The checkpoint/predicate catalog. |

`--scenario` names pinned scenarios by name or repository path and `--all` names the complete required 32-scenario suite. Neither reaches outside the authority's pinned inventory. `--artifacts` and `--content` bind the same evidence and content roots the environment binds in CI, and both routes reach the same suite accounting, so a locally produced bundle has the layout the offline validator already reads. With no selection the run proves the single authority scenario, as it did before the command existed.

`--seed`, `--profile`, `--renderer` and `--platform` are declarations that get checked, not switches that get applied. A `--seed` that differs from a selected script's own seed fails the command; it never rewrites the script, the resolved scenario, or the checkpoint identities derived from them. `--profile` accepts only the canonical release build the acceptance links, `--renderer` only the real OS window this command drives, and `--platform` only the platform the authority declares. Each subcommand refuses options it does not act on, so an option that was never read cannot look like one that was checked.

## Retained-input replay

`native replay` reads a prior suite's retained source, executable, content package, script bytes, seed, runtime contract, acceptance policy, linked-build receipt and fresh initial configuration, stages them, and launches the window again from exactly those bytes through the same launcher `native run` uses. It then re-derives the identity from the bundle it produced and refuses any difference, naming every field that changed rather than only the first.

Offline validation is not replay, and both commands say so. `native validate` states that it checked a retained bundle's own consistency without starting the game. `native replay` refuses a selection that is not the recorded one instead of silently narrowing it, and requires a recorded suite whose scenarios all completed. The outcome comparison covers satisfied checkpoint obligations, presentation-floor stages, the window result and the child exit status; frame counts, image bytes and wall-clock timing belong to one execution of a real window and are excluded, which the output states.

No replay has been executed. That needs an unlocked Aqua session and a successful full-suite run, and neither exists yet.

## Checkpoint gallery

`native gallery` reconstructs each selected script's checkpoint obligations from the script bytes retained in the suite's shared store, and binds every obligation to its original OS screenshot and to the normalized client image derived from that screenshot, as two entries with separate identities. Obligations that produced no image are rendered as `missing`, `failed` or `not_run` rather than omitted, including for scenarios that were never attempted, because an absent row reads as a suite with fewer obligations rather than a suite that lost images. Presentation-floor images are listed separately, and the game's own renderer readbacks appear under their own heading, labelled as internal captures and never as OS screenshots.

The catalog is written to a fresh directory outside the bundle: writing into the bundle would change the inventory the suite validator checks. Its JSON paths are relative to the suite root and its HTML links carry a relative prefix, so the catalog and the bundle relocate together. Every interpolated value is HTML-escaped and an image the retained inventory does not hold is captioned as absent rather than linked.

On macOS, the command must execute in an existing Aqua session whose real, effective and launchd-manager UIDs match. A shell reporting `Background` does not establish that no such session exists: LaunchServices can open a task-owned command in an already-running Terminal application. This does not authorize identity changes, privilege elevation, security-setting changes or candidate self-attestation. A local launch through that route is not trusted merge-context evidence.

For a focused development diagnostic from Aqua, use the pinned toolchain and the directory containing the single content package, rather than the expanded game-content tree:

```sh
UQM_CI_NATIVE_ACCEPTANCE_EVIDENCE_ROOT="$FRESH_ABSOLUTE_EVIDENCE_DIRECTORY" \
UQM_CI_NATIVE_CONTENT_ROOT="$REPOSITORY/sc2/content/packages" \
UQM_CI_AUTOPLAY_SCENARIOS=linked-playable-v1 \
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- native-test
```

This command does not replace required full-suite CI acceptance. Do not start it while another game is running, and do not terminate unrelated user sessions to make room.

## Durable native suite accounting

`native-test` creates `suite-request.json` before source admission or compilation. Every selected pin has a row from the first event. Singleton and multiple-scenario requests use the same `scenarios/0000`, `scenarios/0001` layout. The index order is the exact requested order, not directory discovery.

The controller persists an immutable `suite-events/NNNNNN.json` event and atomically replaces `suite-status.json` at each transition. A row becomes `attempted` before scenario setup. It becomes `completed` only after native proof validation and verification of the selected script identity. On failure, the active row becomes `failed` and all later rows remain `not_run`; a preflight/build failure leaves all rows `not_run` with the suite-level first failure. Elapsed durations are measured, not inferred from watchdog ceilings. Finalization refuses an active attempt, omitted selection, or empty-suite success.

A run that is killed publishes its last transition and stops there, leaving a journal with no manifest, index or catalog. Claiming that root again does not start a new suite. The controller reads the retained journal, holds it to the same requested selection, records the interruption against whichever scenario row was active, and finalizes it into the failure bundle the interrupted run never published. The claim then returns that outcome as an error instead of a fresh suite, so an interrupted run is recovered rather than silently restarted. The interrupted attempt keeps the duration the journal recorded, because the process that measured it is gone and this one cannot measure it after the fact; the suite-level elapsed time carries forward so it still only rises. A journal that is already final is refused outright, and a journal whose request does not match the current selection is refused without publishing anything over it. This is journal recovery only: it does not resume execution, and no interrupted live run has been recovered yet.

## Child faults and the teardown receipt

The controller supervises each scenario child and records what it saw, separately from the manifest the child writes for itself. A child that dies before it can write anything still leaves that record. `suite-teardown.json` (`uqm-native-suite-teardown-v1`) carries one row per selected scenario holding the observed fault class — `completed`, `exit`, `signal`, `timeout`, `output_limit`, `escaped_descendants`, `launch_failed`, `supervision_failed`, or `not_launched` when the attempt failed before any child existed — together with the exit code, signal, timeout flag, termination reason and signal, process-group cleanup result, pipe-drain result and any descendant the controller found still owning the group.

A fault is not a dirty teardown. A child that crashed, timed out, was signalled or was killed for exceeding an output limit still leaves the host clean when its process group was verified empty and its pipes drained. `process_state_clear` is true only when every attempted row was supervised by the publishing process and each one met that bar; an escaped descendant, an unverified group or an undrained pipe makes it false. A row that was never attempted created no process state and is neutral. Journal recovery supervised none of the children it finalizes, so it leaves every row unsupervised and never claims a clear state. The receipt states its own scope: the controller's own children, not host state it never owned.

A faulted child's captured stdout and stderr are retained, bounded, as `suite-faults/NNNN.stdout.log` and `suite-faults/NNNN.stderr.log`, with the full observed length recorded beside the retained one. They live outside `scenarios/` because a scenario directory's inventory is bound to the child's own proof; controller output added there would invalidate it. The receipt and both logs are reserved with the rest of the suite diagnostics before any scenario may spend the artifact budget, so an exhausted budget still publishes the record of why it was exhausted.

Offline validation reads the receipt with the rest of the bundle. It refuses a receipt covering a different selection, a row whose fields contradict the class it declares, a scenario recorded as completed whose child did not complete, a supervised child attached to a scenario that never ran, a retained log that differs from its recorded identity, and a `process_state_clear` value the rows themselves do not derive. These are accounting contracts exercised with synthetic supervision rows; they are not evidence that a game ran.

Every ordinary return through this adapter publishes `suite-manifest.json`, `suite-teardown.json`, `suite-index.json`, `suite-gallery.json` and `suite-gallery.html`. The index covers all retained regular files except itself, including events, diagnostics and nested native bundles. The HTML/JSON catalog distinguishes original OS paths, derived client crops and supplementary renderer images. It is a flat diagnostic catalog rather than the checkpoint/predicate gallery, which `native gallery` renders separately from the retained script bytes. Missing storage can still prevent publication; that error is returned alongside the original execution error rather than reported as successful retention.

Validate a retained local bundle with the exact same requested selection:

```sh
UQM_CI_AUTOPLAY_SCENARIOS='linked-playable-v1 main-menu-v1' \
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- \
  native-suite-validate "$ABSOLUTE_SUITE_ROOT"
```

The validator uses the existing bounded, no-follow evidence snapshot reader. It verifies the selected pins, complete member identities, event order, legal transitions, final status, deterministic catalog and every completed native proof. It rejects rehashed history that erases an attempt, output attributed to a `not_run` or unselected row, substituted completed-script identities and a changed catalog. A valid failed bundle prints its manifest but returns nonzero with `native suite diagnostics validated; gameplay suite failed`. It does not replay the game or confer merge-context acceptance.

S4's existing failure role retains this uniform suite layout and validates its pinned selection, journal and any nested child failure receipts. A failure bundle's own selection is accepted only as a diagnostic description, never as proof that the event-required selection ran. The successful collector and offline reader now consume the same suite layout, require all event-selected scenarios to complete, and resolve each proof's protected shared descriptors while preserving linked-build/source/policy and content/script/runtime correlations. Flat successful native layouts are rejected. A failed postprocessing validation retains bounded native output under the existing diagnostic contract.

The plan and gate controllers independently derive the selection from trusted workflow event and Git revisions. PRs use the existing changed-path mapping; push, scheduled and manual jobs require the original full suite. The binding is retained in S4's source receipt rather than accepted from a scenario manifest. [The S4 gate documentation](ci-gates.md) describes the bounded candidate-policy admission rules implemented with #210 in the same PR. The controller currently installed on main cannot execute those new rules. Tests of this candidate against the old base policy do not establish hosted deployment or gameplay success. Live full-suite checkpoint proof, interrupted-controller journal recovery, final runner integration and hosted controller validation remain required before PR #209 can satisfy Issue #34.

## Shared inputs, configuration and capacity

The suite controller snapshots the executable, pinned content package, selected script bytes, content version and linked-build proof into `shared/objects/SHA256` once. `shared/descriptor.json` binds logical member names to those objects. Each scenario retains `shared-inputs.json`, which binds its selected logical members and the descriptor identity. Native receipt inputs keep their logical `inputs/` names. Offline readers resolve them through bounded, no-follow descriptor reads and verify the selected object bytes. Relocating the whole suite does not require the original repository or executable path. This is retained-input validation; a final native game replay command is still outstanding.

Shared object claims are exclusive, identical bytes reuse one object, and logical-name reuse is refused. The controller verifies the snapshot before each scenario and after its child returns. Runtime executable/script/package aliases are descriptor-relative hardlinks, verified before launch and removed before final inventory. Linked archives are not recopied into each scenario.

The game has the Aqua controller's UID, so file modes alone do not protect shared inputs. The native launch applies macOS Seatbelt restrictions before executing the candidate. Candidate writes are limited to fresh config and runtime automation scratch, with an explicit denial for the controller acknowledgement file. Forking, execution of another image, signals to other processes and privileged task-port access are denied. Shared paths are unreadable to the candidate; only its selected runtime aliases are exposed. `ChildSession::spawn_leaf` registers the process group but closes the controller registration capabilities on exec. The parent keeps cleanup ownership. Kernel tests exercise same-UID shared-object mutation, link/symlink attacks and acknowledgement write/rename/removal refusal. No host accounts, security settings or privileges are changed. These tests do not establish compatibility with the real game or exhaust every macOS service-mediated attack.

Each scenario starts with an empty `config/` and retains `config-initial.json`. After reaping the child, the controller copies all physical configuration members into `config-final/`, records their identities in `config-final.json`, and removes the mutable tree. Runtime automation output follows the same bounded import into `automation/`. These inventories do not exclude files whose names happen to match acceptance envelopes. Success requires both config receipts, exact final identities and absent mutable config/runtime roots. Failure validation checks any retained receipts and rejects false cleanup claims; early preparation or retention failures can lack a receipt and remain failed diagnostics.

The suite shares one artifact budget across immutable objects, directory/path claims, scenario allowances and reserved suite diagnostics. It reserves bounded journal/index/catalog publication before ordinary output. Before each serial launch, it derives the scenario's remaining allowance with suite-prefix path charges. The native runner reserves its success/failure envelopes first, then claims controller writes and imported candidate bytes before opening retained destinations. Duplicate claims, over-limit writes and file/directory collisions fail without overwriting earlier evidence. Failed and partial writes keep their claims. After return, the suite charges the actual retained scenario before another serial launch. Capacity not used by retained scenario members remains available; failed claims within a running scenario are not refunded.

These are retained-evidence limits, not filesystem allocation guarantees. Candidate scratch and materialized content are not a kernel aggregate disk quota. Actual disk exhaustion can prevent even reserved diagnostics from being written. Missing-storage, interruption and actual-game proofs remain required; the reservation tests demonstrate logical capacity and failure publication rather than guaranteed physical disk availability. No script, pin or S4 limit is raised to make a run fit.

## Native observation diagnostics

Acknowledging a pre-binding publication now validates and records its semantic counters. Previously, those acknowledgements advanced the publication sequence without updating counters, and finalization could report a semantic mismatch before reporting the missing window binding. Regression coverage exercises both counter retention and rejection of a regressing publication.

The observer retains `os-observation-N.json` containing the child publication, OS visibility/geometry and containment result before acknowledging a found window. These are diagnostics, not screenshots. In the September 8 local attempts, SDL reported the required client `(80,80,1280,960)` while CoreGraphics eventually reported `(93,58,1254,972)`. The client was not contained and proof failed at `StableFloor`. A live window-specific OS capture also failed while read-only console diagnostics reported a locked Aqua session. No OS PNG was obtained and no visual judgment was possible. Those observations establish the failed geometry contract and the locked session; an unlocked rerun is still needed to determine whether the geometry mismatch persists independently of the session state. Do not reduce bounds or alter host security to obtain a pass.
