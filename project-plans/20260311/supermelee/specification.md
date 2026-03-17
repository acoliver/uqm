# SuperMelee Subsystem — Functional & Technical Specification

## 1. Scope

This document specifies the desired end-state behavior of the SuperMelee subsystem responsible for:

- setup/menu orchestration for local SuperMelee sessions,
- fleet and team editing,
- built-in team browsing,
- loading and saving user team files,
- selecting combatants from prepared fleets, and
- handing control into and back out of battle.

This specification does **not** cover:

- per-ship combat mechanics, ship weapons, or ship AI behavior,
- the full network transport/protocol stack for netplay,
- generic battle-engine internals,
- generic graphics, resource, file-I/O, input, audio, or threading subsystem design.

Those are integration boundaries. This subsystem may depend on them, but it does not own their contracts.

## 2. Boundary and ownership model

### 2.1 Subsystem boundary

The SuperMelee subsystem owns the following end-state responsibilities:

1. presenting and driving the SuperMelee setup/menu flow,
2. maintaining editable per-side team state,
3. presenting built-in teams and file-backed saved teams,
4. validating whether a match can start,
5. persisting and restoring local SuperMelee setup state,
6. selecting the next combatant from each prepared fleet (selection **policy** — which fleet entry is chosen next — is SuperMelee-owned; the queue-entry data model, battle-ready descriptor loading, and spawn of the already-selected entry are owned by the ships subsystem per `ships/specification.md` §2.1 / §7.2), and
7. handing control to the battle subsystem with the selected setup.

### 2.2 Out-of-scope but required integration boundaries

The subsystem must integrate correctly with, but does not define, the following:

- **Ship subsystem boundary:** SuperMelee stores and manipulates ship identities, fleet composition, and selection order, but it does not define combat behavior for those ships.
- **Battle subsystem boundary:** SuperMelee initiates battle and receives control back when battle ends or aborts, but it does not define battle simulation semantics.
- **Netplay subsystem boundary:** when SuperMelee netplay support is present, this subsystem must expose setup-time and selection-time state changes required by that integration boundary, but it does not define transport, protocol framing, remote discovery, or connection-state-machine behavior.
- **Persistence/file-I/O boundary:** SuperMelee defines the semantic content of local setup persistence and team-file persistence, while lower layers provide file handles, stream semantics, and path resolution.

## 3. End-state subsystem model

### 3.1 Runtime state

The subsystem shall maintain runtime state sufficient to represent:

- current setup/menu mode,
- currently focused side and cursor/selection location,
- editable team state for each side,
- team-browser state,
- currently highlighted ship or selection candidate,
- local-only random state needed for setup behavior,
- current menu music/stateful audiovisual assets needed by the setup UI,
- integration state required to start battle, and
- when enabled, setup-time synchronization state needed to cooperate with the netplay layer.

The exact internal data layout is not prescribed, but the subsystem shall preserve the externally visible behavior described below.

### 3.2 Team model

For each side, the subsystem shall maintain:

- a team name,
- a fleet of ship slots up to the configured SuperMelee fleet size,
- an authoritative empty-slot representation,
- an authoritative ship-identifier representation for occupied slots, and
- a fleet-value calculation consistent with the current ship roster/value table.

The subsystem may cache derived fleet values, but externally observable behavior shall remain equivalent to recomputation from the currently selected ships.

## 4. Local setup/menu behavior

### 4.1 Entry and initialization

Entering SuperMelee shall:

- allocate or initialize a fresh runtime state,
- prepare the setup UI assets and any subsystem-local audiovisual state required by the menu,
- initialize the team browser and built-in team catalog,
- initialize local random state used by setup-time behavior,
- load persisted SuperMelee configuration if present and valid, or else fall back to built-in default teams according to the fallback contract below, and
- enter the setup/menu input loop.

### 4.2 Default fallback behavior

If persisted SuperMelee setup cannot be loaded or is invalid, the subsystem shall still enter a usable setup state with:

- valid control modes for both sides,
- valid team data for both sides, and
- a path for the user to start local setup interaction without external recovery steps.

That fallback behavior shall remain tied to the product's built-in team offering. In other words, fallback initialization shall source its default teams from the built-in team catalog or from a clearly defined built-in default subset of that same offering; it shall not silently rely on unrelated one-off defaults disconnected from the built-in team mechanism.

This specification does not require exact preservation of the current built-in team names or compositions unless a later compatibility audit promotes them to compatibility-significant content.

### 4.3 Menu actions

The setup/menu flow shall support, at minimum:

- choosing local control modes relevant to SuperMelee,
- editing fleet composition,
- editing team names,
- loading a team into the active side,
- saving the active side's team,
- initiating match start when both sides are valid, and
- cancelling out of transient subviews such as ship-pick or load/save views without corrupting persistent setup state.

### 4.4 Validity gate for match start

The subsystem shall not allow battle start when either side lacks a playable fleet.

A fleet is playable if it contains at least one valid ship selection according to the current roster/rules. The exact internal representation of emptiness is not prescribed, but the start gate shall prevent zero-value/empty fleets from proceeding to battle.

## 5. Team editing behavior

### 5.1 Team mutation

The subsystem shall support the following team edits for each side:

- set or replace a ship in a fleet slot,
- clear a fleet slot,
- replace the full fleet contents from a source team,
- change the team name, and
- replace the full team state from another team object or equivalent semantic source.

### 5.2 Fleet value behavior

The subsystem shall present or otherwise make available a fleet value that reflects the current team composition.

When ships are added, removed, or replaced, the reported fleet value shall remain consistent with the current fleet contents.

### 5.3 Team name behavior

The subsystem shall preserve a bounded team name. Inputs longer than the supported limit shall be truncated, rejected, or normalized consistently, but the subsystem shall not allow unterminated or unbounded team-name state to escape into persistence or UI rendering.

## 6. Built-in team catalog and file-backed browsing

### 6.1 Unified team-browsing surface

The subsystem shall present a team-browsing flow that can expose both:

- built-in teams, and
- file-backed user teams.

The exact rendering or list-window implementation is not prescribed.

### 6.2 Built-in team catalog

The subsystem shall ship with a built-in catalog of valid teams.

That catalog is not optional flavor content. It shall be available at startup, shall be loadable through the team-browsing flow, and shall be sufficient to satisfy the fallback-initialization contract in §4.2.

This specification does not require preserving exact current built-in team names or compositions unless a later compatibility audit explicitly promotes them to compatibility-significant content. It does require that the subsystem preserve the existence of a browseable built-in team catalog and the use of that catalog as the source of built-in fallback/default teams.

### 6.3 File-backed team list

The subsystem shall enumerate saved team files from the configured SuperMelee teams location and expose them through the team-browsing flow.

Malformed, unreadable, or semantically invalid team files shall not corrupt the setup state. They may be skipped, surfaced as invalid entries, or produce user-visible failure feedback, but they shall fail cleanly.

## 7. Team persistence

### 7.1 Legacy `.mle` load interoperability

The subsystem shall support saving a team for later reuse and loading previously saved teams.

Valid legacy `.mle` team files produced by the current subsystem are a mandatory load-interoperability target. At minimum, loading such a file shall preserve the documented semantic payload of the current format:

- fleet slot contents,
- team name, and
- consistent normalization or handling of invalid ship identifiers according to the compatibility contract.

The exact save-file encoding and byte-for-byte write format remain audit-sensitive. Unless a later compatibility audit requires stronger obligations, the end-state contract requires firm load interoperability with valid legacy `.mle` files and requires newly written save files to be reloadable by the subsystem under the same semantic contract.

### 7.2 Save failure behavior

If saving a team fails after a partially written file has been created, the subsystem shall not leave an apparently successful but corrupted saved-team artifact behind.

A failed save may leave no file or may leave a clearly invalid/replaced artifact only if the surrounding persistence layer makes that unavoidable, but the subsystem shall not report success unless the saved team is actually durable and reloadable according to the chosen persistence contract.

### 7.3 Setup-state persistence

The subsystem shall persist enough setup state to restore a subsequent SuperMelee session to a usable prior setup.

At minimum, persisted setup shall preserve:

- control-mode state relevant to local SuperMelee setup, and
- both sides' team state.

### 7.4 Netplay-mode persistence boundary

If the subsystem supports network-controlled setup states, it shall not treat transient network connection mode as mandatory persisted startup state unless a compatibility audit proves that legacy behavior requires it.

A valid end state may intentionally sanitize or downgrade transient network-only setup state when restoring persisted local setup.

## 8. Combatant selection and battle handoff

### 8.1 Fleet-edit ship picker

When editing fleets, the subsystem shall provide a ship-selection interaction that allows the user to choose a ship identity to place into the active fleet.

The exact presentation, animation, or navigation layout is not prescribed, but the interaction shall support:

- navigation among available ships,
- confirm/cancel behavior, and
- updating the active team only on confirm.

### 8.2 Battle-facing combatant selection contract

The subsystem shall provide a battle-facing selection contract by which the battle/input layer can request:

- the initial combatants for a match, and
- the next combatant for a given side after prior combatant loss.

That contract shall remain compatible with the consuming battle/input boundary. The subsystem shall hand off fully selected combatants in a form immediately usable by that boundary; it shall not weaken the handoff to bare ship identities, fleet-slot indices, or other abstract descriptors unless the surrounding battle-facing contract is explicitly redesigned in lockstep.

### 8.3 Handoff into battle

Starting a match shall:

- finish any setup-time audiovisual transition required before battle,
- prepare current player input/control configuration for battle,
- prepare the active fleet/combatant selection state and other SuperMelee-owned handoff inputs required by battle startup,
- invoke the battle subsystem, and
- restore SuperMelee menu ownership after battle returns unless a higher-level abort/exit condition takes precedence.

SuperMelee owns the correctness of the handoff inputs it supplies at this boundary. Battle simulation itself remains outside SuperMelee ownership.

### 8.4 Return from battle

When battle ends and control returns to SuperMelee, the subsystem shall restore a valid post-battle SuperMelee state consistent with continued setup/menu interaction or subsequent battle entry as allowed by current activity state.

## 9. Netplay integration obligations

### 9.1 Scope of this section

This section defines only SuperMelee-owned obligations at the setup/selection boundary when SuperMelee netplay support exists. It does not define the complete netplay protocol.

### 9.2 Setup-time synchronization boundary

When the product supports SuperMelee netplay and a session enables network-controlled SuperMelee, the subsystem shall expose the setup-time synchronization surface required by that mode, including:

- ship-slot changes,
- team-name changes, and
- whole-team bootstrap/synchronization events required by the integrated setup flow.

These are mandatory SuperMelee-owned obligations for supported netplay sessions, not illustrative examples.

### 9.3 Match-start gating under netplay

When the product supports SuperMelee netplay and a session enables network-controlled SuperMelee, the subsystem shall not start the match unless the SuperMelee/netplay boundary has the required connection, readiness, and confirmation preconditions for a valid handoff.

The detailed connection-state machine remains a netplay concern.

### 9.4 Battle-time selection synchronization boundary

When the product supports SuperMelee netplay and a session enables network-controlled SuperMelee, the subsystem shall expose local battle-time combatant-selection outcomes and accept remote selection updates at the SuperMelee/netplay boundary wherever that integrated mode requires them.

The lower-level transport, retransmission, framing, and remote-state machine behavior remain out of scope here.

**Ship-selection commit/validation boundary contract:** The following ownership split applies at the SuperMelee↔netplay boundary for remote ship selections:

- **Transport-level validation** is netplay-owned. SuperMelee does not validate wire format or protocol phase correctness.
- **Fleet/rules semantic validation** is SuperMelee-owned: whether the selected ship identity is valid for the remote fleet at that moment (e.g., ship exists in the remote fleet and has not already been eliminated). The netplay subsystem delivers a decoded selection identity; SuperMelee determines whether it is semantically valid.
- **Commit authority:** SuperMelee is authoritative for committing a remote selection into the battle-facing selection state. A semantically invalid remote selection shall not be committed.
- **Invalid remote selection handling:** If a remote selection fails fleet/rules semantic validation, the integrated system shall treat this as a sync/protocol error handled according to the netplay subsystem's error recovery rules. SuperMelee shall not silently substitute a different ship or proceed with an invalid selection.
- **Post-acceptance:** SuperMelee shall not re-reject a remote selection that it has already accepted and committed.

This boundary contract is mirrored in `netplay/specification.md` §12.4.

## 10. Error handling and robustness

The subsystem shall fail cleanly when it encounters:

- unreadable or malformed saved-team files,
- unreadable or malformed persisted setup state,
- unsupported or invalid ship identifiers in persisted data,
- save failures, and
- netplay-start attempts with unmet setup preconditions.

Failing cleanly means:

- no memory/object corruption in the active setup state,
- no false success report,
- no transition into battle from invalid setup state, and
- preservation of a usable menu/setup flow whenever recovery is possible.

## 11. Compatibility and non-goals

### 11.1 Compatibility targets

The end-state subsystem shall preserve the observable compatibility surface of the current SuperMelee C subsystem with respect to:

- editable two-sided team setup,
- team load/save behavior,
- built-in team catalog support and fallback use,
- battle-facing initial/next-combatant selection,
- battle handoff/return behavior, and
- setup-time and selection-time netplay integration hooks when SuperMelee netplay is supported.

### 11.2 Non-goals

This specification does not require:

- preserving current internal C struct layouts,
- preserving exact current UI drawing implementation details,
- preserving current built-in-team internal allocation strategy,
- preserving current source-file decomposition, or
- absorbing the ships subsystem or full netplay subsystem into the same port unit.

## 12. Open audit-sensitive areas

The following areas should be treated as compatibility-sensitive verification questions, not implementation-plan steps:

- whether exact built-in team names and compositions are compatibility-significant or only product-content data,
- whether current saved-team binary layout must remain byte-for-byte compatible with legacy `.mle` files or only semantically compatible beyond the firm load-interoperability floor stated in §7.1,
- whether any setup-screen UI behaviors such as cursor wrapping, page movement, animation cadence, or sound timing are externally significant enough to require exact parity, and
- which specific setup-time netplay readiness/confirmation behaviors belong to SuperMelee versus the separate netplay subsystem contract.