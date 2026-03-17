# Ships Subsystem Requirements

## Purpose

This document defines the required externally observable behavior of the ships subsystem in EARS form. The subsystem covers the ship identity/catalog model, two-tier ship loading, the per-race behavioral hook contract, the shared ship runtime pipeline, ship spawn/death/replacement lifecycle, crew writeback to persistent structures, non-melee ship runtime participation, and queue/fragment/fleet-info primitives.

## Scope boundaries

- SuperMelee setup UI, team editing, team persistence, and interactive ship-choice policy are outside this subsystem.
- The battle engine's simulation loop, frame timing, display-list ownership, and battle-mode selection policy are outside this subsystem.
- Roster preparation policy, selection-request protocols, and the decision of which ship to select next are outside this subsystem.
- Netplay transport, synchronization, and protocol behavior are outside this subsystem.
- Campaign encounter flow beyond the crew/ship-writeback boundary is outside this subsystem.
- Lower-level graphics, resource, file-I/O, input, audio, and threading mechanics are outside this subsystem, though the ships subsystem depends on them.

## Ship identity and catalog

- **Ubiquitous:** The subsystem shall assign a unique species identity to every ship in the game, spanning both melee-eligible and non-melee ships within a single identity space.
- **Ubiquitous:** The subsystem shall define a clear boundary between melee-eligible ships and non-melee ships within the species-identity space.
- **Ubiquitous:** The subsystem shall maintain a master ship catalog enumerating all melee-eligible ships, providing for each entry: species identity, cost/value, display icons, melee icons, and race name strings.
- **Ubiquitous:** The master catalog shall be sorted by race name for stable enumeration order.
- **When** the engine starts up, **the subsystem shall** load the master ship catalog using metadata-only loading (without battle assets) and make it available to external consumers before interactive setup begins.
- **When** the engine shuts down, **the subsystem shall** free all master catalog resources.
- **Ubiquitous:** The master catalog shall not include non-melee ships.
- **Ubiquitous:** The subsystem shall provide lookup accessors for catalog entries by species identity, by enumeration index, and for ship cost, display icons, and melee icons by index, without triggering battle-asset loading.

## Two-tier ship loading

- **When** a ship is loaded at the metadata-only tier, **the subsystem shall** allocate and initialize the ship descriptor, load icon assets, melee icons, and race name strings, but shall not load battle frame arrays, captain graphics, victory audio, or ship sounds.
- **When** a ship is loaded at the battle-ready tier, **the subsystem shall** perform all metadata-only loading and additionally load ship body frames, weapon frames, special-ability frames, captain background graphics, victory music, and ship sounds.
- **When** a ship descriptor is freed, **the subsystem shall** release whichever assets were loaded according to the tier at which the descriptor was loaded, and shall invoke the race-specific teardown hook if one is registered.

## Ship descriptor and runtime data model

- **Ubiquitous:** Each ship's runtime behavior shall be expressed through a ship descriptor aggregating: ship info (cost, max crew, max energy, capability flags, resource identifiers), fleet characteristics, movement/energy characteristics, battle data (loaded asset handles), AI parameters, and race-specific behavioral hooks.
- **Ubiquitous:** Each ship descriptor shall carry an opaque private-data slot for per-race mutable state.
- **Ubiquitous:** Each loaded ship shall be backed by isolated mutable runtime state so that gameplay mutations to one ship's descriptor do not affect the baseline data used to load other ships of the same race.
- **Ubiquitous:** Race-specific code may mutate the descriptor instance's characteristics, data, flags, and callbacks during the ship's combat lifetime.

## Ship capability flags

- **Ubiquitous:** The subsystem shall define capability flags characterizing externally observable ship combat properties, including but not limited to: weapon tracking/seeking, point defense, instant-hit versus projectile weapons, crew damage immunity, firing arc, and shield/defense mode presence.
- **Ubiquitous:** Capability flags shall be set per-race at descriptor initialization time and shall be readable by the shared runtime, AI, and collision handling.

## Ship-private state

- **When** a race implementation requires per-instance mutable state beyond the descriptor's standard fields, **the subsystem shall** support allocation and storage of that state through the descriptor's opaque private-data slot.
- **Ubiquitous:** The shared runtime shall not interpret or depend on the contents of ship-private state.
- **Ubiquitous:** The lifetime of ship-private state shall not exceed the lifetime of the descriptor instance that owns it.
- **When** a descriptor instance with allocated private state is freed, **the subsystem shall** invoke the race's teardown hook before releasing the descriptor, giving the race an opportunity to free its private state.

## Race-specific behavioral hooks

- **Ubiquitous:** The subsystem shall support registration of per-descriptor-instance behavioral hooks: preprocess, postprocess, weapon initialization, AI intelligence, and teardown.
- **When** a preprocess hook is registered, **the subsystem shall** invoke it once per ship per frame before the shared runtime applies movement, energy, and status logic.
- **When** a postprocess hook is registered, **the subsystem shall** invoke it once per ship per frame after the shared runtime processes weapon fire, sound, and cooldowns.
- **When** the ship fires its primary weapon and a weapon initialization hook is registered, **the subsystem shall** invoke that hook to create the weapon element(s).
- **When** AI combat decisions are needed for a computer-controlled ship and an AI intelligence hook is registered, **the subsystem shall** invoke that hook to compute control inputs.
- **When** a descriptor instance is being freed and a teardown hook is registered, **the subsystem shall** invoke the teardown hook before releasing the descriptor.
- **Ubiquitous:** A null or absent hook shall be treated as a no-op by the shared runtime.
- **Ubiquitous:** A race may change its own hooks on its descriptor instance during the ship's combat lifetime.
- **Ubiquitous:** Hook calls for a given descriptor instance shall be serialized within battle processing; no hook shall be concurrently invoked on the same descriptor instance.

## Collision behavior

- **Ubiquitous:** Ships shall exhibit correct collision behavior against other ships, projectiles, planets, and crew elements, integrating with the battle engine's collision dispatch.
- **When** a race overrides collision behavior by replacing the collision callback on its spawned element, **the subsystem shall** dispatch collisions through the race's override.
- **Ubiquitous:** Collision outcomes relevant to ship behavior shall match established combat behavior for each race. Compatibility is judged by observable per-race outcomes under the established battle-engine dispatch model.

## Shared ship runtime pipeline

- **Ubiquitous:** For each active ship element per battle frame, the shared runtime shall execute a pipeline of: input/status normalization, first-frame setup (if applicable), race preprocess dispatch, energy regeneration, turn and thrust, status coordination, weapon fire, special activation, race postprocess dispatch, and cooldown updates.
- **Ubiquitous:** The relative ordering of steps within the per-frame pipeline shall be preserved as part of the subsystem's behavioral contract.
- **Ubiquitous:** The movement model shall be inertial: thrust applies acceleration in the ship's facing direction, ships coast at current velocity when not thrusting, turn rate is governed by the ship's characteristics, and maximum speed is enforced per the ship's characteristics.
- **Ubiquitous:** The movement model shall be deterministic given the same inputs and state.
- **Ubiquitous:** Energy shall regenerate at a rate and interval defined by the ship's characteristics, weapon and special use shall deduct energy, and energy shall not exceed the ship's maximum.
- **When** the weapon input is active, energy is sufficient, and weapon cooldown has elapsed, **the subsystem shall** invoke the weapon initialization hook and deduct the weapon's energy cost.
- **When** the special input is active and special cooldown/energy constraints are met, **the subsystem shall** process special-ability activation.

## Battle initialization and teardown

- **When** battle begins, **the subsystem shall** initialize ship-runtime resources and state required for combat, including loading shared assets that the ship runtime depends on (e.g., explosion/blast effects used by the ship death sequence) and preparing internal battle-active state.
- **When** battle ends, **the subsystem shall** stop ship-related audio, free shared ship-runtime assets, enumerate remaining active ships and their crew state, free each active descriptor instance (invoking race teardown hooks), write surviving crew and loss results back to persistent fragments where applicable, clear battle-active state, and clear or release ship-owned queue and runtime state managed by the subsystem.

## Ship selection and spawn

- **When** an external system hands a chosen queue entry to the subsystem for combat, **the subsystem shall** load the descriptor at the battle-ready tier, bind the descriptor instance to the queue entry, patch the descriptor's crew level to match the queue entry's current crew, allocate a ship element in the display list, configure the element with initial position/facing/frame/state, bind the shared runtime callbacks to the element, register the race's behavioral hooks through the descriptor instance, and mark the ship as active.
- **Ubiquitous:** Spawn shall be idempotent per queue entry within a single battle — a destroyed ship's queue entry shall not be respawned.
- **Ubiquitous:** The subsystem shall expose its queue data contracts and spawn entrypoint so that battle, setup, and campaign systems can invoke spawn after making their own selection decisions.

## Ship death, transition, and replacement

- **When** a ship is destroyed during battle, **the subsystem shall** execute death-specific behavior, free the dead ship's descriptor instance (invoking the race teardown hook), record the dead ship's final crew state back to the persistent queue fragment if persistence applies, and mark the queue entry inactive.
- **When** a replacement ship is needed after a death, **the subsystem shall** follow the standard spawn sequence for the next queue entry once the external system has chosen it.
- **When** no replacement ship is available for a side, **the subsystem shall** signal the battle engine that the side has no further ships.
- **Ubiquitous:** Audio state shall be stopped or reset as appropriate during death-to-replacement transitions.

## Persistence-sensitive crew writeback

- **When** a ship is destroyed or battle ends, **the subsystem shall** record surviving crew counts from active descriptor instances back to the corresponding persistent ship fragments.
- **Ubiquitous:** The writeback mechanism shall match runtime ship state to persistent fragments by queue ordering and species identity, not by pointer identity.
- **When** crew writeback applies (campaign encounters), **the subsystem shall** write back both surviving crew for victorious ships and zero crew for destroyed ships.
- **Ubiquitous:** In SuperMelee, where battles are standalone, the subsystem shall still maintain consistent internal crew accounting even though writeback does not affect external persistent state.
- **When** floating crew elements exist at battle teardown, **the subsystem shall** account for them when computing final crew counts according to established game rules.

## Non-melee ships

- **Ubiquitous:** Non-melee ships (player flagship, final-battle opponents, autonomous probes) shall use the same descriptor structure, two-tier loading mechanism, spawn sequence, per-frame pipeline, and behavioral hook dispatch as melee ships.
- **Ubiquitous:** Non-melee ships shall not appear in the master ship catalog.
- **When** the battle mode requires a non-melee ship (e.g., hyperspace flagship, final battle), **the subsystem shall** accommodate spawn of that ship without requiring it to be enumerated in the melee catalog.
- **Ubiquitous:** The subsystem shall not assume that all ships conform to a single weapon/special pattern; non-melee ships may have unique behavioral properties expressed through the same hook contract.

## Queue and build primitives

- **Ubiquitous:** The subsystem shall define the shared combat queue data contracts and provide helper operations for: allocating a new queue entry for a given species identity, initializing the entry with catalog or campaign metadata, enqueuing the entry into a side's combat queue, and looking up queue entries by index.
- **Ubiquitous:** External systems decide which entries to create, when to enqueue them, which entry to choose next, and when to pass a chosen entry to spawn; the subsystem provides the data contracts and helpers they consume.
- **Ubiquitous:** The subsystem shall support a persistent ship-fragment model carrying species identity, current crew level, and display metadata, sufficient to reconstruct a combat queue entry for battle.
- **Ubiquitous:** The subsystem shall support cloning/copying from fleet state or catalog entries into ship fragments, preserving icon handles, name strings, and crew/energy metadata.
- **Ubiquitous:** The subsystem shall support a fleet-info model for campaign-level fleet state, carrying allied/hostile status, fleet size and growth, encounter composition, known location, sphere-of-influence tracking, and actual fleet strength.

## Error handling and failure behavior

- **When** a ship descriptor fails to load (missing resources, corrupted assets, or allocation failure), **the subsystem shall** free any resources that were successfully loaded before the failure, shall not leave a partially initialized descriptor reachable by the runtime, and shall report the failure through the established diagnostic mechanism.
- **When** a ship cannot be spawned due to load or allocation failure, **the subsystem shall** treat the side as having no available replacement for that queue entry, allowing the battle engine to proceed according to its no-replacement rules.
- **Ubiquitous:** Spawn or load failure for one ship shall not corrupt other active ship state.
- **When** a teardown hook is absent but private state appears to be present, **the subsystem shall** log a diagnostic but shall not attempt to interpret or free the private state directly.
- **Ubiquitous:** The teardown sequence shall be robust against ships that were never fully spawned, absent teardown hooks, already-freed descriptors, and queue entries with no associated descriptor.

## Roster and catalog preservation

- **Ubiquitous:** The end-state subsystem shall include every melee-eligible race and every non-melee ship that participates in the ship runtime; no ships shall be removed or added.
- **Ubiquitous:** The master ship catalog shall enumerate the same set of melee-eligible ships with the same cost/value assignments, the same sort order, and the same icon/string metadata as the established game behavior.
- **Ubiquitous:** Races that mutate their descriptor instance at runtime shall continue to produce the same externally observable effects; the subsystem shall not restrict descriptor mutability in ways that break existing race behaviors.
- **Ubiquitous:** Combat queue construction, ship-fragment cloning, and crew writeback shall preserve queue ordering, fragment identity, and crew persistence semantics so that campaign encounter flow depending on those properties is not disrupted.
- **Ubiquitous:** Non-melee ships shall participate in the ship runtime with their established behavioral properties, including the player flagship's configurable loadout, final-battle opponent behavior, and probe behavior.
