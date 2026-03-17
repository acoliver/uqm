# PLANET-SOLARSYS Plan Review

Reviewed inputs:
- All files in `project-plans/20260311/planet-solarsys/plan/`
- `project-plans/20260311/planet-solarsys/requirements.md`
- `project-plans/20260311/planet-solarsys/specification.md`

## Summary

The plan is generally thorough, traces most requirement families to concrete phases, and does a good job surfacing risky integration areas such as generation-handler semantics, persistence-window legality, and global navigation state. Phase decomposition is mostly sensible.

I found **2 SUBSTANTIVE findings** and **2 PEDANTIC findings**.

---

## Findings

### 1. SUBSTANTIVE — No phase actually ports or wires the name-generation handler despite claiming full generation-function-table coverage

**Why this matters**

The requirements and spec both make name generation part of the generation-function contract:
- `requirements.md`: generation-function table includes handlers for "name generation"
- `specification.md` §9.1 / §9.2: name generation is an override/fallback handler with default behavior and dedicated dispatch semantics
- Plan P03 also defines `REQ-PSS-TYPES-004` to include name generation in the table contract

But no implementation phase after P03 assigns concrete work to port or wire name-generation behavior:
- P10 covers orbit entry/menu only
- P11 `generate.rs` implements default planet/moon/orbital/mineral/energy/life/NPC behavior, but does **not** list default name generation
- P12 FFI wiring lists override/fallback wrappers for `planet/moon/name/orbit-content` only indirectly in one bullet, but there is no corresponding implementation or test task anywhere that exercises naming behavior or verifies per-star name dispatch
- P13 parity suite does not list planet-name parity among acceptance observables

If left as planned, the subsystem can miss a required part of the dispatch contract and fail to preserve system-specific or default planet naming behavior.

**Evidence**
- `requirements.md` generation-function injection contract and dispatch semantics include name generation explicitly.
- `specification.md` §9.1 and §9.2 include name generation as a required override/fallback handler.
- `plan/11-solarsys-lifecycle-navigation.md` generation tasks omit default name generation.
- `plan/13-e2e-integration-parity.md` acceptance/parity sections omit planet-name observables.

**Required fix**

Add explicit implementation and verification work for name-generation dispatch:
- default name-generation behavior in `rust/src/planets/generate.rs`
- FFI wrapper tasks for the name-generation slot in P12 with concrete tests in `ffi_tests.rs`
- parity coverage in P13 for representative dedicated/default systems verifying assigned planet names match baseline

---

### 2. SUBSTANTIVE — Legacy save compatibility is required, but the plan never creates or sources the legacy-save fixture corpus needed to verify it

**Why this matters**

Legacy-save compatibility is a hard requirement, not a nice-to-have:
- `requirements.md` requires baseline save files to load with identical orbital-target and retrieval-state outcomes
- `specification.md` §10.5 and Appendix A.3 require legacy save fixtures covering planet orbit, moon orbit, retrieved-node suppression, and pending planetary-change commit

The plan references these checks in P11 and P13, but it never adds a phase/task to obtain, create, store, or validate the required fixture corpus. Tests are named, but the plan omits the concrete artifact path/work needed to make those tests executable.

Without an explicit fixture-acquisition step, execution can reach verification phases and fail because the mandatory evidence corpus does not exist.

**Evidence**
- `requirements.md` appendix requires legacy save fixtures in four categories.
- `specification.md` Appendix A.3 defines the same minimum corpus.
- `plan/11-solarsys-lifecycle-navigation.md` mentions "fixture from C" / legacy save decoding tests, but no task creates or stages those fixtures.
- `plan/13-e2e-integration-parity.md` requires legacy save fixture verification, but again does not define where fixtures live or when they are captured.

**Required fix**

Add an explicit fixture-corpus task and path, either as:
- a new pre-verification phase, or
- concrete additions to P05/P07/P11/P13

At minimum, specify:
- fixture source method (captured from baseline C build)
- storage path under the project, e.g. `rust/src/planets/tests/fixtures/legacy_saves/...` or equivalent
- required fixture set: planet orbit, moon orbit, retrieved-node suppression, pending planetary-change commit
- commands/harness that consume those fixtures during P11/P13

---

### 3. PEDANTIC — P09.5 downstream-reference update is only partially reflected in the documented phase ordering artifacts

**Why this matters**

The plan correctly adds P09.5/P09.5a as a feasibility spike and updates P10/P11 prerequisites, which is good. But the execution tracker still jumps from P09a to P10 and does not list P09.5/P09.5a at all, even though the overview and directory do.

This is unlikely to break implementation by itself, but it weakens template consistency and can cause tracker drift during execution.

**Evidence**
- `plan/00-overview.md` includes P09.5 and P09.5a.
- `plan/execution-tracker.md` omits both phases.

**Suggested fix**

Add rows for P09.5 and P09.5a to `execution-tracker.md` so the tracker matches the plan overview and actual file set.

---

### 4. PEDANTIC — Some concrete artifact paths are still missing for captured parity data

**Why this matters**

Many phases require "captured from C runtime" fixture data for analysis, surfaces, nodes, and saves, but most of those phases do not specify where those artifacts are stored. This is mostly a documentation/template-compliance issue because the plan still states the needed verification intent.

**Evidence**
- P05 requires C output fixtures for analysis.
- P07 requires C topo fixtures.
- P09 requires C node-population fixtures.
- P11/P13 require legacy save fixtures.
- The plan does not consistently assign concrete on-disk locations for these captured artifacts.

**Suggested fix**

For each fixture-bearing phase, name a concrete path such as:
- `rust/src/planets/tests/fixtures/analysis/...`
- `rust/src/planets/tests/fixtures/surface/...`
- `rust/src/planets/tests/fixtures/nodes/...`
- `rust/src/planets/tests/fixtures/legacy_saves/...`

---

## Check-by-check assessment

### Gap identification
- Good on generation-handler semantics, persistence-window legality, and global navigation access.
- Misses the name-generation implementation/verification gap. **SUBSTANTIVE**.
- Misses explicit legacy-save fixture acquisition/staging. **SUBSTANTIVE**.

### Phase ordering
- Overall ordering is sensible.
- Adding P09.5 before P10/P11 is a strong improvement.
- Tracker omission of P09.5/P09.5a is inconsistent but **PEDANTIC**.

### REQ coverage
- Broad coverage is strong.
- Name-generation contract is not actually carried through implementation/parity phases despite being required. **SUBSTANTIVE**.
- Legacy-save compatibility is referenced but not operationalized into fixture work. **SUBSTANTIVE**.

### Concrete paths
- Source file paths are concrete.
- Fixture/artifact paths are often not. Mostly **PEDANTIC**, except legacy-save fixtures where the missing corpus blocks required verification.

### Template compliance
- Generally strong: prerequisites, requirements, implementation tasks, verification, success criteria, recovery markers.
- Tracker inconsistency with added phases is **PEDANTIC**.

### Verification adequacy
- Strong overall breadth.
- Inadequate for legacy-save compatibility because required fixtures are not planned concretely. **SUBSTANTIVE**.
- Inadequate for name-generation parity because no explicit tests/observables exist. **SUBSTANTIVE**.

### Missing phases
- No obviously missing large implementation phase beyond what is already decomposed.
- A small explicit fixture-corpus phase would improve executability, but this is only SUBSTANTIVE because legacy-save verification otherwise cannot be completed.

---

## Verdict

**Overall review result: NEEDS REVISION**

The plan is close, but the two substantive issues are execution blockers for full requirement satisfaction:
1. name-generation handler contract is not actually implemented/verified in the phase plan
2. legacy-save compatibility verification lacks a concrete fixture-corpus acquisition/staging plan
