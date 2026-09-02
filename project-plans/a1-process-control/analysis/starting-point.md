# A1 / Issue #29 verified starting point

Audit base: branch `issue29-deterministic-process-control` at main commit `2d8ca5ea7`.
Scope roots claimed by issue #29: `rust/src/automation/`, `rust/harness/`, `.github/workflows/`.

This document records what exists today. It contains no implementation decisions.

## 1. Capability matrix against the seven required-scope bullets

| Required capability | State | Evidence |
| --- | --- | --- |
| Versioned launch/readiness/progress/command-ack/checkpoint/failure/terminal protocol | PARTIAL | `trace.rs:133` `TraceRecord::SCHEMA: u16 = 1`; `trace.rs:29-39` `RecordKind`; `transport.rs:106` `PROTOCOL_VERSION: u8 = 1` |
| Run ownership by PID, lock, executable identity, provenance | PARTIAL | `child_session.rs:399` `ProcessIdentity`; `child_session.rs:960` `capture_identity`; `identity.rs:218` `IdentityMetadata` |
| Startup/progress/idle/teardown watchdogs | PARTIAL | `watchdog.rs:145` `watchdog_reduce`; `watchdog.rs:36` `WatchdogLimits`; `child_session.rs:1591-1748` deadline handling |
| Signal and interruption handling | PARTIAL | outbound only: `child_session.rs:1697`, `:1702`, `:1887`, `:1914` |
| Ordered, idempotent finalization | PRESENT | `runtime.rs:284` `finalize`; `runtime.rs:363` `FinalizationResult`; `lifecycle.rs:125` `run_lifecycle`; `trace.rs:170-300` `OrderedCommit` |
| Process and resource inventories | PARTIAL | `child_session.rs:1349` `ChildSessionReceipt`; `proof.rs:209` `validate_proof_run`; `identity.rs:83-216` `TreeManifest` |
| Fixture coverage for the six failure classes | PARTIAL | see section 5 |

## 2. Protocol gaps

Emitted record kinds today (`trace.rs:29-39`): `RunStart`, `RunEnd`, `InputTick`, `Presentation`,
`Capture`, `MenuTransition`, `SemanticAssertion`, `SeedApplication`, `Terminal`.

Mapping to the seven required kinds:

- launch: `RunStart` exists but carries no launch or identity payload.
- progress: covered by `InputTick` and `Presentation`.
- terminal: covered by `Terminal` with `terminal_reason`.
- readiness: NOT represented. `SchedulerEvent::MainMenuReady` exists in `scheduler.rs` but emits no record.
- command acknowledgement: NOT represented. `AckKind` (`transport.rs:62`) is in-memory only and never persisted.
- checkpoint: NOT represented.
- failure: NOT represented as a record. Failures surface only as `TerminalClass` or `ChildSessionError`.

`SessionState` (`child_session.rs:354`) is an internal supervisor state machine, not an emitted protocol.

## 3. Run ownership gaps

Every signal site lives in `rust/src/automation/child_session.rs` and only ever signals the
supervisor's own spawned process group:

- `child_session.rs:1226` `signal_process_group`, guarded by the `LeaderAnchor` invariant
  (`:977-1037`, `waitid(P_PID, WNOWAIT)`) so the PID and PGID cannot be reused before cleanup.
- `child_session.rs:1697` / `:1702` SIGTERM then SIGKILL in `teardown_and_reap`, routed through
  `signal_group_if_present` (`:1802`) which first inspects group membership.
- `child_session.rs:1887` / `:1914` partial-spawn cleanup; inspection failure fails closed as
  `ChildSessionError::ProcessGroup`.

Gaps:

- Run lock / single-owner guarantee: ABSENT. No `flock` or lock file anywhere in the module.
  `PreflightCheck.fresh_root_created` (`proof.rs:63`) is a model boolean, not mutual exclusion.
- Stale owned-process adoption and reaping: ABSENT. `PreflightCheck.no_matching_processes` has no
  production scanner behind it.
- `ChildSessionConfig.executable_digest` is caller-supplied and trusted, never verified against the
  spawned binary.
- Provenance beyond digests (git commit, environment, invocation identity) is not captured.

## 4. Watchdog and signal gaps

- Distinct startup/readiness watchdog: ABSENT.
- Idle-inactivity watchdog separate from the total wall budget: ABSENT.
- Inbound signal handling for the supervisor itself (SIGINT, ctrl-c, panic hook): ABSENT.
- Whole-system process inventory and a production preflight matching-process scan: ABSENT.

## 5. Fixture coverage for the six required failure classes

| Class | Coverage |
| --- | --- |
| Hang | Real-process coverage: `timeout_sends_sigterm_then_sigkill` (`:2669`), `timeout_sends_sigterm_child_exits` (`:2692`), `hard_hang_classification`, `drop_backstop_kills_orphan` |
| Signal (outbound) | Real-process coverage: `direct_exit_kills_term_ignoring_descendant_without_blocking_readers` and the timeout pair above |
| Teardown leak | Real-process coverage: `direct_exit_terminates_descendant_holding_both_output_pipes`, the `partial_cleanup_*` family, `lifecycle.rs` ordering tests |
| Crash | WEAK. Only non-zero exit (`normal_completion_exit_nonzero`). No signal-death child fixture |
| Stale identity | MODEL ONLY. `identity_no_match_different_pid/start/digest`. No live PID-reuse fixture |
| Lost acknowledgement | MODEL ONLY. `transport.rs` rejection tests and `validate_proof_run_fails_pending_ack`. No end-to-end fixture with a real child transport |

`rust/tests/automation_conversation_decisions.rs` and `automation_closing_speech_decision.rs` cover
scheduler conversation decisions and none of the six classes.

## 6. Native ownership: unresolved authority conflict

Issue #29 claims a native scope rooted at `rust/harness/` and requires deleting superseded native
providers in the same PR. The in-repo authority does not support that claim.

`rust/ci/native-ownership-ledger-v7.json`:

- `.schema` = `uqm-native-ownership-ledger-v7`, `.assessment_commit` = `54e1dba5f56e9f20a3aa773d5f151470a8cf0662`.
- All seven `rust/harness/*.c` and `*.h` files carry `{"issue": "RUST_HELPERS"}` (lines 10-35).
- `.scope_boundaries` S3 block, line 8210, quoted: "RUST_HELPERS issue 156 retains ownership and
  eventual removal of native helper sources under rust/harness and rust/src. S3 may consume but must
  not delete, alter implementation, or claim those helper sources unless this ledger and native
  relationships are amended again."
- `.control_plane_entries` assigns `rust/harness/**`, `rust/probes/**`, and `rust/src/automation/**`
  to S3, and states native helper deletion "remain[s] RUST_HELPERS issue 156" (line 7939).
- The string `A1` appears nowhere in the ledger. The only A-keys present are `A3`
  (`rust/src/automation/input_ffi.rs`, lines 3858-3859) and `A4`
  (`rust/src/automation/ui_observation.rs`, lines 3861-3863).

Conclusion: the ledger assigns the harness native files to `RUST_HELPERS` / issue 156, not to #29,
and defines no mapping from A-numbers to that key. Whether A1 covers them is undetermined from the
in-repo files. Issue #29 requires amending the ledger and the issue before implementation when such
a mismatch is found.

## 7. Blast radius

Direct, from gate `probes-harnesses` in `rust/ci/gates.json`:

- step `p00-probes`: `bash rust/probes/run_p00_probes.sh`
- step `p00-harness`: `bash rust/harness/run_p00_harness.sh`
- step `menu-binding-probe`: `bash rust/harness/run_menu_binding_probe.sh`

Indirect: `rust/build.rs:143` calls `compile_p00_harness` unconditionally, outside the
`linked_c_archive` branch, so every cargo invocation compiles all four harness `.c` files. That
reaches gates `check` (`check-linked-bin`, `check-linked-fixture`), `clippy` (`clippy-linked-bin`,
`clippy-linked-fixture`), `tests` (`xtask-test`, `native-acceptance`), `package`
(`verify-production-ownership`), and `ownership-link` (`verify-fixture`).

Production reach: only `sdl_surface_accessors.c` enters a production artifact. `build.rs:768-775`
compiles it with the `cc` crate as `p00_sdl_accessors`, which auto-links into every target including
the release `--bin uqm`; its consumer is `rust/src/graphics/sdl_capture.rs:13-38`, registered at
`rust/src/graphics/mod.rs:21`. The other six produce only `OUT_DIR` objects consumed by the two
harness shell scripts.

Provider manifest: `rust/ownership/native-provider-manifest.json` declares nothing for
`uqm_query_menu_binding`, the `p00_harness_*` functions, or the twenty `uqm_sdl_*` accessors.
`verify-fixture.sh` covers only the queue and hash-table contracts. These symbols therefore sit
outside the manifest's strict-link authority even though `p00_sdl_accessors` reaches the production
binary.

Workflow: `.github/workflows/rust-quality.yaml` never names the scripts. Its single transit point is
the `gates` job step `authoritative_gates` (lines 954-994), which runs `xtask ci run all` against
`gates.json` installed at line 617.

## 8. Highest-risk unknown

The ownership authority conflict in section 6. Issue #29 cannot satisfy its own acceptance criterion
"All assigned superseded native files/providers/build entries are deleted in this PR and the
zero-native trend gate records the exact delta" while the governing ledger reserves those files to
`RUST_HELPERS` / issue 156 and forbids other keys from deleting or claiming them. Either the ledger
and issue are amended to move the harness helper sources into A1, or #29's native delta is declared
zero and the helper deletion stays with issue 156. This must be decided before any implementation
phase is written, because it determines whether the plan contains a native-cutover slice at all.
