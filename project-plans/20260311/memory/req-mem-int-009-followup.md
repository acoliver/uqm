# REQ-MEM-INT-009: Remaining Mixed-Language Seam Test Coverage

## Status
Partially satisfied by PLAN-20260314-MEMORY.P05 (Rust-side ABI integration tests).

## What was delivered
- 6 Rust-side ABI integration tests in `rust/tests/memory_integration.rs`
- Tests verify: alloc+free, calloc zero-fill, realloc preservation, zero-size normalization, lifecycle smoke, realloc-to-zero

## What remains
True compiled C↔Rust seam tests that exercise:
1. A real C caller through `memlib.h` macros (HMalloc → rust_hmalloc linkage)
2. C allocation followed by Rust free
3. Rust allocation followed by C free through an actual compiled C caller
4. Zero-size normalization at the compiled ABI seam
5. Lifecycle sequencing (mem_init/mem_uninit) across the compiled boundary

## Owner
Project-level integration — not owned by the memory subsystem plan.

## Artifact path
This file: `project-plans/20260311/memory/req-mem-int-009-followup.md`

## Acceptance criteria
A compiled C test fixture that links against the Rust library and exercises all 5 items above, with passing test results.
