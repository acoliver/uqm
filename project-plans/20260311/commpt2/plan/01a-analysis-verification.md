# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260326-COMMPT2.P01a`

## Prerequisites
- Required: Phase 01 (Analysis) completed
- Analysis document exists at `project-plans/20260311/commpt2/plan/01-analysis.md`

## Structural Verification Checklist

- [ ] Stub inventory covers all 4 FFI stubs in ffi.rs
- [ ] Stub inventory covers all 6 input stubs in talk_segue.rs
- [ ] Stub inventory covers all 3 C rendering stubs in rust_comm.c
- [ ] Stub inventory covers the 1 transition animation stub
- [ ] Call-path traces cover HailAlien flow (C → Rust → C round-trip)
- [ ] Call-path traces cover input bridge flow
- [ ] Call-path traces cover NPCPhrase flow
- [ ] Call-path traces cover rendering bridge flow
- [ ] Resource lifecycle map covers all 7 load resources + 2 contexts
- [ ] Integration touchpoints list existing callers
- [ ] Integration touchpoints list new callers to be added
- [ ] Integration touchpoints list old behavior being replaced
- [ ] Requirements coverage matrix maps every REQ-* to a phase

## Semantic Verification Checklist

- [ ] Every stub identified has a corresponding resolution phase
- [ ] No stub is assigned to multiple phases (no duplication)
- [ ] Call-path traces match the actual C reference code (comm.c:1183–1308)
- [ ] Resource lifecycle matches C's load/free sequence exactly
- [ ] Resource cleanup order is reverse of load order (matching C behavior)
- [ ] All exit paths (normal, abort, load) are covered in cleanup analysis
- [ ] Input key indices match the C constants from controls.h
- [ ] Every requirement (REQ-HL through REQ-E2E) has at least one phase assigned
- [ ] No requirement is orphaned (uncovered by any phase)

## Verification Commands

```bash
# Verify all stubs mentioned in analysis actually exist
grep -n "P11: Stub\|P11: Track\|// P11:" rust/src/comm/ffi.rs
grep -n "false.*production\|false.*hardcoded\|false // " rust/src/comm/talk_segue.rs
grep -n "P11: Stub" sc2/src/uqm/rust_comm.c

# Verify C reference matches analysis
grep -n "HailAlien" sc2/src/uqm/comm.c
grep -n "DoCommunication\|DoInput" sc2/src/uqm/comm.c

# Verify bridge wrappers exist
grep -n "^c_\|^void$\|^int$\|^unsigned" sc2/src/uqm/rust_comm.c | head -50
```

## Pass/Fail Gate Criteria

**PASS if**:
- All structural checks are satisfied
- All semantic checks are satisfied
- Requirements coverage is complete (no gaps)
- Stub count matches actual codebase

**FAIL if**:
- Any stub is missing from inventory
- Any requirement is unassigned
- Call paths don't match C reference
- Resource lifecycle has gaps
