# Communication / Dialogue Subsystem Requirements

## Scope

This document defines requirements for the communication/dialogue subsystem in EARS format. The subsystem covers encounter lifecycle, dialogue script dispatch, NPC speech and subtitle presentation, player response handling, animation and oscilloscope behavior, segue and battle outcomes, ownership/lifecycle obligations, and integration with encounter flow, state, graphics, and audio.

Requirements are organized into the following classes:
- **Functional behavior**: externally visible or script-visible behavior the subsystem must exhibit.
- **Compatibility constraint**: behavior that must be preserved for existing C race scripts or other established integration points.
- **Safety/lifetime constraint**: rules governing memory safety, resource lifetime, and concurrency correctness across integration boundaries.

## Encounter lifecycle

### EC-REQ-001 — Functional behavior
When an encounter is initiated, the communication subsystem shall resolve the requested encounter type into the active communication configuration for that encounter.

### EC-REQ-002 — Functional behavior
When encounter initialization requires encounter-type normalization for externally defined gameplay cases, the communication subsystem shall preserve externally visible encounter behavior while mapping internal communication handling to the correct active configuration.

### EC-REQ-003 — Functional behavior
When an encounter is initialized, the communication subsystem shall initialize race-specific communication data before any dialogue presentation or player response interaction begins.

### EC-REQ-004 — Functional behavior
When the encounter flow permits a hail-or-attack choice, the communication subsystem shall integrate with encounter flow to present that choice before starting dialogue.

### EC-REQ-005 — Functional behavior
If the player chooses to attack instead of hailing, the communication subsystem shall skip dialogue execution and produce the externally visible hostile encounter outcome required by encounter flow.

### EC-REQ-006 — Functional behavior
If the encounter flow does not permit a hail-or-attack choice, the communication subsystem shall enter the dialogue session directly.

### EC-REQ-007 — Functional behavior
When a dialogue session begins, the communication subsystem shall load and bind the visual, text, subtitle, palette, and music resources required by the active communication configuration before script-driven presentation begins.

### EC-REQ-008 — Functional behavior
When a dialogue session is active, the communication subsystem shall maintain all transient encounter-local communication state until the session exits.

### EC-REQ-009 — Functional behavior
When an encounter exits via any non-abort path, the communication subsystem shall invoke the encounter-exit side-effect hook (`post_encounter_func`) followed by the uninitialization callback (`uninit_encounter_func`) in that order. This applies to both the dialogue-session normal-exit path and the attack-without-hail path.

### EC-REQ-010 — Functional behavior
When a dialogue session exits due to abort or load interruption, the communication subsystem shall:
- skip the post-encounter callback,
- invoke the uninitialization callback,
- release all encounter-local resources (graphics contexts, loaded frames, fonts, colormaps, music, string tables),
- and leave encounter flow, graphics state, and communication state in a valid reusable state such that a subsequent encounter can initialize without error.

### EC-REQ-011 — Functional behavior
When dialogue processing completes, the communication subsystem shall evaluate the resulting segue state and integrate with encounter flow to determine whether combat follows.

### EC-REQ-012 — Functional behavior
If a hostile segue is active and combatants are available, the communication subsystem shall transfer control to encounter flow so that battle setup and battle execution occur.

### EC-REQ-013 — Functional behavior
If combat does not follow dialogue completion, the communication subsystem shall leave no pending battle segue visible to later encounter processing.

### EC-REQ-014 — Functional behavior
When encounter teardown completes, the communication subsystem shall release or invalidate all encounter-local resources and handles it owns.

### EC-REQ-015 — Functional behavior
When the player chooses attack without hailing, the communication subsystem shall invoke `post_encounter_func` and `uninit_encounter_func` exactly once each, in that order, without having invoked `init_encounter_func`. The attack-without-hail path is a non-abort exit path and therefore a valid invocation context for `post_encounter_func` under the encounter-exit semantics defined in EC-REQ-009.

### EC-REQ-016 — Functional behavior
Each encounter lifecycle callback (`init_encounter_func`, `post_encounter_func`, `uninit_encounter_func`) shall be invoked at most once per encounter across all exit paths. Encounter teardown (resource cleanup and encounter-record bookkeeping) shall not re-invoke any script lifecycle callback.

## Dialogue script and dispatch behavior

### DS-REQ-001 — Functional behavior
When an encounter requires race-specific dialogue logic, the communication subsystem shall dispatch to the correct race-specific script entrypoint for the active encounter configuration.

### DS-REQ-002 — Functional behavior
When script dispatch depends on external game state, the communication subsystem shall select the script entrypoint required by that external game state.

### DS-REQ-003 — Compatibility constraint
While race-specific scripts remain external to the communication subsystem implementation, the communication subsystem shall provide a source-compatible callable integration contract for those scripts such that all 27 race scripts compile and behave correctly without modification.

### DS-REQ-004 — Functional behavior
When a race-specific script returns encounter communication data, the communication subsystem shall copy or retain that data in a form that remains valid for the duration of the encounter.

### DS-REQ-005 — Functional behavior
When a script emits an NPC phrase by phrase identifier, the communication subsystem shall resolve that identifier against the active conversation phrase resource.

### DS-REQ-006 — Functional behavior
If a phrase identifier denotes externally defined dynamic text, the communication subsystem shall substitute the correct externally visible dynamic text for that identifier.

### DS-REQ-007 — Functional behavior
If a phrase identifier denotes a no-op phrase, the communication subsystem shall not queue visible or audible dialogue output for that phrase.

### DS-REQ-008 — Functional behavior
When a script requests phrase emission with callback behavior, the communication subsystem shall preserve the required ordering between phrase playback and callback execution: the callback fires after the associated phrase's audio clip finishes playback (or after text display if there is no audio clip), and the next queued phrase does not begin until the callback completes.

### DS-REQ-009 — Functional behavior
When a script requests phrase splicing without a page break, the communication subsystem shall preserve continuous externally visible dialogue flow across the splice.

### DS-REQ-010 — Functional behavior
When a script constructs a composed player phrase from multiple phrase fragments, the communication subsystem shall preserve fragment order in the resulting visible response text.

### DS-REQ-011 — Functional behavior
When a script queries or sets segue state, the communication subsystem shall expose segue behavior consistent with encounter flow expectations.

### DS-REQ-012 — Compatibility constraint
When a script disables a phrase, the communication subsystem shall ensure that subsequent phrase-enabled checks reflect the disabled state. Phrase identity and indexing shall remain aligned with string-table index conventions regardless of internal representation changes.

## Phrase-state lifecycle

### PS-REQ-001 — Functional behavior
When a new encounter begins, the communication subsystem shall initialize all phrase enable/disable state to the enabled state as loaded from the phrase resource. No phrase disablement state shall carry over from a previous encounter.

### PS-REQ-002 — Functional behavior
When a script disables a phrase via the established disable API, the communication subsystem shall record that disablement for the remainder of the current encounter's dialogue session.

### PS-REQ-003 — Functional behavior
When a script queries phrase-enabled status, the communication subsystem shall return disabled for any phrase that has been disabled during the current encounter, and enabled for all others.

### PS-REQ-004 — Functional behavior
When a disabled phrase is emitted via phrase-emission APIs, the communication subsystem shall still resolve and queue the phrase for playback using the original unmodified text and audio. Phrase disablement shall affect only enabled-status queries, not phrase emission or audio/text resolution.

### PS-REQ-005 — Functional behavior
The communication subsystem shall not provide an API to re-enable a disabled phrase within an encounter. The only reset path for phrase state shall be encounter teardown followed by a new encounter's resource load.

### PS-REQ-006 — Compatibility constraint
If a game load occurs during an encounter, the loaded encounter (if any) shall re-initialize phrase state from a fresh resource load. No stale phrase state from the interrupted encounter shall persist after load.

### PS-REQ-007 — Compatibility constraint
The phrase-disable implementation shall use a narrower semantic model where disablement affects only `PHRASE_ENABLED` checks and does not alter phrase text content for resolution purposes. Before implementation, all 27 race scripts shall be verified (by static analysis or manual audit) to confirm that no script calls `NPCPhrase` on a phrase it has previously disabled, resolves response text from a disabled phrase index, or depends on observing NUL-prefixed text content of a disabled phrase. If any script violates these invariants, the implementation shall either preserve the legacy NUL-mutation effect for that path or the script shall be corrected.

## Response registration and selection

### RS-REQ-001 — Functional behavior
When a script registers a player response, the communication subsystem shall store the response reference, visible response text, and callback association for later presentation and dispatch.

### RS-REQ-002 — Functional behavior
When a script registers a player response without explicit text, the communication subsystem shall resolve the visible response text from the active conversation phrase resource.

### RS-REQ-003 — Functional behavior
When a script registers a player response with explicit text, the communication subsystem shall present that explicit text without requiring phrase-resource lookup.

### RS-REQ-004 — Functional behavior
The communication subsystem shall support at most eight simultaneously selectable player responses.

### RS-REQ-005 — Functional behavior
If additional responses are registered after the supported maximum has been reached, the communication subsystem shall reject or ignore the excess registrations without corrupting response state.

### RS-REQ-006 — Functional behavior
When one or more responses are available, the communication subsystem shall render the available responses in the communication response region.

### RS-REQ-007 — Functional behavior
When one response is currently selected, the communication subsystem shall render the selected response distinctly from unselected responses.

### RS-REQ-008 — Functional behavior
When the number of registered responses exceeds the visible response capacity, the communication subsystem shall support scrolling through the response list and shall present externally visible scroll affordances.

### RS-REQ-009 — Functional behavior
When the player moves selection upward or downward, the communication subsystem shall update the selected response accordingly, including wrapping behavior if wrapping is part of the established encounter UX.

### RS-REQ-010 — Functional behavior
When the player confirms a selected response, the communication subsystem shall clear active response presentation, preserve the chosen response as the player's spoken selection for encounter feedback, and dispatch the selected response callback.

### RS-REQ-011 — Compatibility constraint
When the selected response callback is dispatched, the communication subsystem shall pass the selected response reference as an argument to the callback, matching the established C convention where `RESPONSE_FUNC` receives `RESPONSE_REF`.

### RS-REQ-012 — Compatibility constraint
If response callbacks are exposed across an integration boundary, the communication subsystem shall preserve the externally required callback signature and argument-passing behavior.

### RS-REQ-013 — Functional behavior
When a response callback registers a new response set, the communication subsystem shall present only the current active response set after callback completion.

### RS-REQ-014 — Functional behavior
If no player responses are available after NPC dialogue completes, the communication subsystem shall provide the established non-response exit behavior for the encounter, including replay capability before exit.

### RS-REQ-015 — Functional behavior
When the player requests dialogue review from the response-selection phase, the communication subsystem shall enter the conversation-summary behavior unless encounter rules prohibit it.

### RS-REQ-016 — Functional behavior
When a response callback is invoked, the following preconditions shall be guaranteed: the prior response list has been cleared, the player's chosen text is displayed in the feedback area, the track has been fully stopped, any pending completion associated with the prior turn's phrases has been either dispatched or discarded so that no phrase callback from the prior turn can arrive after response-callback entry, subtitle state has been reset, and music has been faded to background volume. Response callbacks may rely on these preconditions.

## Trackplayer behavioral contract

### TP-REQ-001 — Functional behavior
When the communication subsystem queues a phrase via the trackplayer's phrase-queue operation, the trackplayer shall treat that phrase as a single logical unit for completion callbacks, subtitle transitions, summary history recording, and skip advancement.

### TP-REQ-002 — Functional behavior
When multiple audio clips are merged via the trackplayer's multi-track merge operation, the trackplayer shall treat the result as a single phrase with one completion point, one subtitle entry, and one history entry.

### TP-REQ-003 — Functional behavior
When a phrase's audio clip finishes natural playback, the trackplayer shall fire that phrase's completion callback (if non-NULL) before beginning the next queued phrase.

### TP-REQ-004 — Functional behavior
When a phrase has no audio clip, the trackplayer shall fire that phrase's completion callback after the phrase's text display duration has elapsed, using the same timing rule as the current C trackplayer implementation.

### TP-REQ-005 — Functional behavior
When the player skips the current phrase via the skip operation, the trackplayer shall fire that phrase's completion callback. Pending phrases after the current one shall remain queued and shall play normally. The skip operation shall not coalesce or skip multiple phrases.

### TP-REQ-006 — Functional behavior
When seeking forward past the end of the current phrase, the trackplayer shall trigger that phrase's completion event and advance to the next queued phrase. When seeking backward, the trackplayer shall clamp at the current phrase's beginning boundary and shall not seek backward across phrase boundaries into already-completed phrases.

### TP-REQ-007 — Functional behavior
When playback is stopped via the stop operation, the trackplayer shall halt playback without firing callbacks for unplayed phrases. Pending phrase callbacks and any undelivered pending completions shall be discarded.

### TP-REQ-008 — Functional behavior
The trackplayer shall record subtitle history entries at queue time (when phrases are queued), not at playback time. Phrases that are queued but never reach natural playback shall appear in history. Replayed phrases shall not create duplicate history entries. Skipped phrases shall appear exactly once. History ordering shall match queue order.

### TP-REQ-009 — Functional behavior
When the communication subsystem requests replay of the last NPC speech segment, the trackplayer shall replay the most recently committed phrase that had associated audio or text content (the replay target). The replay target shall update only when a phrase becomes committed (after its callback has been dispatched or the phrase has been committed without a callback), never when a phrase is merely queued and never when a phrase is merely marked as having a pending completion. If a callback queued multiple follow-on phrases and all were committed before the next response phase, the replay target shall be the last of those committed phrases. Replay shall not alter subtitle history and the replayed phrase's callback shall not re-fire.

### TP-REQ-010 — Compatibility constraint
The trackplayer shall not invoke phrase callbacks on the audio thread. The trackplayer shall provide a mechanism for the main thread to detect pending phrase completions and retrieve the associated callback for main-thread invocation.

### TP-REQ-011 — Functional behavior
When a page-break-free splice appends content to the current phrase, the trackplayer shall treat the appended content as part of the same logical phrase for callback, subtitle, and history purposes.

### TP-REQ-012 — Functional behavior
When a phrase completion is detected, the trackplayer shall record a pending completion without advancing to the next phrase. The next queued phrase shall not become the current phrase until after the comm subsystem has dispatched the completed phrase's callback on the main thread and the callback has returned.

### TP-REQ-013 — Functional behavior
When a phrase has no audio clip and no timestamp metadata, the trackplayer shall use a default text-display duration consistent with the current C trackplayer behavior. When timestamp metadata is present, the trackplayer shall derive the text-display duration from that metadata.

## Subtitle and speech coordination

### SS-REQ-001 — Functional behavior
When an NPC phrase is emitted, the communication subsystem shall queue the associated speech, subtitle text, timing metadata, and callback metadata with the trackplayer in the correct playback order.

### SS-REQ-002 — Functional behavior
If an emitted NPC phrase has subtitle text but no speech audio, the communication subsystem shall still present the required subtitle content.

### SS-REQ-003 — Functional behavior
If an emitted NPC phrase has speech audio but no subtitle text, the communication subsystem shall preserve the required audible behavior and any externally defined subtitle behavior for that phrase.

### SS-REQ-004 — Functional behavior
When subtitle timing metadata is available, the communication subsystem shall coordinate subtitle updates with playback timing supplied by the trackplayer.

### SS-REQ-005 — Functional behavior
When no subtitle timing metadata is available, the communication subsystem shall still present subtitle content in a stable and externally consistent manner.

### SS-REQ-006 — Functional behavior
When speech playback begins, the communication subsystem shall enter the talking playback loop and remain in that loop until playback completion, skip, abort, or other established exit condition occurs.

### SS-REQ-007 — Functional behavior
While speech playback is active, the communication subsystem shall poll or receive current subtitle state from the trackplayer and update displayed subtitles when the effective subtitle content changes. Subtitle display shall track trackplayer state within one frame-update cycle.

### SS-REQ-008 — Functional behavior
When subtitle content changes, the communication subsystem shall clear or redraw subtitle presentation so that stale subtitle text is not left visible.

### SS-REQ-009 — Functional behavior
When subtitle presentation is disabled by configuration, the communication subsystem shall suppress subtitle rendering without suppressing required speech playback behavior.

### SS-REQ-010 — Functional behavior
When the player requests playback skipping during NPC speech, the communication subsystem shall advance playback to the externally defined end state for the current queued speech.

### SS-REQ-011 — Functional behavior
When the player requests seeking or replay during NPC speech, the communication subsystem shall use the established encounter playback controls and preserve subtitle/speech synchronization after the operation.

### SS-REQ-012 — Functional behavior
When the player requests replay of the most recent NPC speech from the response-selection phase, the communication subsystem shall replay the most recently committed phrase that had audio or text content, without re-firing that phrase's callback. The replay target shall reflect only committed phrases, not phrases that are merely queued or have a pending completion.

### SS-REQ-013 — Functional behavior
When the player enters conversation summary, the communication subsystem shall obtain the recorded subtitle history from the trackplayer and present reviewable dialogue text in encounter order.

### SS-REQ-014 — Functional behavior
When the recorded subtitle history exceeds one visible summary page, the communication subsystem shall paginate the summary while preserving text order and continuity.

### SS-REQ-015 — Functional behavior
If summary text spans a page boundary, the communication subsystem shall carry the remaining text onto the next summary page without losing content.

### SS-REQ-016 — Functional behavior
When the player exits conversation summary, the communication subsystem shall return to the appropriate response-selection or encounter-exit state.

### SS-REQ-017 — Functional behavior
Subtitle history scope shall be per encounter. The summary shall reflect the original dialogue queue order, not observed playback events: lines that were skipped or replayed shall appear exactly once. Replay and seek operations shall not alter summary history contents. Subtitle history shall be cleared when the encounter ends.

## Animation and oscilloscope behavior

### AO-REQ-001 — Functional behavior
When a dialogue session begins, the communication subsystem shall initialize communication animation state for the active encounter configuration.

### AO-REQ-002 — Functional behavior
The communication subsystem shall support ambient animation sequences, talking animation sequences, and transition animation sequences during a dialogue session.

### AO-REQ-003 — Functional behavior
When speech playback enters the talking state, the communication subsystem shall activate the required talking and transition animation behavior for the active encounter configuration.

### AO-REQ-004 — Functional behavior
When speech playback leaves the talking state, the communication subsystem shall return animation behavior to the required non-talking state for the active encounter configuration.

### AO-REQ-005 — Functional behavior
While a dialogue session is active, the communication subsystem shall advance active animations according to their configured timing, frame progression mode, and restart behavior.

### AO-REQ-006 — Functional behavior
When animation descriptors define mutual exclusion between animations, the communication subsystem shall not run conflicting animations concurrently.

### AO-REQ-007 — Functional behavior
When an ambient animation is configured to pause or settle during talking, the communication subsystem shall enforce that behavior while talking is active.

### AO-REQ-008 — Functional behavior
When an animation is configured as one-shot, the communication subsystem shall disable or stop that animation after its one-shot run completes unless explicitly restarted.

### AO-REQ-009 — Functional behavior
When an animation affects color transformation rather than portrait frame selection, the communication subsystem shall apply the configured color transformation behavior instead of sprite-frame substitution.

### AO-REQ-010 — Functional behavior
When animation or subtitle changes require visual updates, the communication subsystem shall redraw in a way that preserves correct layering between portrait presentation and subtitle presentation.

### AO-REQ-011 — Functional behavior
When a dialogue session begins, the communication subsystem shall initialize speech-visualization state required for oscilloscope and playback indicator presentation.

### AO-REQ-012 — Functional behavior
While speech playback is active, the communication subsystem shall update the oscilloscope using sample data provided by the audio subsystem.

### AO-REQ-013 — Functional behavior
When no valid oscilloscope samples are available, the communication subsystem shall preserve a stable oscilloscope presentation state without using invalid sample data.

### AO-REQ-014 — Functional behavior
While speech playback is active, the communication subsystem shall update the playback indicator to reflect the current playback state, including play, seek, and stopped states where those states are externally visible.

### AO-REQ-015 — Functional behavior
When speech playback stops or the dialogue session ends, the communication subsystem shall clear or reset speech-visualization state so that stale playback visuals are not left active.

### AO-REQ-016 — Functional behavior
Randomized animation timing (frame delays and restart delays) shall stay within the configured descriptor ranges. Exact random sequences need not be identical across implementations, but timing behavior shall be consistent with the base-rate and random-range parameters defined in each animation descriptor.

## Segue and battle outcomes

### SB-REQ-001 — Functional behavior
When dialogue logic sets the encounter segue to peace, the communication subsystem shall complete the encounter without initiating combat.

### SB-REQ-002 — Functional behavior
When dialogue logic sets the encounter segue to hostile, the communication subsystem shall produce the externally visible encounter outcome that leads to combat.

### SB-REQ-003 — Functional behavior
When dialogue logic sets the encounter segue to instant victory, the communication subsystem shall set `BATTLE_SEGUE` to the combat-pending value, set the instant-victory flag, and ensure the encounter flow path resolves the encounter as a victory without entering actual combat.

### SB-REQ-004 — Functional behavior
When dialogue logic sets the encounter segue to defeat, the communication subsystem shall set the crew sentinel value, trigger the restart check, and ensure the encounter flow path transitions to the game-over flow.

### SB-REQ-005 — Functional behavior
When dialogue completes, the communication subsystem shall translate the active segue state into the battle or non-battle transition behavior required by encounter flow.

### SB-REQ-006 — Compatibility constraint
Segue state mutations performed by `setSegue()` shall be complete and visible to all subsequent encounter-flow checks before the encounter exits the dialogue session and before any post-encounter hooks or teardown logic inspects segue state. The ordering requirements in the specification (§12.4) are normative.

## Ownership and lifecycle obligations

### OL-REQ-001 — Functional behavior
The communication subsystem shall own the authoritative runtime state for communication encounters for the duration of each active dialogue session.

### OL-REQ-002 — Functional behavior
When encounter-local communication resources are created, the communication subsystem shall retain ownership responsibility for their orderly release or invalidation at encounter end.

### OL-REQ-003 — Safety/lifetime constraint
When external scripts provide static or externally owned encounter descriptors, the communication subsystem shall not rely on shorter-lived storage than the script contract guarantees.

### OL-REQ-004 — Safety/lifetime constraint
When resource handles are loaded through graphics, audio, or state integrations, the communication subsystem shall store them in a form consistent with the owning subsystem's lifecycle rules.

### OL-REQ-005 — Functional behavior
When communication state is cleared between encounters, the communication subsystem shall reset transient dialogue, response, subtitle, animation, speech-visualization, and phrase enable/disable state.

### OL-REQ-006 — Functional behavior
When communication initialization is requested while the subsystem is already initialized, the communication subsystem shall reject or safely handle the request without corrupting subsystem state.

### OL-REQ-007 — Functional behavior
When communication-dependent operations are requested before communication initialization completes, the communication subsystem shall reject or safely no-op those operations without corrupting subsystem state.

### OL-REQ-008 — Functional behavior
When the subsystem is uninitialized, the communication subsystem shall release or invalidate any remaining runtime-owned communication state.

### OL-REQ-009 — Safety/lifetime constraint
If the implementation spans an integration boundary, the communication subsystem shall not expose pointers, callbacks, or handles whose validity rules are weaker than the published integration contract.

### OL-REQ-010 — Safety/lifetime constraint
If strings are returned across an integration boundary for immediate external consumption, the communication subsystem shall define and preserve externally safe lifetime behavior for those strings.

## Integration obligations

### IN-REQ-001 — Functional behavior
When NPC phrase content is queued, the communication subsystem shall integrate with the trackplayer to assemble and play speech tracks in dialogue order.

### IN-REQ-002 — Functional behavior
When subtitle review or current subtitle state is needed, the communication subsystem shall integrate with the trackplayer to obtain current and historical subtitle information.

### IN-REQ-003 — Functional behavior
When the trackplayer provides playback controls for skip, seek, rewind, fast-forward, replay, or completion detection, the communication subsystem shall use those controls in a way that preserves established encounter behavior.

### IN-REQ-004 — Functional behavior
When the audio subsystem provides speech sample data, the communication subsystem shall integrate that data into oscilloscope updates without requiring communication scripts to manage sample flow directly.

### IN-REQ-005 — Functional behavior
When dialogue resources or presentation updates are needed, the communication subsystem shall integrate with graphics to create, configure, update, and destroy the drawing contexts, frames, palettes, fonts, and draw operations required for communication presentation.

### IN-REQ-006 — Functional behavior
When portrait animation, subtitle redraw, response rendering, or conversation-summary rendering occurs, the communication subsystem shall use graphics integration in a way that preserves established visible encounter layout and layering.

### IN-REQ-007 — Functional behavior
When dialogue logic depends on persistent game variables, the communication subsystem shall integrate with state accessors to read and write those variables consistently with existing encounter logic.

### IN-REQ-008 — Functional behavior
When dynamic phrase substitution depends on commander identity, ship identity, planet identity, alliance identity, or related external state, the communication subsystem shall integrate with state to obtain the required visible values.

### IN-REQ-009 — Functional behavior
When encounter flow requires fleet construction, battle entry, encounter initialization, encounter teardown, or hyperspace encounter updates, the communication subsystem shall invoke or cooperate with encounter flow integrations in the required order.

### IN-REQ-010 — Functional behavior
When user input is required for response selection, playback seeking, replay, summary navigation, skip, or cancel behavior, the communication subsystem shall integrate with the input system to honor established control mappings.

### IN-REQ-011 — Safety/lifetime constraint
When communication logic and trackplayer callbacks can observe shared runtime state concurrently, the communication subsystem shall protect shared communication state with safe synchronization.

### IN-REQ-012 — Compatibility constraint
When build or configuration switches select the integrated communication implementation, the communication subsystem shall preserve all externally visible communication behavior. Intermediate mixed-mode states during migration are acceptable provided no externally visible behavior regresses. See specification §14.8 for the current and target build-switch definitions.

### IN-REQ-013 — Functional behavior
Per-frame communication work (animation scheduling, subtitle polling, oscilloscope updates, response rendering) shall complete within normal frame-budget constraints and shall not introduce visible stalls or frame drops under typical encounter conditions.

## Callback ordering and reentrancy

### CB-REQ-001 — Compatibility constraint
All script callbacks (phrase callbacks, response callbacks, encounter lifecycle callbacks) shall execute synchronously on the main game thread. No script callback shall execute on the audio thread.

### CB-REQ-002 — Compatibility constraint
Phrase callbacks (`CallbackFunction` queued via `NPCPhrase_cb`) shall fire after the associated phrase's audio clip finishes playback (or after text display if no audio clip), and before the next queued phrase begins playback.

### CB-REQ-003 — Compatibility constraint
Script callbacks may queue new NPC phrases, register new response options, mutate segue state, and disable phrases. These operations shall take effect after the callback returns and the main comm loop resumes.

### CB-REQ-004 — Compatibility constraint
Script callbacks shall not trigger nested response-selection input or nested talk-segue entry.

### CB-REQ-005 — Compatibility constraint
The ordering between callback completion, subtitle clearing, `TalkingFinished` state transition, talking-animation-to-silent transition, and response availability shall follow this sequence:
1. All queued phrases complete (or are skipped).
2. Subtitles are cleared.
3. `TalkingFinished` becomes true.
4. Talking animation transitions to silent.
5. Response selection becomes available (if responses are registered).

### CB-REQ-006 — Compatibility constraint
During execution of any script callback (phrase callback, response callback, or lifecycle callback), the communication subsystem shall permit the callback to call all APIs listed as permitted in the reentrancy contract (specification §12.3) without deadlock, data corruption, or blocking on locks held by the callback's caller.

### CB-REQ-007 — Compatibility constraint
During execution of any script callback, the communication subsystem shall not dispatch a second callback. Callbacks shall execute strictly sequentially with no nesting.

### CB-REQ-008 — Safety/lifetime constraint
When the communication subsystem invokes a C callback through FFI, it shall not hold any exclusive lock or mutable borrow on global communication state that would prevent the callback from re-entering the communication API. The implementation shall use a release-and-reacquire pattern, a staging-buffer pattern, lock-free interior fields, or an equivalent strategy to avoid deadlock. See specification §12.5 for the normative lock discipline rules.

### CB-REQ-009 — Safety/lifetime constraint
The audio thread shall never acquire the communication state lock. Audio-thread signals (phrase completion, sample delivery) shall use lock-free signaling mechanisms that the main thread polls.

### CB-REQ-010 — Compatibility constraint
When a callback queues both new NPC phrases and registers new response options in the same invocation, the communication subsystem shall process the queued phrases first (entering the talk segue) and present the registered responses only after all queued phrases complete.

## Compatibility and validation

### CV-REQ-001 — Compatibility constraint
Repeated encounter entry and exit shall not leak communication resources (graphics contexts, loaded frames/fonts/colormaps, music references, string tables, or internal buffers).

### CV-REQ-002 — Compatibility constraint
Save/load operations that occur during or adjacent to an encounter shall leave the communication subsystem in a state consistent with the loaded game, with no stale encounter state persisting.

### CV-REQ-003 — Compatibility constraint
Representative encounters shall preserve response availability and segue outcomes: the same dialogue choices under the same game state shall produce the same set of available responses and the same post-encounter outcome.

### CV-REQ-004 — Compatibility constraint
Subtitle history and replay controls shall remain usable across long conversations (encounters with many NPC phrases and multiple response rounds).

### CV-REQ-005 — Safety/lifetime constraint
Strings, callbacks, and resource handles passed across integration boundaries shall remain valid for the duration documented by the integration contract. Repeated immediate use (e.g., multiple encounters in sequence) shall not produce dangling references.

### CV-REQ-006 — Compatibility constraint
During the final battle encounter, conversation summary shall be unavailable. Any other encounter-specific control restrictions defined by race scripts shall be preserved.

### CV-REQ-007 — Compatibility constraint
After a game load during or adjacent to an encounter, the subsystem shall suppress spurious input processing until the load transition completes.

### CV-REQ-008 — Compatibility constraint
Response callback argument correctness shall be validated: when a response callback is dispatched, the response reference value passed to the callback shall exactly match the response reference that was registered with that callback, for every response registration path (direct text, phrase-resolved text, and constructed text).

### CV-REQ-009 — Compatibility constraint
Phrase callback ordering under skip, seek, and replay shall be validated: when the player skips a phrase, that phrase's callback shall fire exactly once before the next phrase begins. When the player seeks past a phrase boundary, the crossed phrase's callback shall fire. When a phrase is replayed, its callback shall not re-fire.

### CV-REQ-010 — Compatibility constraint
Abort and load interruption ordering shall be validated: when an abort or load interruption occurs during speech playback, pending phrase callbacks for unplayed phrases shall not fire, the post-encounter callback shall be skipped, the uninitialization callback shall fire, and all encounter-local resources shall be released without leaking.

### CV-REQ-011 — Compatibility constraint
Mixed-mode build validation shall confirm that intermediate migration states (where some comm functions are Rust-owned and others remain C-owned) produce no externally visible behavior regressions in representative encounters compared to the all-C baseline.

### CV-REQ-012 — Compatibility constraint
Summary history uniqueness under replay and skip combinations shall be validated: after any combination of skip, seek, and replay operations within an encounter, each queued phrase shall appear exactly once in the conversation summary, in original queue order, with no duplicates and no omissions.

### CV-REQ-013 — Compatibility constraint
Attack-without-hail lifecycle ordering shall be validated: when the player chooses attack, `post_encounter_func` and `uninit_encounter_func` shall each be invoked exactly once, `init_encounter_func` shall not be invoked, and no lifecycle callback shall be invoked a second time by encounter teardown.

### CV-REQ-014 — Compatibility constraint
Text-only phrase callback timing shall be validated: when a phrase has no audio clip, the phrase's completion callback shall fire after the text display duration elapses (matching the C trackplayer's timing rule), not immediately and not after an unbounded delay.

### CV-REQ-015 — Compatibility constraint
Phrase-disable behavior shall be validated across all script-facing resolution paths:
- `PHRASE_ENABLED` shall return false for a disabled phrase.
- `NPCPhrase` on a disabled phrase shall resolve the original unmodified text and audio.
- Response text resolution on a disabled phrase index shall yield the original unmodified text.
- Constructed response text (via `construct_response`) that includes fragments corresponding to previously disabled phrase indices shall resolve using the original unmodified text for those fragments.
- A callback-driven sequence that disables a phrase and then registers a response referencing that phrase's index within the same encounter shall resolve the response text from the original unmodified content.
- After a game load interrupts an encounter and a new encounter initializes, phrase state shall be freshly initialized from the resource load with no stale disablement state from the interrupted encounter, and subsequent phrase resolution shall reflect the clean state.

### CV-REQ-016 — Compatibility constraint
Replay target correctness after multi-phrase callback chains shall be validated: when a callback queues multiple follow-on phrases and all complete before the next response phase, the replay target shall be the last committed phrase with content, not the first or an intermediate phrase. The replay target shall not reflect phrases that are merely queued or merely pending completion.

### CV-REQ-017 — Compatibility constraint
No double teardown shall occur: across all exit paths (normal exit, abort, load interruption, attack without hail), each lifecycle callback shall be invoked at most once per encounter. Encounter teardown shall not re-invoke any lifecycle callback already called by the dialogue session or the attack path.

### CV-REQ-018 — Compatibility constraint
When a callback queues both phrases and responses in the same invocation, the communication subsystem shall be validated to confirm that phrases are processed first (talk segue entered) and responses are presented only after all queued phrases complete, with no interleaving or lost registrations.

## Script integration contract

### SC-REQ-001 — Compatibility constraint
The communication subsystem shall preserve source-level compatibility for all 27 C race scripts. Scripts shall compile and behave correctly without modification against the published script-facing API.

### SC-REQ-002 — Compatibility constraint
`PHRASE_ENABLED` and `DISABLE_PHRASE` shall continue to work at source level with existing phrase indices. Phrase identity and indexing shall remain aligned with string-table index conventions regardless of internal representation changes.

### SC-REQ-003 — Compatibility constraint
The communication subsystem shall not require binary layout compatibility with the C `LOCDATA` structure. Logical field compatibility obtained through FFI accessors is sufficient.

### SC-REQ-004 — Compatibility constraint
C globals that the communication subsystem replaces (response lists, subtitle state, animation state, encounter flags) shall not be directly accessed by race scripts. The script-facing API shall abstract all access to those values.
