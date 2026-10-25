# Bootstrap LCAR v2 format

`rust/src/bin/uqm_gameplay_proof.rs` implements a dummy-SDL bootstrap runner and
an offline validator. This format is still used by that executable. The removed
S4 CI controller, ownership ledgers and linked C archive are not prerequisites.
For real-window gameplay checks, use [the native scenario runner](scenarios.md).

The bootstrap tool supports:

```text
uqm-gameplay-proof run REPO_ROOT PRODUCTION_MANIFEST SCRIPT OUTPUT_ROOT
uqm-gameplay-proof validate OUTPUT_ROOT/lcar-v2.json
uqm-gameplay-proof validate OUTPUT_ROOT/failure-lcar-v2.json
uqm-gameplay-proof replay REPO_ROOT PRIOR_BUNDLE CONTENT_DIR OUTPUT_ROOT
uqm-gameplay-proof validate-negative-fixtures
```

## Inputs and result

The production manifest uses schema `uqm-deterministic-artifacts-v4`. It binds
`git_head`, target, release profile, the `audio_heart` feature and executable
path/SHA-256. Run and replay require a clean source worktree at that commit and a
matching native target. Read the executable's production-manifest parser before
constructing a manifest; the former CI generator no longer exists.

A passing result is `lcar-v2.json`; a failed result is `failure-lcar-v2.json`.
Both use schema `uqm-lcar-v2`. The filename must agree with `passed`; both result
files cannot coexist. Unknown fields are rejected.

| Field | Meaning |
| --- | --- |
| `passed`, `first_failed_contract` | Result and first failure, if any |
| `git_head`, `target`, `profile`, `features` | Bound source/build identity; release with `audio_heart` |
| `command`, `environment`, `renderer` | Child invocation, dummy SDL variables and `sdl2-software-dummy` renderer |
| `seed`, `input_identity` | Resolved nonzero automation seed and replay-input digest |
| `provenance` | Production manifest, executable, script, content and configuration identities |
| `process`, `cleanup` | Child supervision and teardown receipts |
| `artifacts` | Retained file inventory with byte counts and digests |

The child environment is cleared, then `SDL_AUDIODRIVER=dummy` and
`SDL_VIDEODRIVER=dummy` are set. The bootstrap captures the internal draw surface.
It cannot establish what a player saw or expose a window presentation defect.

## Replay identity

`run/resolved-scenario.json` contains the validated scenario and its
`replay_identity`. The scenario includes complete ordered actions, assertions,
seed, starting scene and effective budgets. The digest binds the compact serde
serialization, including every action, rather than only the step count.

The top-level input identity also binds source, executable, script, build
features, renderer, environment, content tree and initial configuration. Final
configuration, process outcome and captures are separate result evidence.
Replay validates the prior bundle, copies verified inputs and compares normalized
semantic traces. It does not compare capture pixels or prove full gameplay parity.

## Retention and validation

The tool retains production/executable/script snapshots, resolved scenario and
initial/final configuration inventories. Validation checks file types, paths,
byte counts, digests, trace sequence and terminal/process contracts. Missing,
extra, substituted or changed retained files fail validation. The inventory does
not include the result manifest itself.

Replay accepts passing v2 bootstrap bundles, not failure bundles or native-suite
layouts. Offline validation does not require the original checkout/output paths
to exist, but cannot re-hash external content bytes that were not retained.
A valid failure bundle records a failed run; it is not gameplay acceptance.
See the tool's embedded tests for rejection fixtures and the exact schema.
