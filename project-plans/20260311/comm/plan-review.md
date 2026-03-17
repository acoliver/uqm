# COMM Plan Review

Reviewed files:
- `project-plans/20260311/comm/requirements.md`
- `project-plans/20260311/comm/specification.md`
- all files under `project-plans/20260311/comm/plan/`

Review scope requested:
- gap identification
- phase ordering
- REQ coverage
- concrete paths
- template compliance
- verification adequacy
- missing phases

## Summary

The plan is strong overall: it is explicit about major gaps, has substantially improved ownership boundaries, maps requirements to phases, and includes much better verification than a typical migration plan.

I found **2 findings**:
- **1 SUBSTANTIVE**
- **1 PEDANTIC**

A finding is marked SUBSTANTIVE only when it would likely cause execution failure or a missed requirement.

---

## Finding 1 — SUBSTANTIVE

**Title:** No implementation phase owns the C-side trackplayer wrapper seam that Phase 06 depends on

**Why this matters:**
Phase 06 is written around authoritative trackplayer integration via concrete C wrappers such as:
- `c_SpliceTrack`
- `c_SpliceMultiTrack`
- `c_PlayTrack`
- `c_StopTrack`
- `c_JumpTrack`
- `c_PlayingTrack`
- `c_GetTrackSubtitle`
- `c_GetFirstTrackSubtitle`
- `c_GetNextTrackSubtitle`
- `c_GetTrackSubtitleText`
- `c_FastForward_Page`
- `c_FastForward_Smooth`
- `c_FastReverse_Page`
- `c_FastReverse_Smooth`
- `c_PollPendingTrackCompletion`
- `c_CommitTrackAdvancement`
- `c_ReplayLastPhrase`

But the plan never assigns creation of that wrapper layer to an implementation phase before P06. The only places this seam appears are:
- P00.5 as a preflight/check item
- P06 as an assumption/dependency via extern declarations
- P09/P11 as later consumers/mentions of a seam

P11 does mention adding only `c_PollPendingTrackCompletion` / `c_CommitTrackAdvancement` to `rust_comm.c`, but that is far too late for P06, and it still does not claim ownership of the full wrapper set P06 requires.

As written, Phase 06 cannot be executed as planned unless someone informally expands scope and creates the wrappers ad hoc. That is a plan-execution failure, not just a documentation nit. It directly threatens:
- `TP-REQ-001` through `TP-REQ-013`
- `SS-REQ-001` through `SS-REQ-005`, `SS-REQ-012` through `SS-REQ-017`
- `IN-REQ-001` through `IN-REQ-003`
- callback ordering requirements that depend on pending-completion handoff

**Evidence in plan:**
- `06-track-model-trackplayer.md` defines the C wrapper API as if available, but does not assign creation of those wrappers to a prior phase.
- `00a-preflight-verification.md` only asks to verify availability/semantics.
- `11-c-side-bridge-wiring.md` adds only a partial trackplayer seam and schedules it after P06/P09/P10.

**Required fix:**
Add a concrete implementation slice before or within P06 that explicitly creates the C wrapper layer for all trackplayer APIs P06 relies on, with concrete file ownership, likely in:
- `sc2/src/uqm/rust_comm.c`
- `sc2/src/uqm/rust_comm.h`
- wrapper backing paths in `sc2/src/libs/sound/trackplayer.c`

This can be done either by:
1. expanding P05 or P06 to include wrapper creation and verification, or
2. inserting a dedicated pre-P06 phase for trackplayer bridge wiring.

Without that change, the current phase order is internally inconsistent.

---

## Finding 2 — PEDANTIC

**Title:** The requested `spec/requirements` path is not reflected literally in the plan tree or review inputs

**Why this matters:**
The request said to read all plan files and `spec/requirements`, but the project uses:
- `project-plans/20260311/comm/requirements.md`
- `project-plans/20260311/comm/specification.md`

not a `spec/requirements/` directory. The plan itself is consistent with the actual repo layout, so this does not create an execution failure. Still, if there is an external template expecting a `spec/requirements` directory, the artifact layout is not literally compliant with that naming convention.

**Why this is only PEDANTIC:**
The requirements are present, readable, and comprehensively referenced. Nothing in execution appears blocked by the path shape itself.

**Suggested fix:**
If template conformance requires it, either:
- add a note in `00-overview.md` stating that `requirements.md` and `specification.md` are the authoritative equivalents of `spec/requirements`, or
- align the folder naming with the template in a separate documentation cleanup.

---

## Check-by-check assessment

### 1. Gap identification
Pass with one caveat.

Strengths:
- The plan identifies the major architectural gaps clearly.
- The added gaps around `RaceCommunication()` ownership, phrase-disable audit gating, SIS update ordering, callback ABI, and lock discipline are all real and important.
- Gap severity is mostly calibrated correctly.

Caveat:
- The plan identifies the need for trackplayer integration but misses a separate execution gap: the concrete wrapper seam needed to make that integration implementable in the scheduled phase order. That is the SUBSTANTIVE finding above.

### 2. Phase ordering
Mostly good, but blocked by the SUBSTANTIVE seam issue.

Good ordering examples:
- preflight → analysis → pseudocode before implementation
- LOCDATA before glue and lifecycle
- animation before encounter/main-loop use
- lifecycle before talk/main-loop
- main loop before rendering polish
- C-side broad guard/build integration after Rust-side ownership is established

Ordering problem:
- P06 depends on a C wrapper layer that no earlier phase owns.

### 3. REQ coverage
Strong overall.

Observations:
- `01a-analysis-verification.md` gives near-complete coverage mapping.
- Existing-behavior requirements are treated as proof obligations rather than silently trusted.
- P12 has a meaningful closeout structure and scenario-based validation.
- The phase files generally align with the requirement families they claim.

No additional SUBSTANTIVE REQ orphaning issue found in the reviewed plan set.

### 4. Concrete paths
Good overall.

Strengths:
- Most files use explicit repo-relative or absolute paths.
- Later phases improved concrete seam ownership by naming backing C source files.
- P08/P09/P10/P11 are much better than average on source-path specificity.

Weak spot:
- Trackplayer seam ownership names backing sources, but not an implementation phase that actually creates the wrappers before first use.

### 5. Template compliance
Mostly compliant.

Present:
- overview
- per-phase implementation files
- per-phase verification files
- execution tracker
- prerequisites
- success criteria
- verification checklists
- failure recovery
- phase completion markers

Minor template issue:
- the repository structure does not literally mirror a `spec/requirements` directory naming convention.

### 6. Verification adequacy
Good and generally above threshold.

Strengths:
- Verification is not just unit-test boilerplate; it includes lifecycle ordering, mixed-mode builds, scenario evidence, and reproducibility requirements.
- P12 is appropriately demanding.
- P00.5 and P01a correctly convert risky assumptions into explicit verification gates.

Only caution:
- Verification cannot compensate for a missing implementation-owning phase for the trackplayer wrapper seam.

### 7. Missing phases
Yes: effectively one missing implementation slice.

Missing slice:
- A pre-P06 or in-P06 phase/subphase that creates and verifies the C-side trackplayer bridge wrappers required by the Rust integration layer.

This is the same issue as Finding 1 and is SUBSTANTIVE.

---

## Final assessment

**Overall judgment:** solid plan with one important execution hole.

**Findings:**
1. **SUBSTANTIVE** — missing implementation ownership for the C-side trackplayer wrapper seam required by Phase 06
2. **PEDANTIC** — actual requirements/spec artifact layout does not literally match a `spec/requirements` path convention

**Recommendation:**
Revise the plan before execution to add an explicit pre-P06 trackplayer bridge implementation slice, or expand P06 so it owns creation of the wrapper seam rather than assuming it already exists.