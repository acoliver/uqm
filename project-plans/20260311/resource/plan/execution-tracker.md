# Execution Tracker

Plan ID: PLAN-20260314-RESOURCE

| Phase | ID | Title | Status | Verified | Semantic Verified | Notes |
|------:|------|-------|--------|----------|-------------------|-------|
| 0.5 | P00.5 | Preflight Verification | ⬜ | ⬜ | N/A | Verify `uio_stat`, dead-code build deps, existing tests |
| 1 | P01 | Analysis | ⬜ | ⬜ | ⬜ | 11 gaps mapped to requirements |
| 1a | P01a | Analysis Verification | ⬜ | ⬜ | ⬜ | Requirement coverage matrix + revalidation checklist |
| 2 | P02 | Pseudocode | ⬜ | ⬜ | ⬜ | PC-1 through PC-9 |
| 2a | P02a | Pseudocode Verification | ⬜ | ⬜ | ⬜ | Cross-reference + ABI contingency check |
| 3 | P03 | Value-Type Dispatch — TDD | ⬜ | ⬜ | ⬜ | 6 tests for GAP-3, GAP-4 |
| 3a | P03a | Value-Type Dispatch — TDD Verify | ⬜ | ⬜ | ⬜ | Red phase confirmation |
| 4 | P04 | Value-Type Dispatch — Impl | ⬜ | ⬜ | ⬜ | UNKNOWNRES + get_resource fix |
| 4a | P04a | Value-Type Dispatch — Impl Verify | ⬜ | ⬜ | ⬜ | Green phase confirmation |
| 5 | P05 | res_GetString Parity | ⬜ | ⬜ | ⬜ | 6 tests + STRING type check |
| 5a | P05a | res_GetString Verify | ⬜ | ⬜ | ⬜ | Never-null confirmation |
| 6 | P06 | Lifecycle/Replacement Cleanup | ⬜ | ⬜ | ⬜ | cleanup, replacement, free/detach/remove verification |
| 6a | P06a | Lifecycle/Replacement Verify | ⬜ | ⬜ | ⬜ | direct coverage for REQ-RES-LOAD-007/008 |
| 7 | P07 | SaveResourceIndex Filtering | ⬜ | ⬜ | ⬜ | real-path save verification |
| 7a | P07a | SaveResourceIndex Verify | ⬜ | ⬜ | ⬜ | Config save correctness |
| 8 | P08 | res_OpenResFile + LoadResourceFromPath Guards | ⬜ | ⬜ | ⬜ | `uio_stat`, sentinel rejection, zero-length guard |
| 8a | P08a | Sentinel/Load Helper Verify | ⬜ | ⬜ | ⬜ | Sentinel handle correctness |
| 9 | P09 | Minor Fixes | ⬜ | ⬜ | ⬜ | `u32` + doc fix |
| 9a | P09a | Minor Fixes Verify | ⬜ | ⬜ | ⬜ | ABI + doc correctness |
| 10 | P10 | Dead Code Removal Preflight + Removal | ⬜ | ⬜ | ⬜ | evidence-backed cleanup only |
| 10a | P10a | Dead Code Removal Verify | ⬜ | ⬜ | ⬜ | Single module stack |
| 11 | P11 | Integration Verification | ⬜ | ⬜ | ⬜ | Full engine E2E + revalidation evidence |

## Summary Statistics

- **Total gaps:** 11
- **Total implementation phases:** 8 (P03–P10)
- **Total tests planned:** ~30 new or expanded tests
- **Plan files updated by review:** 12
- **Primary production files expected to change:** `rust/src/resource/dispatch.rs`, `rust/src/resource/ffi_bridge.rs`, `rust/src/resource/mod.rs`
- **Potential files removed:** 6 Rust modules (`ffi.rs`, `resource_system.rs`, `loader.rs`, `cache.rs`, `index.rs`, `config_api.rs`)
- **Potential C-side files affected conditionally:** `sc2/src/libs/resource/rust_resource.h`, `sc2/src/libs/resource/rust_resource.c`
- **Estimated LoC changed:** ~250-400 lines modified, plus any evidence-backed dead-code removal
