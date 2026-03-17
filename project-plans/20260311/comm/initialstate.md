# Communication / Dialogue Subsystem Initial State

## Scope and purpose

The communication subsystem owns alien-contact presentation and dialogue flow during encounters. In the current codebase, that includes:

- encounter-level communication entry and exit (`InitCommunication`, `HailAlien`, post-encounter cleanup) in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1306-1442` and `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1171-1296`
- player-response collection, rendering, selection, and callback dispatch in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:415-462`, `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:923-1040`, and `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1129-1167`
- speech/track-driven talk segue behavior, seek/replay controls, and conversation-summary playback review in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:565-712` and `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:795-920`
- subtitle redraw/clear/update against track state in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1586-1642`
- script-side phrase splicing, number phrase construction, segue state mutation, and per-race dispatch glue in `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:35-239`, `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:260-355`, and `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:357-422`
- alien portrait and color-map animation sequencing in `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c:30-260` and the remainder of that file

This is a partially ported subsystem. Rust currently provides a communication state library and a broad FFI surface, but the active gameplay path remains predominantly C-owned.

## Current C structure

### Core orchestration

`/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c` is still the center of the live subsystem.

Important responsibilities still implemented there:

- response-window drawing and scrolling: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:415-462`
- speech graphics and oscilloscope driving: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:490-549`
- talking/seek/replay loop: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:565-712`
- conversation-summary paging from track subtitles: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:795-920`
- player response input and callback invocation: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:923-1040`
- main communication state machine: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1083-1127`
- response registration API used by scripts: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1129-1167`
- encounter resource setup/teardown and script callbacks: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1171-1296`
- high-level encounter selection, battle segue handling, and hail/attack split: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1306-1442`
- subtitle polling from the trackplayer: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1586-1642`

### Script/glue layer

`/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.h:74-119` defines the script-facing API shape:

- `Response(i,a)` macro expands to `DoResponsePhrase(...)`
- `NPCPhrase`, `NPCPhrase_cb`, and `NPCPhrase_splice` are declared here
- `construct_response`, `setSegue`, and `getSegue` are declared here

`/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c` remains the script/runtime bridge:

- `NPCPhrase_cb` fetches strings/clips/timestamps from `CommData.ConversationPhrases` and queues them with `SpliceTrack(...)`: `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:35-91`
- `NPCPhrase_splice` still decides between `SpliceTrack(NULL, ...)` and `SpliceMultiTrack(...)`: `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:93-119`
- `NPCNumber`/`NPCNumberPhrase` still synthesize spoken-number tracks in C: `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:122-239`
- `construct_response` still builds composed player-response strings in C: `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:260-313`
- `setSegue` / `getSegue` still own `BATTLE_SEGUE`, instant-victory, and defeat-state mutation: `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:315-355`
- `init_race` still dispatches to race-specific comm scripts via a large C `switch`: `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:357-422`

### Animation layer

`/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c` still owns the active animation scheduler and state machine. Even in the first 260 lines, the evidence is already clear:

- global animation scheduling state is file-static in C (`LastTime`, `Sequences`, `ActiveMask`, `TalkDesc`, `TransitDesc`): `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c:30-41`
- ambient/talk sequence setup and descriptor mutation happen in C: `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c:70-117`
- colormap and portrait-sequence advancement logic is implemented in C: `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c:142-260`

`comm.c` calls that C animation engine directly through `ProcessCommAnimations`, `InitCommAnimations`, `wantTalkingAnim`, `haveTalkingAnim`, `setRunIntroAnim`, `setRunTalkingAnim`, `runningIntroAnim`, `runningTalkingAnim`, and `setStopTalkingAnim` in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:536`, `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:666-710`, and `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:776-778`.

### Race-script corpus remains C-owned

The per-race dialogue trees are still in C under `/Users/acoliver/projects/uqm/sc2/src/uqm/comm/**/*.c`. The repository currently contains 27 such files, including:

- `/Users/acoliver/projects/uqm/sc2/src/uqm/comm/arilou/arilouc.c`
- `/Users/acoliver/projects/uqm/sc2/src/uqm/comm/orz/orzc.c`
- `/Users/acoliver/projects/uqm/sc2/src/uqm/comm/vux/vuxc.c`
- `/Users/acoliver/projects/uqm/sc2/src/uqm/comm/starbas/starbas.c`
- `/Users/acoliver/projects/uqm/sc2/src/uqm/comm/zoqfot/zoqfotc.c`

Those scripts are still selected by `init_race(...)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:357-422` and still target the C APIs declared in `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.h:74-119`.

## Current Rust structure

### Module layout

The Rust crate exposes the communication module from `/Users/acoliver/projects/uqm/rust/src/lib.rs:7-32`, with `pub mod comm;` at `/Users/acoliver/projects/uqm/rust/src/lib.rs:10`.

`/Users/acoliver/projects/uqm/rust/src/comm/mod.rs:20-35` defines the Rust-side communication subsystem modules:

- `animation`
- `ffi`
- `oscilloscope`
- `response`
- `state`
- `subtitle`
- `track`
- `types`

### Rust state model

The Rust subsystem is organized around a global `COMM_STATE` lock in `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:15-17`.

`CommState` currently stores:

- init flag and optional `CommData`: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:21-27`
- `TrackManager`: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:28-29`
- `SubtitleTracker`: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:31-32`
- `ResponseSystem`: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:34-35`
- `AnimContext`: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:37-38`
- `Oscilloscope`: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:40-41`
- talking / talking-finished / intro / fade / input-paused flags: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:43-59`

The Rust lifecycle is minimal:

- `init()` only flips `initialized` and activates the oscilloscope: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:88-97`
- `uninit()` clears state and deactivates the oscilloscope: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:99-104`
- `update(delta_time)` advances the Rust track, subtitle tracker, animation context, and oscilloscope; if the Rust track reaches finished, it marks talking complete: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:316-340`

### Rust track/subtitle/response/animation/oscilloscope pieces

What exists in Rust today:

- `TrackManager` is an in-memory chunk timeline with synthetic playback state, chunk selection, seek/jump/rewind, and `wait()` semantics: `/Users/acoliver/projects/uqm/rust/src/comm/track.rs:74-306`
- `SubtitleTracker` is a separate sorted subtitle timeline: `/Users/acoliver/projects/uqm/rust/src/comm/subtitle.rs:68-206`
- `ResponseSystem` stores up to 8 responses and selection state, mirroring the C maximum: `/Users/acoliver/projects/uqm/rust/src/comm/response.rs:32-182`
- `AnimContext` is a generic animation container with start/stop/pause/resume/update behavior: `/Users/acoliver/projects/uqm/rust/src/comm/animation.rs:231-339`
- `Oscilloscope` is an in-memory waveform buffer/display model: `/Users/acoliver/projects/uqm/rust/src/comm/oscilloscope.rs:11-172`

These pieces are real code, but there is no evidence in the C gameplay path that they have replaced the legacy comm runtime.

## Build and configuration wiring

### Feature/define wiring

The Unix config currently enables the communication Rust bridge at compile time with `#define USE_RUST_COMM` in `/Users/acoliver/projects/uqm/sc2/config_unix.h:86-87`.

That define affects exactly two communication-side C boundaries found by code search:

- partial compile guard in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:400-412`
- Rust wrapper compilation in `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c:12-29`

### Rust crate build

The Rust crate is a static library/rlib crate in `/Users/acoliver/projects/uqm/rust/Cargo.toml:5-8`.

The communication module is part of the crate surface through `/Users/acoliver/projects/uqm/rust/src/lib.rs:10`.

`/Users/acoliver/projects/uqm/rust/build.rs:5-16` builds only `../sc2/src/mem_wrapper.c` on the Rust side; there is no communication-specific build step there.

### Important implication

The repository evidence shows the feature define and the Rust communication library exist, but the actual C→Rust integration wiring inside gameplay code is extremely narrow. The only direct C call site to a Rust comm symbol found under `sc2/src` is `rust_InitCommunication()` inside `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c:15-27`.

## C↔Rust integration points

### Active wrapper boundary

The C shim is `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c`:

- file is only compiled when `USE_RUST_COMM` is set: `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c:12-29`
- it exports only two C-callable wrappers: `init_communication()` and `uninit_communication()`: `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c:15-27`
- those wrappers call `rust_InitCommunication()` and `rust_UninitCommunication()`: `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c:17-26`

### C-side FFI header

`/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.h:15-75` declares a much larger Rust FFI surface, including:

- initialization/state clear: `rust_InitCommunication`, `rust_UninitCommunication`, `rust_IsCommInitialized`, `rust_ClearCommunication`
- track operations: `rust_StartTrack` through `rust_ClearTrack`
- subtitle access: `rust_GetSubtitle`, subtitle enable/disable toggles
- response operations: `rust_DoResponsePhrase` through `rust_ExecuteResponse`
- animation operations: `rust_StartCommAnimation` through `rust_GetCommAnimationFrame`
- oscilloscope functions: `rust_AddOscilloscopeSamples` through `rust_ClearOscilloscope`
- state-query/update functions: `rust_IsTalking`, `rust_SetTalkingFinished`, `rust_SetCommIntroMode`, `rust_UpdateCommunication`, etc.

### Rust FFI exports

The Rust implementations for those declarations are in `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:16-454`.

Important exported groups:

- init/uninit/status/clear: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:16-45`
- track exports: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:51-156`
- subtitle exports: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:162-188`
- response exports: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:194-294`
- animation exports: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:300-345`
- oscilloscope exports: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:351-378`
- talking/intro/fade/input/update exports: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:384-454`

### Partial-port boundary

The critical split is that the C code only swaps `init_communication()` / `uninit_communication()` under `USE_RUST_COMM`, while the rest of `comm.c` stays live regardless.

Evidence:

- only the no-op `init_communication` / `uninit_communication` bodies are behind `#ifndef USE_RUST_COMM`: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:400-412`
- all subsequent communication runtime code remains compiled in the same file: response rendering starts immediately at `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:415`, and the main comm path continues through `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1645+`
- no C call sites were found for `rust_StartTrack`, `rust_DoResponsePhrase`, `rust_GetSubtitle`, `rust_UpdateCommunication`, or `rust_AddOscilloscopeSamples`; search under `/Users/acoliver/projects/uqm/sc2/src` only found declarations in `rust_comm.h` and the init wrapper call in `rust_comm.c`

That is the strongest evidence-grounded statement of the current split: Rust comm is linked and initialized, but the active encounter/dialogue mechanics are still driven by C code and C-owned script APIs.

## What is already ported

Evidence-backed Rust-owned pieces that do exist:

- global communication state object and lifecycle API: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:15-17`, `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:88-124`
- track timeline abstraction with start/stop/update/seek/jump/rewind/commit: `/Users/acoliver/projects/uqm/rust/src/comm/track.rs:93-306`
- subtitle timeline abstraction: `/Users/acoliver/projects/uqm/rust/src/comm/subtitle.rs:81-206`
- response list abstraction capped at 8 responses: `/Users/acoliver/projects/uqm/rust/src/comm/response.rs:32-182`
- animation model/context abstraction: `/Users/acoliver/projects/uqm/rust/src/comm/animation.rs:35-339`
- oscilloscope waveform model: `/Users/acoliver/projects/uqm/rust/src/comm/oscilloscope.rs:34-172`
- broad FFI surface exposing all of the above to C: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:16-454`
- C init/uninit shim routing into Rust under `USE_RUST_COMM`: `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c:12-29`

## What remains C-owned

The live communication/dialogue gameplay path remains C-owned in these major areas:

### Encounter entry/exit and battle segue policy

Still in C:

- conversation selection and encounter setup: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1306-1405`
- post-comm combat segue and encounter teardown: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1406-1442`
- `setSegue` / `getSegue` game-state mutation: `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:315-355`

### Dialogue script dispatch and content logic

Still in C:

- `init_race(...)` switch dispatch in `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:357-422`
- all 27 race-specific dialogue-tree files under `/Users/acoliver/projects/uqm/sc2/src/uqm/comm/**/*.c`
- player-response registration API used by those scripts: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1129-1167` and `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.h:74-89`

### Track/speech/subtitle runtime integration

Still in C:

- phrase splicing and multi-track assembly: `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:35-119`
- number speech composition: `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c:122-239`
- talk segue control over `PlayTrack`, `JumpTrack`, `FastForward_*`, `FastReverse_*`, `PlayingTrack`, `StopTrack`: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:565-712` and `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:923-968`
- subtitle polling from `GetTrackSubtitle()`: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1610-1642`
- summary-view traversal using `GetFirstTrackSubtitle`, `GetNextTrackSubtitle`, `GetTrackSubtitleText`: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:795-920`

### Response presentation and callback dispatch

Still in C:

- response rendering and scrolling: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:415-462`
- response selection loop: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:971-1040`
- selected response callback invocation with `RESPONSE_FUNC(RESPONSE_REF)`: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:938-941`

### Animation runtime

Still in C:

- animation scheduler/state in `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c:30-41`
- animation processing used by the encounter loop via `ProcessCommAnimations(...)`: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:522-542`

## Parity gaps between Rust comm and the live C subsystem

### 1. Compile split is only init/uninit, not the subsystem body

The subsystem is not replaced wholesale.

- C no-op init/uninit are the only guarded region: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:400-412`
- everything after that remains C-owned and active: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:415-1645+`
- Rust wrapper only forwards init/uninit: `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c:15-27`

### 2. Rust FFI surface is largely unused by C gameplay code

The header advertises full track/response/subtitle/animation/oscilloscope integration, but repository search only found direct C use of `rust_InitCommunication()` in the wrapper file.

Evidence:

- declarations present: `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.h:15-75`
- implementations present: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:16-454`
- active C call site found only for init in `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c:19`

### 3. Rust track model is a synthetic timeline, not the live trackplayer integration

The C runtime depends on legacy trackplayer behavior including:

- `PlayTrack` / `StopTrack` / `JumpTrack` / seek helpers in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:565-712` and `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:923-968`
- subtitle enumeration APIs in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:795-920` and `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1616`

By contrast, the Rust `TrackManager` only updates an internal `position` and chunk index; it does not show any coupling to the C audio/trackplayer subsystem:

- in-memory chunk list and playback state only: `/Users/acoliver/projects/uqm/rust/src/comm/track.rs:76-91`
- `start()` only flips state and selects chunk 0: `/Users/acoliver/projects/uqm/rust/src/comm/track.rs:148-154`
- `update()` only advances `position` numerically and marks `Finished` at end: `/Users/acoliver/projects/uqm/rust/src/comm/track.rs:206-222`

### 4. Rust response callback signature does not match C script callback shape

The live C response dispatcher expects `RESPONSE_FUNC(RESPONSE_REF)` and invokes the callback with the selected reference:

- typedef in `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.h:84-89`
- call site in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:938-941`

The Rust FFI declaration and implementation use `void (*func)(void)` / `Option<extern "C" fn()>`:

- C header declaration: `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.h:41`
- Rust export signature: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:196-200`
- Rust callback execution invokes a no-argument function: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:284-293`

That is a concrete ABI/behavior mismatch at the response-dispatch boundary.

### 5. Rust subtitle export has an invalid-lifetime / type-boundary risk

`rust_GetSubtitle()` returns a raw pointer derived from Rust string data while explicitly noting the pointer is only valid while the lock is held:

- warning comment: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:165-166`
- returned pointer: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:167-170`

No C-side usage was found, which likely avoids immediate breakage, but this export is not safely consumable as written if adopted directly.

### 6. Rust animation model does not replace the C comm animation engine

The live engine uses `CommData` descriptors, ambient/talk/transit sequencing, colormap transforms, and race-specific animation flags in C:

- C engine state and descriptor use: `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c:30-41`, `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c:70-117`, `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c:142-260`

The Rust `AnimContext` is a generic list of local animations with no evidence of integration with `CommData.AlienAmbientArray`, talk/transit descriptors, or portrait rendering:

- generic container: `/Users/acoliver/projects/uqm/rust/src/comm/animation.rs:231-339`

### 7. Rust state initialization is much shallower than C encounter initialization

Rust init only marks the subsystem initialized and activates the oscilloscope: `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:88-97`.

The real encounter path still performs extensive C setup in `HailAlien()`:

- loads graphic/font/colormap/music/string table: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1186-1200`
- allocates subtitle cache context/frame: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1206-1217`
- creates and clips `AnimContext`, draws SIS frame/window, and titles: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1224-1267`
- runs `CommData.init_encounter_func`, `post_encounter_func`, `uninit_encounter_func`: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1270-1275`
- destroys all loaded assets and contexts: `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:1280-1295`

## Notable risks and unknowns

### Risk: misleadingly broad Rust FFI surface

The codebase advertises a large Rust communication API in `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.h:15-75`, but the active C path uses almost none of it. That creates a maintenance risk: readers can easily overestimate how much of comm is actually ported.

### Risk: callback ABI mismatch if response execution is switched to Rust

The mismatch between `RESPONSE_FUNC(RESPONSE_REF)` in C and `extern "C" fn()` in Rust is a concrete blocker for moving script callback dispatch onto Rust without an adapter.

Evidence: `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.h:84-89`, `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:938-941`, `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.h:41`, `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:196-200`, `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:284-293`.

### Risk: unsafe subtitle pointer export

If `rust_GetSubtitle()` becomes used from C, its returned pointer lifetime is not stable beyond the read lock acknowledged in the comment.

Evidence: `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:164-170`.

### Risk: duplicated state models

There are currently two distinct communication-state models:

- the live C globals and encounter state in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c:63-100`
- the Rust `CommState` object in `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:21-59`

Because only init/uninit are bridged, those models are not the single source of truth for the running encounter.

### Unknown: final build linkage path for the Rust staticlib

The repository evidence confirms the Rust crate exists and comm is part of it, but the specific top-level build file that links the Rust static library into the final UQM binary was not identified from the inspected files. This document therefore only claims the code-level integration points that are directly evidenced in the source files above.

## Bottom line

The communication/dialogue subsystem is only lightly integrated with Rust today.

- **Rust-owned today:** communication state container, track/subtitle/response/animation/oscilloscope models, and a broad comm FFI export surface (`/Users/acoliver/projects/uqm/rust/src/comm/*.rs`, especially `/Users/acoliver/projects/uqm/rust/src/comm/ffi.rs:16-454`).
- **Actually switched over in C:** only `init_communication()` / `uninit_communication()` via `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c:15-27`, enabled by `/Users/acoliver/projects/uqm/sc2/config_unix.h:86-87`.
- **Still C-owned and still authoritative for gameplay:** encounter orchestration, dialogue-tree dispatch, script behavior, phrase splicing, trackplayer integration, subtitle polling, response UI, response callback dispatch, and animation runtime (`/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c`, `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c`, `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c`, and `/Users/acoliver/projects/uqm/sc2/src/uqm/comm/**/*.c`).

So the current state is best described as: **Rust comm infrastructure exists, but the live dialogue subsystem is still predominantly C, with only a narrow init/uninit bridge and several clear parity gaps blocking fuller cutover.**
