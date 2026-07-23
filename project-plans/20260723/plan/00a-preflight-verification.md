# Phase 00a: Verify Clean Baseline and Source Preflight

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P00.VERIFY`

## Prerequisites

Separate verifier. Require P00 preservation/RED/remediation evidence and no marker. Do not fix source. Inspect all current plan/rules and compare working tree against the P00 snapshot.

## Independent strict gate

Run and preserve complete output/exits:

```bash
cd /Users/acoliver/projects/uqm/rust
cargo check --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Any nonzero result is `Phase 00: FAIL`; there is no baseline waiver. Verify no lint/config weakening, blanket allows, skipped targets, or removal of tests. Inspect all P00 semantic deltas and run their focused tests.

## Preservation and source grounding

Confirm original user hunks survive and no production automation was implemented. Independently verify:

```bash
rg -n 'DoInput|TaskSwitch|UpdateInputState|inputCallback' sc2/src/uqm/gameinp.c
rg -n '&= *~CHECK_ABORT' sc2/src/uqm
rg -n 'DoConfirmExit|UpdateInputState' sc2/src/uqm/confirm.c
rg -n 'state.cur_state = new_item.as_u8|sync_cur_state' rust/src/mainloop/restart_menu
rg -n 'rust_gfx_postprocess|canvas.present' rust/src/graphics/ffi.rs
rg -n 'TFB_SwapBuffers|TFB_FlushGraphicsEx' sc2/src/libs/graphics
rg -n "USE_RUST_(THREADS|GFX|COMM|RESTART)='1'" sc2/build.vars
rg -n 'rustc-link-arg-bin=uqm|gameinp.c' rust/build.rs
```

Required conclusions:

- two pump opportunities precede the planned injection point;
- one update precedes sound/callback/InputFunc;
- post-observer stop must be added before all three;
- abort clear inventory includes restart/setup/battle/pick-melee/FMV and non-DoInput ConfirmExit;
- active typed main-menu commit is Rust `handle_navigate` assignment plus C sync;
- present call returns `()` and swap has skip/no-redraw paths;
- active build has required capabilities and binary-specific C link.

## Executable P00a feasibility gates

Independently execute P00's probe scripts. Require actual success for lock-free atomics; monotonic clock; Unix datagram permissions/path/auth roundtrip; exclusive/durable file primitives; process identity; dummy+hidden 320x240 software SDL; linked ABI SDL format/MUSTLOCK accessor and real lock-required surface; initialized-child actual `menu.down.N` production VControl key query; and capability flags. Unsupported assumptions are BLOCKED/FAIL, never silently faked.

Build/run the minimal declared linked C harness and preserve a link map plus `nm -A`. It must extract/call source-grounded production members (`gameinp_rust_main.o`, `confirm.c.o`, `sdl_common.c.o`, `input.c.o`, `dcqueue.c.o`) or extracted shared production helpers called by every real site. Verify deterministic object sorting, `build.vars`/`config_unix.h`/source/header/shim rerun dependencies, Rust anchor retention, shim/archive/group/force-load/external-library ordering, and deliberate-bypass mutation failure. Confirm ordinary Cargo tests no longer fail with `ld: library 'uqm_rust' not found`.

Verify the final plan includes exact reducer timelines, complete ABI shells/mirrors/two-phase commit, callback-specific trace integration, separate inactive receipt, atomic capture generation, ABI-authoritative SDL tests, no double DCQ init, synchronous menu Stop propagation, `talk_segue` inventory, `ChildSession`, autonomous real proof, and `REQ-SEM-002` ownership in P08. Missing any item is FAIL.

## Decision and marker

On failure emit `Phase 00: FAIL` with exact command/file and create no marker. On complete success emit `Phase 00: PASS`, update tracker, and create `.completed/P00.md` containing preservation path, strict outputs/exits, semantic-remediation review, source findings, plan feasibility, and PASS. Only then P01 starts.
