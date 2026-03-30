# Ships Remediation — Requirements

## Context

The previous ships/battle/battlept2 plans produced:
- Ship catalog, melee serialization, fleet values — **working in Rust**
- Per-species Rust files (28 files in `rust/src/ships/races/`) — **stubs only** (descriptor templates + trivial no-op behaviors)
- Battle engine infrastructure (`rust/src/battle/`) — **12K lines of Rust** with display list, element system, velocity, collision, weapon basics
- Ship runtime (`ships/runtime.rs`) — preprocess/postprocess framework, thrust, energy, collision
- `spawn_ship` — loads descriptor but **never creates an ELEMENT** (`_element_config` unused)
- `InitShips` — doesn't set up display list, spawn asteroids/planets
- `TemplateOnlyShip` used for all 28 species — all behavior methods are no-ops

The C code being replaced lives in `sc2/src/uqm/ships/*/` (28 directories, ~14K lines total)
and `sc2/src/uqm/ship.c` (spawn, preprocess, postprocess, death, collision).

## Functional Requirements

### REQ-SPAWN: spawn_ship creates battle elements
- `spawn_ship` must call `AllocElement`/`InsertElement` (via existing `battle::process_loop::alloc_element` or C FFI)
- Must set ELEMENT fields: playerNr, mass_points, state_flags (APPEARING|PLAYER_SHIP|IGNORE_SIMILAR), image frame, position, preprocess/postprocess/death/collision callbacks
- Must handle Sa-Matra special positioning (IN_LAST_BATTLE)
- Must handle HyperSpace facing
- Must handle random spawn position with gravity/collision avoidance

### REQ-INIT: InitShips sets up battle arena
- Must call InitSpace, InitDisplayList, InitGalaxy
- Must set graphics contexts (StatusContext, SpaceContext)
- Must spawn asteroids (5) and planet (1) for normal battles
- Must handle HyperSpace mode (BuildSIS, LoadHyperspace)
- Must handle IN_LAST_BATTLE (free_gravity_well instead of asteroids)

### REQ-UNINIT: UninitShips tears down properly
- Must count floating crew, award to surviving ship
- Must free ship descriptors, write back crew levels
- Must clear CurrentActivity IN_BATTLE flag
- Must handle encounter vs non-encounter cleanup

### REQ-BEHAVIOR: All 28 ship species have real combat behaviors

Each ship needs a `ShipBehavior` implementation with:
- `preprocess`: ship-specific per-frame logic (special movement, cloaking, form-switching, etc.)
- `postprocess`: ship-specific post-physics logic (special abilities triggered after weapon/special processing)
- `init_weapon`: weapon spawning (missiles, lasers, projectiles) with correct parameters
- `intelligence`: AI logic — when to thrust, turn, fire, use special
- `uninit`: cleanup (most ships: no-op)

Ships also spawn sub-elements (projectiles, fighters, mines, etc.) that have their own preprocess/collision callbacks.

### REQ-WEAPON-ELEMENTS: Projectiles and sub-elements work
- Missiles need preprocess callbacks (tracking, acceleration)
- Lasers need initialize_laser equivalent
- Sub-elements (fighters, mines, DOGIs, etc.) need full lifecycle
- Collision callbacks for all weapon types
- Blast/explosion effects on weapon death

### REQ-AI: Ship AI produces correct combat behavior
- Each ship's `intelligence` must match C's behavior
- Must interact with `ship_intelligence()` framework (EVALUATE_DESC, ObjectsOfConcern)
- Must set correct input flags (THRUST, LEFT, RIGHT, WEAPON, SPECIAL)

### REQ-PARITY: Behavior matches C implementation
- Ship characteristics (crew, energy, mass, costs, waits) must match C values exactly
- Weapon parameters (damage, speed, life, tracking) must match C values exactly
- AI decision-making must produce equivalent results
- Point defense, cloaking, form-switching, resurrection, etc. must all work

## Non-Functional Requirements

### REQ-COMPILE: All code compiles
- `cargo check` must pass after each phase
- `cargo test` must pass after each phase

### REQ-NO-C-BRIDGES: No new C delegation
- Ship behaviors must be pure Rust, not wrappers around C function pointers
- Battle engine APIs the ships call (AllocElement, SetVelocityVector, etc.) may use C FFI until those subsystems are ported, but the ship logic itself must be Rust

### REQ-INCREMENTAL: Ships can be ported incrementally  
- Each species is independent — porting one ship must not break others
- `TemplateOnlyShip` remains available as fallback for unported species during development
- The registry must dispatch to the real implementation when available, fall back to template when not
