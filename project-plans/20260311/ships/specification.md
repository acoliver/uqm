# Ships Subsystem — Functional & Technical Specification

## 1. Scope

This document specifies the desired end-state behavior of the ships subsystem responsible for:

- defining and cataloging the identity, metadata, and characteristics of every ship in the game,
- loading ship data at two tiers: metadata-only for catalog/selection use and battle-ready for active combat,
- providing the per-race behavioral contract through which each ship's weapon, special, AI, and lifecycle callbacks are expressed,
- managing per-ship mutable runtime state during combat,
- driving the shared ship runtime pipeline that dispatches movement, energy, firing, and callback hooks during battle,
- defining the shared queue/fragment data contracts and helper operations that other systems consume when constructing combat rosters,
- spawning an already-chosen queue entry into an active ship runtime instance, and
- writing back persistence-sensitive results (surviving crew, ship loss) from the battle runtime to persistent ship/fleet structures.

This specification does **not** cover:

- SuperMelee setup UI, team editing, team persistence, or interactive ship-choice policy,
- the battle engine's simulation loop, frame timing, display-list ownership, or battle-mode selection policy,
- roster preparation policy, selection-request protocols, or the decision of which ship to select next,
- netplay transport, synchronization, or protocol behavior,
- campaign encounter flow beyond the crew/ship-writeback boundary,
- generic graphics, resource, file-I/O, input, audio, or threading subsystem design.

Those are integration boundaries. This subsystem depends on them but does not own their contracts.

## 2. Boundary and ownership model

### 2.1 Subsystem boundary

The ships subsystem owns the following end-state responsibilities:

1. maintaining the master ship catalog and its metadata (identity, cost, icons, name strings, fleet characteristics),
2. loading a ship's runtime descriptor at the metadata-only or battle-ready tier,
3. freeing a ship's runtime descriptor and its associated resources,
4. providing the race-specific behavioral hooks (preprocess, postprocess, weapon initialization, AI intelligence, teardown) as externally observable ship behavior,
5. spawning a ship element into the battle display list with bound common and race-specific callbacks,
6. driving the shared ship runtime pipeline (movement, energy regeneration, turn/thrust, weapon/special dispatch, status UI coordination) during each battle frame,
7. defining the shared queue/fragment data contracts and helper operations (allocate, initialize, clone, look up) that other systems use to construct combat queue entries and persistent ship fragments,
8. spawning an already-chosen queue entry into an active ship runtime instance and exposing the spawn entrypoint that battle/setup consumers invoke,
9. managing per-ship private state allocations across the ship's combat lifetime, and
10. updating persistent ship/fleet fragments with post-battle crew and loss results.

External systems decide which entries to create, when to enqueue them, which entry to choose next, and when to pass a chosen entry to spawn. Battle, setup, and campaign systems own roster preparation and selection policy. The ships subsystem provides the shared queue/fragment contracts, helper operations, spawn behavior, and writeback that those systems rely on.

### 2.2 Out-of-scope but required integration boundaries

The subsystem must integrate correctly with, but does not define, the following:

- **Battle engine boundary:** The battle engine owns the overall battle loop, frame timing, element display list management, collision dispatch, and the decision of when to call ship init/uninit. The ships subsystem provides the ship elements and callbacks that plug into that loop. The battle engine decides when to request ship selection and owns the selection-request protocol and transition policy; the ships subsystem exposes the spawn entrypoint and queue data model that the battle engine invokes once a selection decision has been made.
- **SuperMelee/setup boundary:** Setup consumers read the master ship catalog and construct combat queue entries using the ships subsystem's helper operations. The ships subsystem provides the catalog, the queue/fragment data contracts, and the helper operations; it does not own the setup UI, team-editing flow, interactive ship-choice policy, or roster preparation decisions.
- **Campaign/encounter boundary:** Campaign encounter logic decides when battles happen and what fleets participate. The ships subsystem owns crew-writeback at its boundary but does not own encounter flow, fleet composition decisions, or save/load of campaign state.
- **Resource/file-I/O boundary:** Ship loading depends on the resource subsystem for lifecycle management and asset loading. The ships subsystem defines what resources a ship needs; the resource subsystem provides the load/free mechanics.
- **Netplay boundary:** Netplay may constrain ship selection timing or require synchronization of selection outcomes. The ships subsystem exposes selection results; it does not own the synchronization protocol.

## 3. Ship identity and catalog model

### 3.1 Ship identity

Every ship in the game shall have a unique species identity. The identity space shall include:

- **melee-eligible ships:** the full roster of ships available for SuperMelee fleet construction, and
- **non-melee ships:** ships that participate in the broader ship runtime but are not part of the melee catalog (e.g., the player flagship, final-battle opponents, and autonomous probes).

The subsystem shall define a clear boundary between the melee-eligible subset and the full identity space. Non-melee ships share the same runtime contracts (descriptor, callbacks, spawn) but are excluded from the master catalog enumeration.

### 3.2 Master ship catalog

The subsystem shall maintain a master ship catalog that:

- enumerates all melee-eligible ships,
- provides metadata for each entry: ship identity, cost/value, display icons, melee icons, and race name strings,
- loads that metadata at the metadata-only tier without loading battle assets,
- is sorted by race name for stable enumeration order,
- is available to external consumers (setup UI, status display, fleet-value computation) through lookup accessors, and
- is loaded once during engine startup and freed during engine shutdown.

The catalog shall not include non-melee ships. Consumers requiring non-melee ship metadata shall use the ship-loading contract directly.

### 3.3 Catalog lookup contract

The subsystem shall provide lookup accessors for:

- finding a catalog entry by species identity,
- finding a catalog entry by enumeration index,
- retrieving ship cost/value by index,
- retrieving ship display icons by index, and
- retrieving ship melee icons by index.

These accessors shall operate on the loaded master catalog and shall not trigger battle-asset loading.

## 4. Ship descriptor and runtime data model

### 4.1 Ship descriptor

Each ship's runtime behavior shall be expressed through a **ship descriptor** — a composite structure that aggregates:

- **ship info:** cost, maximum crew, maximum energy, ship capability flags, and resource identifiers for the ship's loadable assets,
- **fleet characteristics:** campaign-relevant fleet data (growth, max fleet size, initial encounter composition),
- **movement/energy characteristics:** turn rate, thrust/acceleration parameters, energy regeneration timing, weapon/special energy costs and cooldowns, mass,
- **battle data:** loaded frame arrays for ship body, weapon projectiles, and special-ability visuals, plus captain graphics, victory audio, and ship sounds,
- **AI parameters:** maneuverability class, weapon range, and an AI intelligence callback, and
- **race-specific behavioral hooks:** callbacks for teardown, per-frame preprocess, per-frame postprocess, and weapon initialization.

The descriptor also carries:

- an opaque private-data slot for per-race mutable state and
- any per-race overrides to collision behavior.

### 4.2 Descriptor template and instance semantics

Each race implementation shall define a static descriptor template containing the race's baseline characteristics, resource identifiers, and behavioral parameters.

When a ship is loaded for battle, the subsystem shall produce a **descriptor instance** by copying the template into separately allocated storage. The instance is the live mutable runtime object; the template is never directly mutated during gameplay.

Race-specific code may freely mutate the descriptor instance's characteristics, data, flags, and callbacks during the ship's combat lifetime. This is an expected and supported usage pattern, not a deficiency.

### 4.3 Two-tier loading

The subsystem shall support two loading tiers for ship descriptors:

- **Metadata-only load:** Allocates and initializes the descriptor, loads icon assets, melee icons, and race name strings, but does **not** load battle frame arrays, captain graphics, victory audio, or ship sounds. Suitable for catalog, selection, and status display use.
- **Battle-ready load:** Performs everything in the metadata-only load, plus loads all battle assets: ship body frames, weapon frames, special-ability frames, captain background graphics, victory music, and ship sounds. Required before a ship can be spawned into combat.

The loading tier is selected at load time and determines which assets are present in the resulting descriptor. Freeing a descriptor shall release whichever assets were loaded, and shall invoke the race-specific teardown hook if one is registered.

### 4.4 Ship capability flags

The subsystem shall define capability flags that characterize a ship's combat properties at the descriptor level. These flags express externally observable ship capabilities such as:

- weapon tracking/seeking behavior,
- point-defense capability,
- instant-hit versus projectile weapons,
- crew damage immunity,
- firing arc (fore/aft),
- shield/defense mode presence, and
- similar combat-property categories.

These flags are set per-race at initialization time and may be read by the shared runtime and by other subsystems (e.g., AI, collision handling) to determine ship behavior.

### 4.5 Ship-private state

Races that require per-instance mutable state beyond the descriptor's standard fields shall use the descriptor's opaque private-data slot.

- Private state shall be allocated by the race's initialization or preprocess logic.
- Private state shall be freed by the race's teardown callback before the descriptor is released.
- The shared runtime shall not interpret or depend on the contents of private state.
- The lifetime of private state shall not exceed the lifetime of the descriptor instance that owns it.

## 5. Race-specific behavioral hooks

### 5.1 Hook contract

Each race implementation may register behavioral hooks on its descriptor. The hooks are:

- **Preprocess hook:** Called once per ship per frame before the shared runtime applies movement, energy, and status logic. May mutate the descriptor instance, allocate or modify projectile/effect elements, and alter ship state.
- **Postprocess hook:** Called once per ship per frame after shared runtime has processed weapon fire, sound, and cooldowns. May apply special-ability effects, modify descriptor characteristics, or create secondary elements.
- **Weapon initialization hook:** Called when the ship fires its primary weapon. Responsible for creating the weapon element(s) with appropriate behavior, visuals, and collision properties.
- **AI intelligence hook:** Called to compute AI combat decisions for computer-controlled ships. Receives the ship's state and returns control inputs.
- **Teardown hook:** Called when the descriptor instance is being freed. Responsible for releasing race-specific private state and any race-allocated resources.

### 5.2 Hook dispatch model

The shared ship runtime shall invoke hooks through the descriptor instance's registered function references. The exact mechanism (function pointers, trait objects, vtable, or similar) is not prescribed, but the following properties shall hold:

- hooks are per-descriptor-instance, not global per-race,
- a race may change its own hooks during the ship's lifetime (e.g., switching between combat modes),
- a null/absent hook shall be treated as a no-op by the shared runtime,
- hook calls are serialized within battle processing — no concurrent invocation of hooks for the same descriptor instance, and
- hooks may mutate the descriptor instance and may interact with the element/display-list system through the established battle-engine interface.

### 5.3 Collision behavior

Ships shall exhibit correct collision behavior against other ships, projectiles, planets, and crew elements. Races may override collision behavior by replacing the collision callback on their spawned element, and the subsystem shall dispatch through the race's override when one is present.

The subsystem shall integrate with the battle engine's collision dispatch so that ship-specific collision behavior is applied where the ship's combat rules require it. Race overrides shall be honored when present, and collision outcomes relevant to ship behavior shall match established combat behavior for each race.

## 6. Shared ship runtime pipeline

### 6.1 Per-frame ship processing

For each active ship element in battle, the shared runtime shall execute a per-frame pipeline consisting of:

1. **Input/status normalization:** Read the ship's control inputs and status flags.
2. **First-frame setup:** On the ship's first active frame, perform any one-time initialization (e.g., initial facing, status bar setup).
3. **Race preprocess dispatch:** Invoke the race's preprocess hook, if registered.
4. **Energy regeneration:** Apply energy regeneration according to the ship's timing characteristics.
5. **Turn and thrust:** Apply turn rate and thrust/acceleration according to control inputs and movement characteristics.
6. **Status coordination:** Update status-bar or status-display state as needed.
7. **Weapon fire:** If the weapon input is active, energy is sufficient, and cooldown has elapsed, invoke the weapon initialization hook and deduct energy.
8. **Special activation:** If the special input is active and cooldown/energy constraints are met, process special-ability activation.
9. **Race postprocess dispatch:** Invoke the race's postprocess hook, if registered.
10. **Cooldown updates:** Advance weapon and special cooldown timers.

The exact ordering of steps within this pipeline is part of the subsystem's behavioral contract. Races depend on the relative ordering of preprocess, shared logic, and postprocess to implement correct behavior.

### 6.2 Movement model

The shared runtime shall implement inertial movement with the following properties:

- Ships have a maximum speed determined by their characteristics.
- Thrust applies acceleration in the ship's current facing direction.
- Ships coast at their current velocity when not thrusting.
- Turn rate is governed by the ship's turn-wait characteristic.
- Gravity wells, when present, apply additional velocity influence.

The movement model shall be deterministic given the same inputs and state.

### 6.3 Energy model

- Ships regenerate energy at a rate and interval defined by their characteristics.
- Weapon and special use deduct energy from the ship's current energy level.
- Energy shall not exceed the ship's maximum energy.
- Energy state is per-descriptor-instance and may be mutated by race-specific hooks.

## 7. Battle lifecycle

### 7.1 Battle initialization

When battle begins, the subsystem shall initialize ship-runtime resources and state required for combat. This includes loading any shared assets that the ship runtime depends on (e.g., explosion/blast effects used by the ship death sequence) and preparing the subsystem's internal battle-active state.

The battle engine owns overall battle-environment orchestration (display list setup, space/background rendering, planet/asteroid placement). The ships subsystem integrates with that orchestration by providing ship elements and callbacks and by participating in environment-specific setup where ship-runtime behavior requires it (e.g., hyperspace navigation mode affecting ship spawn).

### 7.2 Ship selection

Battle, setup, and campaign systems own selection policy — they decide which queue entry is chosen next. For SuperMelee, `supermelee/specification.md` §2.1 is the authoritative owner of next-combatant selection policy. The ships subsystem owns the queue-entry data model, battle-ready descriptor loading, and spawn of the already-selected entry. Ships does not own or constrain which entry is selected; it provides the primitives that selection-policy owners invoke once a decision has been made.

The selection result is a queue entry identifying the ship to spawn. Selection does not load battle assets; that occurs during spawn.

### 7.3 Ship spawn

When a chosen queue entry is handed to the subsystem for combat, the subsystem shall:

1. load the ship's descriptor at the battle-ready tier,
2. bind the descriptor instance to the queue entry,
3. patch the descriptor's crew level to match the queue entry's current crew (which may differ from the template maximum due to prior damage or campaign state),
4. allocate a ship element in the battle display list,
5. configure the element with initial position, facing, frame, and state,
6. bind the shared runtime callbacks (preprocess, postprocess, death, collision) to the element,
7. register the race's specific behavioral hooks through the descriptor instance, and
8. mark the ship as active.

Spawn shall be idempotent per queue entry within a single battle — a ship that has been spawned and subsequently destroyed shall not be respawned unless it is a fresh queue entry.

### 7.4 Ship death and transition

When a ship is destroyed during battle:

1. the death callback shall execute any death-specific behavior (explosion, crew scatter),
2. after the death sequence completes, the subsystem shall free the dead ship's descriptor instance (invoking the race teardown hook),
3. the subsystem shall record the dead ship's final crew state (zero for a destroyed ship, or surviving crew for a ship that won) back to the persistent queue fragment if persistence applies,
4. the dead ship's queue entry shall be marked inactive, and
5. the battle engine shall request the next ship via its selection policy.

If no replacement ship is available for a side, the battle ends for that side.

### 7.5 Ship replacement

Ship replacement follows the same spawn sequence described in §7.3. The replacement ship is a new queue entry; the dead ship's descriptor instance has already been freed.

Audio state (music, sound effects) shall be stopped or reset as appropriate during the death-to-replacement transition.

### 7.6 Battle teardown

When battle ends, the subsystem shall:

1. stop ship-related audio,
2. free shared ship-runtime assets loaded during battle initialization,
3. enumerate any remaining active ship elements and their associated crew state,
4. free each remaining active descriptor instance (invoking race teardown hooks),
5. write surviving crew and loss results back to persistent ship/fleet fragments where applicable,
6. clear battle-active state, and
7. clear or release ship-owned queue and runtime state managed by the subsystem.

The battle engine owns the broader teardown orchestration (display list cleanup, environment state, post-battle flow decisions). The ships subsystem is responsible for cleaning up ship-runtime resources and state at its boundary.

## 8. Persistence-sensitive crew and writeback behavior

### 8.1 Crew writeback contract

At the ships-subsystem boundary, post-battle results must flow back to persistent ship structures. The subsystem shall:

- record surviving crew counts from active ship descriptor instances back to the corresponding persistent ship fragment, and
- record crew counts for ships that were destroyed (zero crew) back to their fragments.

This writeback shall occur during ship-death transitions (§7.4) and during battle teardown (§7.6).

### 8.2 Fragment identity matching

The writeback mechanism shall match runtime ship state to persistent fragments by queue ordering and species identity. The subsystem shall not rely on pointer identity between the runtime descriptor and the persistent fragment; the match is by logical identity within the combat queue.

### 8.3 Writeback scope

Crew writeback applies in contexts where battle results are persistent — primarily campaign encounters. In SuperMelee, where battles are standalone, crew writeback does not affect external persistent state, but the subsystem shall still maintain consistent internal crew accounting during the match.

### 8.4 Floating crew elements

During battle, crew members ejected from a destroyed ship may exist as independent display-list elements. The subsystem shall account for these floating crew elements when computing final crew counts during battle teardown. Crew that has not been collected by a surviving ship at teardown time shall be counted according to the established game rules.

## 9. Non-melee ships

### 9.1 Shared runtime participation

Non-melee ships (player flagship, final-battle opponents, autonomous probes) shall participate in the same ship runtime contracts as melee ships:

- they use the same descriptor structure,
- they are loaded through the same two-tier loading mechanism,
- they are spawned through the same spawn sequence,
- they use the same per-frame pipeline, and
- their behavioral hooks follow the same dispatch contract.

### 9.2 Catalog exclusion

Non-melee ships shall not appear in the master ship catalog. Their identity codes shall exist in the full species-identity space but shall be outside the melee-eligible range.

### 9.3 Special selection paths

Non-melee ships may be selected through paths that bypass the melee catalog (e.g., the player flagship is always selected in hyperspace; final-battle opponents may bypass normal selection UI). The queue/spawn interface (§7.2–7.3) shall accommodate these entries without requiring non-melee ships to be enumerated in the melee catalog.

### 9.4 Non-melee ship behavioral scope

Non-melee ships may have unique behavioral properties not shared by melee ships (e.g., the player flagship has a configurable weapon/special loadout; probes may have limited combat behavior). These behaviors are expressed through the same hook contract and descriptor structure. The subsystem shall not assume that all ships conform to a single weapon/special pattern.

## 10. Queue and build primitives

### 10.1 Combat queue model

The subsystem shall define the shared combat queue data contracts and provide helper operations for working with queue entries. These helpers shall support:

- allocating a new queue entry for a given species identity,
- initializing the entry with metadata from the master catalog or from campaign fleet state,
- enqueuing the entry into a side's combat queue, and
- looking up queue entries by index.

External systems decide which entries to create, when to enqueue them, which entry to choose next, and when to pass a chosen entry to spawn. Battle, setup, and campaign systems use these helpers to construct per-side combat rosters according to their own preparation and selection policies.

### 10.2 Ship fragment model

For campaign contexts, the subsystem shall support a persistent ship-fragment model that carries:

- species identity,
- current crew level,
- display metadata (icons, name strings), and
- enough state to reconstruct a combat queue entry for battle.

Fragments are the persistence-side representation; combat queue entries are the battle-side representation. The subsystem owns the cloning/copying contract between fragments and combat entries.

### 10.3 Fleet-info model

The subsystem shall support a fleet-info model for campaign-level fleet state, carrying:

- allied/hostile status and diplomatic state,
- fleet size, growth, and encounter composition,
- known location and sphere-of-influence tracking, and
- actual fleet strength for encounter generation.

This model is consumed by campaign encounter logic and is part of the ships subsystem's shared contract surface, even though campaign encounter flow itself is out of scope.

## 11. Race-implementation dispatch

### 11.1 Dispatch contract

The subsystem shall provide a dispatch mechanism that maps a ship's species identity to the race implementation that produces its descriptor template.

The dispatch shall:

- accept a species identity (or equivalent key),
- invoke the corresponding race initializer, and
- produce a descriptor instance from the returned template.

The mechanism by which the descriptor instance is managed for lifecycle purposes (resource-system registration, reference counting, or equivalent) is an implementation concern; the contract requires that instances are properly tracked and freed.

### 11.2 Implementation registration

All race implementations shall be reachable through the dispatch mechanism. The exact registration strategy (compile-time table, dynamic registration, or similar) is not prescribed, but the dispatch shall cover the full species-identity space including non-melee ships.

### 11.3 No dynamic module loading requirement

The dispatch mechanism is not required to support runtime discovery or hot-loading of race implementations. All ships are known at build time.

## 12. Error handling and failure behavior

### 12.1 Load failure

If a ship descriptor fails to load (missing resources, corrupted assets, or allocation failure):

- the subsystem shall not leave a partially initialized descriptor reachable by the runtime,
- any resources that were successfully loaded before the failure shall be freed,
- the spawn sequence shall not proceed with a partially loaded descriptor, and
- the failure shall be reported through the established diagnostic/logging mechanism.

### 12.2 Spawn failure

If a ship cannot be spawned (load failure, element allocation failure):

- the side shall be treated as having no available replacement for that queue entry,
- the battle engine shall proceed according to its no-replacement rules (which may end the battle for that side), and
- the failure shall not corrupt other active ship state.

### 12.3 Teardown robustness

The teardown sequence (§7.6) shall be robust against:

- ships that were never fully spawned,
- ships whose race teardown hook is absent,
- descriptor instances that have already been freed during mid-battle transitions, and
- queue entries with no associated descriptor.

### 12.4 Private-state leak prevention

If a race implementation allocates private state, the subsystem's teardown path shall ensure the race's teardown hook is invoked before the descriptor is freed. If the teardown hook is absent but private state is present, the subsystem shall log a diagnostic but shall not attempt to interpret or free the private state directly.

## 13. Compatibility expectations

### 13.1 Ship roster preservation

The end-state subsystem shall preserve the complete ship roster: every melee-eligible race and every non-melee ship that participates in the ship runtime. No ships shall be removed or added.

### 13.2 Catalog preservation

The master ship catalog shall enumerate the same set of melee-eligible ships, with the same cost/value assignments, the same sort order (by race name), and the same icon/string metadata as the established game behavior.

### 13.3 Descriptor-field mutation support

Races that mutate their descriptor instance at runtime (changing characteristics, swapping callbacks, switching frame sets, altering collision behavior) shall continue to produce the same externally observable effects. The subsystem shall not restrict descriptor mutability in ways that break existing race behaviors.

### 13.4 Queue and fragment semantics

Combat queue construction, ship-fragment cloning, and crew-writeback shall preserve queue ordering, fragment identity, and crew persistence semantics so that campaign encounter flow depending on those properties is not disrupted.

### 13.5 Non-melee ship preservation

Non-melee ships shall continue to participate in the ship runtime with their established behavioral properties, including the player flagship's configurable loadout, final-battle opponent behavior, and probe behavior.

## 14. Integration points

### 14.1 Resource subsystem

The ships subsystem depends on the resource subsystem for:

- asset loading (frames, sounds, strings, icons) and
- resource lifecycle management (allocation, reference counting, destruction).

The resource-type registration for ships shall remain compatible with the resource subsystem's type-registration contract.

### 14.2 Battle engine

The ships subsystem integrates with the battle engine through:

- `InitShips` / `UninitShips` (or equivalent) lifecycle calls,
- element creation and callback binding on the display list,
- collision dispatch, and
- frame-by-frame preprocess/postprocess execution within the battle loop.

The subsystem provides the ship elements and callbacks; the battle engine owns the loop, display-list management, and battle-environment orchestration.

### 14.3 SuperMelee and setup consumers

Setup consumers integrate with the ships subsystem through:

- master catalog lookup accessors,
- `Build` (or equivalent) queue-construction helpers, and
- combat-queue metadata (ship info, icons, cost) on constructed queue entries.

Setup consumers own roster preparation and selection policy; the ships subsystem provides the data model and helper operations they consume.

### 14.4 Campaign and encounter systems

Campaign systems integrate through:

- fleet-info queries and updates,
- ship-fragment construction and cloning from fleet state,
- crew-writeback from battle results, and
- escort/allied-ship management primitives.

Campaign systems own encounter flow, fleet composition, and roster preparation; the ships subsystem provides the fragment/queue data contracts and writeback behavior.

### 14.5 Input and AI systems

Ship control inputs flow from the input/AI systems to the ship runtime through per-frame control flags on the ship's queue entry. The ships subsystem reads these flags during its per-frame pipeline; it does not own input device handling or AI strategy beyond the per-race AI hook.

## 15. Compatibility and non-goals

### 15.1 Compatibility targets

The end-state subsystem shall preserve the observable compatibility surface of the ships subsystem with respect to:

- complete ship roster (melee and non-melee),
- per-race combat behavior and characteristics,
- master catalog content and lookup semantics,
- two-tier loading behavior (metadata-only vs. battle-ready),
- ship spawn/death/replacement lifecycle,
- crew writeback to persistent fragments,
- queue and fragment construction/cloning semantics, and
- fleet-info model for campaign integration.

### 15.2 Non-goals

This specification does not require:

- preserving current C struct layouts or field ordering,
- preserving current function-pointer-based dispatch as the implementation mechanism for behavioral hooks,
- preserving the current source-file decomposition (one directory per race, specific filenames),
- preserving the `CODERES_STRUCT` allocation and copy-by-value dispatch mechanism,
- preserving the current `ELEMENT` callback signature layout,
- absorbing the battle engine, SuperMelee setup, or netplay subsystem into the same unit, or
- treating the inactive Rust `game_init` FFI wrappers as end-state contract surface.

## 16. Open audit-sensitive areas

The following areas should be treated as compatibility-sensitive verification questions, not implementation-plan steps:

- whether exact frame-by-frame behavioral fidelity for each race ship is required (e.g., identical RNG sequences, identical collision-frame timing) or whether perceptual equivalence is sufficient,
- whether race-specific private-data layouts need to survive save/load cycles or are purely transient combat state,
- whether the campaign fleet-info tables (speed, encounter makeup, cost, communication mapping) belong to the ships subsystem contract or to a separate campaign-data contract,
- whether any race's runtime descriptor mutation pattern depends on specific memory-layout properties (e.g., field adjacency, union aliasing) that would require explicit preservation,
- which specific collision behaviors between specific race pairs are compatibility-significant beyond "damage is applied correctly," and
- whether the current load-failure behavior (incomplete cleanup with a known TODO) should be treated as the compatibility baseline or should be corrected to clean failure.
