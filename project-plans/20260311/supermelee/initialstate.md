# SuperMelee initial state

## Scope and boundary of this document

This document covers the currently active C implementation of the SuperMelee subsystem responsible for setup, fleet selection/editing, load/save, local menu orchestration, and handoff into battle. The active implementation is centered under `sc2/src/uqm/supermelee/` and is entered via `Melee()` in `sc2/src/uqm/supermelee/melee.c:2035-2102`.

Per-subsystem boundaries for later documents:
- Per-ship combat logic belongs with ship implementations, not this subsystem. SuperMelee only stores ship identifiers and hands combat off to the battle system; the ship enum lives in `sc2/src/uqm/supermelee/meleeship.h:10-48`, while battle-time ship selection APIs are exposed through `sc2/src/uqm/supermelee/pickmele.h:31-46` and used by the battle/input side through `sc2/src/uqm/battlecontrols.h:29-49`.
- Networked melee protocol and transport belong with netplay, not this subsystem. SuperMelee code does contain setup-time and selection-time hooks into netplay, but those should be treated as integration boundaries rather than absorbing the entire netplay subsystem. Evidence: `sc2/src/uqm/supermelee/melee.h:117-139`, `sc2/src/uqm/supermelee/meleesetup.h:57-66`, `sc2/src/uqm/supermelee/pickmele.h:44-46`, and the separate netplay tree under `sc2/src/uqm/supermelee/netplay/` listed by the directory contents.

## Verified port status

No Rust port of the SuperMelee setup/selection/load-save/orchestration subsystem was found.

Evidence:
- The active subsystem implementation files are all C files under `sc2/src/uqm/supermelee/`: `melee.c`, `meleesetup.c`, `loadmele.c`, `pickmele.c`, `buildpick.c`, plus headers listed in the directory listing.
- Rust-side search found only CLI/config handling for melee zoom configuration, not SuperMelee orchestration or team-management code: `rust/src/cli.rs:169-170` sets `opts.melee_scale` via `parse_melee_zoom`, `rust/src/cli.rs:283-345` defines/tests that parser, and `rust/src/config.rs:35` plus `rust/src/config.rs:118` define/store `melee_scale` config state.
- Rust search for SuperMelee-specific symbols only matched the melee zoom/config path and did not find Rust equivalents of `Melee`, `MeleeSetup`, `GetNextMeleeStarShip`, or `BuildPickMeleeFrame` beyond the search terms report for `cli.rs` and `config.rs`.

Careful conclusion: there is no verified Rust port of this subsystem beyond unrelated config/melee-zoom parsing and storage on the Rust side (`rust/src/cli.rs:169-170,283-345`, `rust/src/config.rs:35,118`).

## What is ported vs not ported

- Ported in Rust: melee zoom CLI/config parsing and storage only, via `rust/src/cli.rs:169-170,283-345` and `rust/src/config.rs:35,118`.
- Not ported in Rust, still C-owned: SuperMelee entry/menu loop, fleet/team data model, team load/save, prebuilt team catalog, ship-pick UI, next-ship selection, setup-time control-mode handling, and battle handoff. The defining code is in `sc2/src/uqm/supermelee/melee.c:2035-2102`, `sc2/src/uqm/supermelee/meleesetup.c:31-423`, `sc2/src/uqm/supermelee/loadmele.c:53-824`, `sc2/src/uqm/supermelee/buildpick.c:29-220`, and `sc2/src/uqm/supermelee/pickmele.c:860-948`.

## Active C-side structure

### Directory-level structure

The active SuperMelee directory currently contains:
- core setup/orchestration files: `sc2/src/uqm/supermelee/melee.c`, `melee.h`
- team/setup data model: `sc2/src/uqm/supermelee/meleesetup.c`, `meleesetup.h`
- team load/save UI and persistence: `sc2/src/uqm/supermelee/loadmele.c`, `loadmele.h`
- ship selection during battle handoff/inter-battle flow: `sc2/src/uqm/supermelee/pickmele.c`, `pickmele.h`
- fleet-building popup UI: `sc2/src/uqm/supermelee/buildpick.c`, `buildpick.h`
- ship identifier enum: `sc2/src/uqm/supermelee/meleeship.h`
- a nested netplay subtree: `sc2/src/uqm/supermelee/netplay/`

This layout is directly confirmed by the directory listing of `/Users/acoliver/projects/uqm/sc2/src/uqm/supermelee`.

### Entry point and lifecycle

`Melee()` is the top-level entry point for the subsystem in `sc2/src/uqm/supermelee/melee.c:2035-2102`. It:
- initializes global data with `InitGlobData()` (`melee.c:2038`),
- allocates and zeroes a stack `MELEE_STATE` (`melee.c:2040-2044`),
- creates `MeleeSetup` state with `MeleeSetup_new()` (`melee.c:2048`),
- creates a local random context with `RandomContext_New()` and seeds it (`melee.c:2050-2054`),
- initializes load-state support with `InitMeleeLoadState()` (`melee.c:2066`),
- marks current activity as `SUPER_MELEE` (`melee.c:2068`),
- loads graphics/audio/team list with `LoadMeleeInfo()` (`melee.c:2070-2071`),
- loads persistent setup from `melee.cfg` with `LoadMeleeConfig()` and falls back to default prebuilt teams if that fails (`melee.c:2072-2080`),
- runs the menu/input loop through `DoInput(&MenuState, TRUE)` (`melee.c:2082-2084`), and
- persists setup and tears everything down via `WriteMeleeConfig()`, `FreeMeleeInfo()`, `UninitMeleeLoadState()`, `RandomContext_Delete()`, and `MeleeSetup_delete()` (`melee.c:2089-2098`).

The helper `LoadMeleeInfo()` loads UI assets and the team list by calling `BuildPickMeleeFrame()`, loading `MeleeFrame`, building the build-pick frame, initializing space visuals, and then `LoadTeamList()` (`sc2/src/uqm/supermelee/melee.c:1399-1409`). `FreeMeleeInfo()` reverses that, including closing netplay connections when built with `NETPLAY` (`sc2/src/uqm/supermelee/melee.c:1411-1435`).

### Core state structures

`MELEE_STATE` is the main runtime struct in `sc2/src/uqm/supermelee/melee.h:69-100`. It owns:
- the current input-state callback `InputFunc` (`melee.h:71`),
- setup/menu flags such as `Initialized` and `meleeStarted` (`melee.h:73-74`),
- current menu cursor state `MeleeOption`, `side`, `row`, `col`, `CurIndex` (`melee.h:75-87`),
- the persistent team/setup model via `MeleeSetup *meleeSetup` (`melee.h:79`),
- load/save browser state via `struct melee_load_state load` (`melee.h:80`),
- current ship selection/display state via `currentShip` and `buildPickConfirmed` (`melee.h:81-93`),
- a local random context used for local-only random choices (`melee.h:94-96`), and
- menu music handle `hMusic` (`melee.h:99`).

`MeleeSetup` and `MeleeTeam` define the persistent fleet/team model in `sc2/src/uqm/supermelee/meleesetup.h:39-67`:
- `MeleeTeam` contains `ships[MELEE_FLEET_SIZE]` and a team `name` buffer (`meleesetup.h:40-49`).
- `MeleeSetup` contains `teams[NUM_SIDES]` and cached `fleetValue[NUM_SIDES]` (`meleesetup.h:53-56`).
- In `NETPLAY` builds it also tracks `sentTeams` and `haveSentTeamName` to support setup-sync/update protocol state (`meleesetup.h:57-66`).

`melee_load_state` in `sc2/src/uqm/supermelee/loadmele.h:33-52` owns the load/save browser state, including prebuilt-team pointers, scanned directory entries, index indirection, a five-entry view cache, and cursor/view-window positions.

`MeleeShip` is a C enum in `sc2/src/uqm/supermelee/meleeship.h:10-41` covering all melee ship IDs, plus `MELEE_UNSET` for netplay sent-state bookkeeping (`meleeship.h:37-39`) and `MELEE_NONE` for empty fleet slots (`meleeship.h:39-40`).

## Current team/fleet data model behavior

### Team serialization and persistence format

The team image format is implemented in `sc2/src/uqm/supermelee/meleesetup.c:27-114`:
- `MeleeTeam_serialSize` is `MELEE_FLEET_SIZE + sizeof(name)` (`meleesetup.c:27-29`).
- `MeleeTeam_serialize()` writes one byte per fleet slot, then the full fixed-size name buffer (`meleesetup.c:63-77`).
- `MeleeTeam_deserialize()` reads the same layout, validates ship IDs, clamps invalid ship IDs to `MELEE_NONE`, and forces `name[MAX_TEAM_CHARS] = '\0'` (`meleesetup.c:79-109`).

At the setup level, `MeleeSetup_deserializeTeam()` and `MeleeSetup_serializeTeam()` wrap those operations and update cached fleet value on deserialize (`sc2/src/uqm/supermelee/meleesetup.c:323-339`).

### Fleet value caching and mutation

`MeleeSetup_setShip()` is the core mutator for ship slots in `sc2/src/uqm/supermelee/meleesetup.c:258-278`. It compares against the old ship, updates cached `fleetValue` by subtracting the old ship cost and adding the new ship cost via `GetShipCostFromIndex()`, then stores the new ship. `MeleeSetup_setTeamName()` updates the team name only if it changed (`meleesetup.c:293-306`). `MeleeSetup_getFleetValue()` returns the cached value (`meleesetup.c:316-319`).

`MeleeTeam_getValue()` separately computes value by summing `GetShipValue(ship)` over all slots (`sc2/src/uqm/supermelee/meleesetup.c:116-136`), and is used after deserialization in `MeleeSetup_deserializeTeam()` (`meleesetup.c:323-330`).

## Setup/menu orchestration currently in C

### Setup menu states and UI ownership

`melee.c` defines menu-state constants for controls, save/load, start, edit/build-pick, and optionally netplay connect entries under `NETPLAY` in `sc2/src/uqm/supermelee/melee.c:73-91`. The same file also owns core screen geometry, colors, and the main `MeleeFrame` drawable (`melee.c:99-187`).

The menu loop dispatches to specialized handlers. Search evidence shows the central transitions in `sc2/src/uqm/supermelee/melee.c:1783-1795`, where `StartMeleeButtonPressed(pMS)`, `DoLoadTeam(pMS)`, and `DoSaveTeam(pMS)` are invoked from the menu flow.

### Default initial state when config load fails

If `LoadMeleeConfig()` fails, `Melee()` assigns default control/team state: player 0 becomes human and receives `preBuiltList[0]`, while player 1 becomes computer-controlled and receives `preBuiltList[1]` (`sc2/src/uqm/supermelee/melee.c:2072-2080`).

### Persistent SuperMelee config

The setup screen persists a separate `melee.cfg` under `configDir`:
- `LoadMeleeConfig()` opens `configDir/melee.cfg`, validates file size as `(1 + MeleeTeam_serialSize) * NUM_SIDES`, reads one `PlayerControl` byte per side, deserializes each team, and explicitly strips `NETWORK_CONTROL` on load (`sc2/src/uqm/supermelee/melee.c:1964-1994`).
- `WriteMeleeConfig()` writes one `PlayerControl` byte plus serialized team per side and deletes the file on failure (`sc2/src/uqm/supermelee/melee.c:2002-2032`).

The explicit `NETWORK_CONTROL` stripping means netplay mode is not persisted as a startup state for SuperMelee (`sc2/src/uqm/supermelee/melee.c:1988-1990`).

## Load/save and prebuilt-team state

### Where team files live

On Unix builds, the SuperMelee teams directory is defined as `MELEEDIR "${UQM_CONFIG_DIR}/teams/"` in `sc2/config_unix.h:22-23`. `loadmele.c` uses `meleeDir` when opening, listing, writing, and deleting `.mle` files (`sc2/src/uqm/supermelee/loadmele.c:61`, `loadmele.c:454`, `loadmele.c:481`, `loadmele.c:489`).

### Team file browser and load behavior

`loadmele.c` owns the team browser UI and file interaction:
- `LoadTeamImage()` opens a file from `meleeDir` and deserializes it with `MeleeTeam_deserialize()` (`sc2/src/uqm/supermelee/loadmele.c:53-71`).
- `GetFleetByIndex()` first serves prebuilt teams, then directory-backed teams, and logs/filters invalid `.mle` files (`loadmele.c:96-129`).
- `FillFileView()` populates up to `LOAD_TEAM_VIEW_SIZE` view entries (`loadmele.c:210-230`), and `DrawFileStrings()` renders the load-team frame and entries (`loadmele.c:258-285`).
- `DoLoadTeam()` initializes the view, handles up/down/page navigation, and on select copies the chosen team into the active side with `Melee_LocalChange_team()` before returning to `DoMelee()` (`loadmele.c:320-435`, especially `349-360`).

### Save behavior

`DoSaveTeam()` derives the filename from the current team name plus `.mle`, writes the current side's `MeleeTeam` with `MeleeTeam_serialize()`, deletes the file on a failed write, reloads the team list, and reselects the saved filename in the browser (`sc2/src/uqm/supermelee/loadmele.c:465-509`).

### Prebuilt teams

Prebuilt teams are hardcoded in `InitPreBuilt()` in `sc2/src/uqm/supermelee/loadmele.c:511-777`. That function allocates `PREBUILT_COUNT 15` team objects (`loadmele.c:516-520`) and fills each with names and ship compositions, including examples such as "Balanced Team 1", "Balanced Team 2", "200 points", "Ford's Fighters", and "Star Control 2" (`loadmele.c:531-773`).

`InitMeleeLoadState()` and `UninitMeleeLoadState()` initialize and dispose of prebuilt-team storage, the five-entry view cache, and browser indices (`sc2/src/uqm/supermelee/loadmele.c:789-824`).

## Fleet-building and ship-pick flow

### Build-pick UI while editing fleets

`buildpick.c` owns the popup used to add ships while building fleets:
- `BuildBuildPickFrame()` creates an offscreen frame from `MeleeFrame` frame 27 and draws the full 5x5 ship grid (`sc2/src/uqm/supermelee/buildpick.c:29-54`).
- `DrawPickIcon()` draws or erases a ship icon in that popup (`buildpick.c:63-87`).
- `DrawPickFrame()` positions the popup relative to the currently edited fleet area and redraws the current ship info (`buildpick.c:89-113`).
- `BuildPickShip()` runs the popup interaction by setting `InputFunc = DoPickShip`, calling `DoInput`, and returning whether the pick was confirmed (`buildpick.c:202-220`).

`DoPickShip()` handles directional cursor movement across the 5x5 ship grid, ship-spin preview on special input, confirm/cancel, and flashing selection (`sc2/src/uqm/supermelee/buildpick.c:121-199`).

### Battle-time ship selection from fleets

`pickmele.h` exposes `GetInitialMeleeStarShips()` and `GetNextMeleeStarShip()` as the battle-facing ship selection API (`sc2/src/uqm/supermelee/pickmele.h:35-37`). `battlecontrols.h` includes `supermelee/pickmele.h` and defines `SelectShipFunction` over `GETMELEE_STATE`, showing the battle input layer depends on this SuperMelee selection interface (`sc2/src/uqm/battlecontrols.h:28-49`).

In `pickmele.c`:
- `GetInitialMeleeStarShips()` updates both pick frames, fades in, builds a player bitmask, and calls `GetMeleeStarShips()` to obtain initial ships (`sc2/src/uqm/supermelee/pickmele.c:860-884`).
- `GetNextMeleeStarShip()` redraws the requesting player's pick frame, requests a new choice for only that player, and returns the selected `HSTARSHIP` (`pickmele.c:886-901`).

This is a handoff boundary: SuperMelee chooses ships from fleets and hands back `HSTARSHIP` handles, but battle execution itself is elsewhere.

## Battle handoff boundary

`StartMelee()` in `sc2/src/uqm/supermelee/melee.c:1447-1488` is the local orchestration handoff into combat. It:
- fades out menu music and screen (`melee.c:1449-1456`),
- calls `SetPlayerInputAll()` (`melee.c:1465-1466`),
- rebuilds/draws ship lists via `BuildAndDrawShipList()` (`melee.c:1467`),
- loads a gravity well using a random planet type (`melee.c:1471-1472`),
- calls `Battle(NULL)` (`melee.c:1473`),
- frees the gravity well and clears player input afterward (`melee.c:1474-1475`), and
- restores `GLOBAL(CurrentActivity) = SUPER_MELEE` and marks the menu state uninitialized on return (`melee.c:1485-1488`).

The battle subsystem recognizes this boundary explicitly. In `sc2/src/uqm/battle.c:404-405`, non-SuperMelee battle seeds RNG locally because "In Supermelee, the RNG is already initialised." During battle startup, when activity is `SUPER_MELEE`, battle state is prepared and run, and on abort the code returns to the SuperMelee menu rather than the main menu (`sc2/src/uqm/battle.c:426-489`, especially `476-489`).

SuperMelee also affects battle display configuration through the shared melee zoom option: `battle.c` applies `optMeleeScale` before battle draw/update (`sc2/src/uqm/battle.c:434-435`), while the setup menu persists that option as `meleezoom` (`sc2/src/uqm/setupmenu.c:1272-1273`). This is only a display/config boundary, not a Rust port of the subsystem.

## Relevant boundaries outside `supermelee/`

### Ship catalog boundary

The global list of ships available to SuperMelee is `master_q` in `sc2/src/uqm/master.h:43-45`, documented there as the "List of ships available in SuperMelee". It is populated in `LoadMasterShipList()` in `sc2/src/uqm/master.c:29-40`. `master.c` includes `supermelee/melee.h` (`master.c:19-25`), showing the coupling between the global ship catalog and SuperMelee setup.

### Activity-state boundary

`inSuperMelee()` is defined in `sc2/src/uqm/globdata.c:492-497` and simply checks whether `LOBYTE(GLOBAL(CurrentActivity)) == SUPER_MELEE`. This is the global activity flag boundary other systems use to distinguish SuperMelee behavior.

### Menu/battle/input integration boundary

`battlecontrols.h` imports `supermelee/pickmele.h` and routes selection through `SelectShipFunction` over `GETMELEE_STATE` (`sc2/src/uqm/battlecontrols.h:28-49`). `tactrans.c` contains the battle-end synchronization logic for networked play and uses SuperMelee netplay hooks such as `Netplay_Notify_frameCount()` and ready-state logic (`sc2/src/uqm/tactrans.c:124-148`, `169-210`), but that synchronization protocol belongs with netplay/battle transport rather than this setup subsystem.

`shipyard.c` includes `supermelee/melee.h` at the top-level include boundary (`sc2/src/uqm/shipyard.c:26-34`), but this document does not find shipyard owning the SuperMelee subsystem itself.

## Netplay hooks present inside SuperMelee code

These hooks are present and relevant as integration boundaries, but they do not make the whole netplay subsystem part of this document's ownership scope.

- `MELEE_STATE` exposes netplay-related callbacks and local/remote setup mutation entry points in `sc2/src/uqm/supermelee/melee.h:115-139`.
- `MeleeSetup` adds sent-state tracking fields only under `NETPLAY` in `sc2/src/uqm/supermelee/meleesetup.h:57-66`.
- `StartMeleeButtonPressed()` validates network-control combinations, requires in-setup connected state, triggers remote confirmation, and transitions into confirm mode in `sc2/src/uqm/supermelee/melee.c:1491-1581`.
- `DoConfirmSettings()` performs network confirmation, seed exchange, ready negotiation, and input-delay setup before calling `StartMelee()` in `sc2/src/uqm/supermelee/melee.c:1314-1396`.
- `Melee_LocalChange_ship()`, `Melee_LocalChange_teamName()`, `Melee_bootstrapSyncTeam()`, `Melee_RemoteChange_ship()`, and `Melee_RemoteChange_teamName()` implement setup-screen synchronization hooks around local/remote changes in `sc2/src/uqm/supermelee/melee.c:2374-2635`.
- Battle-time ship-pick code can also report remote ship selections through `updateMeleeSelection()` and `reportShipSelected()` in `sc2/src/uqm/supermelee/pickmele.c:904-947`.

The transport/protocol implementation itself is separated under `sc2/src/uqm/supermelee/netplay/`, for example packet types in `sc2/src/uqm/supermelee/netplay/packet.h:52-184,260-298`, state machine/protocol files in `sc2/src/uqm/supermelee/netplay/proto/reset.c:33-158` and `proto/ready.c:30-102`, and connection/orchestration logic in `sc2/src/uqm/supermelee/netplay/netmelee.c:383-697`.

## Key C-owned files, functions, and data structures currently defining the subsystem

### Files

- `sc2/src/uqm/supermelee/melee.c` - top-level SuperMelee entry, menu orchestration, config persistence, setup mutation/view updates, and battle handoff (`melee.c:1399-1488`, `1491-1581`, `1964-2102`, `2270-2635`)
- `sc2/src/uqm/supermelee/melee.h` - `MELEE_STATE`, public subsystem API, netplay-facing change hooks (`melee.h:35-144`)
- `sc2/src/uqm/supermelee/meleesetup.c` - `MeleeTeam`/`MeleeSetup` allocation, mutation, serialization, cached fleet values, netplay sent-state bookkeeping (`meleesetup.c:27-423`)
- `sc2/src/uqm/supermelee/meleesetup.h` - definitions of `MeleeTeam`, `MeleeSetup`, and public mutator/accessor APIs (`meleesetup.h:20-137`)
- `sc2/src/uqm/supermelee/loadmele.c` - `.mle` browsing/loading/saving and prebuilt teams (`loadmele.c:53-824`)
- `sc2/src/uqm/supermelee/loadmele.h` - `melee_load_state` and load/save API (`loadmele.h:22-62`)
- `sc2/src/uqm/supermelee/buildpick.c` - fleet-edit ship-pick popup (`buildpick.c:29-220`)
- `sc2/src/uqm/supermelee/pickmele.c` - initial/next battle ship selection and selection-sync hooks (`pickmele.c:860-947`)
- `sc2/src/uqm/supermelee/pickmele.h` - battle-facing selection API (`pickmele.h:31-46`)
- `sc2/src/uqm/supermelee/meleeship.h` - canonical SuperMelee ship ID enum (`meleeship.h:10-48`)

### Functions

- `Melee()` (`sc2/src/uqm/supermelee/melee.c:2035-2102`)
- `LoadMeleeInfo()` / `FreeMeleeInfo()` (`sc2/src/uqm/supermelee/melee.c:1399-1435`)
- `StartMelee()` / `StartMeleeButtonPressed()` (`sc2/src/uqm/supermelee/melee.c:1447-1581`)
- `LoadMeleeConfig()` / `WriteMeleeConfig()` (`sc2/src/uqm/supermelee/melee.c:1964-2032`)
- `Melee_LocalChange_ship()` / `Melee_LocalChange_teamName()` / `Melee_LocalChange_fleet()` / `Melee_LocalChange_team()` (`sc2/src/uqm/supermelee/melee.c:2374-2454`)
- `Melee_bootstrapSyncTeam()` / `Melee_RemoteChange_ship()` / `Melee_RemoteChange_teamName()` (`sc2/src/uqm/supermelee/melee.c:2460-2635`)
- `MeleeSetup_setShip()` / `MeleeSetup_setTeamName()` / `MeleeSetup_getFleetValue()` / `MeleeSetup_deserializeTeam()` / `MeleeSetup_serializeTeam()` (`sc2/src/uqm/supermelee/meleesetup.c:258-339`)
- `DoLoadTeam()` / `DoSaveTeam()` / `LoadTeamList()` / `InitMeleeLoadState()` (`sc2/src/uqm/supermelee/loadmele.c:320-509`, `447-463`, `809-824`)
- `BuildPickShip()` (`sc2/src/uqm/supermelee/buildpick.c:202-220`)
- `GetInitialMeleeStarShips()` / `GetNextMeleeStarShip()` (`sc2/src/uqm/supermelee/pickmele.c:860-901`)

### Data structures

- `MELEE_STATE` (`sc2/src/uqm/supermelee/melee.h:69-100`)
- `MeleeTeam` (`sc2/src/uqm/supermelee/meleesetup.h:39-49`)
- `MeleeSetup` (`sc2/src/uqm/supermelee/meleesetup.h:53-67`)
- `melee_load_state` (`sc2/src/uqm/supermelee/loadmele.h:33-52`)
- `GETMELEE_STATE` / `struct getmelee_struct` (`sc2/src/uqm/supermelee/pickmele.h:20`, `62-90`)
- `MeleeShip` enum (`sc2/src/uqm/supermelee/meleeship.h:10-48`)

## Concise conclusion

The SuperMelee subsystem described here remains C-owned. Its current implementation is centered in `sc2/src/uqm/supermelee/`, with `melee.c` orchestrating setup/menu flow and battle handoff, `meleesetup.c` owning team/fleet state and serialization, `loadmele.c` owning prebuilt and file-backed team load/save, `buildpick.c` owning fleet-edit ship picking, and `pickmele.c` owning initial/next combatant selection. No Rust port was verified for this subsystem beyond unrelated melee-zoom config parsing/storage in `rust/src/cli.rs` and `rust/src/config.rs`.