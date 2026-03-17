# Phase 04: Campaign Event Catalog & Handlers

## Phase ID
`PLAN-20260314-CAMPAIGN.P04`

## Prerequisites
- Required: Phase 03a completed
- Required: Phase 03.5a completed
- Expected files: `rust/src/campaign/activity.rs`, `types.rs`, `session.rs`
- Dependency: `rust/src/time/events.rs` — event scheduling API
- Dependency: `rust/src/state/game_state.rs` — game-state bit access

## Requirements Implemented (Expanded)

### Event Progression (from requirements.md)
**Requirement text**: When a new game starts, the subsystem shall schedule the initial set of campaign events. When a scheduled campaign event becomes due, the subsystem shall produce the defined campaign progression effects.

**Requirement text**: The subsystem shall support the complete campaign event catalog defined in specification §8.6 at the selector-vocabulary and externally visible outcome-family level.

### Initial Event Registration (§8.2)
Behavior contract:
- GIVEN: A new game starting
- WHEN: `add_initial_game_events()` is called
- THEN: HYPERSPACE_ENCOUNTER_EVENT (relative 0/1/0), ARILOU_ENTRANCE_EVENT (absolute month 3 day 17), KOHR_AH_VICTORIOUS_EVENT (relative 0/0/victory_years), SLYLANDRO_RAMP_UP (immediate) are scheduled

### Event Handler Effects (§8.3, §8.6)
Behavior contract:
- GIVEN: A scheduled event becomes due
- WHEN: `event_handler(selector)` is called
- THEN: The event produces its defined campaign progression effects (story flags, faction changes, follow-on scheduling)

## Implementation Tasks

### Pseudocode traceability
- Uses pseudocode lines: 220-260

### Files to create

- `rust/src/campaign/events/mod.rs` — Event module root
  - marker: `@plan PLAN-20260314-CAMPAIGN.P04`
  - `EventSelector` enum with 18 variants (indices 0-17) matching §8.6 catalog
  - `impl EventSelector`: `from_index(u8) -> Option<Self>`, `to_index(&self) -> u8`, `name(&self) -> &'static str`
  - `is_valid_selector(index: u8) -> bool`
  - Serde serialization to canonical selector strings
  - Unit tests for selector round-trip, name mapping, and validation

- `rust/src/campaign/events/registration.rs` — Initial event registration
  - marker: `@plan PLAN-20260314-CAMPAIGN.P04`
  - marker: `@requirement §8.2`
  - `add_initial_game_events()` — schedules 4 initial events via clock API
  - Constants: `HYPERSPACE_ENCOUNTER_RELATIVE_DAYS = 1`, `ARILOU_ENTRANCE_MONTH = 3`, `ARILOU_ENTRANCE_DAY = 17`, `SLYLANDRO_RAMP_INTERVAL_DAYS = 182`
  - Unit tests verifying correct scheduling calls

- `rust/src/campaign/events/handlers.rs` — Event handler implementations
  - marker: `@plan PLAN-20260314-CAMPAIGN.P04`
  - marker: `@requirement §8.3, §8.6`
  - `dispatch_event(selector: EventSelector, session: &mut CampaignSession)` — main dispatch
  - Individual handler functions for all 18 events:
    - `handle_arilou_entrance()` — set portal open, schedule exit in 3 days
    - `handle_arilou_exit()` — set portal closed, schedule entrance on day 17 next month
    - `handle_hyperspace_encounter()` — advance fleets, check encounter gen, reschedule 1 day
    - `handle_kohr_ah_victorious()` — conditional: delay genocide or initiate directly
    - `handle_advance_pkunk()` — fleet movement toward/away from Yehat, conditional reschedule
    - `handle_advance_thraddash()` — arc stages, strength reduction, conditional Ilwrath scheduling
    - `handle_zoqfot_distress()` — set distress flag or defer 7 days if at homeworld
    - `handle_zoqfot_death()` — eliminate faction or defer 7 days
    - `handle_shofixti_return()` — set allied, reduce crew cost, reset counters
    - `handle_advance_utwig_supox()` — counter-mission arc stages, strength reduction
    - `handle_kohr_ah_genocide()` — target nearest faction, eliminate, cascade or defer
    - `handle_spathi_shield()` — remove from alliance, zero strength, or defer
    - `handle_advance_ilwrath()` — war arc stages, mutual combat, elimination
    - `handle_advance_mycon()` — deploy to Organon, attrition, return home
    - `handle_arilou_umgah_check()` — set completion flag
    - `handle_yehat_rebellion()` — split faction: 2/3 royalist, rebel faction created
    - `handle_slylandro_ramp_up()` — increment multiplier up to cap 4, reschedule 182 days
    - `handle_slylandro_ramp_down()` — decrement multiplier, reschedule 23 days if above zero
  - All handlers interact with session state and game-state bits via trait/API
  - Comprehensive unit tests per handler verifying:
    - Correct story-flag updates
    - Correct faction strength/position changes
    - Correct follow-on event scheduling
    - Correct deferral behavior when player at homeworld
    - Edge cases (cap reached, faction already dead, etc.)

### Files to modify

- `rust/src/campaign/mod.rs`
  - Add `pub mod events;`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `events/mod.rs` created with `EventSelector` enum (18 variants)
- [ ] `events/registration.rs` created with `add_initial_game_events()`
- [ ] `events/handlers.rs` created with all 18 handler functions
- [ ] Module wired into `campaign/mod.rs`

## Semantic Verification Checklist (Mandatory)
- [ ] EventSelector enum indices 0-17 match §8.6 catalog order
- [ ] Initial registration schedules exactly the 4 events from §8.2
- [ ] Each handler produces the documented effects from §8.6
- [ ] Deferral events (Zoqfot, Spathi, Genocide) correctly check player location
- [ ] Self-rescheduling events (Hyperspace Encounter, Slylandro, Arilou cycle) create correct follow-on
- [ ] Genocide handler correctly targets nearest surviving faction with Druuge tiebreaker
- [ ] Yehat rebellion correctly splits to 2/3 royalist strength
- [ ] Tests verify behavior, not only internals

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/events/
```

## Success Criteria
- [ ] All 18 event handlers implemented and tested
- [ ] Event catalog serialization matches canonical selector strings
- [ ] Registration and handler tests pass

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/events/`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P04.md`
