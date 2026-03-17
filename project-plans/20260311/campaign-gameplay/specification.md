# Campaign Gameplay Subsystem — Functional & Technical Specification

## 1. Scope

This document specifies the desired end-state behavior of the campaign-gameplay subsystem responsible for:

- new-game and load-game entry branching,
- top-level campaign loop and activity dispatch,
- transitions among hyperspace, interplanetary, encounter, and starbase activities,
- encounter and starbase handoff at the campaign boundary,
- campaign event policy and progression tied to the game clock, and
- campaign save/load semantics.

**Document layering:** This specification and its companion `requirements.md` are the normative target contract. The companion `initialstate.md` is a descriptive analysis of the current codebase state, not a normative document. Where the two diverge, this specification is authoritative for required behavior. The specification defines the behavioral contract and architecture-level constraints; `requirements.md` defines externally testable requirements only.

**Compatibility contract scope:** The compatibility contract defined by this document is limited to the observable behaviors and acceptance criteria explicitly stated in the normative sections of this specification and `requirements.md`. Semantically equivalent behavior satisfies the contract unless a section explicitly requires exact parity. Areas not explicitly covered by normative statements — including open questions listed in §11 — do not silently expand the acceptance surface.


**Conformance surface model:** This specification uses three distinct layers for save/load compatibility and verification:

1. **Raw persisted representation** — the actual save artifact bytes/structure written by an implementation. Except where this document explicitly requires a normalized identifier or representation at the persistence boundary, the raw representation is implementation-defined.
2. **Canonical persistence-boundary meaning** — the normalized campaign-boundary facts that are in contract for covered save/load contexts: recoverable campaign mode, save-summary meaning, scheduled-event meaning, encounter-handoff meaning, faction/campaign-flag meaning, and other explicitly covered persistence-boundary facts. Gameplay/save-load compatibility attaches to this layer together with the player-visible behavior defined elsewhere in this document.
3. **Inspection representation** — a verifier-facing representation of that canonical persistence-boundary meaning. This may be the raw save artifact itself if it already exposes the needed canonical facts in a documented, machine-readable, stable form, or it may be the Campaign Canonical Export Document defined in §10.1.

**Top-level verification-model exception:** The general persistence-boundary inspection model is raw-save inspection versus Campaign Canonical Export Document inspection. The only sanctioned exception is for covered valid legacy starbase save/load claims when the legacy raw save does not directly expose the full closed starbase comparison object: in that narrow case, specification §10.1 permits and may require observation-based controlled comparison at the starbase post-load conformance observation point. This exception is claim-family-specific and does not create a third general inspection surface for other covered claims.


**Exactness vs equivalence principle:** Exact normalized identifiers, field names, and section schemas are required only where this specification explicitly defines them as canonical comparison tokens on a persistence-boundary or inspection surface. Otherwise, semantic or observable equivalence is sufficient unless a section explicitly requires exact parity.

**Inspection-interface scope note:** Some obligations in this document are defined on persistence-boundary facts that are not purely player-visible. For that reason, the subsystem contract includes a verifier-facing persistence inspection interface: either the raw save artifact itself when it already exposes the needed canonical boundary facts, or an export-based inspection representation as defined in §10.1. This interface is in scope only as a conformance surface for verifying canonical persistence-boundary meaning; it is not a separate user-facing gameplay feature requirement. It is conditionally mandatory, not universally mandatory: if the raw save artifact already exposes all canonical boundary facts required for all covered verification claims and covered contexts in a documented, machine-readable, verifier-stable form, no export interface is required for conformance. Exact JSON field names, section names, and schema-shape rules in §10.1 belong to that inspection-interface contract only. They do not, by themselves, impose any requirement that the persisted save format or runtime subsystem model use the same structure, naming, or decomposition, except where this specification explicitly defines a normalized persistence-boundary token or meaning that implementations must preserve.
**Normative-document precedence rule:** `requirements.md` governs verifier-facing pass/fail obligations and acceptable external evidence. This specification governs behavioral meaning, subsystem boundaries, normalized persistence-boundary semantics, and architecture-level constraints. The union does not silently expand the contract: any apparent conflict requires explicit amendment rather than interpolation. For operational interpretation: when a verifier-facing requirement in `requirements.md` uses a term, comparison surface, or equivalence scope defined more precisely in this specification, the specification's definition limits and interprets that requirement. When `requirements.md` states a verifier-facing obligation not defined more precisely in this specification, the plain requirement text controls. Narrower wording by itself is not a conflict-resolution rule and does not silently narrow another normative clause; only explicit scoping language or explicit amendment may do so.




**Persistence-boundary exactness exception:** The scheduled-event vocabulary, save-summary normalization tokens, and other explicitly normalized identifiers defined in §§8.6 and 10.1 are intentional exactness exceptions to the general semantic-equivalence default. They are fixed because legacy-save interoperability and reproducible external conformance verification require stable comparison tokens at the persistence boundary. Outside those explicitly normalized surfaces, semantic or observable equivalence remains sufficient unless a section explicitly requires exact parity.

**Exactness matrix for this specification:**

**Conformance application note:** The controlling verifier-facing application flow is defined in `requirements.md` under "How to classify and verify a claim" together with the controlling inspection-surface and comparison-object rules in §10.1 below. The bullets that follow define the contract categories and surfaces used by that flow; they are not a second ordered verifier procedure.

- **Exact-token / exact-schema surfaces:** scheduled-event selector vocabulary and required comparable scheduling metadata at the canonical persistence boundary; save-summary normalization tokens; explicit canonical export field names, section names, and schema rules in §10.1 when the Campaign Canonical Export Document is the chosen inspection surface; any other identifier or token this specification explicitly labels as exact.
- **Semantic/observable-equivalence surfaces:** player-visible campaign flow; non-starbase save/load equivalence within the closed non-starbase observable scope; starbase save/load equivalence within the closed progression-point contract and mandatory-next-action outcome; encounter-adjacent campaign-owned handoff meaning; downstream event outcomes except where an exact persistence-boundary token is explicitly required.
- **Failure interpretation rule:** failure of an exact-token or exact-schema obligation on a mandatory inspection surface is an inspection-surface conformance failure and, where that inspection surface is mandatory for the covered claim/context, is sufficient to fail overall conformance for that claim/context. Outside such mandatory inspection-surface cases, semantic/observable conformance is judged by the applicable equivalence scope rather than by export-schema exactness alone.
- **Surface-independent canonical-token rule:** Whenever this specification defines exact normalized comparison tokens (for example save-summary tokens, resume-context tokens, transition-marker tokens, and exact baseline identifier vocabularies), those tokens are semantic comparison objects independent of the chosen inspection surface. A raw-save inspection path is conforming if and only if the raw persisted artifact plus its authoritative verifier-facing field map deterministically yields those same canonical tokens without source-internal inference, runtime-only state, or implementation-extrinsic guesswork; the raw artifact is not required to literally store the same strings used by the Campaign Canonical Export Document.



**Legacy baseline reference:** Throughout this document, "current baseline implementation" and references to "legacy" behavior refer to the legacy C-owned codebase documented by the `/project-plans/20260311` current-state documentation set (including `initialstate.md`) from which the compatibility obligations in this specification and `requirements.md` were derived. This is a fixed historical reference, not a pointer to whatever the codebase looks like at any future date.
**Normative provenance labels used in this specification:**
- **[Legacy-derived]** — a rule grounded directly in the legacy baseline seam summarized by `initialstate.md`.
- **[End-state policy]** — an intentional narrowing or normalization chosen for the target contract where the legacy seam exists but the end-state contract is being closed more sharply.
- **[Verifier-surface normalization]** — a rule chosen to stabilize external comparison/inspection without implying identical internal implementation structure.

These labels are used selectively in the most compatibility-sensitive clauses below; unlabeled clauses should be read according to the general layering and precedence rules above.

**Intentional target normalizations beyond direct current-state evidence:** The following are deliberate target-contract closures, not claims that the legacy current state already exposes them as first-class persistence tokens or verifier surfaces:
- the closed starbase progression-point comparison matrix and verifier-facing settlement procedure,
- the exact Campaign Canonical Export Document section/field vocabulary and JSON schema shape in §10.1,
- the project-defined canonical comparison tokens used for save-summary normalization and export-facing identifier vocabularies,
- the verifier-surface normalization rules that stabilize comparison objects for ambiguous or implementation-dependent internal structures.

**Normative class and ownership map:**

| Normative class | Primary owner | Purpose | Failure classification |
|---|---|---|---|
| Legacy-derived compatibility obligations | `specification.md` behavioral and persistence-boundary sections | Preserve the campaign behavior and persistence-boundary meanings required by the compatibility contract | Gameplay/save-load compatibility failure |
| End-state policy normalizations | `specification.md` clauses explicitly labeled `[End-state policy]` | Close ambiguous compatibility-sensitive seams where the target contract intentionally chooses a sharper normalization | Conformance failure against the chosen target contract, not a claim that legacy raw tokens were first-class |
| Verifier-surface normalizations | `specification.md` §10.1 and `requirements.md` Verification surface | Provide stable external comparison surfaces and evidence modes when direct behavior alone is insufficient | Inspection-surface conformance failure when that surface is chosen or required |
| Verifier procedures and evidence rules | `requirements.md` | Define how a verifier classifies claims, selects evidence, and applies pass/fail rules without source-internal inspection | Verifier-facing conformance failure under the rules in `requirements.md` |

These are authorized end-state policy and verifier-surface normalization choices within the subsystem boundary; they should not be read back into `initialstate.md` as direct evidence claims about the legacy implementation.



This specification does **not** cover:

- solar-system exploration internals (orbit, scan, surface, lander),
- dialogue/comm subsystem internals,
- per-ship combat mechanics or battle simulation internals,
- SuperMelee setup, menuing, or fleet editing,
- netplay transport or protocol behavior, or
- lower-level graphics, input, audio, resource, file-I/O, or threading subsystem design.

Those are integration boundaries. This subsystem may depend on them, but it does not own their contracts.

## 2. Boundary and ownership model

### 2.1 Subsystem boundary

The campaign-gameplay subsystem owns the following end-state responsibilities:

1. presenting the new-game / load-game decision and dispatching accordingly,
2. running the top-level campaign loop that selects the current campaign activity,
3. advancing the campaign through activity transitions (hyperspace ↔ interplanetary, hyperspace → encounter, campaign → starbase, etc.),
4. owning the semantic definition and dispatch meaning of the campaign activity vocabulary — the set of observable campaign modes and transition flags that determine which major campaign mode is active — without implying that the underlying representation must reside in a campaign-local module or data structure,
5. handing control to encounter/communication, starbase, interplanetary, and hyperspace entry points and reclaiming control when those activities complete or yield,
6. defining campaign-specific event content: registering campaign event instances, implementing campaign event handlers, and determining the campaign effects when events fire,
7. owning campaign clock-rate policy — deciding when to initialize and shut down campaign clock usage, selecting clock rates by activity mode, and requesting day advancement, and
8. serializing and deserializing campaign runtime state for save/load.

### 2.2 Out-of-scope but required integration boundaries

The subsystem must integrate correctly with, but does not define, the following:

- **Solar-system exploration boundary:** Campaign gameplay routes to interplanetary/solar-system exploration when the appropriate activity flag is set, but it does not define orbit mechanics, scanning, surface exploration, or lander behavior. Campaign gameplay is the hosting layer responsible for ensuring that planet-info persistence infrastructure is initialized before the solar-system subsystem's first get/put call and torn down after the last (see `planet-solarsys/specification.md` §10 persistence contract). The semantic content of planet-info persistence is owned by the solar-system subsystem; campaign owns the init/uninit lifecycle timing. **Lifecycle boundary events:** The persistence infrastructure shall be initialized before any load path that can resume into interplanetary or orbit state. It shall remain live for the entire solar-system session, including orbit-entry get operations and save-location encoding put operations. It shall be torn down only after solar-system uninit completes and after any pending put triggered by session exit or save has completed. Campaign transitions that tear down solar-system state (e.g., starbase entry, encounter exit, load into non-solar-system state) shall ensure all pending persistence writes are flushed before teardown.
- **Dialogue/comm boundary:** Campaign gameplay invokes communication/dialogue sessions as part of encounter or starbase flow, but it does not define dialogue trees, speech timing, response selection, or track-player semantics.
- **Battle subsystem boundary:** Campaign gameplay prepares encounter handoff inputs and invokes battle, but it does not define combat simulation, ship weapons, collision, or per-ship AI.
- **Ship subsystem boundary:** Campaign gameplay references ship identities in escort, NPC, and encounter queues. It does not define ship descriptors, combat hooks, or ship stats.
- **SuperMelee boundary:** SuperMelee is a separate branch out of the restart/start flow. Campaign gameplay does not own SuperMelee setup, menuing, or fleet editing.
- **Game-state persistence boundary:** Campaign gameplay reads and writes game-state bits and state files. The backing implementation of game-state bit access primitives and state-file I/O is not owned by this subsystem. File/stream mechanics are owned by the file-I/O subsystem (`file-io/specification.md`). Per-planet scan-state persistence semantics are owned by the planet-solarsys subsystem (`planet-solarsys/specification.md`); campaign owns the init/uninit lifecycle timing for that persistence infrastructure (see Solar-system exploration boundary above). Battle-group state-file and per-system state-file access mechanics are integration dependencies whose backing implementation is outside this subsystem's boundary.
- **Clock subsystem boundary:** The clock subsystem owns generic scheduling mechanics, event-queue persistence and restoration, due-event dispatch machinery, and internal timing mechanics. Campaign gameplay owns the semantic definition of campaign event types, their registration policy, and the effects of their handlers. Campaign gameplay requests clock initialization, shutdown, rate changes, and day advancement. Campaign gameplay registers event instances and implements event handlers through the clock subsystem's interface. When the clock subsystem persists or restores scheduling state (e.g., during save/load), the meaning and behavioral contract of those scheduled events remains campaign-owned; persisted scheduling state does not transfer semantic ownership of those events to the clock subsystem.
- **Persistence/file-I/O boundary:** Campaign gameplay defines the semantic content of save files. Lower layers provide file handles, stream semantics, and path resolution.

**Semantic validation at the persistence seam:** The clock subsystem restores scheduling data from persisted state, but campaign gameplay is responsible for determining whether the restored schedule is semantically valid for the loaded campaign state. The concrete required rejection cases are defined in §9.4.1.

## 3. End-state subsystem model

### 3.1 Campaign activity vocabulary

Unless explicitly stated otherwise, references in this document to a campaign "mode" mean the **recoverable campaign mode**: the verifier-visible and dispatch-relevant campaign classification that must be unambiguously derivable from persisted state and runtime state, regardless of how the implementation represents it internally.


The subsystem shall maintain campaign state sufficient to distinguish the following observable campaign modes for the purposes of dispatch, resume, and verification:

- **hyperspace navigation** — the player is traversing hyperspace or quasispace,
- **interplanetary / solar-system** — the player has entered a solar system,
- **encounter** — a campaign encounter (random or scripted) is being resolved through communication and/or battle,
- **starbase visit** — the player is visiting the allied starbase, and
- **last battle** — the final story battle is in progress.

Victory and defeat are terminal campaign outcomes that result from loop exit (§5.2), not resumable or dispatchable campaign modes. They are observable results of leaving the campaign loop, not entries in the campaign-mode vocabulary used for dispatch, resume, or export.

These are the required observable campaign modes. The contract is about externally meaningful mode distinctions, not a required flat internal enum or first-class persisted discriminator. Each mode listed above must be distinguishable at dispatch, resume, and verification time, but the implementation may represent any mode through whatever internal or persisted structure it chooses — including indirect encodings — so long as the observable mode is unambiguously recoverable.

In particular, starbase visit is a required distinct observable campaign mode: dispatch, resume, and verification shall treat it as distinguishable from encounter or any other mode. The implementation may represent starbase as its own top-level activity value, or it may encode starbase through encounter-family state plus explicit starbase-context markers (e.g., a bitfield flag in the game-state), or any equivalent representation — so long as dispatch and resume behavior satisfy the starbase contracts in §5.1 and §6.6. At the top-level selection boundary, starbase is reached through the encounter/starbase dispatch family described in §5.1. The subsystem shall recognize the starbase case within the encounter dispatch path and route to the starbase visit flow (§6.4) instead of normal encounter/communication flow when the encounter context identifies a starbase visit.

The subsystem shall also maintain transition flags sufficient to request activity changes, including at minimum:

- a flag requesting entry into an encounter or starbase visit from hyperspace or interplanetary context,
- a flag requesting entry into interplanetary from hyperspace, and
- control flags for load, restart, and abort that can interrupt the current activity.

The exact internal representation of these states and flags is not prescribed, but the externally visible dispatch behavior described below shall be preserved.

### 3.2 Campaign runtime state

The subsystem shall maintain runtime state sufficient to represent:

- the recoverable campaign mode (which may be represented indirectly through encounter-family state plus markers such as the starbase-context marker, per §3.1),
- pending transition flags,
- the game clock (current date, tick state, scheduled events) — owned by the clock subsystem, referenced by campaign gameplay,
- the player's hyperspace/quasispace navigation position,
- the player's current or last solar-system context sufficient to resume interplanetary on load,
- encounter queue state (pending hyperspace encounters),
- NPC ship queue state (ships involved in the current encounter),
- escort queue state (the player's fleet roster),
- the game-state bitfield (campaign progression flags),
- autopilot target, and
- ship stamp/orientation/velocity state sufficient for save/load round-trip.

The exact internal data layout is not prescribed, but the subsystem shall preserve the externally visible behavior described in this specification.

## 4. New-game and load-game entry

### 4.1 Entry flow

On campaign entry, the subsystem shall present or process a new-game / load-game decision. This decision may be integrated into a start/restart menu flow that also offers other top-level modes (e.g., SuperMelee).

The entry flow shall loop until a valid campaign start condition is established: the user selects new game, the user selects load game and a valid save is loaded, or the user exits the start flow entirely.

### 4.2 New-game initialization

When a new game is selected:

- the subsystem shall initialize all campaign runtime state to the defined starting configuration,
- the subsystem shall ensure the campaign begins at the defined campaign start date with the initial set of campaign events scheduled so that they will fire at their defined times during subsequent play,
- the subsystem may play an introductory sequence before entering the campaign loop, and
- the subsystem shall enter the main campaign loop in interplanetary mode at the Sol starting location, consistent with the legacy baseline start-flow behavior evidenced in `initialstate.md` and the current baseline start-state initialization. Introductory-sequence presentation may vary so long as, after that sequence (if any), the recovered campaign starting mode/location is the same campaign-boundary start state.


### 4.3 Load-game initialization

When a load game is selected and a valid save is loaded:

- the subsystem shall restore campaign runtime state from the loaded save,
- the subsystem shall restore game clock state and scheduled event state via the clock subsystem so that campaign date progression and event behavior continue as they would have from the saved state,
- the subsystem shall restore fleet roster, active encounter, and NPC ship state from the save,
- the subsystem shall resume the campaign loop in the campaign mode and location recoverable from the save, and
- the subsystem shall normalize the restored state so that interplanetary resumes target the saved destination system with matching campaign-owned entry conditions (see §9.4), starbase resumes re-enter the starbase visit flow, and quasispace resumes are handled (see §9.4 for detail).

### 4.4 Restart behavior

The subsystem shall support returning to the start/restart flow from within a running campaign when load, restart, or abort is requested. On restart:

- the subsystem shall tear down the current campaign runtime state and request clock shutdown,
- the subsystem shall return to the entry flow described in §4.1, and
- the subsystem shall not carry stale campaign state into a subsequent new-game or load-game session.

## 5. Main campaign loop and activity dispatch

### 5.1 Loop structure

The main campaign loop shall repeatedly select and dispatch to the current campaign activity until a terminal condition is reached. On each iteration:

1. If a deferred transition is pending (see §5.3), the loop shall adopt the target activity from the pending transition and proceed with that activity.
2. If an encounter or starbase start is requested, the loop shall dispatch to the encounter/starbase flow. If the encounter context identifies a starbase visit, the loop shall route to the starbase visit flow (§6.4); otherwise, the loop shall route to the encounter/communication flow (§6.1).
3. If an interplanetary start is requested, the loop shall dispatch to the solar-system exploration entry point (§7.2).
4. Otherwise, the loop shall enter hyperspace/quasispace navigation (§7.1).

### 5.2 Terminal conditions

The campaign loop shall exit when:

- the player wins the final battle (the last-battle activity reaches a victorious conclusion), or
- the player dies (the flagship is destroyed or equivalent defeat condition), or
- a restart/abort is requested.

On loop exit, the subsystem shall request clock shutdown and tear down campaign runtime structures before returning control to the caller.

### 5.3 Deferred-transition semantics

The subsystem shall support a mechanism by which a sub-activity (e.g., starbase, load initiated from a sub-activity) can request a deferred transition to a different campaign activity. This mechanism shall have the following observable properties:

- a sub-activity can designate a target activity and associated entry flags for the transition,
- the main campaign loop processes the transition on its next selection cycle, adopting the target activity and clearing the pending state,
- the sub-activity exits before the transition takes effect, so the main loop — not the sub-activity — dispatches to the target activity,
- the target activity is entered with the same externally visible initialization and entry behavior as if the top-level campaign loop had selected that activity directly in the normal course of play,
- the deferred transition shall not present itself as a real save/load operation to the player, shall not mutate or consume campaign save slots or persisted campaign save state as part of the handoff, and shall not alter the set of available saves or their content.

> **Non-normative note — current implementation context:** The current C implementation realizes this pattern through a `NextActivity` global: the sub-activity stores the target in `NextActivity`, sets `CHECK_LOAD` in `CurrentActivity`, and the main loop in `starcon.c` adopts `NextActivity` on the next iteration. An end-state implementation may use this exact mechanism, a typed transition request, or any other approach that preserves the observable properties above.

## 6. Encounter and starbase handoff

### 6.1 Encounter dispatch

When an encounter is triggered (by hyperspace collision with an NPC group or by scripted event), the subsystem shall:

- identify the encountered race from the current encounter context or NPC state,
- dispatch to the appropriate communication/dialogue entry point for that encounter, and
- upon return from dialogue, determine whether combat occurs (battle segue) or the encounter resolves peacefully.

### 6.2 Battle segue

When an encounter leads to combat:

- the subsystem shall prepare encounter handoff inputs from the current encounter context (the campaign-owned data needed to invoke battle: participating ship identities, fleet composition from campaign queues, and backdrop selection),
- the subsystem shall select the battle backdrop based on the current campaign context (hyperspace, last battle, planetary, or default),
- the subsystem shall invoke the battle subsystem,
- the subsystem shall record battle outcome counters, and
- on return from battle, the subsystem shall restore the prior campaign activity.

The subsystem owns the encounter-level handoff inputs and post-battle campaign consequence processing. It does not own the combat simulation itself.

### 6.3 Post-encounter processing

After an encounter resolves (whether through dialogue, combat, or a combination), the subsystem shall:

- determine the victory/defeat/escape outcome from battle counters and story state,
- identify the encountered race,
- apply campaign consequences: remove defeated ships from encounter or escort queues, award salvage or resources as appropriate, update campaign progression flags, and
- clean up encounter state so the campaign can resume the prior navigation activity.

The subsystem shall suppress post-encounter processing when the encounter exits due to abort, load, player death, or final-battle resolution, since those cases transfer control outside normal encounter flow.

### 6.4 Starbase visit flow

When the player visits the allied starbase, the subsystem shall:

- enter the starbase visit mode via the encounter/starbase dispatch path described in §5.1, which may include special-case handling for the mandatory special-sequence categories enumerated below,
- manage pre-alliance starbase sequences (Commander conversation, conditional Ilwrath response battle),
- advance the game clock when story beats require a time skip at the starbase,
- run the starbase menu loop (Commander/outfit/shipyard), and
- on starbase departure, resume campaign flow in interplanetary with the correct entry semantics (see §6.5).

**Mandatory special-sequence categories:** For this subsystem contract, starbase conformance is intentionally scoped to the following closed set of mandatory special sequences:

1. **Bomb-transport special sequence** — the story-critical sequence in which the player's bomb device is installed or transported, gating subsequent starbase flow until completion.
2. **Pre-alliance Ilwrath-response sequence** — the conditional Ilwrath battle that may occur during the pre-alliance Commander conversation; includes the battle branch and the return to conversation after battle resolution.


**Minimum retained starbase persistence facts:** For every covered starbase save context, the persisted campaign-owned meaning shall retain enough information to reconstruct the closed progression-point contract at load. At minimum, that retained meaning shall distinguish: (a) whether the forced Commander conversation relevant to first availability or post-bomb-installation is pending or completed; (b) the bomb-transport special-sequence status; (c) the pre-alliance Ilwrath-response status; and (d) enough campaign-owned facts to determine which mandatory route would become actually executable first under the zero-optional-input settlement procedure if more than one mandatory route is latent at save time. That winning latent route is a derivation aid for the closed settled comparison object, not an additional hidden comparison object beyond the settled progression-point elements and mandatory-next-action outcome. Implementations may encode or derive that latent-route fact differently so long as the same settled closed progression-point result and mandatory-next-action outcome are produced at the post-load conformance observation point.


These are the only mandatory special-sequence categories within the starbase contract. Other starbase interactions (Commander conversation, outfit, shipyard, departure) are covered by the normal starbase menu flow and forced-conversation rules, not by the mandatory special-sequence category.

### 6.5 Starbase departure

When the player departs the starbase normally, the subsystem shall resume campaign navigation in interplanetary mode with the interplanetary entry flag set. The departure shall use the deferred-transition mechanism described in §5.3 so the main campaign loop — not the starbase activity — dispatches to the interplanetary activity on the next selection cycle.

### 6.6 Starbase save/load resume

A save made while in starbase shall record enough campaign state that the starbase context is recoverable on load (the current implementation persists this via the `GLOBAL_FLAGS_AND_DATA` marker in the game-state bitfield). On load, the subsystem shall detect that the saved state represents a starbase visit — whether that is encoded as a distinct persisted value, an encounter-family activity plus a starbase-context marker, or any equivalent representation — and resume accordingly:

- the subsystem shall detect the starbase context from the restored campaign state,
- the subsystem shall resume into the starbase visit flow at the **closed starbase progression-point contract** defined by the closed mandatory-element progression-point definition and the mandatory-next-action rule in `requirements.md`.

For conformance, the required restored behavior is the normalized settled progression-point result and mandatory-next-action outcome only. The implementation is not required to restore hidden pre-settlement shell/menu-local state, transient presentation state, or other unlisted starbase-local detail unless preserving that state is necessary to produce the same closed progression-point facts or the same mandatory-next-action outcome at the post-load conformance observation point.

**Starbase post-load conformance observation point:** Conformance of the starbase progression-point assessment after load is anchored to the following observation point: after load completes and after any mandatory automatic routing required by the restored starbase state has settled, but before any optional player interaction. The closed mandatory-element progression-point matrix in `requirements.md` is evaluated at this observation point. Implementations may pass through intermediate shell or menu presentation during settlement, but the progression-point assessment applies to the state that is stable at the observation point.

**Starbase comparison normalization rule:** For starbase save/load claims, both sides of the comparison are evaluated only after applying the same zero-optional-input settlement procedure to reach the settled observation-state at the starbase post-load conformance observation point. Raw save-instant shell/menu-local state is not compared directly unless specification §6.6 explicitly makes it part of the closed comparison object.


The externally observable contract for a starbase save/load round-trip is **observable equivalence within the closed starbase progression-point contract**: the post-load state must match the save-time state only with respect to the closed mandatory-element progression-point definition and mandatory-next-action rule. **[End-state policy]** This closed matrix and its associated settlement/comparison procedure are the project's chosen acceptance surface for this compatibility-sensitive flow. They define the required target behavior for conformance and are not a claim that every distinction in the matrix is independently surfaced as a first-class legacy persistence token or exhaustively evidenced in current-state analysis. Specifically:
**Policy-closure rationale for the starbase comparison surface:** This closed progression-point matrix is adopted because the cited current-state evidence establishes starbase as a distinct campaign-owned resume boundary with mandatory gating and fake-load-style normalization, while not evidencing a simpler raw persistence tokenization that would make the same compatibility-sensitive distinctions verifier-stable. The matrix therefore serves as the project's intentional minimum policy closure for player-visible starbase compatibility at that boundary: omitted starbase-local distinctions are out of contract unless they can change the closed progression-point elements or mandatory-next-action outcome.


- no mandatory starbase conversation or sequence that was already completed before the save was made shall replay solely because of load,
- no mandatory starbase conversation or sequence that was still pending at save time shall be skipped because of load,
- the post-load starbase state shall match the save-time state with respect to every element in the closed starbase progression-point contract in `requirements.md` (see the "Player-visible starbase progression point" definition there for the complete list),
- after starbase-context load, no non-mandatory starbase interaction shall become available before the pending mandatory action or sequence is surfaced; implementations may pass through an intermediate shell or menu presentation so long as no non-mandatory interaction becomes possible before the mandatory action is presented,
- if a forced conversation or mandatory special sequence is pending at save time, it shall surface after load before the player can access any non-mandatory starbase interaction, and
- subsequent normal departure from starbase after such a load shall behave identically to departure without an intervening save/load, as specified in §6.5.

For starbase-context loads, conformance is determined solely by the closed mandatory-element progression-point definition and the mandatory-next-action rule; visual/menu-local restoration beyond that closed set is neither required nor implied.

## 7. Navigation transitions at the campaign layer

### 7.1 Hyperspace/quasispace runtime

When the campaign is in hyperspace or quasispace:

- the subsystem shall ensure that campaign time advances at the hyperspace pacing rate while the player navigates hyperspace or quasispace,
- the subsystem shall enter the hyperspace navigation runtime,
- transitions out of hyperspace shall be handled by campaign-layer transition logic, not by the navigation runtime directly.

### 7.2 Interplanetary entry

When the campaign transitions to interplanetary:

- the subsystem shall ensure that campaign time advances at the interplanetary pacing rate while the player is in solar-system exploration,
- the subsystem shall invoke the solar-system exploration entry point, and
- on return, the subsystem shall resume the campaign loop for the next activity selection.

On resume or transition into interplanetary/solar-system exploration, the next dispatched exploration entry shall target the saved/selected destination system and the campaign-boundary interplanetary entry observables (§7.2.1) shall match the saved/selected context.

#### 7.2.1 Campaign-boundary interplanetary entry observables

The following closed set of campaign-boundary observables defines the campaign-owned entry conditions for interplanetary/solar-system exploration dispatch. References elsewhere in this document and in `requirements.md` to "campaign-owned entry conditions needed for that exploration dispatch" refer to this set:

- **Destination system identity** — the identity of the solar system being entered, sufficient to select the correct system for exploration dispatch.
- **Entry routing kind** — whether the next boundary handoff is normal interplanetary exploration entry or special-case encounter routing (e.g., Arilou homeworld space routes to an encounter instead of normal exploration; see §7.4).
- **Campaign-boundary transition marker** — the normalized campaign-boundary token needed to produce the correct first exploration dispatch behavior for the destination system. For all covered contexts, this observable normalizes to the closed set `none`, `clear_start_interplanetary`, or `preserve_start_encounter`.

This set is closed for the purposes of the interplanetary entry contract. Verifiers confirm that these observables match the saved/selected context; exploration-internal behavior after the first dispatch is outside this boundary. For covered contexts, the `campaign_transition_marker` sufficiency mapping is: `none` for ordinary interplanetary entry, including quasispace-origin entries whose first dispatch needs no additional campaign-boundary transition beyond destination identity and routing kind; `clear_start_interplanetary` when the restored campaign-side handoff must clear a pending interplanetary-start condition before the first exploration dispatch; and `preserve_start_encounter` when the restored campaign-side handoff must preserve a pending encounter-start routing instead of normal exploration entry, including Arilou-homeworld-style special-case encounter destinations. No additional marker values are required for covered contexts.

### 7.3 Hyperspace-to-encounter transition

When a hyperspace encounter is triggered (collision with an NPC group):

- the subsystem shall ensure the resulting encounter corresponds to the collided NPC group,
- the subsystem shall save the player's hyperspace navigation state so it can be restored after the encounter,
- the subsystem shall request entry into encounter flow, and
- the main campaign loop shall dispatch to encounter flow on the next iteration.

### 7.4 Hyperspace-to-interplanetary transition

When the player enters a solar system from hyperspace:

- the subsystem shall transition to solar-system exploration targeting the destination system, with the campaign-boundary interplanetary entry observables (§7.2.1) matching the selected context, and
- if the destination is the Arilou homeworld space (or equivalent special-case system), the subsystem shall route to an encounter instead.

### 7.5 Arilou/quasispace transition

The subsystem shall handle transitions between hyperspace and quasispace as a campaign-layer navigation event, including any special-case portal mechanics defined by the campaign.

### 7.6 Hyperspace menu

The hyperspace menu is campaign-facing orchestration. It shall support device usage, cargo display, roster display, save/load, starmap access, and return to navigation. When a menu action triggers a campaign transition (e.g., device usage causes an encounter), the menu shall exit and the campaign loop shall process the resulting transition flag.

## 8. Event progression and campaign clock

### 8.1 Ownership boundary

Campaign gameplay owns the semantic definition of campaign event types, their registration policy, and the effects of their handlers. The clock subsystem owns generic scheduling mechanics, event-queue persistence and restoration, and due-event dispatch. This division means:

- the campaign subsystem decides which events exist, when they are initially scheduled, what they mean, and what side effects they produce,
- the clock subsystem provides the scheduling infrastructure, fires events when they come due, and persists/restores the scheduling queue across save/load, and
- persisted or restored scheduling state does not transfer semantic ownership of those events to the clock subsystem — the campaign subsystem remains the authority on event meaning and handler behavior regardless of where the scheduling data physically resides.

Campaign gameplay owns semantic interpretation of scheduled events and may perform broader semantic validation of restored scheduling state beyond the concrete rejection cases listed in §9.4.1. However, conformance requires only the concrete persistence-boundary rejection cases defined in §9.4.1; additional semantic validation is permitted but non-normative.

Semantic ownership is a contract-level ownership boundary, not a source-tree or module-placement constraint. Campaign gameplay remains the normative owner of campaign-event meaning and externally visible effects regardless of whether implementation code is physically located in campaign modules or in shared infrastructure, so long as the observable and persistence-boundary contracts defined in this document are preserved.

### 8.2 Initial event registration

On new-game start, the subsystem shall register the initial set of campaign events with the clock subsystem. The initial events shall be:

| Event selector | Initial scheduling |
|---|---|
| `HYPERSPACE_ENCOUNTER_EVENT` | Relative: 0 months, 1 day, 0 years from start |
| `ARILOU_ENTRANCE_EVENT` | Absolute: month 3, day 17 of start year |
| `KOHR_AH_VICTORIOUS_EVENT` | Relative: 0 months, 0 days, campaign-defined years-to-victory from start |
| `SLYLANDRO_RAMP_UP` | Relative: immediate (0/0/0) |

Other events in the campaign event catalog (§8.6) are not initially scheduled; they are triggered by campaign progression (event handlers, dialogue outcomes, or story-flag changes) during play.

### 8.3 Campaign event handlers

The subsystem shall implement campaign event handlers that, when the clock subsystem fires a scheduled event, produce the defined campaign progression effects. Those effects may include:

- updating story flags in the game-state bitfield,
- changing faction fleet destinations, strengths, or behaviors,
- changing alliance state,
- triggering genocide or extinction events,
- adjusting encounter generation parameters,
- scheduling follow-on events with the clock subsystem, and
- any other campaign-progression side effects defined by the event catalog.

### 8.4 Clock rate policy

The subsystem shall ensure that campaign time advances at different rates depending on the current campaign activity:

- hyperspace/quasispace navigation uses the hyperspace pacing rate,
- interplanetary/solar-system exploration uses the interplanetary pacing rate, and
- starbase visits may advance time in discrete day increments rather than continuous ticking.

The exact tick-rate values are not prescribed by this specification, but the subsystem shall preserve the relative pacing behavior: hyperspace time passes at a different rate than interplanetary time. The required observable property is that campaign events and faction fleet movements proceed at pacing consistent with the current activity mode.

### 8.5 Day advancement

The subsystem shall support requesting day advancement from the clock subsystem by a specified number of days, independent of continuous ticking. This is used for story-driven time skips (e.g., starbase arrival sequences).

### 8.6 Campaign event catalog
The campaign event catalog is the canonical compatibility and persistence-boundary event vocabulary. For every selector in this catalog, the end-state shall support the selector vocabulary at the persistence boundary and shall preserve the selector's required externally visible outcome family when that event fires. Scheduled-event persistence is an intentional exactness exception to the general semantic-equivalence default stated in §1: the persistence boundary must preserve the normalized selector vocabulary and required comparable scheduling facts defined here so that legacy-save interoperability and reproducible external verification remain possible. Where this section additionally defines checkpoint bundles, canonical-stage surfacing rules, or other verifier-facing comparison normalizations for specific rows, those clauses are **[End-state policy]** verifier normalizations for stable comparison of the selector's covered persistence-boundary meaning and downstream outcomes; they do not imply that every internal legacy stage distinction was itself a first-class legacy persistence token.


Internal event decomposition and modeling may differ from the catalog's selector taxonomy so long as the implementation preserves the compatibility-surface obligations and the observable effects defined for each event.

**[End-state policy] Architectural rule — primary-artifact-only scheduled-event canonicalization:** The canonical persistence-boundary scheduled-event view shall be deterministically recoverable from the primary persisted save artifact alone. This is an intentional inspection-surface stability rule for the end-state contract: scheduled-event canonicalization must have exactly one primary persisted-source of truth so that persistence-boundary comparison, legacy-save interoperability checks, and same-subsystem round-trip verification do not depend on combining scheduler facts from multiple persisted artifacts or on implementation-specific reconstruction order across artifacts. For all covered contexts in this document, scheduled-event canonical meaning is therefore defined to be wholly primary-artifact-resident even when overall campaign resume also depends on adjunct artifacts. Adjunct artifacts may matter to overall campaign resume and failed-load verification, but they are not additional inputs to scheduled-event canonicalization.

Accordingly, the internal event model and raw persisted scheduler encoding need not mirror the catalog taxonomy one-for-one, provided the canonical persistence-boundary scheduled-event view can be deterministically recovered from the primary persisted save artifact alone, the end-state accepts that vocabulary on load for valid legacy and end-state saves, and the externally visible effects of each catalog entry are preserved. Raw end-state scheduled-event entries may therefore use implementation-specific encodings or helper items so long as they do not prevent successful deterministic canonicalization into the required catalog-vocabulary meaning at covered persistence boundaries. The closed campaign resume adjunct artifact set defined in §9.4.0b is outside this scheduled-event canonicalization rule by design: adjunct artifacts may matter to overall campaign resume and failed-load verification, but they are not additional inputs to the canonical scheduled-event derivation contract.
**Closed canonicalization test:** For conformance, a persisted scheduled item is canonical if and only if omitting that item from the canonical scheduled-event view would change one of these two comparison-critical facts at that persistence boundary: (1) the canonical selector/due-date/recurrence comparison object for the save being verified, or (2) the next required catalog-defined checkpoint/outcome set under the same subsequent gameplay scenario. If omission changes neither, the item is helper state and shall remain non-canonical. This closed test is the entire operative rule; later family notes, examples, and tables only illustrate its application.


**Persistence-boundary vocabulary obligation:** At the canonical persistence-boundary meaning layer, every persisted scheduled campaign event that passes the closed canonicalization test above shall be represented using the catalog selector vocabulary and the required scheduling metadata (due date and recurrence type). Items that fail that test are helper state and are not separate canonical entries. The raw persisted representation need not expose the catalog vocabulary directly if the same canonical meaning is deterministically recoverable from the primary persisted save artifact alone. The canonical scheduled-event meaning remains wholly primary-artifact-resident for covered contexts even when overall resume conformance also depends on adjunct artifacts.

**Mission-family row-level canonicalization cue:** For mission-family rows that name checkpoint bundles or independently meaningful active scheduled stages, a stage becomes canonically surfaceable exactly when omitting it from the canonical scheduled-event view at that persistence boundary would fail the closed canonicalization test by changing either (a) the selector/due-date/recurrence comparison object, or (b) the next required row-defined checkpoint/outcome set under the same subsequent gameplay scenario. If omission changes neither, the stage remains helper state for that boundary.


**Operational consequence of the closed canonicalization test:** If a persisted scheduled item passes the closed canonicalization test, it shall appear in canonical scheduled-event meaning using the catalog selector vocabulary and required scheduling metadata. If it does not pass that test, it is helper state and need not surface as a separate canonical entry. This is the single controlling canonicalization rule for mission families, helper timers, and scheduler decomposition.

**Legacy and end-state load acceptance:** Regardless of internal scheduler structure, the end-state shall accept catalog-vocabulary entries on load from both valid legacy saves and end-state-produced saves. When loading a valid legacy save, every catalog-vocabulary scheduled event present in the persisted schedule state shall be accepted and shall produce the required observable outcomes when it fires. When loading an end-state-produced save, the canonical scheduled-event view shall round-trip: events representable in the canonical view at save time shall be representable with the same catalog vocabulary and scheduling metadata after load.

**Inspection-representation rule for scheduled events:** When the canonical scheduled-event meaning is not directly inspectable from the raw persisted representation, the Campaign Canonical Export Document is the designated inspection representation for export-based verification. In that case, the export document shall expose selector identifiers using the catalog vocabulary and the required normalized scheduling metadata sufficient for conformance verification. Entry ordering in that inspection representation is not significant unless a normative section of this document or `requirements.md` explicitly states otherwise. These are verification-surface obligations for the inspection representation; the underlying compatibility obligation remains preservation of canonical scheduled-event meaning and observable outcomes.

**Recurrence/follow-on scheduling obligation scope:** Unless a normative section of this document or `requirements.md` explicitly states otherwise, the recurrence and follow-on scheduling clauses in the catalog table below define required persistence-boundary canonical scheduled-event view contents and downstream observable campaign effects. They do not require identical transient internal in-memory queue decomposition immediately after every handler execution. Conformance is judged by the canonical scheduled-event view at the next persistence boundary and by the downstream externally visible campaign effects, not by internal queue snapshots between persistence boundaries.

**Event-catalog verification boundary:** Catalog conformance has two distinct parts: (1) persistence-boundary obligations for any catalog event present in persisted schedule state, verifiable through the chosen inspection surface; and (2) observable-outcome obligations when the event fires, verifiable through player-visible behavior, persisted state inspection, or controlled round-trip scenario comparison as defined in `requirements.md`. The contract does not require every catalog row to have an independently inspectable manifestation at arbitrary save boundaries; the persistence-boundary obligation applies only to catalog events actually present in persisted schedule state at that boundary.




**Precedence rule for follow-on scheduling clauses:** When a catalog entry states that an event self-reschedules, schedules a follow-on event, or otherwise creates downstream scheduling consequences, that language creates two distinct obligations: (1) a downstream observable-outcome obligation when the relevant event flow is allowed to proceed, and (2) a persistence-boundary obligation that any resulting scheduled entry shall appear in the canonical scheduled-event view by the next persistence boundary at which that entry is active. It does not require immediate transient materialization of an internal runtime scheduler entry before the next persistence boundary unless another section of this document explicitly says so.


**Minimum canonical-view schema:** Each entry in the canonical scheduled-event view shall expose at minimum the following fields:

- **Selector identifier** — the catalog selector from the event vocabulary defined in this section.
- **Normalized due date** — the campaign date at which the event is next scheduled to fire, in a normalized date representation sufficient for unambiguous comparison.
- **Recurrence kind** — one of the following closed set of values:
  - `one-shot-absolute` — fires once at a fixed calendar date; no recurrence.
  - `one-shot-relative` — fires once at a date computed relative to the scheduling moment; no recurrence.
  - `recurring-absolute` — recurs, with each next occurrence determined by fixed calendar-date parameters.
  - `recurring-relative` — recurs, with each next occurrence determined by a relative offset from the previous firing or scheduling moment.
- **Recurrence parameters** — normalized scheduling metadata associated with the recurrence kind. For conformance comparison across implementations, the normative scheduling facts are the selector identifier, normalized due date, and recurrence kind. `recurrence_parameters` are required only to the extent needed to describe the persisted scheduling model used by that implementation and to support same-implementation inspection/export round trips; they are not an independent cross-implementation conformance axis unless a future schema version defines a tighter normalized parameter schema.

Multiple active entries with the same selector identifier are permitted and shall be represented as distinct entries in the canonical view. Queue ordering among entries is not significant unless explicitly required by a normative section of this document or `requirements.md`. The canonical view represents the active scheduled queue at the persistence boundary, not a derived forecast of future implicit recurrences beyond what is actually scheduled. If the canonical view is surfaced through the Campaign Canonical Export Document (§10.1), that surface shall expose this schema in a machine-readable, stable form suitable for conformance verification. See also the minimum-schema cross-reference in `requirements.md` (Persisted scheduled-event queue inspection).

| # | Event selector | Required externally visible effect(s) | Recurrence / follow-on scheduling |
|---|---|---|---|
| 0 | `ARILOU_ENTRANCE_EVENT` | Sets the Arilou portal state to open (Arilou space becomes accessible). | Schedules `ARILOU_EXIT_EVENT` as a relative event 3 days later. |
| 1 | `ARILOU_EXIT_EVENT` | Sets the Arilou portal state to closed (Arilou space becomes inaccessible). | Schedules `ARILOU_ENTRANCE_EVENT` as an absolute event on day 17 of the next month (wrapping year if needed). Recurs indefinitely via this entrance/exit cycle. |
| 2 | `HYPERSPACE_ENCOUNTER_EVENT` | Advances faction fleet strengths (growth/attrition), advances faction fleet positions toward their strategic destinations (producing destination-arrival outcomes when fleets arrive), and, if the player is currently in hyperspace, checks for and generates a hyperspace encounter. | Self-reschedules as a relative event 1 day later. Recurs indefinitely. |
| 3 | `KOHR_AH_VICTORIOUS_EVENT` | If the Utwig/Supox counter-mission is active, delays genocide by scheduling `KOHR_AH_GENOCIDE_EVENT` 1 year later instead. Otherwise, initiates genocide directly (see `KOHR_AH_GENOCIDE_EVENT`). | Conditional: schedules `KOHR_AH_GENOCIDE_EVENT` if counter-mission is active; otherwise processes genocide directly. |
| 4 | `ADVANCE_PKUNK_MISSION` | Advances the Pkunk migration arc. The Pkunk fleet moves toward Yehat space or reverses homeward depending on campaign conditions. On arrival at Yehat space, the Pkunk faction is absorbed into the Yehat. On arrival home, the Pkunk pause before migrating again. | Conditionally self-reschedules (relative, 3 months) when the Pkunk return home and wait. Otherwise, reschedules to advance fleet movement toward the destination. |
| 5 | `ADVANCE_THRADD_MISSION` | Advances the Thraddash campaign arc. The verifier-facing required checkpoint set is: (a) while the arc is active, the Thraddash strategic destination/home-state progression reflects outbound movement toward the Kohr-Ah front, then return-home progression; (b) at the combat checkpoint, Thraddash strength is reduced from its pre-battle value; and (c) at arc completion, the Thraddash are back in their post-arc home/return state with the campaign conditions required to make the Ilwrath/Thraddash war arc eligible. Exact internal staging is not prescribed beyond those checkpoints. At a covered persistence boundary, the catalog selector must appear canonically whenever the persisted state still contains an independently meaningful active scheduled stage of the Thraddash arc; preparatory helper entries that do not yet distinguish a different canonical campaign-boundary state may remain hidden until that condition is met. | Reschedules to advance fleet movement across arc stages. On final stage, may immediately schedule `ADVANCE_ILWRATH_MISSION`. |
| 6 | `ZOQFOT_DISTRESS_EVENT` | Sets the Zoq-Fot-Pik distress flag (distress signal becomes active in dialogue). Deferred if the player is currently at the Zoq-Fot-Pik homeworld. | When deferred: self-reschedules 7 days later. When not deferred: schedules `ZOQFOT_DEATH_EVENT` as a relative event 6 months later. |
| 7 | `ZOQFOT_DEATH_EVENT` | If distress is still active (unresolved), eliminates the Zoq-Fot-Pik faction (strength to zero, alliance state to dead). Deferred if the player is currently at the Zoq-Fot-Pik homeworld. | When deferred: self-reschedules 7 days later. Does not recur after executing. |
| 8 | `SHOFIXTI_RETURN_EVENT` | Sets the Shofixti to allied status, reduces crew cost, and resets crew-purchased counters. | Does not recur or schedule follow-on events. |
| 9 | `ADVANCE_UTWIG_SUPOX_MISSION` | Advances the Utwig/Supox counter-mission arc. The verifier-facing required checkpoint set is: (a) while the arc is active, Utwig and Supox strategic destination/home-state progression reflects movement toward the Kohr-Ah front; (b) at the combat checkpoints, Utwig and Supox strengths are reduced from their pre-arc or pre-combat values as the counter-mission progresses; and (c) at arc completion, both factions are back in their post-arc return/home state with the counter-mission-complete campaign condition established. Exact internal staging is not prescribed beyond those checkpoints. At a covered persistence boundary, the catalog selector must appear canonically whenever the persisted state still contains an independently meaningful active scheduled stage of the counter-mission arc; preparatory helper entries that do not yet distinguish a different canonical campaign-boundary state may remain hidden until that condition is met. | Self-reschedules via relative events to advance fleet movement and combat attrition across arc stages. |
| 10 | `KOHR_AH_GENOCIDE_EVENT` | If the Kohr-Ah frenzy has not yet started and the player is currently at the Sa-Matra system, defers by rescheduling 7 days later. Otherwise, initiates or continues the Kohr-Ah genocide. The verifier-facing required checkpoint set is: (a) when the event executes at a target, the selected victim faction is the nearest surviving faction to the Kohr-Ah fleet position under the campaign's distance rule, with Druuge chosen when the distance comparison is tied; (b) that victim faction is marked dead, with strength reduced to zero and alliance state updated to the dead/eliminated outcome; (c) frenzy/diplomacy-reset flags are updated consistently with genocide having advanced; and (d) if no surviving factions remain after application, the game-over outcome is reached with the all-factions-destroyed condition recorded. Intermediate helper decisions and scheduler decomposition are not part of the contract beyond these checkpoints. At a covered persistence boundary, the catalog selector must appear canonically whenever a genocide-targeting or deferred-genocide stage is actively scheduled and independently meaningful at that boundary; helper routing details used only to choose or reach the next target need not surface separately unless their omission would change the canonical campaign-boundary state or required downstream checkpoint set. | Self-reschedules by advancing fleet movement toward each target with this event as the arrival effect. When deferred: self-reschedules 7 days later. |
| 11 | `SPATHI_SHIELD_EVENT` | If the Spathi still have fleet strength, removes them from alliance and sets their strength to zero (shielded themselves). Deferred if the player is currently at the Spathi homeworld. | When deferred: self-reschedules 7 days later. Does not recur after executing. |
| 12 | `ADVANCE_ILWRATH_MISSION` | Advances the Ilwrath/Thraddash war arc. The verifier-facing required checkpoint set is: (a) while the arc is active, the Ilwrath strategic destination/home-state progression reflects movement toward the Thraddash theater; (b) at the mutual-combat checkpoint, both Ilwrath and Thraddash strengths are reduced from their pre-war values; and (c) at arc completion, both factions are in the dead/eliminated outcome with strength reduced to zero and alliance state updated consistently with that result. Exact internal stage structure is not prescribed beyond those checkpoints. At a covered persistence boundary, the catalog selector must appear canonically whenever the persisted state still contains an independently meaningful active scheduled stage of the Ilwrath/Thraddash war arc; preparatory helper entries that do not yet distinguish a different canonical campaign-boundary state may remain hidden until that condition is met. | Reschedules to advance fleet movement toward the destination with this event as the arrival effect. May immediately schedule `ADVANCE_THRADD_MISSION` for coordinated movement. |

| 13 | `ADVANCE_MYCON_MISSION` | Advances the Mycon mission arc: the Mycon fleet deploys to Organon, suffers attrition losses at Organon, and returns home with knowledge of the ambush. | Self-reschedules as a relative event (14 days) during attrition at Organon. Reschedules to advance fleet movement for movement phases. |
| 14 | `ARILOU_UMGAH_CHECK` | Sets the Arilou-checked-Umgah state flag to its completion value. | Does not recur or schedule follow-on events. |
| 15 | `YEHAT_REBEL_EVENT` | Splits the Yehat faction: reduces royalist strength to 2/3 and creates a rebel faction with matching strength. The rebel faction becomes observable as a distinct strategic presence. | Does not recur or schedule follow-on events. |
| 16 | `SLYLANDRO_RAMP_UP` | Increments the Slylandro probe encounter multiplier (up to a cap of 4), increasing probe encounter frequency. Only advances if the player does not yet have the destruct code. | Self-reschedules as a relative event 182 days later if the multiplier has not reached the cap and the destruct code is not present. |
| 17 | `SLYLANDRO_RAMP_DOWN` | Decrements the Slylandro probe encounter multiplier, decreasing probe encounter frequency. | Self-reschedules as a relative event 23 days later if the multiplier is still above zero after decrement. |

This catalog is derived from the legacy baseline implementation's event enum and handler code. The externally visible effects described above are normative; the internal mechanism by which they are achieved is not prescribed. Conformance of catalog outcomes is demonstrated through the verification surfaces defined in `requirements.md`.

## 9. Campaign save/load

### 9.1 Serialized campaign state

The subsystem shall serialize and deserialize enough campaign state to support full campaign resume. At minimum, the serialized payload shall include:

- recoverable campaign mode (which may encode starbase visit indirectly through encounter-family state plus starbase-context markers, per §3.1),
- game clock state (current date and tick state),
- autopilot target,
- interplanetary location (system coordinates, planet context),
- ship stamp, orientation, and velocity,
- orbit flags,
- the game-state bitfield (all campaign progression flags, including the starbase-context marker),
- escort queue (player fleet roster),
- NPC ship queue / battle-group state, and
- hyperspace encounter queue.

### 9.2 Save summary

The subsystem shall derive a user-facing save summary from the current campaign state. The summary shall reflect the player's current activity with appropriate remapping for special contexts (quasispace, starbase, planet orbit, final battle).

**Save-summary normalization for covered contexts:** For the covered save contexts in §9.7, the save summary compatibility surface is normalized as follows:

| Covered context | `summary_type` | `location_id` rule |
|---|---|---|
| Hyperspace navigation | `hyperspace` | `hyperspace:<x>,<y>` |
| Quasispace navigation | `hyperspace` | `hyperspace:<x>,<y>` using the remapped hyperspace-equivalent coordinates required by the legacy summary behavior |
| Interplanetary / solar-system entry | `interplanetary` | `system:<x>,<y>` using the baseline system coordinates of the destination system |
| Starbase visit | `starbase` | `starbase:sol` |
| Campaign-owned encounter-entry handoff state | `encounter` | `encounter:<encounter_identity>` |
| Post-encounter campaign consequence state | the `summary_type` and `location_id` of the recoverable campaign mode that will resume after the consequence state is applied; it does not remain `encounter` unless the recoverable mode itself is still `encounter` |
| Final-battle save contexts | `last_battle` | `last_battle:sa_matra` |
| Sub-activity-initiated campaign loads (hyperspace menu) | the `summary_type` and `location_id` of the loaded save's recoverable campaign mode after normalization under this table |
| Homeworld encounter screen saves | `encounter` | `encounter:<encounter_identity>` |

For save-summary normalization claims, exactness applies to the canonical normalized summary object after any permitted deterministic mapping from the chosen inspection surface to the normalized `summary_type`, `location_id`, and date tokens defined here; it does not require literal identity of the underlying raw field decomposition when the chosen surface lawfully derives the same canonical tokens.


**Post-encounter summary observation point:** For post-encounter campaign consequence saves, save-summary comparison is anchored to the normalized campaign-owned consequence boundary defined in §9.7: after campaign-owned consequence application is complete, after salvage/progression-flag updates and affected-queue cleanup are complete, and before any further resumed navigation or exploration progression changes the recoverable campaign-boundary meaning. Summary normalization for this context is judged only at that stabilized boundary.

For covered contexts not listed separately above, the save summary shall normalize from the recoverable campaign mode under this table rather than inventing additional `summary_type` values.

### 9.3 Save semantics

When a save is requested:

- the subsystem shall write the save summary,
- the subsystem shall write full campaign game state,
- the subsystem shall write queue data (escort, NPC/battle-group, encounter),
- the subsystem shall handle campaign-specific save-time adjustments for special activity contexts (e.g., homeworld encounter screens, interplanetary re-entry normalization), and
- battle-group state files shall be persisted for systems with active NPC groups.

### 9.4 Load semantics

When a save is loaded:

- the subsystem shall restore all serialized campaign state fields,
- the subsystem shall derive the resume campaign mode from the loaded state,
- **when** the loaded state represents quasispace navigation or a quasispace-origin transition into interplanetary/solar-system exploration, conformance is judged solely by the same campaign-boundary interplanetary entry observables defined in §7.2.1 together with the resume-context requirements in §10.1; no additional quasispace-specific hidden resume fields are required by this contract,

- **when** the loaded state represents an interplanetary mode without a starbase context, the subsystem shall ensure that the next dispatched exploration entry targets the saved destination system and that the campaign-boundary interplanetary entry observables (§7.2.1) match the saved context,
- **when** the loaded state represents a starbase context (whether encoded as a distinct persisted value, an encounter-family activity plus a starbase-context marker, or any equivalent representation per §3.1), the subsystem shall resume into the starbase visit flow as specified in §6.6, and
- the subsystem shall be prepared for the main campaign loop to process the restored state on its next iteration.

For starbase-context saves, "equivalent to save time" is constrained by the closed mandatory-element progression-point definition and the mandatory-next-action rule in `requirements.md`. Only the elements listed in that closed definition are within the equivalence contract. All other starbase-local state — including menu subentry context, currently open submode or screen before commitment, and any other non-mandatory local presentation state — is outside the contract unless specifically listed in the closed progression-point definition.

### 9.4.0a Non-starbase save/load observable equivalence scope

Outside the starbase context, save/load equivalence is limited to the following campaign-boundary observables:

- recoverable campaign mode (the observable campaign mode is unambiguously recoverable from the save),
- navigation/system location sufficient for correct next dispatch (hyperspace/quasispace coordinates, or interplanetary system identity sufficient to target the correct destination system on exploration re-entry),
- fleet roster (escort queue membership and composition),
- active encounter presence/identity where applicable (whether an encounter is active and the encounter identity at the campaign boundary),
- campaign-owned encounter-entry handoff inputs where the save context is at that boundary (encounter identity, NPC ship queue, and fleet composition at the point of handoff),
- campaign progression flags (the game-state bitfield),
- clock/date state (current campaign date and tick state), and
- scheduled-event resume behavior (event-resume acceptance as defined in `requirements.md`).

Unlisted UI-local or transient runtime state is outside the equivalence contract for non-starbase contexts unless another normative section of this document or `requirements.md` explicitly states otherwise.

#### 9.4.0b General load-failure contract

All campaign-owned load-state failures — including parse failures, structural corruption, missing required persisted components, malformed payload sections, and any other persistence-boundary failure in campaign-owned serialized state — are subject to the following safe-failure guarantees:
**Protected persisted artifact set for rejected loads:** For this subsystem contract, the persisted artifacts protected by the no-mutation guarantee after a rejected covered load are exactly: (1) the campaign save slot and primary persisted save artifact being loaded, and (2) artifacts from the closed campaign resume adjunct artifact set that were touched by that load attempt — battle-group state files and per-system state files only. No other persisted artifact is protected by this subsystem contract unless a later normative amendment explicitly names it.


- No portion of the rejected save shall become the active resumed campaign state.

**Adjunct-dependency table for covered contexts:** For the covered contexts in §9.7, adjunct-artifact dependency is closed as follows:
- **Hyperspace navigation:** no adjunct artifact required.
- **Quasispace navigation:** no adjunct artifact required.
- **Interplanetary / solar-system entry:** per-system state files are required only when the resumed exploration/system context depends on persisted per-system campaign state surfaced through the state-file boundary; otherwise no adjunct artifact is required.

**Campaign-load commit point:** A resumed campaign state becomes active for conformance purposes only after all campaign-owned persistence-boundary validation succeeds, all required adjunct-artifact restores for the covered context succeed through their owning boundaries, and control reaches the resumed campaign boundary ready for the next in-contract dispatch. Before that commit point, implementations may perform staging, tentative initialization, rollback, or teardown work internally. Such temporary work is permitted provided it does not leave any externally observable resumed campaign state, persisted mutation, or other surviving effect from the rejected save after failure.

- **Starbase visit:** no adjunct artifact required for the closed starbase progression-point contract.
- **Campaign-owned encounter-entry handoff state:** battle-group state files are required when the covered encounter resume depends on persisted NPC group composition or battle-group state surfaced through the state-file boundary; otherwise no adjunct artifact is required.
- **Post-encounter campaign consequence state:** battle-group state files and/or per-system state files are required only when the stabilized post-consequence campaign boundary meaning for that save depends on persisted updates surfaced through those state-file boundaries; otherwise no adjunct artifact is required.
- **Final-battle save contexts:** no adjunct artifact required unless a later normative amendment explicitly adds one.
**Pre-load adjunct-dependence rule:** Mandatory adjunct-failure decisions are determined from the covered context's pre-load primary-artifact facts at the campaign boundary, including raw legacy encoding where that is the authoritative legacy surface for the covered context. If those pre-load primary-artifact facts indicate that one of the closed adjunct-backed campaign-boundary facts listed above is required to determine the next in-contract resumed campaign meaning, the context is adjunct-dependent and a missing, unreadable, structurally invalid, or owning-boundary-rejected required adjunct artifact is a mandatory load rejection. Export is never required in order to decide adjunct dependence for a load rejection.

**Verifier-facing adjunct-dependence classifier:** For claim classification and reporting, a covered context is adjunct-dependent if and only if the chosen inspection surface for that context or the pre-load primary-artifact facts authoritative for that context positively indicate that one of the closed adjunct-backed campaign-boundary facts listed above is required to determine the next in-contract resumed campaign meaning. Verifiers shall not infer adjunct dependence merely from broad save category, visible screen type, or implementation architecture. If neither the chosen inspection surface nor the authoritative pre-load primary-artifact facts positively indicate such a dependency for the covered context, adjunct artifacts are not required for that context's claim-local conformance determination.

A covered context counts as adjunct-dependent only when the authoritative pre-load primary-artifact facts indicate a resumed state whose correct campaign-boundary resume would be incomplete without the named state-file-backed data. If no such dependency is indicated at the covered boundary, the context is treated as non-adjunct-dependent for conformance.



A covered context counts as adjunct-dependent only when the primary save artifact indicates a resumed state whose correct campaign-boundary resume would be incomplete without the named state-file-backed data. If no such dependency is indicated at the covered boundary, the context is treated as non-adjunct-dependent for conformance.

- No partial application of state from the rejected save shall be externally observable after the failure. This guarantee applies to post-failure externally observable and persistence-boundary state: no resumed gameplay state, no persisted state mutation, and no other externally observable runtime state derived from the rejected save may remain active after rejection. Temporary internal restoration work (e.g., staging, trial parsing, or rollback of lower-boundary state) may occur during validation, but is not a conformance violation so long as no externally observable or persisted effect of the rejected save survives the failure.
- If the load was initiated from the entry/start flow, control shall return to the start/load flow without entering resumed gameplay.
- If the load was initiated from within a running campaign sub-activity, the pre-load running campaign session shall remain active and the user shall remain outside resumed gameplay from the rejected save. This contract preserves campaign-boundary state and next-dispatch semantics only; it does not require return to the exact same transient submenu, shell, cursor position, or other sub-activity-local presentation state from which the load was initiated.


Structurally parseable campaign-save inputs that fall outside the mandatory acceptance sets (valid legacy saves in covered contexts and same-subsystem round-trip saves) and do not trigger one of the mandatory rejection cases in `requirements.md` are outside the conformance contract if they are semantically inconsistent in some other campaign-owned way. Implementations may accept or reject such inputs. If they reject them, the general load-failure contract in this section shall hold.

These guarantees apply to all persistence-boundary parse and structural failures in campaign-owned load state, not only to the mandatory scheduled-event rejection cases in §9.4.1. The scheduled-event rejection cases defined in §9.4.1 are a specific mandatory subset of this broader contract: they define concrete cases where load **shall** fail, and when load fails for those (or any other) campaign-owned persistence-boundary reasons, the safe-failure guarantees above shall hold.

**Cross-boundary load-failure seam rule:** The overall campaign resume operation shall fail safely under the guarantees above if any required dependency cannot be successfully restored through its own owning boundary — including campaign-owned state sections, clock-restored schedule data rejected by campaign gameplay (§9.4.1), or data from the closed **campaign resume adjunct artifact set** when that save/context actually requires such adjunct data for correct campaign resume — regardless of which boundary detects the failure first. For covered save/context classes that require an artifact from the closed campaign resume adjunct artifact set, a missing, unreadable, structurally invalid, or owning-boundary-rejected required adjunct artifact is itself a mandatory campaign-load rejection condition: campaign load shall fail and the campaign shall not resume from that save. Campaign resume is not considered committed until all required dependent restores, including campaign-owned acceptance of restored scheduled-event state, have succeeded. Lower-boundary staging, trial restore, or rollback is permitted, and different implementations may choose different internal transaction boundaries, provided that no externally observable runtime state or persisted mutation from the rejected save survives the failure. For this contract, the **campaign resume adjunct artifact set** is the closed verifier-facing set of campaign-required persisted artifacts explicitly surfaced at the persistence boundary by the save/load interface in addition to the primary save artifact: battle-group state files and per-system state files used by campaign resume. These artifacts are in conformance scope only for save/context classes whose correct resume actually depends on them. No other adjunct artifact is part of this conformance set unless a later normative amendment to this document explicitly names it. This rule defines the overall campaign resume outcome; it does not transfer ownership of lower-boundary formats, validation policy, or semantic interpretation to campaign gameplay. Each subsystem boundary remains the owner of its own persisted-data validation and restoration logic; this rule only requires that a restore failure in any required dependency produces a safe campaign-load failure rather than a partial-application state.

**Cross-boundary safe-failure verification evidence:** For cross-boundary restore failures, acceptable verifier evidence that the safe-failure guarantees hold is as follows:

This end-to-end conformance obligation is judged solely by campaign-boundary postconditions and persistence-boundary observations after the failed load attempt. It does not impose requirements on how lower layers stage, validate, or roll back internally, so long as no externally observable runtime effect or persisted mutation from the rejected save survives the failure.


- For failed in-session load attempts, acceptable verifier evidence additionally includes that the post-failure recoverable campaign mode, navigation/location identity, fleet roster, campaign date/tick state, and next main-loop dispatch behavior remain consistent with the pre-load running session rather than with the rejected save.

- After rejection, either control is back at the start/load flow with no resumed campaign state from the rejected save, or the pre-load running campaign session remains active with no state from the rejected save applied.
- No campaign save slot, primary persisted save artifact, or artifact from the closed campaign resume adjunct artifact set (battle-group state files and per-system state files only) touched by the failed resume operation has been mutated by the rejected load attempt.
- Any canonical export, save-summary observation, or adjunct persisted-artifact observation performed after the failure reflects either the pre-load session state (if a campaign was running) or no resumed state from the rejected save (if the load was initiated from the start flow).

This clause defines the verification evidence for the overall safe-failure outcome at the campaign boundary; it does not prescribe how lower boundaries implement their own rollback or staging.

#### 9.4.1 Semantic validity of restored scheduled-event state

Load shall fail — and the campaign shall not resume — if restored scheduled-event state matches any of the following concrete rejection cases:

1. **Unknown event selectors:** The restored schedule contains an event selector that is not one of the event selectors defined in the campaign event catalog (§8.6).
2. **Structurally invalid event metadata:** The restored schedule contains an event entry whose metadata encoding is structurally impossible at the persistence boundary (e.g., malformed date encoding, out-of-range fields, or metadata fields that cannot be interpreted as a valid scheduled-event entry).

These are the required rejection cases and define the conformance boundary for mandatory scheduled-event rejection. Campaign gameplay owns semantic interpretation of restored scheduled events and may perform broader semantic validation beyond this set, but conformance requires only these concrete persistence-boundary rejection cases. Valid legacy compatibility is scoped to the defined valid legacy save set (see `requirements.md`); event-resume correctness is scoped to same-subsystem round-trip saves and valid legacy saves, not to arbitrary structurally parseable files that may have been externally modified.

When a load is rejected under any of the above mandatory rejection cases, the safe-failure guarantees defined in §9.4.0b shall hold.

> **Non-normative note:** Broader semantic consistency checks — including event-type-specific metadata validation that goes beyond persistence-boundary parsing — may be valuable for robustness but are implementation guidance, not acceptance criteria. The two cases above define the minimum rejection surface.

### 9.5 Save/load during sub-activities

Save and load may be initiated from within sub-activities (e.g., from the hyperspace menu). When a load occurs during a sub-activity:

- the sub-activity shall exit cleanly,
- the main campaign loop shall detect that a load has occurred, and
- the loop shall process the loaded state using the deferred-transition mechanism (§5.3) or equivalent.

### 9.6 State-file dependencies

Campaign save/load depends on state-file helpers for per-system scan data and per-system battle-group persistence. The subsystem defines the semantic content and sequencing of save/load operations. The backing implementations are integration dependencies outside this subsystem's boundary: per-system scan-data persistence semantics are owned by the planet-solarsys subsystem (`planet-solarsys/specification.md`); file/stream I/O mechanics are owned by the file-I/O subsystem (`file-io/specification.md`).

**Normative adjunct contract for battle-group state files:** Battle-group state-file format and access mechanics are not currently documented as a standalone subsystem specification in this 13-subsystem documentation set. This is an intentionally external boundary: the battle-group persistence owner sits outside the current 13-subsystem review set. **This contract is therefore the authoritative and self-contained campaign-side boundary specification for verifier pass/fail purposes.** A verifier evaluating adjunct-dependent campaign claims shall use only the criteria below to determine whether the battle-group adjunct obligation is satisfied, without requiring a counterpart owner specification:

1. **Validity:** A battle-group state file is **valid** if the battle-group restore path can read it and produce a structurally complete set of NPC group entries for the star system it covers, with each entry containing at least: NPC group identity, ship count, and group disposition sufficient for campaign-boundary encounter handoff.
2. **Invalidity:** A battle-group state file is **invalid** if it is missing, unreadable, truncated, or structurally malformed such that the restore path cannot extract the required NPC group composition.
3. **"Owning-boundary-rejected":** The restore path encountered a structural or integrity error that prevents it from producing a usable NPC group roster. Campaign treats this as a mandatory load-rejection condition per §9.4.0b.
4. **Campaign-relied facts:** The campaign layer relies on restored battle-group data only to the extent required by the covered claim/context: NPC group composition and presence at the campaign boundary for encounter-adjacent and post-encounter claims. Campaign does not interpret internal battle-group format details; it consumes the restored roster as a boundary-observable fact.
5. **Verification sufficiency:** A verifier determining pass/fail for an adjunct-dependent campaign claim may use the above criteria to classify the battle-group adjunct outcome. If the restore path produces a usable NPC group roster, the adjunct is satisfied for campaign's purposes. If it fails, the adjunct is rejected and campaign's mandatory rejection rules apply.

The broader behavioral contract for battle-group persistence — including internal format, versioning, and detailed semantic validation — will be documented as part of a dedicated subsystem specification when that port is undertaken. **Until that specification exists, this normative adjunct contract is the sole authoritative verifier-facing boundary for battle-group adjunct pass/fail within the 13-subsystem documentation set.** It is scoped to campaign's verification needs and does not define the full battle-group persistence contract. No verifier shall require a counterpart owner document to evaluate adjunct-dependent campaign claims; the five criteria above are sufficient. Campaign's mandatory rejection rules (§9.4.0b) apply regardless of which subsystem owns the backing implementation.

### 9.7 Authoritative legacy-save compatibility table — in-scope and out-of-scope campaign save contexts

The following table is the authoritative covered-context reference for this documentation set's legacy-to-end-state compatibility obligation. In this specification, §10.1 defines the behavioral/compatibility meaning of that legacy-to-end-state contract and its persistence-boundary/export implications; the corresponding verifier-facing pass/fail load obligation is stated in `requirements.md` under Save/load round-trip fidelity. Coverage here means one of two things: either (a) the current-state evidence establishes the context strongly enough to ground a compatibility obligation directly, or (b) this specification intentionally adopts a stricter end-state compatibility policy for the campaign boundary where the evidence establishes the seam and the target contract closes it more sharply. Unless a section explicitly labels itself as an intentional end-state compatibility policy or equivalent stronger normalization rule, covered-context obligations should be read as evidence-derived compatibility obligations from the legacy baseline seam described in `initialstate.md`. This table defines the required end-state contract either way.

| Context | Coverage status | Notes |
|---|---|---|
| Hyperspace navigation | **Covered** | Normal hyperspace position, velocity, and navigation state. |
| Quasispace navigation | **Covered** | Treated as a hyperspace variant; save summary remaps to hyperspace coordinates. |
| Interplanetary / solar-system entry | **Covered** | Restored with exploration entry targeting the saved destination system and campaign-boundary interplanetary entry observables matching the saved context (§7.2.1, §9.4). |
| Starbase visit | **Covered** | Restored to player-visible starbase progression point as defined by the closed mandatory-element progression-point definition and the mandatory-next-action rule (§6.6). |
| Campaign-owned encounter-entry handoff state | **Covered** | The normalized campaign-owned transition point immediately before campaign gameplay yields to encounter/communication ownership, where correct resume is still fully determined by campaign-owned handoff inputs: encounter identity, NPC ship queue, fleet composition, and next handoff target. Does not extend into dialogue-tree or battle-simulation internals that execute after handoff. |
| Post-encounter campaign consequence state | **Covered** | The normalized campaign-owned transition point immediately after encounter resolution and after campaign-owned consequence application is complete, but before any further resumed navigation/exploration progression changes the recoverable campaign-boundary meaning. At this point salvage/progression-flag updates are applied, affected queues are cleaned up, and the next recoverable campaign mode is stable. Transitional mid-application consequence states before that point are not covered. |
| Final-battle save contexts | **Covered** | Covered at the campaign-owned final-battle save boundary only; no broader battle-runtime-local checkpointing is implied. |
| Sub-activity-initiated campaign loads (hyperspace menu) | **Covered** | Covered only as campaign loads whose compatibility meaning is that of the loaded covered context; no separate hidden sub-activity-local presentation state is required. |
| Homeworld encounter screen saves | **Covered** | Covered only when normalized to the campaign-owned encounter-entry handoff boundary; dialogue-screen-local presentation and battle-local setup remain excluded. |


**Encounter-adjacent coverage rule:** An encounter-adjacent save is covered if and only if the chosen inspection surface positively exposes the normalized campaign-owned handoff bundle for that save: `resume_context.campaign_mode = encounter`; a non-null canonical encounter identity; campaign-owned NPC and escort queue facts sufficient to identify the handoff payload; and a campaign-owned classification that designates the resume target as pre-communication and pre-battle handoff state. If correct resume would require any dialogue-tree-local branch position, comm-screen-local presentation state, battle-runtime-local setup, or other post-handoff local state, the save is outside this contract. Homeworld encounter screen saves are covered only when save semantics normalize them back to that same campaign-owned handoff bundle.


Contexts not explicitly listed in either table above are outside this document's legacy-save compatibility obligation. This specification does not assume coverage for unlisted contexts.

## 10. Compatibility and non-goals

### 10.1 Compatibility targets

The end-state subsystem shall preserve the observable compatibility surface of the legacy baseline C implementation with respect to the behaviors and acceptance criteria explicitly defined in this specification and `requirements.md`. Specifically, the compatibility contract covers:

- new-game and load-game entry branching behavior,
- top-level campaign loop activity dispatch order and semantics (including the encounter/starbase dispatch model described in §5.1),
- hyperspace/interplanetary/encounter/starbase transition behavior,
- encounter handoff, battle segue, and post-encounter consequence processing,
- starbase visit flow including special-case story sequences and starbase save/load resume,
- campaign event registration, handler effects, and story-flag updates for the complete event catalog defined in §8.6,
- clock rate policy and day-advancement requests,
- save/load round-trip fidelity, and
- save summary derivation.

**Save compatibility contract:** The following save interoperability requirements apply:

- **Legacy-to-end-state (mandatory):** A valid legacy campaign save produced by the legacy baseline implementation, for campaign save contexts listed as covered in §9.7, shall be loadable by the end-state implementation. Semantic equivalence of restored state satisfies this requirement; byte-for-byte identity of persisted data is not required.
- **End-state round-trip (mandatory):** A campaign save produced by the end-state implementation in a campaign save context covered by §9.7 shall be loadable by the same end-state implementation, and the resumed campaign shall proceed with behavior equivalent to the state at save time. This same-subsystem round-trip obligation is limited to the campaign-owned covered contexts in §9.7 unless a later normative section explicitly extends it. For starbase-context saves, "equivalent to save time" is constrained by the closed mandatory-element progression-point definition and the mandatory-next-action rule in `requirements.md` and does not implicitly include unlisted starbase-local UI or transient state. For non-starbase contexts, equivalence is scoped to the campaign-boundary observables listed in §9.4.0a.
- **End-state-to-legacy (not required):** Loading an end-state-produced save in the legacy implementation is not part of the acceptance contract.

The campaign event catalog selector vocabulary is a bidirectional persistence-boundary obligation as defined in §8.6: the canonical persistence-boundary scheduled-event view in end-state-produced saves shall use the catalog vocabulary, and the end-state shall accept the catalog vocabulary on load from both legacy and end-state-produced saves. The broader save-file layout and encoding are not required to match the legacy format.
**Verifier reporting rule:** For each evaluated save/context and claim/context family, verifier output shall state at minimum: (a) the claim/context family evaluated; (b) the chosen inspection surface; (c) the claim-local result; (d) the overall covered-context result; and (e) whether any adjunct dependency changed the overall covered-context result. A claim-local pass shall not be reported as an overall covered-context pass unless the overall covered-context result is also pass.



**Mixed-sufficiency closure rule:** Inspection-surface sufficiency is determined for the entire comparison object of one covered claim/context at a time, not field-by-field within that claim/context. If the raw save artifact fails to expose any canonical fact required for that claim/context's comparison object, raw-save inspection is insufficient for that claim/context and the mandatory alternative named in the inspection-surface decision table applies to the whole claim/context. Verifiers shall not combine raw-save facts for one subset of a claim/context's comparison object with export facts for the remainder of that same claim/context. If both raw-save inspection and canonical export are available for the same covered claim/context and disagree on an overlapping fact, the chosen verifier-facing surface for that claim/context governs comparison and the disagreement is an inspection-surface conformance failure for the non-chosen overlapping representation; it does not authorize mixed-surface reconciliation.
**Mixed-claim verification-session rule:** Inspection-surface choice is made per covered claim/context family, not once for an entire save review artifact. A verifier may therefore use raw-save inspection for one claim/context family and export-based inspection for another claim/context family drawn from the same save, provided each family independently satisfies the sufficiency rule above. Within any one claim/context family, the chosen surface remains exclusive and mixed-surface reconciliation is forbidden. Player-visible claims remain separately assessable and do not force export for canonical persistence-boundary claim families unless a local rule explicitly says so.



**Implementation-level inspection-surface obligation:** A release is conforming with respect to inspection-surface availability if, for every covered claim/context family, it provides at least one sufficient verifier-facing inspection surface under the rules of this section. Raw-save inspection alone is sufficient for a covered claim/context family only when the implementation's declared raw representation satisfies the sufficiency rule for that family. Otherwise, the implementation shall provide the Campaign Canonical Export Document for that family. When export is provided or relied on for any covered family, it shall satisfy the full §10.1 export-surface contract for the saves and claim/context families it is used to cover.


**Inspection-surface decision procedure:** For each covered verification claim and covered save context, conformance may rely on direct raw-save inspection only if the primary persisted save artifact already exposes every canonical fact required for that specific claim/context in documented, machine-readable, verifier-stable form. Otherwise, the Campaign Canonical Export Document is mandatory for that claim/context. A single implementation may mix these two inspection modes across different covered claims or contexts, but for any one claim/context the verifier shall use exactly one chosen inspection surface for comparison rather than blending partial facts from both. Raw-save sufficiency is determined per implementation release (or save-format/schema version, if versioned separately) and per covered claim/context. To claim raw-save sufficiency for a given claim/context, the implementation shall publish a verifier-facing field map or schema description that identifies every raw persisted field or structure relied on for that claim/context and shall commit that those fields remain documented and verifier-stable for the release/version being claimed. Raw-save sufficiency may rely on a documented deterministic mapping from raw persisted identifiers or numeric encodings to the canonical comparison tokens defined in this specification; the raw artifact need not literally store the canonical strings, provided the mapping is complete, verifier-stable, and part of the authoritative declaration for the claimed release/version.


**Inspection-surface decision table:** The following table is a verifier-facing reference summary for the principal covered claim/context families. The controlling rule is the inspection-surface sufficiency rule above together with any explicit claim-family exception stated in this section.


| Claim/context family | Allowed verifier-facing surface(s) | Mandatory surface when raw-save facts are insufficient | Mixed evidence allowed? | Notes |
|---|---|---|---|---|
| Non-starbase save/load equivalence | Direct raw-save inspection or Campaign Canonical Export Document | Campaign Canonical Export Document | No | Use only one chosen surface for the claim/context. |
| Save-summary normalization | Direct raw-save inspection or Campaign Canonical Export Document | Campaign Canonical Export Document | No | Comparison object is only the normalized save-summary fields required by §9.2. |
| Scheduled-event persistence-boundary meaning | Direct raw-save inspection or Campaign Canonical Export Document | Campaign Canonical Export Document | No | Canonical meaning is wholly primary-artifact-resident for covered contexts. |
| Encounter-adjacent campaign-owned handoff claims | Direct raw-save inspection or Campaign Canonical Export Document | Campaign Canonical Export Document | No | Comparison object is limited to campaign-owned handoff indicators and payload. |
| End-state-produced starbase save/load claims | Direct raw-save inspection or Campaign Canonical Export Document | Campaign Canonical Export Document | No | Apply the closed starbase comparison object from `requirements.md`. |
| Covered valid legacy starbase save/load claims | Direct raw-save inspection only when the full closed progression-point facts are directly exposed; otherwise baseline-versus-end-state controlled observation at the starbase post-load conformance observation point | Observation-based controlled comparison | No | This is the only sanctioned exception to the raw-vs-export two-path model. |
| Claim families outside the covered set | Implementation-defined / outside mandatory conformance surface | None | N/A | Successful export, if any, is diagnostic-only unless another section explicitly says otherwise. |

**Covered-claim canonical-fact map:** The minimum canonical facts that must be available on the chosen inspection surface for the principal covered claim/context families are:
- **Non-starbase save/load equivalence claims:** the recoverable campaign mode, the closed non-starbase observable set defined in `requirements.md` (fleet roster, campaign progression flags, clock/date state, scheduled-event resume facts, and any active encounter presence/identity where applicable), plus the interplanetary entry observables when the context is an interplanetary/solar-system entry claim.
- **Starbase save/load claims:** the recoverable campaign mode together with the closed starbase progression-point facts and mandatory-next-action outcome defined in `requirements.md`. Starbase verification follows the explicit verifier-facing decision procedure in `requirements.md`: for covered valid legacy saves, use direct raw-save facts only if they expose the closed progression-point facts needed for comparison; otherwise the required conformance path is baseline-versus-end-state controlled observation at the starbase post-load conformance observation point. For end-state-produced starbase saves, use the chosen inspection surface under the general raw-save-versus-export decision procedure.
- **Encounter-adjacent claims:** `resume_context.campaign_mode`, `encounter_state.encounter_active`, `encounter_state.encounter_identity`, and queue facts sufficient to confirm the campaign-owned handoff payload (`npc_queue`, `escort_queue`, and the next handoff target where represented on the chosen surface).

**Export-contract minimization rule:** The exact section shape, field names, classification fields, and machine-readable error contract in §10.1 are required only for the chosen or mandatory verifier-facing inspection representation. They do not expand the gameplay compatibility surface, they do not create additional covered claim families beyond those already defined elsewhere in this document and `requirements.md`, and they are not independently conformance-critical for implementations or claim/context families that satisfy verification entirely through the raw-save inspection path. Their sole purpose is to stabilize external comparison when export-based inspection is actually used.

- **Scheduled-event persistence-boundary claims:** the canonical scheduled-event facts defined in §8.6 — selector identifier, normalized due date, and recurrence kind; supplementary recurrence parameters only where a later schema version or explicit clause requires them.
- **Save-summary normalization claims:** the normalized `save_summary` fields required by §9.2 for the covered context being verified.

**Optional diagnostic-export non-trigger rule:** An implementation that satisfies raw-save sufficiency for all covered claim/context families is conforming without any export entry point. In that case, any optional diagnostic export that is not declared or relied on as the verifier-facing inspection surface for a covered claim/context family is outside the §10.1 exact-schema contract and does not by itself trigger the canonical export obligations in this section.


If the raw save artifact does not directly expose all required facts for one of the above covered claim/context families, the Campaign Canonical Export Document is mandatory for that claim/context.

**§10.1 interpretation rule:** The exact field names, section shapes, always-present-section policy, full-table `faction_state` requirement, and other explicit schema rules in this section are verification-policy obligations for the inspection representation only. They do not impose a required runtime structure, in-memory model, or raw-save layout unless a normative clause explicitly identifies a persistence-boundary token or meaning as canonical beyond the inspection representation.


**Persistence-boundary inspection surface:** Conformance shall satisfy one of the following two inspection-surface forms for verifier access to canonical persistence-boundary meaning: (1) the persisted save artifact itself directly exposes the required canonical boundary facts in a documented, machine-readable, verifier-stable form; or (2) the implementation provides the Campaign Canonical Export Document entry point defined in this section, and that entry point deterministically derives the same canonical boundary facts from one persisted save artifact with no dependency on runtime-only state. If both a directly inspectable raw save representation and the Campaign Canonical Export Document are available, the Campaign Canonical Export Document is authoritative only for export-based verification; disagreement between the two surfaces is a conformance failure in the inspection representation, not a rule that changes the underlying gameplay/save-load compatibility contract. If neither form is available for the information required by a covered verification claim, that is a conformance failure. Legacy-format saves satisfy this rule by virtue of their known encoding for the covered contexts defined in §9.7.

**[Verifier-surface normalization] Canonical export surface designation:** When an implementation uses an export-based inspection surface rather than relying solely on a directly inspectable raw save representation, the designated inspection representation for campaign-save conformance verification is the **Campaign Canonical Export Document** — the single machine-readable canonical export document defined by the structure and schema obligations in this section. Only an export entry point that the implementation declares or relies on as the verifier-facing inspection surface for a covered claim/context is in scope for the §10.1 conformance obligations. Such an in-scope export entry point is conforming only insofar as it deterministically produces that document from one persisted save artifact, with no dependency on runtime-only state. Auxiliary diagnostic or convenience export tooling that is not declared or relied on for covered conformance claims is outside this contract. When used to satisfy the persistence-boundary inspection-surface rule, the Campaign Canonical Export Document is a first-class required conformance artifact for export-dependent verification defined in this specification and `requirements.md`, with the exact comparison rules in this section applying to that artifact only. It is not an additional gameplay-compatibility deliverable when the raw save artifact itself already satisfies the persistence-boundary inspection-surface rule above.
**Export obligation summary:** Export is mandatory only for covered claim/context families whose canonical facts are not sufficiently available on the declared raw-save inspection surface. When export is mandatory or chosen for a covered claim/context family, the exact-schema, shape, classification, and error-contract rules in §10.1 apply to that export surface for the saves and claims it is used to cover. When raw-save sufficiency is declared and satisfied for all covered claim/context families in a release, no export surface is required for conformance, and any undeclared diagnostic export remains outside this contract.



**Canonical export document structure:** The Campaign Canonical Export Document shall be a single machine-readable document derived solely from one persisted save artifact.
By design, this canonical export document is derived from the primary persisted save artifact only. The closed campaign resume adjunct artifact set defined in §9.4.0b is not folded into that single export document. When a covered conformance claim depends on those adjunct artifacts, they are verified separately through the adjunct-artifact observation rules in `requirements.md` and the failed-load/seam rules in §9.4.0b, not by treating the export document as an aggregate bundle of all resume artifacts.


This section intentionally defines two different layers of obligation for the export surface:

1. **Minimum semantic content requirements** — what verifier-facing canonical information the export must carry.
2. **Exact canonical serialization rules** — the deliberate JSON/schema/field-shape normalization used for stable tool-based comparison when the export surface is used.

The exact serialization rules below are an intentional verification policy for a stable inspection representation; they are not claims that the underlying save format or runtime subsystem must use the same structure internally.

**[Verifier-surface normalization]** When the Campaign Canonical Export Document is provided, it shall use the project-defined top-level document shape below as the verifier-facing reference schema. The following sections are always present in a successfully produced canonical export document:

| Section | Minimum meaning |
|---|---|
| `schema_version` | Version identifier for the canonical export schema, enabling forward-compatible tooling. |
| `save_summary` | The user-facing save summary derived from the persisted save (activity, location, date). |
| `resume_context` | The recoverable campaign mode and navigation/location state sufficient to determine correct resume dispatch. |
| `clock_state` | Current campaign date and tick state as persisted. |
| `scheduled_events` | The canonical scheduled-event view: an array of entries conforming to the minimum canonical-view schema defined in §8.6. |
| `campaign_flags` | Campaign-owned progression flags relevant to covered save/load contexts and event outcomes, represented as defined below. |
| `faction_state` | Campaign-owned faction strategic state relevant to covered event outcomes or save/load equivalence, represented as defined below. |
| `encounter_state` | Campaign-owned encounter-entry handoff state relevant to covered encounter-adjacent contexts, represented as defined below. |

For sections whose underlying state is not applicable in a given save context, the canonical export shall still include the section and shall represent absent or inapplicable content explicitly (null, empty array, or empty object as appropriate for the field), not by omitting the section. In such contexts, the section's required conformance content is limited to that minimal empty/null shape unless another normative clause for the active claim/context requires more. Optional extension sections beyond the table above are permitted. This always-present section shape applies only when the Campaign Canonical Export Document is used; it does not eliminate the raw-save inspection alternative defined by the persistence-boundary inspection-surface rule above. The surface shall be stable enough to support conformance verification for the end-state target defined by this document.



**Canonical export contract rules:**


**Failure/outcome precedence list:** For any covered claim/context, classify outcomes in the following order: (1) determine whether the chosen inspection surface itself is available and interpretable for that claim/context; if not, the result is an inspection-surface conformance failure for that claim/context; (2) determine whether the chosen surface establishes the full comparison object for that claim/context; if not, the result is an inspection-surface conformance failure for that claim/context; (3) determine whether the comparison object satisfies the applicable exact-token/schema or semantic-equivalence rule; failure here is a claim-level conformance failure of the type defined by that local rule; (4) determine whether any additional adjunct-dependent or covered-context-wide conditions are required for overall resume conformance; a claim-local pass does not override an overall covered-context failure caused by those additional conditions.

**Representative mixed-case outcomes:**
- If export-based inspection is mandatory for a covered claim/context and the export document omits a required section needed to interpret that chosen surface, the result is an inspection-surface conformance failure for that claim/context even if gameplay behavior appears correct.
- If a scheduled-event claim family is wholly primary-artifact-resident and its chosen inspection surface establishes the required canonical facts, that claim-local result may pass even when overall covered-context resume fails because a required adjunct artifact is missing or rejected.
- If raw-save inspection is sufficient for one claim family on a save but insufficient for another, the verifier may use different chosen surfaces for those different claim families, but shall not mix raw and export facts within one claim/context comparison object.


**Overall-result aggregation rule:** For one covered save/context, the overall covered-context result is pass if and only if every evaluated mandatory claim/context family for that covered save/context passes on its chosen inspection surface and every applicable adjunct-dependent or other covered-context-wide obligation also passes. Otherwise, the overall covered-context result is fail, regardless of any subset of claim-local passes.


**Mandatory-success boundary rule:** Only the explicitly enumerated mandatory acceptance classes in this specification create required-success obligations. Any structurally decodable input outside those enumerated classes is outside the mandatory-success set unless another normative clause explicitly places it inside. Such inputs may be accepted or rejected, and any successful export for them is diagnostic-only unless another clause explicitly elevates that class.

**Claim-local versus overall conformance rule:** Claim-local conformance is satisfied when the chosen inspection surface correctly establishes the comparison object for the specific claim family being verified. Overall covered-context resume conformance is satisfied only when all additional obligations for that covered context also hold, including adjunct-artifact conditions where applicable. A claim-local pass therefore does not by itself imply overall covered-context conformance.


  - `date` — the campaign date at save time, in the normalized date representation defined above.

  This schema supports verification of the save-summary compatibility surface — including the activity remapping and location remapping rules defined in §9.2 — without requiring source-internal interpretation of the persisted save data.

- **`resume_context` minimum schema:**
  - `campaign_mode` — the fully recovered observable campaign mode, not merely a primary stored discriminator. The normalized vocabulary for this field is the closed set: `hyperspace_navigation`, `interplanetary`, `encounter`, `starbase_visit`, `last_battle`. For a starbase-context save, `campaign_mode` shall be `starbase_visit`.
  - `starbase_context` — boolean: whether the underlying restored state uses the starbase-context condition that distinguishes starbase within an encounter-family representation. This field is verifier-supporting context for starbase-related obligations; it does not replace `campaign_mode` as the recoverable-mode surface. In the canonical export surface, `campaign_mode = starbase_visit` if and only if `starbase_context = true`. For all non-starbase recovered modes, `starbase_context` shall be `false`. A canonical export that presents conflicting values for these fields is an inspection-surface conformance failure.
  - `navigation_identity` — the navigation or destination identity needed for next dispatch. When `campaign_mode` is `hyperspace_navigation`, this shall be the coordinate representation `{"x": <number>, "y": <number>}`. When `campaign_mode` is `interplanetary`, this shall be the stable string identifier `system:<x>,<y>` using the baseline system coordinates of the destination system as defined by the baseline system-identifier normalization rule above. Null when not applicable to the current mode.
  - `entry_routing_kind` — the normalized campaign-boundary routing kind for the next exploration handoff. When the saved or restored context is an interplanetary/solar-system entry context, this shall be one of the closed set `normal_interplanetary_entry` or `special_case_encounter_routing`, matching the campaign-boundary interplanetary entry observables defined in §7.2.1. Null when not applicable to the current mode.
  - `campaign_transition_marker` — the normalized campaign-boundary marker for first-dispatch transition behavior. This field shall always be represented as a single token string from the closed set `none`, `clear_start_interplanetary`, or `preserve_start_encounter`; it shall not be null and shall not use an object form. `none` is required whenever no special campaign-boundary transition case applies, including non-interplanetary modes and ordinary interplanetary-entry contexts with no additional transition requirement beyond `entry_routing_kind` and destination identity. `clear_start_interplanetary` means the restored campaign-side handoff must clear a pending interplanetary-start flag before the first exploration dispatch. `preserve_start_encounter` means the restored campaign-side handoff must preserve a pending encounter-start routing instead of normal exploration entry. Comparison for this field is exact on the token value.

**Claim-family comparison/evidence matrix:** The following table summarizes the authoritative comparison object and failure mode for the principal covered claim families:
**Summary-table status rule:** The inspection-surface decision table and the claim-family comparison/evidence matrix are reference summaries for verifier use. They summarize and coordinate the controlling local rules already established in this specification; they do not create additional independent claim families, additional comparison-critical fields, or broader export obligations beyond the controlling local sections they reference.



| Claim/context family | Comparison object | Exactness vs equivalence mode | Adjunct artifacts can affect overall outcome? | Claim-irrelevant export defects fatal? |
|---|---|---|---|---|
| Non-starbase save/load equivalence | Closed non-starbase observable set in §9.4.0a | Semantic/observable equivalence within the closed set | Yes, where §9.4.0b marks the covered context adjunct-dependent | No |
| Save-summary normalization | Normalized `save_summary.summary_type`, `location_id`, and required date meaning | Exact for normalized comparison tokens; `location_display` informational unless explicitly elevated | No | No |
| Scheduled-event persistence-boundary meaning | Canonical scheduled-event selector identity, normalized due date, recurrence kind, and any explicitly required checkpoint-bundle facts | Exact for canonical comparison tokens; checkpoint bundles for listed mission families | No for the event claim itself; yes for broader overall resume in adjunct-dependent contexts | No |
| Encounter-adjacent handoff | `resume_context.campaign_mode`, canonical encounter identity, queue payload, and next handoff target where represented | Exact for canonical identifiers; semantic equivalence for broader handoff behavior | Yes, if the covered context is adjunct-dependent under §9.4.0b | No |
| Starbase save/load claims | Closed progression-point elements and mandatory-next-action outcome | Semantic/observable equivalence within the closed starbase contract | No | No |
| Mandatory inspection-surface structure for any claim using export | Required section presence, required discriminator/field presence, and required schema shape needed to interpret the chosen export surface | Exact-schema inspection-surface conformance | N/A | Yes, but only as inspection-surface conformance for the claim/context using that mandatory export surface |



- **`faction_state` minimum schema:**
The `faction_state` section shall always be present. **[Verifier-surface normalization]** For export shape stability, the section is always present; however, normative content requirements are limited to the faction entries and fields needed for the covered claim/context being verified unless an explicit section of this specification requires a broader faction-state comparison object. Implementations may provide the full baseline faction table as the project reference schema, but campaign-gameplay conformance does not require every baseline faction entry to carry comparison-critical content in contexts where no covered obligation depends on that faction state. Each required faction entry shall expose at minimum:


  - `faction_id` — a stable string faction identifier from the closed campaign faction vocabulary used by the baseline campaign (`arilou`, `chmmr`, `druuge`, `ilwrath`, `kohr_ah`, `mycon`, `orphans_pkunk`, `shofixti`, `slylandro`, `spathi`, `supox`, `thraddash`, `umgah`, `urquan`, `utwig`, `yehat`, `zoqfotpik`; additional end-state identifiers are allowed only as optional extensions not required for baseline verification).
  - `strength` — the faction's current fleet strength as a normalized numeric value.
  - `position` — the faction's current strategic position as a coordinate `{"x": <number>, "y": <number>}`, or a stable string destination identifier, as applicable. Null when the faction has no active fleet movement.
  - `alliance` — one of the closed normalized values `allied`, `hostile`, `neutral`, or `dead`.
  - `alive` — boolean: whether the faction is alive (true) or dead/eliminated (false).

- **`encounter_state` minimum schema:**
  When a covered save/load context depends on campaign-owned encounter-entry handoff state, the `encounter_state` section shall expose at minimum:
  - `encounter_active` — boolean: whether a campaign-owned encounter is active at the persistence boundary.
  - `encounter_identity` — when active, the identity of the encounter from the closed baseline encounter vocabulary: `arilou`, `black_urquan`, `chmmr`, `druuge`, `human`, `ilwrath`, `mycon`, `orz`, `pkunk`, `shofixti`, `slylandro`, `spathi`, `supox`, `thraddash`, `umgah`, `urquan`, `utwig`, `vux`, `yehat`, `zoqfotpik`, `melnorme`, `talking_pet`, `samatra`, `starbase`, `samatra_homeworld`. Null when no encounter is active.
  - `npc_queue` — an array of NPC ship entries representing the NPC ship queue composition as persisted; each entry shall expose at minimum `race_id` and `ship_type_id` fields sufficient to confirm campaign-owned handoff identity.
  - `escort_queue` — an array of escort ship entries representing the player's fleet roster as persisted; each entry shall expose at minimum `race_id` and `ship_type_id` fields sufficient to confirm campaign-owned handoff identity.
  - `race_id` normalization for queue entries shall use the same closed baseline encounter/race vocabulary used by `encounter_identity` where applicable.
  - `ship_type_id` normalization shall use the baseline species/catalog identifier vocabulary derived from `SPECIES_ID` / ship roster identity, serialized as stable lowercase-with-underscores strings (for example `androsynth`, `arilou`, `chenjesu`, `chmmr`, `earthling`, `ilwrath`, `mycon`, `orz`, `pkunk`, `shofixti`, `slylandro`, `spathi`, `supox`, `thraddash`, `umgah`, `urquan`, `utwig`, `vux`, `yehat`, `zfp`). Additional end-state identifiers are allowed only as optional extensions not required for baseline verification.
  Queue entry ordering is not significant unless a normative requirement for a covered context explicitly makes ordering part of the campaign-owned handoff contract.

**Claim/context comparison rule for always-present sections:** When export-based inspection is used, sections required to be always present (`save_summary`, `resume_context`, `clock_state`, `scheduled_events`, `campaign_flags`, `faction_state`, `encounter_state`) are always mandatory as document-shape obligations. However, field-by-field comparison is normative only for the fields relevant to the covered claim/context being verified, unless this specification explicitly makes a whole section or full table a comparison object for that claim/context. In particular, the `faction_state` full-table presence rule is universal for export shape, but baseline comparison of faction entries is required only to the extent that the covered claim/context or an explicit section of this specification makes those faction-state facts part of the conformance object. The same rule applies to `encounter_state`: required shape is always present, but comparison is normative only where encounter-adjacent handoff facts are part of the covered claim/context. Exact-token mismatches confined to always-present but claim-irrelevant fields or entries are inspection-representation defects only; they are not gameplay-compatibility failures and do not fail conformance for a claim/context that does not make those fields or entries part of its comparison object. By contrast, document-shape failures on a mandatory inspection surface — such as a missing required section, malformed required section type, or absence of a required discriminator/field needed to interpret the chosen surface at all — fail inspection-surface conformance for that claim/context even if the missing/malformed area would otherwise have been claim-irrelevant content.



**Machine-readable export-error contract:** When canonical export fails, the output shall still be JSON and shall be unambiguously distinguishable from a successful canonical export document. At minimum, an error result shall contain: (a) a top-level discriminator field `result` with the exact value `error`; (b) a machine-readable `error_code` string from the implementation's documented export-error vocabulary; and (c) an `error_message` string intended for human diagnosis. An export failure shall not emit the normal success document shape defined above, wrapped or partial, and shall not omit the `result` discriminator.
**Malformed-save export behavior:** When the Campaign Canonical Export Document is the chosen or mandatory inspection surface for the input class and claim/context being evaluated, and the persisted save artifact cannot be decoded sufficiently to produce the required canonical export document, that export surface shall fail with a machine-readable error result. This includes any save whose scheduled-event state contains unknown event selectors or structurally invalid event metadata of the kind defined in §9.4.1, because such an artifact cannot be decoded into the required canonical scheduled-event facts on that export surface. The export surface shall not emit a partial success document that could be mistaken for valid exported save state. This clause does not by itself require a matching export path for implementations or claim/context families that satisfy conformance entirely through raw-save inspection and do not use export for the input being evaluated.

**Successful-export input classification:** Every successfully produced canonical export document shall include a top-level field `conformance_input_class` with one of the following exact values: `covered_mandatory` or `diagnostic_only`. `covered_mandatory` means the input falls within a save/input class for which this document requires successful export on the conformance surface. `diagnostic_only` means the export was produced for a structurally decodable input outside the mandatory acceptance/export-success sets. A successful canonical export with `diagnostic_only` classification is not proof that the save is covered by this document's interoperability obligations.


**Inspection surface acceptance properties:** If the raw save encoding is insufficient to expose all canonical persistence-boundary facts required for covered verification claims in a documented, machine-readable, verifier-stable form, the implementation shall provide the Campaign Canonical Export Document interface defined in this section. In that case, the export interface is mandatory for conformance and shall satisfy the following minimum acceptance properties:

- it shall be machine-readable (not solely human-readable narrative output),
- it shall be deterministic: identical persisted input shall produce identical inspection output,
- it shall be complete for every covered context and for every canonical field required by this specification and `requirements.md` (including save summary, canonical scheduled-event view, campaign-owned encounter/NPC state, and other required persisted fields),
- it shall be documented sufficiently to support automated conformance verification without source-internal access, and
- it shall be available in the environment used for conformance verification.

If the raw save encoding is insufficient and this interface is absent or incomplete with respect to any of the above properties, that is a conformance failure.

The definition of "valid legacy campaign save" for purposes of this contract is given in `requirements.md`. Files presented as legacy-format inputs but falling outside the valid legacy campaign save set (as defined in `requirements.md`) are outside the interoperability obligation but, if presented for loading, remain subject to the general load-failure safe-failure contract defined in §9.4.0b.

Semantically equivalent behavior satisfies the compatibility contract unless a specific section explicitly requires exact parity. Behaviors not explicitly specified in the normative sections of this document or `requirements.md` are outside the compatibility contract, even if they are observable in the legacy baseline implementation.

### 10.2 Non-goals

This specification does not require:

- preserving current internal C struct layouts or global variable organization,
- preserving any specific deferred-transition mechanism (e.g., the current `NextActivity` fake-load pattern) as opposed to a semantically equivalent transition mechanism that preserves the observable properties in §5.3,
- preserving current source-file decomposition,
- preserving current rendering, fade, or music management that happens to be co-located with campaign orchestration code, or
- absorbing solar-system exploration, dialogue, battle simulation, or SuperMelee internals into the same subsystem.

## 11. Open audit-sensitive areas (non-normative)

> **This section is non-normative.** The items below identify areas where the parity-vs-equivalence stance has not yet been resolved. They are recorded here as compatibility-sensitive verification questions for future audit, not as normative requirements. They do not expand the acceptance contract defined by the normative sections above. Until a specific item is resolved and incorporated into a normative section of this specification or `requirements.md`, the compatibility contract for that area is limited to what the normative sections already state.

The following areas should be treated as compatibility-sensitive verification questions, not implementation-plan steps:

- whether exact hyperspace encounter generation parameters (rates, race selection, fleet composition) are externally significant enough to require deterministic parity or only statistical equivalence,
- whether exact clock rate values and tick-per-day calculations must be preserved numerically or only behaviorally (same pacing),
- whether specific save-file binary layout must remain byte-for-byte compatible with legacy saves or only semantically compatible beyond the load interoperability obligation stated in §10.1,
- whether specific post-encounter salvage/reward calculations are externally significant and require exact numeric parity, and
- whether the Ilwrath response battle in the pre-alliance starbase sequence has additional externally visible sequencing constraints beyond what is documented in the current state.
