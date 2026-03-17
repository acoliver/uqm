# Ships Subsystem Initial State

## Scope and boundary

This document covers the **per-ship combat/runtime layer** and the **shared ship catalog/runtime contracts** that those ship implementations depend on.

In scope:

- the shared ship runtime types and contracts in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h`
- shared ship lifecycle/load/build code in:
  - `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c`
  - `/Users/acoliver/projects/uqm/sc2/src/uqm/loadship.c`
  - `/Users/acoliver/projects/uqm/sc2/src/uqm/master.c`
  - `/Users/acoliver/projects/uqm/sc2/src/uqm/build.c`
  - `/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c`
  - `/Users/acoliver/projects/uqm/sc2/src/uqm/init.c`
- race-specific ship implementations under `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/`
- the active runtime boundary with battle entry/exit and master ship list loading

Explicitly out of scope:

- **SuperMelee setup ownership/UI** beyond the points where it consumes the master ship list or instantiates battle queues. The pick/setup flows are only referenced here to define the boundary.
- **Full netplay transport/state synchronization**. Netplay appears inside battle flow, but this document does not treat network transport as part of the ships subsystem.

That boundary is visible in the code:

- battle calls `InitShips()` and `UninitShips()` but owns the overall battle loop in `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:416-512`
- startup/kernel init loads the master ship list via `LoadMasterShipList(TaskSwitch)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:93-98`
- SuperMelee setup consumes `master_q` and `Build(...)` from outside the ships subsystem in `/Users/acoliver/projects/uqm/sc2/src/uqm/supermelee/pickmele.c:587-609`

## Verified port status

The ships subsystem is still C-owned in the active build.

### No ship-specific Rust build toggle exists

The Rust bridge flag list in `/Users/acoliver/projects/uqm/sc2/build/unix/build.config` defines many `USE_RUST_*` toggles, but **none for ships**.

Evidence:

- the substitution-variable list includes `USE_RUST_BRIDGE`, `USE_RUST_FILE`, `USE_RUST_CLOCK`, `USE_RUST_UIO`, `USE_RUST_AUDIO`, `USE_RUST_AUDIO_HEART`, `USE_RUST_COMM`, `USE_RUST_INPUT`, `USE_RUST_VIDEO`, `USE_RUST_VIDPLAYER`, `USE_RUST_GFX`, `USE_RUST_RESOURCE`, `USE_RUST_MOD`, `USE_RUST_WAV`, `USE_RUST_DUKAUD`, `USE_RUST_AIFF`, `USE_RUST_THREADS`, `USE_RUST_MIXER`, `USE_RUST_MEM`, and `USE_RUST_STATE`, but no ship toggle, in `/Users/acoliver/projects/uqm/sc2/build/unix/build.config:86-107`
- the enabled Rust bridge action adds `-DUSE_RUST_BRIDGE -DUSE_RUST_FILE -DUSE_RUST_CLOCK -DUSE_RUST_UIO -DUSE_RUST_OGG -DUSE_RUST_AUDIO -DUSE_RUST_COMM -DUSE_RUST_INPUT -DUSE_RUST_VIDEO -DUSE_RUST_VIDPLAYER -DUSE_RUST_GFX -DUSE_RUST_RESOURCE -DUSE_RUST_MOD -DUSE_RUST_WAV -DUSE_RUST_DUKAUD -DUSE_RUST_AIFF -DUSE_RUST_THREADS -DUSE_RUST_MIXER -DUSE_RUST_MEM -DUSE_RUST_STATE`, but not ships, in `/Users/acoliver/projects/uqm/sc2/build/unix/build.config:497-563`

Search for `USE_RUST_SHIP` / `USE_RUST_SHIPS` found no matches anywhere in the repo.

### The Rust crate has no ships module

`/Users/acoliver/projects/uqm/rust/src/lib.rs` exports `comm`, `game_init`, `graphics`, `input`, `io`, `memory`, `resource`, `sound`, `state`, `threading`, `time`, and `video`, but no ships module (`/Users/acoliver/projects/uqm/rust/src/lib.rs:7-23`).

This is direct evidence that there is no active Rust-owned ships implementation module in the crate root.

### Rust game-init FFI exposes ship-related wrappers, but they are not active integration points

`/Users/acoliver/projects/uqm/rust/src/game_init/ffi.rs` does define wrappers for ship initialization and master ship list loading:

- `rust_init_ships()` at `/Users/acoliver/projects/uqm/rust/src/game_init/ffi.rs:35-41`
- `rust_uninit_ships()` at `/Users/acoliver/projects/uqm/rust/src/game_init/ffi.rs:43-49`
- `rust_load_master_ship_list()` at `/Users/acoliver/projects/uqm/rust/src/game_init/ffi.rs:94-101`
- `rust_free_master_ship_list()` at `/Users/acoliver/projects/uqm/rust/src/game_init/ffi.rs:103-109`

However, repo-wide search found **no C callers** for these symbols. The only matches are their definitions and Rust tests inside the same file. Search results for `rust_init_ships`, `rust_uninit_ships`, `rust_load_master_ship_list`, and `rust_free_master_ship_list` point only to `/Users/acoliver/projects/uqm/rust/src/game_init/ffi.rs`.

So these wrappers exist as prototype FFI surface only. They are **not active integration points** in the current C runtime.

## What is and is not ported

### Not ported / still active in C

The active ships subsystem remains in C:

- shared runtime contracts in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h`
- ship load/free logic in `/Users/acoliver/projects/uqm/sc2/src/uqm/loadship.c:58-200`
- battle ship runtime logic in `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:39-573`
- battle/space ship init and teardown in `/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:113-349`
- master ship catalog build/free/find logic in `/Users/acoliver/projects/uqm/sc2/src/uqm/master.c:27-214`
- queue/build/clone helpers in `/Users/acoliver/projects/uqm/sc2/src/uqm/build.c:29-547`
- code-resource dispatch to per-race `init_*` functions in `/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:38-206`
- all race implementations in `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/*/*.c`

### Ported only as inactive Rust prototypes

There is partial Rust work under `rust/src/game_init/`, but it is not wired into the C runtime for ships. That matters for parity planning because there is some exploratory Rust API surface, but no build toggle, no module root support, and no C call sites.

## Active C-side structure

## Shared contracts in `races.h`

`/Users/acoliver/projects/uqm/sc2/src/uqm/races.h` is the central ship runtime contract.

### Ship identity and catalog enums

- `SPECIES_ID` enumerates ship identities and special cases (`ARILOU_ID` through `MMRNMHRM_ID`, plus `SIS_SHIP_ID`, `SA_MATRA_ID`, `UR_QUAN_PROBE_ID`) in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:74-108`
- `LAST_MELEE_ID = MMRNMHRM_ID` establishes the melee catalog boundary in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:103`
- the race-order enums used by campaign/melee support live in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:401-431`

### Shared battle/runtime flags and characteristics

- ship capability flags like `SEEKING_WEAPON`, `POINT_DEFENSE`, `IMMEDIATE_WEAPON`, `CREW_IMMUNE`, `FIRES_FORE`, and `SHIELD_DEFENSE` are defined in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:43-58`
- battle-time input/status flags like `LEFT`, `RIGHT`, `THRUST`, `WEAPON`, `SPECIAL`, `LOW_ON_ENERGY`, `SHIP_AT_MAX_SPEED`, and `SHIP_IN_GRAVITY_WELL` are defined in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:60-72`
- movement/energy timing and mass are centralized in `CHARACTERISTIC_STUFF` at `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:145-158`

### Core runtime data structures

- `SHIP_INFO` carries cost, crew, energy, ship flags, resource ids, and loaded icon/string handles in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:160-177`
- `DATA_STUFF` carries battle resources: ship/weapon/special frame arrays, captain graphics, victory ditty, and ship sounds in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:187-201`
- `INTEL_STUFF` carries AI maneuverability, weapon range, and the ship intelligence callback in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:136-143`
- `RACE_DESC` aggregates all of that and adds the per-race function hooks `uninit_func`, `preprocess_func`, `postprocess_func`, `init_weapon_func`, plus private `data` and `CodeRef` in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:204-227`

### Runtime queue objects

- `STARSHIP` is the per-battle/per-selection runtime object, containing `RaceDescPtr`, crew state, icons, counters, input/state flags, `hShip`, facing, `playerNr`, and control in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:245-285`
- `SHIP_FRAGMENT` is the persistent queue fragment for escort/built-ship style lists in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:301-324`
- `FLEET_INFO` is the campaign/master-availability side structure in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:337-382`

### Shared catalog tables

`races.h` also embeds cross-subsystem tables that the ship catalog depends on:

- `RACE_COMMUNICATION` in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:433-457`
- `RACE_SHIP_FOR_COMM` in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:459-486`
- `RACE_SHIP_COST` in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:488-512`
- movement/speed tables such as `RACE_IP_SPEED`, `RACE_HYPER_SPEED`, `RACE_HYPERSPACE_PERCENT`, `RACE_INTERPLANETARY_PERCENT`, and `RACE_ENCOUNTER_MAKEUP` in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h:513-645`

For planning purposes, this means ships are not just combat behaviors. The contract file mixes combat, campaign, comm mapping, and catalog metadata.

## Shared C implementation structure

### `build.c`: queue allocation and ship/fragment cloning

`/Users/acoliver/projects/uqm/sc2/src/uqm/build.c` provides the common queue/object layer.

Key pieces:

- `Build(...)` allocates either a `STARSHIP` or `SHIP_FRAGMENT`, zeros it, sets `SpeciesID`, and enqueues it (`/Users/acoliver/projects/uqm/sc2/src/uqm/build.c:29-50`)
- `GetStarShipFromIndex(...)` does queue index lookup (`/Users/acoliver/projects/uqm/sc2/src/uqm/build.c:53-69`)
- escort and campaign helpers like `AddEscortShips`, `CountEscortShips`, `HaveEscortShip`, `EscortFeasibilityStudy`, `SetRaceAllied`, and `StartSphereTracking` operate on `avail_race_q` and `built_ship_q` (`/Users/acoliver/projects/uqm/sc2/src/uqm/build.c:96-403`)
- `CloneShipFragment(...)` clones from `avail_race_q`/`FLEET_INFO` into a `SHIP_FRAGMENT`, copying strings/icons/melee_icon and crew/energy metadata (`/Users/acoliver/projects/uqm/sc2/src/uqm/build.c:461-507`)

This is a shared-contract layer, not battle logic proper, but combat flow depends on its `STARSHIP` construction and queue conventions.

### `dummy.c`: code-resource dispatch to race implementations

`/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c` is the live bridge from resource identifiers to per-race `init_*` functions.

Evidence:

- `ShipCodeRes` enumerates the ship code resources in `/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:43-74`
- `CodeResToInitFunc(...)` maps each code resource to a concrete per-race initializer such as `init_androsynth`, `init_arilou`, `init_chmmr`, `init_sis`, and `init_probe` in `/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:78-117`
- `GetCodeResData(...)` allocates a `CODERES_STRUCT`, calls the initializer, and copies the returned `RACE_DESC` by value into resource-owned storage (`/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:119-145`)
- `InstallCodeResType()` installs the resource type under the string name `"SHIP"` in `/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:154-159`

This is an important current-state fact: ship implementations are not registered through a dynamic module system or Rust bridge. They are selected through C resource dispatch and copied `RACE_DESC` instances.

### `loadship.c`: load/free of `RACE_DESC` plus optional battle assets

`/Users/acoliver/projects/uqm/sc2/src/uqm/loadship.c` is the active loader for both catalog-only and battle-ready ship instances.

Evidence:

- `code_resources[]` maps `SPECIES_ID` order to code resources in `/Users/acoliver/projects/uqm/sc2/src/uqm/loadship.c:27-56`
- `load_ship(SpeciesID, LoadBattleData)` captures a code resource into `RDPtr`, sets `RDPtr->CodeRef`, loads icons/melee icons/race strings, and optionally loads battle assets (`ship`, `weapon`, `special`, captain background, victory music, ship sounds`) when `LoadBattleData` is true (`/Users/acoliver/projects/uqm/sc2/src/uqm/loadship.c:58-167`)
- `free_ship(...)` calls the race-specific `uninit_func` if present, then conditionally frees battle assets and icon/string assets before destroying the code resource (`/Users/acoliver/projects/uqm/sc2/src/uqm/loadship.c:169-200`)

This gives the subsystem a two-tier load mode:

- **catalog mode**: `LoadBattleData = FALSE` for master ship list / metadata-only use
- **battle mode**: `LoadBattleData = TRUE` for active combat runtime use

### `master.c`: active master ship list loading

`/Users/acoliver/projects/uqm/sc2/src/uqm/master.c` owns the global `master_q` catalog.

Evidence:

- `QUEUE master_q;` is defined at `/Users/acoliver/projects/uqm/sc2/src/uqm/master.c:27`
- `LoadMasterShipList(...)` initializes `master_q` for `LAST_MELEE_ID - ARILOU_ID + 1` entries, iterates species ids from `ARILOU_ID`, loads each ship with `load_ship(..., FALSE)`, copies `ship_info` and `fleet`, frees only the code resource side, and then inserts the entry sorted by race name (`/Users/acoliver/projects/uqm/sc2/src/uqm/master.c:29-91`)
- `FreeMasterShipList()` frees icons/melee icons/string tables from each `MASTER_SHIP_INFO` and uninitializes the queue (`/Users/acoliver/projects/uqm/sc2/src/uqm/master.c:93-115`)
- lookup/access helpers `FindMasterShip`, `FindMasterShipIndex`, `GetShipCostFromIndex`, `GetShipIconsFromIndex`, and `GetShipMeleeIconsFromIndex` are implemented in `/Users/acoliver/projects/uqm/sc2/src/uqm/master.c:117-214`

The active integration point for this catalog load is startup/kernel init, not Rust:

- `LoadMasterShipList(TaskSwitch);` is called from `/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:93-98`

### `init.c`: space setup and ship lifecycle into/out of battle

`/Users/acoliver/projects/uqm/sc2/src/uqm/init.c` is the live entry/exit point for the ships runtime from battle.

Evidence:

- `InitSpace()` loads shared space/background/explosion/blast/asteroid assets in `/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:113-144`
- `InitShips()` calls `InitSpace()`, sets contexts, initializes display list and galaxy, and then either builds the SIS in hyperspace or prepares encounter/battle space with asteroids/planets (`/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:177-242`)
- `UninitShips()` stops sound, uninitializes space, counts floating crew elements, frees each live `RaceDescPtr` with `free_ship(..., TRUE, TRUE)`, clears `IN_BATTLE`, records remaining crew back into ship fragments where appropriate, and reinitializes race queues outside encounter persistence (`/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:268-349`)

### `ship.c`: active per-battle runtime behavior

`/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c` is the active combat/runtime layer above the per-race implementations.

Important responsibilities:

- shared animation and inertial movement helpers: `animation_preprocess()` and `inertial_thrust()` (`/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:39-147`)
- ship core preprocess loop: input/status normalization, first-time setup, energy regen, turn/thrust handling, status UI preprocess (`/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:149-280`)
- ship core postprocess loop: weapon fire, sound triggering, special cooldown handling, race-specific postprocess callback, status UI postprocess (`/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:282-350`)
- default ship-vs-planet collision handling in `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:352-377`
- `spawn_ship(...)`: loads the battle-ready `RACE_DESC`, patches crew state, allocates/binds the ship `ELEMENT`, and wires the common callbacks `ship_preprocess`, `ship_postprocess`, `ship_death`, and `collision` (`/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:379-497`)
- `GetNextStarShip(...)` and `GetInitialStarShips()` select and spawn ships for combat (`/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:499-573`)

This file is the clearest expression of the current ships runtime boundary: race-specific code plugs into the common pipeline through `RACE_DESC` callbacks and shared data fields.

## Per-race directory pattern under `sc2/src/uqm/ships/`

The repository currently has one directory per ship implementation under `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/`:

- `androsyn`, `arilou`, `blackurq`, `chenjesu`, `chmmr`, `druuge`, `human`, `ilwrath`, `lastbat`, `melnorme`, `mmrnmhrm`, `mycon`, `orz`, `pkunk`, `probe`, `shofixti`, `sis_ship`, `slylandr`, `spathi`, `supox`, `syreen`, `thradd`, `umgah`, `urquan`, `utwig`, `vux`, `yehat`, `zoqfot`

Shared ship include surface lives in `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/ship.h:17-36`.

The per-race pattern is consistent:

- each implementation includes `../ship.h` plus its race header and `resinst.h`
- each file defines a static `RACE_DESC <race>_desc`
- that descriptor is initialized with `SHIP_INFO`, `FLEET_STUFF`, `CHARACTERISTIC_STUFF`, `DATA_STUFF`, and `INTEL_STUFF`
- the file then fills in callback hooks and returns the descriptor from `init_<race>()`

Concrete examples:

- Androsynth defines `static RACE_DESC androsynth_desc` and later sets `uninit_func`, `preprocess_func`, `postprocess_func`, `init_weapon_func`, and `cyborg_control.intelligence_func` in `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/androsyn/androsyn.c:48-127` and `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/androsyn/androsyn.c:514-523`
- Arilou follows the same pattern in `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/arilou/arilou.c:39-116` and `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/arilou/arilou.c:291-298`

Search across `sc2/src/uqm/ships/**/*.c` found many direct mutations of `RaceDescPtr->ship_info`, `RaceDescPtr->characteristics`, `RaceDescPtr->ship_data`, and callback pointers, which confirms the ship implementations actively depend on the shared mutable `RACE_DESC` contract.

## Lifecycle into and out of battle

## Master list load / catalog path

The master catalog path is:

1. startup/background init calls `LoadMasterShipList(TaskSwitch)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:93-98`
2. `LoadMasterShipList(...)` iterates species ids and calls `load_ship(..., FALSE)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/master.c:30-66`
3. `load_ship(..., FALSE)` creates a `RACE_DESC` and loads icon/string metadata only in `/Users/acoliver/projects/uqm/sc2/src/uqm/loadship.c:58-103`
4. the loaded metadata is copied into `master_q` and sorted by race name in `/Users/acoliver/projects/uqm/sc2/src/uqm/master.c:62-89`

This is the live source of truth for ship-pick/status catalog data used by setup/UI consumers.

## Setup boundary with SuperMelee

SuperMelee setup consumes the catalog but does not own the ships runtime layer itself.

Evidence:

- `pickmele.c` looks up `MASTER_SHIP_INFO` from `master_q`, calls `Build(&race_q[side], MasterPtr->SpeciesID)`, and stamps queue/runtime metadata onto the new `STARSHIP` in `/Users/acoliver/projects/uqm/sc2/src/uqm/supermelee/pickmele.c:587-609`
- `melee.c` reads `master_q` to call `InitShipStatus(&MasterPtr->ShipInfo, NULL, NULL)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/supermelee/melee.c:920-925`

That is the correct active boundary for this document: setup consumes shared ship catalog/runtime contracts, but ownership of actual ship runtime behavior stays in the common ships files and race implementations.

## Entry into battle runtime

The active battle entry path is:

1. battle starts by calling `InitShips()` in `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:414-417`
2. if ships are available, battle sets `IN_BATTLE`, initializes counters, and calls `selectAllShips(num_ships)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:426-452`
3. `selectAllShips()` dispatches to `GetNextStarShip(...)` or `GetInitialStarShips()` depending on mode (`/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:376-393`, from search evidence)
4. `GetInitialStarShips()` / `GetNextStarShip()` in `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:499-573` select the next `STARSHIP` handle and call `spawn_ship(...)`
5. `spawn_ship(...)` calls `load_ship(StarShipPtr->SpeciesID, TRUE)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:379-387`
6. `spawn_ship(...)` then binds the resulting `RACE_DESC` to the `STARSHIP`, allocates the `ELEMENT`, and assigns the shared preprocess/postprocess/death/collision callbacks in `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:389-497`

The selection boundary itself is in `pickship.c`:

- `GetEncounterStarShip(...)` switches between hyperspace SIS, SuperMelee choice, and full-game queue selection in `/Users/acoliver/projects/uqm/sc2/src/uqm/pickship.c:303-357`

## Ship death / transition / replacement during battle

The runtime transition to the next ship is handled outside the per-race files but inside the broader combat runtime.

Evidence:

- `new_ship(...)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/tactrans.c:439-520` is called when a dead ship element expires
- it stops sound/music, clears the dead ship's `RaceDescPtr` via `free_ship(..., TRUE, TRUE)`, updates persistent crew via `UpdateShipFragCrew(...)` if needed, marks the dead ship inactive, and then calls `GetNextStarShip(...)` to spawn the replacement (`/Users/acoliver/projects/uqm/sc2/src/uqm/tactrans.c:466-509`)

This is part of the active boundary with battle: combat owns when transitions happen, but ship runtime ownership of loading/freeing stays in the ship subsystem.

## Exit from battle runtime

Battle exits the ships subsystem by calling `UninitShips()` in `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:511-512`.

`UninitShips()` then:

- stops audio (`/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:276`)
- uninitializes shared space assets (`/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:278`)
- counts crew objects still floating in the display list (`/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:283-285`)
- walks remaining ship-related elements, updates surviving crew, and frees each active `RaceDescPtr` with `free_ship(...)` (`/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:286-320`)
- records post-battle crew back into ship fragments with `UpdateShipFragCrew(...)` during encounter flow (`/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:326-337`)
- clears `IN_BATTLE` and resets queues as appropriate (`/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:324-348`)

`UpdateShipFragCrew(...)` itself lives in `/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:213-247` and persists `STARSHIP.crew_level` back to the matching `SHIP_FRAGMENT`.

## What the current subsystem architecture implies

The current architecture is callback/data driven, centered on `RACE_DESC`.

- common runtime in `ship.c` owns the fixed battle loop integration
- each race file plugs custom behavior in through `preprocess_func`, `postprocess_func`, `init_weapon_func`, and `cyborg_control.intelligence_func`
- `load_ship()` controls whether the `RACE_DESC` is used for metadata-only or fully battle-loaded operation
- `dummy.c` and the `SHIP` resource type remain the active indirection layer between species id and implementation function

This means the subsystem is not organized as a single ship engine plus pure data. Many ship behaviors mutate the descriptor itself at runtime.

Examples from the race files:

- Androsynth mutates `characteristics.energy_regeneration`, `characteristics.turn_wait`, `characteristics.special_wait`, swaps `collision_func`, and switches between `ship_data.ship` and `ship_data.special` forms in `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/androsyn/androsyn.c:375-505`
- Arilou directly drives teleport state by mutating `ElementPtr` flags, `special_counter`, and current/next image arrays in `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/arilou/arilou.c:197-288`
- search results show similar mutable use of shared `RACE_DESC` state across many race files, especially `pkunk`, `mmrnmhrm`, `sis_ship`, `urquan`, `chmmr`, `orz`, and others

For parity planning, this is a strong signal that ship behavior is not just isolated weapon callbacks. It relies on shared mutable combat/runtime structures and common engine conventions.

## Current deficiencies and unknowns relevant to parity planning

### 1. No active Rust integration path exists for ships

This is the most concrete planning fact:

- no `USE_RUST_SHIP` / `USE_RUST_SHIPS` toggle exists
- no `ships` Rust module exists in `rust/src/lib.rs`
- prototype Rust FFI wrappers exist, but no C caller uses them

So any ships parity plan starts from **inactive prototype surface**, not from a partially switched subsystem.

### 2. The boundary between ships and battle is tight

Although this document scopes battle out, the ship runtime depends heavily on battle engine structures and callbacks:

- ship runtime attaches directly to `ELEMENT` callbacks in `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:484-487`
- ship death/transition is driven by `new_ship(...)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/tactrans.c:439-520`
- selection flow uses `GetEncounterStarShip(...)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/pickship.c:303-357`

That means per-race porting is not independent of combat/runtime host behavior, even if setup ownership is excluded.

### 3. `RACE_DESC` is mutable and copied by value

`dummy.c` copies the `RACE_DESC` returned by `init_*()` into allocated resource storage (`/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:129-142`). Many race files then mutate fields inside that descriptor at runtime.

That copy-and-mutate lifecycle is easy to miss but important for parity:

- the static descriptor in each race file is a template
- the runtime uses copied descriptors, not the original static object directly
- some races also allocate per-instance private state via `RACE_DESC.data`

### 4. Shared contract file mixes combat and non-combat concerns

`races.h` mixes:

- battle flags and runtime structs
- campaign fleet metadata
- communication mapping
- ship costs and travel-speed tables

That increases the chance that a "ships" change affects campaign/catalog/UI code even when combat logic is unchanged.

### 5. Loader cleanup is incomplete on failure

`load_ship()` contains an explicit TODO: `// TODO: We should really free the resources that did load here` at `/Users/acoliver/projects/uqm/sc2/src/uqm/loadship.c:159`.

That is a verified current-state deficiency in the active C implementation.

### 6. Master ship list is metadata-only and melee-bounded

`LoadMasterShipList()` only iterates from `ARILOU_ID` through `LAST_MELEE_ID` (`/Users/acoliver/projects/uqm/sc2/src/uqm/master.c:33-35`). It does not load SIS, Sa-Matra, or probe into `master_q`.

That matters for planning because the shared catalog contract and the full ship runtime are not identical in scope.

### 7. SuperMelee and campaign use different queue/object shapes

The subsystem spans both `STARSHIP` and `SHIP_FRAGMENT`, with persistence updates written back through queue-order matching in `UpdateShipFragCrew(...)` (`/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:223-247`).

This means parity work has to preserve more than just battle behavior; it must preserve queue ordering and identity conventions between runtime and persistent fragments.

### 8. Exact parity surface for all race-specific private data is broad

At least some races use `RACE_DESC.data` for private allocated state:

- Androsynth private data allocation/free is in `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/androsyn/androsyn.c:131-162` and `/Users/acoliver/projects/uqm/sc2/src/uqm/ships/androsyn/androsyn.c:507-510`

Search results show similar callback and mutable-state patterns across many ships, but this document did not exhaustively verify every ship's private-data shape. That remains an evidence-backed unknown for full parity inventory work.

## Bottom line

The ships subsystem is currently an **all-C active subsystem** built around `RACE_DESC`, `STARSHIP`, `SHIP_FRAGMENT`, and `master_q`.

Verified current state:

- there is **no `USE_RUST_SHIP` or `USE_RUST_SHIPS` build toggle**
- the Rust bridge flag list in `/Users/acoliver/projects/uqm/sc2/build/unix/build.config` **does not include ships**
- `/Users/acoliver/projects/uqm/rust/src/lib.rs` **has no ships module**
- `/Users/acoliver/projects/uqm/rust/src/game_init/ffi.rs` contains `rust_init_ships` / `rust_load_master_ship_list` style wrappers, but repo search found **no C callers**, so they are **not active integration points**

Operationally, the subsystem works like this:

- startup loads a metadata-only master catalog into `master_q`
- setup/UI consumers read that catalog and build `STARSHIP` queue entries
- battle calls `InitShips()`, ship selection chooses queue entries, and `spawn_ship()` loads a battle-ready `RACE_DESC`
- common ship runtime in `ship.c` drives movement, energy, firing, and callback dispatch
- race-specific logic under `sc2/src/uqm/ships/` supplies the actual ship behaviors
- battle exit and ship-death transitions free those `RACE_DESC` instances and write crew results back into persistent fragments where applicable

For parity planning, the important current-state conclusion is that ships are **not yet a live Rust port at all**; they are a tightly coupled C subsystem with inactive Rust prototype wrappers and a broad shared contract surface that spans catalog, campaign, and combat runtime concerns.
