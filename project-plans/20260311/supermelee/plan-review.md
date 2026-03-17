# SuperMelee Plan Review

Reviewed files:
- all files under `project-plans/20260311/supermelee/plan/`
- `project-plans/20260311/supermelee/specification.md`
- `project-plans/20260311/supermelee/requirements.md`

## Summary

The revised plan fixes many of the earlier scope and coverage problems in the main execution path: it now includes concrete phases for persistence, setup/menu flow, combatant selection, netplay boundary work, compatibility audit, FFI wiring, and statement-level traceability.

However, the plan set still contains contradictory stale files and stale verification templates from an unrelated battle-engine plan. Those stale artifacts are not cosmetic: several of them define impossible or wrong gate criteria for the actual SuperMelee scope, and some duplicate phase numbers with conflicting content. If executed as written, the plan can fail at verification time or send implementation into out-of-scope battle-engine work.

## Findings

### 1. SUBSTANTIVE — Stale duplicate phase files create conflicting execution instructions for the same phase numbers

**Why this matters:**
The `plan/` directory still contains duplicate numbered phases with different content, including both the intended SuperMelee phases and stale battle-engine phases:
- `06-element-display-list-stub.md` and `06-team-model-persistence.md`
- `06a-element-display-list-stub-verification.md` and `06a-team-model-persistence-verification.md`
- `07-element-display-list-tdd.md` and `07-setup-menu-ship-pick.md`
- `08-element-display-list-impl.md` and `08-combatant-selection-contract.md`
- `09-battle-controls-ai.md` and `09-netplay-boundary.md`
- `10-collision-ship-runtime.md` and `10-compatibility-audit.md`
- `11-battle-engine-transitions.md` and `11-ffi-bridge-c-wiring.md`
- `12-team-model-persistence.md` and `12-requirement-traceability.md`
- `13-setup-menu-ship-pick.md` and `13-e2e-local-integration-verification.md`
- `14-ffi-bridge-c-wiring.md` and `14-e2e-netplay-boundary-verification.md`
- `15-e2e-integration-verification.md` and `15-final-integration-signoff.md`

The overview and execution tracker describe one phase sequence, but the directory still contains another incompatible one. A team following the folder contents instead of only the overview can execute the wrong phase artifacts or attempt to satisfy conflicting gates for the same phase number.

**Evidence:**
- `00-overview.md` says P06 is `Team Model & Persistence` and excludes battle-engine internals.
- `06-element-display-list-stub.md` defines P06 as `Element & Display List — Stub`, explicitly porting battle-engine internals.

**Required fix:**
Remove or archive the stale duplicate plan files, or clearly relocate them outside this plan directory so each phase number has exactly one canonical file pair.

### 2. SUBSTANTIVE — Verification templates still require out-of-scope battle-engine analysis/pseudocode, which would fail the intended plan

**Why this matters:**
The early verification gates still demand analysis and pseudocode coverage for battle-engine internals that the specification explicitly excludes from SuperMelee ownership. If the team follows those gate checklists, the plan will fail verification unless it adds out-of-scope content; if they ignore them, the plan is not template-consistent and the gates are not actionable.

**Evidence:**
- `01a-analysis-verification.md` requires: `State machines cover: Battle Engine, Element Lifecycle, SuperMelee Setup, Team Model, Ship Selection` and `Battle engine state machine accounts for all paths in battle.c's Battle() function`.
- `02a-pseudocode-verification.md` requires components for `Battle Engine`, `Display List`, `Collision`, `AI`, and `Transitions`.
- `specification.md` §§1–2 explicitly exclude generic battle-engine internals, per-ship combat mechanics, and AI.
- `00-overview.md` also says these are out of scope.

**Required fix:**
Rewrite `01a-analysis-verification.md` and `02a-pseudocode-verification.md` to validate only SuperMelee-owned artifacts: setup/menu, team model, persistence, ship pick, combatant selection contract, netplay boundary, compatibility audit inputs, and FFI boundary analysis.

### 3. SUBSTANTIVE — Phase P04/P04a requires intentional failing tests while the plan’s definition of done requires no stubs/TODOs and sequential green verification gates

**Why this matters:**
The plan is structured as strict sequential gates, but P04 and P04a explicitly require tests to fail due to `todo!()` stubs. That creates a process contradiction with the plan’s broader verification framing and can halt execution in environments where phase completion is expected to keep the tree green. This is not just stylistic because the plan repeatedly uses workspace-wide test/clippy/fmt commands as gate criteria.

**Evidence:**
- `execution-tracker.md`: `All phases are strictly sequential. No phase may begin until the prior phase's verification has passed.`
- `04-core-types-tdd.md`: `Tests should FAIL at this point`.
- `04a-core-types-tdd-verification.md`: `Running tests produces failures` and uses grep for `FAILED|panicked` as the pass condition.
- `00-overview.md` Definition of Done requires `No placeholder stubs or TODO markers remain in SuperMelee implementation code`.

A TDD red phase can be valid internally, but here it is defined as a formal plan gate using workspace-wide commands. In practice that can block execution standards and CI-style progress tracking.

**Required fix:**
Either:
1. make P04/P04a explicitly non-gating authoring phases whose success criterion is test presence/compilation only, with real green gates in P05/P05a, or
2. narrow the failing-test expectation so it does not conflict with the plan’s sequential verification model.

### 4. SUBSTANTIVE — Concrete path coverage is incomplete because some verification files still point to wrong artifacts for the canonical plan

**Why this matters:**
The request asked for concrete paths, and a plan fails in execution if verification points reviewers/implementers to the wrong files. Even after the revised canonical phases were added, stale verification files still reference unrelated battle files and modules under `rust/src/supermelee/battle/`, which are not part of the scoped module structure in the overview/execution tracker.

**Evidence:**
- `00-overview.md` canonical module structure contains only `rust/src/supermelee/{types,error,c_bridge,setup/...}`.
- `06a-element-display-list-stub-verification.md`, `07a-element-display-list-tdd-verification.md`, `08a-element-display-list-impl-verification.md`, `09a-battle-controls-ai-verification.md`, `10a-collision-ship-runtime-verification.md`, and `11a-battle-engine-transitions-verification.md` all verify non-canonical paths such as `rust/src/supermelee/battle/element.rs`, `display_list.rs`, `collision.rs`, `controls.rs`, `ai.rs`, `engine.rs`, and `transitions.rs`.

Because these files remain in the same plan directory and phase namespace, the path set is not self-consistent.

**Required fix:**
Delete or move the stale verification files, or mark them unequivocally non-applicable. The remaining plan files should reference only canonical paths from the scoped module structure.

### 5. PEDANTIC — `00-overview.md` says “Read all plan files” style coverage but the tracker and directory still include obsolete artifacts without an explicit archival note

**Why this matters:**
This is mostly a documentation hygiene problem once the substantive duplicate-phase issue is fixed, but right now there is no file telling a reviewer which files are canonical versus superseded.

**Evidence:**
- `execution-tracker.md` presents one clean 15-phase flow.
- `plan/` directory listing contains many extra phase files not mentioned by the tracker.

**Suggested fix:**
Add a short `README` or archival note if you intentionally keep superseded files, though removal is better.

### 6. PEDANTIC — Some verification commands use loose grep-based heuristics instead of explicit artifact/content checks

**Why this matters:**
The overview itself warns that verification should prefer targeted tests/scripts over brittle grep heuristics. Most of the revised plan follows that well, but a few checks still use grep for pass/fail in ways that are fragile.

**Evidence:**
- `04a-core-types-tdd-verification.md` uses `grep -E "FAILED|panicked"`.
- several phases use `grep` to detect placeholder markers.

This is not execution-fatal by itself because the main implementation phases still have stronger checks.

**Suggested fix:**
Where possible, replace grep-based pass conditions with explicit test targets or scripted assertions.

## Check-by-check assessment

### Gap identification
Partially adequate. The mainline plan now covers the previously missing implementation areas, but it does not fully close the meta-gap of stale conflicting files/templates in the same directory.

### Phase ordering
The canonical ordering in `00-overview.md` and `execution-tracker.md` is reasonable for the scoped subsystem. The substantive issue is that duplicate stale phases undermine that ordering in the actual directory.

### REQ coverage
The canonical plan is much improved and appears to cover the requirements at a meaningful level, especially with P06–P14 plus the explicit matrix in P12. The remaining major coverage problem is not missing requirements in the canonical flow, but stale verification artifacts that demand unrelated battle-engine work.

### Concrete paths
Not fully compliant. The canonical path set is concrete and good, but the plan directory still contains conflicting path expectations to non-canonical battle modules.

### Template compliance
Not compliant overall because multiple verification templates remain inherited from the wrong subsystem and contradict the specification/scope.

### Verification adequacy
Mixed. The revised canonical verification phases are mostly adequate, but the stale templates make the total plan set inadequate for execution as a whole.

### Missing phases
No additional substantive implementation phase appears missing from the canonical flow. The real issue is cleanup/normalization of the existing phase set, not adding more implementation phases.

## Final verdict

The review is **SUBSTANTIVE**.

The canonical plan content is largely on the right track, but the plan set as stored is not safely executable because stale duplicate phases and stale verification templates can force out-of-scope work or cause the plan to fail its own gates.