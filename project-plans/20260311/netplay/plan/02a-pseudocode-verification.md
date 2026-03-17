# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-NETPLAY.P02a`

## Prerequisites
- Required: Phase 02 (Pseudocode) completed
- Expected artifacts: 15 pseudocode component files in `analysis/pseudocode/`

## Verification Tasks

### Structural Verification
- [ ] All 15 component files exist in `project-plans/20260311/netplay/analysis/pseudocode/`
- [ ] All pseudocode lines are numbered sequentially
- [ ] Every function has explicit parameter types and return types
- [ ] Every function has at least one validation/REQUIRE statement

### Algorithmic Correctness
- [ ] State machine transitions match analysis phase state diagram
- [ ] Packet serialization produces 4-byte-aligned output
- [ ] Packet deserialization handles incomplete packets correctly (wait for more data)
- [ ] Ready protocol correctly handles all 4 cases: local-first, remote-first, simultaneous, re-entrant
- [ ] Confirmation protocol handles the full Handshake0→Handshake1→Cancel→CancelAck dance
- [ ] Reset protocol handles bidirectional reset with correct confirmation semantics
- [ ] Battle input buffer pre-fill matches `inputDelay` neutral frames
- [ ] Checksum buffer sizing formula matches C: `(input_delay * 2 / checksum_interval) + 2`
- [ ] Setup conflict-resolution pseudocode explicitly defines the deterministic winner for crossing edits
- [ ] Shared progress-loop pseudocode flushes, receives, dispatches, delivers events/callbacks, and re-checks wait predicates in one loop
- [ ] Ship-selection semantic-validation pseudocode blocks battle handoff and initiates reset on owner-side rejection

### Error Handling Coverage
- [ ] Every network I/O operation has error handling
- [ ] WouldBlock is handled as "try again later" not as failure
- [ ] Interrupted is handled as "retry immediately"
- [ ] Connection reset / EOF is handled as connection close
- [ ] Buffer overflow (input buffer full) is handled as protocol error
- [ ] Invalid state transitions are rejected with error

### Requirements Traceability
- [ ] `REQ-NP-CONN-*` → Components 003, 004, 012, 014
- [ ] `REQ-NP-PROTO-*` → Component 002 and Init-related dispatch paths
- [ ] `REQ-NP-SETUP-006` → Component 013
- [ ] `REQ-NP-READY-004` and specification §18 → Component 014
- [ ] `REQ-NP-INPUT-*` → Components 010 and 014
- [ ] `REQ-NP-SHIP-005` → Component 015
- [ ] `REQ-NP-CHECK-*` → Component 011
- [ ] `REQ-NP-RESET-*` / `REQ-NP-ABORT-*` → Components 009 and 014
- [ ] `REQ-NP-END-*` → battle-end and ready/progress pseudocode coverage

### Integration Boundary Annotations
- [ ] Every component marks where SuperMelee calls into it
- [ ] Every component marks where it emits events to SuperMelee
- [ ] Transport abstraction boundary is clear

## Gate Decision
- [ ] PASS: proceed to Phase 03
- [ ] FAIL: return to Phase 02 and address gaps

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P02a.md`
