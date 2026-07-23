# Phase 08a: Final Independent Real-Binary Verification

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P08.VERIFY`

Require P07 marker/P08 handoff and no P08 marker. A separate verifier uses different exclusive roots and no worker report as proof.

## Independent execution

1. Run strict check/fmt/Clippy/all tests with zero waiver.
2. Re-execute P00 environment probes, build actual C objects/Rust `uqm`, and run both linked harnesses with link maps/`nm -A`.
3. Confirm real source symbols/helpers and single DCQ init; execute dummy+hidden software SDL, real linked lock-required, and forced-lock-failure/no-read tests.
4. Run fresh preflight; refuse matching process, never kill it. Independently execute main-menu, exact-boundary watchdog, inactive-smoke, and hard-hang twice.
5. Parse every JSONL/report/counter/ack/receipt; decode PNG; recompute SHA-256; verify `ChildSession::Complete`, waited PID identity, joined readers, socket removal, and orphan checks.

## Non-negotiable rejection gates

FAIL for any missing/manual-only item:

- authoritative post-increment inclusive reducer/timelines, exact priority, tap boundary-release, atomic nonzero capture generation, and matching two-phase commits;
- each ABI shell's atomic inactive no-allocation/no-work path, distinct ABI/active counters, depth, full catch, pure transition under mutex, unconditional unlock before C/SDL/I/O/wait/log/observer, ordered publish/cancel, lock-free key/terminal mirrors, fallback, and conservative result;
- complete panic containment of `rust_do_restart_frame`, `rust_start_game`, and `rust_gfx_postprocess`;
- synchronous menu Continue/Stop through draw -> assign -> sync -> observer -> `handle_navigate` -> `do_restart_frame` -> ABI, with no post-Stop sleep/action/retry;
- P08-owned real-binary exactly-once typed `NewGame -> LoadGame`;
- callback-specific lifecycle/input/menu/present/capture records and strict cross-callback order; no reservation gap/deadlock or success after sink failure;
- full clear/direct-update inventory including current `talk_segue.rs::do_talk_segue -> c_UpdateInputState`;
- inactive mode-0600 nonce/auth/replay/acks at exact DoInput/TaskSwitch/Sleep pump points; actual child VControl `menu.down.N` key query; ABI SDL event construction; distinct C poll, Rust dispatch and post-update counters; real quit stop only after C poll and lifecycle `QuitPosted` observation;
- inactive active-gate/service/setter zero (ABI entries may be nonzero), no automation artifacts, and separate inactive receipt after socket/counter/ack close and normal teardown; no active receipt substitution;
- ABI-authoritative pixel metadata/MUSTLOCK, real linked lock-required and lock-failure/no-read tests, no double DCQ init, and supported dummy+hidden setup;
- P00/P00a link feasibility evidence, deterministic archive/rerun/order and production symbols/shared helpers called by every real site; copied shim logic fails;
- explicit `ChildSession` kill/reap/pipe-close/join ordering and all fault tests; no reader/kill/join error skips wait, detached thread, zombie, global kill, or report-before-Complete;
- executable/script/content/build/config digests, active run_end/receipt order, autonomous proof, strict zero exits, or preserved user edits.

Mutation validation removes/reorders one semantic event, active gate counter, transport ack/path counter, reservation cancellation, generation validation, SDL lock, mode-specific receipt, digest, real helper call, child wait/join, and autonomous result; validator/tests must fail. A PNG pair alone never passes.

## Decision

On failure emit `Phase 08: FAIL` with exact requirement/path/command and no marker. On success emit `Phase 08: PASS`, update tracker, and create `.completed/P08.md` recording all commands/exits, roots/manifests, child identities/wait/join status, digests, trace order, exact semantic event, inactive counters/acks/receipt, linked origins/lock evidence, strict gates, preservation result, and `REQ-ARCH-001..004: OPEN`. This is final; no further plan-review loop is requested.
