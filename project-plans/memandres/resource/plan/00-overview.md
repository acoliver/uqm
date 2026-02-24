# Plan: Resource System C → Rust Migration

Plan ID: `PLAN-20260224-RES-SWAP`
Generated: 2026-02-24
Total Phases: 41 (P00.5 through P22, including verification sub-phases)
Requirements: REQ-RES-001 through REQ-RES-115, REQ-RES-R001 through REQ-RES-R015

## Plan History

This plan implements a phased migration of the C resource system to Rust.
The resource system is the most complex of the three subsystems being migrated
(resource, memory, state) due to:
- Broken .rmp parser (TYPE:path format not handled)
- Missing config Put/Save API
- Missing type registration dispatch (C function pointers)
- Missing UIO integration
- Wrong key case handling
- Wrong color parsing format
- 200+ C call sites depending on exact behavioral parity

### What exists and works:
- `ResourceCache` (LRU, thread-safe, eviction) — useful but not part of C API
- `ResourceIndex` — HashMap, merge support, BUT lowercases keys
- `PropertyFile` — parses key=value, BUT uppercases keys, no inline comments
- `ResourceLoader` — filesystem loader, BUT uses std::fs not UIO
- `ResourceSystem` — typed access, BUT format mismatch with C
- `StringBank` — localized strings (different from C stringbank.c arena allocator)
- FFI layer — 15+ exports, BUT different function names from C API
- `rust_resource.c` — C bridge stub, BUT never wired into actual resource pipeline

### What's broken or missing:
- .rmp parser does NOT split `TYPE:path` on `:` (blocking)
- Key casing: C is case-sensitive, Rust lowercases/uppercases (blocking)
- Color parser: Rust does `#RRGGBB`, C does `rgb()`/`rgba()`/`rgb15()` (blocking)
- No `res_PutString`/`res_PutInteger`/`res_PutBoolean`/`res_PutColor`
- No `SaveResourceIndex`
- No `res_DetachResource`
- No `InstallResTypeVectors` that accepts C function pointers
- No `res_GetResource` with lazy loading via C loadFun dispatch
- No `res_FreeResource` with C freeFun dispatch
- No `LoadResourceFromPath`
- No UIO integration (file I/O wrappers)
- No `_cur_resfile_name` global
- FFI function names don't match C API (`rust_*` prefix instead of `res_*`)

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. `unsafe` is explicitly approved for FFI boundary code
6. Type-specific loaders (GFXRES, FONTRES, etc.) STAY IN C
7. UIO stays in C — Rust calls UIO via FFI imports
8. Resource keys are CASE-SENSITIVE — fix existing code

## Slices

| Slice | Name | Phases | Description |
|---|---|---|---|
| A | Analysis & Pseudocode | P01-P02 | Domain analysis, algorithmic design |
| B | .rmp Parser Fix | P03-P05 | Fix TYPE:path splitting, case-sensitive keys |
| C | Color Parser Fix | P06-P08 | Add rgb()/rgba()/rgb15() parsing |
| D | Config API | P09-P11 | Put functions + SaveResourceIndex |
| E | Type Registration | P12-P14 | InstallResTypeVectors with C fn ptrs |
| F | Resource Dispatch | P15-P17 | res_GetResource/FreeResource/DetachResource with lazy loading |
| G | Init & Index | P18-P20 | InitResourceSystem, LoadResourceIndex, UIO wrappers |
| H | C Bridge Wiring | P21 | Wire rust_resource.c, USE_RUST_RESOURCE guards |
| I | Integration | P22 | Full build, game launch, content loading |

## Phase Map

| Phase | Type | Slice | Description |
|---|---|---|---|
| P00.5 | Preflight | — | Toolchain, deps, types, test infra |
| P01 | Analysis | A | Domain model, flow analysis |
| P01a | Verification | A | Analysis verification |
| P02 | Pseudocode | A | Algorithmic pseudocode |
| P02a | Verification | A | Pseudocode verification |
| P03 | Stub | B | .rmp parser TYPE:path + case-sensitive stubs |
| P03a | Verification | B | Stub verification |
| P04 | TDD | B | .rmp parser tests |
| P04a | Verification | B | TDD verification |
| P05 | Impl | B | .rmp parser implementation |
| P05a | Verification | B | Implementation verification |
| P06 | Stub | C | Color parser rgb()/rgba()/rgb15() stubs |
| P06a | Verification | C | Stub verification |
| P07 | TDD | C | Color parser tests |
| P07a | Verification | C | TDD verification |
| P08 | Impl | C | Color parser implementation |
| P08a | Verification | C | Implementation verification |
| P09 | Stub | D | Config Put + SaveResourceIndex stubs |
| P09a | Verification | D | Stub verification |
| P10 | TDD | D | Config API tests |
| P10a | Verification | D | TDD verification |
| P11 | Impl | D | Config API implementation |
| P11a | Verification | D | Implementation verification |
| P12 | Stub | E | Type registration stubs |
| P12a | Verification | E | Stub verification |
| P13 | TDD | E | Type registration tests |
| P13a | Verification | E | TDD verification |
| P14 | Impl | E | Type registration implementation |
| P14a | Verification | E | Implementation verification |
| P15 | Stub | F | Resource dispatch stubs |
| P15a | Verification | F | Stub verification |
| P16 | TDD | F | Resource dispatch tests |
| P16a | Verification | F | TDD verification |
| P17 | Impl | F | Resource dispatch implementation |
| P17a | Verification | F | Implementation verification |
| P18 | Stub | G | Init + index + UIO wrapper stubs |
| P18a | Verification | G | Stub verification |
| P19 | TDD | G | Init + index tests |
| P19a | Verification | G | TDD verification |
| P20 | Impl | G | Init + index + UIO implementation |
| P20a | Verification | G | Implementation verification |
| P21 | Impl | H | C bridge wiring + USE_RUST_RESOURCE guards |
| P21a | Verification | H | Bridge wiring verification |
| P22 | Integration | I | Full integration verification |

## End-State Definition

When `USE_RUST_RESOURCE` is defined:
- All 38 `extern "C"` functions are provided by Rust
- C resource files (`resinit.c`, `getres.c`, `propfile.c`, `loadres.c`,
  `filecntl.c`) are guarded out
- Type-specific loaders remain in C, called from Rust via function pointers
- UIO remains in C, called from Rust via FFI imports
- Game boots identically, config loads/saves correctly, all resources load

## Execution Tracker

| Phase | Status | Verified | Semantic Verified | Notes |
|------:|--------|----------|-------------------|-------|
| P00.5 | ⬜     | ⬜       | N/A               |       |
| P01   | ⬜     | ⬜       | ⬜                |       |
| P01a  | ⬜     | ⬜       | ⬜                |       |
| P02   | ⬜     | ⬜       | ⬜                |       |
| P02a  | ⬜     | ⬜       | ⬜                |       |
| P03   | ⬜     | ⬜       | ⬜                |       |
| P03a  | ⬜     | ⬜       | ⬜                |       |
| P04   | ⬜     | ⬜       | ⬜                |       |
| P04a  | ⬜     | ⬜       | ⬜                |       |
| P05   | ⬜     | ⬜       | ⬜                |       |
| P05a  | ⬜     | ⬜       | ⬜                |       |
| P06   | ⬜     | ⬜       | ⬜                |       |
| P06a  | ⬜     | ⬜       | ⬜                |       |
| P07   | ⬜     | ⬜       | ⬜                |       |
| P07a  | ⬜     | ⬜       | ⬜                |       |
| P08   | ⬜     | ⬜       | ⬜                |       |
| P08a  | ⬜     | ⬜       | ⬜                |       |
| P09   | ⬜     | ⬜       | ⬜                |       |
| P09a  | ⬜     | ⬜       | ⬜                |       |
| P10   | ⬜     | ⬜       | ⬜                |       |
| P10a  | ⬜     | ⬜       | ⬜                |       |
| P11   | ⬜     | ⬜       | ⬜                |       |
| P11a  | ⬜     | ⬜       | ⬜                |       |
| P12   | ⬜     | ⬜       | ⬜                |       |
| P12a  | ⬜     | ⬜       | ⬜                |       |
| P13   | ⬜     | ⬜       | ⬜                |       |
| P13a  | ⬜     | ⬜       | ⬜                |       |
| P14   | ⬜     | ⬜       | ⬜                |       |
| P14a  | ⬜     | ⬜       | ⬜                |       |
| P15   | ⬜     | ⬜       | ⬜                |       |
| P15a  | ⬜     | ⬜       | ⬜                |       |
| P16   | ⬜     | ⬜       | ⬜                |       |
| P16a  | ⬜     | ⬜       | ⬜                |       |
| P17   | ⬜     | ⬜       | ⬜                |       |
| P17a  | ⬜     | ⬜       | ⬜                |       |
| P18   | ⬜     | ⬜       | ⬜                |       |
| P18a  | ⬜     | ⬜       | ⬜                |       |
| P19   | ⬜     | ⬜       | ⬜                |       |
| P19a  | ⬜     | ⬜       | ⬜                |       |
| P20   | ⬜     | ⬜       | ⬜                |       |
| P20a  | ⬜     | ⬜       | ⬜                |       |
| P21   | ⬜     | ⬜       | ⬜                |       |
| P21a  | ⬜     | ⬜       | ⬜                |       |
| P22   | ⬜     | ⬜       | ⬜                |       |
