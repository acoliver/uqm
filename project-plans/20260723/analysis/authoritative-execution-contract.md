# Authoritative Execution and Safety Contract

Plan ID: `PLAN-20260723-RUNTIME-AUTOMATION`
Revision: second and final plan revision
Status: normative; if another plan document is less specific, this document controls

## 1. Source facts and preflight blockers

The 2026-07-23 source inspection established these facts:

- `gameinp.c::DoInput` pumps in `TFB_ProcessEvents`, then `TaskSwitch`, then calls its sole `UpdateInputState`; automation service belongs immediately before that update and observation immediately after it.
- `do_restart.rs::handle_navigate` currently commits `state.cur_state` but does not call `sync_cur_state` there. The observer must follow both operations.
- `do_restart_frame` and the C ABI `rust_do_restart_frame` currently return bool/int continue values; `rust_start_game` is also a C ABI shell. None currently contains the required complete panic boundary.
- `sdl_common.c::TFB_ProcessEvents` is the authoritative C poll path: each `SDL_PollEvent` result goes through `ProcessInputEvent`, which reaches Rust `rust_VControl_HandleEvent`; `SDL_QUIT` sets `QuitPosted`.
- `input.c` resolves menu bindings from `menu.<name>.<alternate>` resources during `TFB_InitInput`; no query API currently exposes the resolved menu gesture. P08 must add a narrow transitional accessor that reads the same loaded resource, parses it through production `VControl_ParseGesture`, and accepts only a `VCONTROL_KEY`. It must not guess a default key or parse config independently in the parent.
- `rust_gfx_process_events` uses a separate Rust `EventPump` and consumes events without dispatching them to `ProcessInputEvent`; inactive transport therefore must use the C `TFB_ProcessEvents` path, not that function.
- `TFB_InitGraphics` already calls `Init_DrawCommandQueue` and `TFB_UninitGraphics` calls `Uninit_DrawCommandQueue`; a linked graphics harness must not call either DCQ function a second time.
- `graphics/ffi.rs` contains a hand-written partial `SDL_Surface` and an untyped `format: *mut c_void`; capture code may not infer `SDL_PixelFormat` layout from it.
- direct non-`DoInput` updates are `confirm.c`, `starcon.c`, `pickmele.c::MeleeGameOver`, `gameinp.c::AnyButtonPress`, and the current user-added `rust/src/comm/talk_segue.rs::do_talk_segue -> c_UpdateInputState`. P06 must cover all five consumers; `rust_comm.c::c_UpdateInputState` is the bridge definition, not a sixth consumer.
- current `cargo test --workspace --all-features` exits 101 because `input_integration_tests` links `-luqm_rust` without a discoverable archive (`ld: library 'uqm_rust' not found`). P00 owns this existing linker blocker.
- current strict Clippy exits 101. The captured all-target log has 2,198 `error` headers; Cargo reports the lib-test target failed with 2,035 previous errors. P00 must inventory and remediate the actual rerun, not treat these counts as a waiver or fixed target.

P00a has two executable feasibility gates before P01:

- **P00 probe:** strict baseline, toolchain, current build flags, symbol inventory, SDL dummy/hidden capability, datagram path length/permissions, directory sync behavior, monotonic clock, executable identity/PID-start identity availability, and real key-binding accessor feasibility.
- **P00a probe:** a minimal linked C harness built through Cargo proves archive construction, archive/search/order, Rust export retention, and production-member extraction. If direct extraction of a large production member makes the harness infeasible, P00 must extract a small shared production guard/order helper and make every real call site and harness call that helper; copying logic into a test shim is forbidden.

Both probes are blockers. No later phase may paper over a failed probe.

## 2. Authoritative callback/watchdog reducer

### 2.1 Counter definitions and exact inclusive limit

`input_seen` and `present_seen` are the number of active callback entries admitted to the watchdog, including the entry that reaches a configured limit. They start at zero. An input callback computes `candidate_input = input_seen.checked_add(1)`; a committed-present callback computes `candidate_present = present_seen.checked_add(1)`. The candidate is stored as part of the pure transition before evaluating limits. The other counter remains unchanged.

A limit is inclusive and terminal: action work is admitted only when all three values are strictly less than their maxima. Therefore the callback whose candidate equals its maximum is the boundary callback and performs no scheduler action work. `elapsed` is sampled once at entry from the monotonic clock and is terminal when `elapsed >= timeout`. Zero maxima are rejected by script validation. Checked-add overflow is `CounterOverflow`, with input/presentation priority determined below.

For `max_input_ticks = 3`, input callbacks have this timeline:

| input callback ordinal | stored `input_seen` | watchdog result | scheduler action work |
|---:|---:|---|---|
| 1 | 1 | admitted if other limits are below max | yes |
| 2 | 2 | admitted if other limits are below max | yes |
| 3 | 3 | `InputTimeout` | no |
| 4+ | unchanged terminal mirror | sticky terminal | no |

The same rule applies to `max_presentations`. Thus a positive maximum `M` admits at most `M - 1` callbacks to action work. This is intentional and is the single meaning of “reaching a maximum before finish terminates.” A script that needs `N` admitted input updates must configure `max_input_ticks >= N + 1`, checked during validation.

### 2.2 Watchdog reducer table

Every active input/present callback performs exactly one pure reducer transition. `kind` is Input or Present.

| Order | Predicate using post-increment candidate/current values | Outcome | Action work |
|---:|---|---|---|
| 1 | applicable counter `checked_add(1)` fails | `InputCounterOverflow` or `PresentationCounterOverflow` | forbidden |
| 2 | `input_seen >= max_input_ticks` | `InputTimeout` | forbidden |
| 3 | `present_seen >= max_presentations` | `PresentationTimeout` | forbidden |
| 4 | `elapsed >= timeout` | `WallTimeout` | forbidden |
| 5 | test clock `now < started_at` or `< last_observed` | `ClockRegression` | forbidden |
| 6 | none | Admit | allowed |

Priority is always input, presentation, wall, clock after applying the applicable checked increment. Overflow of the applicable input/present counter is the corresponding counter failure before comparisons; it cannot wrap. A callback observes the same priority even if multiple limits become true. Terminal state is absorbing and does not increment counters.

### 2.3 Scheduler reducer table

The reducer receives one admitted callback/event and returns only typed state plus an `EffectPlan`; it performs no C, SDL, lock, or I/O operation.

| State/action | Admitted callback/event | Transition and plan |
|---|---|---|
| Ready `wait_input_ticks(0)` | Input | advance in the same reduction; no key effect |
| Ready `wait_input_ticks(n>0)` | Input | consume this admitted input callback; `n-1`; advance in the same reduction when it becomes zero |
| Ready `set_menu_key(k,v)` | Input | plan one owned-key write; advance only on commit; zero-callback actions may continue after commit in this callback |
| `tap Hold(n>1)` | Input | plan held value for this update; commit to `Hold(n-1)` |
| `tap Hold(1)` | Input | plan held value for this update; commit to `ReleasePending` |
| `ReleasePending` | next admitted Input | plan release before this update; commit to `Settle(m)` or next action when `m=0` |
| `Settle(n>0)` | admitted Input | keep released, consume exactly one admitted input callback, commit to `Settle(n-1)`; advance when it becomes zero |
| Ready capture | Input | atomically arm `{generation, request_sequence}` once and commit to WaitingCapture |
| WaitingCapture | Input | no advance; no re-arm |
| WaitingCapture | committed Present carrying matching generation | complete durable capture transaction, then advance |
| semantic wait | typed menu event | exact from/to match commits pass and advances; mismatch is terminal |
| finish | either admitted service after prior commits | reserve/commit success terminal |
| Terminal/Finalizing/Finalized | any | no reducer work; conservative shell behavior |

`tap hold=N` produces held state in exactly N ordinary `UpdateInputState` calls. The boundary-timeout callback still permits the surrounding C `DoInput` to perform its one ordinary update, but because terminal release occurs before it, it is not an admitted held update.

### 2.4 Capture generation

Capture uses `AtomicU64 capture_generation` and `AtomicU64 requested_generation` (`0` means none). Arming reserves a nonzero generation with checked add, stores request metadata under the runtime mutex, then release-stores `requested_generation`. The presentation boundary acquire-loads it, copies only for nonzero generation, and returns an owned snapshot tagged with that exact generation. Completion re-locks and validates all of: runtime still Running/WaitingCapture, pending generation equals snapshot generation, request sequence matches, and generation has not already committed. Stale, duplicate, zero, future, or wraparound generations cannot advance; active mismatches are terminal. Finalization release-stores zero before dropping state.

## 3. C-facing ABI shell contract

### 3.1 Covered shells and conservative results

The contract applies to `rust_automation_service_do_input`, `rust_automation_after_input_update`, every non-DoInput safe point/observer, typed main-menu observer, inactive transport pump/poll/dispatch/update hooks, capture/present automation notification reached by `rust_gfx_postprocess`, and every added automation accessor exported to C. It also applies around the complete existing `rust_do_restart_frame` and `rust_start_game` shells, not merely around observer calls.

Conservative results are: service/after/safe point = stop (`1`); menu observer = `Stop`; `rust_do_restart_frame` = `0`; `rust_start_game` = `0`; pointer/query exports = null/error; void exports return after fallback. `rust_gfx_postprocess` contains panic around the complete existing present shell; a panic is converted to terminal fallback when active and never crosses C.

### 3.2 Exact shell order

1. Increment the shell-specific `ABI_ENTRY` atomic with a nonwrapping/saturating diagnostic operation. This is the only permitted inactive observation and allocates nothing.
2. Acquire-load the relevant activation atomic. For direct automation callbacks, false returns the neutral inactive result immediately: no TLS access, depth mutation, runtime mutex, heap allocation, formatting/logging, C/SDL call, input/activity mutation, trace/artifact creation, or scheduler work. Inactive observer is `Continue`; inactive service/after/safe point is no-stop (`0`). Existing non-automation behavior in `rust_do_restart_frame`, `rust_start_game`, and `rust_gfx_postprocess` still runs; only their automation subcall uses this fast path.
3. Increment `ACTIVE_GATE_ENTRY` only after the gate succeeds. These counters are distinct from `ABI_ENTRY`; inactive proof expects ABI calls at instrumented sites where applicable but active-gate entries/service/setter writes all zero.
4. Enter a thread-local depth guard. Depth other than zero does not lock; it latches lock-free fallback `ReentrantCallback`, requests abort, releases keys from the lock-free mirror outside any mutex, and returns conservative.
5. Wrap every subsequent operation, including pointer validation and external effects, in one outer `catch_unwind(AssertUnwindSafe(...))`. The guard restores depth on every return/unwind.
6. Acquire runtime mutex. Poison recovery may inspect/take state only to produce a terminal plan; normal reducer execution never resumes. Run a pure transition only. It reserves a checked monotonic sequence and returns `Reservation {sequence, generation, planned_state_version}` plus owned `EffectPlan`. It does not commit scheduler advancement yet.
7. Unconditionally drop the runtime guard before C, SDL, graphics acquisition/present, condition-variable wait, trace/file operation, logging, or callback invocation.
8. Execute external effects. Owned-key writes are bounds-checked. On each successful write, update the lock-free owned-key value/mask mirror with release ordering. Terminal release uses only that mirror and safe C setters, then clears mirror bits after successful release.
9. Publish the reserved trace/file result through the ordered commit protocol below. An RAII reservation guard publishes either success or cancelled/failure so a missing sequence can never block later commits.
10. Re-lock runtime and commit only if sequence, generation, state version, and terminal mirror still match. A stale/duplicate commit cannot advance. Unlock before any resulting external work. Repeat only for explicitly table-authorized zero-callback actions with a fresh reservation.
11. On any error or panic, first-wins CAS the lock-free terminal mirror, release-store abort requested, clear capture request, publish cancellation for any reservation, release mirrored keys and OR `CHECK_ABORT` outside every Rust mutex, and return conservative. A later healthy shell imports the mirror as secondary/primary evidence without replacing the first outcome.

Atomic mirrors are fixed-size lock-free atomics only: terminal class/status, abort requested, runtime phase, capture request generation, per-menu-key owned bit/value, ABI entry counters, active-gate counters, service/setter counters, and inactive transport counters. Startup must fail active automation if the target cannot provide the required lock-free atomics. No atomic stores references or writer ownership.

## 4. Ordered trace/file transactions and finalization

Sequence reservation and state commit are separate. Under the runtime mutex, a pure transition reserves one sequence and immutable record/effect payload. After unlock, external work runs. Records enter a dedicated `OrderedCommit` synchronization object, not the runtime mutex. It waits synchronously for `sequence == next_to_publish`, writes exactly that record (or advances an in-memory cancelled slot after a prior fatal sink error), flushes according to the record contract, records result, advances cursor, and notifies waiters. The runtime lock is never held while waiting or writing.

Every reservation has an RAII cancellation publication path. First sink failure atomically terminal-latches `TraceFailure`; later reservations do not write success records but are cancelled and advance the in-memory cursor. There is no deadlock from a skipped sequence. Callback-specific integration is mandatory: P06 integrates input and menu records; P07 integrates present/capture records; P05 integrates start/run-end/finalization records; P08 validates all of them.

Capture is a nested two-phase transaction: reserve capture/record sequence -> unlock -> validate/copy tagged surface -> create-new temporary file in the destination directory -> encode/flush/recover/sync/close -> atomically publish to the exclusive final name without overwrite -> supported directory sync -> ordered capture record -> validate/re-lock/commit scheduler. Any failure removes only the owned temporary path where possible, latches failure, and never emits capture success. Final-name visibility therefore cannot precede completed file sync; record visibility cannot precede final-name publication/directory-sync classification.

Finalization atomically changes phase to Finalizing and takes runtime ownership once. It release-clears capture request and active automation gate before waiting for active shell count to reach zero. It closes the reservation stream, cancels any uncommitted reservations in order, attempts exactly one ordered `run_end`, flushes/syncs/recovers/closes the trace, drops all automation file handles, and stores lock-free final status before marking Finalized. It performs no C/SDL call while holding runtime or ordered-I/O mutex. `teardown_subsystems` runs afterward. Active mode then creates its active teardown receipt; inactive smoke creates a separate `inactive-teardown-complete.json` only after inactive counters are finalized and teardown returned. Neither receipt is a trace record or proof of the other mode.

## 5. Menu observer synchronous control flow

Use a typed `CallbackControl::{Continue, Stop}`. At navigation commit the exact order is:

```text
draw new selection
state.cur_state = new_item.as_u8()
ops.sync_cur_state(state.cur_state)
control = observe_main_menu_transition(from, new_item)
if control == Stop: return Stop immediately
state.last_input_time = now
return Continue
```

`handle_navigate` returns `CallbackControl`; `do_restart_frame` propagates `Stop` before sleep/frame work and maps it to its existing false/stop result; the complete `rust_do_restart_frame` ABI shell catches panic and maps panic/terminal to `0`. `restart_menu_impl`, `try_start_game_impl`, and `start_game_impl` check the outer terminal guard before and after nested calls and cannot convert Stop into retry. The complete `rust_start_game` ABI shell catches panic from construction, `start_game_impl`, observer, and cleanup and returns `0` after fallback. Unit, linked, and real tests force Continue, Stop, and panic at each layer and prove no post-Stop sleep/sound/retry/action.

## 6. Inactive authenticated Unix datagram transport

The adapter exists only when proof-only inactive-smoke options are present; normal inactive launches do not bind, allocate, poll, or write artifacts. Its proof-smoke gate is independent from the automation active gate: transport/counter hooks may run while automation callbacks still take their neutral inactive path. Setup uses an exclusive run root, `AF_UNIX/SOCK_DGRAM`, mode 0600 directory/socket, a random 256-bit nonce, maximum checked socket path length, peer credentials where the platform supports them, closed typed packets `{version, nonce, command_id, command}`, and monotonic duplicate/replay rejection. Every accepted/rejected packet receives a typed acknowledgement; only authenticated accepted commands may push events. Darwin is explicitly classified as not supporting peer credentials for `SOCK_DGRAM`: the executed `LOCAL_PEERCRED` datagram probe returns `EINVAL`; Darwin therefore retains the exclusive path, 0600 permissions, nonce authentication, and replay rejection without claiming a credential check. Stream-socket credential support does not satisfy this datagram contract.

Concrete main-thread points:

1. Immediately before each existing C `TFB_ProcessEvents()` call in the Rust-owned-main pump sites (`gameinp.c::DoInput` and the `rust_thrcommon.c` TaskSwitch/SleepThread pump macros), call a nonblocking transport pump with a fixed packet cap. It may only authenticate and `SDL_PushEvent`; it never mutates VControl/input/activity. Counter: datagram accepted/rejected and push success/failure.
2. In `sdl_common.c::TFB_ProcessEvents`, immediately after successful `SDL_PollEvent` and before `ProcessInputEvent`, call the proof poll-counter hook with event type/command id. This proves C SDL polling.
3. In the production `ProcessInputEvent -> VControl_HandleEvent -> rust_VControl_HandleEvent` path, increment a matching dispatch counter only when the tagged key event reaches Rust dispatch. Do not count the separate `rust_gfx_process_events` event pump.
4. Immediately after each inventoried ordinary `UpdateInputState`/`c_UpdateInputState` consumer, call the inactive post-update hook. For the requested menu-down key, acknowledge `key_observed` only when `CurrentInputState.menu[KEY_MENU_DOWN]` is nonzero in an ordinary update.
5. `quit_smoke` pushes a real `SDL_QUIT`. It acknowledges `quit_pushed`, then `quit_polled` only from point 2. Cooperative smoke stop may be requested only after `QuitPosted` is observed by the existing Rust-owned-main lifecycle boundary; it then unwinds and runs normal teardown. The command handler cannot stop directly.

The actual menu-down binding is queried in the initialized child via a narrow C accessor over production resources: iterate `menu.down.1`, `.2`, ... exactly as `register_menu_controls`, parse each with production `VControl_ParseGesture`, select the first `VCONTROL_KEY`, and return its ABI key code plus a stable binding identity. No binding, non-key-only bindings, malformed values, or ambiguity without a deterministic first-key rule produces a negative ack and failed proof. The parent never assumes an SDL key. Event construction uses `sdl2::sys` ABI-authoritative `SDL_Event`/keyboard fields or a C constructor/pusher accessor compiled against the linked SDL headers, including timestamp/windowID/state/repeat/scancode/sym/mod.

Counters distinguish: per-shell `ABI_ENTRY`, `ACTIVE_GATE_ENTRY`, automation service, setter writes, C poll, Rust dispatch, post-update, datagrams, pushes, acknowledgements, and teardown. In inactive smoke, automation ABI entry may be nonzero because real sites call the shell, but active-gate entry, automation service transitions, setter writes, automation output, and active teardown receipt must be zero. The separate child-written inactive teardown receipt must show socket close/unlink, counter sink close, no pending acknowledgements, normal subsystem teardown returned, and no automation artifacts. Child reap necessarily occurs afterward and is proved only by the parent `ChildSession`/proof report; the receipt must not falsely claim its own process was already reaped.

## 7. SDL capture ABI and linked graphics test

Rust capture must use `sdl2::sys::SDL_Surface` and `sdl2::sys::SDL_PixelFormat` from the same linked SDL2 ABI, or narrow C accessors compiled against those headers for width/height/pitch/pixels/BPP/R/G/B masks and `SDL_MUSTLOCK`. Do not extend or dereference the hand-written partial `SDL_Surface`/`c_void` format as a pixel-format ABI. Prefer C accessors for the `SDL_MUSTLOCK` macro because it is not a function symbol. Lock/unlock use real linked `SDL_LockSurface`/`SDL_UnlockSurface`; a successful lock owns an RAII guard before any read.

The production-linked present harness calls `TFB_InitGraphics` once and relies on it to initialize the DCQ; it never calls `Init_DrawCommandQueue` directly. It calls `TFB_UninitGraphics` once for reverse teardown. Supported CI/headless setup is `SDL_VIDEODRIVER=dummy`, software renderer, 320x240 nonfullscreen hidden window when the Rust initializer gains proof-only hidden-window support; preflight must execute this setup and record driver/renderer. If dummy+hidden is unavailable, the gate is BLOCKED rather than silently switching to fake surfaces.

In addition to skip/no-redraw/forced-redraw cases, linked tests create/use a real SDL surface that satisfies the linked `SDL_MUSTLOCK` predicate (or preflight proves a deterministic linked constructor/flag combination), invoke the production accessor/lock/copy/unlock path, and verify bytes. A separate linked fault seam calls the same production lock helper while forcing `SDL_LockSurface` failure before reads and verifies terminal/no-read/no-success; it may inject the lock function result but may not replace the tested guard/copy logic. Unit fake tests remain supplemental.

## 8. Linked harness P00/P00a feasibility and dependencies

Cargo build-script execution is package-scoped. Exact dependencies are:

1. `sc2/obj/release` exists from `cd sc2 && ./build.sh uqm` with current generated `build.vars`/`config_unix.h`.
2. `build.rs` sorts object paths deterministically, excludes/recompiles `uqm.c` and `gameinp.c` with active flags, and archives `OUT_DIR/libuqm_c.a`.
3. On Darwin harness targets, emit target-specific arguments in this exact left-to-right order: `-L$OUT_DIR`; `-Wl,-force_load,$OUT_DIR/lib<target>_shim.a`; `-Wl,-force_load,$OUT_DIR/libuqm_c.a`; `-lpng16`; `-lz`; `-lm`; `-lSDL2`; `-lobjc`; `-framework Cocoa`; `-framework CoreAudio`; `-framework AudioToolbox`; `-framework CoreFoundation`; `-llzma`; `-lbz2`. The Rust main calls its link anchor before the shim entry. P00a proves extraction/circular Rust-C resolution with a link map. Harness archives cannot leak through unordered global link arguments into other targets. Other platforms require an executed equivalent whole-archive order before support is claimed.
4. Exact rerun inputs are `sc2/build.vars`, `sc2/config_unix.h`, a sorted object-manifest file, each explicitly recompiled C source, compiler dependency files covering included headers, each shim source/header, and `build.rs`. An object path/set change rewrites the manifest and reruns; a directory watch alone is insufficient.
5. P00a's minimal probe reaches source-grounded production symbols already present in the archive: `DoInput`/`AnyButtonPress` in `gameinp_rust_main.o`, `DoConfirmExit` in `confirm.c.o`, `TFB_ProcessEvents`/`TFB_SwapBuffers` in `sdl_common.c.o`, `ProcessInputEvent` in `input.c.o`, and `TFB_FlushGraphicsEx` in `dcqueue.c.o`. The new setter must live with production immediate state or in a shared production guard helper used by all real setter sites.
6. `nm -A`/link-map evidence records defining members. Deliberately bypassing each production helper/site must make the harness fail.

P00 fixes the existing `-luqm_rust` integration-test linker failure before this probe can pass. P06/P07 extend the proven mechanism; they do not invent it for the first time.

## 9. ChildSession lifecycle

`ChildSession` is a state machine, not a best-effort `Drop`-only guard. It owns `Child`, process identity, stdin if any, taken stdout/stderr read ends, two named reader threads, bounded captured output, socket/manifest paths, and states Running -> StopRequested -> Reaped -> PipesClosed -> Joined -> Complete.

Normal completion order is: poll `try_wait`; `Some(status)` is the successful reap and is stored exactly once (do not call `wait` again) -> drop parent stdin -> drop/close remaining parent pipe handles -> readers drain EOF -> join both -> validate. If no reap occurred, failure/deadline/panic order is: record cause -> cooperative child stop when applicable -> bounded `try_wait` poll -> child-only `kill` if live -> call/retry `wait` on `Interrupted` until the one successful reap or classify hard wait failure -> close parent pipe handles -> join readers -> remove socket -> orphan check. Kill/already-exited/reader errors do not skip the required reap/cleanup. Reader threads never own `Child` or outlive `finish`.

`Drop` is an emergency, nonpanicking backstop that performs child-scoped kill/wait and closes pipes; acceptance requires explicit `finish` to reach Complete and join readers. Fault tests cover spawn-partial failure, panic after spawn, stdout reader error, stderr reader panic containment, output cap, child inheriting pipe to grandchild, cooperative-stop timeout, kill error, already exited, `wait` interruption, wait hard failure classification, join panic, socket cleanup failure, and PID reuse identity. No detached thread, zombie, or global/name-based signal is permitted.

## 10. Phase ownership and evidence separation

- P00/P00a: existing strict baseline, cargo-test linker blocker, actual Clippy inventory, executable environment probes, and minimal linked-harness feasibility.
- P01/P01a: closed script and validated budget relationship.
- P02/P02a: exact reducer tables/timelines and atomic capture-generation model as pure types.
- P03/P03a: ordered reservation/commit and durable/exclusive file primitives only.
- P04/P04a: pure shell/fallback/mirror/finalization state model.
- P05/P05a: activation, lifecycle, finalization integration, active teardown receipt.
- P06/P06a: input/safe-point/menu shells, synchronous Continue/Stop propagation, input/menu trace integration, complete update/abort inventory.
- P07/P07a: present/capture shell, ABI-authoritative SDL access, atomic generation integration, present/capture trace integration, real linked lock tests.
- P08/P08a: `REQ-SEM-002`, inactive transport/counters/acks, inactive teardown receipt, `ChildSession`, autonomous real proofs, final cross-callback trace validation.

Active teardown proof and inactive teardown proof are separate runs, schemas, paths, and acceptance assertions. `REQ-ARCH-001..004` remain OPEN in final evidence; transitional C accessors/hooks are explicitly removal-boundaries, never the final architecture.
