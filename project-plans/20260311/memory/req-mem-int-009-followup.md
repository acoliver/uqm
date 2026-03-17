# REQ-MEM-INT-009 Follow-Up: Mixed-Language Seam Test Handoff

## Purpose
Track the remaining project-level work required to close the true C↔Rust seam-testing portion of `REQ-MEM-INT-009`.

## Requirement
- `REQ-MEM-INT-009`

## Owner
- Project-level integration/testing owner for the memory subsystem boundary work

## Artifact Path
- `project-plans/20260311/memory/req-mem-int-009-followup.md`

## Scope
This follow-up exists because the memory plan's Rust-side ABI integration tests validate the exported Rust ABI surface but do not verify the compiled mixed-language seam through actual C callers and headers.

## Required Acceptance Criteria
The downstream mixed-language seam work is complete only when the project test suite includes compiled-boundary coverage for all of the following:
1. A real C caller reaches the Rust memory implementation through `sc2/src/libs/memlib.h`.
2. Memory allocated from the C side is successfully released through the Rust memory path.
3. Memory allocated from the Rust side is successfully released from the C side through the historical API boundary.
4. Zero-size seam behavior is verified across the compiled C↔Rust boundary for allocation and reallocation entry points.
5. Lifecycle sequencing across the compiled boundary is exercised, including init/uninit interactions relevant to the memory subsystem contract.

## Expected Downstream Deliverable
Create or update a project-level test plan or tracker entry that schedules the compiled C↔Rust seam harness work and points to the concrete tests that will satisfy the acceptance criteria above.
