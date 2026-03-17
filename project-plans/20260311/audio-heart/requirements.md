# Audio Heart Requirements

## Scope

This document defines language-agnostic requirements for the audio-heart subsystem using EARS syntax. The subsystem is the high-level sound layer responsible for stream playback orchestration, track assembly, subtitle/speech coordination, music playback, speech playback, SFX playback, lifecycle/control APIs, and resource-loading behavior above the mixer and decoder layers.

## Glossary

See the companion specification document (§3) for normative definitions of: source, channel, playback source, sample, stream, resource handle, decoder, game ticks, chunk, track program, and mixer pump.

## Contract Hierarchy

The following hierarchy governs what is required and what is flexible:

1. **Primary compatibility contract:** The exported C ABI and ABI-visible results and side effects. Function signatures, struct layouts, pointer semantics, sentinel values, and return conventions declared by the existing C integration surface.
2. **Secondary integration contract:** Interactions with mixer, decoder, UIO, resource system, comm, and timing services. These are stable integration boundaries.
3. **Non-contract implementation detail:** Internal module boundaries, helper function names, synchronization primitives, data-structure representations, and thread-internal architecture. These may be reorganized freely so long as primary and secondary contracts remain satisfied.

## Parity Policy

- **Ubiquitous:** The audio-heart subsystem shall match the ABI-visible results and side effects of the historical C implementation unless a deliberate deviation is explicitly adopted and documented.
- **Ubiquitous:** The audio-heart subsystem may differ freely in internal architecture, data structures, synchronization strategy, and module organization so long as externally observable behavior is preserved.
- **Where** the historical C implementation has a bug that callers depend on or that the ABI encodes, **the audio-heart subsystem shall** reproduce that behavior unless the bug is purely internal with no caller-visible effect.
- **Ubiquitous:** Any intentional deviations from historical behavior shall be listed explicitly in the specification rather than discovered accidentally during implementation or testing.

## Stream Playback

### Stream initialization and shutdown

- **Ubiquitous:** The audio-heart subsystem shall provide a high-level initialization operation (`init_sound`) and a stream-decoder initialization operation (`init_stream_decoder`) as separate lifecycle stages.
- **Ubiquitous:** The audio-heart subsystem shall provide corresponding shutdown operations (`uninit_stream_decoder` and `uninit_sound`) as separate lifecycle stages.
- **Ubiquitous:** The required initialization sequence shall be `init_sound` followed by `init_stream_decoder`. The required shutdown sequence shall be `uninit_stream_decoder` followed by `uninit_sound`.
- **When** stream decoding services are initialized, **the audio-heart subsystem shall** allocate and prepare the fixed set of playback sources required for SFX, music, and speech roles.
- **When** stream decoding services are initialized, **the audio-heart subsystem shall** start the background processing needed to keep streaming sources supplied with decoded audio.
- **When** stream decoding services are initialized, **the audio-heart subsystem shall** start the mixer pump that bridges mixed audio output to the platform audio backend.
- **If** stream decoding services are initialized while already initialized, **then the audio-heart subsystem shall** reject the request without duplicating worker threads, playback sources, or other singleton state.
- **If** stream decoding services are shut down without a preceding initialization, **then the audio-heart subsystem shall** treat the shutdown as a no-op.
- **Ubiquitous:** The `init_sound` and `uninit_sound` operations shall be idempotent — calling either multiple times shall have no additional effect beyond the first call.
- **When** stream decoding services are shut down, **the audio-heart subsystem shall** stop background processing, stop the mixer pump, release stream-related resources, and leave the subsystem able to initialize again cleanly.

### Stream playback behavior

- **Ubiquitous:** The audio-heart subsystem shall support starting playback of a stream sample on a specified playback source.
- **When** a stream is started on a source that is already in use, **the audio-heart subsystem shall** stop and detach the previous stream state on that source before starting the new stream.
- **When** a stream is started successfully, **the audio-heart subsystem shall** prefill and queue enough decoded audio to begin playback without requiring the caller to perform additional priming steps.
- **When** a stream is started successfully, **the audio-heart subsystem shall** record sufficient timing state to report playback position relative to the stream start and any configured offset.
- **When** stream playback is requested with rewinding enabled, **the audio-heart subsystem shall** begin playback from the start of the associated decoder timeline.
- **When** stream playback is requested with an explicit offset override, **the audio-heart subsystem shall** begin playback using that externally visible logical position.
- **Where** a stream source is configured for looping playback, **the audio-heart subsystem shall** continue playback across decoder end-of-stream boundaries by restarting or replacing decoders as required by the sample contract.
- **If** a stream cannot obtain an initial decoder or replacement decoder needed to continue playback, **then the audio-heart subsystem shall** stop advancing that stream and report end-of-stream behavior through the subsystem's normal callback and query mechanisms.
- **Ubiquitous:** The audio-heart subsystem shall support stopping, pausing, resuming, and seeking a stream independently for each playback source.
- **When** a stream is paused, **the audio-heart subsystem shall** preserve enough state to resume from the same logical playback position.
- **When** a paused stream is resumed, **the audio-heart subsystem shall** continue playback from the paused logical position rather than restarting from the beginning.
- **When** a seek is requested for an active stream, **the audio-heart subsystem shall** reposition playback to the requested logical time and resume buffering from the new decoder position.
- **If** a stream reaches its terminal end and no continuation is available, **then the audio-heart subsystem shall** mark that source as no longer playing.
- **Ubiquitous:** The audio-heart subsystem shall provide a query that reports whether a given stream source is currently playing.
- **Ubiquitous:** The audio-heart subsystem shall provide a query that reports the logical playback position of a stream source in game ticks as defined by the subsystem's time model.

### Stream callbacks and tagging

- **Ubiquitous:** The audio-heart subsystem shall support stream lifecycle callbacks for stream start, buffer queue, tagged-buffer completion, end-of-chunk, and end-of-stream events.
- **When** a buffer carrying a tag completes playback, **the audio-heart subsystem shall** invoke tagged-buffer handling before advancing subtitle-visible state that depends on later buffers.
- **When** the active decoder for a stream chunk is exhausted, **the audio-heart subsystem shall** offer the sample callback an opportunity to provide or activate the next decoder for continued playback.
- **If** no continuation decoder is made available when a chunk ends, **then the audio-heart subsystem shall** treat the stream as having no further chunk to play on that source.
- **Where** callback execution could contend with internal playback state, **the audio-heart subsystem shall** execute callbacks in a manner that avoids deadlock with internal synchronization, without requiring callers to know or reproduce internal lock ordering or synchronization strategy.

### Scope / waveform generation

- **Ubiquitous:** The audio-heart subsystem shall support optional capture of foreground stream waveform data for UI visualization.
- **When** waveform capture is enabled for a stream, **the audio-heart subsystem shall** retain enough recent decoded audio to generate waveform output aligned with currently queued playback.
- **When** waveform graph data is requested, **the audio-heart subsystem shall** select the speech stream when speech visualization is requested and speech data is available; otherwise it shall fall back to the music stream.
- **When** waveform graph data is produced, **the audio-heart subsystem shall** normalize output in a stable manner suitable for frame-to-frame UI rendering rather than exposing raw mixer amplitudes directly.
- **If** no eligible foreground stream data is available for graphing, **then the audio-heart subsystem shall** return the ABI-defined failure value (`0`) without modifying caller-owned graph output beyond that contract.

### Fade behavior

- **Ubiquitous:** The audio-heart subsystem shall support time-based fading of music volume toward a target volume.
- **When** a fade is requested with zero duration, **the audio-heart subsystem shall** apply the target music volume immediately.
- **When** a fade is requested with non-zero duration, **the audio-heart subsystem shall** interpolate music volume from the current volume to the requested target across the requested interval.
- **When** a new fade request supersedes an active fade, **the audio-heart subsystem shall** treat the current effective music volume as the new fade starting point.

## Track Assembly and Playback

### Track assembly

- **Ubiquitous:** The audio-heart subsystem shall provide a track-assembly facility that represents a playback program as an ordered sequence of chunks with timing, decoder, subtitle, and callback metadata.
- **When** a first track is spliced into an empty track program, **the audio-heart subsystem shall** create the shared playback sample and callback plumbing needed to play the assembled program on the speech source.
- **When** a track splice request includes subtitle text, **the audio-heart subsystem shall** divide that text into display pages according to the subtitle paging rules defined in the specification (§8.6): splitting on CR/LF boundaries, adding `..` continuation prefixes, adding `...` mid-word suffixes, and computing per-page timing as `max(character_count * TEXT_SPEED, 1000)` milliseconds.
- **When** a track splice request includes explicit timestamps, **the audio-heart subsystem shall** use those timestamps (parsed as comma-separated integer millisecond values) to override computed per-page timing for pages where timestamps are supplied.
- **When** a track splice request includes decoder-backed audio pages, **the audio-heart subsystem shall** append chunks in playback order and accumulate logical program time based on decoder durations or explicit timing rules.
- **When** a subtitle-only continuation is spliced without a new audio track, **the audio-heart subsystem shall** append the first continuation text to the preceding subtitle page when the caller contract requires no page break.
- **When** a no-page-break continuation is active, **the audio-heart subsystem shall** preserve subtitle continuity across the splice while still maintaining correct audio chunk sequencing.
- **If** a splice request contains no usable subtitle text and no usable audio content, **then the audio-heart subsystem shall** leave the assembled track program unchanged.

### Multi-track assembly

- **When** a multi-track splice is requested after a base track program exists, **the audio-heart subsystem shall** append the requested additional tracks as part of the same logical program.
- **When** a multi-track splice is accepted, **the audio-heart subsystem shall** load or associate a real decoder for each appended audio track rather than creating placeholder chunks with no playable audio.
- **When** a multi-track splice contributes playable audio, **the audio-heart subsystem shall** advance the logical program timeline to include the duration of each appended track.
- **If** a multi-track splice is requested without an existing base track context, **then the audio-heart subsystem shall** silently ignore the request without constructing an invalid partial program.

### Track assembly and canonical loading

- **Ubiquitous:** Track assembly operations (`splice_track`, `splice_multi_track`) are not themselves canonical file-loading entry points as defined by the loader consolidation requirement. When these operations acquire decoders from resource names, they use the loading infrastructure but constitute a separate decoder-acquisition path.
- **When** decoder acquisition during track assembly fails for a given resource, **the audio-heart subsystem shall** produce a chunk with no decoder, which the stream engine handles as end-of-chunk. The resulting ownership and error semantics shall be equivalent to those of the canonical loading path.

### Track playback and control

- **Ubiquitous:** The audio-heart subsystem shall support starting playback of the assembled track program on the speech playback source.
- **When** track playback starts, **the audio-heart subsystem shall** compute and store the total logical track length for later position queries and seeking.
- **When** track playback starts, **the audio-heart subsystem shall** begin from the first chunk of the assembled track program unless the caller has already positioned playback elsewhere through a defined seek operation.
- **When** track playback is stopped, **the audio-heart subsystem shall** stop speech-source playback, clear current-track pointers, and release resources owned exclusively by the current assembled program.
- **When** track playback is jumped to the terminal state, **the audio-heart subsystem shall** stop further playback progression and clear current chunk and current subtitle state.
- **When** track playback is paused, **the audio-heart subsystem shall** pause the underlying speech stream without discarding the assembled track program.
- **When** track playback is resumed, **the audio-heart subsystem shall** continue from the current logical track position.
- **Ubiquitous:** The audio-heart subsystem shall provide queries for whether a track program is playing, which logical track number is active, and the current track position.

### Track progression and seeking

- **When** the active track chunk ends and a following chunk exists, **the audio-heart subsystem shall** advance to the next chunk, associate its decoder with the speech stream, and continue playback without requiring caller intervention.
- **When** the active track chunk ends and no following chunk exists, **the audio-heart subsystem shall** transition the track program to the ended state.
- **When** smooth reverse seeking is requested, **the audio-heart subsystem shall** move playback backward by the defined smooth-seek increment and update subtitle-visible state to match the resulting position.
- **When** smooth forward seeking is requested, **the audio-heart subsystem shall** move playback forward by the defined smooth-seek increment and update subtitle-visible state to match the resulting position.
- **When** page-back seeking is requested, **the audio-heart subsystem shall** move playback to the previous subtitle-tagged chunk boundary if one exists.
- **When** page-forward seeking is requested, **the audio-heart subsystem shall** move playback to the next subtitle-tagged chunk boundary if one exists.
- **If** page-forward seeking is requested from the final subtitle page, **then the audio-heart subsystem shall** move playback to the terminal logical position defined by the caller contract.
- **When** a seek positions playback within a chunk, **the audio-heart subsystem shall** seek that chunk's decoder to the corresponding intra-chunk offset and preserve coherent program timing for subsequent queries.
- **When** a seek changes the active subtitle page, **the audio-heart subsystem shall** update current subtitle state as if playback had naturally advanced to that page.

## Subtitle and Speech Coordination

### Subtitle synchronization

- **Ubiquitous:** The audio-heart subsystem shall synchronize subtitle changes to actual audio playback using tagged playback-buffer completion rather than subtitle timing alone.
- **When** a chunk is designated as subtitle-bearing, **the audio-heart subsystem shall** tag playback in a way that allows the correct subtitle state transition to occur when that chunk's audible data reaches completion.
- **When** a tagged playback event is reported, **the audio-heart subsystem shall** validate that the referenced subtitle-bearing chunk still belongs to the active track program before using it to update visible subtitle state.
- **If** a tagged playback event refers to a chunk that is no longer part of the active track program, **then the audio-heart subsystem shall** ignore that event for subtitle advancement.

### Subtitle access

- **Ubiquitous:** The audio-heart subsystem shall provide access to the current subtitle text for the active track program.
- **Ubiquitous:** The audio-heart subsystem shall provide iteration access to subtitle-bearing chunks for UI flows that review or page through subtitles independently of live playback.
- **When** an external caller requests C-ABI subtitle text for the current subtitle, **the audio-heart subsystem shall** return a pointer that remains stable for as long as the current subtitle does not change and the track program remains active.
- **When** the current subtitle changes, **the audio-heart subsystem shall** expose that change through pointer identity and content in a manner compatible with existing caller polling behavior.
- **When** track playback ends or is stopped, **the audio-heart subsystem shall** cause subsequent current-subtitle pointer queries to return null.
- **Where** raw subtitle-chunk references are exposed across the ABI, **the audio-heart subsystem shall** validate those references against the active assembled program before dereferencing or advancing them.
- **When** subtitle iteration is requested with no active track program, **the audio-heart subsystem shall** return the ABI-defined null outcome for the specific iteration API (specification §19.3).
- **When** subtitle iteration or text retrieval is requested with a reference that does not belong to the active chunk sequence, **the audio-heart subsystem shall** return null without dereferencing the invalid reference.
- **When** subtitle iteration reaches the end of the subtitle-bearing chunk sequence, **the audio-heart subsystem shall** return null to indicate end of iteration.

### Speech-source arbitration

- **Ubiquitous:** The audio-heart subsystem shall use the dedicated speech playback source for both assembled track playback and standalone speech playback.
- **Ubiquitous:** Track playback has unconditional priority over standalone speech on the shared speech source. This is an asymmetric rule: track playback always wins regardless of which was established first.
- **When** a track program is active on the speech source, **the audio-heart subsystem shall** treat the track player as the owner of the speech source for the duration of that track program.
- **If** standalone speech playback is requested while a track program owns the speech source, **then the audio-heart subsystem shall** silently reject the request without starting playback and without modifying standalone speech reference state.
- **When** standalone speech is active on the speech source and a track playback request arrives, **the audio-heart subsystem shall** allow track playback to proceed. The `play_stream` call for track playback stops any existing stream on the source before starting the new one, so the standalone speech stream is stopped as a side effect. `cur_speech_ref` is not automatically cleared by this sequence.
- **When** a track program is stopped, **the audio-heart subsystem shall** release speech-source ownership so that standalone speech playback requests become valid again.
- **If** `snd_stop_speech` is called while a track program owns the speech source, **then the audio-heart subsystem shall** clear standalone speech reference state without affecting track playback or track state.

## Music and SFX Behavior

### Music behavior

- **Ubiquitous:** The audio-heart subsystem shall provide APIs to play, stop, pause, resume, seek, query, and fade music on the dedicated music source.
- **When** a music resource is played, **the audio-heart subsystem shall** record it as the current music reference for subsequent control and query operations.
- **When** a music stop request names the currently active music resource (matched by raw-handle identity per the specification §15.3), **the audio-heart subsystem shall** stop the music source and clear the current music reference.
- **When** a music-playing query names the currently active music resource, **the audio-heart subsystem shall** report whether that music source is actively playing.
- **When** a music seek request names the currently active music resource, **the audio-heart subsystem shall** seek the music source to the requested logical position.
- **When** a wildcard or current-music control operation is used where the ABI defines one, **the audio-heart subsystem shall** apply the operation to whichever music resource is currently active.
- **When** a music pause request is issued through an ABI that supplies a music reference argument, **the audio-heart subsystem shall** pause music only if the supplied reference matches the current music resource (by raw-handle identity) or represents the externally defined wildcard sentinel (`~0`).
- **If** a music pause request supplies a non-matching non-wildcard reference, **then the audio-heart subsystem shall** leave current music playback unchanged.

### Speech behavior

- **Ubiquitous:** The audio-heart subsystem shall provide APIs to play and stop standalone speech on the dedicated speech source.
- **When** standalone speech playback begins, **the audio-heart subsystem shall** record the active speech reference for later control and cleanup.
- **When** standalone speech playback is stopped, **the audio-heart subsystem shall** stop the speech source and clear the recorded active speech reference.

### SFX behavior

- **Ubiquitous:** The audio-heart subsystem shall provide playback of sound effects on the fixed set of numbered SFX channels.
- **When** an SFX channel playback request is accepted, **the audio-heart subsystem shall** stop any existing playback on that channel before binding and starting the new sample.
- **When** an SFX channel playback request names a sound-bank entry, **the audio-heart subsystem shall** locate and play the specified preloaded sample from that bank.
- **When** an SFX playback request names a direct sample handle rather than a bank entry, **the audio-heart subsystem shall** play that sample using the same externally visible channel semantics.
- **If** an SFX channel index is outside the valid channel range, **then the audio-heart subsystem shall** silently reject the request without modifying valid channels.
- **Ubiquitous:** The audio-heart subsystem shall provide per-channel stop, playing-state query, and volume control operations.
- **When** an SFX channel has finished playback, **the audio-heart subsystem shall** clean up channel state so that subsequent playback on that channel starts from a clean source state.

### Priority parameters

- **Where** music, SFX, or channel-control APIs accept a `priority` parameter in the ABI, **the audio-heart subsystem shall** accept that parameter for ABI compatibility.
- **Ubiquitous:** The audio-heart subsystem shall not use the `priority` parameter to influence channel-stealing, interruption, or playback ordering. The historical C implementation accepted priority but did not use it to affect behavior.

### Positional audio

- **Ubiquitous:** The audio-heart subsystem shall support positional metadata for SFX playback channels.
- **When** positional playback is requested for an SFX channel, **the audio-heart subsystem shall** convert caller-supplied positions into mixer-facing spatial parameters using the subsystem's defined attenuation factor (`ATTENUATION = 160.0`) and minimum distance (`MIN_DISTANCE = 0.5`).
- **When** non-positional playback is requested for an SFX channel, **the audio-heart subsystem shall** place the channel at the subsystem's defined centered non-positional location (0, 0, -1).
- **Ubiquitous:** The audio-heart subsystem shall support storing and retrieving an opaque positional-object association for each playback source where the ABI exposes that capability.

### Volume behavior

- **Ubiquitous:** The audio-heart subsystem shall provide independent externally visible volume control for music, speech, and SFX categories.
- **When** a category volume is changed, **the audio-heart subsystem shall** apply the resulting effective gain to all currently relevant sources for that category.
- **When** a volume API accepts values outside the valid range (0 to `MAX_VOLUME`, inclusive), **the audio-heart subsystem shall** clamp the value to the valid range and shall not produce undefined gain values.
- **Ubiquitous:** The audio-heart subsystem shall use a single canonical definition of normal/default volume (`NORMAL_VOLUME = 160`) wherever that concept is exposed.

## Lifecycle and Control APIs

### Pre-initialization behavior

- **When** any FFI-exposed playback, query, loading, or control API is called before `init_stream_decoder` has completed, **the audio-heart subsystem shall** produce the ABI-defined failure outcome for that specific function as mapped in the ABI failure mode map (specification §19.3), without modifying externally visible playback state.
- **When** the wait-for-end operation is called before `init_stream_decoder` has completed, **the audio-heart subsystem shall** return immediately without blocking.

### Global control and queries

- **Ubiquitous:** The audio-heart subsystem shall provide a top-level operation to stop currently playing sound effects globally.
- **Ubiquitous:** The audio-heart subsystem shall provide a top-level query that reports whether any managed sound source is currently playing.
- **Ubiquitous:** The audio-heart subsystem shall provide a wait operation that blocks until a specified source or the full managed sound set has stopped.
- **When** the wait-for-end operation receives a valid source index (0 through `NUM_SOUNDSOURCES - 1`), **the audio-heart subsystem shall** wait only for that source to stop.
- **When** the wait-for-end operation receives the defined all-sources sentinel value, **the audio-heart subsystem shall** wait for all managed sources (SFX, music, and speech) to stop.
- **When** the wait-for-end operation receives a value that is neither a valid source index nor the defined all-sources sentinel, **the audio-heart subsystem shall** treat it as equivalent to the all-sources sentinel, matching the historical C default-branch behavior.
- **When** a source is paused during a wait-for-end operation, **the audio-heart subsystem shall** treat that source as still active rather than treating pause as equivalent to stop.
- **Ubiquitous:** A source with no attached sample, or with no allocated mixer handle, shall be considered inactive for wait-for-end purposes. The paused-stream exception takes priority: a paused source with an attached sample and allocated mixer handle is still active.
- **When** the wait-for-end operation is active, **the audio-heart subsystem shall** poll or wait without requiring external progress calls from the caller. The specific polling mechanism and interval are not normative; any approach that terminates promptly after all selected sources stop is acceptable.
- **If** the process or game shutdown flag required by the integration contract becomes active during a wait-for-end operation (or is already active when the wait begins), **then the audio-heart subsystem shall** terminate the wait promptly.
- **If** source state is being concurrently torn down during a wait-for-end operation, **then the audio-heart subsystem shall** observe shutdown signaling and exit without accessing released resources.
- **When** a source is stopped through a control API, **the audio-heart subsystem shall** unqueue or discard playback state needed to ensure the source can be reused cleanly.

## Error Handling

- **Ubiquitous:** The audio-heart subsystem shall detect invalid source indices, invalid channel indices, null or missing handles, missing decoders, initialization-state violations, and resource-loading failures and translate each into the ABI-defined failure outcome for the specific API as mapped in the ABI failure mode map (specification §19.3).
- **When** an internal audio-heart operation fails, **the audio-heart subsystem shall** propagate structured failure information within the internal API boundary rather than collapsing all failures to an indistinguishable generic condition.
- **When** an FFI-exposed operation fails, **the audio-heart subsystem shall** translate the failure to the ABI-defined error indication (null, `-1`, `0`/false, or void with silent absorption) as specified per-function in the ABI failure mode map, without unwinding across the language boundary.
- **If** background playback processing encounters an unrecoverable per-source failure, **then the audio-heart subsystem shall** stop progressing that source and leave other sources able to continue operating.
- **If** the mixer-pump or background audio callback encounters an internal fault, **then the audio-heart subsystem shall** fail safely in a way that preserves process integrity and does not propagate language-runtime unwinding through the audio backend callback boundary.
- **When** a resource load cannot find or open a requested file, **the audio-heart subsystem shall** report that condition as a null return from the loading function.
- **When** concurrent file-load requests violate the subsystem's single-active-load guard, **the audio-heart subsystem shall** reject the later request by returning null.

## Ownership and Lifecycle Obligations

### Ownership of runtime objects

- **Ubiquitous:** The audio-heart subsystem shall define opaque ownership boundaries for stream samples, music references, sound banks, subtitle references, and playback sources at every externally visible API boundary.
- **When** a sample or resource handle is created, **the audio-heart subsystem shall** retain or transfer ownership exactly as defined by the associated API contract.
- **When** a caller destroys a music or sound resource handle through the subsystem API, **the audio-heart subsystem shall** release all mixer buffers, decoder objects, and auxiliary allocations owned exclusively by that handle.
- **If** a destroy operation is applied to a resource that is currently active on a playback source, **then the audio-heart subsystem shall** stop or detach active playback as required to avoid use-after-free behavior.
- **When** a track program is stopped or discarded, **the audio-heart subsystem shall** invalidate raw subtitle or chunk references that belong to that program and prevent subsequent dereference through validation checks.

### ABI handle semantics

- **Ubiquitous:** Resource handles returned to C callers shall have single-owner semantics at the ABI boundary. Each load operation returns a unique handle owned by the caller.
- **Ubiquitous:** Each separate load operation shall return a distinct handle. Two loads of the same file path shall produce non-equal handles.
- **When** a play operation borrows a resource handle, **the audio-heart subsystem shall** retain its own internal reference so that the handle remains valid for the caller to destroy independently. The internal reference shall preserve raw-handle identity with the caller's handle for comparison purposes.
- **When** a null pointer is passed to a destroy operation, **the audio-heart subsystem shall** treat it as a no-op.
- **When** a resource handle is destroyed, **the audio-heart subsystem shall** treat that handle as immediately invalid for all subsequent operations.
- **If** a previously destroyed handle is passed to a destroy operation a second time, **then the behavior is undefined**, but **the audio-heart subsystem shall** constrain that undefined behavior so that it does not corrupt unrelated subsystem state, other live handles, or the source table.

### Handle identity

- **Ubiquitous:** Handle equality for control and query APIs shall be determined by raw-handle identity (same pointer value). There shall be no deep comparison of underlying content.
- **Ubiquitous:** The null value (`0`) shall never be equal to any valid handle. The wildcard sentinel (`~0`) shall bypass handle comparison and apply the operation to the currently active resource.

### Sample and source obligations

- **Ubiquitous:** The audio-heart subsystem shall not destroy a sample's underlying playback resources while that sample remains attached to an active source.
- **When** a source changes ownership from one sample to another, **the audio-heart subsystem shall** detach the previous sample before exposing the source as attached to the new sample.
- **When** a sample includes callback and user-data attachments, **the audio-heart subsystem shall** preserve those attachments for the sample's lifetime unless the caller explicitly replaces or clears them through a defined API.

### Resource loading obligations

- **Ubiquitous:** The audio-heart subsystem shall provide explicit destroy operations for resources created by music and sound-file loading APIs.
- **When** a file-loading operation is active, **the audio-heart subsystem shall** maintain any externally required current-resource-name guard for the full duration of the load and clear it on all exit paths, including failure paths.
- **Ubiquitous:** The audio-heart subsystem shall use a single canonical implementation of loading behavior per resource type, ensuring that all entry points (internal and FFI) share the same validation, decode, error handling, and ownership semantics.
- **Ubiquitous:** Resource-name validation at the loading boundary shall require a non-empty filename string. Path resolution and existence checking shall be delegated to UIO. A null or empty filename shall cause the load to fail.
- **When** a file-backed loading API is called before stream decoding services have been initialized, **the audio-heart subsystem shall** return null, because loading creates mixer-ready objects that depend on initialized mixer state.

## Concurrency Expectations

- **Ubiquitous:** The audio-heart subsystem shall support concurrent activity between a caller thread and one or more internal background audio-processing contexts.
- **When** shared stream, source, track, fade, or resource-loading state is accessed from multiple concurrent contexts, **the audio-heart subsystem shall** synchronize that access so that externally visible behavior remains coherent and memory-safe.
- **Where** the subsystem exposes callback-driven interactions between background playback processing and caller-visible track state, **the audio-heart subsystem shall** avoid deadlock by ensuring callbacks are not invoked while internal source or sample synchronization is held.
- **When** background processing needs to invoke caller-visible callbacks or track-state transitions, **the audio-heart subsystem shall** do so in a way that does not require internal source or sample synchronization to remain held across the callback execution.
- **If** initialization and shutdown race with background processing, **then the audio-heart subsystem shall** coordinate those operations so that worker contexts do not access released resources.
- **When** shutdown is requested, **the audio-heart subsystem shall** signal background workers, wake any sleeping worker that must observe shutdown, and wait for worker termination before releasing shared state those workers may access.

## Integration Obligations

### Mixer integration

- **Ubiquitous:** The audio-heart subsystem shall perform low-level audio playback exclusively through the mixer integration contract for source allocation, source control, buffer allocation, buffer upload, queueing, unqueueing, property updates, and mixed-output generation.
- **When** audio-heart needs playback-source handles or playback-buffer handles, **the audio-heart subsystem shall** obtain and release them through the mixer API rather than creating private unmanaged equivalents.
- **When** audio-heart submits decoded audio for playback, **the audio-heart subsystem shall** upload that audio to mixer-managed buffers using the audio format and frequency contract required by the mixer.
- **When** audio-heart needs to query whether non-streaming playback has finished, **the audio-heart subsystem shall** use the mixer source-state query contract.

### Mixer pump integration

- **Ubiquitous:** The audio-heart subsystem shall own the lifecycle of the mixer pump component that bridges mixer output to the platform audio backend.
- **When** stream decoding services are initialized, **the audio-heart subsystem shall** start the mixer pump.
- **When** stream decoding services are shut down, **the audio-heart subsystem shall** stop the mixer pump before releasing related resources.

### Decoder integration

- **Ubiquitous:** The audio-heart subsystem shall treat decoders as the authoritative providers of audio format, frequency, position, seek capability, and decoded frame data.
- **When** audio-heart needs more audio for a streaming source, **the audio-heart subsystem shall** request decoded data through the decoder integration contract rather than bypassing it.
- **When** audio-heart seeks within a stream or track chunk, **the audio-heart subsystem shall** use the decoder's seek capability and synchronize logical time reporting to the resulting decoder position.
- **When** audio-heart assembles a track program or resource handle from file-backed audio, **the audio-heart subsystem shall** associate real decoders with the resulting playable content where externally visible playback requires them.

### Resource and UIO integration

- **Ubiquitous:** The audio-heart subsystem shall load music and sound resources through the project's resource and UIO integration contracts rather than assuming direct host-filesystem access patterns.
- **When** a music file is loaded, **the audio-heart subsystem shall** open the requested content resource through UIO, create an appropriate decoder, and return an opaque music handle representing playable content.
- **When** a sound-bank file is loaded, **the audio-heart subsystem shall** parse the bank resource, load each referenced sound entry through UIO, decode the referenced audio into mixer-ready buffers, and return an opaque handle representing the bank.
- **Where** ABI compatibility requires a specific externally visible handle representation for loaded sound resources, **the audio-heart subsystem shall** construct and destroy that representation exactly according to the established ABI layout and ownership rules.
- **If** a requested resource path is null or empty, **then the audio-heart subsystem shall** fail the load. Beyond non-emptiness, path resolution and existence validation shall be delegated to UIO.

### Comm integration

- **Ubiquitous:** The audio-heart subsystem shall preserve the externally visible behavior required by the comm subsystem for track assembly, track playback, subtitle polling, subtitle iteration, track seeking, and oscilloscope rendering.
- **When** the comm subsystem polls for subtitle changes using stable pointer identity, **the audio-heart subsystem shall** preserve that pointer-identity behavior for the active subtitle.
- **When** the comm subsystem requests current track position in its expected units, **the audio-heart subsystem shall** return values coherent with actual speech playback progress and page navigation.
- **When** the comm subsystem assembles multiple phrases into a single conversation program, **the audio-heart subsystem shall** preserve ordering, paging, callback, and subtitle continuity according to the established caller contract.

### ABI and C integration

- **Ubiquitous:** The audio-heart subsystem shall preserve the externally visible ABI for the functions, structures, pointer semantics, sentinel values, and return conventions declared by the existing C integration surface.
- **When** the build enables the replacement path, **the audio-heart subsystem shall** also provide the matching exported ABI symbols required by that C integration surface.
- **If** the build enables the replacement declarations without enabling the corresponding exported implementation symbols, **then the build or integration configuration shall** fail rather than producing a silently inconsistent binary.
- **Where** the ABI exposes C-struct layouts for sound positions, string tables, or related handle representations, **the audio-heart subsystem shall** preserve exact layout compatibility.
- **When** C callers pass null pointers, wildcard sentinels, or opaque raw references permitted by the ABI, **the audio-heart subsystem shall** handle those values according to the established C contract without causing undefined behavior.

## Consolidation and Final-State Requirements

### Functional correctness

- **Ubiquitous:** The final audio-heart subsystem shall not rely on placeholder internal loaders, placeholder multi-track decoders, or other non-playable stub behavior for externally visible functionality.
- **Ubiquitous:** The final audio-heart subsystem shall remove externally visible behavioral deviations from the established high-level C contract except where a deliberate ABI change is explicitly adopted.
- **Ubiquitous:** The final audio-heart subsystem shall absorb any remaining required high-level sound behavior so that the replacement path is functionally complete at the integration boundary.

### Loader consolidation

- **Ubiquitous:** The final audio-heart subsystem shall consolidate resource-loading logic so that there is a single canonical loading implementation per resource type, with all entry points routing through it.
- **Ubiquitous:** Canonical loading shall apply to file-backed resource construction. Decoder adoption from externally supplied inputs (pre-created decoders passed via FFI, samples constructed by `TFB_CreateSoundSample`) shall not re-run file-loading validation; those inputs share the `SoundSample` interface and its ownership/lifecycle rules as the common contract.

### Maintainability and cleanup

- **Ubiquitous:** The final audio-heart subsystem shall eliminate residual C implementations whose functionality has been fully absorbed, so that the replacement path is complete rather than partial.
- **Ubiquitous:** The final audio-heart subsystem shall remove development-only diagnostic scaffolding (parity-prefixed logging, warning-suppression attributes) or convert it to conditional trace logging.
