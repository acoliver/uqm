# Phase 01: Typed Script and Activation Contracts

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P01`

## Prerequisite

Require `.completed/P00.md` PASS and no later marker. Preserve P00-clean baseline and user edits. Worker only; P01a verifies.

## Owned requirements

`REQ-BUILD-002`, `REQ-DEP-001..003`, `REQ-SCRIPT-001..006`. This phase owns parsing/types only. It does not claim scheduler execution, runtime terminal behavior, FFI, shutdown, capture, lifecycle, or proof integration.

## Files

Create `rust/src/automation/{mod,error,script}.rs`; modify `rust/src/lib.rs`, `rust/Cargo.toml`, and lockfile only as necessary. Add direct `serde` derive and `serde_json`; no async/duplicate libraries or automation feature.

## TDD slices

1. RED strict UTF-8/root parse tests: malformed, duplicate/unknown/missing fields, unsupported version; GREEN typed DTO/validation.
2. RED budgets/counts/activity/final-finish boundary tests, including negative/fraction/overflow and the inclusive-limit contract: any statically required N admitted input/presentation callbacks require checked maximum at least N+1; GREEN validated newtypes/checked conversion with context-rich insufficiency errors.
3. RED exhaustive six-key mappings against 5..10 and unknown/numeric rejection; GREEN `MenuKey`.
4. RED safe-label tables; GREEN validated label newtype.
5. RED typed `assert_main_menu_transition` parsing using existing `RestartMenuItem` names and unsupported semantic action rejection; GREEN closed `Action` enum.
6. REFACTOR errors to retain path/step/context; rerun all focused and workspace tests.

Use pseudocode 001 lines 005-010. Public APIs are documented; no `unwrap`/`expect` in production paths, placeholder, dynamic string status, or invented raw semantic index.

## Commands/acceptance

```bash
cd /Users/acoliver/projects/uqm/rust
cargo test automation::script --all-features
cargo check --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Inspect dependency tree and search for `RUST_AUTOMATION`, async runtimes, placeholders, new unsafe, and unplanned files. Every gate returns zero. Worker hands off RED/GREEN/REFACTOR logs, exact files, and preservation diff; it does not create marker.
