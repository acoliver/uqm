# Phase 13a: Race Batch 3 Verification

## Phase ID
`PLAN-20260314-SHIPS.P13a`

## Prerequisites
- Required: Phase 13 (Race Batch 3) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Per-Race Verification Matrix — Melee Ships

| Race | Template | Weapon | Special | Complex Mechanic | Private Data | AI |
|------|----------|--------|---------|------------------|--------------|----| 
| Chmmr | [ ] | [ ] photon | [ ] tractor | [ ] ZapSats | [ ] sat handles | [ ] |
| Chenjesu | [ ] | [ ] crystal shard | [ ] DOGI | [ ] fragmentation | [ ] | [ ] |
| Mycon | [ ] | [ ] plasmoid | [ ] regen | [ ] homing + growth | [ ] | [ ] |
| Melnorme | [ ] | [ ] charge shot | [ ] confusion | [ ] charge mechanic | [ ] charge state | [ ] |
| Umgah | [ ] | [ ] cone | [ ] zip | [ ] cone area | [ ] | [ ] |
| Ur-Quan | [ ] | [ ] fusion | [ ] fighters | [ ] fighter AI | [ ] fighter handles | [ ] |
| Kohr-Ah | [ ] | [ ] blade | [ ] FRIED | [ ] boomerang + ring | [ ] blade/FRIED state | [ ] |
| Slylandro | [ ] | [ ] lightning | [ ] harvest | [ ] continuous move | [ ] | [ ] |
| ZoqFotPik | [ ] | [ ] spray | [ ] tongue | [ ] pull mechanic | [ ] tongue state | [ ] |

## Per-Race Verification Matrix — Non-Melee Ships

| Race | Template | Weapon | Special | Unique Mechanic | Catalog Excluded | Spawn Path |
|------|----------|--------|---------|-----------------|-----------------|------------|
| SIS Ship | [ ] | [ ] configurable | [ ] configurable | [ ] module loadout | [ ] excluded | [ ] hyperspace/encounter |
| Sa-Matra | [ ] | [ ] multi-weapon | [ ] special | [ ] final battle | [ ] excluded | [ ] final battle only |
| Probe | [ ] | [ ] minimal | [ ] minimal | [ ] autonomous | [ ] excluded | [ ] encounter |

## Critical Checks
- [ ] StubShip struct is deleted from registry.rs
- [ ] No match arms use StubShip
- [ ] `grep -r "StubShip" rust/src/ships/` returns zero results
- [ ] All 28 SpeciesId variants have real implementations
- [ ] Non-melee ships are NOT in master catalog
- [ ] Non-melee ships CAN be spawned outside catalog path

## Semantic Verification Checklist
- [ ] All 28 races have matching descriptor constants
- [ ] All complex mechanics tested (ZapSats, fighters, fragmentation, etc.)
- [ ] SIS configurable loadout reads campaign state
- [ ] Sa-Matra final battle behavior correct
- [ ] No placeholder code remains anywhere

## Gate Decision
- [ ] PASS: proceed to Phase 14
- [ ] FAIL: return to Phase 13 and fix issues
