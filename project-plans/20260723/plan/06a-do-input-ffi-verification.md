# Phase 06a: Verify Input/Menu/Terminal Integration

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P06.VERIFY`

Require P05 marker/P06 evidence. Separate verifier independently regenerates clear/non-DoInput inventories and inspects actual statement order.

## Commands/gates

Build/run declared `automation-input-boundary`, inspect `nm -A` origin for real `DoInput` and setter in production archive, build actual `uqm`, run focused Rust tests, full strict gates, source negative searches. Do not accept a unit shim as production C proof.

## Mandatory semantic rejection gates

FAIL unless all are true:

- service after both pumps and before sole update;
- post observer returns stop and combined stop is checked before journal, sounds, inputCallback, and InputFunc;
- forced observation failure records all four later counters zero;
- every input/menu ABI shell has distinct ABI/active counters, inactive no-work path, depth, full catch, pure reserve, unlock-before-external, ordered publish/cancel, matching commit, lock-free key/terminal mirrors, fallback and conservative return; full `rust_do_restart_frame` and `rust_start_game` are panic-contained;
- typed observer is after draw, actual assignment and C sync, and Continue/Stop/panic propagates through `handle_navigate`/`do_restart_frame`/ABI before sleep/later work;
- P06 proves typed observer behavior but does not claim P08-owned `REQ-SEM-002` real movement;
- input/menu records are strictly ordered and writer/publish failure cannot commit success;
- restart/setup/battle/pick-melee/FMV/ConfirmExit/BackgroundInitKernel/MeleeGameOver/AnyButtonPress/current `talk_segue.rs::c_UpdateInputState`/outer Rust matrix has real-site or shared-production-helper evidence, and regenerated inventory has no unexplained row;
- ordinary inactive/local abort behavior is preserved;
- no automation `c_UpdateInputState` call/current-pulsed write/global exit exists;
- harness does not duplicate production tested functions.

Mutation checks remove observation stop check, move observer before commit, clear terminal at each matrix site, and allow ConfirmExit sound; each must fail.

On FAIL emit `Phase 06: FAIL`, no marker. On PASS emit `Phase 06: PASS`, update tracker, create `.completed/P06.md` with symbol origins, exact matrix, forced-panic results, command exits, preservation, and semantic PASS.
