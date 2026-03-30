# Ships Remediation — Specification

## Current State Inventory

### What exists and works (keep):
- `ships/types.rs` — RaceDesc, Starship, ShipInfo, Characteristics, ShipData, etc.
- `ships/traits.rs` — ShipBehavior trait with preprocess/postprocess/init_weapon/intelligence/uninit
- `ships/registry.rs` — descriptor_template_for_species (all 28 species have correct static data)
- `ships/loader.rs` — Two-tier loader (MetadataOnly / BattleReady) with resource loading via c_bridge
- `ships/lifecycle.rs` — spawn_ship (partial), init_ships (partial), uninit_ships (partial)
- `ships/runtime.rs` — ship_preprocess framework, ship_postprocess framework, delta_energy, inertial_thrust, animation_preprocess, build_ship_state
- `ships/races/*.rs` — 28 per-species files with descriptor templates, stub behaviors
- `ships/ffi.rs` — rust_ships_spawn, rust_ships_preprocess/postprocess/death with C↔Rust marshalling
- `battle/` module — 12K lines: display_list, element, velocity, collision, weapon, process_loop, ai, c_bridge

### What's broken/missing:
1. `spawn_ship` never calls AllocElement/InsertElement — `_element_config` is dead code
2. `InitShips`/`UninitShips` don't call C display/arena setup
3. All 28 species use `TemplateOnlyShip` (no-op behaviors) — zero combat code
4. `init_weapon` implementations return trivial `Vec<WeaponElement>` that don't create real ELEMENT objects
5. Weapon sub-elements (missiles, lasers) need their own preprocess/collision callbacks
6. No bridge for `initialize_missile`/`initialize_laser` — the two primary weapon creation functions
7. Ship AI is stub (`StatusFlags::THRUST` or empty) — no real decision-making

## Architecture

### Ship behavior flow (when complete):

```
C ship.c::ship_preprocess(ELEMENT*)
  → rust_ships_preprocess(ELEMENT*)          [ffi.rs — already exists]
    → extract_starship_from_element           [ffi.rs — already exists]
    → borrow_starship_from_c                  [ffi.rs — already exists]
    → ships::runtime::ship_preprocess         [runtime.rs — already exists]
      → behavior.preprocess(ship_state, ctx)  [per-species — NEEDS IMPL]
    → writeback_starship                      [ffi.rs — already exists]

C ship.c::ship_postprocess(ELEMENT*)
  → rust_ships_postprocess(ELEMENT*)
    → ships::runtime::ship_postprocess
      → behavior.init_weapon(...)             [per-species — NEEDS IMPL]
      → behavior.postprocess(...)             [per-species — NEEDS IMPL]
```

### Weapon/sub-element creation:
Ships create projectiles by calling `initialize_missile` or `initialize_laser` (C functions).
These allocate ELEMENTs and configure them. Ship Rust code calls these via FFI.

The projectile's preprocess/collision callbacks can be:
- C functions (for now — these are simple and shared across ships)
- Eventually Rust functions registered as C-callable extern "C" fn

### Battle engine API layer:
Ships call ~20 battle engine functions. These are currently C. Ships call them via FFI:

| Function | Purpose | Already in Rust? |
|----------|---------|-----------------|
| AllocElement | Create display element | process_loop.rs (Rust-side only) |
| PutElement | Add element to display | No — needs C FFI |
| LockElement/UnlockElement | Access element data | No — needs C FFI |
| SetVelocityVector | Set element velocity | velocity.rs (Rust) + C FFI |
| SetAbsFrameIndex | Set animation frame | c_bridge.rs (C FFI) |
| ProcessSound | Play sound effect | No — needs C FFI |
| DeltaEnergy | Change ship energy | runtime.rs (Rust) |
| initialize_missile | Create missile element | No — needs C FFI |
| initialize_laser | Create laser element | No — needs C FFI |
| TrackShip | Homing missile tracking | weapon.rs (Rust) |
| ship_intelligence | Base AI framework | No — needs C FFI |
| GetElementStarShip | Get starship from element | No — needs C FFI |
| DISPLAY_TO_WORLD | Coordinate conversion | Macro — port as const fn |
| NORMALIZE_FACING | Facing normalization | Macro — port as const fn |
| TFB_Random | Random number | No — needs C FFI |

## Phases

### Phase 0: Battle engine FFI bridge (prerequisite)
Port or create FFI bridges for the ~15 battle engine functions ships need to call.
Many already exist in `battle/c_bridge.rs`. Add the missing ones.

### Phase 1: Fix spawn_ship and InitShips/UninitShips
Wire `_element_config` to actually create elements. Call C arena setup.
Re-enable USE_RUST_SHIPS guards in ship.c and init.c.

### Phase 2: Weapon creation bridge
Create Rust wrappers for `initialize_missile` and `initialize_laser`.
These return element handles that ships use.

### Phase 3: Port ship behaviors — small ships first (6 ships, <350 lines C each)
- probe (118L) — simplest, no weapons
- syreen (284L) — song special
- supox (288L) — thrust-steering
- spathi (301L) — forward + rear weapon
- arilou (303L) — teleport special  
- druuge (324L) — recoil cannon, crew-burn special

### Phase 4: Port ship behaviors — medium ships (13 ships, 350-550 lines C each)
- human (360L) — nuke + point-defense laser
- yehat (369L) — pulse cannon + shield
- mycon (376L) — homing plasmoid + regeneration
- zoqfotpik (377L) — projectile + tongue attack
- utwig (380L) — energy absorb shield
- thradd (400L) — afterburner trail
- vux (398L) — limpet + warp-in-close
- ilwrath (409L) — flame + cloak
- slylandr (438L) — omni-directional bolt + inertia-less
- umgah (434L) — antimatter cone + backstep
- shofixti (521L) — projectile + glory device
- mmrnmhrm (527L) — X-form transformer
- androsyn (528L) — molecular acid + blazer form

### Phase 5: Port ship behaviors — large/complex ships (9 ships, 550+ lines C each)
- urquan (554L) — fusion blast + fighters
- blackurq (567L) — spinning blade + F.R.I.E.D.
- chenjesu (588L) — crystal shard + DOGI  
- pkunk (640L) — projectile + insults + resurrection
- melnorme (658L) — charge shot + confusion pulse
- chmmr (790L) — laser + zapsats
- lastbat (926L) — Sa-Matra boss
- sis_ship (1002L) — modular flagship
- orz (1083L) — howitzer + marines + intradimensional

### Phase 6: Re-enable Rust paths in ship.c/init.c
- Remove C fallbacks added during debugging
- Restore USE_RUST_SHIPS guards
- End-to-end verification: super melee battle works entirely through Rust

## Ship Porting Pattern (per species)

For each C ship file `sc2/src/uqm/ships/SPECIES/SPECIES.c`:

1. Read the C source and identify:
   - Constants (offsets, speeds, damage, waits)
   - `preprocess_func` — what the ship does each frame
   - `postprocess_func` — what happens after physics
   - `init_weapon_func` — how weapons are created (MissileBlock/LaserBlock)
   - `intelligence_func` — AI decision logic
   - Sub-element callbacks (weapon preprocess, collision, death)

2. Implement in `rust/src/ships/races/SPECIES.rs`:
   - Constants as `const` values
   - `ShipBehavior::preprocess` with equivalent logic
   - `ShipBehavior::postprocess` with equivalent logic
   - `ShipBehavior::init_weapon` creating proper weapon elements
   - `ShipBehavior::intelligence` with equivalent AI
   - Sub-element types and their callbacks

3. Update `ships/registry.rs`:
   - Replace `TemplateOnlyShip` with `SpeciesShip` for that species

4. Test:
   - Unit tests for weapon parameters, AI decisions
   - Descriptor template values match C
   - Compile check

## Complexity Estimate

| Phase | Ships | C Lines | Estimated Rust Lines |
|-------|-------|---------|---------------------|
| 0 | — | — | ~300 (FFI bridges) |
| 1 | — | — | ~200 (spawn/init fix) |
| 2 | — | — | ~200 (weapon bridges) |
| 3 | 6 | 1,678 | ~2,500 |
| 4 | 13 | 5,518 | ~8,300 |
| 5 | 9 | 6,747 | ~10,100 |
| 6 | — | — | ~100 (guard restoration) |
| **Total** | **28** | **13,943** | **~21,700** |

Rust lines are higher because: explicit error handling, tests, documentation, no macros/pointer-punning.
