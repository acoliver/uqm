# Phase 05: CLI, Setup, Lifecycle Finalization, and Outer Guard Foundation

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P05`

Require `.completed/P04.md`. Own `REQ-MODE-001..003`, `REQ-BUILD-001`, `REQ-EXIT-006`, `REQ-EXIT-008..009`, and lifecycle integration portion of `REQ-FFI-005`. Do not modify input loops/menu commit/graphics or claim their integration.

## Files

Modify `rust/src/main.rs`, `rust/src/mainloop/{options,init_sequence,game_loop}.rs`, automation setup/runtime/trace/identity, `rust/build.rs` for compile-time capabilities, and focused lifecycle tests. Preserve existing options user edit (`OPT_PC`) and all source work.

## TDD/integration

1. Paired CLI, direct inactive fast-path allocator/TLS/lock/external-work tests, incomplete-pair before `run_uqm` tests. ABI-entry and active-gate counters remain distinct.
2. Build capabilities and required lock-free atomics derive from executed P00 probes; unsupported active setup fails before C init.
3. Setup validates script and creates output exclusively before install; partial setup RAII closes files.
4. Refactor `run_uqm` around a testable lifecycle trait: game init/run, automation finalize, existing teardown, active receipt, status. Init-failure behavior has explicit tests.
5. Integrate lifecycle trace using reservation/publish/commit. Finalization clears active/capture gates, drains active shells/reservations, takes state once, cancels gaps in order, attempts exactly one `run_end`, flushes/recovers/syncs/closes trace and every output handle, then calls teardown with no automation mutex/I/O lock held.
6. Only after active teardown returns, create-new/flush/sync/close `teardown-complete.json`. This active receipt is absent in inactive smoke; P08 owns separate `inactive-teardown-complete.json`. Teardown panic/error cannot emit either false receipt.
7. Install outer terminal-guard API at Rust game/restart orchestration boundaries without yet changing C loops; P06 connects exact callers.
8. Forced panic/reservation-gap/finalization/late-callback tests prove exactly-once state take/run_end and nonzero failure status.

Use current pseudocode 001 setup and pseudocode 003 finalization sections plus execution-contract §4. Production main may call process exit only after lifecycle returns; callbacks may not. Run focused lifecycle tests and all strict gates. Handoff only.
