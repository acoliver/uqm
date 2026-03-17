# Phase 01: Analysis

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P01`

## Prerequisites
- Required: Phase 00.5 (Preflight Verification) completed and passed

## Purpose

Produce domain analysis artifacts that ground the implementation phases. This phase maps C structures to Rust equivalents, catalogs all cross-subsystem integration touchpoints, identifies code to replace, documents edge/error handling paths, and separates internal Rust models from FFI mirror types before code structure hardens.

## Entity / State Transition Analysis

### Core Entities

| C Entity | C File | Internal Rust Model | FFI / Mirror Status | Rust File |
|----------|--------|---------------------|---------------------|-----------|
| `PLANET_DESC` | `planets.h:107-120` | `PlanetDesc` domain struct | `CPlanetDesc` mirror if boundary crossing required | `types.rs` |
| `STAR_DESC` | `planets.h:122-128` | `StarDesc` domain struct | `CStarDesc` mirror if boundary crossing required | `types.rs` |
| `NODE_INFO` | `planets.h:130-148` | `NodeInfo` domain struct | `CNodeInfo` mirror for callback marshaling | `types.rs` |
| `PLANET_ORBIT` | `planets.h:152-176` | `PlanetOrbit` domain struct | Mirror only if C boundary requires it | `types.rs` |
| `SOLARSYS_STATE` | `planets.h:183-252` | `SolarSysState` domain struct | No direct by-value FFI exposure expected; audit required | `solarsys.rs` |
| `SYSTEM_INFO` / `PLANET_INFO` | `plandata.h` | `SystemInfo` / `PlanetInfo` domain structs | `CSystemInfo` / `CPlanetInfo` mirrors if C callbacks require layout fidelity | `types.rs` |
| `GenerateFunctions` | `generate.h:88-101` | Handler-class-specific Rust boundary | Raw C table wrapped, not normalized blindly | `generate.rs` |
| `PlanData` array | `plandata.h` | `PLAN_DATA` const array | N/A | `constants.rs` |
| `SunData` array | `sundata.h` | `SUN_DATA` const array | N/A | `constants.rs` |
| `SysGenRNG` (RandomContext) | `solarsys.c:79` | `SysGenRng` wrapper | N/A | `rng.rs` |
| `pSolarSysState` (global) | `solarsys.c:77` | Thread-local or `Option<Box<SolarSysState>>` | Hosted on Rust side | `solarsys.rs` |
| `CurStarDescPtr` (global) | various | Parameter passing or accessor wrapper | FFI accessor only if needed | `solarsys.rs` |

### Type-model split policy

| Category | Internal Rust allowed | Boundary allowed | Notes |
|----------|-----------------------|------------------|-------|
| Domain-only state | `Vec`, `Option`, enums, owned handles, trait objects | No | Internal ergonomics only |
| `#[repr(C)]` mirrors | Plain C-layout fields only | Yes | Used only at FFI boundary |
| Shared identity types | `WorldRef`, `OrbitTarget`, `PlanetSlot`, `MoonSlot` | Via conversion only | Avoid exposing `Option<usize>` or array offsets as ABI |
| Global/save values | Rust wrappers over raw scalar values | Raw scalar values yes | Encode/decode explicitly |

### State Transitions

```text
[No System] --ExploreSolarSys()--> [Outer System]
[Outer System] --approach planet--> [Inner System]
[Inner System] --approach body--> [In Orbit]
[In Orbit] --scan--> [Scan Mode] --exit scan--> [In Orbit]
[In Orbit] --leave orbit--> [Inner System]
[Inner System] --leave inner--> [Outer System]
[Outer System] --leave system--> [No System]

Any state --encounter trigger--> [Encounter] --return--> [reload system]
Any orbit state --save--> [persisted location]
[Load] --persisted location--> [appropriate state]
```

### Exploration Phase Model

| Phase | C Function | Input State | Output State |
|-------|-----------|-------------|--------------|
| Enter system | `ExploreSolarSys` | Star resolved | `SolarSysState` initialized |
| Load system | `LoadSolarSys` | Star seed | Planets generated, sorted |
| Init outer | `initOuterSystem` | Planets ready | IP flight active |
| Enter inner | `enterInnerSystem` | Planet approached | Moons generated |
| Enter orbit | `EnterPlanetOrbit` | Body collision | Scan state loaded, orbital content processed |
| Planet load | `LoadPlanet` | Orbital ready | Surface generated, nodes populated |
| Scan | `ScanSystem` | In orbit | Scan display active |
| Save location | `SaveSolarSysLocation` | Any in-system | Position persisted |
| Exit system | `UninitSolarSys` | Leaving system | State cleared |

## Generation-handler contract analysis

### Handler classes from spec §9.2

| Handler slot | Class | Planned Rust representation |
|--------------|-------|-----------------------------|
| Planet generation | Override/fallback | Class-specific dispatch wrapper |
| Moon generation | Override/fallback | Class-specific dispatch wrapper |
| Orbit content | Override/fallback | Class-specific dispatch wrapper preserving observable readiness/interrupt semantics |
| Name generation | Override/fallback | Class-specific dispatch wrapper |
| Mineral generation | Data provider | Count/per-node query API, not handled/not-handled |
| Energy generation | Data provider | Count/per-node query API |
| Life generation | Data provider | Count/per-node query API |
| NPC init/reinit/uninit | Side-effect/integration hook | Side-effect dispatch, no branching on return unless audit proves otherwise |
| Pickup hooks | Side-effect/integration hook | Side-effect dispatch, integration-owned consequences |

### Analysis outputs required before implementation

- Exact signature inventory for every handler slot from `generate.h`
- Representative dedicated-generator audits to confirm whether orbit-content and node-generation semantics vary by system
- Confirmation of which arguments are read-only, mutated in place, or encoded through shared global state
- Explicit note on which slots can safely use an internal Rust trait method and which require raw shims or wrapper enums

## Integration Touchpoints

### 1. Graphics Subsystem (`rust/src/graphics/`)

| Operation | Graphics API | Call Sites |
|-----------|-------------|------------|
| Create planet context | `RenderContext::new()` | `LoadPlanet` |
| Frame allocation | `FrameRegistry::create()` | `GeneratePlanetSurface`, sphere setup |
| Drawing primitives | `Canvas::fill_rect()`, `draw_line()` | Orbit drawing, oval, scan display |
| Sphere rotation | Custom sphere rendering | `InitSphereRotation`, `DrawPlanetSphere` |
| Color/colormap | `CmapColorMapRef` | Planet coloring, topo rendering |
| Context switching | `RenderContext::set_context()` | Scan, orbit, planet views |

### 2. Resource Subsystem (`rust/src/resource/`)

| Operation | Resource API | Data |
|-----------|-------------|------|
| Load planet-side frames | Resource loader | Bio canister, energy node, creature frames |
| Load colormaps | Colormap loader | Orbital colormaps |
| Load string banks | StringBank loader | Planet names, scan descriptions |
| Load surface def frames | Resource loader | Predefined planet surfaces |

### 3. State Subsystem (`rust/src/state/`)

| Operation | State API | Existing? |
|-----------|----------|-----------|
| `GetPlanetInfo` | `PlanetInfoManager::get_planet_info()` | [OK] Yes |
| `PutPlanetInfo` | `PlanetInfoManager::put_planet_info()` | [OK] Yes |
| `InitPlanetInfo` | `PlanetInfoManager::init_planet_info()` | [OK] Yes |
| `UninitPlanetInfo` | `PlanetInfoManager::uninit_planet_info()` | [OK] Yes |
| Game globals (GLOBAL) | `GameState` fields or accessors | Partial |

### 4. Input Subsystem (`rust/src/input/`)

| Operation | Input API |
|-----------|----------|
| IP flight input loop | `DoInput`-equivalent |
| Orbital menu input | Menu input dispatch |
| Scan input loop | Scan-mode key handling |

### 5. Sound Subsystem (`rust/src/sound/`)

| Operation | Sound API |
|-----------|----------|
| Planet music | `play_music()` / `stop_music()` |
| Lander sounds | Sound effect triggers |

### 6. Campaign/gameplay host boundary

| Obligation | Owner | Planet-solarsys concern |
|------------|-------|-------------------------|
| Init planet-info persistence before first legal get/put | Host | Required precondition |
| Keep persistence live for full solar-system session | Host | Assumed by orbit entry/save paths |
| Flush pending writes before teardown transitions | Host + plan | Must verify planet-solarsys call ordering and host coordination |
| Tear down persistence only after solar-system uninit and pending puts complete | Host | Explicit verification target |

## Old Code to Replace / Remove

### Files fully replaced (guarded behind `#ifndef USE_RUST_PLANETS`)

| C File | Lines | Purpose |
|--------|-------|---------|
| `calc.c` | ~530 | Planetary analysis |
| `plangen.c` | ~1815 | Surface generation |
| `gentopo.c` | ~200 | Topography deltas |
| `surface.c` | ~150 | Surface rendering helpers |
| `scan.c` | ~1345 | Scan flow + node materialization |
| `planets.c` | ~483 | Orbit menu, planet load |
| `solarsys.c` | ~1900 | System lifecycle, navigation |
| `orbits.c` | ~300 | Orbit rendering |
| `oval.c` | ~100 | Oval primitives |
| `pl_stuff.c` | ~200 | Planet display |
| `report.c` | ~150 | Coarse-scan report |

### Files partially replaced (integration boundaries)

| C File | What's replaced | What remains C |
|--------|----------------|----------------|
| `cargo.c` | Dispatch entry point | Menu implementation (future plan) |
| `devices.c` | Dispatch entry point | Menu implementation (future plan) |
| `roster.c` | Dispatch entry point | Menu implementation (future plan) |
| `lander.c` | Pickup hooks only | Lander gameplay (future plan) |

### Files NOT replaced (out of scope)

| C File | Reason |
|--------|--------|
| `pstarmap.c` | Navigational UI, not exploration flow |
| `sc2/src/uqm/planets/generate/*.c` | 50+ system-specific generators remain in C |

## Edge / Error Handling Map

| Situation | C Behavior | Rust Approach |
|-----------|-----------|---------------|
| Gas giant: no surface scan | Restricts scan menu options | `ScanRestriction::GasGiant` enum variant |
| Shielded world: no nodes | Skips node generation entirely | Early return in `materialize_nodes()` |
| No renderable topography | Skips orbital menu, returns to IP flight | Topography/readiness gate before menu |
| Encounter trigger in orbit | Sets `START_ENCOUNTER`, saves location, exits | `OrbitalOutcome::Encounter` or equivalent interrupt outcome |
| Encounter trigger in scan | Sets flag, saves location, exits scan | `ScanOutcome::Encounter` |
| Invalid planet/moon identity | C: undefined behavior | Rich identity type + checked conversion |
| Orbit re-entry gating | `WaitIntersect` prevents immediate re-entry | State field checked in collision |
| Moon index overflow | C: array bounds issue | Bounds-checked indexing |
| Persistence host not initialized | Undefined behavior attributed to host | Explicit host-boundary verification item; no silent defensive rewrite |
| Get/put after solar-system uninit | Conformance violation | Explicit verification target and prohibited path |

## Explicit Replacement List (Phase → C Code Removed)

| Phase | C Functions Replaced |
|-------|---------------------|
| P04 | `worldIsPlanet`, `worldIsMoon`, `planetIndex`, `moonIndex`, `matchWorld`, `GetRandomSeedForStar` |
| P06 | `DoPlanetaryAnalysis` (all of `calc.c`) |
| P08 | `GeneratePlanetSurface`, `DeltaTopography`, orbit/oval/sphere rendering |
| P09 | `ScanSystem`, `GeneratePlanetSide`, `DrawScannedObjects`, coarse-scan report |
| P10 | `LoadPlanet`, `FreePlanet`, `PlanetOrbitMenu`, `DoPlanetOrbit` |
| P11 | `ExploreSolarSys`, `LoadSolarSys`, `InitSolarSys`, `UninitSolarSys`, `DoIpFlight`, `SaveSolarSysLocation` |
| P12 | FFI wrappers for all above, `#ifndef USE_RUST_PLANETS` guards |

## Key design decisions recorded in analysis

- Initial parity preserves the existing persistence-addressing semantics exactly (star index + planet index + 1-based moon index semantics). No addressing redesign or migration is in scope for this plan.
- Generation handlers will be modeled by audited handler class semantics, not by a prematurely normalized single return-value protocol.
- Boundary-adjacent types require an explicit internal/domain vs. `#[repr(C)]` split before implementation starts.
