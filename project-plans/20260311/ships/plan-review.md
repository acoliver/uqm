# SHIPS Plan Review

Reviewed inputs:
- All files in `project-plans/20260311/ships/plan/`
- `project-plans/20260311/ships/requirements.md`
- `project-plans/20260311/ships/specification.md`

Overall assessment: the plan is strong and materially better than a typical port plan, especially around early ABI freezing, metadata-vs-live-behavior separation, and queue ownership discipline. I found one SUBSTANTIVE issue and several PEDANTIC issues.

## Findings

### 1. SUBSTANTIVE — Plan does not cover the required AI hook requirement in the implementation-phase markers/checklists

**Why this matters**
`requirements.md` requires that AI intelligence hooks be supported and invoked for computer-controlled ships. `specification.md` §5.1 and §6.1 also make the AI intelligence hook part of the behavioral contract and runtime pipeline. If the plan executes exactly as written, AI support is under-specified in the implementation slices and can be missed without failing the phase-specific requirement markers.

**Evidence**
- Canonical requirement exists in `plan/01-analysis.md` as `REQ-AI-HOOK`.
- But the implementation-phase requirement markers do not carry it forward where it is actually implemented:
  - `plan/04-trait-registry.md` marker includes only `REQ-HOOKS-REGISTRATION, REQ-NULL-HOOK-NOOP, REQ-HOOK-CHANGE`, even though the phase defines `intelligence()` on the trait.
  - `plan/08-shared-runtime-pipeline.md` marker includes pipeline/movement/weapon/collision requirements, but not `REQ-AI-HOOK` or `REQ-HOOK-SERIALIZED`, even though the phase explicitly says AI input is invoked in preprocess.
- The phase verification checklists therefore do not explicitly require proof that AI hook registration and invocation satisfy the REQ-level contract.

**Execution risk**
This is SUBSTANTIVE because AI is required behavior, not optional polish. A team following the phase markers/checklists as gates could complete P04/P08 and still fail a requirement in the shipped system.

**Required fix**
- Add `REQ-AI-HOOK` to the Phase 04 and/or Phase 08 requirement markers, with Phase 04 owning registration surface and Phase 08 owning invocation timing.
- Add `REQ-HOOK-SERIALIZED` to the phase that defines runtime invocation guarantees.
- Extend semantic verification checklists to require:
  - AI hook registration per descriptor instance
  - AI hook invocation for computer-controlled ships
  - AI invocation ordering relative to normalization/preprocess/shared logic
  - serialization/no concurrent invocation on the same descriptor instance

---

### 2. PEDANTIC — Requested source path “spec/requirements” is not the actual project layout

**Evidence**
- There is no `project-plans/20260311/ships/spec/` directory.
- The actual files are `project-plans/20260311/ships/requirements.md` and `project-plans/20260311/ships/specification.md`.

**Why this is pedantic**
This is a review-input/path hygiene issue, not a failure of the ships implementation plan itself.

**Suggested fix**
In the overview or a README for this plan set, document the canonical companion docs as:
- `requirements.md`
- `specification.md`

---

### 3. PEDANTIC — Template compliance is mostly good, but phase files are not perfectly uniform in requirement-marker coverage style

**Evidence**
- Core implementation phases generally include `@plan` and `@requirement` markers.
- Race-batch files largely rely on prose requirement sections and per-race `@plan` markers, but they do not show the same level of explicit REQ-marker granularity as the shared-infrastructure phases.
- Verification phases do not consistently restate REQ ownership, relying instead on checklists.

**Why this is pedantic**
The plan remains readable and executable. This inconsistency does not by itself cause execution failure.

**Suggested fix**
For stricter template compliance, add explicit `@requirement` markers to batch files and, if the template expects it, to verification-phase files as well.

---

### 4. PEDANTIC — Concrete file/function paths are strong overall, but some battle-engine coupling items still stop at “identify/document” instead of naming exact concrete symbols

**Evidence**
- The plan is unusually concrete about many files and bridge functions.
- However, a few items remain phrased as analysis obligations rather than final named interfaces, e.g. in `plan/08-shared-runtime-pipeline.md`:
  - “Exact gravity-well / planet influence data path”
  - “Element category/flag mapping used for collision compatibility”
  - “Projectile ownership / damage attribution path”
- Similarly, SIS configuration accessors are required early in `plan/03b-ffi-boundary-ownership.md`, but the plan does not name the exact concrete C functions/fields that will satisfy them.

**Why this is pedantic**
The plan already recognizes these risks and schedules them early. This is incompleteness of naming, not a missing implementation phase.

**Suggested fix**
Replace the remaining analysis-style bullets with explicit symbol/file targets once known, especially for:
- gravity-well/planet state access
- projectile ownership attribution
- SIS module-state reads

---

### 5. PEDANTIC — Verification is strong at the unit/phase level, but could be more explicit about how end-to-end parity will be evidenced for all races

**Evidence**
- The plan has many semantic checklists and mixed C/Rust smoke gates before P15.
- P15 contains scenario-based verification, but not a fully enumerated parity matrix mapping every species to concrete acceptance evidence.

**Why this is pedantic**
The current verification plan is still adequate in principle; it is not obviously missing a required verification phase. The issue is auditability, not likely execution failure.

**Suggested fix**
Add a simple verification matrix keyed by species and requirement-sensitive mechanics (weapon, special, collision override, mutation, writeback relevance, non-melee path).

## Check Summary

### Gap identification
Pass with one important gap: AI-hook requirement traceability is not carried into the implementation-phase markers/checklists strongly enough.

### Phase ordering
Pass. Ordering is generally sound:
- preflight/analysis/pseudocode first
- ABI/ownership frozen before loader/catalog/queue/runtime/lifecycle
- metadata completeness before catalog
- shared infrastructure before race batches
- C wiring late, after behavior exists

### REQ coverage
Mostly pass, but not complete in enforceable phase ownership because `REQ-AI-HOOK` is not propagated into the phases that actually implement and verify it. This is the only SUBSTANTIVE coverage issue I found.

### Concrete paths
Pass overall. The plan names files and many functions concretely. A few coupling surfaces remain analysis-level rather than final-symbol-level, but not enough to be execution-blocking.

### Template compliance
Mostly pass. Markers/checklists are present and better than average, but not perfectly uniform.

### Verification adequacy
Pass overall. Stronger than typical plans due to staged mixed-path smoke tests. Could be improved with a per-race parity matrix, but that is PEDANTIC.

### Missing phases
Pass. I do not see a missing whole phase. The main issue is missing REQ-level enforcement inside existing phases, not absent phase structure.

## Final judgment

**One SUBSTANTIVE finding**
1. Missing enforceable phase-level coverage for `REQ-AI-HOOK` / AI hook invocation contract.

**PEDANTIC findings**
1. Input doc path mismatch (`spec/requirements` vs actual `specification.md` + `requirements.md`)
2. Minor template/marker consistency gaps
3. A few remaining non-finalized concrete symbol paths
4. Verification could be made more audit-friendly with a per-race parity matrix
