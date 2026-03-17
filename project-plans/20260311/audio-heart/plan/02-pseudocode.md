# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-AUDIO-HEART.P02`

## Prerequisites
- Required: Phase P01a completed (analysis verification passed)

## Purpose

Provide numbered algorithmic pseudocode for every gap closure. Implementation phases reference line ranges from this pseudocode.

---

## PC-01: Canonical Music Loader

```text
01: FUNCTION load_music_canonical(filename: &str, uio_ctx: &UioContext) -> Result<MusicRef>
02:   VALIDATE filename is non-empty; RETURN Err(NullPointer) if empty
03:   EXTRACT file extension from filename
04:   IF no extension THEN RETURN Err(ResourceNotFound)
05:   SELECT decoder constructor based on extension (ogg, wav, aif, mod, duk)
06:   IF no matching decoder type THEN RETURN Err(ResourceNotFound)
07:   READ file bytes via UIO: uio_fopen(contentDir, filename, "rb")
08:   IF fopen fails THEN RETURN Err(ResourceNotFound)
09:   READ entire file into byte buffer via uio_fread
10:   CLOSE UIO file handle
11:   CREATE decoder from byte buffer using selected constructor
12:   IF decoder creation fails THEN RETURN Err(DecoderError)
13:   QUERY decoder length in seconds
14:   CREATE SoundSample via create_sound_sample(Some(decoder), NUM_BUFFERS_PER_SOURCE, None)
15:   IF sample creation fails THEN RETURN Err(MixerError)
16:   SET sample.length = decoder_length
17:   WRAP sample in MusicRef(Arc::new(Mutex::new(sample)))
18:   RETURN Ok(music_ref)
```

## PC-02: Canonical SFX Bank Loader

```text
01: FUNCTION load_sound_bank_canonical(filename: &str, uio_ctx: &UioContext) -> Result<SoundBank>
02:   VALIDATE filename is non-empty; RETURN Err(NullPointer) if empty
03:   READ bank listing file via UIO (one sound filename per line)
04:   IF fopen fails THEN RETURN Err(ResourceNotFound)
05:   READ file contents as text, SPLIT into lines
06:   CLOSE UIO file handle
07:   LET samples = empty Vec
08:   FOR EACH line in bank listing:
09:     TRIM whitespace from line
10:     IF line is empty THEN CONTINUE
11:     LET snd_path = construct full path (directory of bank file + "/" + line)
12:     EXTRACT extension from snd_path
13:     IF no extension THEN SKIP (log warning)
14:     SELECT decoder constructor based on extension
15:     IF no matching decoder THEN SKIP (log warning)
16:     READ sound file bytes via UIO
17:     IF fopen fails THEN SKIP (log warning)
18:     CREATE decoder from bytes
19:     IF decoder creation fails THEN SKIP (log warning)
20:     DECODE all audio: loop decoder.decode() until EOF, accumulate PCM bytes
21:     GET format and frequency from decoder
22:     CREATE SoundSample via create_sound_sample(None, 1, None)
23:     UPLOAD decoded PCM to mixer buffer: mixer_buffer_data(buf, format, pcm, freq)
24:     APPEND sample to samples Vec
25:   IF samples is empty THEN RETURN Err(ResourceNotFound)
26:   RETURN Ok(SoundBank { samples, source_file: Some(filename) })
```

## PC-03: Loader Consolidation Routing

```text
01: FUNCTION fileinst::load_music_file(filename: &str) -> Result<MusicRef>
02:   ACQUIRE file-load guard (concurrent load protection)
03:   CALL load_music_canonical(filename, &default_uio_context())
04:   GUARD dropped automatically on return (RAII)
05:
06: FUNCTION fileinst::load_sound_file(filename: &str) -> Result<SoundBank>
07:   ACQUIRE file-load guard
08:   CALL load_sound_bank_canonical(filename, &default_uio_context())
09:
10: FUNCTION heart_ffi::LoadMusicFile(filename_ptr) -> *mut c_void
11:   CONVERT C string to Rust &str
12:   CHECK init_state; RETURN null if not initialized
13:   CALL fileinst::load_music_file(filename)
14:   ON Ok(music_ref): ALLOCATE C handle, store MusicRef, RETURN handle pointer
15:   ON Err: log error, RETURN null
16:
17: FUNCTION heart_ffi::LoadSoundFile(filename_ptr) -> *mut c_void
18:   CONVERT C string to Rust &str
19:   CHECK init_state; RETURN null if not initialized
20:   CALL fileinst::load_sound_file(filename)
21:   ON Ok(bank): BUILD C STRING_TABLE with entries, RETURN table pointer
22:   ON Err: log error, RETURN null
```

## PC-04: Multi-Track Decoder Loading

```text
01: FUNCTION heart_ffi::SpliceMultiTrack(track_names_ptr, track_text_ptr)
02:   PARSE track_names array from C double-pointer (up to MAX_MULTI_TRACKS)
03:   PARSE track_text from C string pointer
04:   LET decoders: Vec<Option<Box<dyn SoundDecoder>>> = empty
05:   FOR EACH track_name in track_names:
06:     IF track_name is null THEN push None to decoders; CONTINUE
07:     READ file bytes via UIO for track_name
08:     IF read fails THEN push None to decoders; CONTINUE
09:     EXTRACT extension, SELECT decoder constructor
10:     CREATE decoder from bytes
11:     IF decoder creation fails THEN push None to decoders; CONTINUE
12:     PUSH Some(decoder) to decoders
13:   CALL trackplayer::splice_multi_track_with_decoders(decoders, texts)
14:
15: FUNCTION trackplayer::splice_multi_track_with_decoders(decoders, texts)
16:   VALIDATE track_count > 0
17:   FOR EACH (i, decoder_opt) in decoders:
18:     IF decoder_opt is None THEN CONTINUE
19:     LET decoder = decoder_opt.take()
20:     LET length_ms = decoder.length() * 1000.0
21:     CREATE SoundChunk with decoder, start_time = dec_offset
22:     SET chunk.run_time = ms_to_ticks(length_ms)
23:     APPEND chunk to sequence
24:     ADVANCE dec_offset by length_ms
25:   APPEND subtitle text from texts[0] to last_sub
26:   SET no_page_break = true
```

## PC-05: PLRPause Ref-Matching

```text
01: FUNCTION heart_ffi::PLRPause(music_ref_ptr)
02:   IF music_ref_ptr is null THEN RETURN
03:   IF is_sentinel(music_ref_ptr, ~0) THEN
04:     CALL music::plr_pause()
05:     RETURN
06:   LET caller_ref = borrow_music_ref(music_ref_ptr)
07:   CALL music::plr_pause_if_matching(&caller_ref)
08:
09: FUNCTION music::plr_pause_if_matching(music_ref: &MusicRef)
10:   LET state = MUSIC_STATE.lock()
11:   IF state.cur_music_ref matches music_ref by Arc::ptr_eq THEN
12:     DROP state
13:     CALL plr_pause()
14:   ELSE
15:     // Non-matching, non-wildcard: leave playback unchanged
16:     RETURN
```

## PC-06: NORMAL_VOLUME Fix

```text
01: IN control.rs:
02:   REMOVE local NORMAL_VOLUME constant
03:   USE types::NORMAL_VOLUME (160) in VolumeState::new()
04:   FIX test_normal_volume_is_max → test_normal_volume_canonical
05:     ASSERT NORMAL_VOLUME == 160
```

## PC-07: Pending-Completion State Machine

```text
01: ADD to TrackPlayerState:
02:   pending_completion: Option<Box<dyn Fn(i32) + Send>>
03:   pending_chunk_track_num: u32
04:
05: FUNCTION on_tagged_buffer(tag_data, state):
06:   VALIDATE tag chunk belongs to active sequence
07:   UPDATE cur_sub_chunk
08:   IF chunk has callback AND this is the last chunk of the logical phrase THEN
09:     IF state.pending_completion is Some THEN
10:       // Existing completion unclaimed — defer (spec §8.3.1 step 1)
11:       RETURN
12:     SET state.pending_completion = chunk.callback.clone()
13:     SET state.pending_chunk_track_num = chunk.track_num
14:
15: FUNCTION poll_pending_track_completion() -> Option<Box<dyn Fn(i32) + Send>>
16:   LET state = TRACK_STATE.lock()
17:   TAKE state.pending_completion (returns Some or None, clears slot)
18:
19: FUNCTION commit_track_advancement()
20:   LET state = TRACK_STATE.lock()
21:   // Advance to next logical phrase
22:   // Update cur_chunk, cur_sub_chunk for the next phrase
23:   // PlayingTrack() now returns the next phrase's track_num + 1
24:
25: FUNCTION stop_track():
26:   ... existing logic ...
27:   SET state.pending_completion = None  // Discard without invoking
```

## PC-08: WaitForSoundEnd Full Spec

```text
01: FUNCTION wait_for_sound_end(channel: u32)
02:   // Selector domain (spec §13.3)
03:   LET wait_sources = MATCH channel:
04:     0..NUM_SOUNDSOURCES => vec![channel as usize]
05:     WAIT_ALL_SOURCES (0xFFFFFFFF) => (0..NUM_SOUNDSOURCES).collect()
06:     _ => (0..NUM_SOUNDSOURCES).collect()  // default = all
07:   LOOP:
08:     IF quit_posted() THEN BREAK
09:     LET any_active = false
10:     FOR source_idx in wait_sources:
11:       LET source = lock source[source_idx]
12:       // Inactive-source rule
13:       IF source.sample is None THEN CONTINUE
14:       IF source.handle is not allocated THEN CONTINUE
15:       // Paused = still active
16:       IF source.pause_time != 0 THEN any_active = true; CONTINUE
17:       // Streaming check
18:       IF source.stream_should_be_playing THEN any_active = true; CONTINUE
19:       // Non-streaming: query mixer state
20:       IF mixer_source_state(source.handle) == Playing THEN any_active = true; CONTINUE
21:     IF NOT any_active THEN BREAK
22:     SLEEP 10ms
```

## PC-09: Pre-Init Guard

```text
01: ADD global: static STREAM_INITIALIZED: AtomicBool = AtomicBool::new(false)
02:
03: FUNCTION init_stream_decoder():
04:   ... existing logic ...
05:   ON success: STREAM_INITIALIZED.store(true)
06:
07: FUNCTION uninit_stream_decoder():
08:   STREAM_INITIALIZED.store(false)
09:   ... existing logic ...
10:
11: FUNCTION is_initialized() -> bool:
12:   STREAM_INITIALIZED.load(Ordering::Acquire)
13:
14: IN heart_ffi.rs, EACH function:
15:   IF NOT is_initialized() THEN RETURN ABI failure value per §19.3 map
```

## PC-10: Diagnostic Cleanup

```text
01: FOR EACH eprintln! in sound modules:
02:   IF prefix is "[PARITY]" THEN:
03:     CONVERT to log::trace!("...") (remove [PARITY] prefix)
04:   ELSE IF operational (mixer pump startup, error paths) THEN:
05:     CONVERT to log::debug!("...") or log::warn!("...")
06:   ELSE IF one-time debug output (PLRPlaySong, PlayChannel, LoadSoundFile diagnostics) THEN:
07:     CONVERT to log::debug!("...") or REMOVE entirely
08:   ELSE IF error-path logging (decoder failures, load failures) THEN:
09:     CONVERT to log::warn!("...")
```

## PC-11: Warning Suppression Removal

```text
01: FOR EACH module with #![allow(dead_code, unused_imports, unused_variables)]:
02:   REMOVE the #![allow(...)] attribute
03:   COMPILE and collect warnings
04:   FOR EACH dead_code warning:
05:     IF function is called from heart_ffi.rs (cfg-gated) THEN
06:       ADD #[cfg(feature = "audio_heart")] gate or #[allow(dead_code)] on that item
07:     ELSE IF function is genuinely unused THEN
08:       REMOVE function
09:   FOR EACH unused_imports warning:
10:     REMOVE unused import
11:   FOR EACH unused_variables warning:
12:     PREFIX with _ or use in implementation
```

---

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P02.md`
