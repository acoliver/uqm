# Phase 3: Small Ships (6 ships, <350 lines C each)

## Purpose
Port the simplest ship behaviors first. Each ship is independent — porting
one cannot break another. This phase proves the pattern works before tackling
complex ships.

## Ships in this phase

### 1. Probe (probe.c — 118 lines)
- Ur-Quan probe / autonomous drone
- No weapon, no special
- Simplest possible ship — validates the basic framework
- C: `init_probe()` sets no function pointers (all NULL)

### 2. Syreen (syreen.c — 284 lines)  
- Penetrator: particle beam weapon + Syreen Song special
- Song: steals crew from enemy ship
- Intelligence: standard + song when enemy close
- Sub-elements: song effect element

### 3. Supox (supox.c — 288 lines)
- Blade: forward glob weapon + lateral/reverse thrust special
- Special: thrust in any direction while holding special
- Preprocess: handles special thrust override
- Intelligence: uses special for dodging

### 4. Spathi (spathi.c — 301 lines)
- Eluder: forward missile + rear-firing B.U.T.T. torpedo
- Two weapons: primary fires forward, special fires backward
- Intelligence: prefers running away

### 5. Arilou (arilou.c — 303 lines)
- Skiff: auto-aim laser + quasi-space teleport
- Laser: IMMEDIATE_WEAPON flag, auto-aims at nearest enemy
- Special: teleport to random location (no velocity)
- Preprocess: handles teleport positioning

### 6. Druuge (druuge.c — 324 lines)
- Mauler: recoil cannon + furnace (sacrifice crew for energy)
- Weapon has strong recoil (modifies ship velocity)
- Special: kills one crew, restores energy
- Postprocess: handles recoil physics

## Per-ship porting pattern

For each ship:

1. **Read C source** — identify all functions, constants, sub-elements
2. **Port constants** — `const` block at top of Rust file
3. **Port init_weapon** — create MissileBlock/LaserBlock, call create_missile/create_laser
4. **Port preprocess** — ship-specific per-frame logic (if any)
5. **Port postprocess** — ship-specific post-physics logic (if any)
6. **Port intelligence** — AI decision logic
7. **Port sub-element callbacks** — weapon preprocess, collision functions
8. **Update registry** — replace TemplateOnlyShip with real impl
9. **Test** — unit tests for weapon params, AI behavior, descriptor match

## Verification
- `cargo check` passes
- `cargo test` passes
- Each ported ship's descriptor_template values match C exactly
- Each ported ship's weapon parameters match C exactly
- Super melee: ported ships fire weapons, AI controls work
