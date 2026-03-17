# Campaign Gameplay Subsystem Requirements

## Purpose

This document defines the required externally observable behavior of the campaign-gameplay subsystem. Every requirement is intended to be verifiable from outside the subsystem boundary through externally visible behavior and, where necessary, approved boundary-level instrumentation as defined in the Verification surface section below. Requirements do not require source-internal inspection for verification. Requirements use event-driven ("When …") or invariant ("The subsystem shall …") forms.

This document defines verifier-facing obligations and acceptable external evidence for the subsystem contract defined in `specification.md`. Verification methodology here is normative only to the extent required to evaluate the covered claim/context families and inspection surfaces explicitly in scope for this documentation set; it does not create additional gameplay obligations beyond the subsystem contract.


Architecture-level constraints, implementation sequencing guidance, and audit-sensitive open questions are in `specification.md`, not here.

## Verification surface

Requirements in this document are verifiable through externally visible behavior and, where noted, approved boundary-level instrumentation. The verification surface uses three verifier-facing classes of observables:

- **Player-visible observables** — what the player can see, hear, and interact with directly.
- **Raw persistence-boundary observables** — facts directly inspectable from the persisted save artifact when it already exposes the needed canonical boundary facts in verifier-stable form.
- **Spec-defined canonical inspection observables** — normalized persistence-boundary facts surfaced through the Campaign Canonical Export Document or an equally direct raw inspection surface as permitted by specification §10.1.
- **Legacy-starbase observational exception:** For covered valid legacy starbase save/load claims only, specification §10.1 may require observation-based controlled comparison at the starbase post-load conformance observation point when raw-save facts do not sufficiently expose the closed starbase comparison object. This is a claim-family-specific fallback method, not a third general inspection surface.


The permitted verification surface includes:

- **Player-visible behavior:** Direct observation of what the player sees, hears, and can interact with during gameplay.
- **Persisted save output inspection:** Examination of persisted save files and state summaries at the persistence boundary — e.g., reading back saved campaign state fields, queue contents, and summary data to confirm round-trip fidelity. This does not require access to source internals; it requires only the ability to read the persisted output.
- **Persisted scheduled-event queue inspection:** Examination of the canonical persistence-boundary scheduled-event meaning, either directly from a verifier-stable raw save representation or through the Campaign Canonical Export Document when export-based inspection is used. This is persistence-boundary inspection, not source-internal inspection. The minimum per-entry schema and comparison rules relied on for this verification are defined in specification §8.6.
- **Persisted campaign-owned encounter/NPC state inspection:** Examination of persisted campaign-owned encounter and NPC ship state at the persistence boundary — e.g., reading back encounter identity, NPC ship queue contents, and fleet composition from persisted save state to confirm that active encounters and NPC state match expectations. For export-based verification, `encounter_identity` and queue entry identifiers shall use the normalized vocabularies defined in specification §10.1. This is persistence-boundary inspection, not source-internal inspection. For persistence-boundary equivalence, these canonical identifiers and queue contents are the primary conformance objects; player-visible observation does not override a canonical mismatch.
- **Controlled round-trip scenario comparison:** Comparison between a baseline play session (no intervening save/load) and a save/load-resumed play session under a controlled scenario that holds constant the event-relevant player choices, player presence/location conditions, timing-sensitive inputs, and any stochastic controls needed for the event under test, to confirm that the resumed session produces equivalent campaign progression outcomes. This evidence method is valid only where those controls can be made sufficiently deterministic for the event under test to support meaningful date/effect comparison. When that level of control is not achievable, conformance should instead rely on canonical scheduled-event inspection plus downstream observable campaign effects. This is a boundary-level comparison of observable campaign effects, not source-internal state diffing.
- **Campaign-to-encounter handoff identity observation:** Observation of which race, fleet composition, and ship identities are presented at the campaign-to-encounter subsystem boundary — e.g., confirming that a hyperspace encounter with a specific NPC group results in communication and/or battle with ships of the corresponding race identity. This is corroborating runtime-dispatch evidence for covered encounter handoff behavior; it does not replace or override the canonical persistence-boundary identifiers and queue contents when those are part of the covered conformance claim.
- **Campaign save-slot and save-state observation:** Observation of the set of campaign save slots and their content at the persistence boundary — e.g., confirming that a deferred transition does not create, modify, or consume any campaign save slot or its persisted content.
- **Campaign resume adjunct artifact set:** The closed verifier-facing set of campaign-required persisted artifacts explicitly surfaced at the persistence boundary by the save/load interface in addition to the primary save artifact: battle-group state files and per-system state files used by campaign resume.
- **Adjunct persisted-artifact observation:** Observation of the campaign resume adjunct artifact set. These are the adjunct artifacts in scope for failed-load no-mutation checks in this document.
- **Event-resume conformance evidence:** Conformance with the event-resume acceptance definition may be established by either (a) controlled round-trip scenario comparison as defined above, demonstrating that the resumed session produces the same campaign progression effects at the same campaign dates as the baseline session, or (b) canonical scheduled-event view inspection confirming correct selector identity, scheduling metadata, and queue membership at the persistence boundary, combined with downstream observable campaign effects confirming that the resumed events produce the required outcomes. Both methods are acceptable evidence; neither is the sole permitted approach.
**Conditional export applicability rule:** Export-based inspection is required only for covered claim/context families whose canonical persistence-boundary facts are not directly exposed by a documented, machine-readable, verifier-stable raw save representation. When raw-save inspection is sufficient for a covered claim/context family, no export entry point is required for conformance of that claim/context family. When export-based inspection is required or chosen, the Campaign Canonical Export Document and its machine-readable error result are the verifier-facing inspection representation defined by specification §10.1.

**Export-success versus overall resume conformance:** Successful canonical export from the primary save artifact establishes only the canonical persistence-boundary facts assigned to the chosen inspection surface for the covered claim/context being verified. For any covered claim/context whose correct resume also depends on the closed campaign resume adjunct artifact set, conformance additionally requires the adjunct-artifact conditions and load-failure behavior defined in the Campaign load — general load-failure contract section below. Successful export alone does not prove overall covered-context resume conformance for such contexts.

**Verifier-use preface:** `specification.md` is the controlling owner of the claim-classification, inspection-surface, comparison-object, exactness/equivalence, and load/export outcome rules for this subsystem. This document applies those controlling rules verifier-facing; it does not define a second independent decision framework.


**Verifier-facing consequence rules:**
- Gameplay/save-load semantic mismatches are subsystem compatibility failures.
- Canonical export schema/shape/token mismatches are inspection-surface conformance failures.
- When specification §10.1 makes the export surface mandatory for a covered claim/context, an inspection-surface conformance failure is sufficient to fail conformance for that claim/context even if gameplay behavior is otherwise believed to be correct.
- Source-internal inspection (examining internal data structures, stepping through code paths, or reading private state that is not externally surfaced or persisted) is not part of the verification surface.

**How to classify and verify a claim:**
1. Identify the covered context under specification §9.7 and the claim/context family being tested.
2. Determine the chosen verifier-facing inspection surface by applying specification §10.1's inspection-surface sufficiency rule, together with any explicit claim-family exception stated there.
3. Determine the comparison object, exactness-versus-equivalence mode, and adjunct sensitivity from the local normative section for that claim/context family, using specification §10.1's summary tables only as reference aids.
4. For starbase save/load claims only, apply the starbase verification checklist in the Key observable definitions section to identify the settled observation-state before comparing outcomes.
5. When one verification session evaluates multiple covered claim/context families from the same save artifact, the verifier shall choose the inspection surface independently for each claim/context family using specification §10.1. The verifier shall not mix raw-save and export facts within one claim/context family, but different covered claim/context families from the same save may legitimately use different surfaces.
6. Apply the verifier-facing consequence rules above and any covered-context adjunct or safe-failure rules that affect overall pass/fail outcome.

**Verifier report minimum fields:** For each evaluated save/context and claim/context family, verifier output shall state the claim/context family evaluated, the chosen inspection surface, the claim-local result, the overall covered-context result, and whether any adjunct dependency changed the overall covered-context result. A claim-local pass shall not be reported as an overall covered-context pass unless the overall covered-context result is also pass.





- **Interplanetary entry claims:** compare only the closed campaign-boundary interplanetary entry observables.
- **Post-encounter consequence claims:** compare save-summary and resumed-mode facts only at the stabilized post-consequence observation point defined in `specification.md`.


## Scope boundaries

- Solar-system exploration internals (orbit, scan, surface, lander) are outside this subsystem.
- Dialogue/comm subsystem internals are outside this subsystem.
- Per-ship combat mechanics and battle simulation internals are outside this subsystem.
- SuperMelee setup, menuing, and fleet editing are outside this subsystem.
- Netplay transport and protocol behavior are outside this subsystem.
- Lower-level graphics, input, audio, resource, file-I/O, and threading mechanics are outside this subsystem, though campaign gameplay depends on them.

## Key observable definitions

The following definitions clarify observables referenced by multiple requirements:

- **Legacy baseline reference:** Throughout this document, "current baseline implementation" and references to "legacy" behavior refer to the legacy C-owned codebase documented by the `/project-plans/20260311` current-state documentation set (including `initialstate.md`) from which the compatibility obligations in this document and `specification.md` were derived. This is a fixed historical reference, not a pointer to whatever the codebase looks like at any future date.

- **Recoverable campaign mode terminology:** In this document, "recoverable campaign mode" refers only to the closed resumable mode vocabulary defined in `specification.md` §3.1. Victory and defeat are terminal outcomes, not recoverable modes. Start-flow / no-resumed-session state after a rejected load is outside the campaign-mode vocabulary entirely.

- **Valid legacy campaign save:** A campaign save file that satisfies all of the following concrete persistence-boundary conditions:
  1. produced to completion by the legacy baseline implementation in a campaign save context covered by specification §9.7,
  2. structurally complete and not truncated or corrupt after production (the file is intact at the persistence boundary),
  3. conforms to the legacy baseline persistence format as produced by the legacy baseline for covered contexts — i.e., its structure, field layout, and encoding match the format the legacy baseline produces, without truncation or corruption,
  4. every event selector present in the persisted scheduled-event state is a member of the campaign event catalog (specification §8.6), and
  5. all persisted scheduled-event records can be successfully decoded into a selector identity (matching the catalog vocabulary) and legacy scheduling metadata (date encoding, field values, and entry structure) without truncation, corruption, or malformed record structure, as evaluated against the legacy baseline persistence representation for covered contexts.

  Conditions 4 and 5 are the legacy-save-facing restatement of the mandatory scheduled-event acceptance boundary defined in specification §9.4.1 — they scope which persisted schedule data the end-state is obligated to accept, not an additional narrowing layer beyond the mandatory rejection cases. Saves whose schedule data would trigger the §9.4.1 rejection cases are outside the valid set by definition; saves that pass these conditions are within the mandatory acceptance obligation.

  Saves that fail any of the above conditions are outside the valid set and are not covered by the legacy save compatibility contract. Files presented as legacy-format inputs but falling outside this valid set are outside the interoperability obligation but, if presented for loading, remain subject to the general load-failure safe-failure contract defined in the Campaign load — general load-failure contract section below and in specification §9.4.0b.

- **Non-starbase save/load observable equivalence scope:** For non-starbase save/load claims, compare only the campaign-boundary observables listed in specification §9.4.0a: recoverable campaign mode, navigation/location sufficient for the next dispatch, fleet roster, active encounter presence/identity where applicable, campaign-owned encounter-entry handoff inputs where applicable, campaign progression flags, clock/date state, and scheduled-event resume behavior. Unlisted UI-local or transient runtime state is outside the equivalence contract for non-starbase contexts unless a later normative section explicitly includes it.

- **Player-visible starbase progression point:** The verifier-facing comparison object for starbase save/load claims, defined by specification §6.6 and used at the starbase post-load conformance observation point. Compare only the following closed observation-state elements:
  - whether a specific forced Commander conversation (first availability or post-bomb-installation) is pending or completed,
  - whether the relevant mandatory special-sequence branch is pending, active, or completed,
  - bomb-transport special-sequence status,
  - pre-alliance Ilwrath-response status,
  - whether the normal starbase menu is accessible as the next non-mandatory interaction,
  - whether normal departure is available, and
  - the mandatory next action that becomes actually executable first under zero optional player input.

  **Mandatory-next-action rule:** At the starbase post-load conformance observation point, the mandatory next action is the first player-visible mandatory route that becomes actually executable after all automatic routing triggered by the restored starbase state has settled and before any optional player interaction is permitted. If multiple mandatory routes are latent at save time, conformance is judged by whichever route surfaces first under that zero-optional-input settlement. Departure availability and menu accessibility are assessed only after applying this rule; a departure or menu affordance does not count as available if a mandatory route must surface first.


  Unlisted starbase-local presentation or pre-settlement UI state is outside the comparison object unless specification §6.6 explicitly makes it comparison-critical.

  **Starbase verifier-facing checklist:**
  1. Identify whether the input is a covered valid legacy starbase save or an end-state-produced starbase save.
  2. Determine the inspection surface using specification §10.1 and the starbase exception rule stated there.
  3. Determine the settled post-load observation-state at the starbase post-load conformance observation point.
  4. Compare only the closed observation-state elements listed above.
  5. For the covered valid legacy exception path, the authoritative baseline comparator is the closed starbase comparison object produced after applying the same zero-optional-input settlement procedure to the legacy baseline and observing the settled result at the same conformance observation point. Intermediate baseline shell/menu presentation differences are non-comparison data.
  6. Observation-based controlled comparison is a claim-family-specific fallback method only for covered valid legacy starbase saves whose raw-save facts do not sufficiently expose the closed comparison object. It is mandatory in that insufficiency case, forbidden as a preference override when raw-save facts are sufficient for that claim, and it is not a third general inspection surface outside this exception.




- **Encounter identity at the campaign boundary:** The race and fleet composition presented to the player when an encounter begins — observable through the communication/dialogue entry (which race's dialogue is initiated) and, if combat occurs, the ship identities fielded by the opposing side. Encounter identity is confirmed when the race and ships presented match the NPC group that triggered the encounter.

- **Event-resume acceptance:** After a save/load round-trip, scheduled campaign events are considered correctly resumed if, under the same subsequent gameplay scenario, they produce the same campaign progression effects (story-flag updates, faction changes, alliance shifts, and other defined event-handler outcomes) at the same campaign dates as they would have without the intervening save/load. "Same subsequent gameplay scenario" means a controlled scenario that holds constant the event-relevant player choices, player presence/location conditions, timing-sensitive inputs, and any stochastic controls needed for the event under test. Acceptable verifier evidence for this definition is specified in the Verification surface section; it is not part of the semantic definition itself.

- **Campaign-boundary interplanetary entry observables:** The closed set of campaign-owned entry conditions for interplanetary/solar-system exploration dispatch, as defined in specification §7.2.1. This set consists of: destination system identity, entry routing kind (normal interplanetary exploration entry versus special-case encounter routing), and any campaign-boundary transition marker needed to produce the correct first exploration dispatch behavior. References in this document to "campaign-owned entry conditions needed for that exploration dispatch" refer to this closed set.


- **When** the user selects new game, **the subsystem shall** enter the main campaign loop in interplanetary mode at the Sol starting location, consistent with the start-flow contract in `specification.md` §4.2. If an introductory sequence is shown first, the campaign state after that sequence shall still recover to the same campaign-boundary starting mode/location.


- **When** the user enters the campaign start flow, **the subsystem shall** present or process a new-game / load-game decision and loop until a valid campaign start is established or the user exits the start flow.
- **When** the user selects new game, **the subsystem shall** initialize all campaign runtime state to the defined starting configuration.
- **When** the user selects new game, **the subsystem shall** begin the campaign at the defined campaign start date. The initial set of campaign events defined in specification §8.2 shall be scheduled such that they fire at their defined times during subsequent play.
- **When** the user selects new game and an introductory sequence is defined, **the subsystem shall** play the introductory sequence before entering the main campaign loop.
- **When** the user selects load game and a valid save is loaded, **the subsystem shall** resume the campaign loop in the recoverable campaign mode and location recoverable from the save, with the player's fleet roster, active encounters, and NPC ship state matching the save.
- **When** the user selects load game and a valid save is loaded, **the subsystem shall** resume campaign date progression and scheduled event behavior consistent with event-resume acceptance as defined above.
- **When** a loaded save represents an interplanetary mode without a starbase context, **the subsystem shall** resume into solar-system exploration such that the next dispatched exploration entry targets the saved destination system and the campaign-boundary interplanetary entry observables (specification §7.2.1) match the saved context.
- **When** a loaded save represents a starbase context (whether encoded as a distinct persisted value, an encounter-family activity plus a starbase-context marker, or any equivalent representation per specification §3.1), **the subsystem shall** resume into the starbase visit flow at the player-visible starbase progression point as defined by the closed mandatory-element progression-point definition and the mandatory-next-action rule (see Starbase save/load resume section).
- **When** a load-game attempt fails or no valid save is available, **the subsystem shall** return to the start/restart flow without entering the campaign loop.

## Campaign loop and activity dispatch

- **When** a deferred transition is pending at the start of a loop iteration, **the subsystem shall** dispatch to the target activity designated by the transition request.
- **When** the campaign state requires an encounter or starbase visit (due to hyperspace collision, scripted event, or equivalent trigger), **the subsystem shall** dispatch to the encounter/starbase flow. If the encounter context identifies a starbase visit, the subsystem shall route to the starbase visit flow; otherwise, it shall route to the encounter/communication flow.
- **When** the campaign state requires solar-system entry (due to hyperspace-to-interplanetary transition or equivalent trigger), **the subsystem shall** dispatch to the solar-system exploration entry point.
- **When** no encounter, starbase visit, or solar-system entry is required, **the subsystem shall** enter the hyperspace/quasispace navigation runtime.
- **When** the player wins the final battle, **the subsystem shall** exit the campaign loop and proceed to the campaign-victory outcome.
- **When** the player dies, **the subsystem shall** exit the campaign loop and proceed to the campaign-defeat outcome.
- **When** a restart or abort is requested from within a running campaign, **the subsystem shall** exit the campaign loop and return to the start/restart flow without carrying stale campaign state into a subsequent session.

## Deferred-transition behavior

- **When** a sub-activity requests a deferred transition, **the subsystem shall** dispatch to the designated target activity on the next main-loop selection cycle.
- **When** a deferred transition is requested, **the subsystem shall** ensure that the requesting sub-activity does not itself enter the target activity and that the target activity is entered exactly once on the next top-level campaign selection cycle.
- **When** a deferred transition has been processed, **the subsystem shall** dispatch to the target activity exactly once; the transition shall not repeat on subsequent loop iterations.
- **When** a deferred transition is processed, the target activity shall be entered with the same externally visible initialization and entry behavior as if the top-level campaign loop had selected that activity directly in the normal course of play.
- **When** a deferred transition is processed, the transition shall not present itself as a real save/load operation to the player, shall not mutate or consume any campaign save slot or persisted campaign save state as part of the handoff, and shall not alter the set of available saves or their content. (Verifiable via campaign save-slot and save-state observation at the persistence boundary as defined in the Verification surface section.)

## Hyperspace and navigation transitions

- **When** the player collides with an NPC group in hyperspace, the resulting encounter identity at the campaign boundary shall correspond to the collided NPC group, and the subsystem shall preserve the player's pre-encounter navigation context so that campaign resume after the encounter restores that context.
- **When** the player enters a solar system from hyperspace, **the subsystem shall** transition to solar-system exploration such that the next dispatched exploration entry targets the destination system and the campaign-boundary interplanetary entry observables (specification §7.2.1) match the selected context.
- **When** the player enters a solar system that is a special-case encounter destination (e.g., Arilou homeworld space), **the subsystem shall** route to an encounter instead of normal interplanetary entry.
- **When** the player transitions between hyperspace and quasispace, **the subsystem shall** handle the transition as a campaign-layer navigation event, including any special-case portal mechanics.
- **When** the hyperspace menu is opened, **the subsystem shall** support device usage, cargo display, roster display, save/load, starmap access, and return to navigation.
- **When** a hyperspace menu action triggers a campaign transition (e.g., device usage causes an encounter), **the subsystem shall** exit the hyperspace menu and allow the campaign loop to process the resulting transition.

## Encounter handoff and post-encounter processing

- **When** an encounter is triggered, **the subsystem shall** identify the encountered race from the current encounter context and dispatch to the appropriate communication/dialogue entry point.
- **When** an encounter leads to combat, **the subsystem shall** provide the battle subsystem with the participating ship identities, fleet composition, and backdrop selection derived from campaign context, and invoke the battle subsystem.
- **When** combat completes and control returns to the campaign, **the subsystem shall** record battle outcome counters and restore the prior campaign activity.
- **When** an encounter resolves normally (not aborted, not player death, not final-battle), **the subsystem shall** determine victory/defeat/escape from battle counters and story state, identify the encountered race, and apply campaign consequences (salvage, progression-flag updates, removal of defeated ships from encounter or escort rosters).
- **When** an encounter resolves normally, **the subsystem shall** clean up encounter state so the campaign resumes the prior navigation activity.
- **When** an encounter exits due to abort, load, player death, or final-battle resolution, **the subsystem shall** suppress normal post-encounter consequence processing.

## Starbase visit flow

- **When** the player visits the allied starbase, **the subsystem shall** enter the starbase visit mode via the encounter/starbase dispatch path with appropriate special-case handling for the mandatory special-sequence categories enumerated in specification §6.4 (bomb-transport special sequence and pre-alliance Ilwrath-response sequence).
- **When** the starbase is not yet allied and the pre-alliance sequence applies, **the subsystem shall** run the Commander conversation, conditionally stage the Ilwrath response battle, and return to conversation after that battle.
- **When** a starbase visit requires a story-driven time skip, **the subsystem shall** advance the game clock by the defined number of days.
- **When** the starbase visit requires a forced Commander conversation (first availability or post-bomb-installation), **the subsystem shall** run that conversation before entering the normal starbase menu flow.
- **When** the starbase menu is active, **the subsystem shall** support Commander conversation, outfit, and shipyard interactions, and the menu shall exit on load, abort, or normal departure.
- **When** the player departs the starbase normally, **the subsystem shall** resume campaign navigation in interplanetary mode via a deferred transition, so the main campaign loop dispatches to the interplanetary activity on the next selection cycle.


## Starbase save/load resume

- **When** a save is made while in starbase, **the subsystem shall** persist enough campaign state that the closed starbase progression-point contract is recoverable on load.
- **When** a save that represents a starbase context is loaded, **the subsystem shall** resume into the starbase visit flow at the closed starbase progression-point contract — as defined by the closed mandatory-element progression-point definition and the mandatory-next-action rule — that was active at save time. Conformance is assessed at the starbase post-load conformance observation point defined in the Key observable definitions section.
- **When** a save that represents a starbase context is loaded, no mandatory starbase conversation or sequence that was already completed before the save was made shall replay solely because of the load.
- **When** a save that represents a starbase context is loaded, no mandatory starbase conversation or sequence that was still pending at save time shall be skipped solely because of the load.
- **When** a save that represents a starbase context is loaded, the mandatory next action after load shall match the mandatory next action that would have been available at save time. No non-mandatory starbase interaction shall become available before the pending mandatory action or sequence is surfaced. Implementations may pass through an intermediate shell or menu presentation so long as no non-mandatory interaction becomes possible before the mandatory action is presented.
- **When** a save that represents a starbase context is loaded and a forced conversation or mandatory special sequence was pending at save time, that conversation or sequence shall surface after load before the player can access any non-mandatory starbase interaction.
- **When** the player departs the starbase after a starbase-context load, the departure and subsequent campaign navigation shall behave equivalently to departure without an intervening save/load.

## Event progression and campaign clock

- **When** a new game starts, **the subsystem shall** schedule the initial set of campaign events defined in specification §8.2 such that they fire at their defined times during subsequent play.
- **When** a scheduled campaign event becomes due, **the subsystem shall** produce the defined campaign progression effects (story-flag updates, faction fleet changes, alliance changes, encounter-generation adjustments, follow-on event scheduling, or other defined effects).
- The subsystem shall support the complete campaign event catalog defined in specification §8.6 at the selector-vocabulary and externally visible outcome-family level.
- When specification §8.6 designates row-specific checkpoint bundles, canonical-stage surfacing rules, or other verifier-facing comparison normalizations for particular event families, those normalizations shall be applied exactly as designated there for those rows and not generalized further by this document.
- For scheduled-event persistence-boundary verification, the controlling comparison object, canonicalization rule, and any row-specific verifier normalization are defined exclusively in specification §8.6.


- **When** the campaign is in hyperspace or quasispace, campaign time shall advance at the hyperspace pacing rate, producing the correct pacing of campaign events and faction fleet movements for hyperspace activity.
- **When** the campaign is in interplanetary or solar-system exploration, campaign time shall advance at the interplanetary pacing rate, producing the correct pacing of campaign events for interplanetary activity.
- **When** the campaign is at the starbase and a story beat requires a time skip, **the subsystem shall** advance the game clock by the specified number of days.
- The subsystem shall preserve the relative pacing behavior: hyperspace time shall pass at a different rate than interplanetary time, and this difference shall be observable through the rate at which campaign events fire and faction fleets move in each activity mode.

## Campaign save

- **When** a save is requested, **the subsystem shall** write a save summary derived from the current campaign state, with appropriate remapping for special contexts (quasispace, starbase, planet orbit, final battle).
- **When** a save is requested, **the subsystem shall** persist enough campaign state to support a full round-trip resume: loading the resulting save shall restore the recoverable campaign mode, game clock, fleet roster, active encounters, NPC ship state, and campaign progression flags equivalently within the applicable closed equivalence scope, and shall restore navigation/location state within the non-starbase save/load observable equivalence scope defined above. For non-starbase contexts, equivalence is scoped to the campaign-boundary observables listed in the Non-starbase save/load observable equivalence scope definition above.
- **When** a save is requested and active NPC battle-group state files exist for visited systems, **the subsystem shall** persist those state files as part of the save.
- **When** a save is requested from a special activity context (e.g., homeworld encounter screen, interplanetary re-entry), **the subsystem shall** apply campaign-specific save-time adjustments so the save resumes correctly.

## Campaign load

- **When** a save is loaded, **the subsystem shall** resume the campaign in the mode recoverable from the save, with fleet roster, active encounters, campaign progression flags, and navigation/location state equivalent within the applicable closed equivalence scope. For starbase-context saves, equivalence is constrained by the closed player-visible starbase progression-point definition and the mandatory-next-action rule in the Key observable definitions section. For non-starbase contexts, equivalence is scoped to the campaign-boundary observables listed in the Non-starbase save/load observable equivalence scope definition. All state not listed in the applicable equivalence scope is outside the contract.
- **When** a save is loaded, **the subsystem shall** resume campaign date progression and scheduled event behavior consistent with event-resume acceptance as defined in the Key observable definitions section.
- **When** a save is loaded and the saved state represents an interplanetary mode (without a starbase context), **the subsystem shall** resume into solar-system exploration such that the next dispatched exploration entry targets the saved destination system and the campaign-boundary interplanetary entry observables (specification §7.2.1) match the saved context.
- **When** a save is loaded and the saved state represents a starbase context (whether encoded as a distinct persisted value, an encounter-family activity plus a starbase-context marker, or any equivalent representation per specification §3.1), **the subsystem shall** resume into the starbase visit flow as specified in the Starbase save/load resume section.
- **When** a load is initiated from within a sub-activity (e.g., hyperspace menu), **the subsystem shall** leave the player outside resumed gameplay from the rejected or superseded sub-activity state and shall enter the activity designated by the loaded save exactly once as the next top-level campaign activity.
- For load/export outcome classes and representative mixed-case examples, the controlling classification rules are specification §10.1 and its representative load/export classification examples table. This document applies those classifications verifier-facing; it does not create a separate additive classification taxonomy.


## Campaign load — general load-failure contract

All campaign-owned load-state failures — including parse failures, structural corruption, missing required persisted components, malformed payload sections, and any other persistence-boundary failure in campaign-owned serialized state — are subject to the following safe-failure guarantees:

- No portion of the rejected save shall become the active resumed campaign state.
- No partial application of state from the rejected save shall be externally observable after the failure. This guarantee applies to post-failure externally observable and persistence-boundary state. Temporary internal restoration work (e.g., staging, trial parsing, or rollback of lower-boundary state) may occur during validation, but after rejection no resumed gameplay state, no persisted state mutation, and no other externally observable runtime state derived from the rejected save may remain active.
- If the load was initiated from the entry/start flow, control shall return to the start/load flow without entering resumed gameplay.
- If the load was initiated from within a running campaign sub-activity, the pre-load running campaign session shall remain active and the user shall remain outside resumed gameplay from the rejected save. This requirement preserves campaign-boundary state and next-dispatch behavior only; it does not require return to the exact same transient submenu, shell, cursor position, or other sub-activity-local presentation state from which the load was initiated.


**Covered-context adjunct-dependency rule:** The required adjunct artifacts for each covered save context are defined exclusively by specification §9.4.0b (Adjunct-dependency table for covered contexts) together with the verifier-facing adjunct-dependence classifier in that same section. Verifiers shall determine adjunct necessity from that context-indexed rule set, not from broad save category, visible screen type, or implementation architecture.

The mandatory scheduled-event rejection cases defined in the next section are a specific mandatory subset of this broader contract: they define concrete cases where load **shall** fail, and when load fails for those (or any other) campaign-owned persistence-boundary reasons, the safe-failure guarantees above shall hold.

- **When** a covered resume context requires an artifact from the closed campaign resume adjunct artifact set (battle-group state files or per-system state files) and that required artifact is missing, unreadable, structurally invalid, or rejected by its owning boundary, **the subsystem shall** fail the load and shall not resume the campaign from that save.

**Cross-boundary load-failure seam rule:** The overall campaign resume operation shall fail safely under the guarantees above if any required dependency cannot be successfully restored through its own owning boundary — including campaign-owned state sections, clock-restored schedule data rejected by campaign gameplay, or adjunct campaign-required data loaded through state/state-file helpers — regardless of which boundary detects the failure first. This rule defines the overall campaign resume outcome; it does not transfer ownership of lower-boundary formats, validation policy, or semantic interpretation to campaign gameplay. Each subsystem boundary remains the owner of its own persisted-data validation and restoration logic; this rule only requires that a restore failure in any required dependency produces a safe campaign-load failure rather than a partial-application state.

**Cross-boundary safe-failure verification evidence:** For cross-boundary restore failures (including failures detected by lower boundaries such as clock-subsystem schedule restore, state-file helpers, or adjunct data loaders), acceptable verifier evidence that the safe-failure guarantees hold is:

- After rejection, either control is back at the start/load flow with no resumed campaign state from the rejected save, or the pre-load running campaign session remains active with no state from the rejected save applied.
- No campaign save slot, primary persisted save artifact, or verifier-facing adjunct persisted artifact in the documented campaign resume set (battle-group state files and per-system state files) has been mutated by the rejected load attempt.
- Any canonical export, save-summary observation, or adjunct persisted-artifact observation performed after the failure reflects either the pre-load session state (if a campaign was running) or no resumed state from the rejected save (if the load was initiated from the start flow).
- For failed in-session load attempts, acceptable verifier evidence additionally includes that the post-failure recoverable campaign mode, navigation/location identity, fleet roster, campaign date/tick state, and next main-loop dispatch behavior remain consistent with the pre-load running session rather than with the rejected save.


This clause defines the verification evidence for the overall safe-failure outcome at the campaign boundary. It does not prescribe how lower boundaries implement their own rollback or staging, and it does not transfer ownership of lower-boundary validation internals to campaign gameplay.

## Campaign load — semantic validity of restored event state

The following are the required rejection cases. Load shall fail — and the campaign shall not resume — if any of these conditions is detected:

- **When** a save is loaded and the restored scheduled-event state contains an event selector not present in the campaign event catalog (specification §8.6), the load shall fail and the campaign shall not resume from the corrupted state.
- **When** a save is loaded and the restored scheduled-event state contains an event entry whose metadata encoding is structurally invalid at the persistence boundary (e.g., malformed date encoding, out-of-range fields, or metadata that cannot be parsed as a valid scheduled-event entry), the load shall fail and the campaign shall not resume from the corrupted state.
- The subsystem shall not silently continue campaign play with scheduled-event state that matches any of the above rejection cases.

When a load is rejected under any of the above mandatory rejection cases, the safe-failure guarantees defined in the general load-failure contract above shall hold.

Structurally parseable campaign-save inputs that fall outside the mandatory acceptance sets (valid legacy saves and same-subsystem round-trip saves) and do not trigger one of the mandatory rejection cases above are outside the conformance contract if they are semantically inconsistent in some other campaign-owned way. Implementations may accept or reject such inputs. If they reject them, the general load-failure safe-failure contract above shall hold.


Campaign gameplay owns semantic interpretation of restored scheduled events and may perform broader semantic validation beyond the above rejection cases. Conformance requires only these concrete persistence-boundary rejection cases. Valid legacy compatibility is scoped to the defined valid legacy save set; event-resume correctness is scoped to same-subsystem round-trip saves and valid legacy saves, not to arbitrary structurally parseable files that may have been externally modified.

> **Non-normative note:** An implementation may detect and reject additional semantic inconsistencies beyond these two cases (e.g., events whose preconditions have been permanently resolved, or event-type-specific metadata interpretation beyond persistence-boundary parsing). Such additional validation is not required for conformance.

## Save/load round-trip fidelity

- A campaign save produced by the subsystem in a campaign save context covered by specification §9.7 shall be loadable by the same subsystem, and the resumed campaign shall proceed with behavior equivalent within the applicable closed equivalence scope. This same-subsystem round-trip obligation is limited to the campaign-owned covered contexts in specification §9.7 unless a later normative section explicitly extends it. For starbase-context saves, equivalence is constrained by the closed player-visible starbase progression-point definition and the mandatory-next-action rule in the Key observable definitions section. For non-starbase contexts, equivalence is scoped to the campaign-boundary observables listed in the Non-starbase save/load observable equivalence scope definition. All state not listed in the applicable equivalence scope is outside the equivalence contract.
- The subsystem shall load valid legacy campaign saves (as defined in Key observable definitions) produced by the legacy baseline implementation for campaign save contexts listed as covered in specification §9.7. Semantic equivalence of restored state satisfies this requirement; byte-for-byte or numeric identity of persisted data is not required unless explicitly specified elsewhere in this document.
- Loading an end-state-produced save in the legacy implementation is not part of the acceptance contract.

## Legacy-save compatibility — in-scope campaign save contexts

The legacy-save load obligation applies to the campaign save contexts listed in specification §9.7. For quick reference, the covered contexts are: hyperspace navigation, quasispace navigation, interplanetary/solar-system entry, starbase visit, campaign-owned encounter-entry handoff state, post-encounter campaign consequence state, final-battle save contexts, sub-activity-initiated campaign loads (hyperspace menu), and homeworld encounter screen saves. Not covered: mid-dialogue/communication internals, battle-runtime-internal checkpoints, and SuperMelee saves. Contexts not explicitly listed in either the covered or not-covered set in specification §9.7 are outside this document's legacy-save compatibility obligation. See specification §9.7 for the complete table and rationale.

## Error handling and robustness

- **When** a save attempt fails, **the subsystem shall not** report success or leave the campaign in an inconsistent state.
- **When** an encounter, starbase, or sub-activity exits due to abort or load, **the subsystem shall** handle the interruption without corrupting campaign runtime state.
- The subsystem shall not carry stale state from a previous campaign run into a new-game or load-game session initiated through the restart flow.
