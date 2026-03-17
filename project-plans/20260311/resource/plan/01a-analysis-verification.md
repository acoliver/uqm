# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260314-RESOURCE.P01a`

## Prerequisites
- Phase 01 analysis document complete

## Structural Verification Checklist
- [ ] Every gap (GAP-1 through GAP-11) has a root cause documented
- [ ] Every gap references specific file paths and line numbers
- [ ] Every gap maps to at least one REQ-* identifier
- [ ] Every gap has a concrete "fix needed" description
- [ ] Integration touchpoints are documented with direction (C→Rust or Rust→C)
- [ ] Entity state transitions cover all resource entry lifecycle states
- [ ] Old code removal list is explicit and complete

## Semantic Verification Checklist
- [ ] All requirements from `project-plans/20260311/resource/requirements.md` are represented by either direct change work or explicit verification coverage
- [ ] No gap duplicates another gap
- [ ] No gap contradicts the specification
- [ ] Fix descriptions are minimal — no scope creep beyond gap closure
- [ ] The analysis does not propose replacing working code
- [ ] Requirements claimed as already implemented are called out separately from requirements directly fixed by a gap phase
- [ ] Every requirement marked "Explicit verification" is mapped below to a concrete later-phase verification task, command, test, or evidence artifact

## Requirement Coverage Matrix

| Requirement | Coverage | Status |
|------------|----------|--------|
| REQ-RES-LIFE-001 | Explicit verification | Already implemented (InitResourceSystem works) |
| REQ-RES-LIFE-002 | Explicit verification | Already implemented (built-in types registered) |
| REQ-RES-LIFE-003 | Explicit verification | Already implemented (LoadResourceIndex works) |
| REQ-RES-LIFE-004 | GAP-5 | UninitResourceSystem doesn't call freeFun |
| REQ-RES-LIFE-005 | GAP-5 + integration verification | Reinit works but prior teardown leaks |
| REQ-RES-LIFE-006 | Explicit verification | Already implemented (init failure is safe) |
| REQ-RES-LIFE-007 | Explicit verification | Already implemented (auto-init via ensure_initialized) |
| REQ-RES-TYPE-001 | Explicit verification | Already implemented |
| REQ-RES-TYPE-002 | Explicit verification | Already implemented |
| REQ-RES-TYPE-003 | Explicit verification | Already implemented |
| REQ-RES-TYPE-004 | GAP-8 | Return type is u16 not u32 |
| REQ-RES-TYPE-005 | Explicit verification | Already implemented |
| REQ-RES-TYPE-006 | Explicit verification | Already implemented |
| REQ-RES-TYPE-007 | Explicit verification | Already implemented |
| REQ-RES-TYPE-008 | Explicit verification | Already implemented (replacement overwrites) |
| REQ-RES-IDX-001 | Explicit verification | Already implemented |
| REQ-RES-IDX-002 | Explicit verification | Already implemented |
| REQ-RES-IDX-003 | Explicit verification | Already implemented |
| REQ-RES-IDX-004 | Explicit verification | Already implemented |
| REQ-RES-IDX-005 | GAP-7 | SaveResourceIndex emits entries without toString |
| REQ-RES-IDX-006 | GAP-6 | Entry replacement doesn't free old resources |
| REQ-RES-IDX-007 | Explicit verification | Already implemented (partial-load, non-transactional) |
| REQ-RES-IDX-008 | Explicit verification | Already implemented (silent return on file-open fail) |
| REQ-RES-UNK-001 | GAP-3 | UNKNOWNRES not stored as value type |
| REQ-RES-UNK-002 | GAP-7 | UNKNOWNRES entries emitted during save |
| REQ-RES-UNK-003 | GAP-3, GAP-4, GAP-2 | UNKNOWNRES accessor behavior wrong |
| REQ-RES-UNK-004 | Explicit verification | Already implemented (no retroactive conversion) |
| REQ-RES-CONF-001 | Explicit verification | Already implemented |
| REQ-RES-CONF-002 | Explicit verification | Already implemented |
| REQ-RES-CONF-003 | GAP-2 | res_GetString lacks type check, returns null |
| REQ-RES-CONF-004 | Explicit verification | Already implemented |
| REQ-RES-CONF-005 | Explicit verification | Already implemented |
| REQ-RES-CONF-006 | Explicit verification | Already implemented |
| REQ-RES-CONF-007 | Explicit verification | Already implemented |
| REQ-RES-CONF-008 | Explicit verification | Already implemented |
| REQ-RES-CONF-009 | GAP-2, GAP-7 + integration verification | Config persistence affected by string/save bugs |
| REQ-RES-LOAD-001 | Explicit verification | Already implemented |
| REQ-RES-LOAD-002 | Explicit verification | Already implemented |
| REQ-RES-LOAD-003 | GAP-4 | Value types don't increment refcount properly |
| REQ-RES-LOAD-004 | Explicit verification | Already implemented |
| REQ-RES-LOAD-005 | Explicit verification | Already implemented |
| REQ-RES-LOAD-006 | Explicit verification | Already implemented |
| REQ-RES-LOAD-007 | Phase 06 explicit verification | Must be verified directly for value-type `res_FreeResource` / `res_DetachResource` safety; do not treat GAP-3 alone as sufficient |
| REQ-RES-LOAD-008 | Phase 06 explicit verification | Must be verified directly via `res_Remove` behavior for materialized resources; do not treat GAP-6 alone as sufficient |
| REQ-RES-LOAD-009 | Explicit verification | Already implemented |
| REQ-RES-LOAD-010 | Explicit verification | Already implemented |
| REQ-RES-LOAD-011 | GAP-4 | Value-type access through general accessor broken |
| REQ-RES-FILE-001 | Explicit verification | Already implemented (UIO wrappers) |
| REQ-RES-FILE-002 | GAP-1 | res_OpenResFile missing directory detection |
| REQ-RES-FILE-003 | GAP-1, GAP-9 | Sentinel not returned for directories and must be rejected by load-from-path |
| REQ-RES-FILE-004 | Explicit verification | Already implemented (_cur_resfile_name guard) |
| REQ-RES-FILE-005 | GAP-9 | LoadResourceFromPath missing sentinel/zero-length guards |
| REQ-RES-FILE-006 | GAP-10 | GetResourceData doc comment misleading |
| REQ-RES-FILE-007 | Explicit verification | Already implemented (FreeResourceData) |
| REQ-RES-FILE-008 | GAP-9 | File handle leak risk on invalid open/zero-length path |
| REQ-RES-OWN-001 | Explicit verification | Already implemented |
| REQ-RES-OWN-002 | Explicit verification | Already implemented |
| REQ-RES-OWN-003 | Explicit verification | Already implemented |
| REQ-RES-OWN-004 | Explicit verification | Already implemented |
| REQ-RES-OWN-005 | GAP-5 + Phase 06 explicit verification | UninitResourceSystem skips freeFun |
| REQ-RES-OWN-006 | Explicit verification | Already implemented |
| REQ-RES-OWN-007 | Explicit verification | Already implemented |
| REQ-RES-OWN-008 | GAP-5 + integration verification | Teardown doesn't properly release |
| REQ-RES-OWN-009 | GAP-6 | Replacement doesn't call freeFun |
| REQ-RES-OWN-010 | GAP-5 | UninitResourceSystem doesn't call type-specific free |
| REQ-RES-ERR-001 | Explicit verification | Already implemented (log-and-return-null/false) |
| REQ-RES-ERR-002 | Explicit verification | Already implemented |
| REQ-RES-ERR-003 | GAP-2 | res_GetString returns null not "" |
| REQ-RES-ERR-004 | Explicit verification | Already implemented |
| REQ-RES-ERR-005 | Explicit verification | Already implemented |
| REQ-RES-ERR-006 | Explicit verification | Already implemented |
| REQ-RES-INT-001 | Explicit verification | Already implemented |
| REQ-RES-INT-002 | Explicit verification | Already implemented |
| REQ-RES-INT-003 | Explicit verification | Already implemented |
| REQ-RES-INT-004 | Explicit verification | Already implemented |
| REQ-RES-INT-005 | Explicit verification | Already implemented |
| REQ-RES-INT-006 | GAP-11 | Dead code creates dual runtime path risk |
| REQ-RES-INT-007 | Explicit verification | Already implemented |
| REQ-RES-INT-008 | GAP-8 | CountResourceTypes ABI mismatch |
| REQ-RES-INT-009 | Explicit verification | Already implemented |

## Explicit-Verification Requirement Execution Matrix
These requirements are believed to be already implemented, but the plan must still provide executable verification evidence rather than assertion-only coverage.

| Requirement | Verification Phase | Concrete verification task / evidence |
|------------|--------------------|---------------------------------------|
| REQ-RES-LIFE-001 | P11 Step 8 | Record engine boot/init evidence showing `InitResourceSystem()` succeeds without replacement workarounds |
| REQ-RES-LIFE-002 | P11 Step 8 + P09a | Record built-in type count/registration evidence, including `CountResourceTypes` ABI-safe check after GAP-8 |
| REQ-RES-LIFE-003 | P11 Step 8 | Record successful `LoadResourceIndex()` integration evidence from startup/config load path |
| REQ-RES-LIFE-006 | P11 Step 8 | Record failure-path evidence from targeted test or existing test covering safe init failure handling |
| REQ-RES-LIFE-007 | P11 Step 8 | Record test/integration evidence that `ensure_initialized`-driven access still works |
| REQ-RES-TYPE-001 | P11 Step 8 | Record handler installation evidence from built-in registry state or targeted test output |
| REQ-RES-TYPE-002 | P11 Step 8 | Record lookup/access evidence showing installed types are discoverable through the authoritative registry |
| REQ-RES-TYPE-003 | P11 Step 8 | Record evidence that built-ins and installed handlers coexist correctly |
| REQ-RES-TYPE-005 | P11 Step 8 | Stable dispatch source evidence; identify exact test or integration path used |
| REQ-RES-TYPE-006 | P11 Step 8 | Registration replacement consistency evidence; identify exact test or integration path used |
| REQ-RES-TYPE-007 | P11 Step 8 | Record evidence that lookup/count behavior remains consistent after installs |
| REQ-RES-TYPE-008 | P06a + P11 Step 8 | Use replacement-path tests plus integration note confirming replacement semantics remain intact |
| REQ-RES-IDX-001 | P11 Step 8 | Record successful index load evidence with exact file/path used |
| REQ-RES-IDX-002 | P11 Step 8 | Record namespace/root handling evidence with exact index path used |
| REQ-RES-IDX-003 | P11 Step 8 | Record descriptor parsing evidence from targeted test or integration load log |
| REQ-RES-IDX-004 | P11 Step 8 | Record duplicate/additive index handling evidence from targeted test or integration path |
| REQ-RES-IDX-007 | P11 Step 8 | Record non-transactional/partial-load behavior evidence from targeted test |
| REQ-RES-IDX-008 | P11 Step 8 | Record file-open-failure silent-return evidence from targeted test or integration path |
| REQ-RES-UNK-004 | P11 Step 8 | Record evidence that unknown-type entries are not retroactively converted |
| REQ-RES-CONF-001 | P11 Step 4 + Step 8 | Config round-trip evidence with exact setting changed and saved |
| REQ-RES-CONF-002 | P11 Step 4 + Step 8 | Config load evidence with exact persisted key/value observed after restart |
| REQ-RES-CONF-004 | P11 Step 8 | Record evidence for non-string config accessor behavior remaining correct |
| REQ-RES-CONF-005 | P11 Step 8 | Record evidence for integer config handling remaining correct |
| REQ-RES-CONF-006 | P11 Step 8 | Record evidence for boolean config handling remaining correct |
| REQ-RES-CONF-007 | P11 Step 8 | Record evidence for color config handling remaining correct |
| REQ-RES-CONF-008 | P11 Step 8 | Record evidence for config mutation/removal behavior remaining correct |
| REQ-RES-LOAD-001 | P11 Step 5 + Step 8 | Record resource materialization evidence during normal gameplay/menu navigation |
| REQ-RES-LOAD-002 | P11 Step 5 + Step 8 | Record evidence that already-loaded resources are reused correctly |
| REQ-RES-LOAD-004 | P11 Step 8 | Record targeted test or integration evidence for heap-backed load path |
| REQ-RES-LOAD-005 | P11 Step 8 | Record refcount/detach behavior evidence for heap-backed resources |
| REQ-RES-LOAD-006 | P11 Step 8 | Record destructor path evidence for heap-backed resources |
| REQ-RES-LOAD-009 | P11 Step 8 | Record callback error-path containment evidence |
| REQ-RES-LOAD-010 | P11 Step 8 | Record evidence that general access path remains correct for loadable resources |
| REQ-RES-FILE-001 | P08a + P11 Step 8 | Record UIO wrapper/open behavior evidence using exact file path/fixture |
| REQ-RES-FILE-004 | P11 Step 8 | Record `_cur_resfile_name` guard evidence from targeted test or integration log |
| REQ-RES-FILE-007 | P11 Step 8 | Record `FreeResourceData` evidence from targeted test or integration check |
| REQ-RES-OWN-001 | P11 Step 8 | Record ownership/refcount baseline evidence |
| REQ-RES-OWN-002 | P11 Step 8 | Record evidence for acquire/use ownership semantics |
| REQ-RES-OWN-003 | P11 Step 8 | Record evidence for detach semantics on heap-backed resources |
| REQ-RES-OWN-004 | P11 Step 8 | Record evidence for free semantics on heap-backed resources |
| REQ-RES-OWN-006 | P11 Step 8 | Record evidence for non-owner path safety |
| REQ-RES-OWN-007 | P11 Step 8 | Record evidence for no-double-free or safe repeated-release behavior |
| REQ-RES-OWN-008 | P06a + P11 Step 7 | Use cleanup tests plus shutdown evidence to confirm teardown release behavior |
| REQ-RES-ERR-001 | P11 Step 8 | Record log-and-return-null/false evidence |
| REQ-RES-ERR-002 | P11 Step 8 | Record non-fatal error-path evidence |
| REQ-RES-ERR-004 | P11 Step 8 | Callback-failure containment evidence; identify exact test/integration path used |
| REQ-RES-ERR-005 | P11 Step 8 | Record error propagation/logging evidence |
| REQ-RES-ERR-006 | P11 Step 8 | Record safe recovery-after-error evidence |
| REQ-RES-INT-001 | P11 Step 8 | Record C→Rust entrypoint evidence via existing callers |
| REQ-RES-INT-002 | P11 Step 8 | Record Rust-side ABI compatibility evidence |
| REQ-RES-INT-003 | P11 Step 8 | Record end-to-end authoritative module path evidence |
| REQ-RES-INT-004 | P11 Step 8 | Record evidence for expected C caller compatibility |
| REQ-RES-INT-005 | P11 Step 8 | Record no-format-change evidence through config/index round-trip |
| REQ-RES-INT-007 | P11 Step 8 | Cross-language callback safety evidence; identify exact test/integration path used |
| REQ-RES-INT-009 | P10a + P11 Step 8 | Runtime authority split evidence using dead-code verification plus final integration confirmation |

## Already-Implemented Requirements Revalidation Checklist
These requirements are not driven by a direct code-change gap in this plan, but must still be revalidated by targeted regression or integration checks:

- [ ] REQ-RES-LIFE-001 — initialization success evidence recorded
- [ ] REQ-RES-LIFE-002 — built-in registration evidence recorded
- [ ] REQ-RES-LIFE-003 — index load startup evidence recorded
- [ ] REQ-RES-LIFE-006 — safe init failure evidence recorded
- [ ] REQ-RES-LIFE-007 — auto-init evidence recorded
- [ ] REQ-RES-TYPE-001 — install contract evidence recorded
- [ ] REQ-RES-TYPE-002 — lookup contract evidence recorded
- [ ] REQ-RES-TYPE-003 — coexistence evidence recorded
- [ ] REQ-RES-TYPE-005 — stable dispatch source evidence recorded
- [ ] REQ-RES-TYPE-006 — registration replacement consistency evidence recorded
- [ ] REQ-RES-TYPE-007 — lookup/count consistency evidence recorded
- [ ] REQ-RES-TYPE-008 — overwrite semantics evidence recorded
- [ ] REQ-RES-IDX-001 — index load evidence recorded
- [ ] REQ-RES-IDX-002 — namespace/root handling evidence recorded
- [ ] REQ-RES-IDX-003 — descriptor parsing evidence recorded
- [ ] REQ-RES-IDX-004 — duplicate/additive handling evidence recorded
- [ ] REQ-RES-IDX-007 — partial-load behavior evidence recorded
- [ ] REQ-RES-IDX-008 — file-open-failure evidence recorded
- [ ] REQ-RES-UNK-004 — no retroactive conversion evidence recorded
- [ ] REQ-RES-CONF-001 — config persistence evidence recorded
- [ ] REQ-RES-CONF-002 — config reload evidence recorded
- [ ] REQ-RES-CONF-004 — non-string accessor evidence recorded
- [ ] REQ-RES-CONF-005 — integer config evidence recorded
- [ ] REQ-RES-CONF-006 — boolean config evidence recorded
- [ ] REQ-RES-CONF-007 — color config evidence recorded
- [ ] REQ-RES-CONF-008 — config mutation/removal evidence recorded
- [ ] REQ-RES-LOAD-001 — materialization evidence recorded
- [ ] REQ-RES-LOAD-002 — reuse evidence recorded
- [ ] REQ-RES-LOAD-004 — heap load path evidence recorded
- [ ] REQ-RES-LOAD-005 — heap detach/refcount evidence recorded
- [ ] REQ-RES-LOAD-006 — heap destructor evidence recorded
- [ ] REQ-RES-LOAD-009 — callback error-path evidence recorded
- [ ] REQ-RES-LOAD-010 — loadable resource accessor evidence recorded
- [ ] REQ-RES-FILE-001 — file-open wrapper evidence recorded
- [ ] REQ-RES-FILE-004 — current-resource-name guard evidence recorded
- [ ] REQ-RES-FILE-007 — `FreeResourceData` evidence recorded
- [ ] REQ-RES-OWN-001 — ownership baseline evidence recorded
- [ ] REQ-RES-OWN-002 — ownership acquire/use evidence recorded
- [ ] REQ-RES-OWN-003 — detach semantics evidence recorded
- [ ] REQ-RES-OWN-004 — free semantics evidence recorded
- [ ] REQ-RES-OWN-006 — non-owner safety evidence recorded
- [ ] REQ-RES-OWN-007 — repeated-release safety evidence recorded
- [ ] REQ-RES-OWN-008 — teardown release evidence recorded
- [ ] REQ-RES-ERR-001 — null/false failure signaling evidence recorded
- [ ] REQ-RES-ERR-002 — non-fatal error-path evidence recorded
- [ ] REQ-RES-ERR-004 — callback-failure containment evidence recorded
- [ ] REQ-RES-ERR-005 — error propagation/logging evidence recorded
- [ ] REQ-RES-ERR-006 — recovery-after-error evidence recorded
- [ ] REQ-RES-INT-001 — C→Rust entrypoint evidence recorded
- [ ] REQ-RES-INT-002 — ABI compatibility evidence recorded
- [ ] REQ-RES-INT-003 — authoritative module path evidence recorded
- [ ] REQ-RES-INT-004 — caller compatibility evidence recorded
- [ ] REQ-RES-INT-005 — no-format-change evidence recorded
- [ ] REQ-RES-INT-007 — cross-language callback safety evidence recorded
- [ ] REQ-RES-INT-009 — runtime authority split evidence recorded

## Gate Decision
- [ ] Analysis is complete and verified
- [ ] Proceed to Phase 02 (Pseudocode)
