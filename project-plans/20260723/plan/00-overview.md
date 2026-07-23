# Plan: Rust Runtime Automation and Real-Game Proof

Plan ID: `PLAN-20260723-RUNTIME-AUTOMATION`
Execution order: `P00 -> P00a -> P01 -> P01a -> ... -> P08 -> P08a`

## Coordination

Create TODOs for all 18 entries before execution. Run one worker then its separate verifier. A verifier emits only `Phase NN: PASS` or `Phase NN: FAIL`; only PASS creates `.completed/PNN.md`. Failure returns to the same worker and blocks all later phases. Preserve all existing edits; never reset/restore/checkout. No time estimates, phase batching, async runtime, global process kill, manual-only proof, or fake architecture completion.

## Ordered ownership

| Sequence | Worker | Primary ownership |
|---|---|---|
| P00/P00a | strict baseline/probes / verify | QUALITY; fix `-luqm_rust` Cargo-test blocker and actual Clippy backlog; execute environment and minimal linked-harness feasibility probes |
| P01/P01a | typed script contracts / verify | SCRIPT, DEP, validated inclusive-budget relationship |
| P02/P02a | pure scheduler/watchdog / verify | exact post-increment tables/timelines, generation/two-phase model, DET |
| P03/P03a | ordered trace/artifact/identity primitives / verify | reservation publish/cancel cursor, durable exclusive files, identity; no callback integration claim |
| P04/P04a | pure ABI/runtime safety model / verify | inactive gate, shell/depth/catch, lock-free mirrors/fallback, finalization; no real FFI claim |
| P05/P05a | CLI/lifecycle/finalization / verify | activation, lifecycle records, shell drain/run_end, active post-teardown receipt |
| P06/P06a | input/menu/terminal C integration / verify | complete ABI shells, input/menu records, synchronous menu Stop, complete clear/update inventory, linked input harness |
| P07/P07a | present/capture C integration / verify | full gfx shell, ABI SDL/MUSTLOCK, generation/durable capture records, linked present/lock harness |
| P08/P08a | transport and real proof / verify | SEM-002, authenticated normal-SDL transport/acks, inactive receipt, ChildSession, digests, autonomous proofs |

Pure P02 does not claim real atomics/setter/FFI/capture. P03 does not claim callback integration/graphics completion. P04 does not claim actual C unwind/lifecycle wiring. P05/P06/P07 integrate lifecycle, input/menu, and present/capture records respectively. P08 owns `REQ-SEM-002`, inactive proof/receipt and final cross-callback validation. The authoritative execution contract is binding in every phase.

## Shared TDD and quality gates

Each behavioral slice is RED -> GREEN -> REFACTOR. Record the intended RED, passing GREEN, and refactor rerun. No placeholder/fake return is handed off. Every verifier independently runs, from `rust/`:

```bash
cargo check --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

No known-baseline waiver exists after P00a. Verifiers compare the working tree with P00 preservation evidence and reject missing user edits. New unsafe is isolated/documented/tested. Every C-facing automation path has forced-panic coverage.

## Linked and real acceptance gates

P06 runs declared `automation-input-boundary`, linked to production `gameinp.c`/`DoInput`/setter. P07 runs declared `automation-present-boundary`, linked to production `TFB_FlushGraphicsEx`/`TFB_SwapBuffers`. `nm` origin evidence and mutation checks reject copied/unsupported harnesses.

P08/P08a use exclusive unique roots. They never call `pkill`, `killall`, or name-wide signal commands. Pre-existing matching processes cause refusal, not termination. Final verifier deliberately rejects:

- no typed observer at actual Rust `CurState` commit;
- no real-binary `NewGame -> LoadGame` machine assertion;
- post-observer failure that can run sound/callback/InputFunc;
- incomplete abort-clearing/non-DoInput matrix or absent outer guard;
- missing panic/poison/reentry/finalization tests;
- inactive transport not using normal SDL path or missing counters;
- unsupported/copied C tests;
- missing executable/script/content/build/config digest;
- missing or pre-teardown `teardown_complete`;
- child not drained/killed+waited/orphan-checked;
- screenshot/manual sign-off required for correctness; or
- any strict quality failure.

## Completion marker schema

Each `.completed/PNN.md` records phase ID, timestamp, prerequisite marker, exact files, requirements, RED/GREEN/REFACTOR evidence, every command and exit, semantic checks, preservation comparison, safety review, and explicit PASS. Final marker records `REQ-ARCH-001..004: OPEN`.
