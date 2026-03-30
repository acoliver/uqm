# Ships Remediation — Initial State

## What works right now
- Ship catalog loading via Rust (rust_ships_load_catalog)
- Ship building via Rust (rust_ships_build)
- Ship cost lookup via Rust (rust_ships_get_cost_by_index)
- Melee team serialize/deserialize via Rust (with byte↔int MeleeShip fix)
- Fleet value calculation via Rust
- Ship cost lookup via Rust
- Descriptor templates for all 28 species (correct static data)
- Battle engine infrastructure (12K lines): display_list, element, velocity, collision, weapon, process_loop
- Ship runtime framework: preprocess/postprocess, inertial_thrust, delta_energy
- FFI marshalling: Starship↔CStarship, ElementState, writeback

## What's broken
- `spawn_ship` creates RaceDesc but no ELEMENT (`_element_config` unused)
- `InitShips` / `UninitShips` routed to C (guards removed as workaround)
- `ship_preprocess` / `ship_postprocess` / `spawn_ship` in ship.c routed to C (guards removed)
- `load_ship` / `free_ship` in loadship.c routed to c_load_ship / c_free_ship
- All 28 species use `TemplateOnlyShip` — all behavior methods are no-ops
- `init_weapon` returns `Vec<WeaponElement>` (Rust struct) not real C ELEMENTs
- No `initialize_missile` / `initialize_laser` FFI wrappers
- No `EvaluateDesc` / `ship_intelligence` FFI for AI framework

## C files being replaced (reference)
```
sc2/src/uqm/ships/androsyn/androsyn.c    528 lines
sc2/src/uqm/ships/arilou/arilou.c        303 lines
sc2/src/uqm/ships/blackurq/blackurq.c    567 lines
sc2/src/uqm/ships/chenjesu/chenjesu.c    588 lines
sc2/src/uqm/ships/chmmr/chmmr.c          790 lines
sc2/src/uqm/ships/druuge/druuge.c        324 lines
sc2/src/uqm/ships/human/human.c          360 lines
sc2/src/uqm/ships/ilwrath/ilwrath.c      409 lines
sc2/src/uqm/ships/lastbat/lastbat.c      926 lines
sc2/src/uqm/ships/melnorme/melnorme.c    658 lines
sc2/src/uqm/ships/mmrnmhrm/mmrnmhrm.c   527 lines
sc2/src/uqm/ships/mycon/mycon.c          376 lines
sc2/src/uqm/ships/orz/orz.c            1083 lines
sc2/src/uqm/ships/pkunk/pkunk.c          640 lines
sc2/src/uqm/ships/probe/probe.c          118 lines
sc2/src/uqm/ships/shofixti/shofixti.c   521 lines
sc2/src/uqm/ships/sis_ship/sis_ship.c  1002 lines
sc2/src/uqm/ships/slylandr/slylandr.c   438 lines
sc2/src/uqm/ships/spathi/spathi.c       301 lines
sc2/src/uqm/ships/supox/supox.c         288 lines
sc2/src/uqm/ships/syreen/syreen.c       284 lines
sc2/src/uqm/ships/thradd/thradd.c       400 lines
sc2/src/uqm/ships/umgah/umgah.c         434 lines
sc2/src/uqm/ships/urquan/urquan.c       554 lines
sc2/src/uqm/ships/utwig/utwig.c         380 lines
sc2/src/uqm/ships/vux/vux.c             398 lines
sc2/src/uqm/ships/yehat/yehat.c         369 lines
sc2/src/uqm/ships/zoqfot/zoqfot.c       377 lines
TOTAL:                                 13,943 lines
```

## Rust files to be modified/completed
```
rust/src/ships/races/androsynth.rs   (141 lines — stub)
rust/src/ships/races/arilou.rs      (139 lines — stub)
rust/src/ships/races/black_urquan.rs (118 lines — stub)
rust/src/ships/races/chenjesu.rs    (136 lines — stub)
rust/src/ships/races/chmmr.rs       (137 lines — stub)
rust/src/ships/races/druuge.rs      (141 lines — stub)
rust/src/ships/races/human.rs       (142 lines — stub)
rust/src/ships/races/ilwrath.rs     (140 lines — stub)
rust/src/ships/races/melnorme.rs    (117 lines — stub)
rust/src/ships/races/mmrnmhrm.rs    (139 lines — stub)
rust/src/ships/races/mycon.rs       (117 lines — stub)
rust/src/ships/races/orz.rs         (140 lines — stub)
rust/src/ships/races/pkunk.rs       (159 lines — stub)
rust/src/ships/races/probe.rs       (108 lines — stub)
rust/src/ships/races/samatra.rs     (130 lines — stub)
rust/src/ships/races/shofixti.rs    (139 lines — stub)
rust/src/ships/races/sis_ship.rs    (125 lines — stub)
rust/src/ships/races/slylandro.rs   (128 lines — stub)
rust/src/ships/races/spathi.rs      (143 lines — stub)
rust/src/ships/races/supox.rs       (140 lines — stub)
rust/src/ships/races/syreen.rs      (140 lines — stub)
rust/src/ships/races/thraddash.rs   (141 lines — stub)
rust/src/ships/races/umgah.rs       (119 lines — stub)
rust/src/ships/races/urquan.rs      (118 lines — stub)
rust/src/ships/races/utwig.rs       (191 lines — stub)
rust/src/ships/races/vux.rs         (141 lines — stub)
rust/src/ships/races/yehat.rs       (152 lines — stub)
rust/src/ships/races/zoqfotpik.rs   (117 lines — stub)
```
