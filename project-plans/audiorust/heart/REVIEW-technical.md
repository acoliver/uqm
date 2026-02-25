# Technical Review -- Audio Heart Plan

**Reviewer**: Claude (pedantic technical review, round 4)
**Date**: 2026-02-24
**Scope**: All 7 pseudocode files, specification, domain model, 10+ plan phase docs, existing Rust source verification
**Focus**: Algorithm correctness, concurrency bugs, FFI safety, requirement coverage, integration gaps

---

## Severity Key

- **[CRITICAL]** -- Will cause crashes, data corruption, UB, or deadlocks at runtime
- **[HIGH]** -- Incorrect behavior, silent data loss, or spec violation
- **[MEDIUM]** -- Correctness risk in edge cases, maintainability problem, or missing validation
- **[LOW]** -- Style, documentation, or minor inconsistency

---

## 1. Concurrency & Locking

### [CRITICAL] CRIT-CONC-01: `stop_stream` inside `stop_track` deadlocks on Source mutex

**File**: `trackplayer.md` S6 (stop_track), lines 220-251

`stop_track` acquires `TRACK_STATE.lock()` at line 221, then calls `stop_stream(SPEECH_SOURCE)` at line 225. `stop_stream` (stream.md S6, line 162) acquires `SOURCES.sources[source_index].lock()` internally. This is fine -- lock ordering is TRACK_STATE -> Source.

**However**, `stop_stream` calls `stop_source(source_index)` at line 163 (stream.md). `stop_source` (control.md S4, lines 50-61) **also** acquires `SOURCES.sources[source_index].lock()` at line 56. Since `stop_stream` already holds that same lock (line 162), and `parking_lot::Mutex` is **not reentrant**, this is a **self-deadlock**.

The fix is one of:
- `stop_stream` must drop its Source lock before calling `stop_source`, or
- `stop_stream` must inline the `stop_source` logic using the guard it already holds, or
- `stop_source` must accept an already-locked guard.

**Evidence**: stream.md line 162 locks source, line 163 calls `stop_source`, control.md line 56 locks same source.

### [CRITICAL] CRIT-CONC-02: `play_stream` callback invocation under both locks

**File**: `stream.md` S5 (play_stream), lines 78-81 and 123-124

`play_stream` acquires the Source lock (line 74) and then the Sample lock (line 75). At lines 78-81 it calls `callbacks.on_start_stream(&mut sample)` while **both locks are held**. For the `TrackCallbacks` implementation, `on_start_stream` acquires `TRACK_STATE` (trackplayer.md S13, line 413).

This violates the stated lock ordering: `TRACK_STATE -> Source -> Sample`. Acquiring TRACK_STATE while holding Source and Sample is a deadlock if any other thread holds TRACK_STATE and tries to acquire Source (e.g., `stop_track` holds TRACK_STATE and calls `stop_stream` which acquires Source).

Similarly, `on_queue_buffer` is called at line 123-124 under both locks. If `TrackCallbacks` ever implements `on_queue_buffer` with state access, this would deadlock.

The deferred callback pattern was correctly applied to `process_source_stream` (S11) but was **not** applied to `play_stream` (S5). The same pattern is needed here: collect callback actions, drop locks, then execute.

**Deadlock scenario**:
- Thread A (main, `play_stream`): holds Source+Sample, wants TRACK_STATE
- Thread B (main, `stop_track`): holds TRACK_STATE, wants Source (via stop_stream)

`play_track` drops TRACK_STATE before calling `play_stream` (trackplayer.md S5, lines 192-207, explicit fix for ISSUE-CONC-01). So `play_stream` enters with NO TRACK_STATE held, acquires Source+Sample, calls `on_start_stream` which acquires TRACK_STATE. This is Source -> Sample -> TRACK_STATE ordering, which **inverts** the defined hierarchy (TRACK_STATE -> Source -> Sample).

Since `play_stream` and `stop_track` are both main-thread-only, this may not be a practical issue in the current codebase. But the lock ordering violation is real, and if any future caller invokes play_stream from a non-main thread, it deadlocks.

**Recommendation**: Apply the deferred callback pattern to `play_stream`'s `on_start_stream` call -- extract needed state, drop locks, call callback.

### [HIGH] HIGH-CONC-03: `fast_reverse_smooth` holds Source lock then calls `play_stream`

**File**: `trackplayer.md` S11, lines 340-349

`fast_reverse_smooth` acquires `SOURCES.sources[SPEECH_SOURCE].lock()` at line 342, calls `seek_track` at line 345, and then calls `play_stream(...)` at line 347. But `play_stream` (stream.md S5) acquires the Source lock internally (line 74). Since the Source lock is already held from line 342, this **self-deadlocks**.

This is the same class of bug that ISSUE-ALG-02 fixed in `seek_stream` -- but the fix was not applied to `fast_reverse_smooth`.

`fast_forward_smooth` (line 351-355) also holds the Source lock during `seek_track`. `seek_track` (line 330) calls `stop_stream(SPEECH_SOURCE)` which acquires the Source lock -- same deadlock.

**Recommendation**: `fast_reverse_smooth`, `fast_forward_smooth`, `fast_reverse_page`, and `fast_forward_page` all need the same lock-drop-then-call pattern.

### [HIGH] HIGH-CONC-04: `jump_track` holds Source lock then calls `seek_track` which calls `stop_stream`

**File**: `trackplayer.md` S7, lines 250-258

`jump_track` acquires `SOURCES.sources[SPEECH_SOURCE].lock()` at line 255, then calls `seek_track` at line 257. `seek_track` (line 330) calls `stop_stream(SPEECH_SOURCE)` when offset exceeds all chunks. `stop_stream` acquires the Source lock -- deadlock.

**Recommendation**: Same pattern -- extract state, drop lock, call seek_track.

### [HIGH] HIGH-CONC-05: `playing_track` holds TRACK_STATE + Source -- unnecessary lock

**File**: `trackplayer.md` S9, lines 290-296

`playing_track` acquires `TRACK_STATE.lock()` (line 291), then `SOURCES.sources[SPEECH_SOURCE].lock()` (line 293). This is TRACK_STATE -> Source ordering, which is correct per the hierarchy. However, the Source lock serves no purpose here -- `cur_chunk` is guarded by TRACK_STATE, and `track_num` is read from the chunk under TRACK_STATE.

The C code locks the Source because `cur_chunk` is accessed from the decoder thread. In the Rust version, `cur_chunk` is inside `TRACK_STATE`, not inside the Source. The Source lock is unnecessary and adds contention.

**Recommendation**: Remove the Source lock acquisition from `playing_track` -- TRACK_STATE alone protects `cur_chunk`.

### [MEDIUM] MED-CONC-06: `process_music_fade` holds FadeState lock while calling `set_music_volume`

**File**: `stream.md` S12, lines 360-370

`process_music_fade` acquires `ENGINE.fade.lock()` at line 361, then calls `set_music_volume(volume)` at line 367. `set_music_volume` (music.md S9, lines 190-195) acquires `MUSIC_STATE.lock()` then `SOURCES.sources[MUSIC_SOURCE].lock()`.

Lock ordering: FadeState -> MUSIC_STATE -> Source. The defined hierarchy is TRACK_STATE -> MUSIC_STATE -> Source -> Sample -> FadeState. This puts FadeState as the **innermost** lock, but `process_music_fade` acquires it **before** MUSIC_STATE -- violating the ordering.

This is safe in practice because no other path acquires MUSIC_STATE then FadeState. But it is a latent ordering violation.

**Recommendation**: Either (a) read the computed volume under the FadeState lock, drop it, then call `set_music_volume`, or (b) document FadeState as a peer to MUSIC_STATE (not a subordinate).

---

## 2. Algorithm Correctness

### [HIGH] HIGH-ALG-01: `SoundSample` missing `looping` field in spec S3.1.1

**File**: `rust-heart.md` S3.1.1 (SoundSample struct), lines 148-157

The spec's `SoundSample` struct definition (lines 148-157) lists fields: `decoder`, `length`, `buffers`, `num_buffers`, `buffer_tags`, `offset`, `data`, `callbacks`. The `looping` field is **not listed**.

The plan document (03-types-stub.md) correctly notes "Add a `looping: bool` field to SoundSample" and the specification's S3.3 mentions "`looping` (bool, stored on sample not decoder)". But the actual struct definition in the spec's code block omits it.

The pseudocode at stream.md line 100 writes `sample.looping = looping`, and at stream.md line 215 reads `sample.looping`. If the implementer follows the spec's struct definition literally, this field won't exist.

**Recommendation**: Add `looping: bool` to the SoundSample struct definition in the spec's code block.

### [HIGH] HIGH-ALG-02: `read_sound_sample` 16-bit sample reading is little-endian-specific

**File**: `stream.md` S15, lines 470-478

```
LET lo = buffer[pos as usize] as i16
LET hi = buffer[((pos + 1) % size) as usize] as i16
RETURN lo | (hi << 8)
```

This assumes little-endian byte order (lo byte first). PCM audio data from the decoders is typically in the native byte order or in the format specified by the decoder. The `SoundDecoder` trait has a `needs_swap()` method that returns true if byte swapping is needed.

The pseudocode ignores `needs_swap()` entirely. If the decoder produces big-endian samples, the oscilloscope display will show garbage.

**Recommendation**: Check `decoder.needs_swap()` and swap bytes if needed, or document that all decoders produce little-endian PCM (if that's the convention).

### [MEDIUM] MED-ALG-03: `splice_track` NullDecoder for continuation pages -- O(N) decoder thread iterations

**File**: `trackplayer.md` S1, line 70

Multi-page subtitles create multiple chunks, but only the first page gets the actual decoder. Subsequent pages get `NullDecoder`. Each NullDecoder chunk immediately returns EOF on decode, triggering `on_end_chunk` which advances `cur_chunk`. This creates a chain of N immediate end-of-chunk callbacks for N non-audio chunks between two audio chunks.

With the deferred callback pattern, the first EOF sets `end_chunk_failed = true` (stream.md line 295, 333), preventing further decode attempts in that iteration. The deferred `EndChunk` callback advances `cur_chunk` once. But only one `EndChunk` fires per decoder-thread iteration, so traversing N NullDecoder chunks requires N iterations of the decoder thread main loop (each with a `yield_now()` call).

For typical usage (2-5 continuation pages per audio chunk), this is fine. For pathological cases with many pages, there could be a noticeable delay between chunks.

**Recommendation**: Add a comment in the implementation noting this O(N) behavior. Consider handling multiple consecutive NullDecoder chunks in a single iteration as an optimization.

### [MEDIUM] MED-ALG-04: `get_track_position` integer overflow for large `in_units`

**File**: `trackplayer.md` S16, line 487

```
RETURN in_units * offset / len
```

If `in_units` is large (e.g., 65535) and `offset` is large (e.g., 120,000 game ticks for a 2-minute track), the product `in_units * offset` overflows u32. In debug Rust this panics; in release it wraps silently.

The C code uses the same formula with `uint32` and has the same overflow risk, so this is behavior-compatible. But Rust debug panics are worse than C wrapping.

**Recommendation**: Use `(in_units as u64 * offset as u64 / len as u64) as u32` to avoid overflow.

### [MEDIUM] MED-ALG-05: `SoundChunk.start_time` type inconsistency: `f32` in spec vs `f64` in pseudocode

**File**: `rust-heart.md` S3.2.1 (SoundChunk) defines `start_time: f32`. Pseudocode `trackplayer.md` S0 defines `start_time: f64`.

`dec_offset` is also `f64` in the pseudocode but `f32` in the spec. For a 2-hour audio track at 44100 Hz, the total milliseconds would be ~7.2 million. `f32` has ~7 decimal digits of precision, so at 7.2 million ms, the resolution is about 1ms -- acceptable but tight. `f64` gives much more headroom.

**Recommendation**: Use `f64` for `start_time` and `dec_offset` (follow the pseudocode, fix the spec). Float precision matters for accumulated sums.

### [MEDIUM] MED-ALG-06: `decode_all` error handling for transient errors is retry-forever

**File**: `stream.md` S18, line 579

```
Err(_) => CONTINUE   // transient errors: retry
```

The catch-all `Err(_)` branch retries the decode loop forever if a decoder repeatedly returns a non-EOF, non-DecoderError error. The `DecodeError` enum includes `NotInitialized`, `InvalidData`, `UnsupportedFormat`, `NotFound`, `IoError`, `SeekFailed` -- several of which are permanent.

`NotInitialized`, `InvalidData`, `UnsupportedFormat`, and `NotFound` are permanent conditions -- retrying won't fix them. This creates an infinite loop.

**Recommendation**: Treat `NotInitialized`, `InvalidData`, `UnsupportedFormat`, and `NotFound` as permanent errors (break with error return). Treat `IoError` and `SeekFailed` as retryable with a max retry count (e.g., 3).

### [LOW] LOW-ALG-07: `graph_foreground_stream` AGC recomputes average every pixel

**File**: `stream.md` S14, line 453

```
LET avg_amp = agc_pages.iter().sum::<i32>() / AGC_PAGE_COUNT
```

This sums 16 elements for every pixel (up to `width` pixels, typically 320). Total: 320 x 16 = 5120 additions per frame. Trivial cost, but a running sum variable updated incrementally would be cleaner.

**No action needed** -- this is a style preference, not a bug.

---

## 3. FFI Safety

### [HIGH] HIGH-FFI-01: `PlayStream` FFI creates Arc from raw pointer that may not be Arc-originated

**File**: `heart_ffi.md` S1, lines 54-70

The FFI `PlayStream` function takes `sample_ptr: *mut SoundSample` and reconstructs an `Arc<Mutex<SoundSample>>` via `Arc::increment_strong_count` + `Arc::from_raw`. But the C signature in the spec (rust-heart.md S5.1) declares `PlayStream(sample: *mut SoundSample, ...)` -- suggesting the pointer is to a bare `SoundSample`, not to `Mutex<SoundSample>`. The FFI code treats it as `*const parking_lot::Mutex<SoundSample>` (line 67). These are different types with different layouts.

If C code creates a `SoundSample` via `TFB_CreateSoundSample` (which returns `*mut SoundSample` via `Box::into_raw`), and then passes it to `PlayStream`, the code would reconstruct an Arc from a Box-allocated pointer -- **undefined behavior**.

The assumption is that `PlayStream` is only ever called with pointers that originated from `Arc::into_raw`. But `TFB_CreateSoundSample` returns `Box::into_raw` pointers. These two raw pointer types are **not interchangeable** for Arc reconstruction.

**Recommendation**: Document clearly which FFI functions accept Box-originated vs Arc-originated pointers. Consider using different opaque handle types (or tagging) to prevent misuse. Or consistently use one allocation strategy at the FFI boundary.

### [HIGH] HIGH-FFI-02: `LoadMusicFile` returns raw inner Arc pointer but drops the Arc

**File**: `heart_ffi.md` S6, line 302

```
Ok(music_ref) => music_ref.0 as *mut c_void
```

`MusicRef.0` is an `Arc<Mutex<SoundSample>>`. Casting an Arc to `*mut c_void` is not `Arc::into_raw`. The Arc is dropped at the end of this expression (it goes out of scope), which decrements the reference count. If the refcount hits zero, the SoundSample is freed, and the returned pointer is **dangling**.

The correct approach is `Arc::into_raw(music_ref.0) as *mut c_void`, which "leaks" the Arc (prevents the refcount decrement), keeping the allocation alive until `DestroyMusic` calls `Arc::from_raw` to reclaim it.

The pseudocode at `music.md` S6 (get_music_data, line 138) creates the MusicRef correctly: `MusicRef(Arc::new(...))`. But the FFI layer at `heart_ffi.md` line 302 just casts the Arc rather than calling `into_raw`.

**Recommendation**: Fix line 302 to use `Arc::into_raw(music_ref.0) as *mut c_void`.

### [MEDIUM] MED-FFI-03: `SpliceTrack` track_text parameter type mismatch

**File**: `heart_ffi.md` S2, line 100; `rust-heart.md` S5.2 (SpliceTrack signature)

The spec declares `track_text: *const c_char` (C string, UTF-8), but the FFI pseudocode at line 102-103 says "track_text is UNICODE* (UCS-2/UTF-16LE) in C. Convert to UTF-8 String." and shows `utf16_ptr_to_option(track_text_ptr)` with a `*const u16` parameter.

The spec and pseudocode disagree on the type. The C code uses `UNICODE*` which is `uint16_t*` (UCS-2/UTF-16LE). The spec's function signature says `*const c_char`. The pseudocode is correct (UTF-16 input), but the spec signature is wrong.

**Recommendation**: Fix the spec's `SpliceTrack` signature to use `*const u16` for `track_text` (or `*const UNICODE`).

### [MEDIUM] MED-FFI-04: `TFB_ClearBufferTag` FFI has no way to find the containing SoundSample

**File**: `heart_ffi.md` S1, lines 87-89

```
FUNCTION TFB_ClearBufferTag(tag_ptr)
  IF tag_ptr.is_null() THEN RETURN END IF
  // Set containing Option to None (requires knowing the containing slot)
```

The comment acknowledges the problem: `clear_buffer_tag` in the Rust API (stream.md S16, lines 509-518) iterates `sample.buffer_tags` to find the slot by pointer comparison. But the FFI function only receives a `*mut SoundTag` -- it doesn't know which SoundSample the tag belongs to.

In the C code, tags are embedded in the sample's array, so the caller always has the sample context. The FFI shim needs the same context.

**Recommendation**: Either (a) change the FFI signature to also accept a `sample_ptr`, matching the Rust API's `clear_buffer_tag(sample, tag_ptr)`, or (b) add logic to find the sample from the tag pointer by scanning all sources.

### [MEDIUM] MED-FFI-05: Thread-local CString cache same-function invalidation

**File**: `heart_ffi.md` S2, lines 136-142

The `SUBTITLE_CACHE` and `SUBTITLE_TEXT_CACHE` are separate thread-local caches. The separate caches protect against cross-function invalidation (GetTrackSubtitle vs GetTrackSubtitleText). But if C code calls `GetTrackSubtitle()`, stores the pointer, then calls `GetTrackSubtitle()` again, the first pointer is dangling.

This matches C behavior exactly (the C code has the same limitation), but it should be explicitly documented in the FFI header comments.

**Recommendation**: Add a comment to the FFI header: "Returned string pointer is valid until the next call to the SAME function on the same thread."

---

## 4. Missing Mixer Functionality

### [HIGH] HIGH-MIX-01: `SourceProp` lacks PositionX/Y/Z -- positional audio is dead code

**File**: `sfx.md` S6, lines 102-109; `rust/src/sound/mixer/types.rs`

The SFX pseudocode calls:
```
mixer_source_f(source.handle, SourceProp::PositionX, x)
mixer_source_f(source.handle, SourceProp::PositionY, y)
mixer_source_f(source.handle, SourceProp::PositionZ, z)
```

Verified against actual source code:

1. `SourceProp` enum (`mixer/types.rs`) has only `Position = 0x1004`. There are no `PositionX`, `PositionY`, `PositionZ` variants.
2. `mixer_source_f` (`mixer/source.rs` line 221) only handles `SourceProp::Gain` -- returns `Err(MixerError::InvalidEnum)` for all other props.
3. `MixerSource` struct (`mixer/source.rs`) has no position fields (x, y, z) -- only `pos: u32` (byte position in buffer) and `count: u32` (fractional position).

This means ALL positional audio calls will return `Err(MixerError::InvalidEnum)` and be silently ignored (per CROSS-ERROR-01). Positional SFX will play at default position -- no stereo panning.

The plan's P03 document says: "Resolution: use three separate `mixer_source_f` calls to set X, Y, Z position components individually. No mixer modification needed."

**This resolution is wrong** -- three separate calls won't work because:
- `mixer_source_f` doesn't handle Position variants (returns InvalidEnum)
- `MixerSource` has no position storage fields
- The mix loop has no panning logic

The mixer itself needs modification to support positional audio. This is outside the Audio Heart plan scope, but the plan incorrectly claims it's handled.

**Recommendation**: Add a P00a preflight item to either (a) extend the mixer with PositionX/Y/Z support, or (b) explicitly document that positional audio is a NO-OP until the mixer supports it. Do not silently claim it works.

---

## 5. Spec/Pseudocode Inconsistencies

### [MEDIUM] MED-SPEC-01: `SoundSource` location conflict -- StreamEngine vs SoundSourceArray

**File**: `rust-heart.md` S3.1.2 defines `StreamEngine { sources: [Mutex<SoundSource>; NUM_SOUNDSOURCES], ... }`.
**File**: `rust-heart.md` S3.5.1 defines `SoundSourceArray { sources: [Mutex<SoundSource>; NUM_SOUNDSOURCES] }`.
**File**: Pseudocode consistently uses `SOURCES.sources[...]` (the SoundSourceArray from control.rs).

The sources array is defined in TWO places: inside `StreamEngine` (spec S3.1.2) AND in `SoundSourceArray` (spec S3.5.1). The pseudocode only uses the SoundSourceArray. StreamEngine's `sources` field is never referenced.

**Recommendation**: Remove `sources` from StreamEngine in the spec. StreamEngine should only contain: `fade`, `decoder_thread`, `shutdown`, `wake`.

### [MEDIUM] MED-SPEC-02: `TrackPlayerState.dec_offset` type -- spec says `f32`, pseudocode says `f64`

**File**: `rust-heart.md` S3.2.2 line 337: `dec_offset: f32`
**File**: `trackplayer.md` S0: `dec_offset: f64`

As noted in MED-ALG-05, accumulated floating-point sums should use `f64` to avoid precision drift. The pseudocode is correct.

**Recommendation**: Fix spec to use `f64`.

### [MEDIUM] MED-SPEC-03: `MusicRef` representation conflict in domain model

**File**: `rust-heart.md` S3.3.1 says `MusicRef(Arc<parking_lot::Mutex<SoundSample>>)`.
**File**: `domain-model.md` S1.1 says `MusicRef(*mut SoundSample)` -- raw pointer wrapper.
**File**: `music.md` uses Arc consistently.

The domain model has an outdated description.

**Recommendation**: Update domain-model.md to match the spec (Arc, not raw pointer).

### [MEDIUM] MED-SPEC-04: `SoundChunk.decoder` ownership -- `Option<Box<...>>` vs `Box<...>`

**File**: `rust-heart.md` S3.2.1 defines `decoder: Box<dyn SoundDecoder>` (non-optional).
**File**: `trackplayer.md` S0 defines `decoder: Option<Box<dyn SoundDecoder>>`.

Since NullDecoder exists in the codebase (`rust/src/sound/null.rs`), the non-optional `Box<dyn SoundDecoder>` from the spec is correct -- use NullDecoder for empty chunks instead of None.

**Recommendation**: Align pseudocode with spec -- use `Box<dyn SoundDecoder>` (non-optional). NullDecoder handles the empty case.

### [LOW] LOW-SPEC-05: Duplicate/non-monotonic line numbers in `execute_deferred_callbacks`

**File**: `stream.md` S11, lines 350-384

Line numbers jump from 370 back to 355, then from 383 back to 350. The function body appears to have overlapping line number ranges, making it confusing to reference specific lines.

**Recommendation**: Renumber lines in execute_deferred_callbacks to be monotonic.

---

## 6. Missing Validation & Edge Cases

### [MEDIUM] MED-EDGE-01: `play_stream` bounds check doesn't show error return

**File**: `stream.md` S5, line 71-74

Line 71 says `VALIDATE source_index < NUM_SOUNDSOURCES` but doesn't show what happens on failure. Line 74 does `SOURCES.sources[source_index].lock()` -- if source_index >= NUM_SOUNDSOURCES, this panics (array out of bounds).

`stop_stream`, `pause_stream`, `resume_stream`, `seek_stream` don't show bounds checks at all. `stop_source` and `clean_source` (control.md) do check bounds explicitly.

**Recommendation**: Add explicit bounds checks with `Err(AudioError::InvalidSource(source_index))` to all stream functions that take `source_index`.

### [MEDIUM] MED-EDGE-02: `sound_playing` uses `unwrap()` -- violates no-unwrap rule

**File**: `control.md` S9, line 135

```
LET sample = source.sample.as_ref().unwrap().lock()
```

This acquires the Sample lock while holding the Source lock (ordering: Source -> Sample, valid). However, `unwrap()` is used -- violating the "no unwrap() in production code" rule from the spec. The `is_some()` check at line 134 ensures this is logically safe, but `unwrap()` should be replaced.

**Recommendation**: Use `if let Some(sample_arc) = &source.sample { let sample = sample_arc.lock(); ... }`.

### [MEDIUM] MED-EDGE-03: `release_sound_bank_data` TOCTOU between source scan and stop

**File**: `sfx.md` S9, lines 186-201

Phase 1 scans all sources to find matches (lines 188-194). Phase 2 stops matched sources (lines 197-201). Between Phase 1 and Phase 2, another thread could have assigned a different sample to a source. The `source.sample = None` at line 200 could clear a **different** sample than the one that was matched.

**Recommendation**: In Phase 2, re-check `arc_matches(source.sample, sample)` before setting `source.sample = None`.

### [LOW] LOW-EDGE-04: `set_sfx_volume` and `set_speech_volume` API types inconsistent with spec

**File**: The spec (rust-heart.md S3.5.3) declares `pub fn set_sfx_volume(volume: f32)` and `pub fn set_speech_volume(volume: f32)` -- float parameters.

The pseudocode (control.md S7-8) treats them as `i32` (0..255 range) and divides by MAX_VOLUME.

The FFI layer (heart_ffi.md S5, lines 270-274) casts `c_int` to `i32`.

**Recommendation**: The pseudocode is correct (matching C behavior where volume is 0-255 integer). Fix the spec's function signatures to use `i32`.

---

## 7. Resource Management

### [MEDIUM] MED-RES-01: `TrackPlayerState` recursive Drop could stack overflow

**File**: `trackplayer.md` S0, specification S5 (REQ-TRACK-ASSEMBLE-19)

REQ-TRACK-ASSEMBLE-19 explicitly calls for "Iterative Drop for linked list." The `SoundChunk` struct uses `next: Option<Box<SoundChunk>>`, meaning the default `Drop` implementation IS recursive.

For typical usage (~150 chunks), the stack depth is safe. But the spec requires iterative Drop, and the plan mentions it. The pseudocode does not show a custom Drop implementation.

**Recommendation**: Include the iterative Drop implementation in the pseudocode:
```rust
impl Drop for SoundChunk {
    fn drop(&mut self) {
        let mut next = self.next.take();
        while let Some(mut chunk) = next {
            next = chunk.next.take();
        }
    }
}
```

### [LOW] LOW-RES-02: `destroy_sound_sample` does not reset `num_buffers`

**File**: `stream.md` S4, lines 60-66

The `num_buffers` field is not reset after destruction. If any code reads `sample.num_buffers` after destruction, it sees the old count but `sample.buffers` is empty.

**Recommendation**: Set `sample.num_buffers = 0` in `destroy_sound_sample`.

---

## 8. Requirement Coverage Gaps

### [MEDIUM] MED-REQ-01: REQ-STREAM-PLAY-02 (empty sample check) cannot be implemented as specified

**File**: `rust-heart.md` REQ-STREAM-PLAY-02: "When `play_stream()` is called with a `SoundSample` whose inner `Arc` strong count would drop to zero (logically empty), the system shall return `Err(AudioError::InvalidSample)`."

`play_stream` receives `sample_arc: Arc<Mutex<SoundSample>>` by value. The strong count is at least 2 (caller's reference + function parameter). It is impossible for the strong count to "drop to zero" while the function holds a reference. This requirement cannot be meaningfully implemented.

The pseudocode handles the real concern (no decoder) at line 90: `sample.decoder.as_mut().ok_or(AudioError::InvalidDecoder)?`.

**Recommendation**: Rewrite REQ-STREAM-PLAY-02 to say "return `Err(InvalidDecoder)` when the sample has no decoder" (which is already implemented in the pseudocode).

### [LOW] LOW-REQ-02: REQ-TRACK-ASSEMBLE-15 says "fully pre-decode" but pseudocode optimizes

The spec says to pre-decode all audio via `decode_all()`. The pseudocode (trackplayer.md S2, lines 145-165) only calls `decode_all()` as a fallback when `decoder.length()` returns 0.0. When length is known, it skips pre-decoding.

This optimization is correct and desirable but makes the requirement text inaccurate.

**Recommendation**: Update REQ-TRACK-ASSEMBLE-15 to say "compute duration for each track (via `length()` or `decode_all()` fallback)" instead of "fully pre-decode each".

---

## 9. Summary of Findings

| Severity | Count | Description |
|----------|-------|-------------|
| CRITICAL | 2 | Self-deadlocks (stop_stream double-lock, play_stream callback ordering) |
| HIGH | 6 | Deadlocks in seek/nav functions, dead positional audio, FFI Arc/Box confusion, FFI pointer leak |
| MEDIUM | 14 | Fade lock ordering, integer overflow, f32/f64 inconsistency, missing bounds checks, TOCTOU, spec inconsistencies |
| LOW | 4 | Style issues, documentation, minor redundancy |
| **Total** | **26** | |

### Must-Fix Before Implementation

1. **CRIT-CONC-01**: `stop_stream` self-deadlocks when calling `stop_source` (both lock same source)
2. **CRIT-CONC-02**: `play_stream` callbacks invoked under Source+Sample locks (inverts lock ordering with TRACK_STATE)
3. **HIGH-CONC-03/04**: Navigation functions and jump_track hold Source lock then call functions that re-acquire it
4. **HIGH-FFI-01**: PlayStream FFI confuses Box-originated and Arc-originated pointers
5. **HIGH-FFI-02**: LoadMusicFile drops Arc instead of calling into_raw (dangling pointer)
6. **HIGH-MIX-01**: PositionX/Y/Z don't exist in mixer -- positional audio silently broken, plan incorrectly claims it's handled

### Should-Fix Before Implementation

7. **HIGH-ALG-01**: Add missing `looping` field to SoundSample struct definition in spec
8. **MED-ALG-05/MED-SPEC-02**: Use f64 for accumulated time offsets (f32 precision insufficient)
9. **MED-ALG-06**: decode_all infinite loop on permanent errors (NotInitialized, InvalidData, etc.)
10. **MED-FFI-03**: SpliceTrack track_text type mismatch (c_char vs u16)
11. **MED-SPEC-01**: Remove redundant sources from StreamEngine
12. **MED-EDGE-03**: TOCTOU in release_sound_bank_data (re-check match in Phase 2)
13. **MED-CONC-06**: FadeState lock ordering violation (drop lock before set_music_volume)
14. **MED-RES-01**: Add iterative Drop for SoundChunk linked list (spec requires it, pseudocode omits it)
