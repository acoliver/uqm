# Plan: Resource Subsystem Gap Closure

Plan ID: PLAN-20260314-RESOURCE
Generated: 2026-03-14
Total Phases: 20 (P00.5 through P11, including verification subphases P01a, P02a, P03a–P10a)
Requirements: REQ-RES-LIFE-001–007, REQ-RES-TYPE-001–008, REQ-RES-IDX-001–008, REQ-RES-UNK-001–004, REQ-RES-CONF-001–009, REQ-RES-LOAD-001–011, REQ-RES-FILE-001–008, REQ-RES-OWN-001–010, REQ-RES-ERR-001–006, REQ-RES-INT-001–009

## Context

The resource subsystem is **already ported and wired** — `USE_RUST_RESOURCE` is active, the C core files (`resinit.c`, `getres.c`, `filecntl.c`, `propfile.c`, `loadres.c`) are compiled out, and 41 `#[no_mangle]` exports in `ffi_bridge.rs` satisfy the `reslib.h` ABI. The game boots and runs through the Rust resource layer.

This plan addresses **remaining behavioral gaps** between the current Rust implementation and the normative specification/requirements. It does NOT re-implement what already works.

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared

## Gap Summary

### GAP-1: `res_OpenResFile` missing directory sentinel detection
**Files:** `rust/src/resource/ffi_bridge.rs` lines 986–995
**Spec:** §9.2, §14.1 | **Reqs:** REQ-RES-FILE-002, REQ-RES-FILE-003
**Issue:** Currently just calls `uio_fopen()` directly. Missing `uio_stat` call to detect directories and return `STREAM_SENTINEL`. The `uio_stat` extern is not even declared.
**Impact:** Resource loaders that distinguish directory vs. file (e.g., loose-file speech) get wrong behavior.

### GAP-2: `res_GetString` missing STRING type check and empty-string fallback
**Files:** `rust/src/resource/ffi_bridge.rs` lines 786–817
**Spec:** §8.3, §14.2 | **Reqs:** REQ-RES-CONF-003, REQ-RES-ERR-003
**Issue:** Returns `fname` for ANY type's entry, not just STRING. Returns `null` on missing key instead of `""`. Spec requires: (a) type must be "STRING", (b) missing/mismatch returns `""` not null.
**Impact:** `res_GetString("config.sfxvol")` on an INT32 entry returns the descriptor "128" instead of "". Null return on missing key crashes callers.

### GAP-3: `UNKNOWNRES` stored as heap type instead of value type
**Files:** `rust/src/resource/dispatch.rs` line 89, `ffi_bridge.rs` line 190
**Spec:** §5.1, §5.2 | **Reqs:** REQ-RES-UNK-001, REQ-RES-UNK-003, REQ-RES-LOAD-011
**Issue:** `UNKNOWNRES` is registered with `(None, None, None)` — no loadFun. When an unknown type is encountered, `is_value_type` is set to `false` (line 89), so it's treated as a heap type. Spec says UNKNOWNRES is a value type (no freeFun) whose loadFun stores descriptor as str_ptr.
**Impact:** `res_GetResource` on UNKNOWNRES entries tries lazy-load dispatch (fails, returns null) instead of returning the stored descriptor string.

### GAP-4: `get_resource` doesn't handle value types correctly
**Files:** `rust/src/resource/dispatch.rs` lines 128–168
**Spec:** §7.1 | **Reqs:** REQ-RES-LOAD-011, REQ-RES-LOAD-003
**Issue:** After lazy-load check, checks `data.ptr.is_null()`. For value types (STRING, INT32, BOOLEAN, COLOR), ptr IS null (data is in `num` or `str_ptr`). Returns null instead of the value. Spec requires: value types return `str_ptr` for string types, `num` cast to pointer for numeric types, and increment refcount.
**Impact:** `res_GetResource("config.sfxvol")` on an INT32 returns null.

### GAP-5: `UninitResourceSystem` doesn't free loaded heap resources
**Files:** `rust/src/resource/ffi_bridge.rs` lines 280–293
**Spec:** §4.3 | **Reqs:** REQ-RES-LIFE-004, REQ-RES-OWN-005, REQ-RES-OWN-010
**Issue:** Simply sets state to `None`, dropping Rust-side data structures. Does NOT iterate entries and call `freeFun` on loaded heap resources.
**Impact:** Loaded resources (graphics frames, sound banks, etc.) leak on shutdown.

### GAP-6: Entry replacement doesn't free old loaded heap resources
**Files:** `rust/src/resource/dispatch.rs` line 117
**Spec:** §6.4 | **Reqs:** REQ-RES-OWN-009, REQ-RES-IDX-006
**Issue:** `process_resource_desc` does `self.entries.insert(key, desc)` which overwrites the old entry. If the old entry had a loaded heap resource, `freeFun` is never called.
**Impact:** Memory/resource leak when reloading indices or calling `res_Put*` to change types.

### GAP-7: `SaveResourceIndex` emits entries without `toString`
**Files:** `rust/src/resource/ffi_bridge.rs` lines 396–461
**Spec:** §6.3 | **Reqs:** REQ-RES-IDX-005, REQ-RES-UNK-002
**Issue:** When no `toString` is found, falls through to `format!("{}:{}", desc.res_type, desc.fname)`. Spec requires: skip entries whose handler has no `toString`. UNKNOWNRES and heap types without toString should NOT be emitted.
**Impact:** Content index entries (GFXRES, SNDRES, etc.) get spuriously saved into config files.

### GAP-8: `CountResourceTypes` return type is `u16` instead of `u32`
**Files:** `rust/src/resource/ffi_bridge.rs` line 664
**Spec:** §5.4 | **Reqs:** REQ-RES-TYPE-004
**Issue:** Spec says `u32`, code returns `u16`. ABI mismatch.

### GAP-9: `LoadResourceFromPath` sentinel/zero-length guards incomplete
**Files:** `rust/src/resource/ffi_bridge.rs` lines 1124–1158
**Spec:** §9.3 step 2, step 4 | **Reqs:** REQ-RES-FILE-005, REQ-RES-FILE-008
**Issue:** The helper does not fully reject non-loadable opens before invoking the loader callback. In particular, it lacks the required zero-length guard and the plan must explicitly treat `STREAM_SENTINEL` as an open failure for file-backed loads.
**Impact:** Loader callbacks can be invoked for invalid resource handles or empty files, producing confusing failures and avoidable handle-management risk.

### GAP-10: `GetResourceData` misleading doc comment
**Files:** `rust/src/resource/ffi_bridge.rs` lines 1160–1163
**Spec:** §9.4, §14.3 | **Reqs:** REQ-RES-FILE-006
**Issue:** Comment says "seek back 4 bytes" but code correctly does NOT seek back. Code matches spec; comment is wrong.

### GAP-11: Non-authoritative dead code modules
**Files:** `rust/src/resource/ffi.rs`, `resource_system.rs`, `loader.rs`, `cache.rs`, `index.rs`, `config_api.rs`
**Spec:** Appendix A.2 | **Reqs:** REQ-RES-INT-006
**Issue:** These modules are not on the authoritative path. They add confusion, compilation burden, and risk of accidental use. Spec explicitly says "must not be used as an implementation target."

## Requirement Coverage Notes

Requirement coverage in this plan is tracked in two buckets:
1. **Covered by change work:** requirements directly addressed by one or more gap-closing implementation phases.
2. **Covered by explicit verification:** requirements believed to be already implemented but still revalidated through targeted or integration verification.

The analysis matrix and later verification phases must keep those buckets distinct so the plan does not overstate coverage.

## Phase Structure

| Phase | ID | Title | Gaps Addressed |
|------:|------|-------|----------------|
| 0.5 | P00.5 | Preflight Verification | — |
| 1 | P01 | Analysis | All gaps |
| 1a | P01a | Analysis Verification | Requirement coverage + revalidation mapping |
| 2 | P02 | Pseudocode | GAP-1 through GAP-10 |
| 2a | P02a | Pseudocode Verification | Pseudocode/spec/ABI cross-check |
| 3 | P03 | Value-type dispatch fix — TDD | GAP-3, GAP-4 |
| 3a | P03a | Value-type dispatch fix — TDD Verification | GAP-3, GAP-4 |
| 4 | P04 | Value-type dispatch fix — Impl | GAP-3, GAP-4 |
| 4a | P04a | Value-type dispatch fix — Impl Verification | GAP-3, GAP-4 |
| 5 | P05 | res_GetString parity — TDD + Impl | GAP-2 |
| 5a | P05a | res_GetString parity — Verification | GAP-2 |
| 6 | P06 | Lifecycle & replacement cleanup — TDD + Impl | GAP-5, GAP-6 |
| 6a | P06a | Lifecycle & replacement cleanup — Verification | GAP-5, GAP-6 |
| 7 | P07 | SaveResourceIndex filtering — TDD + Impl | GAP-7 |
| 7a | P07a | SaveResourceIndex filtering — Verification | GAP-7 |
| 8 | P08 | res_OpenResFile sentinel + load-from-path guard — TDD + Impl | GAP-1, GAP-9 |
| 8a | P08a | res_OpenResFile sentinel — Verification | GAP-1, GAP-9 |
| 9 | P09 | Minor fixes — Impl | GAP-8, GAP-10 |
| 9a | P09a | Minor fixes — Verification | GAP-8, GAP-10 |
| 10 | P10 | Dead code removal preflight + removal | GAP-11 |
| 10a | P10a | Dead code removal — Verification | GAP-11 |
| 11 | P11 | Integration verification | All |

## Execution Order

P00.5 → P01 → P01a → P02 → P02a → P03 → P03a → P04 → P04a → P05 → P05a → P06 → P06a → P07 → P07a → P08 → P08a → P09 → P09a → P10 → P10a → P11

Each phase must pass verification before proceeding. For dead-code cleanup, Phase 10 must first prove the targeted files are not integration dependencies before any removal is attempted, and final removal must remain contingent on the integration verification results from Phase 11.

The phase pattern is intentionally mixed rather than fully uniform. P03/P04 are split into explicit red/green phases because the value-type dispatch changes alter core dispatch semantics shared by STRING, INT32, BOOLEAN, COLOR, and UNKNOWNRES and therefore carry the highest regression risk. P05–P08 combine TDD + implementation in a single execution phase, followed by a mandatory verification subphase, because each slice is narrower and has a tighter implementation-to-verification loop once the shared dispatch behavior is corrected. P09 remains implementation-only because it is limited to an ABI-width correction and a documentation fix, and P10 is evidence-driven cleanup whose safe execution depends more on dependency proof than on a red/green code cycle.

## Verification Baseline

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

End-to-end: build the full engine with `USE_RUST_RESOURCE` enabled using the authoritative command recorded in Phase 0.5, boot to main menu, verify config save/load round-trip, and exercise the concrete directory-backed resource path/fixture recorded in Phase 0.5 for sentinel validation.
