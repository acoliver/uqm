# Planet-SolarSys Subsystem — Functional & Technical Specification

This document specifies the desired end-state behavior of the planet-solarsys subsystem. It describes what the subsystem does, what it owns, how it interfaces with the rest of the engine, and what observable behavior must be preserved. It is not an implementation plan.

Status labels used in this document:

- **Required compatibility behavior**: externally observable behavior that must be preserved regardless of implementation choices.
- **Integration contract**: a boundary obligation that this subsystem must satisfy toward or require from an adjacent subsystem.
- **Persistence contract**: behavior that affects save/load fidelity and must be preserved for round-trip correctness.
- **Open decision**: an area where the end-state contract is not fully settled and requires audit, evidence, or deliberate design choice before signoff.

---

## 1. Scope

The planet-solarsys subsystem owns:

1. solar-system exploration lifecycle (entry, traversal, exit),
2. interplanetary navigation state and flight within a solar system,
3. inner-system (planet close-up) navigation and orbit collision gating,
4. orbit entry and the orbital menu flow,
5. scan-mode entry, scan interaction, and scan-type dispatch,
6. planet surface generation (topography, elevation, sphere rendering data),
7. surface-node materialization (mineral, energy, biological node population on a world surface),
8. planetary analysis — the derivation of world physical characteristics from star/planet seed data,
9. integration with per-system generation functions for system-specific content variation,
10. persistence-sensitive scan-state retrieval and writeback (the planet-info persistence edge), and
11. save-location encoding and restoration for in-system and in-orbit positions.

This specification does **not** cover:

- campaign/story orchestration, encounter flow, or game-event logic,
- per-race dialogue scripts or communication subsystem behavior,
- ships, combat mechanics, or the battle engine,
- lander gameplay mechanics (surface traversal, hazard interaction, cargo collection) beyond the subsystem's obligation to populate the surface with nodes and provide the topographic surface,
- generic lower-level graphics, audio, input, resource, file-I/O, or threading subsystem design.

Those are integration boundaries. This subsystem depends on them but does not own their contracts.

---

## 2. Boundary and ownership model

### 2.1 Subsystem boundary

The planet-solarsys subsystem owns the following end-state responsibilities:

1. **Solar-system lifecycle management** — entering a solar system, initializing generation state, running the exploration loop, and tearing down on exit.
2. **Interplanetary navigation** — driving the player ship through the outer-system view among generated planets, handling zoom transitions to inner systems, and managing collision/intersection gating for orbit entry.
3. **Inner-system navigation** — presenting the planet close-up view with its moons and managing transitions between inner-system flight and orbit entry.
4. **Orbit entry and orbital menu** — transitioning from flight to orbit, loading persisted scan state, dispatching per-world orbital content processing, determining orbital readiness, performing planet loading when appropriate, and presenting the orbital action menu (scan, devices, cargo, roster, game menu, navigation/starmap).
5. **Scan flow** — entering scan mode from the orbital menu, driving the scan-type selection and display loop, rendering scanned-object overlays, and presenting coarse-scan information.
6. **Planet surface generation** — producing topographic elevation data, 3D sphere rendering assets, and per-world visual surfaces from planet seeds, algorithm selectors, and optional predefined surface data.
7. **Surface-node population** — querying generation functions for node counts and per-node info across scan types (mineral, energy, biological), filtering already-retrieved nodes against persisted scan masks, and materializing the remaining nodes into the surface display list.
8. **Planetary analysis** — computing world physical characteristics (temperature, density, radius, gravity, rotation, tilt, tectonics, atmospheric density, weather, life chance) from star energy and planet seed data.
9. **Generation-function integration** — dispatching to a per-system generation-function table for planet layout, moon layout, orbital content, name generation, and scan-node generation.
10. **Planet-info persistence edge** — reading and writing per-planet/per-moon scan-retrieval masks at orbit entry, save, and solar-system load transitions, addressed by star index, planet index, and moon index.
11. **Save-location encoding** — recording and restoring the player's position within a solar system, including outer-system coordinates, inner-system planet index, and in-orbit planet/moon encoding.

### 2.2 Out-of-scope but required integration boundaries

The subsystem must integrate correctly with, but does not define, the following:

- **Campaign/story boundary:** System-specific content variation is injected through generation-function tables. The planet-solarsys subsystem dispatches to those handlers but does not own the story content, encounter triggers, or game-event logic they implement. Encounter-triggering results (e.g., `START_ENCOUNTER`) produced by generation functions or scan interactions are propagated outward; the broader campaign system consumes them.
- **Lander/surface-gameplay boundary:** The subsystem generates the topographic surface and populates it with nodes. The lander subsystem owns surface traversal, hazard processing, cargo pickup mechanics, and crew/resource accounting. Node-pickup callbacks bridge back into the generation-function table; the subsystem dispatches those callbacks as integration hooks that affect node persistence and content outcomes, but does not define lander traversal or interaction behavior.
- **Ships/combat boundary:** Orbit and scan flows can trigger transitions to encounters or battles. The subsystem sets activity flags and exits; it does not own ship selection, battle simulation, or combat resolution.
- **Resource/graphics/audio boundary:** The subsystem depends on the resource system for asset loading, the graphics system for rendering contexts and frame management, and the audio system for planet music. It defines what assets it needs and when; those subsystems provide the load/render/playback mechanics.
- **State/persistence boundary:** The subsystem reads and writes planet scan data through a persistence API. In the end state, the subsystem owns the calling contract — when and what to get/put — and the semantic content of what is persisted. Persistence store initialization and teardown are hosting obligations: the persistence infrastructure must be initialized before the subsystem's first get/put call and torn down after the last. The subsystem depends on this guarantee but does not own the init/uninit lifecycle. The backing storage implementation is an integration dependency.
- **Global game-state boundary:** The subsystem reads and writes navigational globals and game-state flags relevant to in-system position. It depends on these being available but does not own the global state infrastructure.

---

## 3. Runtime state model

### 3.1 Solar-system state

The subsystem shall maintain a runtime state structure sufficient to represent:

- the active input/interaction handler for the current exploration phase,
- whether the player is in interplanetary flight,
- collision/intersection gating state (which planet or moon the ship should not collide with after recently leaving orbit or an inner system),
- the sun descriptor for the current system,
- the array of generated planet descriptors (up to the system maximum),
- the array of generated moon descriptors for the current inner system,
- traversal pointers identifying the current base descriptor set (planets in outer system, moons in inner system) and the current orbital target,
- sorted planet ordering for display and traversal,
- ship movement parameters (turn, thrust, speed),
- the current system's generation-function table,
- the currently populated system/planet analysis data,
- orbital and surface rendering assets (topography frame, sphere/orbit rendering data, planet-side element frames),
- an orbit-mode flag, and
- resource references for translation/colormap data needed during orbital display.

**Required compatibility behavior:** Solar-system runtime state shall be scoped to the current exploration session. No solar-system state persists across exploration sessions except through the save-location and planet-info persistence paths. On exit from the exploration loop, the subsystem shall release all session state so that no stale solar-system context remains accessible to other subsystems.

### 3.2 Planet descriptor model

Each planet and moon in a generated solar system shall be described by a descriptor carrying:

- a deterministic random seed,
- a data index identifying the world type,
- the number of child bodies (moons for planets; always zero for moons),
- an orbital radius and position,
- a temperature-derived display color,
- a sort-ordering link for display traversal,
- a display stamp/image, and
- a parent-body back-reference (sun for planets, planet for moons).

### 3.3 System information model

The subsystem shall maintain per-world analysis data sufficient to represent:

- the full set of planetary physical characteristics produced by planetary analysis (§7),
- scan-retrieval masks for each scan type (mineral, energy, biological), and
- any predefined surface or special-world data supplied by generation functions.

### 3.4 World classification helpers

The subsystem shall provide classification and indexing operations:

- determining whether a world descriptor refers to a planet or a moon,
- computing the zero-based planet index or moon index from a descriptor,
- matching a world descriptor against a specific planet-index/moon-index pair, and
- querying whether the player is currently in a solar system, in an inner system, or in planet orbit.

**Required compatibility behavior:** World classification shall produce planet/moon identity results consistent with the persistence addressing scheme, so that all generation-function consumers and persistence callers agree on planet/moon identity.

---

## 4. Solar-system exploration lifecycle

### 4.1 Entry

**Required compatibility behavior:** Entering a solar system shall:

1. resolve the current star if needed,
2. update logged ship coordinates from the current star,
3. establish a fresh solar-system runtime state scoped to the exploration session,
4. select the appropriate generation-function table for the current star,
5. initialize the solar system (load planets, apply pending persistence, compute analysis, set up rendering), and
6. enter the interplanetary flight loop.

On exit from the flight loop, the subsystem shall uninitialize solar-system state and release the exploration session context.

### 4.2 Solar-system load and generation

**Required compatibility behavior:** Loading a solar system shall:

1. seed the system-generation RNG from the current star's seed,
2. set up the sun descriptor,
3. invoke the generation-function table's planet-generation handler to populate planet descriptors,
4. apply any pending planetary-change persistence by writing scan data and clearing the change flag,
5. run planetary analysis on each generated planet to derive temperature-based display colors,
6. sort planets by display position for rendering order, and
7. initialize either outer-system or inner-system state depending on the player's saved navigational position.

If the saved position indicates the player was in orbit, the load shall return the target planet or moon descriptor for orbit resumption.

### 4.3 Inner-system transition

When the player enters an inner system (approaches a planet), the subsystem shall:

1. generate moons for the target planet via the generation-function table,
2. switch the base descriptor set to moons,
3. set the orbital-target pointer to the approached planet, and
4. transition to inner-system navigation.

### 4.4 Exit

When the player leaves the solar system (navigates past the system boundary or transitions to hyperspace), the subsystem shall:

1. invoke the generation-function table's NPC-uninit handler,
2. free solar-system rendering assets, and
3. restore navigational state for the broader game.

---

## 5. Orbit entry and orbital menu

### 5.1 Orbit entry

**Required compatibility behavior:** Entering orbit around a planet or moon shall follow this sequence:

1. free interplanetary flight assets if the entry was caused by collision during flight,
2. position the ship stamp appropriately (mid-screen for planets; at the body's image origin for moons),
3. load persisted scan-retrieval masks for the target world via the persistence API,
4. perform orbit-content processing by dispatching the generation-function table's orbit-content hook for the target world,
5. check for activity interrupts (abort, load, encounter, crew loss, special game states) and return early if any are set,
6. determine whether orbital presentation can proceed: renderable orbital and surface assets are available (topography frame exists) and no active interrupt prevents entry, and
7. if orbital readiness is confirmed, perform planet loading and enter the orbital menu, then free planet assets on menu exit; if orbital readiness is not confirmed (no renderable topography, or a non-orbital interaction such as a homeworld conversation or encounter), skip the orbital menu entirely.

After orbit exit without an activity interrupt, the subsystem shall reload the solar system, revalidate orbits, and return to inner-system navigation.

### 5.2 Orbital menu

**Required compatibility behavior:** The orbital menu shall present the following actions:

| Action | Behavior |
|---|---|
| Scan | Enter scan mode (§6) |
| Equip Device | Open devices menu |
| Cargo | Open cargo menu |
| Roster | Open roster menu |
| Game Menu | Open game options |
| Starmap / Navigation | Leave orbit and return to navigation |

The orbital menu shall display a rotating planet sphere as a background element. The menu loop shall dispatch to the appropriate sub-flow based on player input and return to the menu after sub-flow completion, unless a navigation or abort action terminates the orbital session.

### 5.3 Orbit-content hook contract

**Integration contract:** The orbit-content hook in the generation-function table is the primary extension point for per-system content at orbit entry. The subsystem dispatches to this hook as part of orbit-content processing and responds to the observable outcomes:

1. **Renderable world:** The hook (or the default generation path when the hook does not override) produces renderable topography for the world (via surface generation or predefined data). After orbit-content processing completes, the subsystem observes that renderable assets are available and proceeds to planet loading and the orbital menu.
2. **Non-orbital interaction:** The hook initiates a homeworld conversation, encounter, or other interaction that sets an activity flag. The subsystem detects the resulting activity state and yields control outward without entering the orbital menu.
3. **Default behavior:** When the hook does not override the default path, the subsystem applies standard surface generation for generic worlds. Whether renderable assets result depends on the world type (e.g., gas giants produce renderable surfaces; there is no world type that silently fails to produce assets under the default path).

The subsystem does not constrain the internal behavior of the hook. The contract is defined by observable outcomes — the presence or absence of renderable assets and activity flags after orbit-content processing — not by specific return values or intermediate data structures. Any refactoring of the hook's internal interface is permissible provided the observable orbit-entry outcomes are preserved.

### 5.4 Orbit-content processing, planet loading, and phase boundaries

Orbit-content processing and planet loading are conceptually distinct phases during orbit entry, but their internal boundary may be refactored so long as the observable readiness gating and results are preserved:

- **Orbit-content processing** may directly create renderable assets (topography frame and sphere rendering data), or it may route to the default generation path that does so. In either case, the observable outcome of this phase is the presence or absence of renderable assets and any activity flags.
- **Planet loading** is the broader post-readiness setup phase. It encompasses any additional preparation needed after renderable assets are confirmed: surface-node materialization, music setup, orbital display initialization, and any further surface/rendering work around the generated topographic data.

Whether the world-generation work that establishes renderable-asset availability occurs within orbit-content processing itself, within the default generation path it invokes, or within a setup step that an implementation treats as part of planet loading is not normative. The normative obligations are:

1. persisted scan state is loaded before orbit-content processing,
2. after orbit-content processing, the subsystem observes renderable-asset availability and activity-flag state to decide whether to proceed,
3. planet loading (however internally decomposed) produces a fully materialized orbital view with surface nodes before the orbital menu becomes interactive, and
4. any refactoring of internal phase boundaries preserves these observable gates and the resulting player-visible behavior.

### 5.5 Glossary of orbit-entry terms

For clarity across this specification and the requirements document:

- **Orbit-content processing:** The phase during orbit entry when the generation-function orbit-content hook is dispatched and the subsystem's default generation path runs if the hook does not override. This phase may produce renderable assets, set activity flags, or both.
- **Planet loading:** The post-readiness setup phase comprising surface-node materialization, music setup, orbital display preparation, and any additional surface/rendering work needed to present the orbital view. This is the broader load/setup phase around generated topographic data, not necessarily a separate generation step.
- **Orbital readiness:** The condition under which the subsystem proceeds to planet loading: renderable orbital/surface assets (topography frame) are available after orbit-content processing and no active interrupt prevents entry.
- **Renderable assets:** The topography frame and associated sphere rendering data needed to present the orbital view. Their presence after orbit-content processing is the observable signal that the world supports orbital presentation.

---

## 6. Scan flow and surface-node population

### 6.1 Scan entry

**Required compatibility behavior:** Entering scan mode shall:

1. prepare scan context for the current world,
2. determine the initial scan-type menu state based on world properties (shielded worlds and gas giants restrict available scan types),
3. initialize the planet location display for normal scans,
4. draw any previously scanned objects,
5. print coarse-scan information (world analysis summary), and
6. enter the scan input loop.

### 6.2 Scan types

The subsystem shall support three scan types:

| Scan type | Index | Content |
|---|---|---|
| Mineral scan | 0 | Mineral deposits with type, density, and location |
| Energy scan | 1 | Energy sources with location |
| Biological scan | 2 | Life forms with type/species and location |

### 6.3 Surface-node population

**Required compatibility behavior:** When populating a world's surface with nodes, the subsystem shall:

1. initialize the surface display list,
2. skip population entirely for shielded worlds,
3. for each scan type (biological, energy, mineral — in that order):
   a. query the generation-function table for the total node count,
   b. for each node, check whether it has already been retrieved by testing the corresponding bit in the persisted scan-retrieval mask,
   c. for non-retrieved nodes, query the generation-function table for the node's per-node info (location, type, density),
   d. allocate a display element and populate it with scan-type-specific visual and data properties.

**Required compatibility behavior for mineral nodes:** Each mineral node shall carry its element type, gross deposit size (image selection), fine deposit size (actual quantity), and appropriate mineral-category frame indexing.

**Required compatibility behavior for energy/biological nodes:** Each energy or biological node shall carry animated scan-dot frames with a fixed animation rate, and biological nodes shall carry per-creature type and variation data constrained by the world's life-variation limit.

### 6.4 Scan-triggered encounters

**Required compatibility behavior:** Certain scan interactions may trigger encounters (e.g., the Fwiffo encounter at Pluto). When this occurs, the subsystem shall set the encounter flag and save the current solar-system location before exiting the scan flow. The encounter itself is handled outside this subsystem.

---

## 7. Planetary analysis

### 7.1 Analysis computation

**Required compatibility behavior:** Planetary analysis shall compute the following physical characteristics for a world, given the world's seed and the parent star's properties:

- star energy and intensity class,
- orbital distance from the parent body,
- surface temperature (accounting for atmospheric greenhouse effects),
- density and composition,
- radius,
- rotation period,
- surface gravity,
- axial tilt,
- tectonic activity,
- atmospheric density and pressure,
- weather severity, and
- life chance.

All derivations shall be deterministic functions of the world seed and star properties, seeded through the system-generation RNG. The same seed and star shall always produce the same analysis results.

### 7.2 Temperature-color derivation

**Required compatibility behavior:** Planetary analysis results shall include a temperature-derived display color used for rendering the planet in the solar-system view. This color is computed during solar-system load and stored in each planet descriptor.

The established baseline contains a known temperature/orbit-color mismatch related to greenhouse-like adjustments, where the temperature used for orbit-color assignment differs from the temperature displayed during coarse scan. This quirk is player-visible (it affects the color shown for certain planets in the solar-system map). For initial parity signoff, the end-state implementation shall reproduce this quirk exactly. Correcting the mismatch is classified as an intentional behavioral divergence and is deferred to a post-parity change outside the scope of initial signoff. See §15 for disposition.

### 7.3 Output fidelity

**Required compatibility behavior:** The planetary analysis outputs are embedded in:

- UI display of world properties during scan,
- temperature-color assignment in the solar-system view,
- surface-generation algorithm selection (gas giant vs. topo vs. cratered),
- node-generation parameters, and
- generation-function handlers that test analysis results to decide content.

Any end-state implementation must produce analysis results identical to the established baseline for the same inputs, verified against seeded reference cases or representative fixture outputs. The acceptance criterion is output equivalence over defined inputs, not retention of any particular internal formula decomposition.

---

## 8. Planet surface generation

### 8.1 Surface generation flow

**Required compatibility behavior:** Generating a planet's surface shall:

1. seed the generation RNG from the world descriptor's random seed,
2. initialize planet orbit rendering buffers,
3. if predefined surface/elevation frames are supplied by the generation function, use those directly,
4. otherwise, select a generation algorithm based on the world's data index:
   - gas-giant algorithm for gas-giant worlds,
   - topographic algorithm for worlds with terrain,
   - cratered algorithm for appropriate world types,
5. generate elevation data into a topographic data array,
6. produce the rendered topography frame and sphere rendering assets, and
7. store the results in the solar-system state for orbital display use.

### 8.2 Determinism

**Required compatibility behavior:** Surface generation shall be deterministic. The same world seed shall always produce the same topographic surface, elevation data, and sphere rendering. This is critical because surface node positions depend on the generated topography, and players must be able to revisit a world and find the same layout.

---

## 9. Generation-function integration

### 9.1 Generation-function table contract

**Integration contract:** The subsystem is parameterized by a per-system generation-function table. This table provides handlers for:

| Handler | Purpose |
|---|---|
| NPC init | Set up system NPC ships on first visit |
| NPC reinit | Refresh system NPC ships on subsequent visits |
| NPC uninit | Tear down system NPCs when leaving |
| Planet generation | Define planet layout within the system |
| Moon generation | Define moon layout around a specific planet |
| Name generation | Assign a name to a planet |
| Orbit content | Populate orbital characteristics and trigger orbital interactions |
| Mineral generation | Provide mineral node count and per-node info |
| Energy generation | Provide energy node count and per-node info |
| Life generation | Provide biological node count and per-node info |

### 9.2 Generation-handler contract by class

**Integration contract:** The handlers in the generation-function table fall into three distinct behavioral classes. The subsystem's dispatch semantics differ by class. This table defines the canonical contract for each:

| Handler | Class | Observable outputs | Default fallback exists | Subsystem behavior |
|---|---|---|---|---|
| Planet generation | Override/fallback | Planet descriptor array populated with count, seeds, orbital params, world types | Yes — generic planet layout | If handler indicates not-handled, subsystem invokes default planet generation |
| Moon generation | Override/fallback | Moon descriptor array populated for the target planet | Yes — generic moon layout | If handler indicates not-handled, subsystem invokes default moon generation |
| Orbit content | Override/fallback | Renderable assets and/or activity flags present after processing | Yes — default surface generation for generic worlds | If handler indicates not-handled, subsystem invokes default surface generation path; subsystem then observes asset/flag state to determine orbital readiness |
| Name generation | Override/fallback | Planet name string assigned | Yes — default naming | If handler indicates not-handled, subsystem applies default name |
| Mineral generation | Data provider | Node count (when queried for count) or per-node mineral info (when queried per node): location, element type, deposit size | Yes — default mineral generation | Handler returns count or per-node data; subsystem uses returned values to drive node population; no handled/not-handled semantic |
| Energy generation | Data provider | Node count or per-node energy info: location | Yes — default energy generation | Same as mineral generation |
| Life generation | Data provider | Node count or per-node biological info: location, creature type, variation | Yes — default life generation | Same as mineral generation |
| NPC init | Side-effect/integration hook | NPC ships spawned in the system | Yes — no-op default | Dispatched on first system visit; subsystem does not interpret return value; observable effect is NPC presence |
| NPC reinit | Side-effect/integration hook | NPC ships refreshed | Yes — no-op default | Dispatched on subsequent system visits; same semantics as NPC init |
| NPC uninit | Side-effect/integration hook | NPC ships removed | Yes — no-op default | Dispatched on system exit; subsystem does not interpret return value |

**Key distinctions:**

- **Override/fallback handlers** return a value indicating whether the handler fully handled the request. If not handled, the subsystem invokes its default behavior for that slot. The observable output is the generated data or state change that results.
- **Data-provider handlers** return counts or per-node data directly. There is no handled/not-handled indicator; the returned value is the data itself. The subsystem uses these values to drive node-population iteration.
- **Side-effect/integration hooks** are dispatched for their external effects (NPC lifecycle, node-pickup consequences). The subsystem does not branch on their return value. Default implementations exist and are typically no-ops.

### 9.3 Node-pickup integration hooks

**Integration contract:** The generation-function table includes per-scan-type node-pickup hooks (mineral, energy, biological). These are side-effect/integration hooks: they are dispatched when the lander subsystem reports a pickup event. The planet-solarsys subsystem dispatches these hooks; the lander subsystem provides the trigger. This subsystem owns the effect on node retrieval state and content outcomes. Lander traversal and interaction mechanics are out of scope.

### 9.4 Dispatch convention

**Integration contract:** Override/fallback handlers return a value indicating whether the handler fully handled the request. If the handler indicates it did not handle the request, the subsystem shall invoke the default generation behavior. This allows system-specific handlers to override only the aspects they need to customize.

Data-provider handlers (mineral, energy, life generation) return counts or per-node data directly. The subsystem uses these values to drive node population. There is no handled/not-handled semantic for these handlers.

Side-effect/integration hooks (NPC init/reinit/uninit, node-pickup hooks) are dispatched for their external effects. The subsystem does not branch on their return values.

### 9.5 Generation-function selection

**Required compatibility behavior:** The generation-function table for a system shall be selected by the current star's index via a lookup function. Each star that has system-specific content shall have a dedicated generation-function table; all other stars shall use the default generation-function table.

---

## 10. Persistence-sensitive behavior

### 10.1 Planet scan-state persistence

**Persistence contract:** The subsystem depends on a planet-info persistence API that supports the following operations:

| Operation | Semantics |
|---|---|
| Init | Initialize the persistence store for all stars (set all offsets to zero/empty) |
| Uninit | Tear down the persistence store |
| Get | Read scan-retrieval masks for the current orbital target into the system-info structure |
| Put | Write scan-retrieval masks for the current orbital target from the system-info structure |

The subsystem owns the calling contract for get and put: it calls get on orbit entry before orbit-content processing, and calls put at solar-system load and save-location encoding when the planetary-change flag is set. Init and uninit are hosting obligations — the persistence infrastructure must be initialized before the subsystem's first get/put call and torn down after the subsystem's last use. The subsystem depends on this guarantee but does not own the init/uninit lifecycle timing.

**Hosting lifecycle boundary events (authoritative contract in `campaign-gameplay/specification.md` §2.2):** The hosting layer (campaign gameplay) shall initialize the persistence infrastructure before any load path that can resume into interplanetary or orbit state. The infrastructure shall remain live for the entire solar-system session, including orbit-entry get operations and save-location encoding put operations. Teardown shall occur only after solar-system uninit completes and after any pending put triggered by session exit or save has completed. Campaign transitions that tear down solar-system state (e.g., starbase entry, encounter exit, load into non-solar-system state) shall ensure all pending persistence writes are flushed before teardown.

**Required hosting precondition and failure attribution:** All get and put operations in this subsystem are conditioned on the hosting lifecycle guarantees above. The first legal get/put call site is orbit entry during an active solar-system session after campaign has initialized the persistence infrastructure. The last legal call site is save-location encoding during session exit, before campaign tears down the persistence infrastructure. Planet-solarsys does not detect or recover from hosting-lifecycle violations; it assumes the host guarantee is met. If the persistence infrastructure is not initialized when a get or put is called, the resulting behavior is undefined and any failure is attributed to the hosting layer (campaign gameplay), not to planet-solarsys. If planet-solarsys calls get or put outside the hosting-guaranteed window (e.g., after solar-system uninit has completed), that is a planet-solarsys conformance violation. `campaign-gameplay/specification.md` §2.2 is the authoritative owner of the lifecycle timing contract.

### 10.2 Scan-retrieval mask model

**Persistence contract:** Each planet and each moon shall have an independent set of three scan-retrieval masks (one per scan type: mineral, energy, biological). Each mask is a bitmask where each bit corresponds to a surface node. A set bit indicates the node has been retrieved (picked up) and should not be materialized on subsequent visits.

### 10.3 Persistence addressing

**Persistence contract:** Scan-retrieval records shall be addressed by:

- **star index:** the index of the current star in the star array,
- **planet index:** the zero-based index of the planet in the current system's planet descriptor array, and
- **moon index:** zero for the planet itself, or 1-based index for a moon.

**Semantic contract:** The persistence addressing depends on the generated planet/moon layout of each system (planet count, per-planet moon counts), because preceding planets' moon counts determine the offset of later planets' records. Any implementation must preserve this semantic addressing relationship so that the correct scan-retrieval record is loaded for each world, regardless of how records are physically stored.

**Current backing layout (informational):** In the current implementation, records are stored in contiguous blocks ordered by planet with each planet's record followed by its moons' records, and a per-star offset table at the beginning of the store indexes into these blocks. An alternative storage layout is permissible if the semantic addressing and round-trip behavior described in §10.5 are preserved.

### 10.4 Persistence timing

**Required compatibility behavior:**

- Get shall be called on orbit entry, before orbit-content processing.
- Put shall be called:
  - during solar-system load, if the planetary-change flag is set (committing pending changes from the previous session), and
  - during save-location encoding while in orbit, if the planetary-change flag is set.
- After each put call triggered by the planetary-change flag, the flag shall be cleared.

### 10.5 Persistence compatibility obligations

**Persistence contract:**

1. **Legacy-load compatibility:** Save files produced by the established baseline game version containing planet scan masks and orbital positions shall load with identical world identity and retrieval-state outcomes in the end-state implementation.
2. **Save/reload round-trip:** Saving and then reloading in the end-state implementation shall preserve the same orbital target identity and the same retrieved-node suppression state. No nodes shall reappear or disappear as a result of a save/reload cycle.
3. **Semantic over layout:** The semantic obligations (correct world addressing, correct retrieval-mask content, correct round-trip behavior) are primary. Any redesign of the backing storage layout is permissible if these semantic obligations and the legacy-load compatibility obligation are met.

### 10.6 Node-retrieval filtering

**Required compatibility behavior:** When populating surface nodes (§6.3), each node shall be tested against the appropriate scan-retrieval mask bit. The test uses the scan type and node index to check the corresponding bit. Nodes whose bits are set shall be skipped (not materialized).

---

## 11. Save-location encoding

### 11.1 Save-location structure

**Persistence contract:** The subsystem shall encode the player's in-system position for save/load using the following scheme:

- **Outside orbit:** The non-orbital location is saved through a separate path that records navigational position and the current planet/inner-system state.
- **In orbit:** The subsystem encodes the orbital position using a scheme that distinguishes the planet itself from each of its moons. The encoding shall ensure that the in-orbit indicator is nonzero whenever the player is in orbit.

### 11.2 Save-location restoration

**Required compatibility behavior:** On solar-system load, if the in-orbit indicator is nonzero, the subsystem shall decode the value to determine whether to resume at the planet or one of its moons, and return the appropriate planet/moon descriptor for orbit resumption.

### 11.3 Save-location compatibility

**Persistence contract:** The orbital-position encoding shall be compatible with the established save-format contract. Save files produced by the baseline game version with in-orbit positions shall decode correctly in the end-state implementation, resuming orbit around the same world.

---

## 12. Cross-subsystem boundaries

### 12.1 Campaign/story orchestration boundary

System-specific story content is injected through generation-function handlers. The planet-solarsys subsystem dispatches to them but does not define their content. Handlers may set encounter flags, initiate conversations, or modify game state; the subsystem propagates those effects but does not interpret them beyond checking for activity interrupts that terminate the current flow.

### 12.2 Ships/combat boundary

Orbit and scan flows can produce `START_ENCOUNTER` as an outcome. The subsystem's responsibility ends at setting that flag and saving the current location. The encounter/combat resolution is handled by external systems. On return from an encounter, the subsystem reloads the solar system and revalidates state.

### 12.3 Lander boundary

The subsystem generates the surface topography and populates it with nodes. The lander subsystem takes over for surface gameplay. The division is:

- **This subsystem owns:** node presence (what nodes exist on the surface), node identity (type, location, quantity), and retrieval-state integration (which nodes have been picked up, as reflected in scan-retrieval masks and updated via node-pickup hooks).
- **The lander owns:** surface traversal, hazard interaction, cargo collection mechanics, crew/resource accounting, and the decision to trigger a pickup event.

Node-pickup hooks in the generation-function table are side-effect/integration hooks: the lander triggers the event, the hook executes, and this subsystem dispatches the hook and integrates the persistence/content outcome. The hook does not define lander gameplay rules.

### 12.4 Lower-level engine boundary

The subsystem depends on lower-level facilities for:

- **Graphics:** context and drawable management, frame allocation, rendering primitives, sphere rotation rendering.
- **Audio:** planet music selection and playback.
- **Input:** the input-loop driver that runs the interplanetary flight and orbital menu loops.
- **Resource:** asset loading for planet-side frames, surface definition frames, colormaps, and string resources.
- **State file I/O:** the persistence storage backend for scan-retrieval masks.

These are infrastructure dependencies, not subsystem-owned logic.

---

## 13. Determinism and RNG contract

### 13.1 Determinism obligation

**Required compatibility behavior:** All procedural generation within this subsystem — planet layout, moon layout, planetary analysis, surface topography, and node generation — shall produce deterministic, reproducible outputs for a given star seed. The same star seed shall always produce the same planet layout, analysis results, surface topography, and node positions.

### 13.2 Seed derivation

**Required compatibility behavior:** The star seed shall be derived deterministically from the star descriptor. The same star shall always produce the same seed, which shall always produce the same generated outputs.

### 13.3 RNG isolation

**Required compatibility behavior:** Procedural generation for solar-system content shall use a dedicated RNG context that is isolated from unrelated game activity (e.g., combat, UI animation). Generation results shall not be affected by the order or occurrence of unrelated gameplay events.

### 13.4 Determinism acceptance

**Required compatibility behavior:** Determinism shall be verifiable through seeded reference cases: for representative star/world seeds, the generated planet layout, analysis values, surface topography, and node populations shall match the outputs produced by the established baseline implementation. The acceptance criterion is output equivalence over defined seed inputs, not identity of internal call graphs or data structures.

---

## 14. System limits

The following limits are part of the externally observable contract:

| Limit | Value | Significance |
|---|---|---|
| Maximum suns per system | 1 | All systems are single-star |
| Maximum planets per system | 16 | Planet descriptor array size |
| Maximum moons per planet | 4 | Moon descriptor array size |
| Scan types | 3 | Mineral, energy, biological |
| Planet-side element frame slots | 3 + life-variation max | Bio canister, energy node, reserved, plus per-world bio variations |

---

## 15. Open decisions and signoff blockers

The following areas are not fully settled and may require audit or deliberate design decisions before final signoff:

1. **Orbit-content hook interface cleanup:** The current generation-function interface for orbit-content dispatch has known architectural awkwardness (acknowledged in the source). Potential improvements include separating calculation from activation, clarifying return semantics, and reducing coupling. Whether this cleanup is mandatory for initial parity or deferred to a later pass is an open decision. Any cleanup must preserve the observable orbit-entry outcomes defined in §5.3.

2. **Persistence addressing stability:** The current scan-record addressing scheme is structurally coupled to generated moon counts. If planet or moon generation is ever changed (different counts, different ordering), existing save files would address the wrong records. Whether to introduce a more robust addressing scheme (e.g., seed-based world identity) or to preserve the current scheme exactly is an open decision with save-compatibility implications. Any change must satisfy the persistence compatibility obligations in §10.5.

3. **Temperature/orbit-color quirk:** The established baseline contains a known temperature/orbit-color mismatch related to greenhouse-like adjustments. **Decision: parity-first.** The end-state implementation shall reproduce this quirk exactly for initial parity signoff. The mismatch is player-visible (it affects solar-system map planet colors) and falls under the baseline-equivalence obligation. Correcting it is classified as an intentional behavioral divergence to be tracked and addressed in a post-parity change outside the scope of initial signoff. This decision is consistent with the determinism and compatibility obligations in §7, §13, and Appendix A.

---

## 16. Items explicitly out of scope

The following items are explicitly excluded from this specification:

- **Hyperspace navigation and interplanetary display driver:** The broader interplanetary display and hyperspace travel logic is campaign-level infrastructure, not planet-solarsys-owned.
- **Lander internal mechanics:** Surface traversal, hazard damage, crew loss, cargo collection, and lander return logic are part of the lander subsystem.
- **Per-race generation-function content:** The 50+ system-specific generation functions (one per special star system) are content, not infrastructure. This subsystem defines the dispatch contract; the content implementations are out of scope.
- **Starmap:** The planetary starmap is a navigational UI that reads star data but is not part of the exploration/orbit/scan flow.
- **Save/load file format:** The broader save/load infrastructure defines file formats and serialization. This subsystem defines what it persists (scan masks, location encoding) and the semantic compatibility obligations, but not the container format.
- **Build/config toggle design:** How the subsystem is gated by build configuration is a project-level infrastructure decision, not a subsystem behavioral contract.

---

## Appendix A. Minimum acceptance evidence

The following defines the minimum representative corpus required for signoff on baseline-equivalence claims made throughout this specification. This is not a full test plan; it defines the minimum evidence scope that acceptance must cover.

### A.1 Seeded reference systems

Acceptance shall include verification against a representative set of seeded star systems covering:

- **Normal systems:** At least several generic systems (using the default generation-function table) spanning a range of planet counts, moon counts, and world types.
- **System-specific content:** At least several systems with dedicated generation-function tables (e.g., Sol, homeworld systems, encounter-bearing systems) to verify generation-function dispatch and orbit-content hook behavior.
- **Gas-giant worlds:** At least one system containing gas-giant planets, verifying correct algorithm selection, scan-type restrictions, and orbital presentation.
- **Shielded worlds:** At least one shielded world, verifying node-population suppression and scan-type restrictions.
- **Encounter-triggering worlds:** At least one world whose orbit-content hook or scan interaction triggers an encounter (e.g., Pluto/Fwiffo), verifying activity-flag propagation and location save behavior.
- **Moon-bearing planets:** At least several planets with moons, verifying moon generation, inner-system navigation, and persistence addressing across planet-and-moon layouts.

### A.2 Determinism verification

For each seeded reference system, acceptance shall verify that the generated outputs (planet layout, planetary analysis values, surface topography, and surface-node populations) match the outputs produced by the established baseline implementation for the same seed inputs.

### A.3 Legacy save fixtures

Acceptance shall include verification against legacy save files produced by the established baseline game version, covering at minimum:

- **Planet orbital restoration:** A save file with the player in orbit around a planet, verifying correct orbital-position decoding and orbit resumption.
- **Moon orbital restoration:** A save file with the player in orbit around a moon, verifying correct moon-index decoding and orbit resumption at the correct moon.
- **Retrieved-node suppression:** A save file where nodes have been previously retrieved on at least one world, verifying that the correct nodes are suppressed (not re-materialized) after load and that no extra nodes appear or disappear.
- **Pending planetary-change commit:** A save file with the planetary-change flag set, verifying that pending scan-state changes are committed on solar-system reload.

### A.4 Round-trip verification

Acceptance shall verify save/reload round-trip fidelity: saving in the end-state implementation and reloading shall preserve orbital target identity and retrieved-node suppression state without node reappearance or disappearance.

### A.5 Acceptance observables

This section defines the minimum observable artifacts that must match the established baseline for each parity/compatibility class. These are the normative comparison dimensions for signoff — what must be compared, not how tests are structured.

| Parity class | Observable artifacts that must match baseline |
|---|---|
| **World layout** | Planet count, planet seeds, world-type data indices, orbital radii, orbital positions, moon counts per planet, moon seeds, moon world-type indices, moon orbital parameters, display sort ordering |
| **Planetary analysis** | All scalar analysis outputs for each world: temperature, density, radius, gravity, rotation period, tilt, tectonics, atmospheric density, weather classification, life chance; temperature-derived orbit-display color (including the greenhouse-adjustment quirk per §7.2) |
| **Surface generation** | Topographic elevation data, rendered topography frame content, sphere rendering asset content — all deterministic for a given world seed and algorithm selection |
| **Node population** | Per-scan-type node counts, per-node locations, per-node type/species/element identifiers, per-node quantity/density values, scan-retrieval mask suppression outcomes (which nodes are materialized vs. filtered) |
| **Persistence/save compatibility** | Orbital-position encoding/decoding round-trip identity, scan-retrieval mask content after get/put round-trip, legacy save file load producing identical world identity and retrieval-state outcomes, no node reappearance or disappearance across save/reload |
| **Generation-handler integration** | System-specific handlers dispatched for the correct star, override/fallback semantics preserved (handler override suppresses default; non-override invokes default), data-provider handlers return values consumed identically for node population, side-effect hooks dispatched at correct lifecycle points. Behavioral compatibility is required; binary/source-level interface identity is not |
