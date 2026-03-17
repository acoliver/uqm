# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P01a`

## Prerequisites
- Required: Phase 01 (Analysis) completed

## Structural Verification Checklist
- [ ] Domain model covers all entity types referenced in specification
- [ ] State transition map covers all campaign modes from §3.1
- [ ] Error/edge case map covers all failure scenarios from requirements
- [ ] Integration touchpoints list is complete (all C files in scope identified)
- [ ] Validated seam inventory template exists for every later P15 bridge candidate
- [ ] Cross-subsystem call inventory covers all dependency boundaries
- [ ] `requirements-traceability.md` created and linked from analysis/overview

## Semantic Verification Checklist
- [ ] Every normative requirement area from `requirements.md` is mapped to at least one implementation phase
- [ ] All 18 event selectors from §8.6 catalog are accounted for
- [ ] All 5 observable campaign modes from §3.1 are represented
- [ ] All covered save contexts from §9.7 are addressed
- [ ] Deferred-transition semantics from §5.3 are captured
- [ ] Starbase mandatory special-sequence categories from §6.4 are identified
- [ ] Save-summary normalization rules from §9.2 are documented
- [ ] Load-failure safe-failure guarantees from §9.4.0b are enumerated
- [ ] Mandatory scheduled-event rejection cases from §9.4.1 are listed
- [ ] Adjunct-dependency table from §9.4.0b is referenced
- [ ] Claim-family inspection-surface selection and no-mixing rule from `requirements.md` are mapped to phases
- [ ] Export-success vs overall covered-context distinction is represented in phase coverage
- [ ] Legacy-starbase observational exception path is represented in phase coverage
- [ ] Verifier report minimum fields are mapped to implementation/verification phases
- [ ] §8.6 row-specific normalization / checkpoint-bundle rules are called out as spec-controlled, not generalized locally
- [ ] §10.1 load/export outcome-class rules are called out as spec-controlled, not generalized locally

## Completeness Gate
- [ ] No specification section left unrepresented in the analysis
- [ ] No orphan normative requirement paragraphs in `requirements.md`
- [ ] All integration boundaries match the boundary model in §2.2
- [ ] No concrete C replacement seam listed without validation notes
- [ ] No FFI export/import named in later phases without a placeholder backlink requirement to the seam inventory

## Gate Decision
- [ ] PASS: proceed to Phase 02
- [ ] FAIL: revise analysis — document gaps
