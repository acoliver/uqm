# LCAR v2: the bootstrap evidence contract

LCAR is the evidence contract for behavioural claims about this port. It exists
because "I ran it and it worked" is not checkable, and because a proof that
cannot be replayed offline by someone else is an assertion, not evidence.

This document describes the implemented bootstrap producer in
`rust/src/bin/uqm_gameplay_proof.rs` and the S4 consumers in
`rust/xtask/src/ci/proof.rs` and `rust/xtask/src/ci/evidence.rs`. Its existing
filename is retained so incoming documentation links continue to work. The
producer and consumers now require LCAR v2; they do not accept LCAR v1 bundles
or resolved v1 receipts that omitted the actual steps.

- Emit a proof: `uqm-gameplay-proof run <repo> <production-manifest> <script> <output>`
- Validate one offline: `uqm-gameplay-proof validate <output>/lcar-v2.json`
- Validate a retained failure: `uqm-gameplay-proof validate <output>/failure-lcar-v2.json`
- Replay a passing bundle: `uqm-gameplay-proof replay <repo> <prior-bundle> <new-output>`
- Check rejection fixtures: `uqm-gameplay-proof validate-negative-fixtures`

## What the letters mean

- **L: Live.** The real built executable performed the behaviour under
  autonomous control. Compilation and unit tests do not satisfy this.
- **C: Complete.** The accepted implementation is active and the superseded
  provider is gone. No fallback, stub, dormant path or duplicate authority.
- **A: Automated.** Focused tests plus the full repository gate suite prevent
  the behaviour, ownership and integration from regressing.
- **R: Reproducible.** A reviewer can identify, validate offline and replay the
  exact source, executable, content, configuration, scenario, seed, commands,
  assertions, captures, logs and teardown outcome.

These are acceptance requirements, not claims that the current #34 checkpoint
has satisfied all four.

## Manifest

A passing result is `lcar-v2.json`; a failed result is `failure-lcar-v2.json`.
Both use schema identifier `uqm-lcar-v2`. The filename must agree with `passed`,
and both result files cannot coexist. Unknown fields are rejected.

| Field | Meaning |
|---|---|
| `schema` | `uqm-lcar-v2` |
| `passed` | Whether the bootstrap result contracts held |
| `first_failed_contract` | The first contract that failed, or null for a pass |
| `git_head` | Full 40-hex source commit bound to the production manifest |
| `command` | Exact executed child argument vector, with historical bundle paths |
| `environment` | Exactly `SDL_AUDIODRIVER=dummy` and `SDL_VIDEODRIVER=dummy` |
| `target` | Supported platform triple |
| `profile` | `release` |
| `features` | Ordered production features `audio_heart`, `linked_c_archive` |
| `renderer` | `sdl2-software-dummy` |
| `seed` | Nonzero u32 RNG initialization argument resolved from the script |
| `input_identity` | SHA-256 binding the complete replay inputs, described below |
| `provenance` | Content and configuration addresses |
| `process` | Child process receipt |
| `cleanup` | Parent teardown receipt |
| `artifacts` | Exact content-addressed file inventory |

Running or replaying requires a clean source worktree at the exact production
commit and a matching native target. There is no implemented dirty-source
exception. Offline validation reads retained inputs without requiring the old
workspace or historical output directory to exist. The child environment is
cleared before the two declared SDL variables are set.

### Resolved scenario v2 and selected/applied seeds

`run/resolved-scenario.json` has exactly `scenario` and `replay_identity`.
The closed `ResolvedScenarioRecord` type validates both. Its `scenario` has:

| Field | Meaning |
|---|---|
| `schema` | `uqm-resolved-scenario-v2` |
| `scenario_version` | Current script version, `2`, after migration |
| `name`, `fixture` | Scenario name and fixture identity |
| `requested_seed` | Selected u32 script seed before zero-sentinel resolution |
| `seed` | Resolved nonzero RNG boundary argument |
| `step_count`, `steps` | Count and complete typed, ordered actions and assertions |
| `start_scene` | Explicit starting scene or null |
| `max_input_ticks`, `max_presentations`, `max_wallclock_seconds` | Validated run budgets |
| `max_startup_seconds`, `max_idle_seconds` | Effective startup and idle budgets, including defaults |

Version 1 scripts remain readable through explicit migration. An absent seed
resolves to the existing automation default, `1437213463` (`0x55AA2317`), and an
absent fixture resolves to the scenario name. Version 2 requires an explicit
seed and fixture. Seeds above `u32::MAX` are rejected rather than truncated.

A selected zero is recorded as `requested_seed: 0`, `seed: 1`. The transitional
callers in `sc2/src/uqm/restart.c` and `sc2/src/uqm/battle.c` treat zero as no
automation seed, so returning zero would leave their time-derived seed active.
Nonzero u32 selections pass through unchanged. `seed` describes the argument
returned at RNG initialization, not the generator's normalized internal state.

The coordinator returns that resolved value at the Super Melee menu, Super
Melee battle, and new-game seed boundaries, and records the same value in each
`seed_application` trace entry. Inactive automation retains the caller's
fallback. Validators reject a seed entry that disagrees with the script, a seed
record without its payload, or a seed payload on another record kind. The
metadata alone does not prove a boundary was reached; trace and result checks
supply that evidence. Failure to publish the resolved receipt stops automation
with `TraceFailure` before actions execute.

### Input identity versus outcomes

The scenario `replay_identity` is SHA-256 of the compact serde JSON serialization
of the validated `ResolvedScenario` struct, in its declared field order. It
includes the versioned schema and every field above. Changing an action without
changing the step count changes this identity. Readers recompute it and require
the complete scenario to equal the parsed, validated retained script.

The top-level `input_identity` is SHA-256 of the compact JSON object serialized
from this material, with object keys sorted by the JSON map serialization:

- `schema: "uqm-replay-input-v1"` and `scenario_identity` from the resolved receipt;
- `seed`, `git_head`, `target`, `profile`, `features`, `renderer`, `environment`;
- `production_manifest_sha256`, `executable_sha256`, `script_sha256`,
  `content_tree_sha256`, `initial_config_tree_sha256` from provenance.

The input-material version is distinct from the LCAR and resolved-scenario
versions. The producer and S4 consumer recompute this digest; checking its shape
alone is insufficient. Raw script bytes are bound as well as resolved actions.

Final configuration, process outcome, absolute bundle location, and capture
pixels are not input identity. Their evidence remains independently validated.
Replay compares outcomes only after validating the prior bundle and verifying
the copied input identities. It compares ordered `seed_application`,
`semantic_assertion`, `menu_transition`, `checkpoint`, and `terminal` trace
records. It normalizes elapsed time and input/presentation counters to zero,
renumbers sequence values, and removes presentation data. Other retained fields
must match exactly. Timing-dependent values embedded in semantic labels can
still differ; this is not a tolerance-based gameplay oracle.

`compare-battle` also requires matching verified input identities before
comparing outcomes. Neither comparison establishes deterministic pixels.

### Provenance and retained replay inputs

`provenance` contains `production_manifest_sha256`, `executable_sha256`,
`script_sha256`, `content_tree_sha256`, `initial_config_tree_sha256`, and
`final_config_tree_sha256`. The retained production manifest must bind the same
source, build profile, target, features and executable digest.

Replay copies the verified retained production manifest, executable, script and
content into a fresh output bundle. It does not substitute the repository's
current executable or content. Content bytes are retained and executed from
`snapshots/sc2/content/`, not merely described by a digest. The copied identities
are checked before spawning.

The supported initial configuration is a fresh empty directory. Its tree
snapshot must have no entries, and replay recreates that empty configuration.
Arbitrary starting profiles and save states are not supported by this bootstrap
replay path. The producer records the final configuration tree identity before
removing the working `config/` directory; it does not copy those file bytes to a
separate snapshot directory. Files left by failed cleanup have the
`retained_config_file` role under `config/`. Final configuration is not reused as
replay input.

Tree snapshots use `uqm-tree-identity-v1`, with `root_role`, `tree_sha256` and
ordered `entries` containing `path`, `sha256`, `bytes`. For each sorted entry,
the tree digest hashes path bytes, a NUL byte, digest text, a NUL byte, decimal
byte length and a newline. Retained content files must exactly match the content
tree, including empty files. S4 also checks retained final-config files against
the final-config tree. Since successful cleanup removes those files, a nonempty
final-config tree cannot satisfy that S4 comparison after successful cleanup.
The fixture checks do not establish a live configuration-writing run through
this producer/consumer boundary.

All historical command operands must refer to one recorded bundle root:
`snapshots/uqm`, `--contentdir=<root>/snapshots/sc2/content`,
`--configdir=<root>/config`, `--automation-script=<root>/snapshots/script.json`,
`--automation-output=<root>/run`, followed by `--res=640x480`, `--windowed`, and
`--scroll=pc`. Validation opens only the current retained artifacts. A relocated
bundle therefore needs no manifest rewrite or access to those old absolute
paths.

### Process and cleanup receipts

`process` contains `pid`, `start_time`, `executable_sha256`, `exit_code`, `signal`,
`term_sent`, `kill_sent`, `stdout_bytes`, `stderr_bytes`, `output_drained`, and
`orphan_check_passed`. Its executable digest must match provenance.

`cleanup` contains `exact_child_reaped`, `orphan_check_passed`, `output_drained`,
and `config_root_removed`. These facts must agree with process and retained
result evidence. A failed gameplay run and a successful cleanup are distinct
outcomes.

### Artifacts

Every artifact carries `role`, `path`, `bytes` and `sha256`. Paths are relative,
must not traverse or repeat, and must match their declared role.

| Role | Retained path |
|---|---|
| `stdout_log`, `stderr_log` | `stdout.log`, `stderr.log` |
| `trace` | `run/trace.jsonl` |
| `teardown_receipt` | `run/teardown-complete.json` |
| `resolved_scenario` | `run/resolved-scenario.json` |
| `capture` | `run/captures/<name>.png` |
| `production_manifest_snapshot` | `snapshots/production-manifest.json` |
| `executable_snapshot` | `snapshots/uqm` |
| `script_snapshot` | `snapshots/script.json` |
| `content_identity_snapshot` | `snapshots/content-identity.json` |
| `content_snapshot_file` | `snapshots/sc2/content/<relative-path>` |
| `initial_config_snapshot`, `final_config_snapshot` | `snapshots/config-initial.json`, `snapshots/config-final.json` |
| `retained_config_file` | `config/<relative-path>` when cleanup leaves files |

The inventory must account for every retained file except its result manifest.
Extra `.tmp` files are not excluded. Missing files, changed bytes or lengths,
wrong roles, duplicate paths and conflicting results fail validation. Empty
content files are valid and must still be inventoried. Both result paths require
the resolved receipt; passing evidence also requires trace, teardown and at
least one capture. A trace, when present, must be nonempty and newline-terminated.
S4 retention uses its bounded, no-follow snapshot and transactional publisher;
its consumer retains the existing trusted command, source and authority-profile
bindings in addition to the v2 input checks.

## Failure contracts

`first_failed_contract` names the first failing bootstrap contract: `timeout`,
`reader`, `budget`, `nonzero_child`, `missing_teardown`, `semantic_evidence`,
`teardown_evidence`, or `config_cleanup`. Offline acceptance of a failure manifest
means that its failure evidence is consistent, not that gameplay passed.
Failures before sufficient evidence exists do not guarantee a valid LCAR bundle.
The replay command accepts passing v2 bundles only, not failure bundles, legacy
LCAR v1, or native acceptance/suite layouts.

## Assertions and captures

Semantic assertions in a passing trace are correlated to presented-frame
generation. The bootstrap producer runs with SDL dummy drivers. Its PNG captures
come from the game's internal draw surface, not an OS window, so they cannot
establish what a player saw or expose a present/swap defect. A visual claim needs
screenshots from the actual presented window.

A capture step may be marked `expect_change`, requiring it to differ from the
preceding capture. That remains separate from replay's normalized semantic
comparison, which does not compare capture pixels.

## Rejection coverage and checkpoint limits

The command-level negative fixtures cover empty inventory, traversal, duplicate
paths, malformed provenance, changed artifacts, unknown fields, forged failure
contracts and changed trace sequence. Embedded runner and S4 tests additionally
cover complete-action identity changes, receipt/seed tampering, rehashed input
chains, empty content retention, extra files, path substitution and relocation.
These fixture tests do not launch their retained executable as a game.

This #34 internal checkpoint includes no live production run or replay, no
real-window screenshots, and no proof of the full scenario suite or gallery.
A clean exact-head production run, actual retained-input replay, native-window
proof, live S4 bundle validation and whole-PR CI evidence remain required. Linux
containment compilation/execution is not established by the local macOS tests;
the attempted cross-check lacked its native compiler and dependency sysroot.

The base-owned controller/policy transition in [#210](https://github.com/acoliver/uqm/issues/210)
has no approved migration route recorded. Head-only edits do not resolve the
base/head authority byte check, and push-context success is not merge-context
proof. No authority, workflow, script or evidence-limit change is authorized by
this checkpoint.

Native admission also remains incompatible with 15 pinned scripts declaring
300-900 seconds: admission requires script seconds plus 45 to be strictly below
300. The full selection's declared ceilings total 10,500 seconds, compared with
a 3,600-second native step and a 9,300-second S4 run. These are static ceilings,
not measured runtime lower bounds. Scenario-equivalence and budget decisions
require explicit approval before changing scripts or policy; neither omitting
scenarios nor increasing limits is an approved workaround.

Remaining native work includes complete selection accounting, shared input
snapshots with tested protection against same-UID writers, domain-aware OS
checkpoint receipts, native replay and manifest-based success/failure gallery
integration, plus actual lifecycle and fault-boundary proof. Issue #34 remains
one indivisible delivery in draft PR #209. This checkpoint is neither final
review nor PR completion.
