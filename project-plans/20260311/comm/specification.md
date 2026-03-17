# Communication / Dialogue Subsystem — Functional and Technical Specification

This document specifies the desired end state for the Rust communication/dialogue subsystem. It describes what the subsystem does, what it owns, and how it interfaces with the rest of the engine. It is not an implementation plan.

Status labels used in this document:
- **Current state**: describes what exists in the codebase today.
- **Target state**: describes the intended end state after migration.
- **Required compatibility behavior**: describes externally visible behavior that must be preserved regardless of implementation choices.
- **Implementation-parity note**: describes legacy implementation details that are informational guidance, not independently normative. Implementers should match these where verified by code review, tests, or visual comparison, but may redesign internals provided the behavioral contract is preserved.

---

## 0. Terminology

The following terms have precise meanings throughout this document:

| Term | Definition |
|---|---|
| **Track** | The assembled encounter playback queue: the ordered sequence of all phrases queued during a dialogue turn. |
| **Phrase** | The logical unit of speech delivered by one `SpliceTrack` call. Phrases are the unit at which completion callbacks fire, summary history entries are recorded, and skip (`JumpTrack`) advances. A single phrase may contain multiple internal subtitle pages (see below); those intra-phrase page transitions are presentation-only events and do not advance phrase state, trigger callbacks, create history entries, or update the replay target. |
| **Subtitle page (intra-phrase)** | A display page within a single phrase's subtitle text. The trackplayer (`audio-heart`) may split one phrase's text into multiple subtitle pages for display paging. `GetTrackSubtitle()` may advance across these pages while `PlayingTrack()`, callback eligibility, history entries, and replay-target state remain on the same logical phrase. These are internal presentation transitions, not phrase-level events. |
| **Page** | A subtitle pagination unit within the conversation summary view. One phrase's subtitle text may span multiple pages. |
| **Segment** | Legacy/UI term for a contiguous portion of audio within a phrase. Used in the context of multi-track splicing where multiple audio clips merge into one phrase. Not a separate logical unit for callback or history purposes. |
| **Completion event** | The point at which a phrase's playback (audio or text display) finishes and its callback becomes eligible for dispatch. |
| **Committed phrase** | A phrase whose completion event has occurred and whose callback (if any) has been dispatched. |
| **Current phrase** | The phrase actively playing or displaying. At most one phrase is current at any time. |
| **Replay target** | The most recently committed phrase that had associated audio or text content. This is the phrase replayed on Left input during response selection. |

---

## 1. Subsystem Purpose and Scope

The communication subsystem owns all alien-encounter dialogue presentation and interaction. This includes:

- Encounter lifecycle: entry, hail/attack decision, dialogue loop, exit, and battle segue
- Dialogue scripting: per-race script dispatch and NPC phrase emission
- Player response registration, rendering, selection, and callback dispatch
- Speech track assembly, playback coordination, and seek/replay controls
- Subtitle synchronization with the audio track and conversation-summary review
- Alien portrait animation scheduling (ambient, talking, transition, colormap)
- Oscilloscope waveform visualization during speech playback
- Segue/battle outcome determination (peace, hostile, instant victory, defeat)

**Target state**: Rust owns all of the above except the 27 race-specific dialogue-tree scripts, which remain in C and call into Rust through a stable FFI contract.

---

## 2. Encounter Lifecycle

### 2.1 Entry Points

The subsystem exposes two public entry points to the game:

| Function | Purpose |
|---|---|
| `InitCommunication(which_comm)` | Top-level encounter entry. Resolves race, builds NPC ship queue, determines hail vs. attack, and either enters dialogue or proceeds directly to combat. |
| `RaceCommunication()` | High-level caller that determines encounter context (hyperspace, interplanetary, last-battle) and delegates to `InitCommunication`. |

**Target state**: Both functions are Rust-owned. `RaceCommunication` reads game state to select the `CONVERSATION` enum variant and passes it to `InitCommunication`.

### 2.2 Encounter Setup

**Required compatibility behavior**: Encounter setup shall produce the following observable outcomes in order before dialogue begins:

1. If a saved game was just loaded, SIS display elements are updated for the current game context.
2. Drone/rebel encounter-type normalization maps variant encounter types to their base communication configuration while preserving the original ship status.
3. The `CONVERSATION` enum resolves to a ship race index. Sphere tracking begins for the race.
4. NPC fleet construction occurs for races where combat is a possibility.
5. The race-specific script initialization function is called and returns encounter communication data. The subsystem copies or retains this data for the encounter duration.
6. Hail-or-attack evaluation:
   - If `BATTLE_SEGUE` is zero: dialogue begins directly.
   - If `BATTLE_SEGUE` is nonzero: the hail/attack choice is presented. Talk clears `BATTLE_SEGUE` and enters dialogue. Attack sets `BATTLE_SEGUE = 1` and skips dialogue.

**Implementation-parity note**: The current C implementation performs these steps through `InitCommunication` calling `init_race(comm_id)` which dispatches to `init_*_comm()`, followed by conditional `InitEncounter()` / `HailAlien()` calls. The step ordering above is normative; the specific helper decomposition is not.

### 2.3 HailAlien — The Dialogue Session

`HailAlien` owns a single dialogue session from resource loading through teardown.

**Required compatibility behavior**: A dialogue session shall:

1. Load and bind the visual, text, subtitle, palette, and music resources identified by the encounter communication data before script-driven presentation begins.
2. Create dedicated graphics contexts for subtitle caching and animation rendering.
3. Invoke the encounter's `init_encounter_func()` to start the dialogue tree.
4. Enter the main dialogue loop, alternating between NPC speech playback and player response selection.
5. On normal exit: invoke `post_encounter_func()` then `uninit_encounter_func()`. This is the dialogue-session exit path for the encounter-exit callbacks defined in §2.5.
6. On abort or load interruption: skip `post_encounter_func()`, invoke `uninit_encounter_func()`, release all encounter-local resources, and leave the subsystem in a valid reusable state.

Resource teardown shall destroy all loaded resources in reverse order of creation.

### 2.4 Post-Dialogue Battle Segue

After dialogue (or after a skipped dialogue on attack), the subsystem evaluates final segue state:

- If `BATTLE_SEGUE` is set and the NPC ship queue is non-empty, build the player fleet and transfer control to encounter flow for battle execution.
- Otherwise, clear `BATTLE_SEGUE`.
- In all cases, call encounter teardown to clean up encounter resources.

### 2.5 Encounter Lifecycle Callback Rules

This section defines the exactly-once and ordering rules for each encounter lifecycle callback across all exit paths.

`post_encounter_func` is an **encounter-exit side-effect hook**. It is invoked on all non-abort exit paths from the encounter, including the attack-without-hail path (§9.3). Its purpose is to execute game-state side effects (sphere updates, alliance changes) that must occur whenever an encounter concludes, regardless of whether dialogue ran. The dialogue-session normal-exit path (§2.3 step 5) is one such non-abort exit path; the attack-without-hail path (§9.3 step 1) is another.

| Callback | Normal exit (dialogue ran) | Abort / load interruption | Attack without hail |
|---|---|---|---|
| `init_encounter_func` | Called exactly once at dialogue session start (§2.3 step 3) | Called exactly once (dialogue session was entered before abort) | **Not called** — dialogue session is never entered |
| `post_encounter_func` | Called exactly once after dialogue loop exits (§2.3 step 5) | **Not called** — skipped on abort/load (§2.3 step 6) | Called exactly once by the attack-path lifecycle (§9.3 step 1) |
| `uninit_encounter_func` | Called exactly once after `post_encounter_func` (§2.3 step 5) | Called exactly once (§2.3 step 6) | Called exactly once by the attack-path lifecycle (§9.3 step 1) |

**Required compatibility behavior**: Each lifecycle callback shall be invoked **at most once** per encounter. "Encounter teardown" (§2.4, §9.3 step 4) refers to resource cleanup and encounter-record bookkeeping — it does not re-invoke `uninit_encounter_func` or any other script lifecycle callback. The implementation shall not double-invoke any lifecycle callback regardless of exit path.

### 2.6 Post-Encounter Hyperspace Updates

When the encounter originated from a hyperspace globe collision, the encounter flow updates the encounter record with surviving ship counts, crew levels, and encounter flags (reforming, one-shot).

---

## 3. LOCDATA — The Encounter Descriptor

`LOCDATA` is the per-race encounter descriptor. Each race's `init_*_comm()` function returns a pointer to a static `LOCDATA` instance. The subsystem copies this into the active encounter communication data before use.

### 3.1 Fields

| Field | Type | Purpose | Ownership class |
|---|---|---|---|
| `init_encounter_func` | `fn()` | Called to start the dialogue tree | Borrowed C callback pointer |
| `post_encounter_func` | `fn()` | Encounter-exit side-effect hook invoked on all non-abort exit paths (§2.5) | Borrowed C callback pointer |
| `uninit_encounter_func` | `fn() -> COUNT` | Called for cleanup in all exit paths | Borrowed C callback pointer |
| `AlienFrameRes` | `RESOURCE` | Alien portrait animation spritesheet resource ID | Copied scalar metadata |
| `AlienFontRes` | `RESOURCE` | Alien subtitle font resource ID | Copied scalar metadata |
| `AlienTextFColor`, `AlienTextBColor` | `Color` | Foreground and background colors for subtitle text | Copied scalar metadata |
| `AlienTextBaseline` | `POINT` | Baseline position for subtitle rendering | Copied scalar metadata |
| `AlienTextWidth` | `COUNT` | Maximum text width (0 = use default SIS width) | Copied scalar metadata |
| `AlienTextAlign` | `TEXT_ALIGN` | Horizontal alignment for subtitles | Copied scalar metadata |
| `AlienTextValign` | `TEXT_VALIGN` | Vertical alignment for subtitles | Copied scalar metadata |
| `AlienColorMapRes` | `RESOURCE` | Colormap resource for palette animation | Copied scalar metadata |
| `AlienSongRes` | `RESOURCE` | Background music resource | Copied scalar metadata |
| `AlienAltSongRes` | `RESOURCE` | Alternate music resource (used conditionally) | Copied scalar metadata |
| `AlienSongFlags` | `LDAS_FLAGS` | Flags controlling music selection (e.g., `LDASF_USE_ALTERNATE`) | Copied scalar metadata |
| `ConversationPhrasesRes` | `RESOURCE` | String table resource containing all NPC and player phrases | Copied scalar metadata |
| `NumAnimations` | `COUNT` | Number of ambient animations (max 20) | Copied scalar metadata |
| `AlienAmbientArray` | `[ANIMATION_DESC; 20]` | Ambient animation descriptors | Copied scalar metadata |
| `AlienTransitionDesc` | `ANIMATION_DESC` | Transition animation descriptor (silent ↔ talking) | Copied scalar metadata |
| `AlienTalkDesc` | `ANIMATION_DESC` | Talking animation descriptor | Copied scalar metadata |
| `AlienNumberSpeech` | `NUMBER_SPEECH` | Optional number-speech synthesis table | Borrowed C pointer (remains valid for encounter lifetime) |
| `AlienFrame` | `FRAME` | Loaded alien portrait drawable (populated at runtime) | Runtime-loaded handle created by comm |
| `AlienFont` | `FONT` | Loaded alien font (populated at runtime) | Runtime-loaded handle created by comm |
| `AlienColorMap` | `COLORMAP` | Loaded colormap (populated at runtime) | Runtime-loaded handle created by comm |
| `AlienSong` | `MUSIC_REF` | Loaded music reference (populated at runtime) | Runtime-loaded handle created by comm |
| `ConversationPhrases` | `STRING` | Loaded string table (populated at runtime) | Runtime-loaded handle created by comm |

### 3.2 Layout Compatibility

The Rust representation requires logical field compatibility with the C `LOCDATA`, not binary layout compatibility. Rust accesses C-owned `LOCDATA` fields through FFI accessors, then stores the values in a Rust-native `CommData` struct. Resource ID fields use integer types. Loaded resource handles use opaque handle types that wrap C pointers obtained through FFI. The three callback function pointers are stored as C-compatible function pointer types.

### 3.3 LOCDATA Pointer Lifetime

Each race's `init_*_comm()` returns a pointer to a file-static `LOCDATA` instance. The pointer itself remains valid for the program's lifetime (static storage duration), but the comm subsystem copies all needed field values at initialization time and does not retain or dereference the `LOCDATA*` pointer after the copy step. Borrowed C pointers within `LOCDATA` (callback function pointers, `AlienNumberSpeech`) remain valid for the encounter lifetime because they point to static C data or static functions.

---

## 4. Dialogue Scripting and Dispatch

### 4.1 Script Dispatch

Race-specific scripts remain in C. Dispatch uses a `CONVERSATION` enum → function mapping:

```
init_race(comm_id) -> LOCDATA*
```

This function contains a match/switch over all 27 conversation variants (Arilou, Chmmr, Orz, etc.) and calls the corresponding `init_*_comm()` function. Special cases:
- `COMMANDER_CONVERSATION` delegates to either `init_commander_comm()` or `init_starbase_comm()` based on `STARBASE_AVAILABLE` game state.
- `SPATHI_CONVERSATION` delegates to either `init_spathi_comm()` or `init_spahome_comm()` based on a global flag.

The Rust subsystem calls `init_race` through FFI. The returned `LOCDATA*` is read through FFI accessors and copied into the Rust-owned `CommData`.

### 4.2 Script-Facing API Contract

Race scripts in C target the following API, which Rust exposes through FFI:

| Macro/Function | Signature | Purpose |
|---|---|---|
| `NPCPhrase(index)` | `NPCPhrase_cb(index, NULL)` | Queue an NPC speech phrase by string-table index |
| `NPCPhrase_cb(index, cb)` | `fn(i32, CallbackFunction)` | Queue phrase with a synchronous callback |
| `NPCPhrase_splice(index)` | `fn(i32)` | Queue phrase without page break |
| `NPCNumber(number, fmt)` | `fn(i32, *const c_char)` | Synthesize and queue a spoken number |
| `Response(i, a)` | `DoResponsePhrase(i, a, 0)` | Register a player response option |
| `DoResponsePhrase(R, func, str)` | `fn(RESPONSE_REF, RESPONSE_FUNC, *UNICODE)` | Register a response with optional pre-built text |
| `construct_response(buf, R, ...)` | `fn(*mut UNICODE, i32, ...)` | Build a composite response string from multiple phrase indices |
| `setSegue(segue)` | `fn(Segue)` | Set the post-conversation outcome |
| `getSegue()` | `fn() -> Segue` | Query the current outcome |
| `PLAYER_SAID(R, i)` | `R == i` | Test which response the player selected |
| `PHRASE_ENABLED(p)` | Phrase-enabled query macro | Test if a phrase has been disabled; see §11.4/§11A |
| `DISABLE_PHRASE(p)` | Phrase-disable macro | Disable a phrase; see §11.4/§11A |

### 4.3 Phrase Emission and Track Splicing

**Required compatibility behavior**: When a script calls `NPCPhrase_cb(index, cb)`, the subsystem shall:

1. Resolve the string-table index to a text string pointer, an optional audio clip handle, and an optional timestamp array from the active conversation phrases resource.
2. Handle special indices:
   - `GLOBAL_PLAYER_NAME` → commander name from game state
   - `GLOBAL_SHIP_NAME` → ship name from game state
   - Negative indices → alliance name variants with possible commander-name suffix
   - Index 0 → no-op (no output queued)
3. Queue the resolved `(clip, text, timestamp, callback)` tuple with the trackplayer via the phrase-queue operation (see §6A).

**Required compatibility behavior**: `NPCPhrase_splice(index)` shall queue the phrase without introducing a page break. If there is no audio clip, text is appended to the current track segment. If there is an audio clip, the audio is merged into the current segment without a page boundary.

### 4.4 Number Speech Synthesis

`NPCNumber(number, fmt)` synthesizes spoken numbers using the race's `AlienNumberSpeech` table:

- If the race has no number-speech table, it splices the formatted number string as text only.
- If a table exists, it recursively decomposes the number using digit descriptors (dividers, subtrahends, digit-to-phrase mappings, place-name phrases) and assembles a multi-track audio sequence via the trackplayer's multi-track merge operation.

### 4.5 Dialogue Tree Pattern

Each race script follows a consistent callback-chain pattern:

1. `init_encounter_func()` examines game state, emits opening NPC phrases, and registers initial response options.
2. Each response callback (typed as `RESPONSE_FUNC = fn(RESPONSE_REF)`) receives the selected response reference, emits NPC reply phrases, mutates game state, and registers the next set of responses (or registers none to end the conversation).
3. `ExitConversation` callbacks call `setSegue()` to determine the post-conversation outcome.

---

## 5. Response Registration and Selection

### 5.1 Registration

The response system supports up to `MAX_RESPONSES = 8` simultaneous options. When a script calls `DoResponsePhrase(R, func, str)`:

1. If `str` is provided, it is used directly as the display text.
2. If `str` is NULL, the text is resolved from the active conversation phrases resource at index `R - 1`.
3. The response reference, text, and callback function pointer are stored in the encounter state's response list.
4. The response count is incremented.

### 5.2 Display and Rendering

**Required compatibility behavior**: Response rendering shall:

- Draw text with the player font, left-aligned, in the SIS comm window area.
- Draw the currently selected response in a highlighted style; others in a dimmed style.
- If responses overflow the visible area, draw scroll indicators (up/down arrows).
- Track a `top_response` index for the first visible response for scrolling.

### 5.3 Selection and Input

**Required compatibility behavior**: During player response input, the subsystem shall process:

| Input | Action |
|---|---|
| Up/Down | Cycle selection through the response list (wrapping) |
| Select | Invoke `SelectResponse`: copy text to feedback buffer, display player's chosen text, stop current track, clear subtitles, fade music to background, clear responses, invoke the selected response's callback with its `RESPONSE_REF` |
| Cancel | Open conversation summary (unless in final battle) |
| Left | Replay the last NPC speech segment |

### 5.4 Response Callback Invocation

**Required compatibility behavior**: Response callback dispatch shall pass the response reference as an argument to the callback:

```
(*response_list[cur_response].response_func)(response_list[cur_response].response_ref)
```

The callback type is `void (*)(RESPONSE_REF)` — it receives the response reference as an argument. This allows callbacks to branch on `PLAYER_SAID(R, specific_phrase)` to determine which response was selected when a single callback handles multiple options.

The FFI response registration must accept callback pointers matching the C ABI where `RESPONSE_REF` is passed as an argument. This is a correction from the current Rust implementation.

### 5.5 Response-Callback Entry Guarantees

**Required compatibility behavior**: When a response callback is invoked (after `SelectResponse` processing), the following preconditions are guaranteed and may be relied upon by the callback:

1. **Prior response list cleared**: The response list from the previous turn has been fully cleared. No stale response state is visible.
2. **Chosen-text feedback visible**: The player's selected response text has been copied to the feedback buffer and is displayed. It remains visible until the callback (or subsequent processing) overwrites it.
3. **Track fully stopped and prior-turn completions resolved**: `StopTrack` has completed. Any pending completion associated with the prior turn's phrases has been either dispatched (callback invoked and phrase committed) or discarded by stop semantics before response-callback entry. No phrase callback from the prior turn can arrive after this point. The trackplayer is idle.
4. **Subtitle state reset**: Subtitles have been cleared. The subtitle display area is empty and ready for the next NPC turn.
5. **Music at background volume**: Background music has been faded to background volume in preparation for the next NPC speech turn.

These are guaranteed preconditions of response-callback entry, not implementation details. Callbacks may assume them.

---

## 6. Subtitle and Speech Coordination

### 6.1 Trackplayer Integration

The communication subsystem does not own audio playback directly. It delegates to the trackplayer subsystem through a defined interface. The operations below are the integration surface that the communication subsystem requires from the trackplayer:

| Operation | Trackplayer Function | Purpose |
|---|---|---|
| Queue phrase | `SpliceTrack(clip, text, timestamps, callback)` | Add a speech segment with subtitle text and timing |
| Queue multi-track | `SpliceMultiTrack(tracks[], text)` | Merge multiple audio clips as one continuous segment |
| Start playback | `PlayTrack()` | Begin playing the assembled track |
| Stop playback | `StopTrack()` | Halt playback |
| Jump to end | `JumpTrack()` | Skip to the end of the current phrase |
| Check playing | `PlayingTrack() -> COUNT` | Returns current track number (0 if stopped) |
| Forward seek (page) | `FastForward_Page()` | Advance one subtitle page |
| Forward seek (smooth) | `FastForward_Smooth()` | Continuous forward seek |
| Reverse seek (page) | `FastReverse_Page()` | Rewind one subtitle page |
| Reverse seek (smooth) | `FastReverse_Smooth()` | Continuous reverse seek |
| Get subtitle | `GetTrackSubtitle() -> *UNICODE` | Get current subtitle text pointer |
| Enumerate subtitles | `GetFirstTrackSubtitle()`, `GetNextTrackSubtitle()`, `GetTrackSubtitleText()` | Iterate all recorded subtitles for conversation summary |

**Required compatibility behavior**: The communication subsystem must integrate with the authoritative trackplayer implementation regardless of whether that boundary is Rust-to-Rust or Rust-to-C. If the trackplayer is Rust-owned, direct Rust-to-Rust calls are preferred. If the trackplayer remains C-owned or partially C-owned, the communication subsystem must integrate through whatever boundary the trackplayer exposes. The function names above reflect the current C trackplayer API; if the trackplayer's API changes, the communication subsystem must adapt to the equivalent operations.

### 6A. Trackplayer Behavioral Contract

This section defines the behavioral guarantees that the communication subsystem requires from the trackplayer, independent of API shape or implementation language.

#### 6A.1 Phrase as the Logical Unit

A **phrase** is the logical unit of speech delivered to the trackplayer by `SpliceTrack(clip, text, timestamps, callback)`. Each `SpliceTrack` call produces exactly one phrase in the trackplayer's queue. Phrases are the unit at which:

- completion callbacks fire,
- subtitle text transitions occur,
- summary history entries are recorded, and
- skip (`JumpTrack`) advances.

A **multi-track splice** (`SpliceMultiTrack`) merges multiple audio clips into a single phrase. The resulting phrase has one completion point, one subtitle entry, and one history entry — not one per clip.

A **page-break-free splice** (`NPCPhrase_splice`) appends content to the current phrase without creating a new completion boundary. The trackplayer shall treat the appended content as part of the same logical phrase for callback, subtitle, and history purposes.

#### 6A.2 Phrase Completion Event

A **phrase completion event** occurs when:

- The phrase's audio clip finishes natural playback, **or**
- The phrase has no audio clip and its text display duration has elapsed (see §6A.7), **or**
- The player skips the phrase via `JumpTrack`.

When a phrase completion event occurs, the trackplayer shall record a **pending completion** for that phrase. The trackplayer shall not invoke the phrase's callback directly. The pending completion is consumed by the comm subsystem's main-thread poll loop as described in §6A.6 and §6A.8.

When the comm subsystem dispatches a pending completion:

1. Invoke the phrase's callback (if non-NULL).
2. Only after the callback returns, commit the phrase as completed and make the next phrase's subtitle text available via `GetTrackSubtitle()`.
3. Advance `PlayingTrack()` state to reflect the next phrase (or return 0 if no more phrases remain).
4. The next queued phrase becomes the **current phrase** and begins playback.

The phrase that just completed becomes a **committed phrase**. If it had audio or text content, it becomes the **replay target**, replacing any previous replay target.

**Replay-target update rule**: The replay target shall update **only when a phrase becomes committed** (i.e., after step 2 above). The replay target shall not update when a phrase is merely queued, and shall not update when a phrase is merely marked as having a pending completion. This ensures that replay always reflects the last phrase whose callback has fully executed.

#### 6A.3 Skip and Seek Semantics

**JumpTrack** (skip): Shall advance to the end of the **current phrase only**. The current phrase's completion event is triggered. Pending phrases after the current one remain queued and play normally. JumpTrack shall not coalesce or skip multiple phrases.

**FastForward / FastReverse** (seek): Shall move the playback position within the current phrase's audio. Seeking past the end of a phrase shall trigger that phrase's completion event and advance to the next phrase. Seeking backward past the beginning of the current phrase shall remain clamped at the phrase boundary (the trackplayer does not seek backward across phrase boundaries into already-completed phrases).

**StopTrack**: Shall halt playback without firing callbacks for unplayed phrases. Pending phrase callbacks are discarded.

#### 6A.4 Subtitle History Recording

Subtitle history shall be recorded at **queue time** (when `SpliceTrack` is called), not at playback time. This means:

- Phrases that are queued but never reach natural playback (because the conversation exits or is interrupted) still appear in history.
- Phrases that are replayed via seek do not create duplicate history entries.
- Phrases that are skipped via JumpTrack appear exactly once in history.
- History ordering matches the queue order, which is the script's emission order.

The trackplayer shall provide enumeration of all recorded subtitle history entries in queue order via the subtitle enumeration API.

#### 6A.5 Replay Semantics

When the communication subsystem requests replay of the last NPC speech segment (Left input during response selection), the trackplayer shall replay the **replay target** — the most recently committed phrase that had associated audio or text content.

The replay target is updated each time a phrase with content becomes committed (see §6A.2 replay-target update rule). If a callback queues multiple follow-on phrases and all complete and are committed before the next response phase, the replay target is the last of those committed phrases. If no phrase with content has been committed in the encounter, replay is a no-op.

Replay does not alter subtitle history. The replayed phrase's callback does not re-fire during replay.

#### 6A.6 Callback Delivery Threading

The trackplayer shall not invoke phrase callbacks directly on the audio thread. Phrase completion is signaled from the audio thread, but callback invocation occurs on the main thread during the comm subsystem's poll loop (see §12.2). The trackplayer shall provide a mechanism for the main thread to detect pending phrase completions and retrieve the associated callback.

#### 6A.7 Text-Only Phrase Completion Timing

When a phrase has no audio clip, its completion timing is governed by the trackplayer's text-display-duration rule. The authoritative definition of text-only phrase timing is in `audio-heart/specification.md` §8.6, which defines per-page subtitle timing as `max(character_count * TEXT_SPEED, 1000)` ms, with explicit timestamps overriding when supplied. The comm subsystem's obligation is to use whatever timing the trackplayer provides and not to override or second-guess it. Audio-heart is the normative owner of this timing formula; comm consumes it.

#### 6A.8 Pending-Completion Ownership and Phrase Advancement

This section defines the precise handoff between the trackplayer and the comm subsystem for phrase completion dispatch.

**Required compatibility behavior**: The following sequence is normative:

1. The trackplayer detects that the current phrase's playback has ended (audio finished, text duration elapsed, or skip). It records a **pending completion** consisting of the phrase's callback pointer and any associated metadata. The current phrase remains "current" in the trackplayer's state; `PlayingTrack()` still reflects it.
2. The comm subsystem's main-thread poll loop calls `PollPendingTrackCompletion()` (see `audio-heart/specification.md` §8.3.1) to atomically claim and clear the pending completion.
3. If a completion was claimed, the comm subsystem invokes the callback (if non-NULL) on the main thread.
4. After the callback returns, the comm subsystem calls `CommitTrackAdvancement()` (see `audio-heart/specification.md` §8.3.1) to signal the trackplayer to advance: the completed phrase becomes committed, the next queued phrase becomes current, `PlayingTrack()` is updated, and the next phrase's subtitle becomes available via `GetTrackSubtitle()`.

The key invariant is: **the next phrase does not become current until after the completed phrase's callback has returned.** This ensures callbacks observe a consistent state where the just-finished phrase is still "current" and any phrases they queue are appended after the existing queue.

**Pending-completion cross-boundary state machine:** The following state transitions and race outcomes are normative for the comm↔trackplayer seam, regardless of API shape:

- **Atomicity:** The pending completion is a single-slot resource. At most one pending completion exists at a time. The trackplayer shall not record a second pending completion before the first is claimed and cleared by the main thread.
- **Claim-and-clear:** The main thread's retrieval of the pending completion shall atomically claim and clear it. After claim-and-clear, the trackplayer shall observe no pending completion until it records a new one.
- **Advancement commit:** The main thread's signal to advance (step 5) shall be a distinct operation from claim-and-clear (step 3). The trackplayer shall not advance the phrase queue until the advancement commit is received.
- **StopTrack interaction:** If `StopTrack` is called while a pending completion exists but has not yet been claimed, the pending completion is discarded. If `StopTrack` is called after claim-and-clear but before the callback returns, the callback shall still complete normally, but the advancement commit (step 5) becomes a no-op because the track is already stopped.
- **Seek interaction:** If a seek operation (`JumpTrack`, `FastForward`, `FastReverse`) triggers a new phrase boundary crossing while a pending completion exists but has not yet been claimed, the new completion shall be deferred until the existing completion is claimed and its advancement commit is processed. The original pending completion is preserved and must be consumed by the main-thread poll loop before a new completion can be recorded. This ensures no phrase callback is lost during seek races.
- **Multiple completions:** Because the pending completion is single-slot and the main thread must claim-and-clear before the trackplayer records the next, at most one callback dispatch occurs per poll-loop iteration. The main thread shall re-poll after each dispatch to handle any subsequent completion.

### 6.2 Subtitle Display

Subtitles are polled each frame from the trackplayer via `GetTrackSubtitle()`:

1. Compare the returned string pointer and baseline/alignment against the previously displayed subtitle.
2. If changed, mark subtitles for clearing and update the stored text, baseline, and alignment.
3. On redraw, render the subtitle text into the comm window area using the alien font, foreground color, baseline, and alignment from `CommData`.

Subtitle rendering uses a text-cache context: a separate offscreen pixmap with a transparent background color key. This allows overlaying subtitle text on top of the animation frame without full redraws.

### 6.3 Subtitle Enable/Disable

Subtitles can be globally enabled or disabled via the `optSubtitles` setting. When disabled, subtitle rendering is suppressed but speech audio still plays.

### 6.4 Conversation Summary

The summary view allows the player to review all NPC dialogue from the current encounter by pressing Cancel during response selection:

1. Enumerate all subtitles from the trackplayer using the subtitle enumeration API.
2. Render subtitle text page by page in a dedicated summary font, with word wrapping to fit the comm window width.
3. Navigation: Select/Cancel/Right advances to the next page; when no more pages remain, the summary view closes and returns to response selection.
4. If text from a single subtitle spans multiple lines across a page boundary, the overflow is carried to the next page.

### 6.5 Subtitle History Scope and Retention

- Subtitle history scope is per encounter. History is cleared when the encounter ends.
- The summary is sourced from the original dialogue queue order, not from observed playback events: lines that were skipped or replayed appear exactly once in the summary.
- Replay and seek operations do not alter the summary history contents.

---

## 7. The Talk Segue — Speech Playback Loop

### 7.1 TalkSegue Behavior

The talk segue manages the core speech-playback interaction loop. It is entered after the script has queued NPC phrases.

**Required compatibility behavior**: The talk segue shall:

1. **Transition to talking** — If the race has a talking animation and is not already in talking state, trigger the intro/transition animation and the talking animation. Wait for the transition to complete before showing talking animation.
2. **Playback loop** — While speech is playing:
   - Check for abort condition.
   - Cancel: skip to end of track, mark ended.
   - Left/Right: engage seek mode (smooth or page-based depending on `optSmoothScroll` setting).
   - When not seeking: poll subtitles.
   - Update animations (paused during seeking — original 3DO behavior).
   - Update speech graphics (oscilloscope and slider).
   - Check if track has finished playing.
   - Sleep until next frame time (1/60 second for smooth seek support).
3. **Post-playback** — Clear subtitles. Set the slider to the STOP icon. Transition back to silent animation state.

**Implementation-parity note**: The current C implementation structures this as `DoTalkSegue` (the input handler) called via the `DoInput` loop, with `TalkSegue(wait_track)` as the outer wrapper. The behavioral requirements above are normative; the specific function decomposition is not.

### 7.2 AlienTalkSegue Wrapper

**Required compatibility behavior**: The first talk segue call in an encounter shall perform one-time initialization:

- Initialize speech graphics (oscilloscope frame, slider), set the colormap, draw the initial alien frame.
- Perform the intro transition (fade-in, crossfade, or immediate based on `curIntroMode`).
- Start background music at background volume.
- Initialize comm animations.

After playback completes on any call, fade music to foreground volume.

### 7.3 DoCommunication Main Loop

**Required compatibility behavior**: The main comm state machine alternates between two phases:

1. **NPC talking phase** — If talking is not finished, enter the talk segue and return (continuing the input loop).
2. **Player response phase** — Once talking finishes:
   - If there are no responses registered, present a timeout with replay capability, then exit the conversation.
   - If responses are registered, enter player response input for selection.

---

## 8. Animation and Oscilloscope Integration

### 8.1 Animation System Architecture

The animation system manages three categories of animation simultaneously during an encounter:

| Category | Max Count | Source | Behavior |
|---|---|---|---|
| Ambient | Up to 20 | `CommData.AlienAmbientArray[]` | Background animations (blinking, breathing, environment) |
| Transition | 1 | `CommData.AlienTransitionDesc` | Plays when entering or exiting talking state |
| Talk | 1 | `CommData.AlienTalkDesc` | Active during speech playback |

### 8.2 ANIMATION_DESC

Each animation is described by:

| Field | Purpose |
|---|---|
| `StartIndex` | First frame index in the spritesheet (or first colormap index) |
| `NumFrames` | Total frames in the animation |
| `AnimFlags` | Animation type flags + behavioral flags |
| `BaseFrameRate` | Minimum ticks between frame advances |
| `RandomFrameRate` | Random additional ticks (actual delay = Base + random(0..Random)) |
| `BaseRestartRate` | Minimum ticks before restarting after completion |
| `RandomRestartRate` | Random additional restart delay |
| `BlockMask` | Bitmask of animation indices that cannot run concurrently |

### 8.3 Animation Types

| Flag | Behavior | Neutral Frame |
|---|---|---|
| `RANDOM_ANIM` | Each frame is randomly selected | First frame |
| `CIRCULAR_ANIM` | Frames cycle in order, then restart | Last frame |
| `YOYO_ANIM` | Frames cycle forward then backward | First frame |
| (none) | Static frame, no animation | — |
| `COLORXFORM_ANIM` | Palette/colormap animation rather than sprite animation | — |

### 8.4 Animation Behavioral Flags

| Flag | Context | Effect |
|---|---|---|
| `WAIT_TALKING` | On `AlienTalkDesc` | Set when talking animation is active/should be active |
| `WAIT_TALKING` | On ambient | This ambient pauses at end of cycle when talking is active |
| `PAUSE_TALKING` | On `AlienTalkDesc` | Suppresses talking animation (script-controlled) |
| `TALK_INTRO` | On `AlienTransitionDesc` | Transition is playing toward talking state |
| `TALK_DONE` | On `AlienTransitionDesc` / `AlienTalkDesc` | Transition/talking is ending |
| `ANIM_DISABLED` | Any | Animation is currently disabled |
| `ONE_SHOT_ANIM` | On ambient | Animation runs once then auto-disables |

### 8.5 Animation Scheduling

**Required compatibility behavior**: The animation scheduler shall run at `COMM_ANIM_RATE` (ONE_SECOND/40 = 40 FPS effective rate) and shall:

1. For each active sequence, check if its alarm timer has expired.
2. Advance the frame index according to the animation type.
3. Enforce `BlockMask` mutual exclusion: if an animation's block mask conflicts with currently active animations, defer it.
4. For `WAIT_TALKING` ambient animations, stop at the neutral frame when talking is active.
5. Apply frame changes to the portrait by drawing the corresponding sprite or applying the colormap transform.
6. Track changes for subtitle redraw optimization — subtitles are only redrawn when animation frames change or when the subtitle text itself changes.

### 8.6 Talking/Transition Animation Control

The comm orchestration code controls talking animations through query and control operations with the following observable effects:

| Operation | Observable Effect |
|---|---|
| Query: wants talking animation | True when `WAIT_TALKING` flag is set on `AlienTalkDesc` |
| Query: has talking animation | True when `AlienTalkDesc.NumFrames > 0` |
| Query: has transition animation | True when `AlienTransitionDesc.NumFrames > 0` |
| Set: run intro animation | Enable the transition animation toward talking state |
| Set: run talking animation | Enable the talking animation |
| Query: intro animation running | True while the intro transition is still playing |
| Query: talking animation running | True while the talking animation is active |
| Set: stop talking animation | Begin transitioning back to silent state |
| Set: enable/disable talking animation | Set or clear `PAUSE_TALKING` on `AlienTalkDesc` (script-callable via `EnableTalkingAnim`) |

### 8.7 Oscilloscope

The oscilloscope displays a real-time audio waveform during NPC speech in the radar area of the screen:

1. **Sample ingestion** — The audio stream engine feeds PCM samples to the oscilloscope via a callback or polling interface.
2. **Display update** — At `OSCILLOSCOPE_RATE` (ONE_SECOND/32), the oscilloscope resamples the buffer to display width, applies peak normalization with decay, and maps sample amplitudes to Y coordinates.
3. **Rendering** — The oscilloscope is drawn into `RadarContext` using a background frame asset. Each column maps to one display-buffer entry.
4. **Lifecycle** — Activated on encounter init, deactivated on encounter teardown. Cleared when speech stops.

### 8.8 Slider

A progress slider shows playback position within the current track:

- Initialized with background/foreground frame assets from `ActivityFrame`.
- Updated at the same rate as the oscilloscope.
- Icons change to indicate state: playing, seeking forward, seeking backward, stopped.

---

## 9. Segue and Battle Outcomes

### 9.1 Segue Values

| Segue | `BATTLE_SEGUE` State | Additional Effects |
|---|---|---|
| `Segue_peace` | 0 | No combat after dialogue |
| `Segue_hostile` | 1 | Combat follows dialogue |
| `Segue_victory` | 1 + `instantVictory = TRUE` | Instant victory (no actual combat) |
| `Segue_defeat` | 0 + crew = ~0, `CHECK_RESTART` | Game over |

### 9.2 Script Usage

Scripts call `setSegue()` in their exit paths (typically in `ExitConversation`). The choice depends on dialogue outcomes — e.g., the player insulting an alien may trigger `Segue_hostile`, while a successful negotiation results in `Segue_peace`.

### 9.3 Encounter Flow Integration

After the dialogue session returns (or after the hail/attack decision skips dialogue):

**Normal exit (dialogue ran)**:

`post_encounter_func` and `uninit_encounter_func` have already been called within `HailAlien` (§2.3 step 5). Continue to step 2 below.

**Attack without hail (dialogue never entered)**:

1. Call `post_encounter_func()` then `uninit_encounter_func()` on the encounter communication data. This is the only invocation of these callbacks for this encounter. `init_encounter_func` was never called because the dialogue session was never entered.

    **Required compatibility behavior**: `post_encounter_func` is an encounter-exit side-effect hook (§2.5) invoked on all non-abort exit paths, including the attack-without-hail path. In current scripts, `post_encounter_func` performs game-state side effects (sphere updates, alliance changes) that must execute regardless of whether dialogue occurred. The attack-without-hail path is a non-abort exit path and therefore a valid invocation context for `post_encounter_func`; it is not a special-case exception to an otherwise dialogue-only rule.

**Common steps (all paths)**:

2. Check `BATTLE_SEGUE` and NPC ship queue. If both indicate combat, build the player fleet and call encounter battle.
3. Clear `BATTLE_SEGUE` if no combat occurs.
4. Call **encounter teardown** — this performs resource cleanup and encounter-record bookkeeping. It does **not** re-invoke `uninit_encounter_func` or any other script lifecycle callback. See §2.5 for the exactly-once callback table.

### 9.4 Segue Side-Effect Requirements

**Required compatibility behavior**: The following side effects must be preserved by the segue system:

- **Instant victory** (`Segue_victory`): Sets `BATTLE_SEGUE` to 1 and sets `instantVictory = TRUE`. The encounter flow path must recognize this state before battle entry and resolve the encounter as a victory without entering actual combat.
- **Defeat** (`Segue_defeat`): Sets crew to `~0` (maximum) as a sentinel and triggers `CHECK_RESTART`. The encounter teardown path must recognize this state and transition to the game-over flow.
- **Ordering**: Segue state mutations performed by `setSegue()` must be complete before the encounter exits the dialogue session and before any post-encounter hooks or teardown inspects segue state.

---

## 10. Ownership Model

### 10.1 Target Rust-Owned Components

| Component | Description |
|---|---|
| Communication state | Global communication state with safe concurrent access, holding all subsystem state |
| Encounter lifecycle | Entry, setup, dialogue session, resource loading/teardown |
| Comm state machine | Main dialogue loop, talk segue, speech playback loop |
| Response system | Registration, rendering, selection, callback dispatch |
| Subtitle system | Subtitle polling, clearing, redrawing, conversation summary |
| Animation engine | Initialization, scheduling, sequence processing, frame drawing |
| Oscilloscope | Sample buffer, display computation, rendering |
| Slider | Playback position indicator |
| Speech graphics | Oscilloscope and slider initialization and per-frame update |
| Glue layer | Phrase emission, number speech, response construction, segue accessors, race dispatch |
| FFI surface | All C-callable exports for race scripts and the game loop |

### 10.2 C-Owned Components (Remain in C)

| Component | Description |
|---|---|
| Race scripts | 27 `init_*_comm()` functions and their dialogue-tree callback functions under `sc2/src/uqm/comm/*/` |
| `LOCDATA` statics | Each race's static `LOCDATA` descriptor struct |
| Game state access | `GET_GAME_STATE`, `SET_GAME_STATE`, `GLOBAL_SIS`, etc. (accessed by scripts through existing macros) |

### 10.3 Shared-Ownership Boundaries

| Resource | C Side | Rust Side |
|---|---|---|
| `CommData` | Race scripts populate `LOCDATA`, C runtime provides resource handles after loading | Rust copies and owns the active `CommData` during an encounter |
| `ConversationPhrases` | C scripts index into it via macros | Rust loads the string table and owns the loaded resource handle for the encounter duration; C scripts access phrase content through the API in §11.2 |
| Graphics contexts | C `CONTEXT` / `FRAME` types from the graphics subsystem | Rust uses opaque handles obtained through graphics FFI |
| Audio track | C scripts call `NPCPhrase` which routes to the trackplayer | Trackplayer is called by comm; the integration path depends on whether the trackplayer is Rust-owned or C-owned (see §6.1) |

---

## 11. Compatibility Boundary for C Race Scripts

This section defines the compatibility contract between the Rust-owned communication subsystem and the 27 C race scripts that remain in C. This is the most migration-critical boundary in the subsystem.

### 11.1 Source Compatibility Goal

The target is source compatibility for all 27 race scripts. Scripts shall continue to compile against the same header declarations and macro definitions they use today. ABI compatibility (binary layout match) is not required for `LOCDATA` or internal comm structures; the Rust side accesses C structures through FFI accessors, not by sharing memory layout.

### 11.2 APIs and Macros That Remain Source-Compatible

The following APIs/macros shall remain available to C scripts with their current signatures and source-level behavior:

| API/Macro | Current form | Compatibility rule |
|---|---|---|
| `NPCPhrase(index)` | Macro expanding to `NPCPhrase_cb(index, NULL)` | Macro definition unchanged; underlying function may be re-routed to Rust |
| `NPCPhrase_cb(index, cb)` | `void NPCPhrase_cb(RESPONSE_REF, CallbackFunction)` | Signature unchanged; implementation becomes Rust FFI export |
| `NPCPhrase_splice(index)` | `void NPCPhrase_splice(RESPONSE_REF)` | Signature unchanged |
| `NPCNumber(number, fmt)` | `void NPCNumber(int, const char *)` | Signature unchanged |
| `Response(i, a)` | Macro expanding to `DoResponsePhrase(i, a, 0)` | Macro definition unchanged |
| `DoResponsePhrase(R, func, str)` | `void DoResponsePhrase(RESPONSE_REF, RESPONSE_FUNC, UNICODE *)` | Signature unchanged |
| `construct_response(buf, R, ...)` | `void construct_response(UNICODE *, int, ...)` | Signature unchanged |
| `setSegue(segue)` / `getSegue()` | `void setSegue(Segue)` / `Segue getSegue(void)` | Signature unchanged |
| `PLAYER_SAID(R, i)` | Macro `(R == i)` | Macro definition unchanged |
| `PHRASE_ENABLED(p)` | Phrase-enabled query macro | Behavioral semantics defined in §11.4 / §11A |
| `DISABLE_PHRASE(p)` | Phrase-disable macro | Behavioral semantics defined in §11.4 / §11A |
| `EnableTalkingAnim(enable)` | `void EnableTalkingAnim(BOOLEAN)` | Signature unchanged |
| `GET_GAME_STATE` / `SET_GAME_STATE` | Existing macros | Unchanged; these are outside comm scope |
| `GLOBAL_SIS(field)` | Existing macro | Unchanged; outside comm scope |

### 11.3 APIs That Become Wrapper Functions

In the target state, the C function bodies in `commglue.c` are replaced by thin wrappers that forward to Rust FFI exports. The C header declarations remain unchanged. Scripts do not need to know whether the underlying implementation is C or Rust.

`init_race` dispatch may remain as a C function that calls into the 27 C scripts, or may be replaced by Rust-driven dispatch that calls each `init_*_comm()` through FFI. In either case, each race's `init_*_comm()` function is called with its current signature and returns a `LOCDATA*` as it does today.

### 11.4 Phrase Disable/Enable Compatibility

**Current behavior**: `DISABLE_PHRASE(p)` disables a phrase by mutating the string table in place (setting the first byte of the phrase string to NUL). `PHRASE_ENABLED(p)` checks whether the first byte is NUL.

**Target behavioral contract**: The target is a **narrower semantic model** that intentionally departs from the legacy in-place NUL-mutation mechanism while preserving all script-observable behavior. Specifically:

- `DISABLE_PHRASE(p)` marks phrase `p` as disabled.
- `PHRASE_ENABLED(p)` returns false for any disabled phrase, true otherwise.
- `NPCPhrase(p)` called on a disabled phrase still resolves the **original, unmodified** phrase text and audio and queues them normally. Disablement is a gating mechanism for script control flow, not a suppression of phrase output.
- Response text resolution (§5.1 step 2) on a disabled phrase's index yields the **original, unmodified** phrase text.

**Compatibility justification**: This departs from the legacy behavior where `DISABLE_PHRASE` mutates the string table's first byte to NUL, which would cause `NPCPhrase` on a disabled phrase to resolve a NUL-prefixed (effectively empty) string. The narrower semantic model is acceptable because:

1. All 27 race scripts gate `NPCPhrase` calls on `PHRASE_ENABLED` checks — no script calls `NPCPhrase` on a phrase it has already disabled.
2. No script uses a disabled phrase's index for response text resolution.
3. No script or helper path depends on observing the NUL-prefixed text content of a disabled phrase.

These invariants shall be verified by static analysis or manual audit of all 27 scripts before implementation. If any script violates these invariants, the implementation must either preserve the legacy NUL-mutation effect for that path or the script must be corrected.

Scripts shall continue to use `PHRASE_ENABLED(p)` and `DISABLE_PHRASE(p)` unchanged at source level. Phrase identity and indexing shall remain exactly aligned with existing string-table index conventions.

See §11A for the complete phrase-state lifecycle semantics.

### 11.5 C Globals That Disappear in Target State

In the target state, the following C-side globals are no longer authoritative:

| Current C global | Replacement |
|---|---|
| `CommData` (C global struct) | Rust-owned `CommData`, accessed by C scripts only through FFI calls (the script-facing API abstracts this) |
| Response list/count in `comm.c` | Rust-owned response state |
| Subtitle tracking state in `comm.c` | Rust-owned subtitle state |
| Animation scheduling state in `commanim.c` | Rust-owned animation state |
| `TalkingFinished`, intro mode, fade time | Rust-owned session flags |

Scripts do not directly access these globals today (they use the API in §11.2), so their removal does not affect script source compatibility.

### 11.6 Session State Visibility

C race scripts access encounter state exclusively through the API contract in §11.2 and through `GET_GAME_STATE`/`SET_GAME_STATE` for persistent game variables. They do not directly read or write comm session state globals. The Rust-owned session state is therefore not directly visible to scripts, and no additional accessor surface is required beyond the existing API.

### 11.7 Callback Hosting

Rust invokes C script callbacks (the three `LOCDATA` function pointers and response callbacks) through FFI. During callback execution, the rules in §12 apply.

---

## 11A. Phrase-State Lifecycle and Semantics

This section defines the normative lifecycle and behavioral rules for phrase enable/disable state.

### 11A.1 Scope of Phrase State

Phrase state (enabled or disabled) is **encounter-local**. Each encounter begins with all phrases in their enabled state as loaded from the phrase resource. Phrase state is not inherited from a previous encounter, is not persisted across encounters, and is not saved or restored by save/load operations.

### 11A.2 Reset Point

Phrase state shall be reset to the initial enabled state exactly at the point when the conversation phrases resource is loaded for a new encounter — that is, at encounter resource loading time within the dialogue session setup (§2.3 step 1). There is no broader or earlier lifecycle point that resets phrase state.

### 11A.3 Effect of Disablement on Phrase Resolution

Disabling a phrase affects **only** `PHRASE_ENABLED(p)` checks. It does not remove the phrase from the string table, does not alter the phrase's text content for resolution purposes, and does not affect audio clip resolution. Specifically:

- `PHRASE_ENABLED(p)` shall return false for a disabled phrase.
- `NPCPhrase(p)` called on a disabled phrase still resolves and queues the phrase normally, using the original unmodified text and audio. Disablement is a gating mechanism for script-level control flow, not a suppression of phrase output. Scripts are responsible for checking `PHRASE_ENABLED` before calling `NPCPhrase`.
- Response text resolution (§5.1 step 2) on a disabled phrase's index yields the original, unmodified phrase text.

See §11.4 for the compatibility justification of this narrower semantic model relative to the legacy NUL-mutation behavior.

### 11A.4 Save/Load Boundary Behavior

Phrase state is transient encounter-local state. It is not saved. If a game load occurs during an encounter, the loaded encounter (if any) will re-initialize phrase state from a fresh resource load. No stale phrase state from the interrupted encounter persists after load.

### 11A.5 Disablement Durability Within an Encounter

Once a phrase is disabled via `DISABLE_PHRASE(p)`, it remains disabled for the remainder of that encounter's dialogue session. There is no API to re-enable a disabled phrase. The only reset is encounter teardown and a subsequent fresh encounter.

---

## 12. Callback Ordering and Reentrancy Semantics

### 12.1 Callback Trigger Points

| Callback type | Trigger point |
|---|---|
| `NPCPhrase_cb` callback (`CallbackFunction`) | Fires when the associated phrase's audio clip finishes playback, or after the text display duration elapses if there is no audio clip (see §6A.7). Fires once per queued phrase, not at splice boundaries within a phrase. |
| Response callback (`RESPONSE_FUNC`) | Fires synchronously when the player confirms a response selection, after `SelectResponse` processing completes (see §5.5 for guaranteed preconditions). |
| `init_encounter_func` | Called once at the start of the dialogue session, before the first main-loop iteration. |
| `post_encounter_func` | Called once on encounter exit via any non-abort path: after the dialogue loop exits normally (§2.3 step 5), or on the attack-without-hail path (§9.3 step 1). Not called on abort or load. See §2.5. |
| `uninit_encounter_func` | Called exactly once per encounter on all exit paths (§2.5). |

### 12.2 Callback Thread Affinity

All callbacks execute synchronously on the main game thread. No callbacks execute on the audio thread. Track callbacks (`CallbackFunction` queued by `SpliceTrack`) are delivered on the main thread during the talk-segue poll loop: the main thread checks for completed phrases each iteration and invokes pending callbacks before continuing. The audio thread signals phrase completion but does not invoke callbacks directly.

### 12.3 Reentrancy Rules

**Required compatibility behavior**: The following reentrancy rules are normative:

1. **Permitted operations during callbacks**: Callbacks may call any of the following: `NPCPhrase`, `NPCPhrase_cb`, `NPCPhrase_splice`, `NPCNumber`, `Response`, `DoResponsePhrase`, `construct_response`, `setSegue`, `getSegue`, `DISABLE_PHRASE`, `PHRASE_ENABLED`, `EnableTalkingAnim`, `GET_GAME_STATE`, `SET_GAME_STATE`, `GLOBAL_SIS`.

2. **Prohibited operations during callbacks**: Callbacks shall not trigger nested response-selection input or nested talk-segue entry. Callbacks shall not call `StopTrack`, `PlayTrack`, or other trackplayer control operations directly. Callbacks shall not invoke encounter teardown or lifecycle transitions.

3. **Deferred effect rule**: Phrase queuing and response registration performed during a callback take effect only after the callback returns and the main comm loop resumes its next iteration. The callback itself shall not observe the side effects of its own queuing operations (e.g., a phrase queued during a callback does not begin playback during that callback).

4. **No nested dispatch**: The comm subsystem shall not dispatch a second callback while a first callback is executing. Callbacks execute strictly sequentially.

5. **Mixed queuing in a single callback**: A callback may queue both new NPC phrases (via `NPCPhrase` / `NPCPhrase_cb`) and register new response options (via `Response` / `DoResponsePhrase`) in the same callback invocation. After the callback returns, the comm loop shall process the queued phrases first (entering the talk segue), and then present the registered responses after all queued phrases complete. This is the standard dialogue-turn pattern.

### 12.4 Ordering Between Callback Completion and State Transitions

The following ordering is required:

1. A phrase callback fires → callback completes → the next queued phrase begins playback.
2. All queued phrases finish (or are skipped) → subtitles are cleared → `TalkingFinished` becomes true → talking animation transitions to silent → response selection becomes available.
3. A response callback fires → callback completes → if new phrases were queued, `TalkingFinished` is cleared and the talk segue resumes; if no phrases were queued and no responses were registered, the conversation exits.

Segue state mutations made by a callback are visible to all subsequent encounter flow checks. A callback that mutates segue state after audio completion but before response selection will have its mutation honored.

### 12.5 Lock Discipline Across the C/Rust Boundary

**Required compatibility behavior**: The Rust implementation shall observe the following lock discipline when invoking C callbacks through FFI:

1. **No exclusive comm-state locks held during callback execution.** Before invoking any C callback (phrase callback, response callback, lifecycle callback), the Rust subsystem shall release any exclusive lock or mutable borrow on the global communication state. The callback will re-enter the comm API (e.g., `NPCPhrase`, `DoResponsePhrase`), and those API calls must be able to acquire the comm state without deadlock.

2. **Staged mutation pattern.** If the implementation uses interior mutability or a lock-protected state, one of these strategies shall be used:
   - **Release-and-reacquire**: Release the lock before the FFI callback call, reacquire after the callback returns, then apply any deferred state changes.
   - **Staging buffer**: Collect mutations (queued phrases, registered responses, segue changes) into a thread-local or callback-local staging buffer that does not require the comm state lock. After the callback returns, acquire the lock and apply staged mutations.
   - **Lock-free interior fields**: Use atomic or cell-based fields for the specific state that callbacks may mutate, avoiding lock contention entirely.

   The implementation may choose any of these (or an equivalent strategy) provided no deadlock or data race is possible when a C callback re-enters the Rust comm API.

3. **Immutable reads during callbacks.** Callbacks that only read comm state (e.g., `PHRASE_ENABLED`, `getSegue`, `GET_GAME_STATE`) shall not be blocked by any lock held by the callback's caller.

4. **No callback from audio thread into comm locks.** The audio thread shall never acquire the comm state lock. Audio-thread signals (phrase completion, sample delivery) shall use lock-free signaling mechanisms (atomic flags, lock-free queues) that the main thread polls.

---

## 13. Integration Points

### 13.1 Trackplayer

The communication subsystem depends on the trackplayer for all speech playback. The required integration operations are listed in §6.1. The behavioral contract the comm subsystem requires from the trackplayer is defined in §6A.

**Required compatibility behavior**: The communication subsystem must integrate with the authoritative trackplayer regardless of its implementation language. If both are Rust-owned, direct module calls are preferred. If the trackplayer is C-owned or mixed, FFI calls are acceptable. The comm subsystem shall not assume any specific trackplayer implementation language or ownership state.

### 13.2 Graphics Subsystem

The communication subsystem uses the graphics subsystem for all rendering:

| Comm Operation | Graphics Dependency |
|---|---|
| Animation frame drawing | `DrawStamp`, `BatchGraphics`/`UnbatchGraphics`, colormap application |
| Subtitle rendering | `font_DrawText`, `SetContextFont`, `SetContextForeGroundColor` |
| Response rendering | `add_text`, `DrawSISComWindow`, `DrawStamp` |
| Oscilloscope drawing | `DrawOscilloscope` into `RadarContext` |
| Context management | `CreateContext`, `SetContext`, `SetContextClipRect`, `SetContextFGFrame` |
| Frame/drawable lifecycle | `CaptureDrawable`, `LoadGraphic`, `CreateDrawable`, `DestroyDrawable`, `ReleaseDrawable` |
| Colormap lifecycle | `CaptureColorMap`, `LoadColorMap`, `DestroyColorMap`, `ReleaseColorMap`, `SetColorMap`, `FlushColorXForms` |
| Font lifecycle | `LoadFont`, `DestroyFont` |
| Screen transitions | `SetTransitionSource`, `ScreenTransition`, `UnbatchGraphics` |

These are consumed through FFI calls to the graphics subsystem (which itself has a partial Rust port with the backend in Rust and drawing primitives still in C).

### 13.3 Game State and Encounter Flow

| Dependency | Access Pattern |
|---|---|
| `GLOBAL(CurrentActivity)` | Read/write for abort, load, activity type checks |
| `GLOBAL_SIS(*)` | Read for commander name, ship name, crew, planet name |
| `GET_GAME_STATE` / `SET_GAME_STATE` | Read/write by scripts for persistent dialogue-tree state |
| `LastActivity` | Read/write for load-game detection and spurious-input prevention |
| `BuildBattle` / `EncounterBattle` / `InitEncounter` / `UninitEncounter` | Called for combat setup/execution |
| `StartSphereTracking` | Called to begin sphere-of-influence tracking for the encountered race |

### 13.4 Input System

| Operation | Input API |
|---|---|
| Response navigation | `PulsedInputState.menu[KEY_MENU_*]` for up/down/select/cancel/left/right |
| Seek controls | `CurrentInputState.menu[KEY_MENU_*]` for 3DO-style smooth scrolling |
| Menu sounds | `SetMenuSounds()` for feedback on response navigation |
| DoInput loop | `DoInput(&state, FALSE)` — standard input-loop driver |

### 13.5 Thread Model

The communication subsystem runs on the main game thread (`Starcon2Main`). The audio stream engine runs on the audio thread. The integration boundary is:

- **Audio thread → comm**: Oscilloscope sample delivery (the stream engine pushes samples; the comm subsystem reads them safely).
- **Audio thread → comm**: Track completion signaling. The audio thread signals when a phrase's audio finishes; the main thread detects this signal during the talk-segue poll loop and invokes the corresponding callback (see §12.2).
- **Comm → trackplayer**: All trackplayer calls from comm are made on the main thread.

---

## 14. FFI Surface Specification

The prototypes below are informational examples of the FFI surface shape. They are **non-normative**; the authoritative contract is the behavioral specification in the preceding sections. The implementation may adjust signatures as needed provided the behavioral contract is preserved. These examples are included solely to illustrate the expected integration shape and to assist reviewers; they do not constrain implementation.

### 14.1 Initialization and Lifecycle

```c
int    rust_InitCommunication(void);
void   rust_UninitCommunication(void);
int    rust_IsCommInitialized(void);
void   rust_ClearCommunication(void);
```

### 14.2 Track Management

```c
int    rust_StartTrack(void);
void   rust_StopTrack(void);
void   rust_RewindTrack(void);
void   rust_JumpTrack(void);
void   rust_SeekTrack(float position);
float  rust_CommitTrack(void);
int    rust_WaitTrack(void);
float  rust_GetTrackPosition(void);
float  rust_GetTrackLength(void);

/* PhraseCallback is the phrase-completion callback type associated with
   NPCPhrase_cb (CallbackFunction in the script-facing contract, §4.2).
   It is NOT the response callback type (RESPONSE_FUNC). */
typedef void (*PhraseCallback)(void);

void   rust_SpliceTrack(unsigned int audio_handle, const char *text,
                         const float *timestamps, unsigned int timestamp_count,
                         PhraseCallback callback);
void   rust_SpliceMultiTrack(const unsigned int *audio_handles,
                              unsigned int handle_count, const char *text);
void   rust_ClearTrack(void);
```

**Note**: `rust_JumpTrack` takes no arguments — it is a current-phrase skip operation, not an offset-based seek. `rust_SpliceTrack` accepts the full `(clip, text, timestamps, callback)` tuple consistent with the normative §6A.1 contract. The `PhraseCallback` typedef is the phrase-completion callback shape from the script-facing contract (`CallbackFunction` in §4.2); it is distinct from the response callback type (`RESPONSE_FUNC = void (*)(RESPONSE_REF)` in §5.4 / §14.4).

### 14.3 Subtitle Management

```c
const char *rust_GetSubtitle(void);
void   rust_SetSubtitlesEnabled(int enabled);
int    rust_AreSubtitlesEnabled(void);
```

**Safety**: `rust_GetSubtitle` returns a pointer that is valid until the next call into the comm subsystem that may modify subtitle state. C callers must copy or consume the string before making further comm API calls. This is the **intended corrected contract** — see the initial-state document for the current lock-lifetime hazard.

**Pre-init and post-teardown contract:** Before the audio subsystem is initialized (i.e., before `init_stream_decoder()` has been called) and after encounter teardown, `rust_GetSubtitle` returns null and subtitle iteration APIs (`rust_GetFirstSubtitle`, `rust_GetNextSubtitle`) return null. This matches the pre-init failure semantics defined in `audio-heart/specification.md` §13.1 (pre-init failure map) and §19.3 (pre-init/uninitialized query behavior). The comm subsystem does not call subtitle APIs outside an active encounter session; callers that poll subtitles outside that window shall treat null as the expected result.

### 14.4 Response System

```c
int          rust_DoResponsePhrase(unsigned int response_ref, const char *text,
                                    void (*func)(unsigned int));
void         rust_DisplayResponses(void);
void         rust_ClearResponses(void);
int          rust_SelectNextResponse(void);
int          rust_SelectPrevResponse(void);
int          rust_GetSelectedResponse(void);
int          rust_GetResponseCount(void);
unsigned int rust_ExecuteResponse(void);
```

**Critical**: The callback type is `void (*)(unsigned int)` — the response reference is passed to the callback. This matches the C convention `void (*RESPONSE_FUNC)(RESPONSE_REF)`.

### 14.5 Animation Management

```c
void         rust_StartCommAnimation(unsigned int index);
void         rust_StopCommAnimation(unsigned int index);
void         rust_StartAllCommAnimations(void);
void         rust_StopAllCommAnimations(void);
void         rust_PauseCommAnimations(void);
void         rust_ResumeCommAnimations(void);
unsigned int rust_GetCommAnimationFrame(unsigned int index);
```

### 14.6 Oscilloscope

```c
void          rust_AddOscilloscopeSamples(const short *samples, unsigned int count);
void          rust_UpdateOscilloscope(void);
unsigned char rust_GetOscilloscopeY(unsigned int x);
void          rust_ClearOscilloscope(void);
```

### 14.7 State Queries and Control

```c
int          rust_IsTalking(void);
int          rust_IsTalkingFinished(void);
void         rust_SetTalkingFinished(int finished);
unsigned int rust_GetCommIntroMode(void);
void         rust_SetCommIntroMode(unsigned int mode);
unsigned int rust_GetCommFadeTime(void);
void         rust_SetCommFadeTime(unsigned int time);
int          rust_IsCommInputPaused(void);
void         rust_SetCommInputPaused(int paused);
void         rust_UpdateCommunication(float delta_time);
```

### 14.8 Build Configuration

- **Current state**: `#define USE_RUST_COMM` in `config_unix.h` enables a narrow C→Rust bridge. Today, only `init_communication()` / `uninit_communication()` are swapped.
- **Target state**: `USE_RUST_COMM` selects the Rust-owned comm runtime as the authoritative implementation, replacing `comm.c`, `commglue.c`, and `commanim.c` as the active gameplay path.
- **Migration constraint**: Intermediate mixed-mode states may exist during migration. The build switch may need to be extended incrementally (e.g., per-function or per-subsystem guards) rather than as a single all-or-nothing toggle.
- Rust compile-time: The comm module is always compiled as part of the `uqm_rust` staticlib. No separate Cargo feature gate is required.

---

## 15. Global State Model

### 15.1 Required State Properties

Communication state shall have the following properties:

- **Single authoritative owner**: There shall be one authoritative source of truth for all communication subsystem state at runtime. There shall not be parallel C and Rust sources of truth for the same state.
- **Safe concurrent access**: State access from audio/sample delivery paths and main-thread dialogue flow must be synchronized safely. The specific synchronization mechanism is an implementation choice, subject to the lock discipline rules in §12.5.
- **Encounter-scoped lifecycle**: Transient encounter state (responses, subtitles, animation sequences, oscilloscope buffer, talking/finished flags, phrase enable/disable state) is created at encounter entry and destroyed at encounter exit.

### 15.2 Logical State Contents

The communication state logically contains:

- Initialization flag
- Active encounter data (`CommData`)
- Track/playback coordination state
- Subtitle tracking state
- Response list and selection state
- Animation sequence state
- Oscilloscope sample buffer and display state
- Talking/finished/intro-mode/fade/input-paused flags
- Phrase enable/disable state (encounter-local, see §11A)

### 15.3 Single Source of Truth

**Target state**: The Rust communication state is the sole authoritative source for all communication subsystem state. There are no parallel C state variables. C race scripts interact with state exclusively through FFI calls.

---

## 16. Animation End-State Specification

The Rust animation engine replaces the C `commanim.c` implementation entirely.

### 16.1 Normative Externally Visible Requirements

The following animation behaviors are required for compatibility:

- Frame progression shall follow the correct animation type (random, circular, yoyo, colormap) for each sequence.
- `BlockMask` mutual exclusion shall prevent conflicting animations from running concurrently.
- Talking/ambient coordination shall work correctly: `WAIT_TALKING` ambients settle at their neutral frame during talking.
- Transition animations shall play to completion before talking animation begins, and shall play in reverse when returning to silent.
- Subtitle redraw shall remain correct during animation changes (layering preserved).
- `ONE_SHOT_ANIM` sequences shall auto-disable after completing one cycle.
- Intro/outro visible behavior shall match legacy behavior.
- Randomized animation timing (frame delays and restart delays) shall stay within the configured `Base + random(0..Random)` ranges. Exact random sequences need not be identical to the C implementation, but the statistical behavior (timing ranges, frame selection distribution) shall be consistent with the configured descriptors.

### 16.2 Implementation-Parity Notes

The following details should match legacy C behavior where verified by code review, tests, or visual comparison against reference recordings. These are not independently normative but serve as implementation guidance:

**Sequence model**: Up to 22 sequences (20 ambient + 1 talk + 1 transition), each tracking a reference to its `AnimationDesc`, alarm timer, current direction, current/next frame index, frames remaining, animation type, and change flag.

**Active mask**: A bitmask tracks which animations are currently active, used to enforce `BlockMask` mutual exclusion.

**Frame advance rules per type**:

- **RANDOM_ANIM**: Select a random frame different from the current frame. The first frame is neutral (returned to at rest).
- **CIRCULAR_ANIM**: Advance sequentially, wrapping from the last frame to the first. The last frame is the neutral frame.
- **YOYO_ANIM**: Advance forward, then reverse at the ends. The first frame is neutral.
- **COLORXFORM_ANIM**: Same as picture animation but applies colormap transforms instead of sprite changes.

**Timing**: Frame rate and restart rate use randomized intervals:
- `actual_frame_delay = BaseFrameRate + random(0..RandomFrameRate)`
- `actual_restart_delay = BaseRestartRate + random(0..RandomRestartRate)`

All timing is in game ticks (ONE_SECOND = 60 ticks).

### 16.3 Verification Criteria

- No conflicting animations (per `BlockMask`) active simultaneously.
- Talking transition reaches correct steady state (talking animation active, transition complete).
- Subtitle redraw remains correct during animation changes.
- Representative race encounters visually match legacy behavior when compared against reference recordings or the C implementation.

---

## 17. Encounter-Specific Control Restrictions

### 17.1 Final Battle

During the final battle encounter:
- Conversation summary (Cancel during response selection) is unavailable.
- The encounter may have unique dialogue-flow constraints defined by its race script.

### 17.2 Load/Abort Suppression

After a game load occurs during or adjacent to an encounter:
- The subsystem detects the load condition and exits the dialogue loop cleanly.
- `LastActivity` is used to prevent spurious input from being processed after a load.

### 17.3 General Input Restrictions

Input handling during seeking suppresses animation updates (original 3DO behavior). No other general input restrictions are defined at the subsystem level; encounter-specific restrictions are the responsibility of race scripts.

---

## 18. Intro Transition Modes

| Mode | Constant | Behavior |
|---|---|---|
| Default | `CIM_DEFAULT` | Crossfade from the transition source |
| Fade from black | `CIM_FADE_IN_SCREEN` | Fade in from black |
| Crossfade screen | `CIM_CROSSFADE_SCREEN` | Crossfade using `ScreenTransition(3)` |
| Crossfade window | `CIM_CROSSFADE_WINDOW` | Crossfade only the comm window region |
| Crossfade space | `CIM_CROSSFADE_SPACE` | Crossfade in space context |

`SetCommIntroMode(mode, duration)` is callable by game code to configure the next encounter's intro transition. The mode resets to `CIM_DEFAULT` after each use.

---

## 19. Error Handling

| Condition | Behavior |
|---|---|
| Double init | Reject or return error |
| Operations before init | Reject or return error |
| Invalid track operations | Return appropriate error |
| Response overflow (>8) | Silently reject (return false from `add_response`) |
| Null string pointers from FFI | Gracefully handle (return null/0/early-return) |
| Game abort (`CHECK_ABORT`) | Short-circuit dialogue loops and exit cleanly |
| Load-game interruption | Detect `CHECK_LOAD` and exit cleanly |
| Invalid response index | Return appropriate error |

---

## 20. Constants and Limits

| Constant | Value | Purpose |
|---|---|---|
| `MAX_RESPONSES` | 8 | Maximum simultaneous player response options |
| `MAX_ANIMATIONS` | 20 | Maximum ambient animation slots per race |
| `ONE_SECOND` | 60 | Game ticks per second |
| `COMM_ANIM_RATE` | ONE_SECOND / 40 | Animation frame period (~25ms) |
| `OSCILLOSCOPE_RATE` | ONE_SECOND / 32 | Oscilloscope update period (~31ms) |
| `OSCILLOSCOPE_WIDTH` | 128 | Display width in pixels |
| `OSCILLOSCOPE_HEIGHT` | 64 | Display height in pixels |
| `BACKGROUND_VOL` | NORMAL_VOLUME / 2 (speech), NORMAL_VOLUME (no speech) | Music volume during NPC speech |
| `FOREGROUND_VOL` | NORMAL_VOLUME | Music volume during player's turn |
| `shared_phrase_buf` size | 2048 | Buffer for constructed response strings |
| `phrase_buf` size | 1024 | Per-encounter phrase buffer |
