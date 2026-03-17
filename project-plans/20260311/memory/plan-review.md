# MEMORY Plan Review

Reviewed files:
- `project-plans/20260311/memory/plan/00-overview.md`
- `project-plans/20260311/memory/plan/00a-preflight-verification.md`
- `project-plans/20260311/memory/plan/01-analysis.md`
- `project-plans/20260311/memory/plan/01a-analysis-verification.md`
- `project-plans/20260311/memory/plan/02-pseudocode.md`
- `project-plans/20260311/memory/plan/02a-pseudocode-verification.md`
- `project-plans/20260311/memory/plan/03-zero-size-oom-and-init-tdd.md`
- `project-plans/20260311/memory/plan/03a-zero-size-oom-and-init-verification.md`
- `project-plans/20260311/memory/plan/04-copy-argv-fix-tdd.md`
- `project-plans/20260311/memory/plan/04a-copy-argv-fix-verification.md`
- `project-plans/20260311/memory/plan/05-integration-tests-and-markers.md`
- `project-plans/20260311/memory/plan/05a-integration-tests-verification.md`
- `project-plans/20260311/memory/specification.md`
- `project-plans/20260311/memory/requirements.md`

## Summary
The plan is generally well-structured, concrete, and appropriately scoped as gap-closure rather than subsystem reimplementation. It has one substantive planning defect around requirement closure/verification for mixed-language coverage, plus several pedantic issues around naming, artifact references, and template consistency.

## Findings

### 1. SUBSTANTIVE — REQ-MEM-INT-009 is marked as only partial, but the plan never creates the required downstream artifact it treats as mandatory
**Why this matters:** The plan correctly states that true mixed-language seam coverage is project-level work and outside module-local closure. However, Phase P05 and P05a require a "concrete downstream tracking artifact" with path, owner, and acceptance criteria, yet no phase actually creates or updates any artifact outside the phase markdown itself. `00-overview.md` says the handoff artifact is to update phase docs; `05-integration-tests-and-markers.md` says to modify `05a-integration-tests-verification.md`; `05a-integration-tests-verification.md` then only verifies that some downstream artifact exists. This is circular. If executed literally, the plan can finish without producing the project-level handoff it claims is mandatory, leaving REQ-MEM-INT-009 residual work undocumented and undispatchable.
**Evidence:**
- `00-overview.md`: "Required handoff artifact ... Update ... 05-integration-tests-and-markers.md and 05a..."
- `05-integration-tests-and-markers.md`: says residual work must be recorded in a concrete downstream tracking artifact, but only lists a modification to `05a-integration-tests-verification.md`
- `05a-integration-tests-verification.md`: verifies existence of a downstream artifact, but no prior phase creates one at a concrete path
**Recommended fix:** Add an explicit deliverable phase task that creates a concrete file at a specific path, such as a follow-up plan or tracker document under `project-plans/20260311/memory/` (or another approved project tracking location), with owner, path, and acceptance criteria for compiled C↔Rust seam tests. Then have P05a verify that exact file.

### 2. PEDANTIC — Phase/artifact naming is inconsistent (`P00.5` vs file `00a-preflight-verification.md`)
**Why this matters:** This does not block execution, but it makes navigation and completion tracking less crisp. The overview says there are 12 artifacts including `P00.5` and `P00.5a`, while the actual filenames are `00-overview.md` and `00a-preflight-verification.md`, and there is no `00.5*` filename family.
**Evidence:**
- `00-overview.md`: "Total Phases: 12 artifacts (P00.5, P00.5a, ...)"
- Actual files: `00a-preflight-verification.md`, no `00.5-*` files
**Recommended fix:** Align phase IDs and filenames, or explicitly document the filename-to-phase-ID mapping.

### 3. PEDANTIC — P03/P03a filenames and titles still say "init" even though the phase no longer implements lifecycle work
**Why this matters:** This appears to be stale naming from an earlier draft. It will not cause implementation failure, but it creates avoidable confusion when following the plan.
**Evidence:**
- Filenames: `03-zero-size-oom-and-init-tdd.md`, `03a-zero-size-oom-and-init-verification.md`
- Actual contents: zero-size OOM fix + unit-test gap closure; no init change work in the phase tasks
**Recommended fix:** Rename the files/titles to match the actual scope.

### 4. PEDANTIC — Verification commands are not consistently concrete despite the plan explicitly calling out concrete path/invocation verification as a gap
**Why this matters:** The plan mostly handles this correctly via preflight, but several later phases fall back to placeholder wording like "Use the Phase P00.5-confirmed ... invocation" instead of carrying forward the resolved command. That is acceptable, but weaker than the otherwise high concreteness standard the plan sets for itself.
**Evidence:**
- `00a-preflight-verification.md` explicitly requires recording the exact integration-test invocation
- P03, P04, P05, P05a verification sections use deferred wording instead of embedding the resolved command
**Recommended fix:** Once P00.5 confirms the commands, propagate the exact command strings into downstream phases or reserve a dedicated "resolved commands" section in the overview for reuse.

### 5. PEDANTIC — Some requirement/coverage language blurs subsystem closure with spec test obligations
**Why this matters:** This is mostly wording, not logic. The plan sometimes identifies a gap because explicit tests are missing even though the underlying implementation is already described as correct. That is fine if the plan is enforcing spec-required coverage, but it should be stated consistently as a verification gap rather than an implementation gap.
**Evidence:**
- `00-overview.md` Gap 3 and `01-analysis.md` Gap 3 treat missing explicit unit coverage as a gap
- Elsewhere the overview says the underlying requirements are already satisfied "subject to the missing explicit test coverage"
**Recommended fix:** Label Gap 3 consistently as a verification/test-surface gap to avoid implying the runtime behavior is known-bad.

## Check-by-check assessment

### Gap identification
Good overall. The major code gaps and verification gaps are identified, and the plan correctly distinguishes module-local fixes from program-level obligations. The only important miss is that the residual REQ-MEM-INT-009 handoff is identified but not actually planned as a concrete created artifact.

### Phase ordering
Good. Preflight → analysis → pseudocode → code/test fixes → integration/traceability is sensible. No missing prerequisite ordering issues were found.

### REQ coverage
Mostly good. The coverage matrix is thorough and appropriately distinguishes subsystem obligations from usage constraints and program-level obligations. The one substantive issue is that REQ-MEM-INT-009 residual closure is not turned into an executable deliverable even though the plan says it must be.

### Concrete paths
Mostly good. Source file paths are concrete. The weak spot is the missing concrete path for the downstream mixed-language seam artifact.

### Template compliance
Mostly good. The plan includes prerequisites, tasks, verification commands, structural and semantic checklists, success criteria, and completion markers. Minor naming/template drift remains in the phase/file naming inconsistencies.

### Verification adequacy
Good for local code changes and ABI-surface tests. Adequacy is incomplete only for the residual mixed-language seam obligation because the plan verifies an artifact that it never explicitly creates.

### Missing phases
No additional implementation phases are clearly required for the in-scope module-local fixes. However, the plan needs either:
- a concrete task inside P05 to create the residual seam handoff artifact, or
- a separate explicit handoff phase/document task.

## Overall verdict
Needs revision before execution due to 1 SUBSTANTIVE issue.
