# Phase 08: Inactive SDL Transport and Autonomous Real-Binary Proof

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P08`

Require `.completed/P07.md`. Own `REQ-SEM-002`, `REQ-TRANSPORT-001..003`, `REQ-PROOF-001..008`, end-to-end `REQ-TRACE-004..005`, cross-callback trace validation, inactive teardown receipt, watchdog/hard-hang proof, and final architecture review (`REQ-ARCH-001..004` remain OPEN).

## Files/scope

Create proof modules/tool/tests/scripts/review and modify only required lifecycle/SDL/input adapters. Reuse P00/P00a environment/link mechanisms, P03 ordered I/O, and P05-P07 callbacks. No async runtime, custom input mutation, fixed shared root, detached drain, or global/name process signal.

## Inactive authenticated real-child transport

Only proof inactive-smoke options enable transport through a proof-smoke gate independent of the automation active gate; normal inactive callbacks remain neutral/no-work while transport proof hooks run. Under the exclusive run root create a mode-0600 Unix datagram socket, random 256-bit nonce, typed version/command IDs, checked path length, peer-credential check where supported, replay/duplicate rejection, fixed packets-per-pump cap, and typed ack for every accepted/rejected command. Darwin `SOCK_DGRAM` peer credentials are unsupported (`LOCAL_PEERCRED` returns `EINVAL` in the P00 executable probe); Darwin must preserve path, mode, nonce, and replay defenses and must not substitute stream credentials or claim credential verification.

Concrete main-thread integration is mandatory:

1. immediately before existing C `TFB_ProcessEvents()` in `gameinp.c::DoInput` and the `rust_thrcommon.c` TaskSwitch/Sleep pump macros, bounded nonblocking receive/authenticate and only `SDL_PushEvent`;
2. immediately after C `SDL_PollEvent` succeeds and before `ProcessInputEvent`, count/tag the polled event;
3. count matching Rust dispatch only in `ProcessInputEvent -> VControl_HandleEvent -> rust_VControl_HandleEvent`, not the separate consuming `rust_gfx_process_events` pump;
4. after each ordinary inventoried update, count post-update and ack `key_observed` only when real current menu-down is nonzero;
5. after key evidence ack, push real `SDL_QUIT`; ack polled only at point 2; request cooperative stop only after lifecycle observes `QuitPosted`, then normal teardown.

The initialized child queries the actual binding through a narrow production C accessor: iterate `menu.down.1..N` exactly like `register_menu_controls`, use production resources and `VControl_ParseGesture`, select the first `VCONTROL_KEY`, and return ABI key code/binding identity. No guessed default, parent-side parser, non-key-only fallback, or malformed binding is accepted. Construct event via `sdl2::sys::SDL_Event` or a C helper compiled with linked SDL headers, filling timestamp/windowID/state/repeat/scancode/sym/mod.

`inactive-counters.jsonl` separately records datagram accept/reject/replay/acks, push/fail, C poll, Rust dispatch, ordinary post-update observation, real quit push/poll/lifecycle observation, per-shell ABI entry, active-gate entry, scheduler service, and setter writes. Inactive acceptance requires complete tap/quit acknowledgements; active-gate/service/setter=0; ABI entry may be nonzero; no automation output or active receipt.

After normal teardown, close/unlink socket, close/sync counter sink, prove no pending ack, then create-new/flush/sync/close `inactive-teardown-complete.json`. It is distinct from and cannot substitute for active `teardown-complete.json`.

## `ChildSession` supervision

`ChildSession` immediately owns `Child`, PID/start/executable identity, parent stdin, taken stdout/stderr read ends, two named bounded reader threads, socket and manifest.

- Normal: poll `try_wait`; `Some(status)` is the one successful reap and is stored (no later `wait`) -> drop stdin/remaining parent pipe handles -> readers drain EOF -> join -> validate.
- Failure/deadline/panic before reap: record cause -> cooperative child-only stop when applicable -> bounded `try_wait` -> child-only kill if live -> call/retry `wait` on `Interrupted` until the one successful reap or hard wait failure -> close parent handles -> join -> socket cleanup -> identity-aware orphan check.
- Kill error/already-exited/reader error/join panic never skips wait/reap. Reader threads never own Child. Explicit `finish` must reach Complete; Drop is only a nonpanicking child-scoped kill/wait/close backstop.

Fault tests: partial spawn; panic after spawn; stdout error; stderr panic; output cap; grandchild inherits pipe; cooperative-stop timeout; kill failure; already exited; wait interruption; hard wait failure; join panic; socket cleanup failure; PID reuse. No zombie, blocked pipe, detached reader, or non-owned process signal.

## Real proofs

- `main-menu-v1`: capture before, genuine tap hold=1/settle, require exactly typed `NewGame -> LoadGame` from actual commit, capture after, activity check, finish. This owns `REQ-SEM-002`; images are supplemental.
- `watchdog-v1`: reach exact inclusive boundary, prove boundary callback did no scheduler work, sticky abort survives clear sites, nonzero cooperative status, ordered run_end then active receipt, and parent deadline not reached.
- `inactive-smoke`: prove full authenticated/push/C-poll/Rust-dispatch/update/quit/teardown chain and separate inactive receipt.
- controlled hard hang: no callback; `ChildSession` child-only kills/reaps and classifies hard hang distinctly.

Every real run uses a fresh exclusive root and SHA-256 executable/script/content/build/initial-final-config identity. Preflight refuses matching live owned/matching-executable processes and never terminates them. Proof report is create-new only after child Complete, output drained/joined, trace/captures/counters/acks/digests/mode-specific receipt validated, socket removed, and orphan check passes.

## Verification

TDD all validator negatives, transport replay/auth/ack failures, binding failure, counter-path confusion, quit-before-poll, receipt substitution, trace reordering, digest mutation, and `ChildSession` faults. Run strict gates, both production-linked harnesses/link maps, production build, and fresh-root `run`, `watchdog`, `inactive-smoke`, and `hard-hang` flows twice. Search rejects `pkill|killall`, fixed shared deletion, async runtimes, screenshot-only acceptance, and architecture-complete claims.

Worker hands off all roots/manifests/commands/exits and an architecture review marking `REQ-ARCH-001..004: OPEN`; P08a alone creates the marker.
