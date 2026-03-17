# Phase 05: Save Summary & Export Types

## Phase ID
`PLAN-20260314-CAMPAIGN.P05`

## Prerequisites
- Required: Phase 04a completed
- Expected files: campaign types, events module

## Requirements Implemented (Expanded)

### Save Summary Derivation (§9.2)
**Requirement text**: When a save is requested, the subsystem shall write a save summary derived from the current campaign state, with appropriate remapping for special contexts.

Behavior contract:
- GIVEN: Campaign in quasispace navigation
- WHEN: `prepare_summary()` is called
- THEN: `summary_type = "hyperspace"`, `location_id = "hyperspace:<remapped_x>,<remapped_y>"`

- GIVEN: Campaign at starbase
- WHEN: `prepare_summary()` is called
- THEN: `summary_type = "starbase"`, `location_id = "starbase:sol"`

### Campaign Canonical Export Document Types (§10.1)
**Requirement text**: The subsystem shall provide machine-readable canonical export with required sections.

Behavior contract:
- GIVEN: A valid save artifact
- WHEN: Canonical export is requested
- THEN: JSON document with all 8 required sections produced

### Inspection-Surface / Verifier Contract Freeze (`requirements.md`, specification §10.1)
**Requirement text**: Export-based inspection applicability, claim-family surface selection, malformed-save error/result shape, and verifier reporting are normative conformance obligations, not late optional polish.

Behavior contract:
- GIVEN: A claim family and covered context
- WHEN: verifier-facing comparison is designed or implemented in later phases
- THEN: it must use the frozen Phase 05 contract for section schema, chosen inspection surface, no-mixing rule, malformed-save error shape, and report-entry fields

## Implementation Tasks

### Pseudocode traceability
- Uses pseudocode lines: 286-298, 390-410

### Files to create

- `rust/src/campaign/save/mod.rs` — Save/load module root
  - marker: `@plan PLAN-20260314-CAMPAIGN.P05`

- `rust/src/campaign/save/summary.rs` — Save summary derivation
  - marker: `@plan PLAN-20260314-CAMPAIGN.P05`
  - marker: `@requirement §9.2`
  - `SaveSummary` struct: `summary_type: SummaryType`, `location_id: String`, `date: CampaignDate`
  - `SummaryType` enum: `Hyperspace`, `Interplanetary`, `Starbase`, `Encounter`, `LastBattle`
  - `prepare_summary(session: &CampaignSession) -> SaveSummary`
  - Context remapping logic:
    - Quasispace -> `hyperspace` with remapped coordinates
    - Starbase -> `starbase:sol`
    - Planet orbit -> `interplanetary` with system coords
    - Last battle -> `last_battle:sa_matra`
    - Encounter -> `encounter:<identity>`
    - Post-encounter -> remapped to recoverable mode
  - `impl Display` for canonical normalized strings
  - Comprehensive tests for every covered context from §9.7

- `rust/src/campaign/save/export.rs` — Export document types and frozen verifier-facing contract
  - marker: `@plan PLAN-20260314-CAMPAIGN.P05`
  - marker: `@requirement §10.1`
  - `ExportDocument` struct with Serde derives:
    - `schema_version: String`
    - `result: String` ("success" or "error")
    - `conformance_input_class: String` ("covered_mandatory" or "diagnostic_only")
    - `save_summary: ExportSaveSummary`
    - `resume_context: ExportResumeContext`
    - `clock_state: ExportClockState`
    - `scheduled_events: Vec<ExportScheduledEvent>`
    - `campaign_flags: serde_json::Value` (flexible flag map)
    - `faction_state: Vec<ExportFactionEntry>`
    - `encounter_state: ExportEncounterState`
  - `ExportSaveSummary`: `summary_type`, `location_id`, `date`
  - `ExportResumeContext`: `campaign_mode`, `starbase_context`, `navigation_identity`, `entry_routing_kind`, `campaign_transition_marker`
  - `ExportClockState`: `year`, `month`, `day`, `tick_state`
  - `ExportScheduledEvent`: `selector`, `normalized_due_date`, `recurrence_kind`, `recurrence_parameters`
  - `ExportFactionEntry`: `faction_id`, `strength`, `position`, `alliance`, `alive`
  - `ExportEncounterState`: `encounter_active`, `encounter_identity`, `npc_queue`, `escort_queue`
  - `ExportError` struct: `result: "error"`, `error_code`, `error_message`
  - `ClaimFamily` enum and `InspectionSurface` enum to freeze claim-family surface selection vocabulary early
  - `VerifierReportEntry` struct with minimum stable fields: claim/context family, chosen inspection surface, claim-local result, overall covered-context result, adjunct dependency changed overall result, notes/artifacts used
  - `LoadExportOutcomeClass` / equivalent closed-set outcome vocabulary required by specification §10.1
  - Canonical vocabulary constants for all closed-set fields
  - `select_inspection_surface(...)` helper stubbed as a finalized contract boundary even if full derivation logic is filled in later
  - `produce_error_json(error: &ExportError) -> String` defining the canonical malformed-save/export-failure JSON shape used by P07 and later phases
  - Serialization tests verifying exact JSON field names and structure
  - Tests for error document shape
  - Tests that verifier-report entries serialize with the required minimum fields and closed vocabulary

### Files to modify

- `rust/src/campaign/mod.rs`
  - Add `pub mod save;`

- `rust/Cargo.toml`
  - Ensure `serde` and `serde_json` dependencies present

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `save/mod.rs`, `save/summary.rs`, `save/export.rs` created
- [ ] Module wired into `campaign/mod.rs`
- [ ] Serde dependencies available
- [ ] Phase 05 freezes canonical export schema, malformed-save error JSON shape, claim-family surface-selection vocabulary, and verifier-report entry schema for downstream phases

## Semantic Verification Checklist (Mandatory)
- [ ] All 9 covered contexts from §9.7 summary-normalization table produce correct `summary_type` and `location_id`
- [ ] Quasispace remapping uses hyperspace-equivalent coordinates
- [ ] Post-encounter summary uses recoverable mode, not "encounter"
- [ ] ExportDocument JSON serialization produces all 8 required sections
- [ ] Always-present sections appear even when inapplicable (null/empty)
- [ ] Error document shape is distinguishable from success (has `result: "error"`)
- [ ] `campaign_mode` vocabulary matches closed set: `hyperspace_navigation`, `interplanetary`, `encounter`, `starbase_visit`, `last_battle`
- [ ] `campaign_transition_marker` is always a string token, never null
- [ ] Claim-family inspection-surface vocabulary is finalized before P06/P07 and forbids same-claim-family raw-save/export mixing by contract
- [ ] Verifier-report entry schema includes claim-local result, overall covered-context result, and adjunct-sensitivity fields before save/load implementation depends on it

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/save/
```

## Success Criteria
- [ ] Summary derivation correct for all covered contexts
- [ ] Export types and verifier-facing contract serialize to valid JSON matching §10.1 schema
- [ ] Downstream save/load phases can depend on a frozen export/report contract instead of redefining it

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/save/`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P05.md`
