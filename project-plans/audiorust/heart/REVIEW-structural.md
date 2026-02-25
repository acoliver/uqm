# Structural Review — Audio Heart Plan

**Plan ID**: `PLAN-20260225-AUDIO-HEART`
**Review Date**: 2026-02-25
**Reviewer**: LLxprt Code (claude-opus-4-6)
**Templates**: `PLAN.md`, `PLAN-TEMPLATE.md`, `RULES.md`
**Fix Rounds Completed**: 3

---

## Summary

44 plan documents reviewed (22 phases × 2 files each) plus specification, domain model, and 7 pseudocode files. Overall structural compliance is **very high** after 3 fix rounds. The plan is well-organized, follows the required directory structure, uses sequential phases with proper verification, and provides extensive traceability. Issues found are minor and mostly cosmetic.

**Verdict**: PASS with minor observations.

---

## Template Compliance Matrix

### PLAN.md (Autonomous Plan-Creation Guide) Requirements

| Requirement | Status | Notes |
|:---|:---:|:---|
| Plan ID format `PLAN-YYYYMMDD-<SLUG>` | [OK] | `PLAN-20260225-AUDIO-HEART` — correct format |
| Sequential phase execution | [OK] | P00a → P01 → P01a → … → P21 → P21a, strictly sequential, documented in 00-overview.md |
| Traceability markers (`@plan`, `@requirement`) | [OK] | Every phase doc specifies markers for files to create/modify |
| Required directory structure | [OK] | `specification.md`, `analysis/domain-model.md`, `analysis/pseudocode/*.md`, `plan/*.md`, `.completed/` all present |
| Phase 0: Specification | [OK] | `specification.md` covers all 8 required sections |
| Phase 0.5: Preflight verification | [OK] | `00a-preflight-verification.md` covers toolchain, deps, type/interface, call-path, test infra, build system |
| Phase 1: Analysis | [OK] | Domain model produced with entities, state transitions, error map, integration points, old code list |
| Phase 2: Pseudocode (numbered, algorithmic) | [OK] | 7 pseudocode files with numbered lines, validation points, error handling, integration boundaries |
| Implementation cycle: Stub → TDD → Impl per slice | [OK] | 6 slices (types, stream, trackplayer, music+sfx, control+fileinst, FFI) each follow stub→TDD→impl |
| Integration requirements answered | [OK] | P21 explicitly answers: who calls, what replaced, end-to-end path, state migration, backward compat |
| Verification: structural + semantic | [OK] | Every phase has both structural and semantic checklists |
| Deferred implementation detection | [OK] | Every phase has mandatory `grep` commands for TODO/FIXME/HACK/todo!() |
| Phase completion markers | [OK] | Every phase doc specifies `.completed/PNN.md` creation |
| Fraud/failure pattern detection | [OK] | Covered by deferred impl detection + semantic verification in each phase |
| Plan evaluation checklist | [OK] | 00-overview.md serves as gate; Critical Reminders section present |

### PLAN-TEMPLATE.md (Phase Template) Requirements

| Template Element | Status | Notes |
|:---|:---:|:---|
| Plan header (ID, date, phases, requirements) | [OK] | Present in 00-overview.md with all 234 REQ-IDs |
| Critical Reminders section | [OK] | 8 critical reminders listed in 00-overview.md |
| Phase ID format `PLAN-…P[NN]` | [OK] | All phases use correct `PLAN-20260225-AUDIO-HEART.PNN` format |
| Prerequisites (previous phase + expected files) | [OK] | Every phase lists required predecessor and expected files |
| Requirements Implemented (expanded) | [OK] | Every impl/stub phase lists specific REQ-IDs with GIVEN/WHEN/THEN contracts |
| Files to create/modify with markers | [OK] | All phases specify exact files with `@plan` and `@requirement` markers |
| Pseudocode traceability (impl phases) | [OK] | P05, P08, P11, P14, P17, P20 all reference pseudocode line ranges |
| Verification commands | [OK] | Every phase has `cargo fmt`, `cargo clippy`, `cargo test`, and often `build.sh uqm` |
| Structural verification checklist | [OK] | Present in all phases |
| Semantic verification checklist (mandatory) | [OK] | Present in all phases with both deterministic and subjective checks |
| Deferred implementation detection (mandatory) | [OK] | Present in all phases (N/A noted for analysis/pseudocode/TDD phases) |
| Success criteria | [OK] | Present in all phases |
| Failure recovery (rollback + blocking issues) | [OK] | Present in all phases with specific git commands |
| Phase completion marker | [OK] | Present in all phases |
| Preflight verification template (P0.5) | [OK] | `00a-preflight-verification.md` follows template closely |
| Integration contract template | [OK] | P21 includes full integration contract (callers, replaced code, user path, migration, E2E) |
| Execution tracker template | [OK] | Present in 00-overview.md with all 44 phase rows |

### RULES.md Requirements

| Rule | Status | Notes |
|:---|:---:|:---|
| Rule 1: TDD mandatory (RED→GREEN→REFACTOR) | [OK] | Stub→TDD→Impl cycle enforced per slice; P04 explicitly calls out "RED phase" |
| Rule 2: Quality baseline (fmt, clippy, test) | [OK] | All phases include the 3 required cargo commands |
| Rule 3: Rust coding rules (types, errors, safety) | [OK] | Spec mandates Result/Option, no unwrap, unsafe only at FFI; reinforced in each phase |
| Rule 4: Architecture boundaries | [OK] | §2 layered architecture, lock ordering, module boundaries explicit |
| Rule 5: Testing rules (behavior, not internals) | [OK] | Semantic verification checklists explicitly check "tests verify behavior, not internals" |
| Rule 6: Anti-placeholder rule | [OK] | Every phase's deferred detection section; fraud patterns addressed |
| Rule 7: Persistence rules | N/A | Audio subsystem has no persistence |
| Rule 8: Theme/UX rules | N/A | Audio subsystem, not UI |
| Rule 9: LLM-specific rules | [OK] | Phase completion markers require files changed, behavior, tests, validation outputs |
| Rule 10: Verification checklist before merge | [OK] | P21a serves as final merge gate |

---

## Per-File Structural Review

### `specification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Purpose/problem statement | [OK] | §1 clearly states 6 C files to replace |
| Architectural boundaries | [OK] | §2 with module mapping table, layered diagram, key design decisions |
| Data contracts and invariants | [OK] | §3 with AudioError, constants, 15+ struct definitions |
| Integration points | [OK] | §4 covers mixer API, decoder API, C game engine FFI, C build system |
| Functional requirements (REQ-IDs) | [OK] | §5 with 234 requirements across 7 categories |
| Error/edge case expectations | [OK] | §6 lists 10 error/edge cases |
| Non-functional requirements | [OK] | §7 covers thread safety, no panics, no unsafe, memory safety, performance, compatibility |
| Testability requirements | [OK] | §8 covers unit tests, mock mixer/decoder, integration, FFI boundary, thread safety, coverage |
| No implementation timeline | [OK] | No timeline present in specification |
| Memory budget | [OK] | §7a provides detailed memory breakdown per component |
| Section numbering gap | WARNING: | §5 jumps to §6; there is no §5 footer. Minor (content complete, just numbering) |

### `analysis/domain-model.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Entity inventory | [OK] | §1 lists 16+ entities with types, ownership, lifetime |
| Trait entities | [OK] | §1.2 lists StreamCallbacks and SoundDecoder |
| State transition diagrams | [OK] | §2 covers SoundSource (4 states), FadeState (3 states), TrackPlayerState (4 states), FileInstState (3 states) |
| Error handling map | [OK] | §3 with 10 error conditions, propagation paths |
| Panic-free guarantee | [OK] | §3.2 documents parking_lot's non-poisoning behavior |
| Integration touchpoints | [OK] | §4 covers module→module deps, module→existing code deps, C→Rust FFI callers |
| Old code to replace | [OK] | §5 lists 6 C files to exclude and 8 new files to add |
| Threading model | [OK] | §6 covers 2 threads, lock ordering, deferred callback pattern |
| Decoder trait gaps | [OK] | §7 lists 4 action items with resolutions |

### `analysis/pseudocode/stream.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Numbered algorithm lines | [OK] | Lines numbered 01-700+ |
| Plan ID present | [OK] | `PLAN-20260225-AUDIO-HEART` |
| Validation points | [OK] | REQ-* references inline |
| Error handling documented | [OK] | Error returns documented per function |
| Integration boundaries | [OK] | Mixer calls, decoder calls marked |
| Side effects documented | [OK] | Thread spawning, state mutations noted |
| Coverage: 17 algorithms | [OK] | init, uninit, create/destroy sample, play/stop/pause/resume/seek, thread, processing, fade, scope, tags |

### `analysis/pseudocode/trackplayer.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Numbered algorithm lines | [OK] | Lines numbered, 840 total lines |
| Data structures documented | [OK] | SoundChunk, TrackPlayerState structs with field explanations |
| Callback issue fix documented | [OK] | ISSUE-ALG-05: Fn not FnOnce |
| Coverage: 15+ algorithms | [OK] | splice, multi-track, split_sub_pages, timestamps, play/stop/jump, seek, callbacks, subtitles |

### `analysis/pseudocode/music.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Numbered algorithm lines | [OK] | 264 lines |
| Ownership model documented | [OK] | Arc<Mutex<SoundSample>> ownership explained at top |
| Coverage: 10+ algorithms | [OK] | plr_play/stop/playing/seek/pause/resume, speech, load/release, volume, fade |

### `analysis/pseudocode/sfx.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Numbered algorithm lines | [OK] | 256 lines |
| Coverage: 9 algorithms | [OK] | play/stop channel, position, bank load/release |

### `analysis/pseudocode/control.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Numbered algorithm lines | [OK] | 241 lines |
| Coverage: 10 algorithms | [OK] | init, volume, stop/clean, playing, wait |

### `analysis/pseudocode/fileinst.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Numbered algorithm lines | [OK] | 98 lines |
| RAII guard pattern | [OK] | FileLoadGuard with Drop documented |
| Coverage: 4 algorithms | [OK] | guard, load_sound, load_music, destroy delegates |

### `analysis/pseudocode/heart_ffi.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Numbered algorithm lines | [OK] | 488 lines |
| Thread-local cache pattern | [OK] | ISSUE-FFI-01 fix documented |
| Error convention documented | [OK] | bool→1/0, count→0, pointer→null |
| Coverage: 60+ FFI functions | [OK] | Organized by module (stream, track, music, sfx, control, fileinst) |

### `plan/00-overview.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Plan ID | [OK] | `PLAN-20260225-AUDIO-HEART` |
| Generated date | [OK] | 2026-02-25 |
| Total phases declared | [OK] | 22 impl + verification = 44 documents |
| Requirements listed | [OK] | All 234 REQ-IDs enumerated |
| Critical reminders | [OK] | 8 items including lock ordering, parking_lot, unsafe confinement |
| Phase execution order | [OK] | Linear sequence with dependency graph |
| Phase dependency graph | [OK] | ASCII art diagram + dependency table |
| Execution tracker | [OK] | Full table with all 44 phase rows, Status/Verified/Semantic columns |

### `plan/00a-preflight-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P00a` |
| Toolchain verification | [OK] | cargo, rustc, clippy, parking_lot, lazy_static, log |
| Dependency verification | [OK] | Version compatibility checks |
| Type/interface verification | [OK] | SoundDecoder trait, DecodeError, AudioFormat, MixerError, SourceProp, mixer functions — all explicitly listed |
| Gaps to address | [OK] | 4 decoder trait gaps with resolutions |
| Call-path feasibility | [OK] | 10 import path checks |
| Test infrastructure | [OK] | cargo test, NullDecoder check |
| Build system verification | [OK] | build.sh, static lib, no_mangle visibility |
| Gate decision | [OK] | PASS/FAIL checkboxes |

### `plan/01-analysis.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P01` |
| Prerequisites | [OK] | P00a required |
| Requirements expanded (GIVEN/WHEN/THEN) | [OK] | REQ-CROSS-GENERAL-07 expanded |
| Files to create | [OK] | `analysis/domain-model.md` |
| Analysis deliverables listed | [OK] | 5 deliverables |
| Verification commands | [OK] | File existence + line count |
| Structural + semantic checklists | [OK] | Present |
| Deferred detection | [OK] | N/A (analysis, no code) |
| Phase completion marker | [OK] | `.completed/P01.md` |

### `plan/01a-analysis-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P01a` |
| Prerequisites | [OK] | P01 required |
| Deterministic checks | [OK] | Entity count, error variant count, C file count, lock ordering |
| Subjective checks | [OK] | State machine completeness, integration graph accuracy, threading model |
| Phase completion marker | [OK] | `.completed/P01a.md` |

### `plan/02-pseudocode.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P02` |
| Prerequisites | [OK] | P01a required |
| Coverage specified | [OK] | 7 files, algorithm counts per file |
| Files to create | [OK] | 7 pseudocode files listed |
| Verification commands | [OK] | File existence loop |
| Numbered line requirement stated | [OK] | "Each file has numbered algorithm lines" |
| Phase completion marker | [OK] | `.completed/P02.md` |

### `plan/02a-pseudocode-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P02a` |
| Line count thresholds | [OK] | Per-file minimums: stream>200, trackplayer>150, etc. |
| Deterministic function coverage | [OK] | Lists specific functions per module |
| Subjective checks | [OK] | Error handling, mutex acquisition, integration boundaries, side effects, implementability |
| Phase completion marker | [OK] | `.completed/P02a.md` |

### `plan/03-types-stub.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P03` |
| Prerequisites | [OK] | P02a required |
| Requirements expanded | [OK] | 6 REQ groups with GIVEN/WHEN/THEN |
| SoundDecoder trait gap resolution | [OK] | set_looping → SoundSample.looping; decode_all → free fn; get_decoder_time → free fn |
| mixer_source_fv resolution | [OK] | 3 separate mixer_source_f calls |
| Stub contents enumerated | [OK] | 17 items listed |
| Allowed/not-allowed in stubs | [OK] | todo!() allowed; fake success not allowed |
| Verification commands | [OK] | check, fmt, clippy |
| Deferred detection | [OK] | grep for exactly 2 todo!() occurrences |
| Phase completion marker | [OK] | `.completed/P03.md` |

### `plan/03a-types-stub-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P03a` |
| Build verification included | [OK] | `build.sh uqm` added |
| Deterministic checks | [OK] | Import paths, parking_lot check, looping field |
| Subjective checks | [OK] | Variant count, From conversions, constant values |
| Phase completion marker | [OK] | `.completed/P03a.md` |

### `plan/04-types-tdd.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P04` |
| Prerequisites | [OK] | P03a required |
| Requirements expanded | [OK] | Constants, errors, Send+Sync, decoder gaps |
| Test list | [OK] | 13 named tests with behavioral descriptions |
| RED phase acknowledged | [OK] | "This is the RED phase: tests compile and run, but most will FAIL" |
| Verification commands | [OK] | cargo test for specific module |
| Phase completion marker | [OK] | `.completed/P04.md` |

### `plan/04a-types-tdd-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P04a` |
| Specific test names checked | [OK] | 9 specific test names |
| Subjective checks | [OK] | Behavior vs compilation, variant mapping, thread safety pattern |
| Phase completion marker | [OK] | `.completed/P04a.md` |

### `plan/05-types-impl.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P05` |
| Prerequisites | [OK] | P04a required |
| Pseudocode traceability | [OK] | `stream.md` lines 540-585, 95 referenced |
| decode_all implementation detail | [OK] | Two-phase buffer growth strategy documented |
| Anti-placeholder: remove todo!() | [OK] | Explicit instruction to remove all stubs |
| Deferred detection: 0 results required | [OK] | grep must return 0 |
| Phase completion marker | [OK] | `.completed/P05.md` |

### `plan/05a-types-impl-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P05a` |
| Build verification | [OK] | `build.sh uqm` |
| Zero-deferred deterministic check | [OK] | grep -c returns 0 |
| Subjective: decode_all actually decodes | [OK] | Checks loop behavior, NullDecoder, real data |
| Phase completion marker | [OK] | `.completed/P05a.md` |

### `plan/06-stream-stub.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P06` |
| Prerequisites | [OK] | P05a required |
| Requirements expanded | [OK] | 7 REQ groups with behavior contracts |
| Init ordering constraint | [OK] | "CRITICAL: must be called after mixer_init()" |
| Stub contents | [OK] | StreamEngine, lazy_static, 19+ public functions, internal functions |
| Phase completion marker | [OK] | `.completed/P06.md` |

### `plan/06a-stream-stub-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P06a` |
| Function count check | [OK] | >= 19 public functions |
| StreamEngine field check | [OK] | 5 fields enumerated |
| GIVEN/WHEN/THEN contracts | [OK] | 3 contracts testing importability |
| Init ordering documented check | [OK] | Explicit subjective check |
| Phase completion marker | [OK] | `.completed/P06a.md` |

### `plan/07-stream-tdd.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P07` |
| Prerequisites | [OK] | P06a required |
| Test list | [OK] | 29 named tests across 6 categories |
| Tests verify behavior | [OK] | Semantic checklist: "Tests verify behavior, not internals" |
| Phase completion marker | [OK] | `.completed/P07.md` |

### `plan/07a-stream-tdd-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P07a` |
| Test count >= 29 | [OK] | Deterministic check |
| Specific test names per category | [OK] | 7 categories checked |
| "No trivially-passing tests" check | [OK] | Subjective check present |
| Phase completion marker | [OK] | `.completed/P07a.md` |

### `plan/08-stream-impl.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P08` |
| Prerequisites | [OK] | P07a required |
| 75 requirements enumerated | [OK] | All STREAM-* grouped by category |
| Pseudocode traceability | [OK] | 16 function→line-range mappings |
| Implementation priority order | [OK] | 8-step order from simplest to most complex |
| Lock ordering rule | [OK] | CRITICAL note with full hierarchy and deferred callback pattern |
| parking_lot::Condvar note | [OK] | No spurious wakeups documented |
| Deferred detection: 0 results | [OK] | grep must return 0 |
| Phase completion marker | [OK] | `.completed/P08.md` |

### `plan/08a-stream-impl-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P08a` |
| Concurrency verification | [OK] | 4 concurrency tests: N-thread stress, deferred callback verification, deadlock detection, rapid play/stop |
| TOCTOU stress test | [OK] | ISSUE-VER-01 fix: 100+ iteration race window test |
| Lock ordering in code check | [OK] | grep for "lock ordering" |
| Phase completion marker | [OK] | `.completed/P08a.md` |

### `plan/09-trackplayer-stub.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P09` |
| Prerequisites | [OK] | P08a required |
| Requirements expanded | [OK] | 6 REQ groups |
| Lifetime safety documentation | [OK] | 4 safety invariants for `unsafe impl Send` |
| Phase completion marker | [OK] | `.completed/P09.md` |

### `plan/09a-trackplayer-stub-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P09a` |
| Safety doc check | [OK] | "unsafe impl Send present with detailed SAFETY documentation" |
| GIVEN/WHEN/THEN contracts | [OK] | 3 importability contracts |
| Phase completion marker | [OK] | `.completed/P09a.md` |

### `plan/10-trackplayer-tdd.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P10` |
| Prerequisites | [OK] | P09a required |
| Test list | [OK] | 25 named tests across 8 categories |
| Phase completion marker | [OK] | `.completed/P10.md` |

### `plan/10a-trackplayer-tdd-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P10a` |
| Test count >= 25 | [OK] | Deterministic check |
| Per-category test names | [OK] | 5 categories checked |
| Phase completion marker | [OK] | `.completed/P10a.md` |

### `plan/11-trackplayer-impl.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P11` |
| Prerequisites | [OK] | P10a required |
| 57 requirements enumerated | [OK] | All TRACK-* grouped by category |
| Pseudocode traceability | [OK] | 10 function→line-range mappings |
| Iterative Drop for SoundChunk | [OK] | Code sample, stack overflow prevention documented |
| Deferred detection: 0 results | [OK] | grep must return 0 |
| Phase completion marker | [OK] | `.completed/P11.md` |

### `plan/11a-trackplayer-impl-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P11a` |
| Iterative Drop check | [OK] | grep for "impl Drop for SoundChunk" |
| Concurrency verification | [OK] | 3 concurrency tests |
| Phase completion marker | [OK] | `.completed/P11a.md` |

### `plan/12-music-sfx-stub.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P12` |
| Prerequisites | [OK] | P11a required |
| Requirements expanded | [OK] | 10 REQ groups for music + sfx |
| 3D positioning resolution | [OK] | 3 separate mixer_source_f calls documented |
| Two files created | [OK] | music.rs and sfx.rs |
| Phase completion marker | [OK] | `.completed/P12.md` |

### `plan/12a-music-sfx-stub-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P12a` |
| Function counts per file | [OK] | music >= 12, sfx >= 10 |
| 3D positioning doc check | [OK] | Subjective check present |
| GIVEN/WHEN/THEN contracts | [OK] | 3 importability contracts |
| Phase completion marker | [OK] | `.completed/P12a.md` |

### `plan/13-music-sfx-tdd.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P13` |
| Prerequisites | [OK] | P12a required |
| Test list | [OK] | 25 named tests (12 music + 13 sfx) |
| Phase completion marker | [OK] | `.completed/P13.md` |

### `plan/13a-music-sfx-tdd-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P13a` |
| Test counts per module | [OK] | music >= 12, sfx >= 15 |
| Specific test names | [OK] | 5 music + 6 sfx test names |
| Phase completion marker | [OK] | `.completed/P13a.md` |

### `plan/14-music-sfx-impl.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P14` |
| Prerequisites | [OK] | P13a required |
| 47 requirements enumerated | [OK] | All MUSIC-* and SFX-* |
| Pseudocode traceability | [OK] | 12 function→line-range mappings |
| Fade replacement behavior | [OK] | Detailed explanation of mid-fade replacement |
| C resource system integration | [OK] | 4-step data flow: registration, data flow, release flow, raw bytes |
| Deferred detection: 0 results | [OK] | grep must return 0 |
| Phase completion marker | [OK] | `.completed/P14.md` |

### `plan/14a-music-sfx-impl-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P14a` |
| 3D positioning check | [OK] | "uses 3 separate mixer_source_f calls — NOT mixer_source_fv" |
| Phase completion marker | [OK] | `.completed/P14a.md` |

### `plan/15-control-fileinst-stub.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P15` |
| Prerequisites | [OK] | P14a required |
| Requirements expanded | [OK] | 5 REQ groups |
| Circular dependency resolution noted | [OK] | SOURCES pub(crate) visibility discussed |
| Two files created | [OK] | control.rs and fileinst.rs |
| Phase completion marker | [OK] | `.completed/P15.md` |

### `plan/15a-control-fileinst-stub-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P15a` |
| Function counts | [OK] | control >= 10, fileinst >= 5 |
| RAII guard pattern check | [OK] | Subjective check present |
| GIVEN/WHEN/THEN contracts | [OK] | 3 contracts |
| Phase completion marker | [OK] | `.completed/P15a.md` |

### `plan/16-control-fileinst-tdd.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P16` |
| Prerequisites | [OK] | P15a required |
| Test list | [OK] | 15 named tests (9 control + 6 fileinst) |
| Phase completion marker | [OK] | `.completed/P16.md` |

### `plan/16a-control-fileinst-tdd-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P16a` |
| Test counts | [OK] | control >= 12, fileinst >= 8 |
| Specific test names | [OK] | 6 control + 4 fileinst |
| Phase completion marker | [OK] | `.completed/P16a.md` |

### `plan/17-control-fileinst-impl.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P17` |
| Prerequisites | [OK] | P16a required |
| 24 requirements enumerated | [OK] | All VOLUME-* and FILEINST-* |
| Pseudocode traceability | [OK] | 13 function→line-range mappings |
| WaitForSoundEnd polling detail | [OK] | 5-point detailed explanation including 10ms granularity match |
| Deferred detection: 0 results | [OK] | grep must return 0 |
| Phase completion marker | [OK] | `.completed/P17.md` |

### `plan/17a-control-fileinst-impl-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P17a` |
| RAII cleanup check | [OK] | "guard clear cur_resfile_name even if load panics/errors" |
| Phase completion marker | [OK] | `.completed/P17a.md` |

### `plan/18-ffi-stub.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P18` |
| Prerequisites | [OK] | P17a required |
| Requirements expanded | [OK] | 6 REQ groups for FFI |
| 60+ functions categorized | [OK] | Stream (18), Track (14), Music (12), SFX (8), Control (6), File (2) = 60 |
| CCallbackWrapper | [OK] | Struct + convert helper |
| Phase completion marker | [OK] | `.completed/P18.md` |

### `plan/18a-ffi-stub-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P18a` |
| 60+ no_mangle count | [OK] | Deterministic check |
| extern "C" count | [OK] | Deterministic check |
| Symbol verification with `nm` | [OK] | GIVEN/WHEN/THEN for nm |
| Phase completion marker | [OK] | `.completed/P18a.md` |

### `plan/19-ffi-tdd.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P19` |
| Prerequisites | [OK] | P18a required |
| Test list | [OK] | 17 named tests across 4 categories (null, error, string, callback) |
| Phase completion marker | [OK] | `.completed/P19.md` |

### `plan/19a-ffi-tdd-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P19a` |
| Test count >= 15 | [OK] | Deterministic check |
| UTF-16 conversion check | [OK] | Subjective check for SpliceTrack |
| C-compatible types check | [OK] | "Tests use C-compatible types (c_int, *mut c_void) not Rust-native" |
| Phase completion marker | [OK] | `.completed/P19a.md` |

### `plan/20-ffi-impl.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P20` |
| Prerequisites | [OK] | P19a required |
| Pseudocode traceability | [OK] | 7 section→line-range mappings |
| PlayChannel handle resolution | [OK] | Technical Review Issue #6 documented with full lifecycle |
| GraphForegroundStream rendering path | [OK] | 5-step data flow, no shared ring buffer pointer |
| Safety documentation requirement | [OK] | Every unsafe block must have // SAFETY: comment |
| Symbol verification command | [OK] | `nm` check included |
| Deferred detection: 0 results | [OK] | grep must return 0 |
| Phase completion marker | [OK] | `.completed/P20.md` |

### `plan/20a-ffi-impl-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P20a` |
| SAFETY comment count | [OK] | >= 10 required |
| PlayChannel handle resolution check | [OK] | Technical Review Issue #6 |
| Box::into_raw/from_raw pairing check | [OK] | "no double-free or use-after-free" |
| Phase completion marker | [OK] | `.completed/P20a.md` |

### `plan/21-integration.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P21` |
| Prerequisites | [OK] | P20a required |
| Integration contract (5 questions) | [OK] | Existing callers, replaced code, user path, data migration, E2E verification — all answered |
| C header creation | [OK] | `audio_heart_rust.h` with include guards |
| Build system modification | [OK] | Conditional exclusion with `USE_RUST_AUDIO_HEART`; mandatory first-step discovery commands |
| Duplicate symbol prevention | [OK] | Explicit warning and `nm` verification |
| All 6 header guards | [OK] | Pattern documented for music.h, sfx.h, sound.h, stream.h, trackplayer.h, fileinst.h |
| Module registration verification | [OK] | All 7 modules listed |
| End-to-end manual verification | [OK] | 9 steps: build, launch, music, SFX, speech, volume, fade, seeking, regression |
| Backward compatibility build | [OK] | Build without flag regression check |
| Deferred detection across ALL files | [OK] | Combined grep across all 8 .rs files |
| Phase completion marker | [OK] | `.completed/P21.md` with detailed contents specification |

### `plan/21a-integration-verification.md`

| Check | Status | Detail |
|:---|:---:|:---|
| Phase ID | [OK] | `PLAN-20260225-AUDIO-HEART.P21a` |
| Backward compatibility verification | [OK] | 6 deterministic checks for disabled flag |
| End-to-end semantic checks | [OK] | 9 subjective scenario checks |
| Performance sanity check | [OK] | decode_all throughput + no memory growth |
| Combined deferred detection | [OK] | All 8 files in single grep |
| Plan completion declaration | [OK] | "When this phase passes, the entire PLAN is complete" |
| Phase completion marker | [OK] | `.completed/P21a.md` |

---

## Cross-Cutting Observations

### Strengths

1. **Exceptional traceability**: 234 requirements systematically mapped through specification → domain model → pseudocode → stub → test → implementation → verification. Every phase references specific REQ-IDs.

2. **Lock ordering thoroughly documented**: The 5-level hierarchy (TRACK_STATE → MUSIC_STATE → Source → Sample → FadeState) is stated in the specification, domain model, overview, and reinforced in stream/trackplayer phases. The deferred callback pattern is well-explained.

3. **Technical review issues addressed**: Issues ISSUE-ALG-05 (Fn vs FnOnce), ISSUE-FFI-01 (thread-local caches), ISSUE-MISC-02 (iterative Drop), ISSUE-VER-01 (TOCTOU stress test), Technical Review Issues #6 (PlayChannel handle) and #8 (TrackPlayerState safety) are all explicitly documented in the relevant phases.

4. **Verification depth**: Both deterministic checks (grep counts, test counts, symbol verification with `nm`) and subjective checks (behavioral correctness, race conditions) in every verification phase.

5. **Backward compatibility**: P21/P21a explicitly test both WITH and WITHOUT `USE_RUST_AUDIO_HEART`, ensuring no regression.

6. **Memory budget**: Specification §7a provides concrete memory estimates per component — unusual level of detail for a plan.

### Minor Observations (Non-Blocking)

| # | File | Observation | Severity |
|:-:|:---|:---|:---:|
| 1 | `specification.md` | Section numbering skips from §5 to §6 (no visible §5 footer before §6 header). Content is complete; numbering artifact only. | Cosmetic |
| 2 | `00-overview.md` | Phase count states "22 implementation phases (P00a through P21)" — there are actually 12 "main" phases (P00a, P01…P21) plus 11 verification phases (P01a…P21a) = 23 distinct phase IDs, not 22. The "44 phase documents" count is correct though. | Cosmetic |
| 3 | `13-music-sfx-tdd.md` | States "All MUSIC-* requirements (21) and SFX-* requirements (26)" in one sentence, then the total test count is only 25 — but P13a expects music>=12 and sfx>=15 = 27. The P13 test list enumerates 25 tests. Minor discrepancy in test count between phases. | Minor |
| 4 | `16-control-fileinst-tdd.md` | States "15+ total tests" but P16a expects control>=12 and fileinst>=8 = 20. The P16 test list enumerates 15 tests. The verification phase expects more tests than the TDD phase enumerates. | Minor |
| 5 | `12-music-sfx-stub.md` | MusicRef described as "wrapper around `Arc<Mutex<SoundSample>>`" but also referenced in `03-types-stub.md` as `#[repr(transparent)]` raw pointer wrapper. The pseudocode (music.md) clarifies it's Arc-based. The stub phase should note this is defined in types.rs (not music.rs), or that music.rs re-exports it. | Minor |
| 6 | `plan/08-stream-impl.md` | States "75 total" STREAM-* requirements but the spec enumerates STREAM-INIT(7) + STREAM-PLAY(20) + STREAM-THREAD(8) + STREAM-PROCESS(16) + STREAM-SAMPLE(5) + STREAM-TAG(3) + STREAM-SCOPE(11) + STREAM-FADE(5) = 75. Count verified correct. | N/A |
| 7 | Various verification phases | Some verification phases list different minimum test counts than the TDD phase specifies. E.g., P07 lists 29 tests, P07a checks >=29; P16 lists 15 tests, P16a checks >=20. The verification phases generally ask for MORE tests, which is fine — implies tests may be added during implementation. | Info |
| 8 | `plan/18-ffi-stub.md` | Track Player FFI lists 14 functions but also lists `FastReverse_Smooth`, `FastForward_Smooth`, `FastReverse_Page`, `FastForward_Page` separately = 17 functions total. The category header says 14 but more are listed. | Cosmetic |

### Coverage Gap Check

| Spec Requirement Category | Plan Coverage | Missing |
|:---|:---:|:---|
| STREAM-INIT (7) | [OK] P06-P08 | — |
| STREAM-PLAY (20) | [OK] P06-P08 | — |
| STREAM-THREAD (8) | [OK] P06-P08 | — |
| STREAM-PROCESS (16) | [OK] P06-P08 | — |
| STREAM-SAMPLE (5) | [OK] P06-P08 | — |
| STREAM-TAG (3) | [OK] P06-P08 | — |
| STREAM-SCOPE (11) | [OK] P06-P08 | — |
| STREAM-FADE (5) | [OK] P06-P08 | — |
| TRACK-ASSEMBLE (19) | [OK] P09-P11 | — |
| TRACK-PLAY (10) | [OK] P09-P11 | — |
| TRACK-SEEK (13) | [OK] P09-P11 | — |
| TRACK-CALLBACK (9) | [OK] P09-P11 | — |
| TRACK-SUBTITLE (4) | [OK] P09-P11 | — |
| TRACK-POSITION (2) | [OK] P09-P11 | — |
| MUSIC-PLAY (8) | [OK] P12-P14 | — |
| MUSIC-SPEECH (2) | [OK] P12-P14 | — |
| MUSIC-LOAD (6) | [OK] P12-P14 | — |
| MUSIC-RELEASE (4) | [OK] P12-P14 | — |
| MUSIC-VOLUME (1) | [OK] P12-P14 | — |
| SFX-PLAY (9) | [OK] P12-P14 | — |
| SFX-POSITION (5) | [OK] P12-P14 | — |
| SFX-VOLUME (1) | [OK] P12-P14 | — |
| SFX-LOAD (7) | [OK] P12-P14 | — |
| SFX-RELEASE (4) | [OK] P12-P14 | — |
| VOLUME-INIT (5) | [OK] P15-P17 | — |
| VOLUME-CONTROL (5) | [OK] P15-P17 | — |
| VOLUME-SOURCE (4) | [OK] P15-P17 | — |
| VOLUME-QUERY (3) | [OK] P15-P17 | — |
| FILEINST-LOAD (7) | [OK] P15-P17 | — |
| CROSS-THREAD (4) | [OK] Throughout | — |
| CROSS-MEMORY (4) | [OK] Throughout | — |
| CROSS-CONST (8) | [OK] P03-P05 | — |
| CROSS-FFI (4) | [OK] P18-P20 | — |
| CROSS-ERROR (3) | [OK] Throughout | — |
| CROSS-GENERAL (8+) | [OK] P03, P18-P21 | — |

**All 234 requirements have plan phase coverage.** No gaps found.

---

## Final Verdict

**PASS** — The plan is structurally sound and ready for execution. All template requirements are met. The 8 minor observations are cosmetic or informational and do not affect executability. The plan exceeds template requirements in several areas (memory budget, Technical Review issue tracking, concurrency verification detail, backward compatibility testing).
