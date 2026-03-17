# Audio Heart / High-Level Sound Subsystem — Functional and Technical Specification

## 1. Purpose and scope

This document specifies the desired end state of the audio-heart subsystem: the high-level sound layer that sits above the mixer and decoder infrastructure and below the game logic (comm, melee, starmap, etc.). It defines what the subsystem must do, what contracts it must honor, what it owns, what it delegates, and how errors and lifecycle are handled.

This is not an implementation plan. It does not prescribe task ordering, migration strategy, or incremental delivery milestones.

---

## 2. Subsystem boundaries

### 2.1 Contract hierarchy

The audio-heart subsystem maintains three tiers of contract, listed from most stable to most flexible:

1. **Primary compatibility contract — the exported C ABI and ABI-visible results and side effects.** The function signatures, struct layouts, pointer semantics, sentinel values, and return conventions declared in `audio_heart_rust.h` are the authoritative external interface. Any change here is a breaking change.
2. **Secondary integration contract — interactions with mixer, decoder, UIO, resource system, comm, and timing services.** Audio-heart depends on these services and must satisfy the behavioral expectations of its downstream consumers (comm subtitle polling, oscilloscope rendering, etc.). These contracts are stable but may evolve if both sides coordinate.
3. **Non-contract implementation detail — internal module boundaries, helper function names, synchronization primitives, data-structure layouts, and thread-internal architecture.** These may be reorganized freely so long as primary and secondary contracts remain satisfied.

### 2.2 What audio-heart owns

The audio-heart subsystem is the sole owner of:

- **Stream playback orchestration** — starting, stopping, pausing, resuming, seeking, and monitoring decoder-to-mixer streaming for all source types (music, speech, SFX).
- **Track assembly and playback** — building ordered sequences of audio chunks with per-chunk decoders, subtitle text, timing metadata, and callbacks; playing, stopping, seeking, and paging through those sequences.
- **Subtitle/speech coordination** — synchronizing subtitle text changes with buffer playback via buffer tagging; exposing stable subtitle pointers for the comm subsystem's `CheckSubtitles` polling loop.
- **Music behavior** — playing, stopping, pausing, resuming, seeking, and fading music on the dedicated music source; tracking the current music reference.
- **Speech behavior** — playing and stopping speech on the dedicated speech source; tracking the current speech reference.
- **SFX behavior** — playing sound effects on numbered SFX channels with positional audio; stopping, querying, and volume-controlling individual channels; cleaning up finished channels.
- **Oscilloscope/scope data** — capturing decoded audio into a per-source ring buffer and generating waveform data for the comm oscilloscope display via AGC-normalized sampling.
- **Top-level control** — initialization/uninitialization of the streaming decoder subsystem and mixer pump; global stop-all, sound-playing queries, and blocking wait-for-end; per-category volume control (SFX, speech, music).
- **Resource loading (type-specific implementation)** — loading music files and sound bank files from the content directory through the UIO filesystem (owned by `file-io/specification.md`), decoding them into mixer-ready buffers, and returning opaque handles; destroying those handles and releasing mixer resources. Audio-heart owns the type-specific loading *implementation* for `SNDRES` and `MUSICRES` resource types: file parsing, decoder creation, format detection, error handling, and opaque handle semantics. The typed resource *dispatch* layer — resource key lookup, lazy-load lifecycle, reference counting, and handler registration — is owned by the resource subsystem (`resource/specification.md`). Audio-heart's loading functions are invoked *by* the resource dispatch layer as registered handlers.
- **Mixer pump lifecycle** — starting and stopping the mixer pump component that bridges mixer output to the platform audio backend. This is part of the audio-heart contract because audio-heart is the subsystem that knows when streaming is needed, and initialization/shutdown requirements are defined around this ownership. See §17.2.

### 2.3 What audio-heart does NOT own

- **Mixer internals** — buffer allocation/deletion, source handle management, buffer data upload, source state queries, gain/position setting, play/stop/pause/rewind primitives. These are delegated to the `mixer` module.
- **Decoder internals** — format detection, codec implementation, frame-level decode, seek, format/frequency queries. These are delegated to the `decoder` module and its format-specific implementations (Ogg, WAV, AIFF, DukAud, MOD, Null).
- **Audio backend / output** — the audio output stream and device management. Audio-heart manages the mixer pump lifecycle (see §2.2), but the backend's device configuration and platform-specific output handling are not part of the audio-heart API contract.
- **Comm orchestration** — encounter entry/exit, dialogue tree dispatch, response UI, animation scheduling. The comm subsystem is a consumer of audio-heart through the track player and speech APIs.
- **Game-state volume preferences** — the settings UI and config persistence that determine initial volume levels. Audio-heart exposes setters; callers are responsible for the values.

### 2.4 Ownership model

The audio-heart subsystem operates in a mixed-language runtime. Ownership responsibilities are:

- **Behavior ownership:** All high-level sound behavior (playback orchestration, track assembly, music/speech/SFX control, resource loading, subtitle coordination) is owned by the audio-heart subsystem.
- **Runtime-service dependencies:** Audio-heart may continue to call established cross-language platform services such as UIO file access, timing (`GetTimeCounter`), shutdown detection (`QuitPosted`), and C-compatible memory allocation. These are runtime-service dependencies, not replacement targets.
- **Replacement scope:** Only the old high-level sound logic and duplicated helper logic (the C implementations in `stream.c`, `trackplayer.c`, `music.c`, `sfx.c`, `sound.c`, `fileinst.c` and their residual tails) are in scope for replacement. Foundational engine/runtime services (UIO, timing, memory allocation) are not in scope.

### 2.5 Downstream consumers

| Consumer | APIs used |
|---|---|
| Comm subsystem (`comm.c`, `commglue.c`) | `SpliceTrack`, `SpliceMultiTrack`, `PlayTrack`, `StopTrack`, `JumpTrack`, `PauseTrack`, `ResumeTrack`, `PlayingTrack`, `FastReverse_Smooth`, `FastForward_Smooth`, `FastReverse_Page`, `FastForward_Page`, `GetTrackPosition`, `GetTrackSubtitle`, `GetFirstTrackSubtitle`, `GetNextTrackSubtitle`, `GetTrackSubtitleText` |
| Oscilloscope display (`oscill.c`) | `GraphForegroundStream` |
| Music playback (melee, starmap, etc.) | `PLRPlaySong`, `PLRStop`, `PLRPlaying`, `PLRSeek`, `PLRPause`, `PLRResume`, `FadeMusic`, `SetMusicVolume` |
| Speech playback | `snd_PlaySpeech`, `snd_StopSpeech` |
| SFX playback (combat, UI) | `PlayChannel`, `StopChannel`, `ChannelPlaying`, `SetChannelVolume`, `UpdateSoundPosition`, `GetPositionalObject`, `SetPositionalObject` |
| Resource system | `LoadSoundFile`, `LoadMusicFile`, `DestroySound`, `DestroyMusic` |
| Game init/shutdown | `InitSound`, `UninitSound`, `InitStreamDecoder`, `UninitStreamDecoder`, `StopSound`, `SoundPlaying`, `WaitForSoundEnd` |
| Sample management (internal + external) | `TFB_CreateSoundSample`, `TFB_DestroySoundSample`, `TFB_SetSoundSampleData`, `TFB_GetSoundSampleData`, `TFB_SetSoundSampleCallbacks`, `TFB_GetSoundSampleDecoder`, `TFB_FindTaggedBuffer`, `TFB_TagBuffer`, `TFB_ClearBufferTag` |
| Source management | `StopSource`, `CleanSource`, `SetSFXVolume`, `SetSpeechVolume` |
| Fade control | `SetMusicStreamFade` |

---

## 3. Glossary

The following terms are used consistently throughout this specification and the companion requirements document:

| Term | Definition |
|---|---|
| **source** | One of the fixed set of mixer output slots (indices 0–6). Each source can have at most one sample attached at a time. |
| **channel** | An SFX source (indices 0–4). "Channel" and "SFX source" are interchangeable in SFX-specific APIs. |
| **playback source** | Any source (SFX, music, or speech) considered from the perspective of active playback. |
| **sample** | A `SoundSample` — the primary unit of playable audio data, owning decoder state, mixer buffers, tags, and callbacks. |
| **stream** | Active decoder-to-mixer streaming on a source. A source is "streaming" when background processing is feeding it decoded audio. |
| **resource handle** | An opaque pointer returned to C callers representing a loaded music file (`MusicRef` / `MUSIC_REF`) or sound bank (`SoundBank` / `SOUND_REF` / `STRING_TABLE`). |
| **decoder** | An object implementing the `SoundDecoder` trait, providing decode, seek, format, and length operations for a specific audio file. |
| **game ticks** | The unit of `GetTimeCounter()`. One second = `ONE_SECOND` (840) ticks. |
| **chunk** | A `SoundChunk` — one node in the track player's ordered sequence of audio segments with timing, subtitle, and decoder metadata. |
| **track program** | The complete ordered sequence of chunks assembled by one or more splice operations, representing a conversation or other sequenced audio. |
| **mixer pump** | The bridge component that calls the mixer's mix function and writes the result to the audio backend. Audio-heart owns its lifecycle (see §2.2 and §17.2). |

---

## 4. Data model

> **Normative scope note:** The tables in this section describe one compatible data representation that satisfies the externally visible contracts. Field names, types, and groupings are illustrative of a valid design and are used as reference vocabulary throughout this specification. Implementations may use different field names, data structures, and representations as long as all ABI-visible and state-observable requirements defined in the normative sections of this specification are satisfied.

### 4.1 SoundSample

A `SoundSample` is the primary unit of playable audio data. It owns:

| Field | Description |
|---|---|
| `decoder` | Active decoder for streaming sources. Absent for pre-decoded SFX. |
| `length` | Total accumulated decoder chain length in seconds. |
| `buffers` | Mixer buffer handles allocated for this sample. |
| `num_buffers` | Number of active mixer buffers. |
| `buffer_tags` | Per-buffer subtitle synchronization tags. |
| `offset` | Initial time offset in game ticks (for track positioning). |
| `looping` | Whether the stream loops at EOF. |
| `data` | Opaque user data (game-specific). |
| `callbacks` | Stream event callbacks. |

**Ownership**: Samples must support shared ownership between the source table and callers, and concurrent access from background processing and the game thread must be synchronized. The specific synchronization mechanism is an implementation detail.

**Lifecycle**: Created via `create_sound_sample`, destroyed via `destroy_sound_sample`. Destruction releases all mixer buffers. The sample must not be attached to any source when destroyed.

### 4.2 SoundSource

A `SoundSource` represents one mixer output slot. There are exactly `NUM_SOUNDSOURCES` (7) sources, allocated at initialization:

| Index | Role |
|---|---|
| 0–4 | SFX channels (`FIRST_SFX_SOURCE` through `LAST_SFX_SOURCE`) |
| 5 | Music (`MUSIC_SOURCE`) |
| 6 | Speech (`SPEECH_SOURCE`) |

The fixed count and role assignment are ABI-visible because callers pass source/channel indices by number.

Each source holds:

| Field | Description |
|---|---|
| `sample` | Currently attached sample (shared-ownership reference). |
| `handle` | Mixer source handle. |
| `stream_should_be_playing` | Whether background processing should feed this source. |
| `start_time` | Playback start timestamp (game ticks), for position computation. |
| `pause_time` | Pause timestamp (0 = not paused). |
| `positional_object` | Opaque game object identifier for positional audio. |
| `last_q_buf` | Last queued buffer handle (for end-chunk callback context). |
| `sbuffer` | Oscilloscope ring buffer. Present only for scope-enabled sources. |
| `sbuf_size`, `sbuf_tail`, `sbuf_head`, `sbuf_lasttime` | Scope buffer bookkeeping. |
| `queued_buf_sizes` | FIFO of actual decoded byte counts per queued buffer (for accurate scope removal). |
| `end_chunk_failed` | Persistent flag preventing repeated futile end-chunk callback calls. Reset when a new stream starts. |

**Ownership**: Sources are held in a fixed-size collection. All access must be synchronized. The specific synchronization mechanism is an implementation detail.

### 4.3 SoundChunk (track player)

A `SoundChunk` is one node in the track player's ordered sequence of audio segments:

| Field | Description |
|---|---|
| `decoder` | Per-chunk decoder. Taken by the stream engine during playback. |
| `start_time` | Absolute position in the track sequence (milliseconds). |
| `run_time` | Display hint in game ticks. Positive = exact duration, negative = minimum display time. |
| `tag_me` | Whether this chunk's buffer should be tagged for subtitle sync. |
| `track_num` | Which track this chunk belongs to (0-based). |
| `text` | Subtitle text for this chunk. |
| `text_cstr` | Lazily-cached C string for stable FFI pointer identity. |
| `callback` | Per-chunk callback. Must be callable multiple times because callbacks can fire multiple times on seek. |
| `next` | Link to the next chunk in the sequence. |

**Drop behavior**: Iterative cleanup to prevent stack overflow on long chunk sequences.

### 4.4 SoundTag

A `SoundTag` pairs a mixer buffer handle with opaque subtitle data:

| Field | Description |
|---|---|
| `buf_handle` | Mixer buffer this tag is attached to. |
| `data` | Opaque payload (chunk identifier for subtitle matching). |

### 4.5 MusicRef

A `MusicRef` is a shared-ownership wrapper around a `SoundSample`. It is cloneable (incrementing the reference count) and serves as the opaque handle type for music resources at the C boundary.

### 4.6 SoundBank

A `SoundBank` holds a collection of pre-decoded SFX samples loaded from a single resource file:

| Field | Description |
|---|---|
| `samples` | Decoded samples, indexed by sound effect number within the bank. |
| `source_file` | Resource filename this bank was loaded from. |

### 4.7 FadeState

Music fade is tracked in a dedicated synchronized struct, separate from sources:

| Field | Description |
|---|---|
| `start_time` | Fade start timestamp (game ticks). |
| `interval` | Fade duration in ticks (0 = inactive). |
| `start_volume` | Volume at fade start. |
| `delta` | Volume delta (`end_volume - start_volume`). |

### 4.8 SoundPosition

A `#[repr(C)]` struct for positional SFX:

| Field | Description |
|---|---|
| `positional` | Whether positional audio is enabled. |
| `x` | X coordinate. |
| `y` | Y coordinate. |

### 4.9 TrackPlayerState

Synchronized state for the track player. The track player must maintain:

- the assembled chunk sequence (head and tail for O(1) append),
- current playback chunk and current displayed subtitle chunk navigation state,
- the last subtitle-bearing chunk reference (for subtitle continuation),
- the shared sample used for streaming,
- track count, accumulated decoder offset, subtitle continuation flag, total track length, and last spliced track resource name.

All navigation state (current chunk, current subtitle chunk, last subtitle chunk, tail) is valid only while the chunk sequence is alive and must always be accessed under the track-state synchronization boundary. All navigation state is invalidated in `stop_track` before the sequence is released.

---

## 5. Time model

Audio-heart uses several time and position units. This section defines each unit, where it appears, and the conversion rules.

### 5.1 Unit definitions

| Unit | Domain | Value space | Used in |
|---|---|---|---|
| **Game ticks** | External time, position queries | Unsigned integer; `ONE_SECOND = 840` ticks per second | `start_time`, `pause_time`, `run_time`, `get_stream_position_ticks`, `get_track_position`, `ACCEL_SCROLL_SPEED`, fade timing |
| **Milliseconds (ms)** | Internal stream/track timing | `f64` (track) or integral | `SoundChunk.start_time`, `seek_stream(pos_ms)`, subtitle pacing (`TEXT_SPEED`), `dec_offset` |
| **Seconds** | Decoder lengths | `f32` | `SoundSample.length`, decoder `length()` return |
| **PCM frames** | Decoder seek positions | Unsigned integer | `decoder.seek(frame)`, `decoder.get_frame()` |
| **Proportional units** | Position query scaling | Caller-defined integer scale | `get_track_position(in_units)` where `in_units > 0` |

### 5.2 Conversions

- **Seconds → milliseconds**: multiply by 1000.
- **Milliseconds → PCM frames**: `ms * frequency / 1000`. Truncation toward zero.
- **Game ticks → seconds**: divide by `ONE_SECOND` (840).
- **Game ticks → milliseconds**: `ticks * 1000 / ONE_SECOND`.
- **Proportional**: `in_units * pos_ticks / length_ticks`.

### 5.3 Position query semantics

- `get_stream_position_ticks(source_index)` returns `GetTimeCounter() - source.start_time`, clamped to non-negative. This is an elapsed-wall-clock estimate, not a decoded-sample-accurate position.
- `get_track_position(in_units)` returns position in game ticks (if `in_units == 0`) or scaled proportional position (if `in_units > 0`), clamped to track length.
- Position queries are **approximate** and **non-monotonic** under pause/resume and seek. They are not required to reflect buffered-but-not-yet-played audio.

---

## 6. Constants

| Name | Value | Description |
|---|---|---|
| `NUM_SFX_CHANNELS` | 5 | Simultaneous SFX channels. |
| `FIRST_SFX_SOURCE` | 0 | First SFX source index. |
| `LAST_SFX_SOURCE` | 4 | Last SFX source index. |
| `MUSIC_SOURCE` | 5 | Music source index. |
| `SPEECH_SOURCE` | 6 | Speech source index. |
| `NUM_SOUNDSOURCES` | 7 | Total source count. |
| `MAX_VOLUME` | 255 | Maximum volume level. |
| `NORMAL_VOLUME` | 160 | Default volume level. This is the single canonical value matching the original C codebase. |
| `PAD_SCOPE_BYTES` | 256 | Extra bytes in scope buffer beyond queued data. |
| `ONE_SECOND` | 840 | One second in `GetTimeCounter` ticks. |
| `NUM_BUFFERS_PER_SOURCE` | 8 | Mixer buffers per streaming source. |
| `BUFFER_SIZE` | 16384 | Bytes per mixer buffer. |
| `ACCEL_SCROLL_SPEED` | 300 | Smooth seek step in game ticks. |
| `TEXT_SPEED` | 80 | Milliseconds per character for subtitle pacing. |
| `MAX_MULTI_TRACKS` | 20 | Maximum tracks in a multi-track splice. |
| `ATTENUATION` | 160.0 | Distance attenuation factor for positional SFX. |
| `MIN_DISTANCE` | 0.5 | Minimum source distance (prevents zero-distance artifacts). |
| `WAIT_ALL_SOURCES` | `~0` (bitwise complement of zero, i.e., `UINT32_MAX`) | Sentinel value passed to `wait_for_sound_end` to wait on all managed sources. See §13.4. |

There must be exactly one definition of `NORMAL_VOLUME` (160) used throughout the subsystem. Any conflicting local redefinitions must be eliminated.

---

## 7. Stream playback

### 7.1 Engine architecture

The stream engine manages background audio processing and the source table.

The engine requires:

- Fade state, synchronized independently from sources.
- Background decoder processing and a mechanism to wake it.
- A shutdown flag observable by the background processing.

The source table contains `NUM_SOUNDSOURCES` entries, each independently synchronized.

> **Design note (non-normative):** The current implementation uses a global singleton with a condvar/mutex pair for waking the decoder thread and per-source mutexes. These are reasonable choices but are not part of the contract. Alternative architectures (e.g., event-driven, task-based) are acceptable as long as the concurrency properties in §20 are satisfied.

### 7.2 Initialization

`init_stream_decoder()` shall:

1. Allocate `NUM_SOUNDSOURCES` mixer source handles via `mixer_gen_sources`.
2. Store each handle in the corresponding `SoundSource.handle` slot.
3. Start the mixer pump (a component that calls `mixer_mix_channels` to mix all sources into PCM and writes the result to the audio device).
4. Start the background decoder processing thread.
5. Reject with an error if called twice without intervening `uninit_stream_decoder`.

`uninit_stream_decoder()` shall:

1. Signal mixer pump shutdown and wait for it to stop.
2. Signal the decoder thread shutdown, wake it, and wait for it to stop.
3. Allow re-initialization afterward.

### 7.3 Decoder thread

The decoder thread loops until shutdown, on each iteration:

1. Process music fade (adjusting volume toward the fade target based on elapsed time).
2. For each source from `MUSIC_SOURCE` through `SPEECH_SOURCE`:
   - Skip sources with no sample or `stream_should_be_playing == false`.
   - Call `process_source_stream` to decode, buffer, and queue audio.
3. If no sources were active, wait for a wake signal (with a reasonable timeout) or until shutdown is requested.
4. If sources were active, yield to avoid busy-spinning.

### 7.4 Source stream processing

`process_source_stream` for each iteration of processed buffers:

1. Unqueue one processed buffer from the mixer source.
2. If the unqueued buffer has a tag, fire the `on_tagged_buffer` callback (for subtitle sync).
3. Remove the corresponding scope data from the ring buffer.
4. If the decoder hit EOF on the previous decode:
   - Fire `on_end_chunk` to request a new decoder. If the callback returns `true`, a new decoder was set.
   - If the callback returns `false`, set `end_chunk_failed = true` and skip further decode attempts on this source until a new stream starts.
5. Decode the next chunk of audio from the current decoder.
6. Upload the decoded data to the mixer buffer and queue it on the source.
7. Fire `on_queue_buffer` callback.
8. Append decoded data to the scope ring buffer if scope is enabled.

If the source is stopped but has buffers queued, restart it. If no buffers remain and the decoder is exhausted, mark `stream_should_be_playing = false` and fire `on_end_stream`.

> **Design note (non-normative):** Callbacks must not be invoked while internal source or sample synchronization is held. The current implementation uses a deferred-callback collection pattern to achieve this. Any equivalent strategy that prevents deadlock between callbacks and internal state is acceptable.

### 7.5 Play stream

`play_stream(sample, source_index, looping, scope, rewind)`:

1. Stop any existing stream on the source.
2. Fire `on_start_stream` callback. If it returns `false`, abort with end-of-stream.
3. Clear all buffer tags.
4. Handle rewind (seek decoder to 0) or compute offset from current decoder position.
5. Attach the sample to the source.
6. Allocate scope buffer if `scope` is true.
7. Pre-fill buffers: decode and queue up to `num_buffers` buffers. On EOF during pre-fill, fire `on_end_chunk` for decoder replacement.
8. Set `start_time`, clear `pause_time`, set `stream_should_be_playing = true`.
9. Start mixer source playback.
10. Wake the decoder thread.

`play_stream_with_offset_override` is the same but accepts an explicit offset value (in milliseconds), used by page-based seek operations.

### 7.6 Stop / pause / resume / seek

- **`stop_stream(source_index)`**: Stop the mixer source, clear `stream_should_be_playing`, detach the sample, release the scope buffer, clear all scope/pause/queue state, reset `end_chunk_failed`.
- **`pause_stream(source_index)`**: Set `stream_should_be_playing = false`, record `pause_time`, pause the mixer source.
- **`resume_stream(source_index)`**: Adjust `start_time` to account for pause duration, clear `pause_time`, set `stream_should_be_playing = true`, resume the mixer source.
- **`seek_stream(source_index, pos_ms)`**: Stop the mixer source, seek the decoder to the PCM frame corresponding to `pos_ms`, then restart the stream (non-rewinding) to re-fill buffers from the new position.

### 7.7 Queries

- **`playing_stream(source_index)`**: Returns `stream_should_be_playing` for the source.
- **`get_stream_position_ticks(source_index)`**: Returns `GetTimeCounter() - source.start_time`, clamped to non-negative. This is the authoritative position for track seeking.

### 7.8 Scope / oscilloscope

Scope data is a per-source ring buffer (`sbuffer`) that accumulates decoded PCM as buffers are queued and removes it as buffers are unqueued. The scope buffer size is `num_buffers * BUFFER_SIZE + PAD_SCOPE_BYTES`.

`graph_foreground_stream(data, width, height, want_speech)`:

1. Select source: prefer `SPEECH_SOURCE` if `want_speech` is true and a non-null decoder is attached; otherwise fall back to `MUSIC_SOURCE`.
2. Read samples from the scope ring buffer at a time-delta-adjusted read position.
3. For each of `width` output samples, read and sum channels, compute energy, track max amplitude, and write AGC-normalized y-coordinate to `data[x]`.
4. Update the persistent AGC state (page/frame accumulation) for smooth waveform normalization across calls.
5. Return 1 on success, 0 on failure (matches C convention).

### 7.9 Fade

`set_music_stream_fade(how_long, end_volume)`:

1. Record the current music volume as `start_volume`.
2. Set fade `start_time`, `interval`, and `delta`.
3. The decoder thread calls `process_music_fade()` each iteration, linearly interpolating the volume and calling `set_music_volume` until the interval elapses.

---

## 8. Track assembly and playback

**Cross-boundary contract note:** The track player is the audio-heart's implementation of the trackplayer contract required by the comm subsystem (`comm/specification.md` §6A). The comm subsystem is the primary consumer and defines the behavioral requirements: phrase completion signaling, main-thread callback dispatch, subtitle ordering, and skip/seek semantics. This section describes the audio-heart's internal implementation of that contract. Where this section describes callback or completion behavior, it must be consistent with comm §6A — specifically, phrase callbacks are never invoked directly by the audio/decoder thread; they are signaled as pending completions and consumed by the comm subsystem's main-thread poll loop.

### 8.1 Track assembly

The track player maintains an ordered sequence of `SoundChunk` nodes. New chunks are appended to the tail.

**`splice_track(track_name, track_text, timestamp, callback, decoders)`**:

1. If `track_text` is absent/null, return immediately (no content to splice).
2. If `track_name` is absent/null (subtitle-only append):
   - Append the first sub-page's text to the last subtitle chunk's text.
   - Create new chunks for subsequent sub-pages (with no decoder).
3. If this is the first track (`track_count == 0`), create the shared `SoundSample` with `NUM_BUFFERS_PER_SOURCE` buffers and track callbacks.
4. Split `track_text` into sub-pages (see §8.6 for complete subtitle paging rules).
5. Parse explicit timestamps from `timestamp` string if provided.
6. Handle `no_page_break` continuation: if set, append the first page's text to the previous subtitle but still create an audio-only chunk from the decoder.
7. For each page/decoder pair:
   - Create a `SoundChunk` with decoder, start_time (ms), run_time (game ticks), tag_me, track_num, text, and callback (callback attached to first tagged chunk only). The callback is stored with the chunk but is not invoked by the track player directly; it is delivered to the main thread via the pending-completion mechanism described in §8.3.
   - Advance `dec_offset` by the decoder's length in milliseconds.
   - Append to the sequence; if the chunk has text, update `last_sub`.
8. Increment `track_count` (unless this was a `no_page_break` continuation).

**`splice_multi_track(tracks, texts, timestamp)`**:

1. Require `track_count > 0` (a base track must exist).
2. For each non-null track in `tracks` (up to `MAX_MULTI_TRACKS`):
   - Load the real decoder for the track.
   - Create a chunk with the decoder and appropriate timing.
   - Advance `dec_offset` by the decoder's length in milliseconds.
   - Append to the sequence.
3. Append subtitle text from `texts[0]` to `last_sub`.
4. Set `no_page_break = true` for the next splice.

**Relationship to canonical loading:** Track assembly (`splice_track`, `splice_multi_track`) is not itself a canonical file-loading entry point as defined in §14.4. When these operations receive resource names that require decoder creation, they acquire decoders through the loading infrastructure but are defined as a separate decoder-acquisition path. The resulting decoders must satisfy the same ownership and error semantics as file-loaded content: failed decoder acquisition for a track resource produces a chunk with no decoder (which the stream engine handles as end-of-chunk), and ownership of successfully acquired decoders transfers to the chunk sequence.

### 8.2 Track playback

**`play_track(scope)`**:

1. Compute total track length via `tracks_end_time_inner` and store in `tracks_length`.
2. Set `cur_chunk` to the head of the chunk sequence.
3. Call `play_stream(sample, SPEECH_SOURCE, false, scope, true)`.

**`stop_track()`**:

1. Stop the speech stream.
2. Clear `track_count`, `tracks_length`, `cur_chunk`, `cur_sub_chunk`.
3. Destroy the shared sample's mixer resources.
4. Drop the entire chunk sequence.
5. Reset all track state (`dec_offset`, tail/sub references).

**`jump_track()`**: Stop the speech stream and clear `cur_chunk` and `cur_sub_chunk` (effectively jumps past end).

**`pause_track()`** / **`resume_track()`**: Delegate to `pause_stream` / `resume_stream` on `SPEECH_SOURCE`. Resume is a no-op if `cur_chunk` is absent.

**`playing_track()`** / **`PlayingTrack()`**: Returns the current 1-based track number (`cur_chunk.track_num + 1`), or 0 if no current chunk (i.e., no track is active). The C-facing `PlayingTrack` FFI entry point returns this integer value directly. Consumers (notably the comm subsystem) use the return value both as a boolean test (nonzero = active) and as an ordinal track identifier. Per the §8.3.1 invariant, `PlayingTrack()` remains nonzero during the pending-completion window (including at end-of-track) until the main thread commits advancement or `StopTrack` clears the track. It transitions to 0 only after `CommitTrackAdvancement()` clears the last phrase (no next phrase) or after `StopTrack`. This is the authoritative definition of `PlayingTrack` semantics at the consumer boundary.

### 8.3 Track callbacks (StreamCallbacks implementation)

The track callbacks integrate the track player with the stream engine:

- **`on_start_stream`**: Takes the decoder from the current chunk, sets the sample's offset from the chunk's start_time, fires the chunk's tag if `tag_me` is set. Returns `false` if no current chunk or no sample.
- **`on_end_chunk`**: Returns the current decoder to the current chunk, advances `cur_chunk` to the next chunk, takes the next chunk's decoder (rewinding it to position 0), tags the buffer if the next chunk has `tag_me` set. Returns `true` (decoder replaced) or `false` (no next chunk).
- **`on_end_stream`**: Clears `cur_chunk` and `cur_sub_chunk`.
- **`on_tagged_buffer`**: Validates the tag's chunk identifier against the active sequence, then records a **pending phrase completion** for that chunk's callback and updates `cur_sub_chunk`. The chunk's callback is not invoked directly by the stream engine; instead, the pending completion is consumed by the main-thread poll loop as required by the comm subsystem's trackplayer contract (see `comm/specification.md` §6A.8). This ensures phrase callbacks always execute on the main thread.

### 8.3.1 Pending-completion provider-side state machine

This section defines the trackplayer-side state transitions for the pending-completion handshake required by `comm/specification.md` §6A.8. Audio-heart is the provider of this mechanism; comm is the consumer.

**State:** The trackplayer maintains a single-slot pending completion state. At any time, this slot is either empty or contains exactly one pending phrase callback.

**Transitions:**

1. **Record:** When `on_tagged_buffer` fires for the last chunk of a logical phrase (see §8.3.2 for phrase-to-chunk mapping), the trackplayer stores the phrase's callback in the pending-completion slot. The trackplayer shall not record a second pending completion before the first is claimed and cleared by the main thread (`comm/specification.md` §6A.8). Under seek or stop race conditions, the trackplayer shall defer or suppress the new completion until the existing one is claimed, or shall discard the racing completion if StopTrack semantics apply (step 4).
2. **Claim-and-clear:** The main-thread poll loop atomically reads and clears the pending-completion slot. This is an atomic operation: after claim-and-clear, the slot is empty regardless of what the stream engine does concurrently. The claimed callback is then invoked on the main thread.
3. **Advancement commit:** After the callback returns, the main thread signals the trackplayer to advance: the completed phrase becomes committed, the next logical phrase becomes current, `PlayingTrack()` is updated, and the next phrase's subtitle becomes available via `GetTrackSubtitle()`. The next phrase does **not** become current until this advancement commit is received.
4. **StopTrack interaction:** `StopTrack` discards any pending completion in the slot without invoking the callback. After StopTrack, the slot is empty and no advancement commit occurs.
5. **Seek interaction:** Seeking past a phrase boundary triggers completion recording (step 1). If a completion is already pending and unclaimed, the seek-triggered completion shall be deferred until the existing completion is claimed and its advancement commit (step 3) is processed. The main-thread poll loop handles at most one completion per iteration; seek-triggered completions wait for the slot to be available.

**Invariant:** The next phrase does not become current, and `PlayingTrack()` does not reflect the next phrase's track number, until the advancement commit (step 3) is received from the main thread. This ensures callbacks observe a consistent state where the just-finished phrase is still "current."

**Consumer-facing boundary operations:** The trackplayer shall expose two distinct operations for the comm subsystem's main-thread poll loop:

| Operation | Purpose | Effect |
|---|---|---|
| `PollPendingTrackCompletion()` | Claim-and-clear (step 2). Returns the pending callback if present, or null if no completion is pending. Atomically clears the slot. | Main thread now holds the callback; slot is empty. |
| `CommitTrackAdvancement()` | Advancement commit (step 3). Called by the comm poll loop after the callback returns. Advances the trackplayer to the next logical phrase. | Next phrase becomes current; `PlayingTrack()` and `GetTrackSubtitle()` update. |

These are the concrete boundary operations referenced by `comm/specification.md` §6A.8. The exact Rust function signatures and module placement are implementation details; the behavioral contract is that these two operations are distinct, ordered (poll → callback → commit), and callable only from the main thread. `StopTrack` implicitly clears any pending completion without requiring a `CommitTrackAdvancement()` call.

### 8.3.2 Phrase-to-chunk mapping at the comm boundary

A single `splice_track` call may create multiple internal `SoundChunk` nodes (one per subtitle page/decoder pair). At the comm boundary, these chunks compose a **single logical phrase** with one completion event, one history entry, and one replay-target update. Completion is emitted only after the **last chunk** of the logical phrase finishes playback (or is skipped). The callback, attached to the first tagged chunk, fires as a pending completion only when all chunks in the logical phrase have been consumed. Subtitle page transitions within a single phrase are internal presentation events, not phrase-completion boundaries. This mapping is consistent with `comm/specification.md` §6A.1, which says each `SpliceTrack` call produces exactly one phrase.

### 8.4 Track seeking

**Smooth seeking** (`fast_reverse_smooth`, `fast_forward_smooth`):

1. Get current position via `get_stream_position_ticks(SPEECH_SOURCE)`.
2. Subtract or add `ACCEL_SCROLL_SPEED` (game ticks).
3. Call `seek_to_position`.

**Page seeking** (`fast_reverse_page`, `fast_forward_page`):

1. Find the previous or next tagged chunk relative to `cur_sub_chunk`.
2. Set `cur_chunk` and `cur_sub_chunk` to the target.
3. Call `play_stream_with_offset_override` with the chunk's `start_time` (milliseconds) as the offset.
4. For forward page with no next page: seek to `tracks_length + 1` (triggers end-of-track).

**`seek_to_position(pos)`** (internal, tick-based):

1. Clamp `pos` to `tracks_length + 1`.
2. Walk the chunk sequence to find the chunk whose time range contains `pos`.
3. Track the last tagged chunk seen for subtitle state.
4. Update `cur_sub_chunk` via `do_track_tag_inner` if a tagged chunk was found.
5. Update the source's `start_time` to keep the timebase coherent.
6. Return the currently-attached decoder to the old `cur_chunk`.
7. Take the target chunk's decoder, seek it to the relative offset within the chunk.
8. Attach the decoder to the sample and set the sample's offset.
9. If the stream is not playing, restart it.

### 8.5 Track position

`get_track_position(in_units)`:

- If `in_units == 0`: return raw position in game ticks (clamped to track length).
- If `in_units > 0`: return `in_units * pos / length` (proportional).

### 8.6 Subtitle paging rules

This section defines the complete subtitle paging algorithm used by `splice_track` to split `track_text` into display pages. These rules are ABI-visible because they determine the number, content, and timing of subtitle chunks exposed through the subtitle iteration APIs and the `GetTrackSubtitle` polling contract.

**Page splitting:**

1. The input text is split into pages at each carriage return (`\r`) or newline (`\n`) character.
2. A `\r\n` sequence (CR followed immediately by LF) is treated as a single page break, not two.
3. Trailing empty pages produced by trailing break characters are not emitted as separate chunks.
4. Leading break characters produce an empty first page (which may receive continuation text from a preceding splice if `no_page_break` is active).

**Continuation markers:**

5. If a page break occurs and the preceding page does not end at a word boundary (i.e., the character immediately before the break is not whitespace), the preceding page receives a `...` (ellipsis) suffix to indicate the sentence continues.
6. Every page after the first receives a `..` (double-dot) prefix to indicate it is a continuation of the preceding page's text.

**Per-page timing:**

7. Each page's display duration is computed as `max(character_count * TEXT_SPEED, 1000)` milliseconds, where `character_count` is the length of the page text after applying continuation markers, and `TEXT_SPEED` is 80 ms/character.
8. If explicit timestamps are provided via the `timestamp` parameter, they override computed timing for pages where the timestamp string supplies a value. Explicit timestamps are parsed as comma-separated integer millisecond values.

**Encoding:**

9. Subtitle text is treated as a byte string. Paging splits on `\r` and `\n` byte values only. Non-ASCII content is passed through without interpretation; the game's string encoding is the caller's responsibility.

---

## 9. Subtitle / speech coordination

### 9.1 Buffer tagging mechanism

When a chunk has `tag_me = true`, the stream engine attaches a `SoundTag` to the buffer when it is queued. The tag's `data` field holds a chunk identifier (used for subtitle matching). When the mixer reports that tagged buffer as processed, the `on_tagged_buffer` callback validates the identifier against the active chunk sequence and updates `cur_sub_chunk`.

### 9.2 Subtitle access API

- **`get_track_subtitle()`**: Returns a copy of the current subtitle chunk's text.
- **`get_track_subtitle_cstr()`**: Returns a stable C-string pointer to a lazily-cached version. This pointer remains identical (same address) as long as `cur_sub_chunk` does not change. This is critical for the C comm subsystem's `CheckSubtitles` function, which uses pointer-identity comparison to detect subtitle changes.
- **`get_first_track_subtitle()` / `get_next_track_subtitle()`**: Subtitle iteration for the summary/review paging UI in comm.
- **Chunk-pointer iteration API**: Raw-pointer iteration matching the C `SUBTITLE_REF` contract. These validate that the provided pointer/identifier belongs to the active chunk sequence before dereferencing.

### 9.3 Comm integration contract

The comm subsystem polls `GetTrackSubtitle()` (via `get_track_subtitle_cstr()`) in its subtitle update loop. The C-facing pointer lifetime contract for `rust_GetSubtitle` is defined by `comm/specification.md` §14.3, which is the normative owner of the consumer-facing subtitle pointer contract. Audio-heart must satisfy that contract. The key requirements are:

1. The returned pointer is valid until the next call into the comm subsystem that may modify subtitle state. C callers must copy or consume the string before making further comm API calls.
2. Pointer identity changes only when the subtitle actually changes (same address returned for repeated polls of the same subtitle).
3. The text content is UTF-8 / ASCII compatible (matching the game's string encoding).
4. When track playback ends or is stopped, the pointer becomes null.

**Implementation note:** Internally, the lazily-cached `text_cstr` on each `SoundChunk` provides stable pointer identity for as long as `cur_sub_chunk` does not change, which is sufficient to satisfy the comm contract. The comm contract is intentionally narrower than the internal pointer lifetime to allow safe FFI without requiring callers to reason about track-player-state lifetime.

---

## 10. Music behavior

### 10.1 State

The music module maintains:

- `cur_music_ref` — currently playing music reference.
- `cur_speech_ref` — currently playing speech reference.
- `music_volume` — current music volume (0..`MAX_VOLUME`).
- `music_volume_scale` — music volume scale factor (0.0..1.0).

### 10.2 Playback

- **`plr_play_song(music_ref, continuous, priority)`**: Play the music reference on `MUSIC_SOURCE` with `looping = continuous`, `scope = true`, `rewind = true`. Store as `cur_music_ref`. The `priority` parameter is accepted for ABI compatibility but is not used to influence playback behavior, matching the historical C implementation.
- **`plr_stop(music_ref)`**: If `music_ref` matches `cur_music_ref` (by reference identity), stop the music stream and clear `cur_music_ref`.
- **`plr_playing(music_ref)`**: Returns `true` if `music_ref` matches `cur_music_ref` and the stream is playing.
- **`plr_seek(music_ref, pos)`**: If matching, seek the music stream.
- **`plr_stop_current()` / `plr_playing_current()` / `plr_seek_current(pos)`**: Sentinel variants that operate on whatever music is currently active (matching C's `~0` / wildcard semantics).
- **`plr_pause()` / `plr_resume()`**: Pause/resume the music source.

### 10.3 Volume and fade

- **`set_music_volume(volume)`**: Clamp to 0..`MAX_VOLUME`, compute effective gain as `music_volume_scale * (volume / MAX_VOLUME)`, apply to the music source via the mixer's gain property.
- **`current_music_volume()`**: Returns the stored `music_volume`.
- **`fade_music(how_long, end_volume)`**: If `how_long == 0`, set volume immediately. Otherwise delegate to `set_music_stream_fade` which starts the fade; the decoder thread applies it progressively.

### 10.4 PLRPause semantics

`PLRPause` at the ABI boundary accepts a music-reference argument. The required behavior matches the historical C implementation: pause only when the supplied reference matches the current music reference or is the wildcard sentinel (`~0`). A non-matching, non-wildcard reference must leave playback unchanged.

---

## 11. Speech behavior

### 11.1 Standalone speech

- **`snd_play_speech(music_ref)`**: Play the speech reference on `SPEECH_SOURCE` with `looping = false`, `scope = false`, `rewind = true`. Store as `cur_speech_ref`.
- **`snd_stop_speech()`**: Stop the speech stream and clear `cur_speech_ref`.

### 11.2 Speech-source arbitration

Both track playback and standalone speech use `SPEECH_SOURCE`. The arbitration rule is **asymmetric: track playback always has priority over standalone speech.** The complete behavior for every ordering is:

1. **Track-first, then standalone speech request:** The standalone speech request is a **silent no-op** — the function returns without starting playback and without modifying `cur_speech_ref`. No error code is returned to the caller. Diagnostic logging of the rejection is permitted but is not part of the ABI contract.
2. **Standalone-speech-first, then track request:** Track playback **does** proceed. The track player takes ownership of `SPEECH_SOURCE` as part of `play_track`, which calls `play_stream` on `SPEECH_SOURCE`. The `play_stream` call stops any existing stream on the source before starting the new one (see §7.5 step 1), so the standalone speech stream is stopped as a side effect. `cur_speech_ref` is **not** automatically cleared by this sequence — it retains its stale value until a subsequent `snd_stop_speech()` call clears it.
3. **`snd_stop_speech()` while a track program owns the source:** Clears `cur_speech_ref` only. Does not affect track playback or track state in any way. The speech stream stop is not forwarded to the source because the track player owns it.
4. **`stop_track()` while standalone speech had previously been rejected or stopped:** After `stop_track()` completes, `SPEECH_SOURCE` is free. Subsequent `snd_play_speech()` calls are valid and will start standalone speech normally.

This is an asymmetric rule: track playback always wins regardless of ordering. Standalone speech never prevents or delays track playback. This matches the historical C behavior where the game never issues standalone speech during comm dialogue, but the subsystem must be safe if it happens.

---

## 12. SFX behavior

### 12.1 Channel playback

- **`play_channel(channel, sound_bank, sound_index, pos, positional_object, priority)`**:
  1. Validate `channel <= LAST_SFX_SOURCE`.
  2. Stop any existing playback on the channel.
  3. Clean up finished SFX channels.
  4. Look up `sound_bank.samples[sound_index]`.
  5. Apply positional audio if stereo SFX is enabled.
  6. Bind the sample's first buffer to the source via the mixer's buffer property.
  7. Start mixer playback.

  The `priority` parameter is accepted for ABI compatibility but is not used to influence channel-stealing or interruption policy, matching the historical C implementation.

- **`play_sample(channel, sample, pos, positional_object, priority)`**: Same as above but operates on a single pre-decoded `SoundSample` directly (used from FFI for samples loaded outside the bank system). The `priority` parameter is accepted for ABI compatibility but has no behavioral effect.

- **`stop_channel(channel, priority)`**: Delegates to `stop_source`. The `priority` parameter is accepted for ABI compatibility but has no behavioral effect.

- **`channel_playing(channel)`**: Queries the mixer source state; returns `true` if the state is `Playing`.

### 12.2 Positional audio

`update_sound_position(source_index, pos)`:

- If `pos.positional`: compute normalized X/Z coordinates (divided by `ATTENUATION`), enforce `MIN_DISTANCE`, set via the mixer's position properties.
- If non-positional: center the source at (0, 0, -1).

`get_positional_object` / `set_positional_object`: Direct getter/setter on the source's `positional_object` field.

### 12.3 Volume

- **`set_channel_volume(channel, volume, priority)`**: Compute gain as `(volume / MAX_VOLUME) * sfx_volume_scale`, apply via the mixer's gain property. The `priority` parameter is accepted for ABI compatibility but has no behavioral effect.
- **`set_sfx_volume(volume)`**: Update `sfx_volume_scale` and apply gain to all SFX sources.

### 12.4 Cleanup

`check_finished_channels()`: Iterate all SFX channels; if a source's mixer state is `Stopped`, call `stop_source` to clean it up. Called automatically before each `play_channel`.

---

## 13. Lifecycle and control APIs

### 13.1 Initialization and shutdown lifecycle

The subsystem has two initialization pairs. They serve distinct roles and have a required call sequence:

| Function | Role |
|---|---|
| `init_sound()` | One-time high-level audio-heart state preparation. Currently a success-returning no-op because all substantive state is created by `init_stream_decoder`. Present as a lifecycle hook for any future per-init work not tied to streaming. |
| `init_stream_decoder()` | Allocates mixer sources, starts the mixer pump, starts the decoder thread. This is the function that makes the subsystem usable for playback. |
| `uninit_stream_decoder()` | Stops the mixer pump and decoder thread, releases stream-related resources. |
| `uninit_sound()` | Releases any high-level audio-heart state not already released by `uninit_stream_decoder`. Currently a no-op. |

**Required call sequence:**

1. `init_sound()` must be called before `init_stream_decoder()`.
2. `init_stream_decoder()` must be called before any playback, loading, or query API.
3. Shutdown is symmetric: `uninit_stream_decoder()` first, then `uninit_sound()`.

**Idempotence and double-call behavior:**

- `init_stream_decoder()` called twice without intervening `uninit_stream_decoder()` shall be rejected with an error. It must not duplicate worker threads or mixer sources.
- `uninit_stream_decoder()` called without a preceding `init_stream_decoder()` is a no-op.
- `init_sound()` and `uninit_sound()` are idempotent — calling either multiple times has no additional effect.

**Behavior of APIs before `init_stream_decoder()`:**

All FFI-exposed APIs called after `init_sound()` but before `init_stream_decoder()` shall produce the ABI-defined failure outcome for the specific function, as listed in §19.3. Specifically:

- Functions returning a pointer (e.g., `LoadMusicFile`, `LoadSoundFile`, `GetTrackSubtitle`, `TFB_CreateSoundSample`) shall return null.
- Functions returning an integer success/failure code (e.g., `GraphForegroundStream`) shall return the failure value (`0`).
- Functions returning a boolean/playing-state (e.g., `SoundPlaying`, `ChannelPlaying`, `PLRPlaying`, `PlayingTrack`) shall return false/`0`.
- Void functions (e.g., `PLRPlaySong`, `PlayChannel`, `StopSound`, `SpliceTrack`) shall return without modifying externally visible playback state (silent no-op).
- `WaitForSoundEnd` shall return immediately without blocking.

This is a defined, testable contract. The historical C implementation did not guard this case, but the ABI failure map provides natural outcomes that require no additional mechanism beyond detecting the uninitialized state.

**Logical relationship:**

- `init_sound()` / `uninit_sound()` and `init_stream_decoder()` / `uninit_stream_decoder()` are logically separate functions. `init_sound()` is not merely a wrapper around `init_stream_decoder()`. The historical C implementation treated them as separate lifecycle stages, and the end state preserves that separation even though `init_sound` currently performs no work.

### 13.2 Global control

- **`stop_sound()`**: Stop all SFX channels (sources 0 through `LAST_SFX_SOURCE`).
- **`stop_source(source_index)`**: Stop the source and clean it up (unqueue buffers, rewind, clear positional object).
- **`clean_source(source_index)`**: Unqueue processed buffers and rewind the mixer source.

### 13.3 Global queries and wait-for-end

- **`sound_playing()`**: Returns `true` if any source is currently playing. For streaming sources, checks `stream_should_be_playing`. For non-streaming SFX, queries the mixer state.

- **`wait_for_sound_end(channel)`**: Blocks until the specified playback activity has stopped:

  - **Selector domain**: The `channel` argument selects what to wait on. Valid values are:
    - Any integer in the range `0` through `NUM_SOUNDSOURCES - 1` (i.e., 0–6): wait for that single source.
    - The sentinel value `WAIT_ALL_SOURCES` (`~0`, i.e., `UINT32_MAX`): wait for all managed sources.
    - Any other value outside the range `0` through `NUM_SOUNDSOURCES - 1` that is not `WAIT_ALL_SOURCES` is treated identically to `WAIT_ALL_SOURCES`. This matches the historical C behavior where the "all" path was the default/else branch.
  - **Scope of "all"**: When waiting for all sources, the wait covers SFX channels, music, and speech — every source in the source table.
  - **Stopped vs. paused**: A paused source is considered **still active** for wait purposes. The wait does not return until the source is fully stopped (not merely paused).
  - **Inactive-source rule**: A source is considered inactive (not playing) for wait purposes if any of the following is true: (a) the source has no attached sample, (b) the source has no allocated mixer handle (e.g., `init_stream_decoder` was never called or `uninit_stream_decoder` has already released it), or (c) the source's streaming flag is false and the mixer reports no active playback state. The paused-stream exception in the preceding rule takes priority: a paused source with an attached sample and allocated mixer handle is still active.
  - **Polling behavior**: The wait polls source state at a short interval. The historical implementation uses approximately 10ms; implementations may use any equivalent mechanism (polling, condvar, event-based wait) with any wake cadence, as long as the wait terminates promptly (within a few tens of milliseconds) after all selected sources stop. The specific interval is **not normative**.
  - **Shutdown**: If `QuitPosted` becomes true during the wait, the wait exits promptly regardless of source state. This applies whether shutdown begins before the wait starts or during the wait — in both cases, the wait observes the flag and exits.
  - **Concurrent teardown**: If source state is being torn down concurrently (e.g., during shutdown), the wait must not access freed resources. It should observe shutdown signaling and exit.

### 13.4 Volume control

- **`set_sfx_volume(volume)`**: Set SFX volume scale and apply to all SFX sources.
- **`set_speech_volume(volume)`**: Set speech volume scale and apply to `SPEECH_SOURCE`.
- **`set_music_volume(volume)`**: Set music volume and apply to `MUSIC_SOURCE` (see §10.3).
- **`fade_music(how_long, end_volume)`**: Start or immediately apply a music volume fade (see §10.3).

---

## 14. Resource loading

### 14.1 Music loading

**Required end-state behavior:** There is a single canonical music-loading implementation that all entry points — internal and FFI — route through. That implementation:

1. Acquires the file-load guard (ensuring `cur_resfile_name` is cleared on all exit paths).
2. Validates the filename: the filename must be a non-empty string. The canonical loader delegates to UIO for path resolution; there is no subsystem-level path syntax validation beyond non-emptiness.
3. Opens the file via UIO, creates an appropriate decoder, creates a `SoundSample` with the decoder attached, and returns a `MusicRef`.

**FFI path** (`LoadMusicFile`): Translates C arguments and delegates to the canonical implementation, then wraps the result as an opaque handle for C.

**`DestroyMusic(music_ref_ptr)`**: Reconstruct ownership of the handle, stop if active, destroy the sample's mixer resources, release the handle.

> **Current-state note (non-normative):** The current implementation has two separate loading paths — a real loader at the FFI boundary and a stub internal helper. The end state requires consolidation into a single canonical implementation. See §14.4.

### 14.2 Sound bank loading

**Required end-state behavior:** There is a single canonical sound-bank-loading implementation that all entry points — internal and FFI — route through. That implementation:

1. Acquires the file-load guard.
2. Parses the bank file (listing of audio filenames), loads a decoder for each entry, pre-decodes all audio, uploads the decoded PCM to mixer buffers, and returns a `SoundBank` with fully-populated samples.

**FFI path** (`LoadSoundFile`): Translates C arguments, delegates to the canonical implementation, then builds a C-compatible `STRING_TABLE` with entries pointing to the samples (for the C resource system's SOUND/SOUND_REF convention) and returns the opaque pointer.

**`DestroySound(bank_ptr)`**: Reconstruct the bank, destroy all mixer buffers, free the C string table structure.

> **Current-state note (non-normative):** The current implementation has a real FFI loader and a stub internal helper that returns an empty bank. The end state requires consolidation. See §14.4.

### 14.3 File load guard

The file-load guard is a scoped guard mechanism that sets `cur_resfile_name` on acquisition and clears it on exit, regardless of success or failure. Only one file can be loading at a time; concurrent loads return an error. This matches the C `cur_resfile_name` guard pattern.

### 14.4 Loader consolidation

There must be a single canonical loading implementation per resource type. All entry points — internal and FFI — must route through the same implementation so that validation, decode behavior, error handling, and ownership semantics remain consistent. The current state where internal helpers are stubs while real loading only happens at the FFI boundary is not acceptable as a final state and must be resolved.

**Scope of canonicalization:** Canonical loading applies to all paths that produce playable content from file-backed resources. This includes both the `LoadMusicFile`/`LoadSoundFile` FFI entry points and the internal `fileinst`/`get_music_data`/`get_sound_bank_data` paths.

**Decoder adoption from external inputs:** When decoders or sample-like objects are received from outside the canonical loading path (e.g., pre-created decoders passed via FFI for track assembly, or samples constructed by `TFB_CreateSoundSample`), the subsystem accepts them as-is without re-running file-loading validation. These inputs have already been constructed by their creator. The canonical loading path's validation and decode logic applies only to file-backed resource construction. The shared contract for all playable content — regardless of origin — is the `SoundSample` interface and its ownership/lifecycle rules (§4.1, §15).

### 14.5 Resource-name validation

The audio-heart subsystem validates resource names at the loading boundary:

- The filename must be a non-empty string. A null or empty filename causes the load to fail with a structured error.
- Beyond non-emptiness, the subsystem delegates path resolution and existence checking to UIO. If UIO cannot open the path, the load fails with a resource-not-found error.
- There is no additional subsystem-level validation of path syntax, character set, or directory structure. UIO is the authority for path validity.

### 14.6 Resource loading before `init_stream_decoder()`

File-backed loading APIs (`LoadMusicFile`, `LoadSoundFile`) require `init_stream_decoder()` to have completed successfully. If called before `init_stream_decoder()`, they return null.

This is required because loading creates mixer-ready objects (buffer allocation, decoder-to-buffer upload) that depend on initialized mixer state. Music loading requires mixer buffer allocation for the sample, and sound-bank loading requires mixer buffer allocation and PCM upload for every entry.

The `TFB_CreateSoundSample` API, which creates an empty sample shell without performing file-backed loading or mixer buffer allocation, is also subject to this rule: it returns null before `init_stream_decoder()`. While a zero-buffer sample does not strictly require mixer state, consistent pre-init behavior across the API surface is preferred over per-function exceptions.

---

## 15. ABI handle ownership rules

### 15.1 General rules

Every opaque handle returned to a C caller has these properties:

- **Single-owner semantics at the ABI boundary.** Each call to `LoadMusicFile` or `LoadSoundFile` returns a unique handle. The C caller owns that handle and is responsible for exactly one `Destroy` call.
- **Handles are not aliasable by C callers.** The C caller must not duplicate or share the raw pointer. Internal reference counting may exist for implementation purposes, but the ABI contract is single-owner.
- **Playback retains internal ownership.** After a play call (e.g., `PLRPlaySong`, `PlayChannel`), the subsystem holds its own internal reference to the underlying data. Destroying the handle while playback is active is safe — the subsystem must stop or detach active playback before releasing internal references.
- **Null destroy is a no-op.** Passing a null pointer to `DestroyMusic` or `DestroySound` has no effect.
- **Destroyed handles are immediately invalid.** After destruction, the handle must not be used for equality comparisons, queries, or any other operation.

### 15.2 Double-destroy and stale-handle semantics

Double-destroy (calling a destroy operation on an already-destroyed handle) is **undefined behavior**. The subsystem is not required to detect or gracefully handle it.

This is the accepted contract, not an open decision. The rationale:

- The historical C implementation did not detect or guard against double-destroy.
- The ABI contract is single-owner: callers are responsible for exactly one destroy call per handle.
- Requiring detection would impose runtime overhead (e.g., a global handle registry) that is disproportionate for a compatibility replacement and that the original C implementation did not provide.

However, double-destroy **must not corrupt unrelated subsystem state**. The undefined behavior is scoped to the destroyed handle itself — the subsystem may crash, return garbage, or silently misbehave on the stale handle, but it must not corrupt the source table, other live handles, or other subsystem state that is unrelated to the stale handle.

Stale-pointer use (using a handle after destroy for operations other than destroy) is similarly undefined behavior, with the same corruption-containment constraint.

> **Design note (non-normative):** A future hardening pass may add debug-mode detection (e.g., poisoning freed handle memory, maintaining a debug handle registry). This is not required by the current contract but would be a welcome defensive improvement.

### 15.3 Handle identity and equality semantics

Control and query APIs (`plr_stop`, `plr_playing`, `plr_seek`, `PLRPause`) compare a caller-supplied handle against the currently active handle to determine whether the operation applies. The following rules govern handle identity:

- **Equality is raw-handle identity.** Two handles are considered equal if and only if they are the same raw pointer value. There is no deep comparison of underlying content.
- **Each load produces a distinct handle.** Two separate calls to `LoadMusicFile` with the same file path return distinct handles that are not equal to each other.
- **Internal retained references preserve identity.** When the subsystem borrows a handle during a play call (e.g., `plr_play_song` clones the `MusicRef` internally), the internal reference preserves identity with the caller's handle for comparison purposes. Specifically, the raw pointer that the comparison examines is the same pointer value the caller holds.
- **Null and wildcard sentinels.** Null (`0`) is never equal to any valid handle. The wildcard sentinel (`~0`) is not compared for equality — it bypasses the comparison and applies the operation to whatever is currently active. These two sentinel values are disjoint from the handle identity namespace.
- **Handles become invalid after destroy.** After `DestroyMusic` or `DestroySound`, the handle pointer is invalid and must not be used for comparison. If a stale handle happens to compare equal to a live handle (due to memory reuse), the behavior is governed by the stale-handle rules in §15.2, not by the equality rules here.

### 15.4 MusicRef handles

- `LoadMusicFile` returns a `MusicRef` handle. The caller owns it.
- `PLRPlaySong` borrows the handle without consuming it. The caller retains ownership.
- `PLRStop`, `PLRPlaying`, `PLRSeek` borrow the handle for comparison against the current music reference.
- `DestroyMusic` consumes ownership and releases resources.

### 15.5 SoundBank handles

- `LoadSoundFile` returns a `STRING_TABLE`-based handle. The caller owns it.
- `PlayChannel` borrows the handle to look up a sample by index. The caller retains ownership.
- `DestroySound` consumes ownership and releases all mixer buffers and the C string table structure.

---

## 16. Parity policy

The following rules govern how historical C behavior relates to the end-state specification:

1. **ABI-visible results and side effects must match the historical C implementation** unless a deliberate change is explicitly adopted and documented in this specification.
2. **Internal architecture may differ freely.** The implementation may use different data structures, algorithms, synchronization strategies, and module organization as long as externally observable behavior is preserved.
3. **Bug-compatible behavior is required only where callers depend on it** or where the ABI encodes the bug (e.g., specific return values, side effects, or ordering that callers rely on). Bugs that are purely internal and have no caller-visible effect may be fixed without documentation.
4. **Any intentional deviations from historical behavior must be listed explicitly** in this specification. Deviations must not be discovered accidentally during implementation or testing.
5. **Diagnostic and cleanup changes** (removing warning suppressions, removing `[PARITY]` logging, consolidating duplicate code) are maintenance goals, not parity-affecting changes, and may be done without deviation documentation.

---

## 17. Integration points

### 17.1 Mixer integration

Audio-heart depends on the mixer module for all low-level audio operations:

| Operation | Mixer API |
|---|---|
| Buffer allocation | `mixer_gen_buffers(count)` → buffer handles |
| Buffer deletion | `mixer_delete_buffers(handles)` |
| Buffer data upload | `mixer_buffer_data(handle, format, data, freq, mixer_freq, mixer_fmt)` |
| Source allocation | `mixer_gen_sources(count)` → source handles |
| Source play/stop/pause/rewind | `mixer_source_play/stop/pause/rewind(handle)` |
| Source property set (int) | `mixer_source_i(handle, prop, value)` |
| Source property set (float) | `mixer_source_f(handle, prop, value)` |
| Source property get (int) | `mixer_get_source_i(handle, prop)` |
| Source queue/unqueue buffers | `mixer_source_queue_buffers(handle, bufs)` / `mixer_source_unqueue_buffers(handle, count)` |
| Global mix | `mixer_mix_channels(output_buf)` |
| Global format/frequency | `mixer_get_frequency()` / `mixer_get_format()` |

Audio-heart must not bypass the mixer to interact with audio hardware directly.

### 17.2 Mixer pump lifecycle

The mixer pump is the bridge between the mixer's `mixer_mix_channels` output and the platform audio backend. **Audio-heart owns the mixer pump's lifecycle.** This is a normative part of the audio-heart contract:

- `init_stream_decoder` starts the mixer pump.
- `uninit_stream_decoder` stops the mixer pump.

The mixer pump's internal implementation (platform audio device selection, output format, callback structure) is not part of the audio-heart API contract. Audio-heart owns the lifecycle because it is the subsystem that knows when streaming is needed, and because the historical C implementation managed it from the same initialization/shutdown path. Extracting it to a separate subsystem would require coordinating lifecycle across subsystem boundaries that do not currently exist and would change the scope of "functionally complete replacement."

**Cross-subsystem boundary note:** No other subsystem in this documentation set owns or constrains the mixer-pump worker lifecycle. Audio-heart is the sole owner of mixer-pump start/stop within the 13-subsystem set.

**Threading dependency:** Audio-heart's worker-lifecycle contract (mixer-pump thread and decoder processing thread creation in `init_stream_decoder`, teardown in `uninit_stream_decoder`) is defined against the threading subsystem's currently controlling interim acceptance rules (`threading/specification.md`): immediate thread creation and no stack-size dependency. These interim rules are normative for downstream pass/fail determination per the threading spec. If a future threading audit changes those rules, audio-heart's worker-lifecycle contract will be updated accordingly; until such revision, audio-heart's worker creation/teardown behavior is correct when it conforms to the interim rules. Adjacent subsystems that depend on audio availability (e.g., comm for speech playback) assume that audio-heart's `init_stream_decoder` has been called before they begin audio operations; they do not independently manage mixer-pump lifecycle.

### 17.3 Decoder integration

Audio-heart uses decoders through the `SoundDecoder` trait:

| Operation | Trait method |
|---|---|
| Decode audio | `decode(buf) → decoded byte count or error` |
| Seek to position | `seek(frame) → success or error` |
| Get frequency | `frequency() → sample rate` |
| Get format | `format() → audio format descriptor` |
| Get length | `length() → seconds (f32)` |
| Get current frame | `get_frame() → frame position` |
| Check null | `is_null() → bool` |

Decoder instances are created by the resource loading path (Ogg, WAV, AIFF, DukAud, MOD format detection). Audio-heart never creates decoders directly — it receives them from the loading path or from C via FFI.

### 17.4 UIO / resource integration

File loading depends on C-owned UIO filesystem services:

- `uio_fopen(contentDir, path, mode)` — open a file in the content directory.
- `uio_fread`, `uio_fseek`, `uio_ftell`, `uio_fclose` — standard file operations.
- `contentDir` — C global pointer to the mounted content directory.

Audio-heart also depends on C-owned memory and string table services for certain ABI boundary operations:

- `AllocStringTable` / `FreeStringTable` — for building C `STRING_TABLE` structures for sound bank handles.
- C-compatible memory allocation — for ABI-required allocation patterns.

### 17.5 Comm integration

The comm subsystem interacts with audio-heart primarily through:

1. **Track player**: `SpliceTrack` (called from `NPCPhrase_cb` in `commglue.c`), `PlayTrack`, `StopTrack`, `JumpTrack`, `PauseTrack`, `ResumeTrack`, seek/page functions, `PlayingTrack`, `GetTrackPosition`.
2. **Subtitle access**: `GetTrackSubtitle` (polled by `CheckSubtitles` in `comm.c`), `GetFirstTrackSubtitle` / `GetNextTrackSubtitle` / `GetTrackSubtitleText` (used by summary/review paging).
3. **Oscilloscope**: `GraphForegroundStream` (called from `oscill.c`).

The comm subsystem expects that:

- Subtitle pointers have stable identity until the subtitle advances.
- Track position reporting is coherent with actual audio playback.
- `PlayingTrack()` returns 0 after the last phrase's advancement commit (or after `StopTrack`). During the end-of-track pending-completion window — after the last phrase finishes playback but before the main thread claims the completion and commits advancement — `PlayingTrack()` continues to reflect the just-finished phrase's track number (nonzero), consistent with the §8.3.1 invariant that the current phrase remains current until advancement commit.
- `SpliceTrack` can be called multiple times to build a multi-page conversation before `PlayTrack` starts playback.
- Phrase callbacks are not invoked directly by the audio/decoder thread; instead, phrase completion is signaled as a pending completion consumed by the comm subsystem's main-thread poll loop (see `comm/specification.md` §6A and §8 cross-boundary contract note above).

### 17.6 C timing integration

Audio-heart depends on two C-owned timing/state functions:

- `GetTimeCounter() → u32` — returns the current game time in ticks (`ONE_SECOND = 840` ticks per second). Used for playback position tracking, fade timing, scope timing.
- `QuitPosted → i32` — global flag indicating the game is shutting down. Used by `wait_for_sound_end` to break out of blocking waits.

---

## 18. FFI boundary

### 18.1 Role

The FFI layer is the C ABI translation layer. It is feature-gated and exports C-callable functions matching the declarations in `audio_heart_rust.h`.

### 18.2 Responsibilities

The FFI layer must:

1. **Translate pointer types**: Convert between C opaque pointers and internal types according to the handle ownership rules in §15.
2. **Handle null pointers**: Return early or return null/0 on null input. Never panic on null FFI input.
3. **Translate error codes**: Convert internal errors to C-style return codes (0 = success, -1 = error) or void functions that silently absorb errors with diagnostic output.
4. **Manage ABI-compatible struct layouts**: C-facing struct representations must match the exact C layout. `SoundPosition` must be `#[repr(C)]`.
5. **Borrow handles safely**: When temporarily accessing handle-backed objects without consuming ownership, the implementation must not decrement reference counts or invalidate the handle.
6. **Build C-compatible resource handles**: Construct `STRING_TABLE` structures for sound bank handles to satisfy the C resource system's `SOUND` / `SOUND_REF` conventions.

### 18.3 Feature gate contract

The C preprocessor macro `USE_RUST_AUDIO_HEART` and the Cargo feature `audio_heart` must be enabled together. If the C macro is defined without the Cargo feature, the C header declarations will not have backing symbols, causing link errors. The build system must guarantee both are set in tandem.

---

## 19. Error handling

### 19.1 Error type

The subsystem uses a unified error type with these failure categories:

| Category | Description |
|---|---|
| Not initialized | Subsystem not yet initialized. |
| Already initialized | Double initialization attempt. |
| Invalid source | Source index out of range. |
| Invalid channel | Channel index out of range. |
| Invalid sample | Null or missing sample. |
| Invalid decoder | Null or missing decoder. |
| Decoder error | Decoder-level error (with detail). |
| Mixer error | Mixer-level error. |
| I/O error | File I/O error (with detail). |
| Null pointer | Null pointer passed to an API. |
| Concurrent load | File load already in progress. |
| Resource not found | Resource file not found (with path). |
| End of stream | End of audio stream reached. |
| Buffer underrun | Buffer underrun during decode. |

### 19.2 Error propagation rules

- **Internal APIs** return results and propagate errors to callers.
- **FFI boundary functions** catch all errors, log them via diagnostic output, and return C-compatible error indicators (null pointer, -1, or void with silent absorption).
- **The decoder thread** catches errors per-source and marks `stream_should_be_playing = false` on unrecoverable errors. It must not panic.
- **Panic safety**: The mixer pump output callback must catch panics and produce silence output rather than crashing the process.

### 19.3 ABI failure mode map

The following table defines the failure reporting channel for each public ABI function family. This is the authoritative mapping from API to failure mode at the C boundary.

| API function(s) | Success return | Failure mode | Notes |
|---|---|---|---|
| `InitSound` | `0` (int) | Returns `-1` | |
| `UninitSound` | void | Silent absorption | Logs diagnostics on error |
| `InitStreamDecoder` | `0` (int) | Returns `-1` | Rejects double-init |
| `UninitStreamDecoder` | void | Silent absorption | |
| `LoadMusicFile` | Non-null `MUSIC_REF` | Returns null | Pre-init returns null |
| `LoadSoundFile` | Non-null `SOUND_REF` | Returns null | Pre-init returns null |
| `DestroyMusic` | void | Silent absorption | Null input is a no-op |
| `DestroySound` | void | Silent absorption | Null input is a no-op |
| `PLRPlaySong` | void | Silent absorption | |
| `PLRStop` | void | Silent absorption | Non-matching ref is a no-op |
| `PLRPlaying` | `true` (bool/int) | Returns `false`/`0` | Non-matching ref returns false |
| `PLRSeek` | void | Silent absorption | Non-matching ref is a no-op |
| `PLRPause` | void | Silent absorption | Non-matching non-wildcard ref is a no-op |
| `PLRResume` | void | Silent absorption | |
| `FadeMusic` | void | Silent absorption | |
| `SetMusicVolume` | void | Silent absorption | Out-of-range values are clamped |
| `snd_PlaySpeech` | void | Silent absorption | Track-active rejection is a silent no-op |
| `snd_StopSpeech` | void | Silent absorption | |
| `PlayChannel` | void | Silent absorption | Invalid channel is rejected silently |
| `StopChannel` | void | Silent absorption | |
| `ChannelPlaying` | `true` (bool/int) | Returns `false`/`0` | |
| `SetChannelVolume` | void | Silent absorption | |
| `UpdateSoundPosition` | void | Silent absorption | |
| `StopSound` | void | Silent absorption | |
| `SoundPlaying` | `true` (bool/int) | Returns `false`/`0` | |
| `WaitForSoundEnd` | void (returns when done) | Silent absorption | Exits on shutdown; returns immediately pre-init |
| `GraphForegroundStream` | `1` (int) | Returns `0` | |
| `SpliceTrack` | void | Silent absorption | |
| `SpliceMultiTrack` | void | Silent absorption | Requires existing base track |
| `PlayTrack` | void | Silent absorption | |
| `StopTrack` | void | Silent absorption | |
| `JumpTrack` | void | Silent absorption | |
| `PauseTrack` | void | Silent absorption | |
| `ResumeTrack` | void | Silent absorption | |
| `PlayingTrack` | Track number (int) | Returns `0` | |
| `GetTrackPosition` | Position value (int) | Returns `0` | |
| `GetTrackSubtitle` | Non-null C string | Returns null | No active subtitle |
| `GetFirstTrackSubtitle` | Non-null subtitle ref | Returns null | No active track program |
| `GetNextTrackSubtitle` | Non-null subtitle ref | Returns null | End of iteration or no active track program |
| `GetTrackSubtitleText` | Non-null C string | Returns null | Invalid or end-of-sequence subtitle ref |
| `FastReverse_Smooth`, `FastForward_Smooth` | void | Silent absorption | |
| `FastReverse_Page`, `FastForward_Page` | void | Silent absorption | |
| `TFB_CreateSoundSample` | Non-null sample ptr | Returns null | Pre-init returns null |
| `TFB_DestroySoundSample` | void | Silent absorption | Null input is a no-op |
| `StopSource`, `CleanSource` | void | Silent absorption | |
| `SetSFXVolume`, `SetSpeechVolume` | void | Silent absorption | |
| `SetMusicStreamFade` | void | Silent absorption | |

**Subtitle iteration failure semantics:** The subtitle iteration APIs (`GetFirstTrackSubtitle`, `GetNextTrackSubtitle`, `GetTrackSubtitleText`) have the following failure behavior:

- **No active track program:** `GetFirstTrackSubtitle` returns null. `GetNextTrackSubtitle` returns null regardless of input. `GetTrackSubtitleText` returns null.
- **Invalid subtitle reference:** If a subtitle reference does not belong to the active chunk sequence (e.g., stale reference from a previous track program, arbitrary pointer), `GetNextTrackSubtitle` and `GetTrackSubtitleText` return null. Validation is performed against the active sequence before dereferencing.
- **End of iteration:** `GetNextTrackSubtitle` returns null when no further subtitle-bearing chunk follows the current reference.
- **Track stopped during iteration:** If the track program is stopped between iteration calls, subsequent calls return null because the active sequence no longer exists.

**"Silent absorption"** means the function logs diagnostic output and returns without modifying externally visible state (or returns the void/zero/false default). The caller receives no structured error indication.

---

## 20. Concurrency model

### 20.1 Threading requirements

The subsystem requires concurrent execution between the game thread and background audio processing. The required concurrency roles are:

| Role | Purpose |
|---|---|
| Game thread | Calls all public audio-heart APIs. |
| Decoder processing | Feeds decoded audio to mixer sources. Started by `init_stream_decoder`. |
| Mixer pump processing | Mixes all sources and feeds PCM to the audio backend. Started by `init_stream_decoder`. |
| Audio backend callback | Calls the mixer pump's data callback. Managed by the audio backend. |

> **Design note (non-normative):** The current implementation maps each role to a dedicated OS thread. Alternative concurrency models (e.g., thread pools, async tasks) are acceptable as long as the synchronization and deadlock-freedom properties below are satisfied.

### 20.2 Synchronization requirements

The following shared state must be synchronized:

| State | Accessed by |
|---|---|
| Individual `SoundSource` entries | Game thread, decoder processing. |
| `SoundSample` state (decoder, tags, callbacks) | Game thread, decoder processing. |
| Fade state | Game thread (set), decoder processing (read/apply). |
| Decoder processing handle | Init/uninit paths. |
| Track player state (chunk sequence, playback pointers) | Game thread, decoder processing (via callbacks). |
| Music/speech reference state | Game thread. |
| SFX volume/stereo state | Game thread. |
| Per-category volume scales | Game thread. |
| File load guard | Game thread. |
| Oscilloscope AGC state | Game thread. |

### 20.3 Deadlock prevention

When multiple synchronization boundaries must be crossed in a single operation, they must be acquired in a consistent order. The key constraint is:

- **Callbacks must not be invoked while holding source or sample synchronization.** The decoder processing must release source and sample locks before executing callbacks that may access track state or other subsystem state. This prevents circular dependencies between background processing and game-thread-visible state.

The specific lock-ordering strategy and deferred-callback mechanism are implementation details, but the deadlock-freedom property is a requirement.

### 20.4 Shutdown coordination

Shutdown must:

1. Signal background workers (decoder processing and mixer pump).
2. Wake any sleeping worker that must observe the shutdown signal.
3. Wait for worker termination before releasing shared state those workers may access.

---

## 21. Open decisions

All previously open behavioral decisions have been resolved in this revision:

- **Speech-source arbitration error behavior** → resolved in §11.2: silent no-op, with asymmetric track-priority rule fully specified for all orderings.
- **Mixer pump ownership** → resolved in §2.2 and §17.2: audio-heart owns the lifecycle.
- **Double-destroy / stale-handle semantics** → resolved in §15.2: undefined behavior, with corruption-containment constraint.

No open decisions remain that affect the ABI contract or externally visible behavior.

---

## 22. Build configuration

### 22.1 C side

- `USE_RUST_AUDIO_HEART` defined in `config_unix.h` (or equivalent per-platform config).
- When defined, the C source files `stream.c`, `trackplayer.c`, `music.c`, `sfx.c`, `sound.c`, `fileinst.c` have their high-level implementations guarded out.
- `audio_heart_rust.h` is included to declare the exported symbols.

### 22.2 Rust side

- Cargo feature `audio_heart` enables the FFI module.
- All other audio-heart modules (stream, trackplayer, music, sfx, control, fileinst, types) are always compiled regardless of feature state.

### 22.3 Remaining C code

Even with `USE_RUST_AUDIO_HEART`, some C elements currently remain compiled (volume globals, resource helpers, etc.). The end-state requirement for these is addressed in §23.

---

## 23. End-state requirements

This section defines normative end-state properties. Items listed here are **required for the subsystem to be considered complete**, not descriptions of current behavior.

### 23.1 Functional correctness requirements

The following known gaps must be resolved in the end state:

| Gap | Required end-state behavior |
|---|---|
| Internal music loader | Must load file, create decoder, return populated `MusicRef` — using the single canonical loading implementation (§14.4). |
| Internal SFX bank loader | Must parse bank file, load/decode all samples, upload to mixer — using the single canonical loading implementation (§14.4). |
| Internal file-instance layer delegates to stubs | Must route through real loaders (unified per §14.4). |
| Multi-track decoder loading | Must load real decoders for each track and advance `dec_offset` — not placeholder chunks without decoders. |
| `PLRPause` semantics | Must match C: pause only when ref matches current or is sentinel. |
| `NORMAL_VOLUME` conflict | Single canonical value (160, matching original C). No conflicting local redefinitions. |
| `init_sound` / `uninit_sound` stubs | Correct lifecycle hooks (may remain no-op if nothing additional is needed, but must be verified against the historical C behavior). |

### 23.2 Maintainability and cleanup requirements

The following are required end-state properties for maintainability. They are not functional-correctness issues but are required for the subsystem to be considered complete:

| Item | Required end-state behavior |
|---|---|
| Residual C code compiled with guard | C volume globals, resource helpers fully replaced by Rust equivalents. |
| Parity diagnostic output | `eprintln` with `[PARITY]` prefixes removed or converted to conditional trace logging. |
| Warning suppression attributes | `#![allow(dead_code, unused_imports)]` on all modules removed; all code is used or explicitly cfg-gated. |

---

## 24. Diagnostic and parity scaffolding

In the end state, all `[PARITY]`-prefixed diagnostic output and development-only debug logging must be removed or converted to conditional logging behind a debug/trace configuration flag. The current diagnostic output in stream seek, track seek, subtitle logging, mixer pump diagnostics, and splice debug output is acceptable during development but not in the final subsystem. This is a maintainability goal (see §23.2).
