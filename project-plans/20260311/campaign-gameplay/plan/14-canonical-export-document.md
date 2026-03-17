# Phase 14: Canonical Export Document Execution

## Phase ID
`PLAN-20260314-CAMPAIGN.P14`

## Prerequisites
- Required: Phase 13a completed
- Expected files: All save/load modules, types, events, starbase progression point, and the frozen export/report contract from Phase 05
- Dependency: serde/serde_json for JSON serialization

## Requirements Implemented (Expanded)

### Campaign Canonical Export Document (§10.1)
**Requirement text**: When an implementation uses an export-based inspection surface, provide a Campaign Canonical Export Document: a single machine-readable canonical export derived solely from one persisted save artifact.

### Inspection-Surface Applicability Rule (`requirements.md` / specification §10.1)
**Requirement text**: Export-based inspection is required only for covered claim/context families whose canonical persistence-boundary facts are not directly exposed by a documented, machine-readable, verifier-stable raw save representation. When raw-save inspection is sufficient, no export entry point is required for conformance of that claim/context family.

### Required Sections (§10.1)
The export document shall always contain these 8 sections:
1. `schema_version` — version identifier
2. `save_summary` — user-facing summary (activity, location, date)
3. `resume_context` — recoverable mode, navigation, transition markers
4. `clock_state` — campaign date and tick state
5. `scheduled_events` — canonical event view per §8.6 schema
6. `campaign_flags` — progression flags
7. `faction_state` — faction strategic state (17 factions)
8. `encounter_state` — encounter handoff state

### Export Error Contract (§10.1)
**Requirement text**: When export fails, output shall be JSON with `result: "error"`, `error_code`, `error_message`. No partial success document.

### conformance_input_class (§10.1)
**Requirement text**: Include `conformance_input_class` field: `covered_mandatory` or `diagnostic_only`.

### Verifier Reporting / Adjunct-Sensitive Outcome Reporting (`requirements.md`)
**Requirement text**: Verifier output shall distinguish claim-local result from overall covered-context result and state whether adjunct dependency changed the overall result.

### Legacy-Starbase Observational Exception
**Requirement text**: For covered valid legacy starbase save/load claims only, observation-based controlled comparison may be required when raw-save facts do not sufficiently expose the closed comparison object.

### Malformed-Save Export (§10.1)
**Requirement text**: Saves with unknown event selectors or structurally invalid metadata shall fail export with machine-readable error.

## Implementation Tasks

### Pseudocode traceability
- Uses pseudocode lines: 390-410

### Files to modify

- `rust/src/campaign/save/export.rs` — Complete canonical export implementation using the Phase 05 frozen contract
  - marker: `@plan PLAN-20260314-CAMPAIGN.P14`
  - marker: `@requirement §10.1`
  - This phase must fill in derivation/execution logic without changing the Phase 05 schema, claim-family surface vocabulary, malformed-save error JSON shape, or verifier-report entry schema except for narrowly justified additive fields
  - `export_canonical_document(save_path: &Path) -> Result<String, ExportError>`
    - Read persisted save artifact
    - Validate: if scheduled events contain unknown selectors or invalid metadata, return error JSON
    - Derive all 8 sections from persisted state
    - Include `conformance_input_class` classification
    - Serialize to JSON
  - `ExportError` struct with `error_code: String`, `error_message: String`
  - Implement `produce_error_json(error: &ExportError) -> String`
    - Always produces `{"result": "error", "error_code": "...", "error_message": "..."}`
    - Must remain byte/field compatible with the contract frozen in P05
  - Section derivation functions:
    - `derive_save_summary(state: &GameStateBlob) -> SaveSummarySection`
      - Apply §9.2 normalization: quasispace remaps to hyperspace coords, starbase -> starbase:sol, etc.
    - `derive_resume_context(state: &GameStateBlob) -> ResumeContextSection`
      - `campaign_mode`: from closed set (hyperspace_navigation, interplanetary, encounter, starbase_visit, last_battle)
      - `starbase_context`: bool, true iff campaign_mode is starbase_visit
      - `navigation_identity`: coords or system identifier
      - `entry_routing_kind`: normal_interplanetary_entry or special_case_encounter_routing
      - `campaign_transition_marker`: from closed set (none, clear_start_interplanetary, preserve_start_encounter)
    - `derive_clock_state(state: &GameStateBlob) -> ClockStateSection`
    - `derive_scheduled_events(state: &GameStateBlob) -> Vec<ScheduledEventEntry>`
      - Per §8.6 minimum schema: selector, normalized_due_date, recurrence_kind, recurrence_parameters
      - Apply closed canonicalization test: only items whose omission would change comparison object
      - Respect any row-specific checkpoint-bundle or normalization rules delegated to `../specification.md` §8.6 without generalizing them beyond the named rows/event families
    - `derive_campaign_flags(state: &GameStateBlob) -> CampaignFlagsSection`
    - `derive_faction_state(state: &GameStateBlob) -> Vec<FactionEntry>`
      - 17 baseline factions with faction_id, strength, position, alliance, alive
    - `derive_encounter_state(state: &GameStateBlob) -> EncounterStateSection`
      - encounter_active, encounter_identity, npc_queue, escort_queue
  - `classify_input(state: &GameStateBlob) -> ConformanceInputClass`
    - `covered_mandatory`: save falls within covered contexts per §9.7
    - `diagnostic_only`: structurally decodable but outside mandatory acceptance sets
  - `select_inspection_surface(claim_family: ClaimFamily, context: CoveredContext, raw_sufficiency: RawInspectionSufficiency) -> InspectionSurface`
    - Executes the Phase 05 frozen no-mixing/surface-selection contract
    - Allows different claim families from the same save to use different surfaces
    - Handles legacy-starbase exception path explicitly
  - `build_verifier_report_entry(...) -> VerifierReportEntry`
    - Populate the Phase 05 frozen minimum fields: claim/context family, chosen inspection surface, claim-local result, overall covered-context result, adjunct dependency changed overall result, notes/artifacts used
  - `classify_load_export_outcome(...) -> LoadExportOutcomeClass`
    - Apply `../specification.md` §10.1 classification rules/examples instead of inventing a new taxonomy
  - `write_verifier_report(report_path: &Path, entries: &[VerifierReportEntry]) -> Result<(), ExportError>`
    - Emit machine-readable verifier-report artifact for evaluated claim/context families
  - Comprehensive tests:
    - Successful export for each covered context that requires export produces valid JSON with all 8 sections
    - All sections present even when inapplicable (null/empty)
    - Save-summary normalization correct for all covered contexts per §9.2 table
    - resume_context: campaign_mode and starbase_context never conflict
    - campaign_transition_marker is always a string token, never null
    - Scheduled events use catalog vocabulary selectors
    - Row-specific §8.6 normalization cases are handled exactly where required
    - Faction state includes all 17 baseline factions
    - Encounter state correctly reflects active/inactive encounters
    - Error export: unknown event selector produces error JSON, not partial success
    - Error JSON has result=error, error_code, error_message
    - conformance_input_class correctly classifies covered vs diagnostic
    - Deterministic: same input produces identical output
    - Verifier report entry distinguishes claim-local pass from overall covered-context fail when adjunct dependency fails
    - Claim-family surface selection forbids mixing raw-save and export facts within one claim family
    - Legacy-starbase exception path is selected only when its sufficiency condition is met

### Files to modify

- `rust/src/campaign/save/mod.rs`
  - Ensure `export` module fully accessible

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Export function produces valid JSON for contexts whose chosen inspection surface requires export
- [ ] All 8 sections present in successful exports
- [ ] Error JSON structure correct
- [ ] Claim-family surface-selection/reporting helpers implemented at the same verifier-facing boundary frozen in P05
- [ ] Concrete verifier-report artifact path/format is defined for later P16 evidence collection
- [ ] P14 executes the earlier contract instead of redefining it late

## Semantic Verification Checklist (Mandatory)
- [ ] Save-summary normalization matches §9.2 table exactly
- [ ] campaign_mode uses exact closed vocabulary
- [ ] starbase_context = true iff campaign_mode = starbase_visit
- [ ] campaign_transition_marker is always string from closed set, never null
- [ ] Scheduled events use catalog selector vocabulary (§8.6)
- [ ] Closed canonicalization test applied: only comparison-critical items surfaced
- [ ] Row-specific §8.6 normalization/checkpoint rules applied only where required
- [ ] Faction state covers all 17 baseline factions
- [ ] encounter_identity uses closed baseline encounter vocabulary
- [ ] npc_queue and escort_queue entries have race_id and ship_type_id
- [ ] Error export for malformed saves produces error JSON, no partial success
- [ ] conformance_input_class classification correct
- [ ] Claim-family inspection-surface selection follows the no-mixing rule
- [ ] Verifier report entry includes minimum fields required by `requirements.md`
- [ ] Export-success vs overall covered-context distinction is representable when adjunct artifacts matter
- [ ] Legacy-starbase observational exception path is explicitly represented
- [ ] Export is deterministic: identical input -> identical output
- [ ] No contract drift from the Phase 05 frozen export/report surface without explicit justified amendment

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/save/export.rs
```

## Success Criteria
- [ ] Canonical export document fully conforms to §10.1 where export is the chosen/required surface
- [ ] All covered contexts that require export produce correct exports
- [ ] Error handling produces correct error JSON
- [ ] Verifier-facing reporting and adjunct-sensitive pass/fail distinction are concretely supported
- [ ] P14 only fills in remaining derivation logic and does not force save/load rework through late contract changes

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/save/export.rs`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P14.md`
