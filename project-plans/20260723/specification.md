# Specification: Rust Runtime Automation

Plan ID: `PLAN-20260723-RUNTIME-AUTOMATION`
Normative detail: [`analysis/authoritative-execution-contract.md`](analysis/authoritative-execution-contract.md)

## Purpose

Add runtime-opt-in synchronous automation to the active Rust-owned `uqm` executable. It must exercise the real hybrid game, drive real nested input, observe actual post-update input and typed Rust menu semantics, capture the logical surface at present-call completion, and cooperatively unwind through real teardown. Transitional C hooks are removal seams; `REQ-ARCH-001..004` remain OPEN until Rust owns the full path with zero in-tree C implementation.

## Exact execution contracts

- An applicable active callback checked-increments its ordinal before watchdog comparison. Equality with a configured maximum is terminal and admits no scheduler work. Maximum `M` admits at most `M-1` callbacks; input, presentation, wall, clock is the deterministic limit priority after applicable overflow.
- Scheduler transitions are pure. Side effects use checked sequence/state-version reservation under the runtime mutex, unconditional unlock, external execution, ordered publish/cancel, then matching-token commit. Capture adds a nonzero atomic request generation and rejects stale/duplicate completion.
- Direct inactive automation callbacks perform only lock-free ABI-entry observation plus activation load, allocate nothing, touch no TLS/lock/C/SDL/I/O/input/activity, and return neutral. ABI-entry, active-gate, scheduler-service, and setter counters are separate.
- Every active C-facing shell has a depth guard and full `catch_unwind`. Fixed lock-free mirrors hold terminal/status, abort, runtime phase, owned key mask/values, capture generation, and proof counters. Poison/reentry/panic/external failure release mirrored keys and OR abort outside locks, cancel reservations, and return callback-specific conservative results.

## Input/menu call order

```text
DoInput iteration
  existing pumps -> TaskSwitch
  service_do_input() -> service_stop
  UpdateInputState() exactly once
  after_input_update() -> observation_stop
  if service_stop || observation_stop: break immediately
  journal -> sounds -> inputCallback -> InputFunc
```

All direct non-`DoInput` updates receive equivalent before/update/after handling or a source-proven outer guard: ConfirmExit, BackgroundInitKernel, MeleeGameOver, AnyButtonPress, and the current `talk_segue.rs::do_talk_segue -> c_UpdateInputState` edit.

Main-menu navigation order is draw -> assign typed state -> `sync_cur_state` -> typed observer. `CallbackControl::Stop` propagates synchronously through `handle_navigate`, `do_restart_frame`, and full panic-contained `rust_do_restart_frame` before sleep/later work. The complete `rust_start_game` shell is panic-contained and maps terminal/panic to stop.

## Trace, capture, and teardown

All trace/file operations are two-phase and strictly ordered by a dedicated commit cursor; every reservation has RAII cancellation so gaps cannot deadlock. P05 integrates lifecycle records, P06 input/menu records, P07 present/capture records, and P08 validates their cross-callback order.

Capture uses ABI-authoritative `sdl2::sys` types or linked C accessors for `SDL_Surface`, `SDL_PixelFormat`, and `SDL_MUSTLOCK`; it never assumes layout behind the current partial `c_void` format field. A matching generation completes only after copy under real lock when required, temporary create-new encode, flush/recover/sync/close, exclusive final-name publication, directory-sync classification, ordered record, and state commit.

Finalization clears activation/capture, drains shells/reservations, takes state once, writes/closes one `run_end`, and drops all automation handles before subsystem teardown. Active and inactive modes have separate post-teardown receipts; inactive proof cannot pass with the active receipt or vice versa.

## Inactive transport

Only proof inactive-smoke binds an authenticated mode-0600 Unix datagram endpoint. Bounded nonblocking pumps run immediately before existing C `TFB_ProcessEvents` calls in `DoInput` and TaskSwitch/Sleep pump macros. The child queries the actual initialized production-parsed `menu.down.N` `VCONTROL_KEY`, builds ABI-authoritative down/up events, and only uses `SDL_PushEvent`. Evidence separately counts C `SDL_PollEvent`, Rust VControl dispatch, and ordinary post-update observation; the separate `rust_gfx_process_events` pump is not accepted.

Every command has a typed acknowledgement. Real `SDL_QUIT` can stop smoke only after the C poll records it and lifecycle observes `QuitPosted`. Inactive acceptance permits ABI-entry counts but requires active-gate/service/setter counts zero, no automation artifacts, and a separate receipt after socket/counters/ack close and normal teardown.

## Verification architecture

P00 fixes the existing `-luqm_rust` Cargo-test linker failure and the actual strict-Clippy backlog (planning capture: 2,198 error headers; lib-test summary: 2,035 errors), without waiver. P00/P00a execute environment probes and a minimal production-archive Cargo harness before feature phases. Source-grounded members include `DoInput`/`AnyButtonPress`, `DoConfirmExit`, `TFB_ProcessEvents`/`TFB_SwapBuffers`, `ProcessInputEvent`, and `TFB_FlushGraphicsEx`; if direct member extraction is infeasible, every real site and harness must call the same extracted production guard helper.

The linked graphics harness uses preflight-proven dummy video + hidden 320x240 software rendering, relies on `TFB_InitGraphics` for the single DCQ init, and tests real lock-required and forced lock-failure paths. Final proof uses explicit `ChildSession`: `try_wait Some` is the one stored reap; otherwise child-only stop/kill and Interrupted-retried wait obtains it. Pipe close, drain/join, socket cleanup and orphan check follow reap. Kill/reader/join errors never skip required reap.

## Acceptance

Strict fmt/check/Clippy/all tests, linked input/present harnesses, and fresh-root real main-menu/watchdog/inactive/hard-hang flows must pass autonomously. P08 owns `REQ-SEM-002` and must machine-observe exactly `NewGame -> LoadGame`; screenshots are supplemental. Missing callback containment, ordered I/O, real SDL lock proof, transport counters/acks, mode-specific teardown evidence, `ChildSession` completion, digests, or autonomous real-game evidence is failure.
