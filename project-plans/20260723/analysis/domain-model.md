# Domain Model: Runtime Automation

Plan ID: `PLAN-20260723-RUNTIME-AUTOMATION`
Normative transitions: [authoritative-execution-contract.md](authoritative-execution-contract.md)

| Entity | Owner | Invariant |
|---|---|---|
| `ValidatedScript` | P01 | closed v1 schema; required updates fit inclusive budgets (`max >= N+1`) |
| `Scheduler`/`Watchdog` | P02 pure reducer | post-increment equality is terminal; exact table/priority; no side effects |
| `CaptureGeneration` | P02/P07 | checked nonzero atomic request; stale/duplicate completion cannot advance |
| `Reservation`/`EffectPlan` | runtime reducer | checked sequence/state version; external effects cannot commit implicitly |
| `OrderedCommit` | P03 | strict publish/cancel cursor; no runtime-lock nesting; no missing-sequence deadlock |
| `KeyMirror` | ABI boundary | lock-free owned mask/value; fallback can release without runtime mutex |
| `TerminalMirror` | ABI/lifecycle | lock-free first-wins class/status, abort, phase; conservative fallback survives poison |
| `AutomationRuntime` | synchronous global owner | Inactive/Running/WaitingCapture/Terminal/Finalizing/Finalized |
| `CallbackDepth` | active ABI shell | depth 0/1; inactive path does not touch TLS; nested active entry fails closed |
| `MainMenuTransition` | P06 | draw -> assign -> C sync -> typed observer -> synchronous Continue/Stop |
| `SurfaceSnapshot` | P07 | ABI-authoritative SDL metadata, owned RGB, exact capture generation |
| `ActiveTeardownReceipt` | P05 | only after active finalization handles close and subsystem teardown returns |
| `InactiveTransport` | P08 | authenticated datagram only requests genuine SDL events at bounded main-thread pumps |
| `InactiveTeardownReceipt` | P08 | distinct path/schema; after socket/counters/acks close and normal teardown |
| `ChildSession` | P08 | owns child/pipes/readers/socket/manifest through reap, close, join, orphan check |
| `IdentityManifest` | P03/P08 | SHA-256 bytes/tree/build/config identity, never path-only |

## State and transaction transitions

```text
Inactive --complete setup--> Running
Running --capture reserve+commit--> WaitingCapture(generation)
WaitingCapture(g) --matching durable capture+ordered record+commit--> Running
Running/WaitingCapture --first failure/finish/timeout/panic--> Terminal
Terminal --every active callback--> release mirror + OR abort + conservative stop
Terminal --atomic take, clear gate/generation--> Finalizing
Finalizing --drain shells/reservations, run_end, close handles--> Finalized
Finalized --subsystem teardown returned--> active OR inactive mode-specific receipt
```

A callback transaction is `reserve under runtime mutex -> unlock -> external effects -> ordered publish/cancel -> re-lock matching commit -> unlock`. Runtime mutex ownership never overlaps C, SDL, graphics, logging, condition waits, observer callbacks, or file I/O. Ordered I/O never nests the runtime mutex.

## ABI shells

Direct inactive callbacks permit only a saturating ABI-entry atomic and acquire activation load; neutral return follows without TLS/allocation/lock/external work. Active shells count active-gate entry, enforce depth, catch the complete shell, run pure transitions, unlock before effects, and return callback-specific conservative values after fallback. Full `rust_do_restart_frame`, `rust_start_game`, and `rust_gfx_postprocess` also contain panic around their complete non-automation behavior.

## Source-boundary inventory

P06 covers C `DoInput`, ConfirmExit, BackgroundInitKernel, MeleeGameOver, AnyButtonPress, and the current Rust `talk_segue.rs::do_talk_segue -> c_UpdateInputState` path. P07 does not trust the partial hand-written SDL format layout. P08 uses C `TFB_ProcessEvents -> ProcessInputEvent -> Rust VControl`, never the separate consuming Rust event pump.

## Phase ownership

- P00/P00a: strict/linker remediation and executable/link probes.
- P01: script types.
- P02: exact pure reducer and generation model.
- P03: ordered I/O/durable primitives.
- P04: pure shell/mirror/fallback model.
- P05: activation/finalization/active receipt.
- P06: input/menu shells, synchronous observer, callback-specific trace.
- P07: graphics/capture shell, ABI SDL, generation, callback-specific trace.
- P08: `REQ-SEM-002`, inactive transport/receipt, `ChildSession`, real proof.

`REQ-ARCH-001..004` remain OPEN; all transitional C hooks/accessors are removal seams.
