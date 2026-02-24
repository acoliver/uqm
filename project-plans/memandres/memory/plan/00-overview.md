# Plan: Swap C Memory Allocator to Rust

Plan ID: PLAN-20260224-MEM-SWAP
Generated: 2026-02-24
Total Phases: 9 (plus preflight, analysis, pseudocode)
Requirements: REQ-MEM-001, REQ-MEM-002, REQ-MEM-003, REQ-MEM-004, REQ-MEM-005, REQ-MEM-006, REQ-MEM-007

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 00a)
2. Integration points are explicitly listed — macro redirect in `memlib.h` routes 322+ call sites
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. This is a SIMPLE swap — the Rust implementation already exists and is tested

## Scope

Replace C memory allocator (`w_memlib.c`) with existing Rust implementation (`memory.rs`) via:
1. Fix Rust log level semantic alias (minor)
2. Add `USE_RUST_MEM` flag to `config_unix.h`
3. Add `#ifdef USE_RUST_MEM` macro redirect block to `memlib.h`
4. Add `#error` guard to `w_memlib.c`
5. Add conditional to `Makeinfo`
6. Enable the flag and verify end-to-end

## Requirements Summary

| ID | Title | Phase(s) |
|---|---|---|
| REQ-MEM-001 | Header Macro Redirect | P06, P07, P08 |
| REQ-MEM-002 | C Source Guard | P06, P07, P08 |
| REQ-MEM-003 | Build System Conditional | P06, P07, P08 |
| REQ-MEM-004 | Config Flag | P06, P08 |
| REQ-MEM-005 | OOM Log Level Correctness | P03, P04, P05 |
| REQ-MEM-006 | Behavioral Equivalence | P04, P05, P09 |
| REQ-MEM-007 | Build Both Paths | P07, P08, P09 |

## Execution Tracker

| Phase | Title | Status | Verified | Semantic Verified | Notes |
|------:|-------|--------|----------|-------------------|-------|
| P00a | Preflight Verification | ⬜ | ⬜ | N/A | |
| P01 | Analysis | ⬜ | ⬜ | ⬜ | |
| P02 | Pseudocode | ⬜ | ⬜ | ⬜ | |
| P03 | Rust Fixes — Stub | ⬜ | ⬜ | ⬜ | LogLevel::Fatal alias |
| P04 | Rust Fixes — TDD | ⬜ | ⬜ | ⬜ | Test log level semantics |
| P05 | Rust Fixes — Impl | ⬜ | ⬜ | ⬜ | Apply alias in memory.rs |
| P06 | C Header Redirect — Stub | ⬜ | ⬜ | ⬜ | memlib.h, w_memlib.c, Makeinfo, config |
| P07 | C Header Redirect — TDD | ⬜ | ⬜ | ⬜ | Build verification tests |
| P08 | C Header Redirect — Impl | ⬜ | ⬜ | ⬜ | Enable USE_RUST_MEM |
| P09 | Integration Verification | ⬜ | ⬜ | ⬜ | End-to-end runtime test |
