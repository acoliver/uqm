# Graphics Plan Review

## Overall assessment
The plan is strong on decomposition, concrete file targeting, migrated-path revalidation, and requirement traceability. It explicitly calls out the main migration risks and does not stop at Rust-local unit work. Most findings below are therefore about execution safety and completeness at the C/Rust boundary.

## Findings

### 1. Missing explicit phase for event-pump / SDL event forwarding verification
**Rating:** SUBSTANTIVE

The requirements/spec place SDL event collection/forwarding inside graphics subsystem scope, and the requirements coverage matrix marks `REQ-INT-001` and backend/lifecycle obligations as covered/revalidated, but no phase includes explicit verification that the Rust-enabled path still preserves event-pump behavior (`rust_gfx_process_events`, ordering, no dropped events, correct lifecycle interaction after reinit).

Why this is substantive:
- The specification explicitly includes SDL backend lifecycle and event-pump ownership in subsystem scope.
- Reinit work in P07 can invalidate or replace event-pump state.
- A graphics migration that renders correctly but drops or mishandles SDL events would fail subsystem execution in practice.

What is missing:
- A concrete phase task naming the event-processing entry points and files.
- Verification after reinit and normal init/uninit that event collection remains behaviorally compatible.
- Requirement traceability for this domain instead of leaving it implicit under broad REQ-INT coverage.

Recommended fix:
- Add a dedicated subphase or expand P07/P10 to cover event-pump lifecycle and forwarding verification, including post-reinit validation.

---

### 2. Draw-queue command inventory is inconsistent with the plan’s own “all 16 command types” claim
**Rating:** SUBSTANTIVE

The overview/DoD repeatedly asserts “all 16 DCQ command types,” but the enumerated command set in P00.5 and elsewhere omits `SetPalette` until later, and other places still talk about “all 15 variants.” The plan also mixes command completeness, control-path ingress, and draw wrappers in a way that leaves the actual authoritative command inventory ambiguous.

Why this is substantive:
- P05/P09 depend on a precise migrated command/control inventory.
- Ambiguity here can leave one queue-mediated externally visible operation unwired or unverified.
- This directly affects REQ-DQ-001 single-ingress and FFI symbol compatibility work.

Evidence in the plan:
- Overview DoD says “All 16 DCQ command types.”
- P00.5 says DrawCommand has “all 15 variants” and lists a set without `SetPalette`.
- G13 separately says `SetPalette` variant is missing.

Recommended fix:
- Normalize the command inventory in overview, preflight, analysis, pseudocode, and verification docs to one authoritative list, explicitly distinguishing queue commands from non-queue control operations.

---

### 3. Verification adequacy for some requirements is overstated because several “DONE / REVALIDATE” items have no phase-level evidence path
**Rating:** SUBSTANTIVE

The requirements matrix marks many requirements as already done and to be revalidated later, but some of those later phases do not actually contain task-level verification for the specific obligation. The most visible example is event processing, but the same pattern also affects some lifecycle/ownership obligations where the matrix promises revalidation without a named test or check.

Why this is substantive:
- The user asked for strict REQ coverage review.
- A coverage matrix that overstates closed coverage can let the plan finish while still missing requirements.
- This is especially risky for migrated-path behaviors that are easy to assume and hard to observe indirectly.

Recommended fix:
- For every matrix row marked `DONE / REVALIDATE`, point to at least one concrete phase task or verification checklist item by phase ID.
- If no such task exists, downgrade the row to planned/partial and add the missing verification work.

---

### 4. Template compliance drift between execution and verification documents
**Rating:** PEDANTIC

There are a few internal consistency/template issues:
- P03 plans 8 tests, but P03a says 5 new tests.
- P10 plans 16+ integration tests, but P10a says 6+ tests.
- Some phase titles differ slightly between overview and phase docs.

Why this is pedantic:
- These do not by themselves cause execution failure or missed requirements.
- They do, however, weaken auditability and can confuse implementers during completion tracking.

Recommended fix:
- Align counts/titles/checklists so each verification file matches its implementation phase exactly.

---

### 5. Concrete pathing is generally good, but a few required integration sites remain intentionally unresolved too late in the plan
**Rating:** PEDANTIC

The plan is usually very concrete, but some items still say “actual lifecycle owner (likely ... but verify)” or “the real image lifecycle boundary identified in Task 1.” That is acceptable during analysis, but it leaves some implementation docs less path-concrete than others.

Why this is pedantic:
- The plan explicitly requires those sites to be resolved before implementation, so this is mostly process hygiene.
- The later tasks do acknowledge the uncertainty and require grounding before edits.

Recommended fix:
- In P01/P02 or as an amendment to overview, record the resolved file/function owners once identified, so later phases are fully path-concrete before execution starts.

---

### 6. Phase ordering is mostly correct, but event/lifecycle verification should be placed before final bridge confidence claims
**Rating:** PEDANTIC

The ordering of pixel coherence → postprocess → command completeness → queue semantics → reinit/system-box → rotation → C wiring → integration is sensible. The only ordering concern is that lifecycle-sensitive event processing, if added, should sit alongside P07/P09 rather than being left to implied final verification.

Why this is pedantic:
- The current order will still work if the missing event phase is added.
- It is an ordering refinement, not a structural blocker by itself.

## Summary judgment
The plan is close to execution-ready, but it has **3 SUBSTANTIVE findings**:
1. missing explicit event-pump/event-forwarding phase coverage,
2. inconsistent authoritative DCQ command inventory,
3. overstated REQ coverage where some “revalidate later” claims lack concrete downstream verification tasks.

If those are fixed, the remaining issues are cleanup-level only.
