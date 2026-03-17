# planet-solarsys current state (20260311)

## Scope and evidence posture

This document describes the **current implemented state** of the planet-solarsys subsystem, bounded to:

- solar-system exploration
- planet/moon orbit flow
- planet scan and surface generation flow
- planetary analysis/generation integration
- interplanetary/orbital navigation state
- persistence-sensitive scan/save-location behavior

Out of scope except as boundaries:

- campaign/story orchestration
- ships/combat
- generic lower-level rendering/audio/input/resource/state-file infrastructure

All key claims below are grounded in source evidence with exact file/line citations. Where a statement is an implication rather than a directly named fact, it is marked as **Inferred**.

## Executive summary

The subsystem is **not fully unported**. The exploration/gameplay implementation for solar systems, inner/outer system movement, orbit entry, orbital menus, scanning, planet-surface generation, and planetary analysis is still C-owned. Evidence for the main entry points and implementations is in `sc2/src/uqm/planets/solarsys.c`, `planets.c`, `scan.c`, `plangen.c`, `calc.c`, and the public contracts in `planets.h` and `generate.h` (`sc2/src/uqm/planets/solarsys.c:1713-1744`, `sc2/src/uqm/planets/planets.c:247-285`, `sc2/src/uqm/planets/planets.c:365-483`, `sc2/src/uqm/planets/scan.c:1160-1210`, `sc2/src/uqm/planets/scan.c:1256-1345`, `sc2/src/uqm/planets/plangen.c:1723-1815`, `sc2/src/uqm/planets/calc.c:356-529`, `sc2/src/uqm/planets/planets.h:183-252`, `sc2/src/uqm/planets/generate.h:49-100`).

However, the subsystem also has a **live Rust persistence edge** behind `USE_RUST_STATE`: `InitPlanetInfo`, `UninitPlanetInfo`, `GetPlanetInfo`, and `PutPlanetInfo` in `uqm/state.c` call Rust FFI when that bridge toggle is enabled, and the Rust side implements the scan-mask storage logic in `rust/src/state/ffi.rs` and `rust/src/state/planet_info.rs` (`sc2/src/uqm/state.c:335-455`, `rust/src/state/ffi.rs:341-435`, `rust/src/state/planet_info.rs:58-170`). Build configuration shows `USE_RUST_STATE` is part of the broader Rust bridge toggle set, but there is **no dedicated planet/solarsys Rust toggle** in the build config (`sc2/build/unix/build.config:86-107`, `sc2/build/unix/build.config:530-583`).

## Subsystem structure

### Core shared state and public surface

`SOLARSYS_STATE` is the central runtime structure for this subsystem. It carries:

- the active input function pointer
- interplanetary-flight state (`InIpFlight`)
- collision/orbit gating (`WaitIntersect`)
- solar/planet/moon descriptors (`SunDesc`, `PlanetDesc`, `MoonDesc`)
- current traversal pointers (`pBaseDesc`, `pOrbitalDesc`)
- solar-system generation hooks (`genFuncs`)
- currently analyzed/generated planetary data (`SysInfo`)
- orbital/surface rendering assets (`TopoFrame`, `Orbit`, `PlanetSideFrame`)
- orbit mode flag (`InOrbit`)

Evidence: `sc2/src/uqm/planets/planets.h:183-252`.

The same header exports the major subsystem entry points and helpers, including `ExploreSolarSys`, `LoadPlanet`, `GeneratePlanetSurface`, `PlanetOrbitMenu`, `SaveSolarSysLocation`, and location/world classification helpers such as `worldIsPlanet`, `worldIsMoon`, `planetIndex`, and `moonIndex` (`sc2/src/uqm/planets/planets.h:261-317`).

### Generation contract used by solar-system code

The solar-system code is parameterized by a `GenerateFunctions` table. That table includes handlers for:

- solar-system NPC setup/teardown
- planet layout generation
- moon layout generation
- orbital generation
- mineral / energy / life generation
- node pickup behavior

Evidence: `sc2/src/uqm/planets/generate.h:49-100`.

This means the subsystem is not a single monolith; its main flow is centralized in the generic planet/solarsys code, while system-specific content variations are injected through generator tables.

## Main control-flow entry points

### 1. Solar-system exploration entry

`ExploreSolarSys()` is the top-level entry into this subsystem. It:

1. resolves the current star if needed
2. updates logged SIS coordinates from the current star
3. installs a stack-local `SOLARSYS_STATE` into global `pSolarSysState`
4. zeroes the state
5. selects generation functions via `getGenerateFunctions(CurStarDescPtr->Index)`
6. initializes the solar system
7. runs the main input loop with `DoIpFlight`
8. uninitializes and clears the global pointer

Evidence: `sc2/src/uqm/planets/solarsys.c:1713-1744`.

### 2. Solar-system load/generation and inner-vs-outer initialization

`LoadSolarSys()` seeds `SysGenRNG` from the current star seed, sets up the sun descriptor, invokes `genFuncs->generatePlanets`, applies pending `PLANETARY_CHANGE` persistence by calling `PutPlanetInfo()`, computes temperature colors for generated planets via `DoPlanetaryAnalysis`, sorts planet positions, and then initializes either outer-system or inner-system state depending on `GLOBAL(ip_planet)`. If `GLOBAL(in_orbit)` is set, it returns the planet/moon to resume orbit around.

Evidence: `sc2/src/uqm/planets/solarsys.c:368-460`.

Observed facts from that function:

- planet layout generation is still direct C-side dispatch through `(*pSolarSysState->genFuncs->generatePlanets)` (`sc2/src/uqm/planets/solarsys.c:397`)
- temperature-color assignment depends on C `DoPlanetaryAnalysis` (`sc2/src/uqm/planets/solarsys.c:415-423`)
- moon generation in inner-system setup is still C-side `GenerateMoons(...)` (`sc2/src/uqm/planets/solarsys.c:439-442`)
- pending planetary persistence writes are committed from this C path using `PutPlanetInfo()` (`sc2/src/uqm/planets/solarsys.c:398-401`)

### 3. Orbit entry and orbital menu flow

`EnterPlanetOrbit()` is the transition from interplanetary movement into orbit. It:

- frees solar-system flight assets if this was reached by collision during IP flight
- repositions the ship stamp differently for planets vs moons
- loads persisted scan state with `GetPlanetInfo()`
- dispatches per-world orbital generation through `genFuncs->generateOrbital`
- enters the orbital UI if `TopoFrame` exists, via `PlanetOrbitMenu()`
- frees orbital/planet assets on exit
- reloads the solar system and revalidates orbits when returning from orbit

Evidence: `sc2/src/uqm/planets/solarsys.c:1255-1310`.

`PlanetOrbitMenu()` sets up the orbital menu loop, rotating-planet callback, and dispatch function `DoPlanetOrbit()` (`sc2/src/uqm/planets/planets.c:461-483`).

`DoPlanetOrbit()` shows the active orbital actions:

- `SCAN` -> `ScanSystem()`
- `EQUIP_DEVICE` -> `DevicesMenu()`
- `CARGO` -> `CargoMenu()`
- `ROSTER` -> `RosterMenu()`
- `GAME_MENU` -> `GameOptions()`
- `STARMAP` / `NAVIGATION` -> leave orbit/menu flow

Evidence: `sc2/src/uqm/planets/planets.c:365-452`.

### 4. Scan flow

`ScanSystem()` is the main scan-mode entry. It:

- prepares scan context
- handles shielded/gas-giant restrictions for menu start state
- initializes the planet location image for normal scans
- draws scanned objects
- prints coarse-scan information
- runs the scan input loop through `DoScan`
- performs scan display cleanup on exit

Evidence: `sc2/src/uqm/planets/scan.c:1160-1210`.

Within scan-related flow, there is persistence-sensitive behavior: when the Pluto/Fwiffo branch triggers, the code sets `START_ENCOUNTER` and calls `SaveSolarSysLocation()` before leaving the scan flow (`sc2/src/uqm/planets/scan.c:630-639`).

### 5. Planet load / surface generation / side generation

`LoadPlanet()` is called on orbit entry and on some orbit-return paths. It:

- creates planet context
- optionally draws wait-mode orbital UI
- stops existing music
- calls `GeneratePlanetSurface(pPlanetDesc, SurfDefFrame)`
- sets planet music
- calls `GeneratePlanetSide()`
- plays lander music if needed
- updates the orbital display

Evidence: `sc2/src/uqm/planets/planets.c:239-285`.

`GeneratePlanetSurface()` performs topography generation or ingestion of predefined surface data, seeds generation from `pPlanetDesc->rand_seed`, initializes planet orbit buffers, and either:

- uses supplied surface/elevation frames for defined worlds, or
- generates surface elevation/topography from planet algorithm data (`GAS_GIANT_ALGO`, `TOPO_ALGO`, `CRATERED_ALGO`, etc.)

Evidence: `sc2/src/uqm/planets/plangen.c:1723-1815`.

`GeneratePlanetSide()` builds the on-surface node population. It initializes the display list, skips shielded worlds, iterates scan types, asks generation hooks for node counts and per-node info via `callGenerateForScanType(...)`, skips already-retrieved nodes using `isNodeRetrieved(...)`, allocates elements for remaining nodes, and installs mineral/energy/bio specifics.

Evidence: `sc2/src/uqm/planets/scan.c:1256-1345`.

## Planetary analysis / generation integration

`DoPlanetaryAnalysis()` is the C implementation that turns a `PLANET_DESC` into populated `SYSTEM_INFO.PlanetInfo`. It seeds `SysGenRNG` from the world seed, derives star energy/intensity, binds `PlanData`, derives orbital distance, temperature, density, radius, rotation period, gravity, tilt, tectonics, atmospheric density, weather class, and life chance.

Evidence: `sc2/src/uqm/planets/calc.c:356-529`.

This function is actively used during solar-system load to derive each planet's `temp_color` in `LoadSolarSys()` (`sc2/src/uqm/planets/solarsys.c:415-423`).

**Observed fact:** planetary analysis is still implemented in C and directly consumed by the C exploration code.

**Inferred implication:** the current C implementation is the baseline evidence source for expected outputs and behavior of planetary analysis, especially where those outputs affect UI display, temperature classification, traversal/setup logic, and persistence-sensitive behavior. Future parity work should verify output equivalence against this baseline, though it need not preserve the exact internal formula decomposition if the externally visible results are identical.

## Current C/Rust split

### C-owned today

The following parts are directly implemented in C and are the live gameplay path:

- solar-system entry and lifetime management: `ExploreSolarSys`, `InitSolarSys`, `UninitSolarSys`, `LoadSolarSys` (`sc2/src/uqm/planets/solarsys.c:368-460`, `sc2/src/uqm/planets/solarsys.c:1316-1460`, `sc2/src/uqm/planets/solarsys.c:1713-1744`)
- interplanetary navigation and orbit transition state in `SOLARSYS_STATE` and `DoIpFlight`-driven flow (`sc2/src/uqm/planets/planets.h:185-252`, `sc2/src/uqm/planets/solarsys.c:1739-1742`)
- planet/moon classification and indexing helpers (`sc2/src/uqm/planets/solarsys.c:136-191`)
- orbit entry and orbit menu flow (`sc2/src/uqm/planets/solarsys.c:1255-1310`, `sc2/src/uqm/planets/planets.c:365-483`)
- scan UI and scan interaction (`sc2/src/uqm/planets/scan.c:1160-1210`)
- planet surface/topography generation (`sc2/src/uqm/planets/plangen.c:1723-1815`)
- surface node materialization and retrieved-node filtering (`sc2/src/uqm/planets/scan.c:1256-1345`)
- planetary analysis calculations (`sc2/src/uqm/planets/calc.c:356-529`)
- generator-table integration for planets, moons, orbital, and scan-node generation (`sc2/src/uqm/planets/generate.h:49-100`, `sc2/src/uqm/planets/solarsys.c:397`, `sc2/src/uqm/planets/solarsys.c:1278-1279`, `sc2/src/uqm/planets/scan.c:1278-1299`)

### Rust-owned today

The live Rust portion inside this subsystem boundary is the **planet info persistence edge**, not exploration gameplay.

When `USE_RUST_STATE` is enabled, `uqm/state.c` routes these functions to Rust:

- `InitPlanetInfo()` -> `rust_init_planet_info(get_num_stars())`
- `UninitPlanetInfo()` -> `rust_uninit_planet_info()`
- `GetPlanetInfo()` -> `rust_get_planet_info(...)`
- `PutPlanetInfo()` -> `rust_put_planet_info(...)`

Evidence: `sc2/src/uqm/state.c:335-455`.

The Rust FFI layer exposes those exact functions and marshals star index, planet index, moon index, moon-count arrays, and scan masks to/from the Rust manager (`rust/src/state/ffi.rs:341-435`).

The Rust implementation in `planet_info.rs` stores per-star offsets and per-planet/per-moon scan records, validates target indices, allocates new per-star record blocks on first write, computes record offsets accounting for preceding planets and moons, and reads/writes the three scan-mask values in C enum order.

Evidence: `rust/src/state/planet_info.rs:9-18`, `rust/src/state/planet_info.rs:58-170`.

### Build/config split evidence

`sc2/build/unix/build.config` defines `SYMBOL_USE_RUST_STATE_DEF` alongside the other Rust bridge symbols, and includes it in the generated-header substitution list (`sc2/build/unix/build.config:86-107`).

When the Rust bridge menu path is enabled, the config turns on a broad set of Rust subsystems including `USE_RUST_STATE=1`, sets `-DUSE_RUST_STATE`, and exports the corresponding symbol definition (`sc2/build/unix/build.config:530-583`).

There is no separate `USE_RUST_PLANETS`, `USE_RUST_SOLARSYS`, or similar planet/solar-system-specific toggle in the cited build configuration. The current Rust participation for this subsystem is therefore a **state-bridge inclusion**, not a dedicated exploration/planets port toggle.

## What has been ported vs. what remains C-only

### Ported / live Rust path

Ported within this subsystem boundary:

- planet scan retrieval/persistence backing for `InitPlanetInfo`, `UninitPlanetInfo`, `GetPlanetInfo`, `PutPlanetInfo` when `USE_RUST_STATE` is enabled (`sc2/src/uqm/state.c:335-455`)
- Rust-side representation and file-layout logic for scan retrieve masks and star/planet/moon offset addressing (`rust/src/state/planet_info.rs:58-170`)
- Rust FFI surface used by the C caller (`rust/src/state/ffi.rs:341-435`)

### Still C-only

Still C-only in the observed codebase:

- solar-system gameplay loop and traversal state (`sc2/src/uqm/planets/solarsys.c:1713-1744`)
- solar-system loading and procedural world layout integration (`sc2/src/uqm/planets/solarsys.c:368-460`)
- moon generation handoff and world indexing helpers (`sc2/src/uqm/planets/solarsys.c:136-191`, `sc2/src/uqm/planets/solarsys.c:217-239`)
- orbit-entry flow and orbital UI (`sc2/src/uqm/planets/solarsys.c:1255-1310`, `sc2/src/uqm/planets/planets.c:365-483`)
- scan-mode UI and interaction (`sc2/src/uqm/planets/scan.c:1160-1210`)
- planet-surface/topography generation (`sc2/src/uqm/planets/plangen.c:1723-1815`)
- planetary analysis formulas (`sc2/src/uqm/planets/calc.c:356-529`)
- node generation/display setup for mineral/energy/bio scans (`sc2/src/uqm/planets/scan.c:1256-1345`)

## Persistence-sensitive behavior inside the subsystem

### Planet scan state retrieval on orbit entry

On entering orbit, the subsystem explicitly loads persisted scan-retrieval state by calling `GetPlanetInfo()` before orbital generation (`sc2/src/uqm/planets/solarsys.c:1277-1279`).

`GetPlanetInfo()` zeroes the three scan masks first, then either:

- calls the Rust FFI path and copies back the Rust mask on success, or
- reads the same information from the legacy C state-file path

Evidence: `sc2/src/uqm/state.c:375-439`.

### Persistence writeback on planetary change

`LoadSolarSys()` checks `PLANETARY_CHANGE` and, if set, commits the updated scan state with `PutPlanetInfo()` before clearing the flag (`sc2/src/uqm/planets/solarsys.c:398-401`).

`SaveSolarSysLocation()` also checks `PLANETARY_CHANGE` while in orbit, writes planet info via `PutPlanetInfo()`, and clears the flag before recording orbital location (`sc2/src/uqm/planets/solarsys.c:1868-1895`).

This is the main persistence-sensitive edge that already crosses into Rust under `USE_RUST_STATE`.

### Save-location encoding of orbital position

`SaveSolarSysLocation()` is explicitly two-stage. Outside orbit it delegates to `saveNonOrbitalLocation()`. In orbit it persists scan changes if needed, asserts `GLOBAL(ip_planet)`, then stores `GLOBAL(in_orbit)` as a 1-based encoding where `1` means the planet itself and higher values encode moons with an additional offset.

Evidence: `sc2/src/uqm/planets/solarsys.c:1860-1895`.

`LoadSolarSys()` consumes `GLOBAL(in_orbit)` to reconstruct whether resumption should occur at the planet or one of its moons (`sc2/src/uqm/planets/solarsys.c:449-460`).

## Current deficiencies / gaps relevant to future parity or spec work

### 1. No dedicated Rust subsystem boundary for planet-solarsys

Observed fact: build config exposes `USE_RUST_STATE` but no planet/solarsys-specific Rust toggle (`sc2/build/unix/build.config:86-107`, `sc2/build/unix/build.config:530-583`).

Implication: current Rust participation is incidental to shared state bridging, not evidence of a broader exploration/gameplay port boundary.

### 2. `generateOrbital` contract is explicitly acknowledged as incomplete/awkward

`generate.h` contains a to-do list calling out desired refactoring such as splitting `generateOrbital` into calculation vs activation, making its return value meaningful, and reducing global coupling (`sc2/src/uqm/planets/generate.h:32-46`).

This is direct evidence that the generation/orbit API surface is known to be imperfect in the current architecture.

### 3. Heavy dependence on global mutable state

Observed facts:

- `pSolarSysState` is a global pointer (`sc2/src/uqm/planets/solarsys.c:77`, `sc2/src/uqm/planets/planets.h:254`)
- analysis, generation, and persistence helpers read global `CurStarDescPtr`, `GLOBAL(...)`, `GLOBAL_SIS(...)`, and `SysGenRNG` (`sc2/src/uqm/planets/solarsys.c:1717-1744`, `sc2/src/uqm/planets/solarsys.c:383-460`, `sc2/src/uqm/planets/calc.c:363-365`, `sc2/src/uqm/state.c:137-171`)

Implication: parity/spec work will need to account for implicit context, not just function signatures.

### 4. Persistence indexing depends on current traversal pointers and moon-count arrays

The persistence bridge is not generic world-ID storage; it depends on live star index, current planet index, moon index, current number of planets, and the array of moon counts (`sc2/src/uqm/state.c:137-171`, `sc2/src/uqm/state.c:381-455`, `rust/src/state/ffi.rs:364-435`, `rust/src/state/planet_info.rs:78-170`).

Implication: future parity work must preserve this address scheme or explicitly replace it with migration/compatibility handling.

### 5. Exploration/gameplay path remains entirely on the C side

There is no evidence in the inspected Rust sources or build config of a Rust implementation for:

- solar-system traversal
- orbit UI/menu flow
- scan UI
- planet analysis
- planet topography/surface generation
- surface node generation

The inspected Rust state module covers only persistence APIs (`rust/src/state/ffi.rs:341-435`, `rust/src/state/planet_info.rs:58-170`).

### 6. Some behavior-relevant comments indicate known quirks rather than clean abstractions

Examples:

- `calc.c` documents a temperature/orbit-color mismatch quirk around greenhouse-like adjustments (`sc2/src/uqm/planets/calc.c:161-173`)
- `generate.h` documents unresolved architectural cleanup tasks (`sc2/src/uqm/planets/generate.h:32-46`)
- orbit entry comments note a to-do for a better orbital-entry test based on `TopoFrame` (`sc2/src/uqm/planets/solarsys.c:1286-1293`)

These are relevant for specification work because they identify places where current behavior may be intentional but structurally awkward.

## Cross-subsystem boundaries

### Campaign/story orchestration boundary

System-specific story/content is injected through `GenerateFunctions` handlers, especially `generateOrbital` and related generators, but the generic planet-solarsys subsystem only dispatches to them; it does not itself define the story content. Evidence: generator interface in `generate.h` and calls in `solarsys.c` (`sc2/src/uqm/planets/generate.h:49-100`, `sc2/src/uqm/planets/solarsys.c:397`, `sc2/src/uqm/planets/solarsys.c:1278-1279`).

### Ships/combat boundary

Orbit and scan flows can trigger `START_ENCOUNTER`, and comments mention homeworld conversations, guards, and device-triggered encounters, but those systems are outside this subsystem. The solarsys code only sets activity flags, calls save-location logic, and exits back to broader game flow (`sc2/src/uqm/planets/solarsys.c:1281-1303`, `sc2/src/uqm/planets/solarsys.c:1295-1303`, `sc2/src/uqm/planets/scan.c:630-639`).

### Generic lower-level subsystem boundary

The planet-solarsys code depends on lower-level graphics, music, input, element, and state-file facilities, but those are infrastructure dependencies rather than part of this subsystem's owned logic. Examples include context/drawable management in `planets.c` and `plangen.c`, input-loop driving via `DoInput`, and state-file APIs/FFI in `state.c` (`sc2/src/uqm/planets/planets.c:54-83`, `sc2/src/uqm/planets/planets.c:247-285`, `sc2/src/uqm/planets/plangen.c:1737-1739`, `sc2/src/uqm/state.c:53-69`).

## Bottom line

The current subsystem state is a **hybrid, but not a split gameplay implementation**:

- **C still owns** solar-system exploration, orbit flow, scan flow, planet generation, planetary analysis, and traversal/save-location behavior.
- **Rust already owns a real edge** of the subsystem: the planet scan persistence path used by `InitPlanetInfo`, `UninitPlanetInfo`, `GetPlanetInfo`, and `PutPlanetInfo` when `USE_RUST_STATE` is enabled.
- Build/config evidence supports that this Rust edge is part of the broader Rust bridge toggle set, **not** a dedicated planet-solarsys Rust subsystem toggle.

So the accurate current-state description is: **planet-solarsys is mostly C-owned, with a live Rust persistence bridge at the planet scan/state edge.**
