# Phase 15: Final Integration & Signoff

## Phase ID
`PLAN-20260314-SUPERMELEE.P15`

## Prerequisites
- Required: Phase 14 completed and passed
- Local integration verification, netplay-boundary verification, compatibility audit, and requirement matrix all complete

## Purpose

Perform final signoff using the statement-level requirement matrix, compatibility-audit decisions, and integrated verification evidence gathered in earlier phases. This phase no longer claims blanket coverage without explicit matrix support.

## Final Verification Inputs
- `requirements.md`
- `requirement-traceability-matrix.md`
- compatibility-audit outputs from Phase P10
- local integration results from Phase P13
- netplay-boundary integration results from Phase P14
- FFI/build verification results from the scoped wiring phase

## Signoff Tasks

### 1. Requirement-by-requirement review
For each row in `requirement-traceability-matrix.md`:
- confirm at least one concrete automated or manual verification artifact exists,
- confirm the mapped implementation phase actually delivered the required behavior,
- confirm any audit-gated obligation is evaluated according to the compatibility audit outcome,
- record pass/fail and any residual follow-up.

### 2. Build and integration verification
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If the scoped Rust/C integration uses a dedicated build flag or target, run the project-verified command established earlier in the plan rather than assuming an unverified launch/make invocation.

### 3. Manual signoff checklist
- [ ] SuperMelee local setup flow is usable end-to-end
- [ ] Built-in and saved-team loading both work on the active side
- [ ] Fleet-edit confirm/cancel semantics match requirements
- [ ] Match-start validation prevents invalid starts
- [ ] Battle-facing initial/next combatant handoff preserves the audited battle-ready contract
- [ ] Post-battle return restores a valid SuperMelee state
- [ ] Local-only mode works without network state
- [ ] Netplay-boundary obligations pass their dedicated verification when that mode is supported
- [ ] Compatibility-audit-sensitive items are enforced only to the level required by the audit outcome

## Structural Verification Checklist
- [ ] Requirement matrix is complete and reviewed row-by-row
- [ ] Final signoff artifact records actual evidence rather than broad topic claims
- [ ] No remaining plan text claims full coverage without matrix support

## Semantic Verification Checklist (Mandatory)
- [ ] Every requirement statement from `requirements.md` is explicitly accounted for via the matrix
- [ ] Built-in team load and saved-team-file load remain separately verified at signoff
- [ ] Netplay setup sync, start gating, and remote selection acceptance/rejection semantics remain separately verified at signoff
- [ ] Battle-facing handoff is verified against the audited contract and not described in weakened ship-ID terms
- [ ] Compatibility-sensitive exactness obligations are only marked pass if the audit required them and the evidence supports them

## Success Criteria
- [ ] All required automated checks pass
- [ ] Requirement matrix shows full statement-level coverage
- [ ] Compatibility-audit conclusions and verification evidence are internally consistent
- [ ] Final signoff can be defended against the specification/review without scope drift

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P15.md`

Contents:
- Plan ID: PLAN-20260314-SUPERMELEE
- Final timestamp
- Files modified across the scoped SuperMelee plan
- Verification outputs actually used for signoff
- Completed requirement traceability matrix reference
- Compatibility-audit decisions reference
- Overall pass/fail decision
