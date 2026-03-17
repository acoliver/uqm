# Planet-SolarSys Subsystem Requirements

## Purpose

This document defines the required externally observable behavior of the planet-solarsys subsystem in EARS-like form. The subsystem covers solar-system exploration entry and lifetime, interplanetary and inner-system navigation, planet/moon orbit entry, the orbital menu, scan behavior, planet surface generation, planetary analysis and generation integration, persistence-sensitive scan/save-location behavior, and the generation-function injection contract.

## Scope boundaries

- Campaign/story orchestration, encounter scripting, and system-specific NPC/content logic are outside this subsystem. The subsystem dispatches to injected generation functions but does not own story content.
- Ship combat, encounter dialogue, and encounter resolution are outside this subsystem. The subsystem may set activity flags that trigger encounters but does not own encounter execution.
- Lower-level graphics, audio, input, resource, element/display-list, and state-file infrastructure are outside this subsystem, though the subsystem depends on them.
- The lander surface-exploration gameplay (surface traversal, hazard interaction, cargo pickup mechanics, crew/resource accounting, lander movement) is outside this subsystem. The subsystem owns node presence, node identity, and retrieval-state integration but does not own lander interaction.

## Solar-system exploration entry

- **When** the player enters a solar system, **the subsystem shall** resolve the current star and update the logged ship coordinates from the current star.
- **When** the player enters a solar system, **the subsystem shall** establish a fresh solar-system runtime state scoped to the exploration session.
- **When** the player enters a solar system, **the subsystem shall** select the appropriate generation-function table for the current star and initialize the solar system.
- **When** the player enters a solar system and initialization is complete, **the subsystem shall** enter the interplanetary flight loop.

## Solar-system exit

- **When** the player leaves a solar system, **the subsystem shall** uninitialize solar-system state and release solar-system-owned runtime resources.
- **When** the player leaves a solar system, **the subsystem shall** clear the active solar-system context so that no stale solar-system state remains accessible to other subsystems.
- **Ubiquitous:** At most one solar-system context shall be active at a time. The subsystem shall not permit overlapping or nested solar-system sessions.

## Solar-system initialization

- **When** a solar system is initialized, **the subsystem shall** seed generation from the current star's seed, configure the sun descriptor, and invoke the injected planet-generation function to produce the planet layout.
- **When** a solar system is initialized and a pending planetary change exists, **the subsystem shall** commit the pending scan state to persistence and clear the planetary-change flag before continuing initialization.
- **When** a solar system is initialized, **the subsystem shall** compute temperature classification for each generated planet via planetary analysis.
- **When** a solar system is initialized, **the subsystem shall** sort planet positions for display ordering and initialize either outer-system or inner-system navigation state depending on the persisted navigation position.
- **When** initialization determines that the player was last positioned in the inner system, **the subsystem shall** generate moons for the active planet and initialize inner-system navigation state.
- **When** initialization determines that the player was last in orbit around a planet or moon, **the subsystem shall** identify the correct planet or moon to resume orbit around and return that target to the caller.

## World-layout determinism

- **Ubiquitous:** The planet layout produced by the injected generation function, including planet count, orbital parameters, and world seeds, shall be deterministic for a given star seed.
- **Ubiquitous:** The moon layout produced by the injected generation function shall be deterministic for a given planet seed and parent star.

## Interplanetary navigation

- **Ubiquitous:** The subsystem shall support outer-system navigation among planets and inner-system navigation among a planet's moons.
- **When** the player transitions from outer-system to inner-system navigation by approaching a planet, **the subsystem shall** generate the planet's moons, initialize inner-system state, and enter inner-system navigation.
- **When** the player transitions from inner-system back to outer-system navigation, **the subsystem shall** release inner-system state and restore outer-system navigation.
- **Ubiquitous:** The subsystem shall track interplanetary flight state, including whether the ship is currently in interplanetary flight and any collision/orbit gating state that prevents re-entering orbit immediately after leaving it.

## Orbit entry

- **When** the player reaches a planet or moon for orbit entry, **the subsystem shall** release interplanetary flight assets if the transition was triggered by a collision during flight.
- **When** the player enters orbit, **the subsystem shall** load persisted scan state for the target world before orbit-content processing.
- **When** the player enters orbit, **the subsystem shall** perform orbit-content processing by dispatching the injected orbit-content hook for the target world. Orbit-content processing may directly create renderable assets or may route to the default generation path that does so.
- **When** orbit-content processing is complete, **the subsystem shall** check for activity interrupts (abort, load, encounter, crew loss, special game states) and return early if any are set.
- **When** orbit-content processing is complete, no activity interrupt is active, and renderable orbital/surface assets are available (topography frame exists), **the subsystem shall** perform planet loading and present the orbital menu. Planet loading is the broader post-readiness setup phase: it encompasses surface-node materialization, music setup, orbital display preparation, and any additional surface/rendering work around generated topographic data.
- **When** orbit-content processing is complete and either no renderable assets are available or a non-orbital interaction (homeworld conversation, encounter, or other non-landable outcome) has been initiated, **the subsystem shall** skip the orbital menu entirely.
- **When** the player leaves orbit without an activity interrupt, **the subsystem shall** free orbital and planet assets, reload the solar system, and revalidate orbital state to ensure consistent resumption of interplanetary navigation.
- **Ubiquitous:** Internal phase boundaries between orbit-content processing and planet loading may be refactored so long as the observable readiness gating (scan state loaded before processing, renderable-asset/activity-flag check after processing, fully materialized orbital view before menu interaction) and player-visible results are preserved.

## Orbital menu

- **Ubiquitous:** The orbital menu shall present the following actions: scan, equip device, cargo, roster, game menu, starmap, and navigation.
- **When** the player selects scan, **the subsystem shall** enter scan mode.
- **When** the player selects starmap or navigation, **the subsystem shall** exit the orbital menu and leave orbit.
- **When** the player selects equip device, cargo, roster, or game menu, **the subsystem shall** dispatch to the corresponding external subsystem and return to the orbital menu when that subsystem yields control.
- **Ubiquitous:** While in the orbital menu, the subsystem shall maintain and display a rotating-planet visual consistent with the current world's generated topography.

## Scan behavior

- **When** scan mode is entered, **the subsystem shall** prepare the scan context for the current world.
- **When** scan mode is entered for a shielded or gas-giant world, **the subsystem shall** restrict available scan types appropriately.
- **When** scan mode is entered, **the subsystem shall** display the planet surface image, draw any previously scanned objects, display coarse-scan information, and enter the scan interaction loop.
- **When** the player exits scan mode, **the subsystem shall** clean up scan display state and return to the orbital menu.
- **When** a scan interaction triggers an encounter, **the subsystem shall** set the encounter activity flag, save the current solar-system location, and exit scan mode to encounter handling.

## Planet surface generation

- **When** a planet's surface is generated, **the subsystem shall** seed generation from the planet's world seed, initialize orbit rendering buffers, and produce topography using the planet's algorithm classification (gas giant, standard topographic, cratered, or equivalent algorithm type).
- **When** the planet has predefined surface or elevation data, **the subsystem shall** use the predefined data instead of procedural generation.
- **Ubiquitous:** Surface generation for a given world seed and algorithm classification shall produce deterministic topography, elevation data, and sphere rendering assets.

## Surface-node materialization

- **When** surface nodes are materialized for a scannable world, **the subsystem shall** initialize the surface display list.
- **When** surface nodes are materialized for a shielded world, **the subsystem shall** skip node generation entirely.
- **When** surface nodes are materialized, **the subsystem shall** iterate over each scan type (biological, energy, mineral in that order) and query the injected generation functions for node counts and per-node information.
- **When** surface nodes are materialized, **the subsystem shall** filter out already-retrieved nodes using persisted scan-retrieval masks so that nodes the player has already collected do not reappear.
- **When** surface nodes are materialized, **the subsystem shall** allocate display elements for each remaining un-retrieved node with type-appropriate visual and data attributes.

## Planetary analysis — seed and derivation

- **When** planetary analysis is performed for a world, **the subsystem shall** seed generation from the world's seed and derive stellar energy and intensity from the parent star.
- **When** planetary analysis is performed for a world, **the subsystem shall** compute: orbital distance, surface temperature, density, radius, rotation period, gravity, axial tilt, tectonics, atmospheric density, weather classification, and life chance from the seeded generation state and stellar inputs.

## Planetary analysis — output equivalence

- **Ubiquitous:** Planetary analysis shall produce results identical to the established baseline for the same inputs, verified against seeded reference cases or representative fixture outputs.
- **Ubiquitous:** Planetary analysis outputs are embedded in UI display (coarse-scan information, temperature-classification coloring), surface-generation algorithm selection, node-generation parameters, and generation-function handler decisions. Any deviation from baseline outputs for the same inputs is a compatibility break.

## Planetary analysis — quirk preservation for initial parity

- **Ubiquitous:** For initial parity signoff, the subsystem shall preserve the established baseline temperature/orbit-color behavior exactly, including the known mismatch related to greenhouse-like adjustments.
- **Ubiquitous:** Any change that makes the solar-system map temperature coloring fully consistent with coarse-scan temperature display is an intentional behavioral divergence and is outside the scope of initial parity signoff.



## Generation-function injection contract

- **Ubiquitous:** The subsystem shall be parameterized by a per-system generation-function table that is selected per star and injected at solar-system initialization time.
- **Ubiquitous:** The generation-function table shall include handlers for: solar-system NPC setup and teardown, planet layout generation, moon layout generation, orbit-content generation, mineral node generation, energy node generation, life node generation, and node-pickup integration hooks.
- **Ubiquitous:** The subsystem's generic exploration, orbit, scan, and surface-generation flows shall dispatch to the injected generation functions at the appropriate points rather than embedding system-specific content directly.
- **Ubiquitous:** System-specific world-generation behavior shall vary by star according to the established generator dispatch contract, where each star with system-specific content maps to a dedicated generation-function table and all other stars use the default table.

## Generation-handler dispatch semantics

The generation-function table contains three distinct classes of handlers. The subsystem's dispatch behavior differs by class, and the requirements below preserve those distinctions.

### Override/fallback handlers

- **Ubiquitous:** Planet generation, moon generation, orbit-content, and name-generation handlers shall follow override/fallback dispatch: the handler indicates whether it fully handled the request; if not handled, the subsystem shall invoke default generation behavior for that slot.
- **Ubiquitous:** For orbit-content handlers, "fully handled" means the handler produced renderable assets and/or set activity flags such that the subsystem's default surface-generation path is not needed. If the handler does not override, the subsystem shall invoke the default path, which produces renderable assets for generic worlds.

### Data-provider handlers

- **Ubiquitous:** Mineral-generation, energy-generation, and life-generation handlers shall follow data-provider dispatch: the handler returns node counts (when queried for count) or per-node data (when queried per node). There is no handled/not-handled semantic; the returned value is the data itself, consumed by the subsystem to drive node-population iteration.
- **Ubiquitous:** Default data-provider implementations shall exist for all node-generation slots, providing baseline mineral, energy, and life generation for generic worlds.

### Side-effect/integration hooks

- **Ubiquitous:** NPC init, NPC reinit, NPC uninit, and node-pickup hooks shall follow side-effect/integration dispatch: the subsystem dispatches these hooks for their external effects (NPC lifecycle management, node-pickup consequences) and does not branch on their return values.
- **Ubiquitous:** Default implementations for side-effect hooks shall exist and shall be no-ops where no system-specific behavior is defined.

## Planet and moon classification

- **Ubiquitous:** The subsystem shall distinguish planets from moons and provide classification and indexing operations that correctly identify whether a given world descriptor refers to a planet or a moon and return the appropriate zero-based index within its category.
- **Ubiquitous:** Planet indices and moon indices produced by the subsystem's classification operations shall be consistent with the indices used by the persistence subsystem for scan-retrieval mask addressing.

## Scan-state persistence — loading

- **When** the player enters orbit around a planet or moon, **the subsystem shall** load the persisted scan-retrieval masks for that world before orbit-content processing and before any scan-dependent display or interaction occurs.
- **Ubiquitous:** Scan-retrieval masks shall cover all scan types (mineral, energy, biological) and shall accurately reflect which nodes have been previously retrieved.

## Scan-state persistence — writeback

- **When** a planetary change has occurred and the subsystem re-enters solar-system loading, **the subsystem shall** commit the updated scan state to persistence and clear the planetary-change flag before continuing initialization.
- **When** a planetary change has occurred and the subsystem saves the solar-system location while in orbit, **the subsystem shall** commit the updated scan state to persistence and clear the planetary-change flag before recording the orbital location.

## Save-location encoding

- **When** the player's position is saved while outside orbit, **the subsystem shall** delegate to the non-orbital location-saving path.
- **When** the player's position is saved while in orbit, **the subsystem shall** first commit any pending scan-state changes, then encode the orbital position using a scheme that distinguishes the planet itself from each of its moons.
- **Ubiquitous:** The orbital-position encoding shall use a convention where one value identifies the planet and higher values identify moons with an additional offset, consistent with the established save-format contract.

## Save-location restoration

- **When** the solar system is loaded with a persisted in-orbit position, **the subsystem shall** decode the orbital-position encoding and correctly identify whether to resume orbit around the planet or a specific moon.

## Persistence addressing

- **Ubiquitous:** The persistence addressing scheme for scan-retrieval masks shall correctly map each world (identified by star index, planet index, and moon index within the generated layout) to its scan-retrieval record. Changes to the addressing scheme require explicit migration or compatibility handling.
- **Ubiquitous:** The subsystem shall use the same persistence addressing when reading scan state (orbit entry) and writing scan state (planetary change commit), ensuring round-trip consistency.

## Legacy-load compatibility

- **Ubiquitous:** Save files produced by the established baseline game version containing planet scan masks shall load with identical retrieval-state outcomes in the end-state implementation.
- **Ubiquitous:** Save files produced by the established baseline game version containing orbital positions shall load with identical world identity and orbital-target outcomes in the end-state implementation.

## Save/reload round-trip

- **Ubiquitous:** Saving and then reloading in the end-state implementation shall preserve the same orbital target identity.
- **Ubiquitous:** Saving and then reloading in the end-state implementation shall preserve the same retrieved-node suppression state. No nodes shall reappear or disappear as a result of a save/reload cycle.

## Cross-subsystem interaction boundaries

### Encounter boundary

- **When** the subsystem sets an encounter activity flag during orbit entry or scan interaction, **the subsystem shall** save the current solar-system location and yield control so that encounter handling can proceed. The subsystem does not own encounter execution.

### State/persistence boundary

- **Ubiquitous:** The subsystem shall read and write planet scan-retrieval state through the established persistence API. The subsystem does not own the persistence storage implementation.
- **Ubiquitous:** The subsystem depends on the persistence store being initialized before the subsystem's first get/put call and torn down after the subsystem's last use. Persistence init/uninit timing is a hosting obligation, not a subsystem-owned lifecycle responsibility.

### Navigation-state boundary

- **Ubiquitous:** The subsystem shall read and write interplanetary position, current planet index, and in-orbit state through the established global navigation state. The subsystem owns the interpretation of that state during solar-system exploration but does not own the global state storage mechanism.

### Lander boundary

- **Ubiquitous:** The subsystem owns node presence (what nodes exist on the surface), node identity (type, location, quantity), and retrieval-state integration (updating scan-retrieval masks when nodes are picked up). The lander subsystem owns surface traversal, hazard interaction, cargo collection, and the decision to trigger a pickup event.
- **When** a node-pickup event is reported by the lander, **the subsystem shall** dispatch the appropriate generation-function pickup hook (a side-effect/integration hook) and integrate the resulting persistence and content effects. The subsystem does not define lander gameplay rules.

## Error handling and robustness

- **When** a world has no renderable topography after orbit-content processing, **the subsystem shall** skip planet loading and the orbital menu, returning the player to interplanetary navigation without error.
- **When** surface-node materialization encounters a shielded world, **the subsystem shall** skip node generation entirely rather than generating invisible or inaccessible nodes.
- **When** gas-giant restrictions apply during scan mode, **the subsystem shall** limit available scan interactions appropriately rather than displaying inapplicable scan options.

## Determinism obligations

- **Ubiquitous:** The subsystem shall preserve the deterministic world-generation contract: for any given star seed and world seed, the generated planet layout, moon layout, surface topography, planetary analysis results, and surface-node population shall match the established baseline outputs.
- **Ubiquitous:** Determinism shall be verifiable through seeded reference cases: for representative star/world seeds, the generated outputs shall match the outputs produced by the established baseline implementation. The acceptance criterion is output equivalence over defined seed inputs.

## Compatibility obligations

- **Ubiquitous:** The subsystem shall preserve the save-format contract for orbital-position encoding so that save files produced by the established game version remain loadable with identical orbital-target outcomes.
- **Ubiquitous:** The subsystem shall preserve the save-format contract for scan-retrieval mask addressing so that save files produced by the established game version remain loadable with identical retrieval-state outcomes.
- **Ubiquitous:** The subsystem shall preserve the generation-function dispatch contract so that system-specific generators written against the established handler interface continue to operate correctly. Behavioral compatibility is required: override/fallback, data-provider, and side-effect/integration dispatch semantics must be preserved. Binary or source-level interface identity is not required.
- **Ubiquitous:** The subsystem shall preserve the externally observable orbital menu actions, scan-mode restrictions, and navigation flow so that player-facing behavior matches the established game behavior.
- **Ubiquitous:** The known temperature/orbit-color quirk shall be preserved for initial parity signoff. Correction of this quirk is deferred as a post-parity intentional divergence.

## Appendix: Minimum acceptance evidence

The following defines the minimum representative evidence required for signoff. This is not a full test plan; it defines the categories of evidence that acceptance must cover.

### Seeded reference systems

Acceptance shall include verification against representative seeded star systems covering:

- **Normal systems:** Several generic systems using the default generation-function table, spanning a range of planet counts, moon counts, and world types.
- **System-specific content:** Several systems with dedicated generation-function tables to verify generation-function dispatch and orbit-content hook behavior.
- **Gas-giant worlds:** At least one system containing gas-giant planets, verifying algorithm selection, scan-type restrictions, and orbital presentation.
- **Shielded worlds:** At least one shielded world, verifying node-population suppression and scan-type restrictions.
- **Encounter-triggering worlds:** At least one world whose orbit-content hook or scan interaction triggers an encounter, verifying activity-flag propagation and location save behavior.
- **Moon-bearing planets:** Several planets with moons, verifying moon generation, inner-system navigation, and persistence addressing across planet-and-moon layouts.

### Determinism verification

For each seeded reference system, acceptance shall verify that generated planet layout, planetary analysis values, surface topography, and surface-node populations match the baseline implementation outputs for the same seed inputs.

### Legacy save fixtures

Acceptance shall include verification against legacy save files from the established baseline game version, covering:

- **Planet orbital restoration:** A save with the player in orbit around a planet, verifying orbital-position decoding and orbit resumption.
- **Moon orbital restoration:** A save with the player in orbit around a moon, verifying moon-index decoding and orbit resumption at the correct moon.
- **Retrieved-node suppression:** A save where nodes have been previously retrieved, verifying that correct nodes are suppressed after load and no extra nodes appear or disappear.
- **Pending planetary-change commit:** A save with the planetary-change flag set, verifying that pending scan-state changes are committed on solar-system reload.

### Round-trip verification

Acceptance shall verify that saving in the end-state implementation and reloading preserves orbital target identity and retrieved-node suppression state without node reappearance or disappearance.

### Acceptance observables

The following table defines the minimum observable artifacts that must match the established baseline for each parity/compatibility class. These are the normative comparison dimensions for signoff. See the specification (Appendix A.5) for the canonical definition; this summary is provided for requirements-level traceability.

| Parity class | Observable artifacts that must match baseline |
|---|---|
| **World layout** | Planet count, seeds, world-type indices, orbital radii/positions, moon counts, moon seeds, moon parameters, display sort ordering |
| **Planetary analysis** | All scalar analysis outputs per world (temperature, density, radius, gravity, rotation, tilt, tectonics, atmospheric density, weather, life chance); temperature-derived orbit-display color including the greenhouse-quirk |
| **Surface generation** | Topographic elevation data, topography frame content, sphere rendering assets — deterministic per world seed and algorithm |
| **Node population** | Per-scan-type node counts, per-node locations, per-node type/element/species identifiers, per-node quantity/density, scan-mask suppression outcomes |
| **Persistence/save compatibility** | Orbital-position encoding/decoding round-trip, scan-mask get/put round-trip, legacy save load identity, no node reappearance/disappearance |
| **Generation-handler integration** | Correct per-star handler dispatch, override/fallback semantics preserved, data-provider return values consumed for node population, side-effect hooks dispatched at correct lifecycle points; behavioral compatibility required, interface identity not required |
