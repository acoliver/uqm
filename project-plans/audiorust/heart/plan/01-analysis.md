# Phase 01: Analysis

## Phase ID
`PLAN-20260225-AUDIO-HEART.P01`

## Prerequisites
- Required: Phase P00a (Preflight Verification) completed and passed
- Verify: toolchain, dependencies, type/interface checks all green
- Expected files from previous phase: `project-plans/audiorust/heart/plan/00a-preflight-verification.md` (with gate=PASS)

## Requirements Implemented (Expanded)

### REQ-CROSS-GENERAL-07: Module Registration
**Requirement text**: All new modules shall be added to `sound::mod.rs` as `pub mod` declarations and re-export key types.

Behavior contract:
- GIVEN: The existing `rust/src/sound/mod.rs` declares modules for decoder, mixer, formats, etc.
- WHEN: The analysis phase identifies all new module touchpoints
- THEN: A complete entity/state transition model, edge/error handling map, integration touchpoints list, and old-code-to-replace list is produced

Why it matters:
- Understanding the full domain model before coding prevents architectural mistakes

## Implementation Tasks

### Files to create
- `project-plans/audiorust/heart/analysis/domain-model.md` — entity inventory, state transitions, threading model
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P01`
  - marker: `@requirement REQ-CROSS-GENERAL-07`

### Analysis deliverables
1. **Entity/state transition notes** — All types from §2-3 of rust-heart.md mapped with ownership, lifetime, state machines
2. **Edge/error handling map** — Every AudioError variant mapped to its trigger conditions and handlers
3. **Integration touchpoints list** — Module→module and module→existing-code dependency graph
4. **Old code to replace/remove list** — Six C files that get excluded under `USE_RUST_AUDIO_HEART`
5. **Decoder trait gap analysis** — Document the 4 missing APIs (`set_looping`, `decode_all`, `get_time`, `mixer_source_fv`) and resolution strategy

## Verification Commands

```bash
# Structural: verify analysis file exists with substantive content
test -f project-plans/audiorust/heart/analysis/domain-model.md
wc -l project-plans/audiorust/heart/analysis/domain-model.md  # expect > 100 lines
```

## Structural Verification Checklist
- [ ] `analysis/domain-model.md` created with all 5 sections
- [ ] No skipped phases
- [ ] All requirements from spec are represented in entity model

## Semantic Verification Checklist (Mandatory)
- [ ] Every entity from spec §2-3 appears in entity inventory
- [ ] State machines cover all states reachable from the API
- [ ] Error map covers all AudioError variants
- [ ] Integration graph matches spec §1.3 layered architecture
- [ ] Old code list matches spec §6.6 migration path
- [ ] Decoder gaps documented with resolution strategy

## Deferred Implementation Detection (Mandatory)
N/A — analysis phase, no code

## Success Criteria
- [ ] Domain model complete and reviewed
- [ ] All spec entities accounted for
- [ ] Integration touchpoints explicit

## Failure Recovery
- rollback: N/A (documentation only)
- blocking issues: If spec ambiguities found, document in domain-model.md and flag for resolution

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P01.md`
