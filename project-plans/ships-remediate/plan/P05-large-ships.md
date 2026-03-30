# Phase 5: Large/Complex Ships (9 ships, 550+ lines C each)

## Purpose
Port the most complex ships. These have multiple sub-element types,
stateful behaviors, unique mechanics, and extensive AI.

## Ships in this phase

### 1. Ur-Quan (urquan.c — 554 lines)
- Dreadnought: fusion blast + autonomous fighters
- Fighters: independent sub-elements with own AI
- Fighter preprocess: pursuit logic, return-to-ship
- Fighter collision: deals damage, can be shot down
- Up to 4 fighters active simultaneously

### 2. Kohr-Ah / Black Ur-Quan (blackurq.c — 567 lines)
- Marauder: spinning blade + F.R.I.E.D. (ring of fire)
- Blade: boomerang-style return weapon
- Blade preprocess: complex trajectory (outward, pause, return)
- F.R.I.E.D.: expanding ring of fire elements
- Multiple ring elements with timed spawning

### 3. Chenjesu (chenjesu.c — 588 lines)
- Broodhome: crystal shard + D.O.G.I. (de-energizer)
- Shard: fragments into smaller pieces on command
- Shard preprocess: handles fragmentation trigger
- DOGI: independent pursuit element, drains enemy energy
- DOGI AI: own pursuit/return behavior

### 4. Pkunk (pkunk.c — 640 lines)
- Fury: triple mini-gun + insult channel + resurrection
- Insults: plays random voice clips, restores energy
- Resurrection: 50% chance to revive after death
- Death override: resurrection check, respawn logic
- Preprocess: insult timer and energy restoration

### 5. Melnorme (melnorme.c — 658 lines)
- Trader: charged shot + confusion pulse
- Weapon charges over time (hold to charge, release to fire)
- Preprocess: manages charge state
- Confusion pulse: reverses enemy controls
- Pulse collision: modifies target's input mapping

### 6. Chmmr (chmmr.c — 790 lines)
- Avatar: photon laser + ZapSat satellites
- ZapSats: 3 orbiting defense satellites
- ZapSat preprocess: orbital motion, auto-targeting
- Laser: continuous beam weapon (not projectile)
- Most complex ship element interaction

### 7. Sa-Matra / Last Battle (lastbat.c — 926 lines)
- Boss ship with unique mechanics
- Multiple weapon systems
- Shield generators (must be destroyed individually)
- Unique battle flow (not standard 1v1)
- Gate sentinels as sub-elements

### 8. SIS / Flagship (sis_ship.c — 1002 lines)
- Modular ship with configurable weapons
- Weapon loadout determined by installed modules
- Multiple weapon types depending on modules
- Precursor bomb, point-defense, various guns
- Escape pod (special — flee battle)
- Crew management across modules

### 9. Orz (orz.c — 1083 lines)
- Nemesis: howitzer + space marine boarding
- Marines: launched at enemy, board on contact
- Marine preprocess: pursuit, boarding sequence
- Marine collision: transfers to enemy ship, fights crew
- Intradimensional: marine inside enemy damages crew over time
- Most complex sub-element behavior in game

## Notes
- sis_ship and lastbat interact heavily with game state (GLOBAL_SIS, etc.)
- These will need additional FFI for game state access
- Orz marines and Chmmr zapsats are the most complex sub-element systems
- Pkunk resurrection modifies battle flow (respawning a dead ship)

## Verification
- Same as Phase 3/4
- Verify fighter/DOGI/marine AI matches C behavior
- Verify Pkunk resurrection probability and respawn
- Verify Melnorme charge mechanics
- Verify Chmmr ZapSat orbital math
- Verify SIS module-dependent weapon selection
- Sa-Matra: shield generator destruction sequence
