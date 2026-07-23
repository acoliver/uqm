# Architecture Overview: Rust Runtime Automation and Real-Game Proof

Plan ID: `PLAN-20260723-RUNTIME-AUTOMATION`
Status: executable implementation plan; production implementation has not started
Source baseline: working tree independently inspected 2026-07-23
Final normative detail: [`analysis/authoritative-execution-contract.md`](analysis/authoritative-execution-contract.md). Its exact callback tables/timelines, ABI shells, ordered I/O, transport, SDL ABI, linked-probe, `ChildSession`, and evidence-separation rules supersede any abbreviated wording below.

## 1. Goal and architecture direction

The target remains a Rust-owned UQM runtime with zero in-tree C implementation code. Rust ultimately owns event polling, input state/repeat, game scheduling, draw processing, presentation, automation, shutdown, and process supervision. The implementation slice in this plan drives today's hybrid binary; its C hooks are explicit migration seams and must not be described as the final architecture.

The proof is not a mock: it launches `rust/target/debug/uqm`, follows `main -> mainloop::init_sequence::run_uqm -> uqm_c_do_init -> rust_game_loop -> teardown_subsystems`, loads real `sc2/content`, drives real nested input, observes a typed real menu transition, captures the real logical surface at a presentation call boundary, and validates teardown.

## 2. Source-grounded baseline

The working tree has user edits in, among other files, `rust/src/graphics/ffi.rs`, `rust/src/mainloop/options.rs`, `sc2/src/uqm/gameinp.c`, and `sc2/src/uqm/rust_comm.c`. No phase may reset, restore, checkout, overwrite, or globally format these changes. P00 snapshots the complete status/diff and remediates quality failures forward.

The mandatory quality baseline currently fails:

- `cargo fmt --all --check` reports repository-wide formatting differences.
- The planning rerun of strict Clippy exits 101: the captured all-target stream contains 2,198 `error` headers and Cargo reports the lib-test target failed with 2,035 previous errors. P00 must preserve a fresh categorized inventory; these values describe scale, not a waiver or fixed expected count.
- `cargo test --workspace --all-features` exits 101 at the `input_integration_tests` link: the linker receives `-luqm_rust` but cannot find that archive (`ld: library 'uqm_rust' not found`). P00 owns this existing blocker before feature work or linked probes.

These are blockers, not waivers. P00 remediates baseline failures and runs executable environment/link-feasibility probes; P00a independently reruns and verifies zero-exit gates plus a minimal production-archive harness. Baseline work preserves semantics/user modifications and adds regression tests for non-format semantic edits.

Current source facts override stale statements in `dev-docs/rust/howtorun.md` and `howtoconfigure.md`: Cargo auto-discovers `rust/src/main.rs`, `rust/build.rs` links C objects specifically into binary `uqm`, and that Rust binary is the active process entry for this build. `sc2/build.vars` and `sc2/config_unix.h` currently enable Rust threads, graphics, communication, and restart menu.

## 3. Current input and menu path

`sc2/src/uqm/gameinp.c::DoInput` currently executes:

1. `Async_process`;
2. `TFB_ProcessEvents` and `ProcessUtilityKeys` under `RUST_OWNS_MAIN`;
3. `TaskSwitch`, which pumps SDL again in the reviewed Rust-thread configuration;
4. one `UpdateInputState`;
5. journal/menu sounds;
6. `inputCallback`;
7. screen `InputFunc` in the loop condition.

Automation injection therefore belongs after `TaskSwitch` and before the sole `UpdateInputState`. Post-input observation belongs immediately after that update. The observer returns a stop result. C computes `stop = service_stop || observation_stop` and checks it immediately after observation, before journal work, menu sounds, `inputCallback`, or `InputFunc`. Thus a trace/getter/panic/lock failure discovered during observation cannot execute any of those once.

`ImmediateInputState`, `CurrentInputState`, and `PulsedInputState` are still C-owned. The transition uses a bounds-checked `c_SetImmediateMenuKey`; automation never calls the existing `c_UpdateInputState` bridge and never writes current/pulsed arrays. Exact existing reads are `c_GetCurrentMenuKey`/`c_GetPulsedMenuKey`. The source inventory nevertheless includes the current non-DoInput consumer `rust/src/comm/talk_segue.rs::do_talk_segue`, which directly calls that bridge and therefore needs the same safe-point/post-update hooks as ConfirmExit/BackgroundInit/MeleeGameOver/AnyButtonPress.

The real main menu is Rust-owned in the active build (`USE_RUST_RESTART=1`). `handle_navigate` currently commits `state.cur_state` without a local C-field sync. P06 changes the exact order to draw -> assign -> `ops.sync_cur_state` -> typed observer. A typed `CallbackControl::{Continue,Stop}` propagates synchronously through `handle_navigate`, `do_restart_frame`, and full panic-contained `rust_do_restart_frame`; Stop precedes sleep/later frame work. The full `rust_start_game` shell is also panic-contained. P08 machine-asserts `NewGame -> LoadGame`; PNG inspection remains supplemental.

## 4. Sticky terminal and unwind contract

Automation terminal state is first-wins and sticky across all integration layers:

- `Running -> Terminal(outcome)` is irreversible until finalization.
- Every automation callback in terminal state releases all owned keys, ORs exact `CHECK_ABORT` (`0x4000`) into current activity while preserving other bits, and requests stop.
- Post-update observation can itself transition to terminal and returns stop.
- An outer Rust terminal guard in the main lifecycle reasserts `CHECK_ABORT` at every available outer game-loop boundary and before/after nested calls until game cleanup begins. This guard is independent of another `DoInput` callback occurring.
- Finalization consumes the runtime once, attempts exactly one `run_end`, closes output, records final status, and never reactivates it.

Source-grounded clearing/loop matrix that P06 must encode and integration-test:

| Site | Current behavior | Required terminal behavior/test |
|---|---|---|
| active `rust/src/mainloop/restart_menu/orchestration.rs::restart_menu_impl` lines 119-134 | clears startup abort before `ops.run_do_input(true)` | outer guard bypasses/reasserts the clear and prevents normal input entry when terminal; real nested unwind test |
| inactive legacy counterpart `restart.c::RestartMenu` line 318 | same clear before `DoInput`; not selected because `USE_RUST_RESTART=1` | source-regression parity test only; do not misstate it as active production ownership |
| `setupmenu.c::SetupMenu` line 1229, after `DoInput` | clears abort then propagates | guard immediately reasserts and prevents continued setup propagation; linked integration path |
| `battle.c::Battle` line 489 | clears abort for Super Melee return | automation terminal is distinguished from ordinary local battle abort and reasserted before outer continuation; linked integration path |
| `supermelee/pickmele.c` line 385 | clears abort on ship-pick abort | guard reasserts before parent loop; linked integration path |
| `fmv.c::ShowPresentation` line 109 | clears only after earlier abort check returned | source test proves terminal path returns before clear; guard test covers future regression |
| `restart.c::StartGame` loop / Rust `start_game_impl` | loops around restart state | outer Rust guard prevents sticky terminal from being treated as a normal retry |
| `confirm.c::DoConfirmExit` | non-`DoInput` loop calls `UpdateInputState`, clears `ExitRequested`, sleeps | call a panic-contained automation safe point before update and a stop check immediately after post-update observation; terminal skips response sounds and exits loop; linked real implementation test |
| `starcon.c::BackgroundInitKernel` lines 99-104 | splash initialization loop calls `UpdateInputState` then `TaskSwitch` | safe point/observer around update or outer lifecycle guard must stop it; linked/source-order integration test |
| `supermelee/pickmele.c::MeleeGameOver` lines 677-692 | result loop updates input, async-processes, and task-switches | safe point/observer prevents terminal from waiting for button/timeout; linked integration test |
| `gameinp.c::AnyButtonPress` line 489 | one-shot update used by wait/presentation paths | safe point before update and observer after, returning conservative button/abort result on terminal; linked integration test |
| `rust/src/comm/talk_segue.rs::do_talk_segue` current lines 252-259 | current user edit directly calls `c_UpdateInputState` outside C `DoInput` | preserve edit; add same before/update/after control flow through a shared production guard helper; terminal skips communication input/action; focused plus linked-helper test |
| other non-`DoInput` loops found by repository search | may update/pump without service | P06 regenerates inventory and either instruments an automation safe point or proves the outer guard reaches it; unexplained loop is verifier failure |

No phase may solve this with global process exit, `ExitRequested`, `KEY_EXIT`, SDL quit, or parent-wide process killing.

## 5. Runtime synchronization and C-facing safety

Global automation access uses a synchronous state owner. Execution-contract §3 is authoritative: each direct shell first counts ABI entry and performs the allocation-free atomic inactive gate; active paths then apply depth and a top-level `catch_unwind` around the complete shell, pure reservation transition, unlocked effects, ordered publish/cancel and matching commit. Full `rust_do_restart_frame`, `rust_start_game`, and `rust_gfx_postprocess` shells are panic-contained. No panic crosses C.

Normative synchronization rules:

1. Never hold the automation mutex while calling C, SDL presentation, graphics-state acquisition, trace callbacks that re-enter, or teardown.
2. Phase order is graphics state -> copy owned snapshot -> release graphics state -> pure runtime reserve/commit lock -> release runtime lock -> ordered trace/file synchronization. Runtime and ordered-I/O locks never overlap; neither is held across C/SDL/graphics/logging/waits/observer calls.
3. A thread-local callback-depth/reentrancy guard rejects nested automation callback entry. Reentry latches `ReentrantCallback`, requests terminal through a lock-free fallback, and returns a conservative stop; it never deadlocks.
4. `Mutex::lock` poison is recovered with `PoisonError::into_inner` only to latch `MutexPoisoned`, release keys where safely possible, and finalize terminal state. Normal scheduling never resumes from poisoned state.
5. If the mutex cannot be safely used (panic during recovery/reentry), an atomic fallback terminal latch and atomic final-exit class are set; the callback ORs `CHECK_ABORT` through the safe activity API and returns stop. The next non-reentrant callback imports that fallback into the first-wins outcome.
6. Finalization atomically takes the global runtime before flushing/dropping it. Concurrent/late callbacks see `Finalizing/Finalized`, reassert abort if needed, and do not access dropped writers.
7. Forced-panic tests cover before-lock, while-locked (poison), after-lock, observer, menu observer, present observer, writer, and finalization paths.

## 6. Scheduler and watchdog semantics

Actions are strongly typed and sequential. Version 1 includes `wait_input_ticks`, `set_menu_key`, `tap_menu_key`, `capture`, `assert_activity`, `assert_main_menu_transition`, and final `finish`. A screenshot is never a semantic assertion.

Scheduler transition table:

| State/action | Callback | Predicate | Effects | Next |
|---|---|---|---|---|
| Running/ready | input | budget already reached | latch timeout; release; abort | Terminal |
| `wait(0)` | input | true | no ownership write | next action, same callback |
| `wait(n>0)` | input | current tick consumed | decrement once | wait(n-1); advance only when zero |
| `set(k,v)` | input | valid | update ownership then apply | next, same callback |
| `tap Hold(n)` | input | n>1 | apply held; decrement | Hold(n-1) |
| `tap Hold(1)` | input | true | apply held for this update | ReleasePending |
| `ReleasePending` | next input | true | apply released before update | Settle(m) or next |
| `Settle(n>0)` | input | true | keep released; decrement | Settle(n-1)/next |
| capture | input | not armed | arm once | WaitingCapture |
| WaitingCapture | input | no committed present | no advance | WaitingCapture |
| WaitingCapture | present complete | copy/write/record succeeds | clear pending | next action |
| semantic assertion | observer event | exact typed event matches | record pass | next |
| semantic assertion | observer event/budget | mismatch/absent by budget | record failure | Terminal |
| finish | either service | all prior requirements complete | latch success | Terminal |
| Terminal | any callback | always | release, OR abort, stop | Terminal |

Watchdog uses execution-contract §2 exactly: the applicable counter is checked-added and stored before comparison; equality is terminal and admits no action work. Maximum M admits at most M-1 callbacks (max=3: ordinals 1/2 admit, ordinal 3 times out). Every callback samples wall time once; applicable overflow is typed, then simultaneous priority is input, presentation, wall, clock regression. Terminal callbacks do not increment. Exact timelines/property tests are mandatory.

## 7. Presentation and durable capture

`TFB_SwapBuffers` may return before backend calls. `TFB_FlushGraphicsEx(TRUE)` skips swap. Neither may count. In the Rust backend, `Canvas::present()` returns `()`; “present success” therefore means the present call returned normally, not that SDL acknowledged display or vsync. The observer runs exactly once after that return. A contained panic/failure before completion does not count.

Capture reads logical surface 0 (320x240), not the scaled window or renderer overlays. If `SDL_MUSTLOCK(surface)` is true, call `SDL_LockSurface`, handle nonzero failure, copy while locked, and always unlock via a guard. Validate pointer, format, dimensions, pitch, bytes-per-pixel, masks, and checked lengths. Release graphics state before automation/file work.

Durability contract is supportable and explicit: create the final PNG path with `create_new`; encode; `BufWriter::flush`; recover the inner `File` with error propagation; `File::sync_all`; drop/close the file; only then emit the capture record. Directory `sync_all` is attempted on platforms that support opening/syncing the directory; `Unsupported` is recorded as `directory_sync: unsupported`, while other failures are fatal. The plan does not claim power-loss durability beyond successful supported file/directory sync calls.

## 8. Inactive-mode real-child transport

Inactive proof launches the normal real binary without automation options. The parent sends nonce-authenticated commands over a unique Unix datagram socket under the run root. The child transport adapter resolves the isolated configuration's real binding for menu-down and calls `SDL_PushEvent` with genuine `SDL_KEYDOWN` and `SDL_KEYUP` events (correct window ID, scancode, keycode, modifiers, and nonrepeat field). The existing `TFB_ProcessEvents` -> Rust VControl keyboard mapping -> `ImmediateInputState` -> ordinary `UpdateInputState` path consumes them. It must not call the automation setter or scheduler. After evidence acknowledgement, a `quit_smoke` command pushes a genuine `SDL_QUIT`; proof-smoke lifecycle exits only after the normal event pump records that quit, then runs teardown.

Child emits observable transport counters to a separate `inactive-counters.jsonl`: datagrams received/rejected, SDL key down/up and quit events pushed/push-failed, matching SDL events polled, VControl dispatches, normal input-state updates with menu-down observed, automation service calls, and automation setter writes. Acceptance requires one accepted key tap, both key events pushed/polled/dispatched, menu-down observed by a normal update, quit pushed/polled, automation service calls=0, setter writes=0, and no automation output directory. The socket protocol itself never mutates input; only the events polled through the normal SDL path do so.

## 9. Linked C boundary tests

Copied C logic is not proof. P06 adds explicit Cargo harness binaries, built by `build.rs` with target-specific link arguments:

- `automation-input-boundary`: links the same generated `libuqm_c.a` used by `uqm`, including the recompiled production `gameinp.c`; test-only C control functions live in a separate shim and invoke the real exported setter and real `DoInput` through production symbols. Rust supplies forced outcomes/counters. It verifies update/observe/stop ordering, zero journal/sounds/callback/InputFunc after either service or observation stop, non-DoInput safe points, and clearing-site unwind scenarios.
- `automation-present-boundary`: links the same archive, initializes real thread/graphics/DCQ prerequisites, and invokes production `TFB_FlushGraphicsEx(TRUE)` and `TFB_SwapBuffers` in a fresh process. A real queued line proves skip-swap drains without observing; neutral invalid-BBox no-redraw observes zero; forced redraw observes once.

`Cargo.toml` declares both `[[bin]]` targets with `test = false`; their mains live under `rust/tests/c_harness/`, import `uqm_rust`, and call a Rust link anchor before the shim entry. Build scripts are package-scoped, so `build.rs` must not depend on a fictional per-target `CARGO_BIN_NAME`. It always compiles tiny shim archives with `cc::Build::cargo_metadata(false)` and emits target link arguments in dependency order (shim before `-luqm_c`), while preserving `rustc-link-arg-bin=uqm=-luqm_c`, the `OUT_DIR` search path, and external-library ordering. Shims stay out of ordinary `uqm`. A checked-in `rust/tests/c_harness/run-linked.sh` builds/runs both. The harness may expose test controls but may not duplicate tested production functions. `nm -A` must prove symbol origins; verifier deliberately breaks each real call path and observes test failure.

## 10. Proof-runner safety and identity

The synchronous proof runner immediately owns a `ChildSession`: Child/identity, parent pipe ends, bounded reader threads, socket and manifest. `try_wait Some` is the one stored reap and is not followed by wait; otherwise failure uses cooperative stop, child-only kill if live, and wait retried on Interrupted until the one reap. Only afterward are parent pipes closed, readers drained/joined, socket cleaned, and orphans checked. Kill/reader/join errors never skip reap. Explicit `finish` must reach Complete; Drop is only a nonpanicking backstop. Deadlines use `Instant`; each run uses an exclusive unique root and never removes a shared path.

Preflight refuses to run if a prior proof manifest identifies a live child with matching PID plus start identity, or if an existing `uqm` whose executable digest equals the requested binary is present. It reports the orphan and stops; it never invokes `pkill`, `killall`, or name-wide signals. After each run it verifies its child is waited, transport/readers are joined, the socket is removed, and no matching manifest-owned child remains.

`run-metadata.json` and `proof-report.json` include SHA-256 identities for the executable bytes, script bytes, content manifest (sorted relative path + size + digest), relevant generated build configuration, and isolated config tree. Use an existing hash crate only if verified; otherwise add one justified direct dependency. Paths alone are not identity.

`run_end` occurs before subsystem teardown. Finalization closes every automation writer/handle before teardown. Only after `teardown_subsystems` has actually returned, lifecycle code opens a previously nonexistent `teardown-complete.json` with `create_new`, writes the run identity and `teardown_complete`, flushes, syncs, drops it, and attempts supported parent-directory sync. No marker handle is kept open across teardown, so an earlier crash cannot leave a false marker. The proof report is written only after the parent validates that receipt, child status, drained output, and orphan checks.

## 11. Acceptance boundary

Machine acceptance requires the real binary to:

- reach the typed `RestartMenuItem` observer;
- observe and assert `NewGame -> LoadGame` from the actual commit point;
- record intended/current/pulsed input;
- produce valid 320x240 logical-main PNGs;
- emit identity digests, `run_end`, and post-teardown `teardown_complete`;
- cleanly unwind cooperative failures despite abort-clearing callers;
- prove inactive normal-SDL transport with counters;
- leave no owned child/orphan; and
- pass fmt, check, strict Clippy, all Rust tests, both linked C harnesses, and proof runs.

Human screenshot inspection may detect visual regressions but cannot be required for correctness. A verifier must fail for a missing semantic observer, transport/counters, teardown marker, identity digest, unsupported/copied C test, global process killing, or manual-only claim.
