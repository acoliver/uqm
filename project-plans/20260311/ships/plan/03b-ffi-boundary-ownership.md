# Phase 03.5: FFI Boundary & Ownership Contract

## Phase ID
`PLAN-20260314-SHIPS.P03.5`

## Prerequisites
- Required: Phase 03a (Core Types & Enums Verification) completed and PASS
- Expected files: `types.rs` with bridge-safe helper types and REQ naming normalized across the plan

## Purpose
Freeze the C/Rust integration boundary before loader, catalog, queue, runtime, and lifecycle phases depend on it. This phase defines the authoritative ABI, ownership, lifetime, and layout contracts for all cross-language ships-subsystem calls.

## Requirements Implemented (Expanded)

### Boundary Ownership Preservation
**Requirement text**: External systems decide which entries to create, when to enqueue them, which entry to choose next, and when to pass a chosen entry to spawn. The ships subsystem provides shared queue/fragment data contracts, helper operations, spawn behavior, and writeback.

Behavior contract:
- GIVEN: Existing C battle/setup/campaign callers
- WHEN: Rust ships is enabled
- THEN: Canonical queue/catalog/runtime ownership still matches the established subsystem boundary and no parallel authoritative queue state is introduced accidentally

### ABI Contract Completeness
**Requirement text**: The subsystem shall expose queue contracts, catalog lookups, loading, spawn, runtime callbacks, and teardown behavior at the established integration points.

Behavior contract:
- GIVEN: Any C↔Rust ships entrypoint
- WHEN: The plan references that entrypoint
- THEN: Exact signatures, pointer ownership, lifetime, and layout model are specified

## Implementation Tasks

### Files to create

- `project-plans/20260311/ships/plan/03b-ffi-boundary-ownership.md` — this phase document
  - marker: `@plan PLAN-20260314-SHIPS.P03.5`
  - marker: `@requirement REQ-QUEUE-MODEL, REQ-QUEUE-OWNER-BOUNDARY, REQ-SPAWN-ENTRYPOINT, REQ-CATALOG-LOOKUP, REQ-WRITEBACK-MATCHING`

- `rust/src/ships/ffi_contract.rs` — boundary contract reference module
  - marker: `@plan PLAN-20260314-SHIPS.P03.5`
  - Contents:
    - Bridge wrapper/type aliases for all ABI-facing types
    - Documentation comments recording canonical ownership and lifetime rules
    - No behavior yet; this is the source-of-truth contract consumed by later phases

### Contract decisions to record

#### 1. Canonical ownership model
- Canonical `STARSHIP`, `SHIP_FRAGMENT`, `FLEET_INFO`, side race queues, and any queue-order semantics remain C-owned.
- Rust helper functions operate on:
  - `*mut STARSHIP`, `*mut SHIP_FRAGMENT`, `*mut FLEET_INFO`, and queue/list handles that resolve to C-owned memory, or
  - explicit snapshot structs when metadata must be copied for stable storage.
- Rust-created descriptor instances (`RaceDesc`) are Rust-owned runtime objects whose handles are attached to C-owned `STARSHIP` entries.
- Rust master-catalog entries are metadata snapshots with explicit ownership of copied metadata resources; they do not borrow from temporary loader descriptors.

#### 2. Actual layout/ownership decision per ABI type

| ABI type | Layout model | Canonical owner | Allocates | Frees | Pointer validity window |
|----------|--------------|-----------------|-----------|-------|-------------------------|
| `MASTER_SHIP_INFO` | Shared layout via `#[repr(C)]`/bindgen-compatible definition | Rust catalog store | Rust catalog loader in `catalog.rs` | Rust catalog teardown in `free_master_ship_list()` | Returned pointers valid from successful `rust_load_master_ship_list()` until `rust_free_master_ship_list()` |
| `RACE_DESC` | Opaque Rust-owned allocation exposed as typed `*mut RACE_DESC` behind a stable FFI wrapper type | Rust loader/runtime | `rust_load_ship()` / `spawn_ship()` | `rust_free_ship()` / lifecycle teardown | Valid from successful load until explicit free or teardown-driven free; C must never free directly |
| `STARSHIP` | Shared C layout; Rust borrows typed `*mut STARSHIP` into canonical C-owned queue storage | C battle/setup/campaign owners | Existing C queue/build allocation path | Existing C queue/build teardown path | Valid under existing C queue lifetime rules; Rust may not retain beyond documented battle/queue use without an attached owner contract |
| `SHIP_FRAGMENT` | Shared C layout; Rust borrows typed `*mut SHIP_FRAGMENT` | C campaign/build owners | Existing C fragment allocation path | Existing C fragment teardown path | Valid under existing C fragment/queue lifetime rules |
| `FLEET_INFO` | Shared C layout; Rust borrows typed `*mut FLEET_INFO` | C campaign owners | Existing C campaign state creation path | Existing C campaign state teardown path | Valid while owning campaign structures remain alive |

- No ABI-facing function may use an untyped `usize` or `*const c_void` where a concrete C-facing typedef/struct pointer can be used instead.
- `MASTER_SHIP_INFO`, `STARSHIP`, `SHIP_FRAGMENT`, and `FLEET_INFO` are not converted to opaque handles in this plan; shared-layout access is the explicit decision.
- `RACE_DESC` remains typed at the C boundary for compatibility, but ownership is Rust-only and its interior layout is not a shared mutable contract for arbitrary C writes.

#### 3. Required FFI signature table
Specify exact C declaration, Rust export, ownership, lifetime, and layout for each of the following categories:

- Catalog:
  - `BOOLEAN rust_load_master_ship_list(void);`
  - `void rust_free_master_ship_list(void);`
  - `const MASTER_SHIP_INFO *rust_find_master_ship(SPECIES_ID species);`
  - `const MASTER_SHIP_INFO *rust_find_master_ship_by_index(COUNT index);`
  - `COUNT rust_get_ship_cost_from_index(COUNT index);`
  - icon/string accessors use typed fields on returned `MASTER_SHIP_INFO` or dedicated typed accessors; no `*const c_void`

- Queue/build:
  - `HSTARSHIP rust_build_ship(HSTARSHIP queue_head, SPECIES_ID species);`
  - `STARSHIP *rust_get_starship_from_index(HSTARSHIP queue_head, COUNT index);`
  - `BOOLEAN rust_clone_ship_fragment(const SHIP_FRAGMENT *src, SHIP_FRAGMENT *dst);`
  - `COUNT rust_add_escort_ships(FLEET_INFO *fleet, HSTARSHIP built_queue, COUNT max_build);`
  - `COUNT rust_count_escort_ships(const FLEET_INFO *fleet);`
  - `BOOLEAN rust_have_escort_ship(const FLEET_INFO *fleet, SPECIES_ID species);`
  - `void rust_set_race_allied(FLEET_INFO *fleet, BOOLEAN allied);`
  - `BOOLEAN rust_escort_feasibility_study(FLEET_INFO *fleet, COUNT crew_budget, COUNT fuel_budget);`
  - `void rust_start_sphere_tracking(FLEET_INFO *fleet);`

- Loader/runtime ownership:
  - `RACE_DESC *rust_load_ship(SPECIES_ID species, BOOLEAN battle_ready);`
  - `void rust_free_ship(RACE_DESC *desc, BOOLEAN free_battle, BOOLEAN free_metadata);`
  - returned `RACE_DESC *` is Rust-owned memory with lifetime ending at `rust_free_ship`; C must not free directly

- Spawn/lifecycle:
  - `BOOLEAN rust_spawn_ship(STARSHIP *starship);`
  - `COUNT rust_init_ships(void);`
  - `void rust_uninit_ships(void);`
  - callback trampolines with exact `ELEMENT *` / `STARSHIP *` argument types

- Runtime callbacks:
  - `void rust_ship_preprocess(ELEMENT *element);`
  - `void rust_ship_postprocess(ELEMENT *element);`
  - `void rust_ship_death(ELEMENT *element);`
  - if `STARSHIP *` is derived through `GetElementStarShip()`, document that rather than passing redundant opaque pointers

- SIS/campaign configuration bridge:
  - exact accessor set needed for module state / flagship configuration
  - ownership model for campaign data reads is borrow-only; no long-lived caching without an explicit invalidation plan

#### 4. Pointer lifetime rules
- Returned catalog pointers remain valid until `rust_free_master_ship_list()`.
- Returned queue-entry pointers refer to C-owned queue storage and remain valid under the same rules as the existing C queue API.
- Returned `RACE_DESC *` values remain valid until explicit free or successful attachment/cleanup during teardown.
- No FFI API may return a pointer to data behind a Rust `MutexGuard` or temporary stack frame.

#### 5. Mixed-path smoke-test contract
Add a minimal executable verification slice before implementation-heavy phases:
- link/load symbols with empty/no-op-safe implementations,
- invoke catalog load/free through C entrypoints,
- invoke a typed queue helper on real C-owned queue storage,
- verify callback trampoline registration can round-trip through the C battle element callback signature.

### Files to modify

- `rust/src/ships/mod.rs`
  - Add `pub mod ffi_contract;`

- `project-plans/20260311/ships/plan/00-overview.md`
  - already updated to insert this phase into sequencing and definition of done

### Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `ffi_contract.rs` exists and centralizes ABI-facing type aliases and docs
- [ ] Every planned C↔Rust entrypoint has an exact typed signature recorded
- [ ] Every planned returned pointer has explicit ownership/lifetime rules
- [ ] Queue/catalog/runtime canonical ownership is documented consistently
- [ ] SIS/module-state dependency surface is identified before race batches

## Semantic Verification Checklist (Mandatory)
- [ ] No planned FFI signature still relies on ambiguous `usize` or `*const c_void` for ABI-facing primary APIs
- [ ] Catalog lookups return stable pointers or a documented accessor-only model
- [ ] Queue helpers operate over canonical C-owned storage or explicitly documented adapter handles
- [ ] Spawn contract uses C-owned `STARSHIP` integration rather than Rust-only queue objects
- [ ] Callback registration path is documented against actual `ELEMENT` callback signatures
- [ ] Mixed C/Rust smoke-test slice is defined for use in P05/P08/P09
- [ ] No placeholder/deferred implementation patterns remain in the contract module

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/ffi_contract.rs
```

## Success Criteria
- [ ] ABI contract is explicit enough that P05-P10 can be implemented without re-deciding ownership/layout
- [ ] All later FFI-facing phases can cite this contract instead of inventing signatures ad hoc
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/ffi_contract.rs`

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P03.5.md`
