# Phase 4: Medium Ships (13 ships, 350-550 lines C each)

## Purpose
Port the mid-complexity ships. These have more sophisticated specials
(shields, cloaking, form-switching, self-destruct) and multi-step weapon
behaviors (homing, tracking, trails).

## Ships in this phase

### 1. Human (human.c — 360 lines)
- Cruiser: homing nuke + point-defense laser
- Nuke preprocess: TrackShip homing, accelerating speed
- Point defense: scans nearby enemies, fires laser at each
- Intelligence: fires nuke when enemy in range, PD auto-triggers

### 2. Yehat (yehat.c — 369 lines)
- Terminator: twin pulse cannon + energy shield
- Shield: absorbs damage, costs energy per hit
- Postprocess: shield activation/deactivation
- Intelligence: shield up when threatened

### 3. Mycon (mycon.c — 376 lines)
- Podship: homing plasmoid + crew regeneration
- Plasmoid preprocess: homing, grows in size over time
- Special: regenerates crew (costs energy)
- Preprocess: handles regeneration timer

### 4. Zoqfotpik (zoqfot.c — 377 lines)
- Stinger: anti-matter spray + tongue attack
- Tongue: melee-range instant kill
- Intelligence: tongue when very close, spray otherwise

### 5. Utwig (utwig.c — 380 lines)
- Jugger: lance weapon + energy-absorb shield
- Shield: absorbs incoming damage and converts to energy
- Must track shield state, absorbed energy
- Most complex shield in game

### 6. Thraddash (thradd.c — 400 lines)
- Torch: blaster + afterburner trail
- Trail: leaves damaging fire behind when thrusting
- Postprocess: creates trail elements
- Trail elements have their own collision/preprocess

### 7. Vux (vux.c — 398 lines)
- Intruder: laser + limpet special + warp-in-close
- Preprocess: warp-in-close at battle start (unique APPEARING behavior)
- Limpet: attaches to enemy, slows them permanently
- Limpet collision: modifies target's characteristics

### 8. Ilwrath (ilwrath.c — 409 lines)
- Avenger: hellfire blast + cloaking device
- Cloak: ship becomes invisible
- Preprocess: manages cloak state, visibility
- Intelligence: uncloak to fire, re-cloak

### 9. Slylandro (slylandr.c — 438 lines)
- Probe: lightning weapon + harvest (absorb enemy crew)
- Unique: inertia-less movement (instant velocity change)
- Preprocess: handles inertia-less thrust
- No turn_wait — rotates instantly

### 10. Umgah (umgah.c — 434 lines)
- Drone: antimatter cone + retro-propulsion (backstep)
- Cone: wide-angle short-range weapon (multiple elements)
- Special: thrust backward
- Preprocess: manages cone geometry

### 11. Shofixti (shofixti.c — 521 lines)
- Scout: energy dart + Glory Device (self-destruct)
- Glory Device: kills self, massive damage to everything nearby
- Death override: can trigger glory device on death
- Most complex death behavior

### 12. Mmrnmhrm (mmrnmhrm.c — 527 lines)
- X-Form: transforms between two ship configurations
- Each form has different stats (thrust, turn, weapons)
- Special: switches form, morphs characteristics
- Preprocess: handles transformation animation

### 13. Androsynth (androsyn.c — 528 lines)
- Guardian: molecular acid bubble + Blazer form
- Blazer: transforms into comet, damages on contact
- Two forms with completely different behavior
- Complex preprocess for form management

## Verification
- Same as Phase 3 but for all 13 ships
- Special attention to ships with state (cloak, shield, form-switch)
- Verify shield absorption math matches C exactly
- Verify form-switching preserves correct characteristics
