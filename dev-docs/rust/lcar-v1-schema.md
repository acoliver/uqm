# LCAR v1: the bootstrap evidence contract

LCAR is the evidence contract for behavioural claims about this port. It exists
because "I ran it and it worked" is not checkable, and because a proof that
cannot be replayed offline by someone else is an assertion, not evidence.

This document describes the schema as the validator enforces it. The validator
is the authority; where this document and `uqm-gameplay-proof` disagree, the
code is right and this document is a bug.

- Emit a proof: `uqm-gameplay-proof run <repo> <production-manifest> <script> <output>`
- Validate one offline: `uqm-gameplay-proof validate <output>/lcar-v1.json`
- Check the validator still rejects bad evidence: `uqm-gameplay-proof validate-negative-fixtures`

## What the letters mean

- **L — Live.** The real built executable performed the behaviour under
  autonomous control. Compilation and unit tests do not satisfy this.
- **C — Complete.** The accepted implementation is active and the superseded
  provider is gone. No fallback, stub, dormant path or duplicate authority.
- **A — Automated.** Focused tests plus the full repository gate suite prevent
  the behaviour, ownership and integration from regressing.
- **R — Reproducible.** A reviewer can identify, validate offline and replay the
  exact source, executable, content, configuration, scenario, seed, commands,
  assertions, captures, logs and teardown outcome.

## Manifest

`lcar-v1.json`, schema identifier `uqm-lcar-v1`. Unknown fields are rejected, so
the manifest cannot carry undeclared content.

| Field | Meaning |
|---|---|
| `schema` | `uqm-lcar-v1` |
| `passed` | Whether every contract held |
| `first_failed_contract` | The first contract that failed, or null |
| `git_head` | Full 40-hex commit the proof ran at |
| `command` | Exact argument vector that produced the run |
| `environment` | Environment that affects the result, such as the SDL drivers |
| `target` | Platform triple |
| `profile` | Build profile |
| `features` | Cargo features enabled |
| `renderer` | Renderer in use |
| `seed` | Deterministic seed |
| `provenance` | Content addresses, below |
| `process` | Process receipt, below |
| `cleanup` | Teardown receipt, below |
| `artifacts` | Content-addressed inventory, below |

A run must be made from a clean worktree at a full 40-hex commit. If
exceptional uncommitted input is approved, every tracked difference and
untracked input must be content-addressed in a canonical patch or tree
manifest. The validator rejects unhashed dirty content.

### provenance

`production_manifest_sha256`, `executable_sha256`, `script_sha256`,
`content_tree_sha256`, `initial_config_tree_sha256`, `final_config_tree_sha256`.

The two configuration digests bracket the run, so a proof that mutated the
player's configuration and left it behind is visible rather than silent.

### process

`pid`, `start_time`, `exit_code`, `signal`, `term_sent`, `kill_sent`,
`stdout_bytes`, `stderr_bytes`, `output_drained`, `orphan_check_passed`.

### cleanup

`exact_child_reaped`, `orphan_check_passed`, `output_drained`,
`config_root_removed`.

A proof that leaves a process, a listener or a temporary profile behind has not
finished, however green its assertions look.

### artifacts

Every artifact carries `role`, `path`, `bytes` and a SHA-256. Roles are
`stdout_log`, `stderr_log`, `trace`, `teardown_receipt`, `capture`,
`production_manifest_snapshot`, `executable_snapshot`, `script_snapshot`,
`content_identity_snapshot`, `initial_config_snapshot`, `final_config_snapshot`
and `retained_config_file`.

Paths are relative and must not traverse or repeat. The inventory must match
what is actually on disk: an artifact whose bytes changed after the run fails
validation.

## Failure contracts

When a run fails, `first_failed_contract` names the first contract that broke:
`timeout`, `reader`, `budget`, `nonzero_child`, `missing_teardown`,
`semantic_evidence`, `teardown_evidence`, `config_cleanup`.

Naming the first failure matters: a run that fails several contracts at once is
usually explained by the earliest one.

## Assertions and captures

Semantic assertions in the trace are correlated to the presented frame
generation, so an assertion cannot be satisfied by state the player never saw.

Captures are PNGs written from the game's draw surface. That is sufficient for
"the game reached this state" and **not** sufficient for "the player saw this",
because an internal capture cannot show a present or swap defect. A claim about
what is on screen needs a capture of the real window.

A capture step may be marked `expect_change`, which requires it to differ from
the capture before it. A frozen screen still completes captures and still
records presentations, so without that marker a stalled run reads as a pass.

## Rejection

`validate-negative-fixtures` proves the validator still refuses bad evidence. It
covers an empty artifact inventory, a traversal path, a duplicate path,
malformed provenance, a mutated artifact, an unknown manifest field, a forged
failure contract and a mutated trace sequence.

A validator nobody has tried to fool is not known to work.
