# Resource System: C → Rust Migration Specification

## Purpose

Replace the C resource system (`libs/resource/`) with a Rust implementation
that is fully ABI-compatible with all 200+ existing C call sites. The Rust
system owns the resource index (HashMap), property file parsing, key-value
config API, type handler registry, reference counting, and serialization.
Type-specific binary loaders (GFXRES, FONTRES, SNDRES, etc.) remain in C
and are called by Rust via stored function pointers. File I/O goes through
UIO (C), which Rust accesses via FFI imports.

## Architectural Boundaries

- **Inbound (C → Rust)**: All public functions from `reslib.h` are
  reimplemented as `#[no_mangle] pub extern "C"` in Rust. 38 functions total.
- **Outbound (Rust → C type loaders)**: Rust stores C function pointers
  (`loadFun`, `freeFun`, `toString`) registered via `InstallResTypeVectors`
  and invokes them for heap-type resource loading/freeing.
- **Outbound (Rust → UIO)**: Rust imports `uio_fopen`, `uio_fclose`,
  `uio_fread`, `uio_fwrite`, `uio_fseek`, `uio_ftell`, `uio_getc`,
  `uio_putc`, `uio_stat`, `uio_unlink` via `extern "C"` blocks.
- **Global state**: `contentDir` and `configDir` (`uio_DirHandle*`) are
  imported from C. `_cur_resfile_name` is exported to C.
- **Build flag**: `USE_RUST_RESOURCE` in `config_unix.h` (already defined).

## Data Contracts and Invariants

### ResourceDesc (per entry)
- `fname`: Rust-owned `CString`. For STRING type, `resdata.str` aliases this.
- `vtable`: Pointer to `ResourceHandlers`. NULL for type-registration entries.
- `resdata`: `#[repr(C)]` union matching C `RESOURCE_DATA` (num/ptr/str).
- `refcount`: u32. Incremented by `res_GetResource`, decremented by `res_FreeResource`.

### ResourceHandlers (per type)
- `res_type`: `*const c_char` (static lifetime, points to C string literal).
- `load_fun`, `free_fun`, `to_string`: C function pointers (Option<fn>).

### Key case sensitivity
- Resource keys are **case-sensitive** (matching C `CharHashTable`).
- The existing Rust code lowercases/uppercases keys — this MUST be fixed.

### Two-phase loading
- Value types (`freeFun == NULL`): parsed immediately at index time. No lazy loading.
- Heap types (`freeFun != NULL`): `resdata.ptr = NULL` until first `res_GetResource`.

## Integration Points

| Integration | Direction | Mechanism |
|---|---|---|
| `InitResourceSystem` | C calls Rust | `extern "C"` export |
| `InstallResTypeVectors` | C calls Rust with C fn ptrs | Rust stores fn ptrs |
| `LoadResourceIndex` | C calls Rust | Rust calls UIO to read file |
| `res_GetResource` | C calls Rust | Rust calls C `loadFun` for heap types |
| `res_DetachResource` | C calls Rust | `extern "C"` export |
| `res_FreeResource` | C calls Rust | Rust calls C `freeFun` |
| `res_Get/Put{String,Integer,Boolean,Color}` | C calls Rust | `extern "C"` exports |
| `SaveResourceIndex` | C calls Rust | Rust calls UIO to write file |
| `LoadResourceFromPath` | C calls Rust | Rust calls UIO + C loadFileFun |
| File I/O wrappers | C calls Rust | Rust delegates to UIO |

## Functional Requirements

### Index and Lifecycle
- **REQ-RES-001** through **REQ-RES-013**: See `rust-resource-system.md` §13
- **REQ-RES-088, REQ-RES-089**: Uninit and multi-load support

### Type Registration
- **REQ-RES-014** through **REQ-RES-017**: `InstallResTypeVectors` stores C fn ptrs

### Resource Lookup
- **REQ-RES-018** through **REQ-RES-023**: `TYPE:path` parsing, key override

### Lazy Loading and Refcount
- **REQ-RES-024** through **REQ-RES-046**: Two-phase loading, refcount, detach

### Config API
- **REQ-RES-047** through **REQ-RES-059**: Get/Put for String, Integer, Boolean, Color

### Config Persistence
- **REQ-RES-060** through **REQ-RES-065**: `SaveResourceIndex` serialization

### Color Parsing
- **REQ-RES-066** through **REQ-RES-074**: `rgb()`, `rgba()`, `rgb15()` formats

### Path Resolution
- **REQ-RES-075** through **REQ-RES-081**: UIO delegation, sentinel handling

### Error Handling
- **REQ-RES-082** through **REQ-RES-085**: Auto-init, replace-while-live warnings

### Rust-Specific
- **REQ-RES-R001** through **REQ-RES-R015**: No panics across FFI, NULL validation, interior mutability, `#[repr(C)]` on all FFI types

## Error/Edge Case Expectations

- NULL key → warning + safe default return
- Undefined key → warning + safe default return
- Double free → warning (refcount already 0)
- Detach with refcount > 1 → warning + return NULL
- Poisoned mutex → safe default + error log
- Non-UTF8 C strings → graceful fallback (lossy conversion)
- File not found → silent return (no error for index loading)
- LZ-compressed data prefix → warning + return NULL

## Non-Functional Requirements

- **Reliability**: Game must boot and run identically with Rust resource system.
- **Performance**: Uncontended mutex overhead is acceptable (single-threaded callers).
- **Compatibility**: Config files written by C must be readable by Rust and vice versa.
- **Operability**: `USE_RUST_RESOURCE` flag enables/disables at build time.

## Testability Requirements

- All value-type parsing tested against actual `.rmp`/`.cfg` file content
- Config get/put roundtrip tests for each type
- `SaveResourceIndex` output must be parseable by `LoadResourceIndex`
- Color parsing tested for `rgb()`, `rgba()`, `rgb15()` with hex/decimal/octal
- Key case sensitivity explicitly tested
- Prefix mechanism tested
- Addon override (last-writer-wins) tested

## What Stays in C

- Type-specific binary loaders: `_GetCelData`, `_GetFontData`, `_GetSoundBankData`,
  `_GetMusicData`, `_GetConversationData`, `GetLegacyVideoData`, `GetCodeResData`,
  `_GetStringData`, `_GetBinaryTableData`
- Type-specific free functions: `_ReleaseCelData`, `_ReleaseFontData`, etc.
- Type registration callers: `InstallGraphicResTypes`, `InstallStringTableResType`,
  `InstallAudioResTypes`, `InstallVideoResType`, `InstallCodeResType`
- `Load*Instance` convenience functions and `nameref.h` macros
- UIO virtual filesystem (`libs/uio/`)
- `options.c` path resolution (`prepareContentDir`, `prepareConfigDir`)

## What Gets Replaced by Rust

- `resinit.c`: `InitResourceSystem`, `UninitResourceSystem`, `InstallResTypeVectors`,
  all `res_Get*`/`res_Put*`/`res_Is*`/`res_HasKey`/`res_Remove`, `SaveResourceIndex`,
  `LoadResourceIndex`, `newResourceDesc`, `process_resource_desc`, value-type loaders
  (`UseDescriptorAsRes`, `DescriptorToInt`, `DescriptorToBoolean`, `DescriptorToColor`,
  `ColorToString`, `IntToString`, `BooleanToString`, `RawDescriptor`)
- `getres.c`: `res_GetResource`, `res_FreeResource`, `res_DetachResource`,
  `LoadResourceFromPath`, `lookupResourceDesc`, `loadResourceDesc`
- `propfile.c`: `PropFile_from_string`, `PropFile_from_file`, `PropFile_from_filename`
- `loadres.c`: `GetResourceData`, `FreeResourceData`
- `filecntl.c`: All file I/O wrappers
- `index.h` data structures (reimplemented in Rust with `#[repr(C)]`)


## Plan Review

**Reviewer**: LLxprt Code (claude-opus-4-6)
**Date**: 2026-02-24
**Scope**: Full review of `PLAN-20260224-RES-SWAP` against PLAN.md, PLAN-TEMPLATE.md, RULES.md
**Files reviewed**: specification.md, resource.md (gap analysis), rust-resource-system.md (functional spec), 00-overview through 22-integration-verification (all 46 plan/verification files), domain-model.md, component-001/002/003.md pseudocode

---

### 1. Compliance Scores

| Category | Score | Notes |
|----------|-------|-------|
| **Plan structure matches PLAN.md** | 9/10 | Plan ID, sequential phases, directory structure, verification layers all present. Minor: no `.completed/` directory pre-created. |
| **Phase template compliance (PLAN-TEMPLATE.md)** | 9/10 | All phases have Phase ID, prerequisites, requirements, tasks, verification commands, structural/semantic checklists, success criteria, failure recovery, completion markers. Minor: a few verification phases omit the Deferred Implementation Detection step. |
| **RULES.md compliance** | 10/10 | TDD mandatory (RED→GREEN→REFACTOR), `cargo fmt`/`clippy`/`test` gates on every phase, no `unwrap`/`expect` in prod paths, `unsafe` isolated to FFI boundary, `#[repr(C)]` on all FFI types, typed errors, no placeholder markers in impl phases. |
| **C loaders kept in C** | 10/10 | Explicitly stated in spec §"What Stays in C", P00 overview, P12-P14 (type registration stores C fn ptrs), P15-P17 (dispatch calls C loadFun/freeFun). Nine heap-type loaders remain untouched. |
| **Phasing order correctness** | 9/10 | Strictly sequential: Analysis→Pseudocode→Parser→Color→Config→Types→Dispatch→Init/UIO→Bridge→Integration. Each slice follows Stub→TDD→Impl. One concern noted below (P17 vs P18 dependency). |
| **Integration explicitness** | 9/10 | P21 has detailed Integration Contract (callers, replaced code, user access path). P22 has 7 end-to-end scenarios. Both build modes verified. Minor: no explicit rollback plan for partial integration failure. |
| **Requirements traceability** | 9/10 | All REQ-RES-001 through REQ-RES-R015 are referenced from plan phases. Pseudocode line references present in impl phases. Minor gap noted below. |
| **Gap coverage** | 9/10 | All 7 gaps from resource.md §2 are addressed by dedicated phases. Minor gaps noted below. |
| **.rmp parser** | 10/10 | P03-P05 specifically fix TYPE:path splitting, case-sensitive keys, inline # comments, prefix mechanism, bare-key-at-EOF. Comprehensive test suite (21+ tests). |
| **Color parsing** | 10/10 | P06-P08 add rgb()/rgba()/rgb15() with C integer formats (hex/octal/decimal), clamping, serialization. 21+ tests including roundtrip. Existing #RRGGBB deprecated. |
| **Config API** | 10/10 | P09-P11 add all Put functions, SaveResourceIndex with root filtering and strip_root, auto-creation on put. 22+ tests. |
| **Type registration** | 10/10 | P12-P14 add `#[repr(C)]` types, C function pointer storage, `sys.*` key pattern. Matches C design exactly. |
| **UIO integration** | 9/10 | P18-P20 declare all UIO extern imports, contentDir/configDir globals, sentinel handling. One concern noted below. |
| **Dispatch** | 10/10 | P15-P17 implement res_GetResource (lazy load), res_FreeResource (refcount + freeFun), res_DetachResource (ownership transfer), res_Remove (cleanup), LoadResourceFromPath (_cur_resfile_name). |

**Overall Score: 9.2/10**

---

### 2. Issues Found

#### Issue 2.1 — P17 LoadResourceFromPath Has Deferred UIO Dependency (Medium)

P17 (Resource Dispatch — Implementation) acknowledges that `load_resource_from_path` needs UIO integration but notes "Full UIO integration is in Phase 18." The Deferred Implementation Detection grep exception in P17 (`# Expected: 0 matches (except LoadResourceFromPath UIO note, which is addressed in P18)`) explicitly permits a partial implementation in what is supposed to be a full implementation phase.

**PLAN.md rule violated**: "TODO/HACK/placeholder in implementation phases is a phase failure."

**Recommendation**: Either (a) move `LoadResourceFromPath` to P18-P20 (Init/Index/UIO slice) since it fundamentally depends on UIO, or (b) split P17 into P17a (dispatch logic) and P17b (LoadResourceFromPath) with P17b after P18. The current plan says this function stubs UIO and completes later — that IS a placeholder in an impl phase.

#### Issue 2.2 — Init Registers 5 Types, Spec Says 14 (Medium)

P20 implementation notes say "Init registers 5 value types" (STRING, INT32, BOOLEAN, COLOR, UNKNOWNRES). But REQ-RES-004 and the spec say `InitResourceSystem` shall register **14 types** (5 value + 9 heap). The 9 heap types are registered later by C subsystem code via `InstallResTypeVectors` — this is correct behavior, but the plan's P20 semantic verification says "Init registers 5 value types" which contradicts REQ-RES-004's "exactly 14 resource types."

**Resolution needed**: Clarify whether `InitResourceSystem` registers all 14 (with the 9 heap types having NULL loadFun/freeFun as placeholders until C subsystem code re-registers them), or only registers 5 (matching actual C behavior where the 9 heap types are registered separately). The gap analysis (resource.md §1.1) says C registers 5 built-in + calls subsystem installers separately. The spec (rust-resource-system.md §3.1) says register all 14 in InitResourceSystem. These two sources disagree and the plan follows the gap analysis, not the spec. **The spec should be the authority — the plan should match it.**

#### Issue 2.3 — Missing Explicit REQ Trace for File I/O Wrappers (Low)

REQ-RES-108-110 (GetResourceData/FreeResourceData), REQ-RES-113-115 (file I/O layer details), and REQ-RES-090-095 (CONVERSATION/3DOVID/SHIP special types) are referenced in the spec but no plan phase explicitly lists them in the "Requirements Implemented" section. They are implicitly covered by P18-P20 (File I/O wrappers) and P12-P14 (type registration accepts whatever C registers), but explicit traceability is required by PLAN.md ("All substantial implementation artifacts should include traceability markers").

**Recommendation**: Add explicit REQ references in P18/P20 for REQ-RES-108-115 and note in P12-P14 that REQ-RES-090-095 are satisfied by the C loaders calling through the Rust-stored function pointers.

#### Issue 2.4 — No Coverage Gate Defined (Low)

PLAN.md and PLAN-TEMPLATE.md both reference optional `cargo llvm-cov` coverage gates. The plan's preflight (P00.5) does NOT include `cargo llvm-cov --version` verification, and no phase declares a coverage threshold. Given that this is a critical-path subsystem with 200+ call sites, a coverage gate would be prudent.

**Recommendation**: Either (a) add a coverage gate (e.g., 80% line coverage for `rust/src/resource/`) to the verification commands starting at P05, or (b) explicitly state "Coverage gate deferred — test count and behavioral verification are sufficient." Currently the omission is silent.

#### Issue 2.5 — P21 Lacks TDD/Stub Separation (Low)

Slice H (C Bridge Wiring) has only P21 + P21a (impl + verification). Per PLAN.md, implementation phases should follow Stub→TDD→Impl. However, P21 is purely C-side guard insertion (`#ifndef USE_RUST_RESOURCE`) — it doesn't write new behavior, so Stub/TDD is arguably N/A. This is acceptable but should be explicitly noted.

**Recommendation**: Add a one-line note to P21 explaining why the Stub→TDD→Impl cycle is not applicable (no new Rust behavior written in this phase, only C preprocessor guards).

#### Issue 2.6 — Partial Integration Rollback Strategy (Low)

P21 and P22 list `git checkout` rollback commands for individual files, but there is no plan for partial integration failure — e.g., if P21 builds but P22 reveals behavioral differences. In that scenario, bisecting which of the 38 functions has a bug requires more than `git checkout`.

**Recommendation**: Add a diagnostic strategy section to P22: "If behavioral parity fails, enable logging in all FFI bridge functions, run both C and Rust builds, diff the log output to identify divergent function calls."

#### Issue 2.7 — PropertyFile Deprecation May Break Existing Consumers (Low)

P03 marks `PropertyFile::from_string` as `#[deprecated]`. If any existing Rust code (outside the resource module) calls this, it will trigger clippy warnings. The plan doesn't audit for other callers.

**Recommendation**: Grep for `PropertyFile` usage before deprecating. If external callers exist, provide a migration path or keep the old API as a thin wrapper.

#### Issue 2.8 — `_cur_resfile_name` is `static mut` (Low, Acknowledged)

P20 exposes `_cur_resfile_name` as `#[no_mangle] pub static mut`. This is unsound in multi-threaded contexts and triggers clippy warnings. The plan acknowledges single-threaded access but doesn't address the clippy lint.

**Recommendation**: Use `#[allow(non_upper_case_globals)]` and document the single-threaded invariant. Alternatively, use `AtomicPtr<c_char>` which satisfies Rust's safety model without runtime cost.

---

### 3. Strengths

1. **Exceptionally thorough gap analysis**: The resource.md document is one of the most comprehensive C→Rust gap analyses I've seen. Every behavioral nuance (sentinel values, STRING aliasing, CC5TO8 formula, addon override order, `#RRGGBB` being disabled in C due to comment syntax) is documented.

2. **Correct phasing order**: Parser must come first (broken parser blocks everything), then color (needed by config), then config (needed by type registration), then types (needed by dispatch), then dispatch (needed by init), then init+UIO (needed by bridge), then bridge. This topological ordering is correct.

3. **C loaders correctly kept in C**: The plan explicitly avoids the trap of trying to reimplement binary format parsers (cel, font, sound, etc.) in Rust. The function-pointer dispatch pattern is the right architecture.

4. **Both build modes required**: P21 and P22 verify that `USE_RUST_RESOURCE=0` still builds and works. This is essential for safe rollback.

5. **Behavioral parity testing**: P22 Scenario 7 (compare C mode vs Rust mode behavior) is the gold standard for migration verification.

6. **Pseudocode quality**: All three component pseudocode files have numbered lines, validation points, error handling, and explicit side-effect documentation. Implementation phases reference specific line ranges.

7. **Test design quality**: Tests are behavioral (input→output), not implementation-detail assertions. Roundtrip tests, edge cases, format compatibility tests all present.

8. **Memory ownership clarity**: The spec and plan clearly delineate who allocates and who frees for every data type (Table in rust-resource-system.md §9.4).

---

### 4. Verdict

**APPROVED WITH MINOR REVISIONS**

The plan is well-structured, thorough, and follows PLAN.md/PLAN-TEMPLATE.md/RULES.md with high fidelity. The phasing order is correct, all major gaps from the analysis are addressed, C loaders stay in C, integration is explicit, and requirements are traceable.

**Required before execution**:
- **Fix Issue 2.1** (P17 LoadResourceFromPath deferred UIO) — move to P18-P20 or split the phase
- **Fix Issue 2.2** (Init 5 vs 14 types) — reconcile spec and plan, pick one source of truth

**Recommended before execution**:
- Fix Issues 2.3-2.8 (all low severity — traceability, coverage gate, rollback strategy, deprecation audit, static mut lint)

The plan is executable once the two medium-severity issues are resolved.
