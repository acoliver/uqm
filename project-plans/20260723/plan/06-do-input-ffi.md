# Phase 06: Input, Menu Semantics, Terminal Unwind, and Linked C Harness

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P06`

Require `.completed/P05.md`. Own `REQ-INJECT-001..007`, `REQ-SEM-001` (observer/in-process propagation only), `REQ-EXIT-004..007` integration, input/menu portions of `REQ-FFI-001..005`, input/menu `REQ-TRACE-001..003` integration, and the P06 extension of `REQ-TEST-002`. `REQ-SEM-002` real machine assertion belongs to P08.

## Source integration

Modify exact symbols:

- `sc2/src/uqm/gameinp.c::DoInput`: service after `TaskSwitch`; one update; post observer returns `int`; combine stop; check immediately before journal/sound/callback/InputFunc. Add bounds-checked setter beside authoritative immediate state. Preserve user's no-flush change.
- `sc2/src/uqm/confirm.c::DoConfirmExit`: panic-contained safe point before update and post-update stop; terminal path skips response/menu sounds and exits.
- Rust automation FFI/runtime: every input/menu C-facing shell follows execution-contract §3 exactly—ABI/inactive/active counters, inactive neutral no-allocation/no-work path, active depth and complete catch, pure reserve under mutex, unconditional unlock, C/getter/key effects, ordered input/menu record publish/cancel, validated commit, lock-free key/terminal mirrors, fallback and conservative return.
- `handle_navigate`: return `CallbackControl`; preserve typed old item; draw, assign `state.cur_state`, call `ops.sync_cur_state`, then typed observer. Stop propagates immediately through `do_restart_frame` and the full panic-contained `rust_do_restart_frame` shell before timestamp/sleep/later work. Panic-contain the complete `rust_start_game` shell and prevent restart retry.
- Rust lifecycle/restart orchestration plus current clear-site C callers where necessary to apply sticky automation guard without changing ordinary local abort behavior.

## Mandatory regenerated unwind matrix

Search all `&= ~CHECK_ABORT`, `DoInput`, `UpdateInputState`, `c_UpdateInputState`, and loops before editing and attach the complete inventory. At minimum cover active Rust restart clear, inactive C restart parity, setup, battle, pick-melee, FMV, `start_game_impl`, ConfirmExit, BackgroundInitKernel, MeleeGameOver, AnyButtonPress, and current user-edited `rust/src/comm/talk_segue.rs::do_talk_segue -> c_UpdateInputState`. Every direct update uses the real shell or one extracted shared production before/update/after helper called by every real site and linked harness. Preserve ordinary local abort semantics and the user edit. Unexplained site blocks handoff.

## Exact production-linked harness integration

Create:

- `rust/tests/c_harness/input_boundary_main.rs`
- `rust/tests/c_harness/input_boundary_shim.c`
- `rust/tests/c_harness/run-linked.sh`

Extend, do not replace, the P00/P00a proven Cargo harness mechanism. Preserve deterministic sorted archive construction and exact rerun dependencies for `build.vars`, `config_unix.h`, recompiled source/header dependencies, object manifest, and shim. Use the preflight-proven macOS shim/archive group or force-load order, Rust anchor retention, and identical external libraries. The harness must reach `gameinp_rust_main.o::DoInput`/`AnyButtonPress`, `confirm.c.o::DoConfirmExit`, and any extracted shared production update/guard helper called by all real sites. The shim may control dependencies/counters but calls the real production setter/loop/helpers; no copied behavior. Preserve link map/`nm -A` and deliberate real-site/helper bypass failures.

## TDD slices

1. Production setter first/last/invalid/normalization/sentinels; ASan when supported, but sentinel linked test always required.
2. Inactive all-sentinel no-op and service tick/order conflict overlay.
3. Exact real `DoInput` order. Force stop from service and separately from post-observation trace/getter/panic/poison/reentry; in every case update=1, observe=1, journal/sound/inputCallback/InputFunc=0 after stop.
4. Per-shell forced panics before/while/after lock, external C/getter, ordered writer, publish, commit, menu observer, full `rust_do_restart_frame`, and full `rust_start_game`; no unwind, no lock overlap, cancellation gap, stale commit, or lost mirror release.
5. Typed menu Continue/Stop/panic at assignment+sync; each layer propagates synchronously and Stop performs zero timestamp/sleep/sound/retry/later action.
6. Clearing/update matrix and outer guard: every active terminal cannot resume; ordinary inactive/local abort behavior remains.
7. Linked real/shared-helper tests for ConfirmExit, BackgroundInitKernel, MeleeGameOver, AnyButtonPress, and Rust talk-segue direct update; terminal yields zero later response/communication action.
8. Input/menu record ordering and failure tests prove records publish before matching scheduler commit and no writer failure reports success.
9. Negative search proves automation itself never calls `c_UpdateInputState` and never writes current/pulsed arrays; the separately inventoried talk-segue ordinary update remains guarded.

Run harness, `nm`, focused tests, C/Rust production build, and all strict gates. No global exit/signal. Worker handoff only.
