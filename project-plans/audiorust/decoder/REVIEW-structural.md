# Structural Review: AIFF Decoder Plan

**Plan ID:** PLAN-20260225-AIFF-DECODER  
**Review Date:** 2026-02-25  
**Review Type:** Pedantic structural compliance against PLAN.md, PLAN-TEMPLATE.md, RULES.md  
**Fix Rounds Completed:** 3  

---

## Template Reference Summary

### PLAN.md Requirements
- Plan ID format: `PLAN-YYYYMMDD-<FEATURE-SLUG>`
- Sequential phase execution (no skipping)
- Traceability markers (`@plan`, `@requirement`, `@pseudocode`)
- Required directory structure: `specification.md`, `analysis/domain-model.md`, `analysis/pseudocode/*.md`, `plan/*.md`, `.completed/`
- Phase 0: Specification (no timeline)
- Phase 0.5: Preflight verification
- Phase 1: Analysis
- Phase 2: Pseudocode (numbered, algorithmic)
- Implementation cycle: Stub → TDD → Impl per slice
- Integration phase answering 5 questions
- Verification: structural + semantic
- Fraud/failure pattern detection
- Phase completion markers in `.completed/`
- Plan evaluation checklist (gate before execution)

### PLAN-TEMPLATE.md Requirements
- Plan header with ID, date, total phases, requirements, critical reminders
- Per-phase: Phase ID, Prerequisites, Requirements Implemented (GIVEN/WHEN/THEN), Implementation Tasks (files to create/modify with markers), Pseudocode traceability, Verification Commands, Structural Verification Checklist, Semantic Verification Checklist, Deferred Implementation Detection, Success Criteria, Failure Recovery, Phase Completion Marker
- Preflight phase template (toolchain, dependencies, types, test infra, blockers, gate decision)
- Integration contract template (callers, replaced code, user access path, data migration, E2E verification)
- Execution tracker template

### RULES.md Requirements
- TDD mandatory (RED → GREEN → REFACTOR)
- Quality baseline: `cargo fmt`, `cargo clippy`, `cargo test`
- Rust rules: explicit types, Result/Option, no unwrap/expect, no unsafe (except approved)
- Architecture: preserve module boundaries, no `*_v2`/`new_*`
- Testing: behavior-based, not implementation-detail assertions
- Anti-placeholder rule in impl phases
- Persistence rules (not applicable here)
- LLM rules: follow patterns, no speculative abstractions

---

## Directory Structure Compliance

| Required Element | Present | Notes |
|---|---|---|
| `specification.md` | [OK] | Comprehensive, all required sections present |
| `analysis/domain-model.md` | [OK] | Entities, states, errors, integration, data flow |
| `analysis/pseudocode/` | [OK] | Two files: `aiff.md`, `aiff_ffi.md` |
| `plan/00-overview.md` | [OK] | Contains plan header, structure table, execution tracker, integration contract |
| `plan/00a-preflight-verification.md` | [OK] | Full preflight template |
| `plan/01..18 + verification` | [OK] | 36 phase files total (18 phases × 2) |
| `.completed/` | [OK] | Directory exists, empty (execution not started) |

**Verdict:** [OK] PASS — directory structure fully compliant.

---

## Per-File Structural Review

### specification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Purpose/problem statement | [OK] | [OK] | — |
| Architectural boundaries | [OK] | [OK] | Module scope table, boundary rules |
| Data contracts and invariants | [OK] | [OK] | Input/output contracts, 5 invariants |
| Integration points (existing modules) | [OK] | [OK] | 5 Rust-side, 5 C-side |
| Functional requirements (`REQ-*` IDs) | [OK] | [OK] | 84 requirements across 9 categories |
| Error/edge case expectations | [OK] | [OK] | 12 edge cases enumerated |
| Non-functional requirements | [OK] | [OK] | Memory, thread safety, performance, compatibility, safety |
| Testability requirements | [OK] | [OK] | Unit tests (7 categories), FFI tests (4 categories) |
| No implementation timeline | [OK] | [OK] | — |
| Intentional deviations documented | [OK] | [OK] | 4 deviations with justification (bonus, not required) |

**Verdict:** [OK] PASS — fully compliant.

---

### analysis/domain-model.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Entity definitions | [OK] | [OK] | AiffDecoder, CompressionType, CommonChunk, SoundDataHeader, ChunkHeader, AudioFormat, DecoderFormats |
| State transition diagram | [OK] | [OK] | ASCII diagram + state table covering full lifecycle |
| Error handling map | [OK] | [OK] | All operations × error conditions × DecodeError variants × last_error codes |
| Integration touchpoints | [OK] | [OK] | 5 Rust-side, 4 C-side, old code replaced section |
| Data flow diagram | [OK] | [OK] | 6-step C→FFI→Rust→FFI→C flow |
| All 9 REQ categories referenced | [OK] | [OK] | FP, SV, CH, DP, DS, SK, EH, LF, FF all present in error map |

**Verdict:** [OK] PASS — fully compliant.

---

### analysis/pseudocode/aiff.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Numbered algorithmic lines | [OK] | [OK] | Lines 1–378+ |
| Validation points | [OK] | [OK] | All REQ-SV, REQ-CH validations shown with explicit conditions |
| Error handling | [OK] | [OK] | Specific Err variants, last_error assignments, close() on failure |
| Ordering constraints | [OK] | [OK] | parse → validate → extract → metadata → need_swap → predictor init |
| Integration boundaries | [OK] | [OK] | SoundDecoder trait methods, formats dependency |
| Side effects | [OK] | [OK] | State mutations documented (cur_pcm, data_pos, prev_val, etc.) |
| REQ-FP coverage | [OK] | [OK] | All 15 FP requirements have pseudocode lines |
| REQ-SV coverage | [OK] | [OK] | All 13 SV requirements have pseudocode lines |
| REQ-CH coverage | [OK] | [OK] | All 7 CH requirements have pseudocode lines |
| REQ-DP coverage | [OK] | [OK] | All 6 DP requirements with no-inline-swap contract documented |
| REQ-DS coverage | [OK] | [OK] | All 8 DS requirements with SDX2 algorithm |
| REQ-SK coverage | [OK] | [OK] | All 4 SK requirements |
| REQ-EH coverage | [OK] | [OK] | All 6 EH requirements |
| REQ-LF coverage | [OK] | [OK] | All 10 LF requirements |
| Line numbering sequential | [OK] | WARNING: | Minor gap: line 83 missing between 82 and 84 (f80 right-shift branch). Cosmetic only; does not affect traceability. |

**Verdict:** [OK] PASS — fully compliant. Minor cosmetic line numbering gap (non-blocking).

---

### analysis/pseudocode/aiff_ffi.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Numbered algorithmic lines | [OK] | [OK] | Lines 1–193 |
| All 12 vtable functions covered | [OK] | [OK] | GetName, InitModule, TermModule, GetStructSize, GetError, Init, Term, Open, Close, Decode, Seek, GetFrame |
| Null safety checks | [OK] | [OK] | Every function checks null decoder and null rust_decoder |
| Box lifecycle | [OK] | [OK] | Init: Box::new + into_raw; Term: from_raw + drop |
| Format mapping | [OK] | [OK] | Open maps AudioFormat → C format codes via RUST_AIFA_FORMATS Mutex |
| Error-to-return-value conversion | [OK] | [OK] | Ok(n)→n, EndOfFile→0, Err→0 |
| read_uio_file helper | [OK] | [OK] | Lines 6–30 |
| Vtable static definition | [OK] | [OK] | Lines 179–193 |
| Line numbering sequential | [OK] | WARNING: | Lines 69–78 (Term) overlap/restart numbering from lines 69 (Init ends at 73). Cosmetic inconsistency. |

**Verdict:** [OK] PASS — fully compliant. Minor line numbering overlap between Init and Term sections (cosmetic, non-blocking).

---

### plan/00-overview.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Plan ID: `PLAN-YYYYMMDD-FEATURE` | [OK] | [OK] | `PLAN-20260225-AIFF-DECODER` |
| Generated date | [OK] | [OK] | 2026-02-25 |
| Total phases | [OK] | [OK] | 18 (P01–P18, plus P00a preflight) |
| Requirements list | [OK] | [OK] | All 84 REQ IDs listed |
| Critical reminders (4 items) | [OK] | [OK] | Preflight, integration, TDD, gates |
| Plan structure table | [OK] | [OK] | All phases with type, requirements |
| Execution tracker | [OK] | [OK] | All phases with Status/Verified/Semantic columns |
| Integration contract | [OK] | [OK] | Callers, replaced code, user path, module registration, data migration, E2E verification |

**Verdict:** [OK] PASS — fully compliant.

---

### plan/00a-preflight-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | `PLAN-20260225-AIFF-DECODER.P00a` |
| Toolchain verification | [OK] | [OK] | cargo, rustc, clippy; coverage gate noted as N/A |
| Dependency verification | [OK] | [OK] | libc crate, std::io types, no new external crates |
| Type/interface verification | [OK] | [OK] | 10 type/trait existence checks |
| Test infrastructure verification | [OK] | [OK] | Existing tests, pattern verification |
| Call-path feasibility | [OK] | [OK] | 5 feasibility checks |
| Blocking issues section | [OK] | [OK] | — |
| Gate decision | [OK] | [OK] | PASS/FAIL decision |
| Phase completion marker | [OK] | [OK] | `.completed/P00a.md` reference |
| Verification commands | [OK] | [OK] | Concrete bash commands |
| Structural verification checklist | [OK] | [OK] | — |
| Semantic verification checklist | [OK] | [OK] | Deterministic + subjective checks |
| Deferred implementation detection | [OK] | [OK] | N/A noted |
| Success criteria | [OK] | [OK] | — |
| Failure recovery | [OK] | [OK] | — |

**Verdict:** [OK] PASS — fully compliant.

---

### plan/01-analysis.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | `PLAN-20260225-AIFF-DECODER.P01` |
| Prerequisites | [OK] | [OK] | P00a PASS required |
| Requirements Implemented (GIVEN/WHEN/THEN) | [OK] | [OK] | — |
| Implementation Tasks (files to create/modify) | [OK] | [OK] | `domain-model.md` to create, markers specified |
| Pseudocode traceability | N/A | [OK] | Analysis phase — no pseudocode yet |
| Verification Commands | [OK] | [OK] | File existence, requirement category count |
| Structural Verification Checklist | [OK] | [OK] | 5 items |
| Semantic Verification Checklist | [OK] | [OK] | 5 items |
| Deferred Implementation Detection | [OK] | [OK] | N/A noted |
| Success Criteria | [OK] | [OK] | 4 items |
| Failure Recovery | [OK] | [OK] | Rollback + blockers |
| Phase Completion Marker | [OK] | [OK] | `.completed/P01.md` reference |

**Verdict:** [OK] PASS — fully compliant.

---

### plan/01a-analysis-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | `PLAN-20260225-AIFF-DECODER.P01a` |
| Prerequisites | [OK] | [OK] | P01 completed |
| Verification Commands | [OK] | [OK] | File check, section grep, requirement count |
| Structural Verification Checklist | [OK] | [OK] | 5 items |
| Semantic Verification Checklist | [OK] | [OK] | 4 deterministic + 5 subjective checks |
| Deferred Implementation Detection | [OK] | [OK] | N/A noted |
| Success Criteria | [OK] | [OK] | 5 items |
| Failure Recovery | [OK] | [OK] | Return to P01 |
| Phase Completion Marker | [OK] | [OK] | `.completed/P01a.md` reference |

**Verdict:** [OK] PASS — fully compliant.

---

### plan/02-pseudocode.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | `PLAN-20260225-AIFF-DECODER.P02` |
| Prerequisites | [OK] | [OK] | P01 completed, domain-model.md expected |
| Requirements Implemented (GIVEN/WHEN/THEN) | [OK] | [OK] | — |
| Files to create (with markers) | [OK] | [OK] | Two pseudocode files, markers specified |
| Verification Commands | [OK] | [OK] | File existence, numbered line count |
| Structural Verification Checklist | [OK] | [OK] | 4 items covering both files |
| Semantic Verification Checklist | [OK] | [OK] | 11 items — every REQ category checked |
| Deferred Implementation Detection | [OK] | [OK] | N/A noted |
| Success Criteria | [OK] | [OK] | — |
| Failure Recovery | [OK] | [OK] | — |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS — fully compliant.

---

### plan/02a-pseudocode-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Verification Commands | [OK] | [OK] | Numbered line count, algorithm presence grep |
| Structural Verification Checklist | [OK] | [OK] | 5 items |
| Semantic Verification Checklist | [OK] | [OK] | 9 deterministic + 5 subjective checks |
| Deferred Implementation Detection | [OK] | [OK] | N/A noted |
| Success Criteria | [OK] | [OK] | 6 items |
| Failure Recovery | [OK] | [OK] | — |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS — fully compliant.

---

### plan/03-parser-stub.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | `PLAN-20260225-AIFF-DECODER.P03` |
| Prerequisites | [OK] | [OK] | P02 completed |
| Requirements Implemented (GIVEN/WHEN/THEN) | [OK] | [OK] | 5 requirement groups expanded |
| Files to create (with markers) | [OK] | [OK] | `aiff.rs` with detailed content spec |
| Files to modify (with markers) | [OK] | [OK] | `mod.rs` |
| Pseudocode traceability | [OK] | [OK] | Lines 1–19, 339–378, 333–338 |
| Verification Commands | [OK] | [OK] | fmt, clippy, test |
| Structural Verification Checklist | [OK] | [OK] | 7 items |
| Semantic Verification Checklist | [OK] | [OK] | 7 items |
| Deferred Implementation Detection | [OK] | [OK] | grep for FIXME/HACK (todo!() allowed in stub) |
| Success Criteria | [OK] | [OK] | 5 items |
| Failure Recovery | [OK] | [OK] | git checkout rollback |
| Phase Completion Marker | [OK] | [OK] | — |
| `todo!()` usage clarified | [OK] | [OK] | Explicit which methods are implemented vs stubbed |

**Verdict:** [OK] PASS — fully compliant. Excellent level of detail on what is implemented vs stubbed.

---

### plan/03a-parser-stub-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Deterministic checks | [OK] | [OK] | 7 checks including specific constant values |
| Subjective checks | [OK] | [OK] | 5 checks |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/04-parser-tdd.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P03 completed |
| Requirements Implemented (GIVEN/WHEN/THEN) | [OK] | [OK] | 8 requirement groups with explicit contracts |
| Test cases enumerated | [OK] | [OK] | 30 test cases with detailed descriptions |
| Test helper defined | [OK] | [OK] | `build_aiff_file()` |
| f80 test vectors | [OK] | [OK] | 6 known rates + zero + denorm + infinity + NaN + negative — with raw byte encodings and derivation |
| Pseudocode traceability | [OK] | [OK] | Lines 73–238, 32–93, 48–68 |
| Verification Commands | [OK] | [OK] | `--no-run` for RED phase |
| Structural Verification Checklist | [OK] | [OK] | — |
| Semantic Verification Checklist | [OK] | [OK] | 12 items including f80 edge cases |
| Deferred Implementation Detection | [OK] | [OK] | `todo!()` count check |
| Success Criteria | [OK] | [OK] | Tests compile, would fail (RED) |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS — exemplary test case specification with concrete test vectors.

---

### plan/04a-parser-tdd-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Deterministic checks | [OK] | [OK] | 8 checks including f80 edge cases |
| Subjective checks | [OK] | [OK] | 5 checks including "would tests pass with fake impl" |

**Verdict:** [OK] PASS.

---

### plan/05-parser-impl.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P04 completed (failing tests) |
| Requirements Implemented (GIVEN/WHEN/THEN) | [OK] | [OK] | 4 requirement groups |
| Files to modify (with markers) | [OK] | [OK] | Detailed implementation steps |
| Pseudocode traceability | [OK] | [OK] | 6 line-range references |
| Verification Commands | [OK] | [OK] | All tests pass (GREEN) |
| Semantic Verification Checklist | [OK] | [OK] | 14 items including f80 algorithm details |
| Deferred Implementation Detection | [OK] | [OK] | Verify `todo!()` only in decode/seek |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/05a-parser-impl-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Deterministic checks | [OK] | [OK] | 10 checks |
| Subjective checks | [OK] | [OK] | 6 checks including real-file validation suggestion |

**Verdict:** [OK] PASS.

---

### plan/06-pcm-decode-stub.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P05 completed |
| Requirements Implemented (GIVEN/WHEN/THEN) | [OK] | [OK] | Dispatch + EH-6 + stubs |
| Implementation Tasks | [OK] | [OK] | Replace decode() todo, add decode_pcm/decode_sdx2 stubs |
| Pseudocode traceability | [OK] | [OK] | Lines 316–319, 226–249, 250–295 |
| Verification Commands | [OK] | [OK] | Compile + existing tests pass |
| All checklist sections | [OK] | [OK] | — |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/06a-pcm-decode-stub-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/07-pcm-decode-tdd.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P06 completed |
| Requirements Implemented (GIVEN/WHEN/THEN) | [OK] | [OK] | All 6 REQ-DP with explicit contracts |
| Test cases enumerated | [OK] | [OK] | 15 test cases with detailed descriptions |
| No-inline-swap tests | [OK] | [OK] | Tests 11–14 explicitly verify no byte swap (critical correctness requirement) |
| Zero-length buffer test | [OK] | [OK] | Test 15 |
| Pseudocode traceability | [OK] | [OK] | Lines 239–267 |
| All checklist sections | [OK] | [OK] | — |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS — excellent coverage of the no-inline-swap contract.

---

### plan/07a-pcm-decode-tdd-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/08-pcm-decode-impl.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P07 completed |
| Requirements Implemented | [OK] | [OK] | All REQ-DP with endianness contract documented |
| Implementation steps | [OK] | [OK] | 7-step implementation guide |
| Pseudocode traceability | [OK] | [OK] | Lines 239–269 |
| Verification Commands | [OK] | [OK] | GREEN phase — all tests pass |
| Deferred Implementation Detection | [OK] | [OK] | Verify decode_sdx2 + seek still todo |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/08a-pcm-decode-impl-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Deterministic checks | [OK] | [OK] | 7 checks including no-inline-swap verification |
| Subjective checks | [OK] | [OK] | 4 checks |

**Verdict:** [OK] PASS.

---

### plan/09-sdx2-decode-stub.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P08 completed |
| Requirements Implemented | [OK] | [OK] | Dispatch confirmation |
| Implementation Tasks | [OK] | [OK] | Verify existing stub (no new code) |
| Pseudocode traceability | [OK] | [OK] | Lines 270–315 |
| All checklist sections | [OK] | [OK] | — |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS — appropriately minimal for a confirmation-only phase.

---

### plan/09a-sdx2-decode-stub-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/10-sdx2-decode-tdd.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P09 completed |
| Requirements Implemented (GIVEN/WHEN/THEN) | [OK] | [OK] | REQ-DS-1, DS-4, DS-5, DS-7, DS-8 with exact math |
| Test cases enumerated | [OK] | [OK] | 13 test cases |
| Test helper defined | [OK] | [OK] | `build_aifc_sdx2_file()` |
| Golden test vectors section | [OK] | [OK] | Procedural extraction from C decoder documented |
| Pseudocode traceability | [OK] | [OK] | Lines 270–315 |
| All checklist sections | [OK] | [OK] | — |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS — golden test vector strategy is excellent for parity validation.

---

### plan/10a-sdx2-decode-tdd-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Deterministic checks | [OK] | [OK] | 8 checks with exact math values |
| Subjective checks | [OK] | [OK] | 6 checks |

**Verdict:** [OK] PASS.

---

### plan/11-sdx2-decode-impl.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P10 completed |
| Requirements Implemented | [OK] | [OK] | REQ-DS-1..8 (except DS-7 noted elsewhere) |
| Implementation steps | [OK] | [OK] | 6-step algorithm |
| Pseudocode traceability | [OK] | [OK] | Lines 270–315 |
| Semantic Verification Checklist | [OK] | [OK] | 8 items including endianness runtime check |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/11a-sdx2-decode-impl-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Deterministic checks | [OK] | [OK] | 8 checks |
| Subjective checks | [OK] | [OK] | 5 checks including runtime endianness emphasis |

**Verdict:** [OK] PASS.

---

### plan/12-seek-stub.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All required sections | [OK] | [OK] | — |
| Appropriately minimal | [OK] | [OK] | Confirmation-only phase |

**Verdict:** [OK] PASS.

---

### plan/12a-seek-stub-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/13-seek-tdd.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P12 completed |
| Requirements Implemented (GIVEN/WHEN/THEN) | [OK] | [OK] | All 4 REQ-SK with explicit contracts |
| Test cases enumerated | [OK] | [OK] | 10 test cases |
| Decode-after-seek tests | [OK] | [OK] | Both PCM and SDX2 |
| Pseudocode traceability | [OK] | [OK] | Lines 320–332 |
| All checklist sections | [OK] | [OK] | — |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/13a-seek-tdd-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Deterministic checks | [OK] | [OK] | 6 checks |
| Subjective checks | [OK] | [OK] | 5 checks |

**Verdict:** [OK] PASS.

---

### plan/14-seek-impl.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P13 completed |
| Requirements Implemented | [OK] | [OK] | All REQ-SK with errno clarification |
| Implementation steps | [OK] | [OK] | 5-step implementation |
| Pseudocode traceability | [OK] | [OK] | Lines 320–332 |
| Milestone noted | [OK] | [OK] | Zero todo!() in aiff.rs |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/14a-seek-impl-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Zero todo check | [OK] | [OK] | Explicit `grep -c "todo!()"` with expected 0 |
| Milestone marker | [OK] | [OK] | `aiff.rs` feature-complete |

**Verdict:** [OK] PASS.

---

### plan/15-ffi-stub.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P14 completed |
| Requirements Implemented | [OK] | [OK] | REQ-FF-1, FF-2, FF-3, FF-10, FF-11, FF-12 |
| Files to create (with markers) | [OK] | [OK] | `aiff_ffi.rs` with exhaustive content spec |
| Files to modify (with markers) | [OK] | [OK] | `mod.rs` |
| Pseudocode traceability | [OK] | [OK] | Lines 1–5, 6–30, 31–78, 179–193 |
| Implemented vs stubbed distinction | [OK] | [OK] | 10 implemented, 2 stubbed (Open, Decode) |
| All checklist sections | [OK] | [OK] | — |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/15a-ffi-stub-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Init pattern verification | [OK] | [OK] | Checks wav_ffi.rs pattern match |

**Verdict:** [OK] PASS.

---

### plan/16-ffi-tdd.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P15 completed |
| Requirements Implemented (GIVEN/WHEN/THEN) | [OK] | [OK] | REQ-FF-2, FF-10, FF-11, FF-12 |
| Test cases enumerated | [OK] | [OK] | 13 test cases |
| Null pointer tests for all functions | [OK] | [OK] | 8 null pointer test cases |
| Pseudocode traceability | [OK] | [OK] | Lines 31–178 |
| All checklist sections | [OK] | [OK] | — |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/16a-ffi-tdd-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/17-ffi-impl.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P16 completed |
| Requirements Implemented | [OK] | [OK] | REQ-FF-4..9, FF-13..15 |
| Implementation Tasks | [OK] | [OK] | Open (9-step) and Decode (5-step) |
| Pseudocode traceability | [OK] | [OK] | Lines 79–134, 143–156 |
| Milestone noted | [OK] | [OK] | Both files feature-complete |
| All checklist sections | [OK] | [OK] | — |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/17a-ffi-impl-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Zero todo check | [OK] | [OK] | — |
| Box lifecycle safety checks | [OK] | [OK] | Leak + use-after-free in subjective checks |
| Milestone marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

### plan/18-integration.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| Phase ID | [OK] | [OK] | — |
| Prerequisites | [OK] | [OK] | P17 completed |
| Integration questions answered (PLAN.md §5) | | | |
| — Who calls this? | [OK] | [OK] | `sd_decoders[]` table |
| — What old behavior replaced? | [OK] | [OK] | `aifa_DecoderVtbl` under `USE_RUST_AIFF` |
| — How user triggers E2E? | [OK] | [OK] | `.aif` file loaded by game |
| — State/config migration? | [OK] | [OK] | None needed (vtable API identical) |
| — Backward compatibility? | [OK] | [OK] | `#ifdef` conditional, C fallback preserved |
| Files to create | [OK] | [OK] | `rust_aiff.h` with exact content |
| Files to modify (with markers) | [OK] | [OK] | `decoder.c`, `config_unix.h.in`, `build.vars.in` with exact changes |
| Verification Commands | [OK] | [OK] | Rust + C build commands |
| Semantic Verification Checklist | [OK] | [OK] | 9 items including pattern matching |
| Deferred Implementation Detection | [OK] | [OK] | Final check across both Rust files |
| Failure Recovery | [OK] | [OK] | Detailed C-side and Rust-side rollback commands |
| Phase Completion Marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS — all 5 integration questions answered explicitly.

---

### plan/18a-integration-verification.md

| Template Requirement | Present | Compliant | Issue |
|---|---|---|---|
| All verification template sections | [OK] | [OK] | — |
| Final plan evaluation checklist | [OK] | [OK] | 9 items matching PLAN.md gate checklist |
| Completeness checks | [OK] | [OK] | All 84 requirements, patterns, milestones |
| Full rollback commands | [OK] | [OK] | C-side + Rust-side |
| PLAN COMPLETE marker | [OK] | [OK] | — |

**Verdict:** [OK] PASS.

---

## Cross-Cutting Compliance Checks

| PLAN.md Requirement | Compliant | Evidence |
|---|---|---|
| Plan ID format `PLAN-YYYYMMDD-FEATURE` | [OK] | `PLAN-20260225-AIFF-DECODER` |
| Sequential phase execution enforced | [OK] | Every phase has prerequisite referencing previous phase |
| Traceability markers (`@plan`, `@requirement`) | [OK] | Every file-creation/modification task specifies markers |
| Pseudocode line references in impl phases | [OK] | P03, P05, P06, P08, P11, P14, P15, P17 all have line refs |
| Stub → TDD → Impl cycle per slice | [OK] | Parser (P03→P04→P05), PCM (P06→P07→P08), SDX2 (P09→P10→P11), Seek (P12→P13→P14), FFI (P15→P16→P17) |
| Verification phases (structural+semantic) | [OK] | Every phase has both `a` verification and inline checklists |
| Deferred implementation detection | [OK] | Every phase has grep-based detection section |
| Phase completion marker path specified | [OK] | Every phase specifies `.completed/PNN.md` |
| Integration questions answered | [OK] | P18 + overview integration contract |
| Fraud/failure pattern detection | [OK] | grep for TODO/FIXME/HACK in every impl verification |

| RULES.md Requirement | Compliant | Evidence |
|---|---|---|
| TDD mandatory (RED→GREEN→REFACTOR) | [OK] | TDD phases (P04, P07, P10, P13, P16) are RED; impl phases (P05, P08, P11, P14, P17) are GREEN |
| Quality baseline (fmt, clippy, test) | [OK] | Every phase includes all three commands |
| No `unwrap`/`expect` in production | [OK] | Specification mandates Result/Option; FFI phase notes no unwrap pattern |
| No unsafe except in FFI | [OK] | Specification: "No unsafe in aiff.rs. All unsafe isolated to aiff_ffi.rs" |
| Behavior-based tests | [OK] | TDD phases specify GIVEN/WHEN/THEN contracts, check output values not internals |
| Anti-placeholder rule in impl phases | [OK] | Every impl verification phase has grep for forbidden markers |
| No `*_v2`/`new_*` patterns | [OK] | Plan modifies existing modules, creates new files only where needed |

| PLAN-TEMPLATE.md Requirement | Compliant | Evidence |
|---|---|---|
| Plan header with all fields | [OK] | 00-overview.md |
| Per-phase template compliance | [OK] | All 36 phase files checked above |
| Preflight template | [OK] | 00a-preflight-verification.md |
| Integration contract template | [OK] | 00-overview.md integration contract section |
| Execution tracker template | [OK] | 00-overview.md execution tracker |

---

## Issues Found

### Cosmetic (Non-Blocking)

1. **Pseudocode line numbering gap (aiff.md):** Line 83 is missing between lines 82 and 84 in the f80 parsing section. This is cosmetic — all line references in implementation phases use the actual existing line numbers, so traceability is not broken.

2. **Pseudocode line numbering overlap (aiff_ffi.md):** The Init section ends around line 73, but the Term section restarts at line 69. Line numbers 69–73 are used by both Init and Term. Implementation phases reference the correct functions regardless, so traceability is not broken, but could confuse a reader.

### Substantive

**None found.** After 3 fix rounds, the plan is structurally clean.

---

## Summary

| Category | Files | Pass | Warn | Fail |
|---|---|---|---|---|
| Specification | 1 | 1 | 0 | 0 |
| Analysis | 3 | 3 | 0 | 0 |
| Plan phases | 36 | 36 | 0 | 0 |
| Directory structure | 1 | 1 | 0 | 0 |
| Cross-cutting (PLAN.md) | 10 checks | 10 | 0 | 0 |
| Cross-cutting (RULES.md) | 7 checks | 7 | 0 | 0 |
| Cross-cutting (PLAN-TEMPLATE.md) | 5 checks | 5 | 0 | 0 |
| **Total** | **40 files + 22 checks** | **62/62** | **0** | **0** |

**Cosmetic issues:** 2 (pseudocode line numbering — non-blocking)  
**Substantive issues:** 0  

**Overall Verdict:** [OK] **PASS** — Plan is structurally compliant with all three templates. Ready for execution.
