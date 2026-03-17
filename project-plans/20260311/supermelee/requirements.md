# SuperMelee Subsystem Requirements

## Purpose

This document defines the required externally observable behavior of the SuperMelee subsystem in EARS form. The subsystem covers setup/menu orchestration, fleet/team editing, built-in team browsing, team persistence, battle-time combatant selection, and battle handoff for SuperMelee sessions.

## Scope boundaries

- Per-ship combat behavior is outside this subsystem.
- Full netplay transport/protocol behavior is outside this subsystem.
- Lower-level graphics, input, audio, resource, file-I/O, and threading mechanics are outside this subsystem, though SuperMelee depends on them.

## Entry, initialization, and teardown

- **When** the user enters SuperMelee, **the subsystem shall** initialize a valid SuperMelee runtime state, load required setup/menu assets, initialize team-browsing state and the built-in team catalog, initialize any local-only random state required by setup flow, and enter a usable setup/menu interaction state.
- **When** persisted SuperMelee setup state is present and valid, **the subsystem shall** restore that setup state before interactive setup begins.
- **When** persisted SuperMelee setup state is missing, unreadable, or invalid, **the subsystem shall** fall back to valid startup state using built-in default teams from the product's built-in team offering without requiring external recovery steps.
- **When** the user leaves SuperMelee through the normal setup/menu exit path, **the subsystem shall** persist the configured setup state according to the setup-persistence contract and release SuperMelee-owned runtime/menu resources.

## Team and fleet model

- **Ubiquitous:** The subsystem shall maintain editable team state independently for each side.
- **Ubiquitous:** Each team's state shall include a team name and a fleet of ship slots.
- **Ubiquitous:** The subsystem shall represent empty fleet slots distinctly from occupied fleet slots.
- **When** a ship is assigned to a fleet slot, **the subsystem shall** make that ship available as part of the team's current fleet state.
- **When** a ship is removed from a fleet slot, **the subsystem shall** update the team's state so that slot is treated as empty.
- **When** the full team for a side is replaced from another valid team source, **the subsystem shall** update the destination side's fleet contents, team name, and derived fleet state consistently.
- **Ubiquitous:** The subsystem shall maintain a fleet value or equivalent derived team-strength summary consistent with the team's current ship contents.

## Team-name behavior

- **When** a team name is set or edited, **the subsystem shall** preserve a bounded valid team name for that team.
- **When** a supplied team name exceeds the supported limit or violates accepted representation constraints, **the subsystem shall** truncate, reject, or normalize it consistently without producing invalid persisted or rendered state.

## Setup/menu behavior

- **Ubiquitous:** The setup/menu flow shall support team editing, team loading, team saving, and match-start initiation.
- **When** a transient setup subview such as ship-pick or team-load/save is cancelled, **the subsystem shall** return to a valid setup/menu state without committing an unconfirmed edit.
- **When** the user confirms a fleet-edit ship choice, **the subsystem shall** apply that selection to the active team's state.
- **When** the user cancels a fleet-edit ship choice, **the subsystem shall** leave the affected team state unchanged.

## Match-start validation and battle handoff

- **When** either side has no playable fleet, **the subsystem shall not** start a match.
- **When** both sides have valid playable fleets and all other applicable start preconditions are satisfied, **the subsystem shall** permit battle start.
- **When** a match starts, **the subsystem shall** prepare the battle-facing combatant-selection state and other SuperMelee-owned handoff inputs needed by the battle subsystem, perform any required setup-to-battle transition work, and hand control to the battle subsystem.
- **When** battle returns control to SuperMelee without a higher-level abort or exit taking precedence, **the subsystem shall** restore a valid SuperMelee-owned post-battle state so setup/menu interaction can continue.
- **Ubiquitous:** The subsystem shall preserve the correctness of the handoff inputs and post-battle restoration behavior that it owns at the SuperMelee/battle boundary, while leaving battle simulation itself outside SuperMelee ownership.

## Built-in team browsing

- **Ubiquitous:** The subsystem shall provide a built-in catalog of valid teams.
- **When** persisted SuperMelee setup cannot be loaded, **the subsystem shall** source fallback default teams from that built-in team catalog or from a clearly defined built-in default subset of the same offering.
- **When** the user browses available teams for loading, **the subsystem shall** expose both built-in teams and file-backed saved teams through the team-browsing flow.
- **When** the user selects a valid built-in team for loading, **the subsystem shall** copy that team's semantic contents into the active side's editable team state.

## Saved-team browsing and loading

- **When** saved team files are present in the configured SuperMelee teams location, **the subsystem shall** enumerate them for the load/browse flow.
- **When** the user selects a valid saved team file for loading, **the subsystem shall** load that team's semantic contents into the active side's editable team state.
- **When** the user loads a valid legacy `.mle` team file produced by the current subsystem, **the subsystem shall** preserve at least that file's fleet-slot contents and team name, with invalid ship identifiers handled according to the subsystem's compatibility contract.
- **When** a saved team file is unreadable, malformed, or semantically invalid, **the subsystem shall** fail cleanly without corrupting the active setup state.

## Team persistence

- **When** the user saves a team successfully, **the subsystem shall** persist enough information to restore that team's ship slots and team name later.
- **When** a save attempt fails after partial output has begun, **the subsystem shall not** leave behind an apparently successful corrupted saved-team artifact.
- **When** the subsystem writes a saved team file, **the subsystem shall** produce output that the subsystem can later reload under the same semantic team-compatibility contract.
- **When** setup-state persistence succeeds, **the subsystem shall** preserve each side's relevant control-mode state and team state for restoration on the next SuperMelee entry.
- **When** restored setup state contains transient network-control mode that is not valid as local startup state, **the subsystem shall** sanitize or downgrade that state to a valid startup configuration rather than restoring an unusable network startup mode.

## Battle-time combatant selection

- **When** the battle/input layer requests the initial combatants for a SuperMelee match, **the subsystem shall** provide those combatants in a form compatible with the consuming battle/input boundary.
- **When** the battle/input layer requests the next combatant for a given side after a prior combatant loss, **the subsystem shall** provide that combatant in a form compatible with the consuming battle/input boundary.
- **Ubiquitous:** The subsystem shall not weaken the battle-facing handoff to bare ship identities or abstract descriptors unless the surrounding battle/input contract is explicitly redesigned in lockstep.
- **Ubiquitous:** SuperMelee combatant selection shall operate on fleet composition and selection state only; it shall not redefine ship combat behavior.

## Netplay integration boundary

- **When** SuperMelee netplay is not supported or not enabled for a session, **the subsystem shall** support complete local setup, selection, and battle handoff behavior without requiring network state.
- **When** the product supports SuperMelee netplay and a session enables network-controlled SuperMelee, **the subsystem shall** expose setup-time synchronization events for ship-slot changes, team-name changes, and any required whole-team bootstrap state used by that integrated mode.
- **When** the product supports SuperMelee netplay and a session enables network-controlled SuperMelee, **the subsystem shall** enforce the connection, readiness, and confirmation preconditions required at the SuperMelee/netplay boundary before starting the match.
- **When** the product supports SuperMelee netplay and a session enables network-controlled SuperMelee, **the subsystem shall** expose local battle-time combatant-selection outcomes and accept remote selection updates wherever that integrated mode requires them.
- **Ubiquitous:** The subsystem shall not require the SuperMelee specification to define lower-level netplay transport, framing, retransmission, or connection-state-machine behavior.

## Error handling and recovery

- **When** the subsystem encounters malformed persisted setup data, **the subsystem shall** reject that data and fall back to a valid setup state.
- **When** the subsystem encounters invalid ship identifiers in persisted team data, **the subsystem shall** fail cleanly or normalize those entries consistently without corrupting the active setup state.
- **When** the user attempts to start a match from invalid setup state, **the subsystem shall** remain in setup/menu flow and report or signal failure through the established UI/interaction path rather than entering battle.
- **Ubiquitous:** Failure in one team file, built-in team entry, or setup artifact shall not corrupt unrelated valid team state already present in memory.

## Compatibility-sensitive audit areas

- **Ubiquitous:** The implementation shall preserve firm load interoperability for valid legacy saved-team content even if internal storage or layout differs.
- **Ubiquitous:** If a compatibility audit proves that legacy saved-team files require byte-for-byte save-format compatibility, **the subsystem shall** preserve that file format.
- **Ubiquitous:** If a compatibility audit proves that exact built-in team content, exact UI navigation details, or exact setup-screen audiovisual timing are compatibility-significant, **the subsystem shall** preserve those behaviors accordingly.
- **Ubiquitous:** Until such an audit proves stronger obligations, the subsystem may satisfy those areas through semantic compatibility rather than internal-implementation identity.