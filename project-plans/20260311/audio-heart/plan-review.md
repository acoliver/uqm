# AUDIO-HEART Plan Review

Reviewed files:
- All plan files under `project-plans/20260311/audio-heart/plan/`
- `project-plans/20260311/audio-heart/specification.md`
- `project-plans/20260311/audio-heart/requirements.md`
- `project-plans/20260311/audio-heart/initialstate.md`

Overall assessment: the plan is strong, detailed, and mostly aligned with the spec/requirements. I found one SUBSTANTIVE issue and several PEDANTIC issues.

## Findings

### 1. SUBSTANTIVE — Missing explicit implementation/verification phase for the ABI/Cargo feature-coupling requirement

**Why this matters**

The requirements/spec make the build-coupling contract normative, not optional:
- `requirements.md`: “If the build enables the replacement declarations without enabling the corresponding exported implementation symbols, then the build or integration configuration shall fail rather than producing a silently inconsistent binary.”
- `specification.md §18.3`: “The C preprocessor macro `USE_RUST_AUDIO_HEART` and the Cargo feature `audio_heart` must be enabled together.”
- `initialstate.md` identifies this as a live integration risk.

The plan checks this in **P00.5 Preflight Verification**, but that is only an assumption check. There is no later phase that explicitly owns fixing or proving the build-system coupling itself if preflight discovers it is missing or brittle. Since the subsystem cannot link correctly without this contract, omission of an implementation owner here can cause plan execution to fail before functional work even begins.

**Evidence in plan**
- `00a-preflight-verification.md` checks for build-system passage of `--features audio_heart`.
- No later phase explicitly modifies build configuration or adds a final build-coupling verification artifact.
- P12 mentions C build verification broadly, but not this specific feature/macro coupling contract.

**Required fix**
Add an explicit phase task/owner for build-configuration coupling, either:
- an early dedicated phase after P00.5/P01, or
- concrete tasks in P10/P12,

that must:
1. identify the authoritative build path(s),
2. enforce `USE_RUST_AUDIO_HEART` ↔ `--features audio_heart` coupling,
3. fail fast on mismatch,
4. verify the coupled build with concrete project build commands.

**Rating:** SUBSTANTIVE

---

### 2. PEDANTIC — Template compliance is slightly inconsistent around the stated phase count and naming

**Why this matters**

This does not appear execution-blocking, but it is inconsistent.

**Evidence in plan**
- `00-overview.md` says “Total Phase Entries: 24 (P00.5, P00.5a-style verification phases, and P09.5 through P12a)”.
- The actual directory contains 26 plan files if counting overview/execution tracker separately, and the “P00.5a-style verification phases” wording is awkward because there is no actual `P00.5a` file.
- `execution-tracker.md` labels `P08a` as “PLRPause Verification” even though `P08` was broadened well beyond PLRPause.

**Suggested fix**
Tighten the overview/tracker wording so the counted units and titles match the actual phase artifacts exactly.

**Rating:** PEDANTIC

---

### 3. PEDANTIC — Requirement coverage is described well, but the plan never shows the actual final matrix shape/content in a single authoritative artifact

**Why this matters**

The plan clearly requires a matrix, so this is not a missing-coverage problem. It is mostly an auditability/documentation issue.

**Evidence in plan**
- `00-overview.md` establishes a requirement-coverage policy.
- `01-analysis.md` requires a traceability matrix.
- `12-warning-suppression-c-residual.md` requires the matrix to be updated before completion.

However, the plan does not designate one final authoritative file/location for the completed matrix. It references overview, analysis, and P12, which could lead to drift during execution.

**Suggested fix**
Name one canonical artifact/location for the requirement-coverage matrix and have later phases update only that artifact.

**Rating:** PEDANTIC

---

### 4. PEDANTIC — Verification commands remain partially placeholder for full-system/C-side proof

**Why this matters**

The plan already acknowledges project-specific commands in a few places, so this is not fatal. But it weakens verification completeness a bit.

**Evidence in plan**
- P09.5 and P12 still contain comments like “project-specific build/test command here” and “(project-specific build command)”.
- Because several requirements are cross-language/build-integrated, concrete commands would make the plan more executable.

**Suggested fix**
Replace placeholders with the actual repo commands for:
- full C/Rust build under `USE_RUST_AUDIO_HEART`
- any comm-focused integration test/build target
- any runnable end-to-end verification path

**Rating:** PEDANTIC

---

### 5. PEDANTIC — Concrete file paths are strong overall, but a few phase tasks still defer path resolution to “analysis dictates”

**Why this matters**

This is not a blocker because the owning area is narrow and already bounded, but it is slightly softer than the rest of the plan.

**Evidence in plan**
- `08-plrpause-behavioral-fixes.md` says speech-stop work will occur in the owning speech-control module “as analysis dictates.”
- `09.5-comm-handshake-integration-verification.md` references `sc2/src/uqm/comm/* or other exact comm files identified in P09 integration proof`.

Given how concrete the rest of the plan is, these are minor soft spots.

**Suggested fix**
Pin the expected files more concretely once known, or explicitly state the bounded candidate file list in the phase itself.

**Rating:** PEDANTIC

## Coverage Summary Against Requested Review Dimensions

### Gap identification
Strong. The plan captures the initial-state gaps well and expands some of them into better-scoped work items than the initialstate document itself.

### Phase ordering
Good overall. The ordering is sensible: analysis → pseudocode → low-risk constant fix → loader consolidation → multi-track → behavior parity → comm-sensitive pending-completion → control hardening → cleanup/final closure. The only substantive ordering/ownership miss is the build-coupling contract not having an implementation phase.

### REQ coverage
Mostly strong. The plan covers the major requirement areas, including some important contracts beyond the explicit G1–G13 list. The main hole is the lack of an implementation owner for the ABI/Cargo feature-coupling requirement.

### Concrete paths
Generally strong. Most files/functions are concrete and actionable. A few later-phase items still defer exact path selection.

### Template compliance
Mostly compliant, with minor count/title consistency issues.

### Verification adequacy
Good in principle and much stronger than a typical plan, especially around P09.5/P10/P12. Still somewhat weakened by a few placeholder project-command slots and the absence of an explicit build-coupling verification phase.

### Missing phases
One meaningful missing phase/slice: explicit implementation and final verification of build-system coupling for `USE_RUST_AUDIO_HEART` and Cargo feature `audio_heart`.

## Final verdict

- **SUBSTANTIVE findings:** 1
- **PEDANTIC findings:** 4

The plan is close to execution-ready, but it should not be treated as complete until it gains an explicit owner for the build-configuration coupling contract.