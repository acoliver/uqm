# Phase 03.5: C-State Accessor Bridge & Ownership Model

## Phase ID
`PLAN-20260314-CAMPAIGN.P03.5`

## Prerequisites
- Required: Phase 03a completed
- Expected files: `rust/src/campaign/activity.rs`, `types.rs`, `session.rs`
- Dependency: validated seam inventory from P01

## Requirements Implemented (Expanded)

### Boundary Ownership Model (§2.2, §3.2)
**Requirement text**: Campaign gameplay owns campaign semantics, but lower boundaries retain ownership of their own persisted-data validation, queue/storage representations, and restoration logic where specified. The implementation shall establish explicit read/write ownership rules before campaign orchestration mutates those boundaries.

Behavior contract:
- GIVEN: Activity globals, game-state markers, and queue/state structures still resident in C-owned storage
- WHEN: Rust campaign logic needs to inspect or mutate them
- THEN: The access path is explicit, validated, and preserves source-of-truth ownership without hidden copies or ambiguous synchronization

### Load Safe-Failure Seam Support (§9.4.0b)
**Requirement text**: Cross-boundary restore failures shall fail safely without partial application.

Behavior contract:
- GIVEN: A load path that needs campaign-required adjunct files or lower-boundary restores
- WHEN: Any required dependency fails
- THEN: The accessor/bridge seam exposes rollback-safe commit boundaries and mutation ordering sufficient for safe failure

## Implementation Tasks

### Files to create

- `rust/src/campaign/state_bridge.rs` — validated C-state bridge and ownership helpers
  - marker: `@plan PLAN-20260314-CAMPAIGN.P03.5`
  - marker: `@requirement §2.2, §3.2, §9.4.0b`
  - `ActivityStateBridge` for validated access to `CurrentActivity`, `NextActivity`, `LastActivity` or equivalent owning seam
  - `CampaignQueueBridge` for escort, NPC, encounter, and other required queue readers/writers
  - `StarbaseMarkerBridge` for starbase-context marker read/write
  - `AdjunctDependencyClassifier` helper or equivalent mapping layer for covered-context adjunct needs
  - `CampaignStateSnapshot` or equivalent rollback-safe snapshot for in-session load attempts
  - Accessor methods separated by ownership mode:
    - read-only inspection
    - staged mutation
    - commit/apply mutation
    - rollback/restore pre-load state
  - Rust-side wrapper types documenting whether each seam is `read_only`, `write_through`, `staged_commit`, or `leave_in_c`
  - Unit tests for bridge invariants and snapshot/rollback behavior

- `sc2/src/uqm/campaign_state_bridge.h` — C declarations for validated accessor seams
  - marker: `@plan PLAN-20260314-CAMPAIGN.P03.5`
  - Only accessor/helper APIs proven necessary by P01 seam inventory
  - No speculative exported helpers

- `sc2/src/uqm/campaign_state_bridge.c` — thin accessor implementations
  - marker: `@plan PLAN-20260314-CAMPAIGN.P03.5`
  - Read/write wrappers around validated globals/queues/state markers
  - No campaign policy logic beyond bridge adaptation

### Files to modify

- `rust/src/campaign/mod.rs`
  - Add `pub mod state_bridge;`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `state_bridge.rs` created with explicit ownership-mode wrappers
- [ ] `campaign_state_bridge.h` / `.c` created with only validated accessor seams
- [ ] Module wired into `campaign/mod.rs`
- [ ] No speculative accessors added without seam-inventory backing

## Semantic Verification Checklist (Mandatory)
- [ ] Activity-global ownership is explicit and documented
- [ ] Queue ownership and synchronization strategy are explicit and documented
- [ ] Starbase marker access strategy is explicit and documented
- [ ] Safe-failure snapshot/rollback seam exists for in-session load attempts
- [ ] Adjunct dependency classification is available to later load/export verification phases
- [ ] No campaign implementation phase after this needs to guess whether a field is Rust-owned or C-owned

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/state_bridge.rs sc2/src/uqm/campaign_state_bridge.c sc2/src/uqm/campaign_state_bridge.h
```

## Success Criteria
- [ ] Later phases can use validated bridge APIs instead of direct assumptions about C-owned globals/queues
- [ ] Ownership and rollback model are settled before event/load/loop orchestration depends on them

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/state_bridge.rs sc2/src/uqm/campaign_state_bridge.h sc2/src/uqm/campaign_state_bridge.c`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P03.5.md`
