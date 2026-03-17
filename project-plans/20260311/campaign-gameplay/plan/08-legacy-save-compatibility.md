# Phase 08: Legacy Save Compatibility

## Phase ID
`PLAN-20260314-CAMPAIGN.P08`

## Prerequisites
- Required: Phase 07a completed
- Expected files: `save/deserialize.rs`, `save/validation.rs`

## Requirements Implemented (Expanded)

### Legacy-to-End-State Load (§10.1, from requirements.md)
**Requirement text**: The subsystem shall load valid legacy campaign saves produced by the legacy baseline implementation for campaign save contexts listed as covered in specification §9.7. Semantic equivalence of restored state satisfies this requirement; byte-for-byte identity is not required.

Behavior contract:
- GIVEN: A save file produced by the legacy C implementation in a covered context
- WHEN: The Rust load path attempts to load it
- THEN: The campaign resumes with semantically equivalent state in the correct mode

### Valid Legacy Campaign Save Definition (from requirements.md Key observable definitions)
A valid legacy save must be:
1. Produced to completion by the legacy baseline
2. Structurally complete and not truncated
3. Conforms to legacy persistence format
4. All event selectors are in the campaign event catalog
5. All scheduled-event records can be decoded

### Legacy-Starbase Observational Exception
**Requirement text**: For covered valid legacy starbase save/load claims only, if the legacy raw save does not sufficiently expose the closed starbase comparison object, verification may require observation-based controlled comparison at the starbase post-load conformance observation point.

### End-State Round-Trip (§10.1)
**Requirement text**: A campaign save produced by the end-state implementation shall be loadable by the same implementation, with equivalent resumed behavior.

## Implementation Tasks

### Files to create

- `rust/src/campaign/save/legacy.rs` — Legacy save format reader
  - marker: `@plan PLAN-20260314-CAMPAIGN.P08`
  - marker: `@requirement §10.1`
  - `detect_save_format(data: &[u8]) -> SaveFormat` — determine if legacy or end-state format
  - `SaveFormat` enum: `LegacyC`, `EndState`
  - `read_legacy_summary(reader: &mut impl Read) -> Result<SaveSummary, LoadError>`
  - `read_legacy_game_state(reader: &mut impl Read) -> Result<GameStateBlob, LoadError>`
  - `read_legacy_queue_data(reader: &mut impl Read) -> Result<Vec<QueueEntry>, LoadError>`
  - Legacy field mapping grounded to validated source declarations:
    - `CurrentActivity` byte offset and flag extraction
    - `GameClock` field offsets (day, month, year, tick)
    - Autopilot target coordinates
    - Validated ship-state persistence field offsets
    - Orbit flags
    - Game-state bitfield byte range and bit extraction
    - `GLOBAL_FLAGS_AND_DATA` starbase-context marker location
  - Endianness handling for legacy format (little-endian on disk)
  - `map_legacy_activity_to_campaign_mode(activity_byte: u8) -> CampaignActivity`
  - `map_legacy_encounter_race(race_index: u8) -> Option<EncounterIdentity>`
  - `legacy_raw_save_sufficiency(context: CoveredContext) -> RawInspectionSufficiency`
    - Determines whether raw-save inspection is sufficient for that claim/context family or whether the legacy-starbase observational exception path is required
  - Comprehensive tests:
    - Parse known-good legacy save bytes from `rust/tests/fixtures/campaign/legacy/`
    - Verify correct field extraction for all §9.1 fields
    - Verify correct activity-to-mode mapping for all covered contexts
    - Verify starbase context detection from `GLOBAL_FLAGS_AND_DATA`
    - Verify encounter identity mapping from legacy race indices
    - Verify round-trip: end-state save -> load -> end-state save produces equivalent content
    - Reject truncated legacy saves
    - Reject legacy saves with unknown event selectors (validation applies)

### Files to modify

- `rust/src/campaign/save/mod.rs`
  - Add `pub mod legacy;`

- `rust/src/campaign/save/deserialize.rs`
  - Integrate format detection: call `detect_save_format()` to choose legacy or end-state reader
  - Route to `read_legacy_*` functions when legacy format detected

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `save/legacy.rs` created with format detection and legacy readers
- [ ] Deserialize module updated to support both formats
- [ ] Legacy field offsets documented and tested
- [ ] Legacy fixtures stored under `rust/tests/fixtures/campaign/legacy/`
- [ ] Curated legacy fixture inventory is concrete enough to cover each covered context required by §9.7

## Semantic Verification Checklist (Mandatory)
- [ ] Legacy format detection correctly distinguishes legacy from end-state saves
- [ ] Legacy `CurrentActivity` maps to correct `CampaignActivity` for all covered contexts
- [ ] Legacy starbase context detected from `GLOBAL_FLAGS_AND_DATA` marker
- [ ] Legacy encounter race indices map to correct `EncounterIdentity` values
- [ ] Legacy clock fields (day/month/year/tick) correctly extracted and converted
- [ ] Legacy queue data (escort, NPC, encounter) correctly parsed
- [ ] §9.4.1 validation applies equally to legacy saves (unknown selectors rejected)
- [ ] End-state round-trip: save -> load -> save produces semantically equivalent content
- [ ] Truncated or corrupt legacy saves rejected with safe-failure guarantees
- [ ] Legacy-starbase observational exception applicability is explicitly classified, not improvised ad hoc

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/save/legacy.rs
```

## Success Criteria
- [ ] Legacy saves load successfully with semantic equivalence
- [ ] End-state round-trip works for all covered contexts
- [ ] Format detection is reliable
- [ ] Legacy-starbase fallback verification path is concretely prepared when raw-save inspection is insufficient

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/save/legacy.rs rust/src/campaign/save/deserialize.rs`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P08.md`
