# Binary Inversion — Overview

*Created 2026-07-08*

## Goal

Make Rust own `main()`. Invert the architecture from **C binary → Rust library**
to **Rust binary → C library**. This eliminates the game thread, the
cross-thread DCQ coordination, and reduces FFI boundary crossings per frame.

## Current State

- `sc2/src/uqm.c` owns `main()` (~700 lines, zero Rust)
- It spawns a separate thread for `Starcon2Main` (now a 1-line Rust delegate)
- Main thread runs SDL event pump + graphics flush
- `rust/src/main.rs` exists but is a Phase-0 stub (calls `c_entry_point` which just prints)
- Shipping binary is compiled from C

## Phased Plan

| Phase | Description | Implementor |
|-------|------------|-------------|
| P01 | Analysis (this document + 01-analysis.md) | done |
| P01a | Verification | deepthinker |
| P02 | FFI externs for all C init/teardown functions | rustcoder |
| P02a | Verification | deepthinker |
| P03 | Event pump FFI externs | rustcoder |
| P03a | Verification | deepthinker |
| P04 | Teardown FFI externs | rustcoder |
| P04a | Verification | deepthinker |
| P05 | Rewrite Rust main.rs with real init + interleaved loop + teardown | rustcoder |
| P05a | Verification | deepthinker |
| P06 | Build integration (Cargo.toml binary, link C as lib, disable C main) | rustcoder |
| P06a | Verification | deepthinker |
| P07 | E2E boot test (Rust binary → menu → melee → shutdown) | manual |
| P07a | Verification | deepthinker |

## Key Constraints

1. SDL requires init + event polling on OS main thread → Rust `fn main()` satisfies this
2. initAudio uses C task dispatch → keep calling through C for now
3. DCQ still works single-threaded (synchronous drain)
4. C globals (options, activity) set during init → set via FFI wrappers

## Definition of Done

- `cargo build` produces a Rust binary that links against C object files
- Running the Rust binary boots UQM to the main menu
- Melee combat works without crashes
- Clean shutdown (no leaks, no hangs)
- C `main()` is compiled out via `#ifdef RUST_OWNS_MAIN`
