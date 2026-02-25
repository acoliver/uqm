# C Audio Streaming Pipeline — Technical Analysis & EARS Requirements

**Scope:** The six C source files that form the "heart" of UQM's audio streaming, music playback, sound effects, and volume control pipeline — the last major C-only audio subsystem to be ported to Rust.

**Files analyzed:**

| File | Lines | Role |
|------|-------|------|
| `stream.c` | 815 | Background streaming thread, decoder pump, scope buffer, fade |
| `trackplayer.c` | 882 | Speech/dialogue track orchestration, subtitles, seeking |
| `music.c` | 233 | Music loading, PLR* playback API, speech helpers |
| `sfx.c` | 308 | Sound effect triggering, positional audio, sound banks |
| `sound.c` | 178 | Volume control, source cleanup, global queries |
| `fileinst.c` | 87 | File-based sound/music instance loading |

**Headers referenced:** `sndintrn.h`, `sound.h`, `audiocore.h`, `stream.h`, `trackint.h`, `trackplayer.h`, `sndlib.h`, `decoder.h`

---

## Part 1: Technical Analysis

### 1. Architecture Overview

The UQM audio pipeline is layered:

```
┌─────────────────────────────────────────────────────────┐
│  Game Logic (comm screens, menus, combat, etc.)         │
├──────────┬──────────┬───────────┬───────────────────────┤
│ music.c  │ sfx.c    │ trackplayer.c │ fileinst.c        │
│ PLR*     │ Play     │ SpliceTrack   │ LoadMusicFile     │
│ snd_*    │ Channel  │ PlayTrack     │ LoadSoundFile     │
├──────────┴──────────┴───────────┴───────────────────────┤
│              stream.c                                    │
│  PlayStream / StopStream / PauseStream / ResumeStream   │
│  StreamDecoderTaskFunc (background thread)               │
│  TFB_SoundSample / TFB_SoundTag management              │
│  SetMusicStreamFade / GraphForegroundStream              │
├──────────────────────────────────────────────────────────┤
│              sound.c                                     │
│  soundSource[] array, StopSource, CleanSource            │
│  Volume control (SetSFXVolume, SetSpeechVolume, etc.)    │
│  SoundPlaying, WaitForSoundEnd                           │
├──────────────────────────────────────────────────────────┤
│              audiocore.h (API layer)                      │
│  audio_SourcePlay, audio_BufferData, etc.                │
├──────────────────────────────────────────────────────────┤
│       Rust mixer (rust_mixer.h) / Rust rodio backend     │
│       Rust decoders (wav, ogg, mod, dukaud)              │
└──────────────────────────────────────────────────────────┘
```

There are three distinct audio "lanes":

1. **Music** — Uses `MUSIC_SOURCE` (source index `LAST_SFX_SOURCE + 1 = 5`). Streamed continuously via `PlayStream()`. Managed by `PLRPlaySong`/`PLRStop`/etc. in `music.c`.
2. **Speech** — Uses `SPEECH_SOURCE` (source index `MUSIC_SOURCE + 1 = 6`). Streamed via `PlayStream()` but orchestrated by the track player (`trackplayer.c`) which manages chapter transitions, subtitles, and seek.
3. **Sound Effects** — Use sources `FIRST_SFX_SOURCE` (0) through `LAST_SFX_SOURCE` (4), i.e. 5 channels (`NUM_SFX_CHANNELS = 5`). NOT streamed — entirely pre-decoded and played with a single `audio_SourcePlay`. Managed by `sfx.c`.

### 2. File Dependency Graph / Call Flow

```
fileinst.c
  LoadSoundFile() ──→ _GetSoundBankData() [sfx.c]
  LoadMusicFile() ──→ _GetMusicData() [music.c]

music.c
  _GetMusicData() ──→ SoundDecoder_Load() → TFB_CreateSoundSample()
  _ReleaseMusicData() ──→ StopStream() → SoundDecoder_Free() → TFB_DestroySoundSample()
  PLRPlaySong() ──→ PlayStream() [stream.c]
  PLRStop() ──→ StopStream() [stream.c]
  PLRPlaying() ──→ PlayingStream() [stream.c]
  PLRSeek() ──→ SeekStream() [stream.c]
  PLRPause() ──→ PauseStream() [stream.c]
  PLRResume() ──→ ResumeStream() [stream.c]
  snd_PlaySpeech() ──→ PlayStream() [stream.c]
  snd_StopSpeech() ──→ StopStream() [stream.c]
  SetMusicVolume() ──→ audio_Sourcef()
  FadeMusic() ──→ SetMusicStreamFade() [stream.c]

sfx.c
  PlayChannel() ──→ StopSource() [sound.c] → CheckFinishedChannels() → audio_SourcePlay()
  _GetSoundBankData() ──→ SoundDecoder_Load() → SoundDecoder_DecodeAll() → TFB_CreateSoundSample()
  _ReleaseSoundBankData() ──→ StopSource() → SoundDecoder_Free() → TFB_DestroySoundSample()

sound.c
  StopSource() ──→ audio_SourceStop() → CleanSource()
  CleanSource() ──→ audio_SourceUnqueueBuffers() → audio_SourceRewind()
  StopSound() ──→ StopSource() for all SFX sources
  SoundPlaying() ──→ PlayingStream() for stream sources, audio_GetSourcei() for SFX
  WaitForSoundEnd() ──→ SoundPlaying() / ChannelPlaying()
  FadeMusic() ──→ SetMusicStreamFade() [stream.c]

stream.c
  PlayStream() ──→ StopStream() → SoundDecoder_Decode() → audio_BufferData() → audio_SourceQueueBuffers() → audio_SourcePlay()
  StreamDecoderTaskFunc() ──→ processMusicFade() → process_stream() [background thread loop]
  process_stream() ──→ audio_SourceUnqueueBuffers() → SoundDecoder_Decode() → audio_BufferData() → audio_SourceQueueBuffers()

trackplayer.c
  SpliceTrack() ──→ SoundDecoder_Load() → create_SoundChunk() → TFB_CreateSoundSample()
  PlayTrack() ──→ PlayStream() [stream.c]
  StopTrack() ──→ StopStream() → destroy_SoundChunk_list() → TFB_DestroySoundSample()
  JumpTrack() ──→ seek_track()
  FastForward_Smooth() ──→ seek_track()
  FastReverse_Smooth() ──→ seek_track() → PlayStream()
  FastForward_Page() ──→ find_next_page() → PlayStream() / seek_track()
  FastReverse_Page() ──→ find_prev_page() → PlayStream()
  OnChunkEnd() [callback] ──→ SoundDecoder_Rewind() → TFB_TagBuffer()
  OnStreamStart() [callback] ──→ sets sample->decoder, sample->offset
  OnStreamEnd() [callback] ──→ clears cur_chunk, cur_sub_chunk
  OnBufferTag() [callback] ──→ DoTrackTag()
```

### 3. The Streaming Thread Model (`stream.c`)

#### 3.1 Thread Lifecycle

**Initialization** (`InitStreamDecoder`, line 786–798):
- Creates `fade_mutex` with `SYNC_CLASS_AUDIO`
- Spawns a dedicated background task via `AssignTask(StreamDecoderTaskFunc, 1024, "audio stream decoder")`
- The task runs as a separate thread with its own stack

**Shutdown** (`UninitStreamDecoder`, line 800–814):
- Calls `ConcludeTask(decoderTask)` which sets the `TASK_EXIT` flag and waits for the thread to finish
- Destroys `fade_mutex`

#### 3.2 Main Loop (`StreamDecoderTaskFunc`, lines 535–579)

The decoder thread runs in a continuous loop:

```
while (!Task_ReadState(task, TASK_EXIT)):
    processMusicFade()                    // handle volume fades
    for each source from MUSIC_SOURCE to NUM_SOUNDSOURCES-1:
        lock source->stream_mutex
        if source has valid sample, decoder, and stream_should_be_playing:
            process_stream(source)
            active_streams++
        unlock source->stream_mutex
    if active_streams == 0:
        HibernateThread(ONE_SECOND / 10)  // 100ms sleep when idle
    else:
        TaskSwitch()                       // yield CPU when busy
```

Key observations:
- Only iterates over streaming sources (indices 5 and 6: MUSIC_SOURCE and SPEECH_SOURCE), NOT SFX channels
- Thread throttles to 10 Hz when no streams are active
- Each source is individually mutex-locked during processing
- The thread checks `stream_should_be_playing` flag, `sample` pointer, `decoder` pointer, and `decoder->error` before processing

#### 3.3 Stream Processing (`process_stream`, lines 353–503)

For each active source, the stream processor:

1. **Queries buffer state**: Gets `audio_BUFFERS_PROCESSED` and `audio_BUFFERS_QUEUED` counts from the audio backend
2. **Handles stall detection** (lines 365–388):
   - If `processed == 0` (nothing played yet): checks source state
   - If source is NOT playing AND queued == 0 AND decoder hit EOF: stream is finished → sets `stream_should_be_playing = FALSE`, calls `OnEndStream` callback
   - If source is NOT playing but there ARE queued buffers: buffer underrun → restarts playback with `audio_SourcePlay`
3. **Buffer recycling loop** (lines 391–502): For each processed buffer:
   - Unqueues the buffer from the audio source
   - Processes `OnTaggedBuffer` callback if buffer was tagged (used by trackplayer for subtitle sync)
   - Removes scope data for oscilloscope display
   - Handles decoder EOF: calls `OnEndChunk` callback to potentially get a new decoder (trackplayer chapter transitions)
   - Handles decoder errors: skips, continues
   - Decodes new audio: `SoundDecoder_Decode(decoder)` fills the decoder's internal buffer
   - Uploads decoded data: `audio_BufferData(buffer, ...)` then `audio_SourceQueueBuffers`
   - Processes `OnQueueBuffer` callback
   - Adds scope data for oscilloscope

#### 3.4 Stream Start (`PlayStream`, lines 42–128)

The `PlayStream()` function is called from the main thread (NOT the decoder thread):

1. Stops any existing stream on the source
2. Calls `OnStartStream` callback (used by trackplayer to set the correct decoder)
3. Clears all buffer tags
4. If `rewind` is true: rewinds the decoder; else: calculates time offset from decoder position
5. Sets `soundSource[source].sample = sample`
6. Sets `decoder->looping` but sets `audio_LOOPING = false` on the source (looping is handled by the decoder, not the audio backend)
7. If `scope` is true: pre-allocates cyclic scope buffer (`num_buffers * buffer_size + PAD_SCOPE_BYTES`)
8. Pre-fills all `num_buffers` buffers by decoding audio in a loop:
   - Decodes via `SoundDecoder_Decode(decoder)`
   - Uploads via `audio_BufferData` and `audio_SourceQueueBuffers`
   - Calls `OnQueueBuffer` callback
   - Adds scope data
   - On EOF: calls `OnEndChunk` to potentially switch decoders (for multi-chunk tracks)
9. Records timestamps: `sbuf_lasttime`, `start_time` (adjusted for offset), `pause_time = 0`
10. Sets `stream_should_be_playing = TRUE`
11. Calls `audio_SourcePlay()` to begin playback

#### 3.5 Music Fade (`processMusicFade`, lines 505–533; `SetMusicStreamFade`, lines 763–783)

The fade system runs on the decoder thread (called every iteration):

- Protected by `fade_mutex`
- `SetMusicStreamFade()` (called from main thread via `FadeMusic()`) sets:
  - `musicFadeStartTime` = current time
  - `musicFadeInterval` = duration in game time units
  - `musicFadeStartVolume` = current `musicVolume`
  - `musicFadeDelta` = `endVolume - startVolume`
  - Returns `false` if `howLong == 0` (immediate set, no fade)
- `processMusicFade()` on each tick:
  - If `musicFadeInterval == 0`: no fade active, return
  - Calculate elapsed time, clamp to interval
  - Linear interpolation: `newVolume = startVolume + delta * elapsed / interval`
  - Calls `SetMusicVolume(newVolume)`
  - When `elapsed >= interval`: sets `musicFadeInterval = 0` to end fade

#### 3.6 Oscilloscope / Scope Buffer

The scope buffer is a cyclic ring buffer attached to `TFB_SoundSource`:

- **Fields**: `sbuffer` (data pointer), `sbuf_size`, `sbuf_head` (read pointer), `sbuf_tail` (write pointer), `sbuf_lasttime`
- **`add_scope_data`** (line 321–351): Copies decoded audio bytes to the tail of the ring buffer, wrapping around when needed
- **`remove_scope_data`** (line 308–319): Advances the head pointer by the byte-size of a dequeued buffer
- **`GraphForegroundStream`** (lines 596–759): Reads from the scope buffer to produce oscilloscope display data:
  - Prefers speech source over music (if speech is available and `wantSpeech`)
  - Uses `readSoundSample()` helper to handle 8-bit (unsigned → signed conversion) and 16-bit samples
  - Calculates time delta from `sbuf_lasttime` to find current read position
  - Adjusts step size to source frequency (normalized to 11025 Hz units)
  - Includes Automatic Gain Control (AGC) with:
    - 16-page running average (`AGC_PAGE_COUNT = 16`, `DEF_PAGE_MAX = 28000`)
    - 8-frame sub-pages (`AGC_FRAME_COUNT = 8`)
    - Voice Activity Detection (VAD) with `VAD_MIN_ENERGY = 100` threshold — pauses in speech don't affect the running average

### 4. The Track Player State Machine (`trackplayer.c`)

The track player manages dialogue/speech playback with subtitle synchronization. It's the most complex subsystem.

#### 4.1 Core Data Structures

**`TFB_SoundChunk`** (defined in `trackint.h`):
```c
struct tfb_soundchunk {
    TFB_SoundDecoder *decoder;   // decoder for this chunk's audio segment
    float start_time;            // relative time from track start (seconds)
    int tag_me;                  // 1 if this chunk has subtitles/callbacks
    uint32 track_num;            // logical track number (for PlayingTrack())
    UNICODE *text;               // subtitle text (heap-allocated)
    CallbackFunction callback;   // comm callback, executed on chunk start
    struct tfb_soundchunk *next; // linked list pointer
};
```

**Static state** (lines 30–55 of `trackplayer.c`):
- `track_count` — total number of logical tracks
- `no_page_break` — flag for combining multi-tracks into one
- `sound_sample` — THE single `TFB_SoundSample` used for speech (modified in-place during playback)
- `tracks_length` — total length of all tracks in game time units (`volatile uint32`)
- `chunks_head` / `chunks_tail` — linked list of all chunks
- `last_sub` — last chunk that has subtitle text
- `cur_chunk` — currently playing chunk (guarded by `stream_mutex`)
- `cur_sub_chunk` — currently displayed subtitle chunk (guarded by `stream_mutex`)

#### 4.2 Track Assembly (`SpliceTrack`, lines 427–603)

`SpliceTrack(TrackName, TrackText, TimeStamp, callback)` builds the chunk list before playback:

**Case 1: TrackName is NULL** — Appending subtitle text to existing track:
- Requires `track_count > 0` and valid `last_sub`
- Splits `TrackText` into pages via `SplitSubPages()`
- Makes last page's timestamp negative (= suggested, not mandatory)
- Concatenates first page onto `last_sub->text`
- For remaining pages: tries `find_next_page()` to fill pre-allocated nodes; if none, creates new decoder+chunk with `SoundDecoder_Load(contentDir, last_track_name, 4096, dec_offset, time_stamps[page])`

**Case 2: TrackName is non-NULL** — Adding a new track:
- Copies track name to `last_track_name` static buffer (128 bytes)
- Splits subtitles into pages
- Makes last page timestamp negative
- If `no_page_break` and tracks exist: concatenates first page to last subtitle (multi-track mode)
- Otherwise: increments `track_count`
- Parses timestamps from `TimeStamp` string via `GetTimeStamps()` if provided
- Resets `dec_offset = 0` for the new track
- For each timestamp/page: loads a decoder segment with `SoundDecoder_Load(contentDir, TrackName, 4096, dec_offset, time_stamps[page])`
  - Buffer size: 4096 bytes
  - `startTime`: accumulated offset in ms
  - `runTime`: timestamp value (negative = play to end of file)
- Creates the `sound_sample` with 8 buffers and `trackCBs` callbacks (first chunk only)
- Each chunk is appended to the linked list with `tag_me = 1`, subtitle text, and callback

#### 4.3 Multi-Track Splicing (`SpliceMultiTrack`, lines 363–423)

`SpliceMultiTrack(TrackNames[], TrackText)` combines multiple audio files into one track:

- Loads up to `MAX_MULTI_TRACKS = 20` decoders with buffer size 32768 and runTime `−3 * TEXT_SPEED`
- Calls `SoundDecoder_DecodeAll()` on each (fully pre-decodes)
- Appends each as a new chunk in the linked list
- All chunks share the same `track_num` as the current track
- Concatenates `TrackText` onto `last_sub->text`
- Sets `no_page_break = 1`

#### 4.4 Subtitle Page Splitting (`SplitSubPages`, lines 321–356)

- Splits text at `\r\n` boundaries
- Adds ellipsis (`..`) prefix for continuations and (`...`) suffix for mid-word breaks
- Calculates display time as `text_length * TEXT_SPEED` (where `TEXT_SPEED = 80` ms per character)
- Minimum time per page: 1000ms

#### 4.5 Playback Control

**`PlayTrack()`** (line 103–116):
- Sets `tracks_length = tracks_end_time()`
- Sets `cur_chunk = chunks_head`
- Calls `PlayStream(sound_sample, SPEECH_SOURCE, false, true, true)` — no looping, with scope, with rewind

**`StopTrack()`** (line 172–196):
- Stops stream, resets all state (`track_count`, `tracks_length`, `cur_chunk`, `cur_sub_chunk`)
- Destroys entire chunk list and frees all decoders
- Destroys `sound_sample` (sets `sample->decoder = NULL` first to avoid double-free since decoders are owned by chunks)

**`JumpTrack()`** (line 92–100):
- Seeks to `tracks_length + 1` (past the end), effectively stopping playback while preserving data structures for rewind

**`PauseTrack()`** / **`ResumeTrack()`** (lines 118–151):
- Delegates to `PauseStream`/`ResumeStream`
- `ResumeTrack` checks audio source state is `audio_PAUSED` before resuming

**`PlayingTrack()`** (line 153–169):
- Returns `cur_chunk->track_num + 1` if playing, 0 if not

#### 4.6 Seeking

**`seek_track(offset)`** (lines 612–663):
- Clamps offset to `[0, tracks_length + 1]`
- Adjusts `soundSource[SPEECH_SOURCE].start_time = GetTimeCounter() - offset`
- Walks the chunk list to find which chunk should be playing at `offset`
- Tracks the last tagged chunk for callback purposes
- If found: seeks the decoder to the right position, sets `sound_sample->decoder`, calls `DoTrackTag` if tagged
- If offset is past the end: stops stream, clears `cur_chunk` and `cur_sub_chunk`

**`FastReverse_Smooth()`** / **`FastForward_Smooth()`** (lines 680–716):
- Gets current position, adjusts by `±ACCEL_SCROLL_SPEED` (300 time units)
- Calls `seek_track()`
- `FastReverse_Smooth` also restarts the stream if it had ended

**`FastReverse_Page()`** / **`FastForward_Page()`** (lines 718–760):
- Jumps to previous/next chunk with `tag_me` set (subtitle page boundary)
- Uses `find_prev_page()`/`find_next_page()` to traverse the list
- Restarts playback via `PlayStream()`

#### 4.7 Callbacks

The track player registers four callbacks on `sound_sample` (lines 65–72):

1. **`OnStreamStart`** (line 212–229): Called by `PlayStream()` before initial buffering. Sets `sample->decoder` and `sample->offset` from `cur_chunk`. Calls `DoTrackTag` if chunk is tagged.

2. **`OnChunkEnd`** (line 234–260): Called by `process_stream()` when decoder reaches EOF. Advances `cur_chunk` to `cur_chunk->next`. Sets new decoder. Rewinds new decoder. Tags the buffer with the new chunk pointer (for subtitle sync when buffer actually *plays*).

3. **`OnStreamEnd`** (line 263–272): Called when all buffers are played and no more data. Clears `cur_chunk` and `cur_sub_chunk`.

4. **`OnBufferTag`** (line 277–289): Called when a tagged buffer finishes playing. Extracts the `TFB_SoundChunk*` from `tag->data`, clears the tag, and calls `DoTrackTag()` to update subtitle display and fire the chunk's callback.

**`DoTrackTag`** (line 198–207): Under mutex, fires `chunk->callback(0)` and sets `cur_sub_chunk = chunk`.

### 5. Music Loading End-to-End (`music.c`, `fileinst.c`)

#### 5.1 Resource-Based Loading

```
Game code
  → LoadMusicInstance(RESOURCE res)     [reslib]
    → _GetMusicData(fp, length)         [music.c, line 163–202]
      → SoundDecoder_Load(contentDir, filename, 4096, 0, 0)  [decoder layer]
      → TFB_CreateSoundSample(decoder, 64, NULL)             [stream.c]
      → stores sample pointer in MUSIC_REF (which is TFB_SoundSample**)
```

**`_GetMusicData`** (music.c:163–202):
- Gets filename from `_cur_resfile_name`
- Validates with `CheckMusicResName()` (warns if file doesn't exist)
- Loads decoder with 4096-byte buffer, no time offsets
- Creates sample with 64 streaming buffers, no callbacks (callbacks are per-source, not per-sample for music)
- Returns `MUSIC_REF` (which is `TFB_SoundSample**`)

#### 5.2 File-Based Loading

**`LoadMusicFile`** (fileinst.c:53–87):
- Guards against concurrent loading (`_cur_resfile_name` race check — noted as FIXME)
- Validates filename, opens resource file
- Delegates to `_GetMusicData`

**`LoadSoundFile`** (fileinst.c:27–51):
- Same pattern as `LoadMusicFile` but delegates to `_GetSoundBankData`

#### 5.3 Music Playback

**`PLRPlaySong(MusicRef, Continuous, Priority)`** (music.c:31–46):
- Dereferences `MusicRef` to get `TFB_SoundSample*`
- Locks `MUSIC_SOURCE` stream mutex
- Calls `PlayStream(*pmus, MUSIC_SOURCE, Continuous, true, true)` — always scoped, always rewinds
- Records `curMusicRef`
- `Priority` parameter is ignored

**`PLRStop(MusicRef)`** (music.c:48–59):
- Accepts specific `MusicRef` or wildcard `(MUSIC_REF)~0`
- Stops stream, clears `curMusicRef`

**`PLRPlaying(MusicRef)`** (music.c:61–76):
- Checks if `curMusicRef` matches, then queries `PlayingStream(MUSIC_SOURCE)`

**`PLRSeek`**, **`PLRPause`**, **`PLRResume`** (music.c:78–109):
- Thin wrappers around `SeekStream`, `PauseStream`, `ResumeStream` with mutex locking

**`snd_PlaySpeech(SpeechRef)`** (music.c:112–125):
- Plays music-as-speech on `SPEECH_SOURCE` — no scope, not looping

**`snd_StopSpeech()`** (music.c:127–138):
- Stops the speech source stream

#### 5.4 Music Release

**`_ReleaseMusicData`** (music.c:204–232):
- If the sample is currently playing on `MUSIC_SOURCE`, stops the stream first
- Detaches decoder from sample, frees decoder separately
- Destroys sample, frees the `MUSIC_REF` allocation

### 6. Sound Effects (`sfx.c`)

#### 6.1 SFX Loading (`_GetSoundBankData`, sfx.c:158–257)

Sound banks are text files listing audio filenames. Loading:

1. Extracts directory prefix from `_cur_resfile_name`
2. Reads each line to get a filename
3. For each file (up to `MAX_FX = 256`):
   - Loads decoder: `SoundDecoder_Load(contentDir, filename, 4096, 0, 0)`
   - Creates sample with 1 buffer, no callbacks: `TFB_CreateSoundSample(NULL, 1, NULL)` — decoder is NOT stored (set to NULL)
   - **Fully pre-decodes**: `SoundDecoder_DecodeAll(decoder)`
   - Uploads entire decoded audio into the single buffer: `audio_BufferData(sample->buffer[0], ...)`
   - Records length for informational purposes
   - Frees the decoder (data is now in the audio buffer)
4. Allocates a `STRING_TABLE` with `snd_ct` entries
5. Each entry stores a heap-allocated `TFB_SoundSample**` pointing to the sample

#### 6.2 SFX Playback (`PlayChannel`, sfx.c:35–60)

```c
PlayChannel(channel, snd, pos, positional_object, priority)
```

1. Gets `SOUNDPTR` (= `TFB_SoundSample**`) from `GetSoundAddress(snd)` which delegates to `GetStringAddress()`
2. Calls `StopSource(channel)` to halt any current playback on this channel
3. Calls `CheckFinishedChannels()` to clean up all stopped SFX sources
4. Dereferences to get `TFB_SoundSample*`
5. Sets `soundSource[channel].sample` and `positional_object`
6. Calls `UpdateSoundPosition(channel, pos)` (respects `optStereoSFX` setting)
7. Binds the sample's single buffer to the source: `audio_Sourcei(handle, audio_BUFFER, sample->buffer[0])`
8. Plays: `audio_SourcePlay(handle)`

SFX are NOT streamed — the entire decoded audio is in a single audio buffer. No scope buffer, no decoder thread involvement.

#### 6.3 Positional Audio (`UpdateSoundPosition`, sfx.c:113–146)

- Converts game coordinates to 3D audio position using `ATTENUATION = 160.0f`
- Maps `(x, y)` to `(x/ATTENUATION, 0, y/ATTENUATION)` for audio positioning
- Enforces `MIN_DISTANCE = 0.5f` — objects too close are scaled outward along the same vector
- Non-positional sounds are placed at `(0, 0, -1)` (directly in front of listener)

#### 6.4 SFX Release (`_ReleaseSoundBankData`, sfx.c:259–294)

- Iterates all entries in the `STRING_TABLE`
- For each sample: checks all `NUM_SOUNDSOURCES` to see if it's currently playing; if so, stops it
- Frees decoder (if any — usually NULL for SFX), then destroys sample
- Frees the `STRING_TABLE`

#### 6.5 Channel Queries

- **`ChannelPlaying(channel)`** (sfx.c:89–99): Queries `audio_SOURCE_STATE`, returns TRUE if `audio_PLAYING`
- **`GetPositionalObject(channel)`** / **`SetPositionalObject(channel, obj)`** (sfx.c:101–111): Direct accessor for `soundSource[channel].positional_object`
- **`SetChannelVolume(channel, volume, priority)`** (sfx.c:148–156): Sets gain as `(volume/MAX_VOLUME) * sfxVolumeScale`

### 7. Volume Control Architecture

#### 7.1 Global State (`sound.c`)

```c
int musicVolume = NORMAL_VOLUME;  // 160, range [0, MAX_VOLUME=255]
float musicVolumeScale;           // set externally (from options)
float sfxVolumeScale;             // set externally
float speechVolumeScale;          // set externally
```

#### 7.2 Volume Functions

- **`SetMusicVolume(Volume)`** (music.c:147–152): `audio_Sourcef(MUSIC_SOURCE.handle, audio_GAIN, (Volume / 255.0) * musicVolumeScale)`. Also stores `musicVolume = Volume` for fade reference.
- **`SetSFXVolume(volume)`** (sound.c:143–150): Sets gain on ALL SFX sources (`FIRST_SFX_SOURCE` through `LAST_SFX_SOURCE`)
- **`SetSpeechVolume(volume)`** (sound.c:153–156): Sets gain on `SPEECH_SOURCE` only
- **`SetChannelVolume(channel, volume, priority)`** (sfx.c:148–156): Per-channel SFX volume, `(volume/MAX_VOLUME) * sfxVolumeScale`
- **`FadeMusic(end_vol, TimeInterval)`** (sound.c:158–176):
  - If `QuitPosted` or `TimeInterval < 0`: clamp to 0 (instant)
  - Calls `SetMusicStreamFade(TimeInterval, end_vol)`
  - If fade rejected (interval=0): directly calls `SetMusicVolume(end_vol)`, returns current time
  - Otherwise returns `GetTimeCounter() + TimeInterval + 1` (expected completion time)

### 8. Key Data Structures

#### 8.1 `TFB_SoundSample` (sndintrn.h:41–51)

```c
struct tfb_soundsample {
    TFB_SoundDecoder *decoder;       // current decoder (may change during playback!)
    float length;                    // total decoder chain length in seconds
    audio_Object *buffer;            // array of audio buffer handles
    uint32 num_buffers;              // count of buffers (8 for speech, 64 for music, 1 for SFX)
    TFB_SoundTag *buffer_tag;        // parallel array for buffer tagging (lazily allocated)
    sint32 offset;                   // initial time offset
    void* data;                      // user-defined data
    TFB_SoundCallbacks callbacks;    // user-defined callbacks (5 function pointers)
};
```

#### 8.2 `TFB_SoundSource` (sndintrn.h:54–72) — the global `soundSource[]` array

```c
typedef struct tfb_soundsource {
    TFB_SoundSample *sample;            // currently assigned sample (NULL = inactive)
    audio_Object handle;                // audio backend source handle
    bool stream_should_be_playing;      // flag checked by decoder thread
    Mutex stream_mutex;                 // per-source mutex for thread safety
    sint32 start_time;                  // playback start timestamp (for position math)
    uint32 pause_time;                  // when paused (0 = not paused)
    void *positional_object;            // game object for positional audio

    audio_Object last_q_buf;            // last queued buffer (for callback context)

    // Oscilloscope ring buffer
    void *sbuffer;                      // cyclic waveform data
    uint32 sbuf_size;                   // total ring buffer size
    uint32 sbuf_tail;                   // write pointer
    uint32 sbuf_head;                   // read pointer
    uint32 sbuf_lasttime;               // timestamp of first queued buffer
} TFB_SoundSource;
```

`soundSource` is a global array of `NUM_SOUNDSOURCES = 7` entries:
- Indices 0–4: SFX channels
- Index 5: `MUSIC_SOURCE`
- Index 6: `SPEECH_SOURCE`

#### 8.3 `TFB_SoundTag` (sound.h:33–38)

```c
typedef struct {
    int in_use;              // 0 = available, 1 = active
    audio_Object buf_name;   // which audio buffer this tags
    intptr_t data;           // user payload (used for TFB_SoundChunk*)
} TFB_SoundTag;
```

#### 8.4 `TFB_SoundCallbacks` (sound.h:40–52)

```c
typedef struct tfb_soundcallbacks {
    bool (*OnStartStream)(TFB_SoundSample*);           // before initial buffering
    bool (*OnEndChunk)(TFB_SoundSample*, audio_Object); // decoder EOF
    void (*OnEndStream)(TFB_SoundSample*);              // all playback done
    void (*OnTaggedBuffer)(TFB_SoundSample*, TFB_SoundTag*); // tagged buffer played
    void (*OnQueueBuffer)(TFB_SoundSample*, audio_Object);   // buffer just queued
} TFB_SoundCallbacks;
```

#### 8.5 `TFB_SoundChunk` (trackint.h:22–31)

```c
struct tfb_soundchunk {
    TFB_SoundDecoder *decoder;
    float start_time;
    int tag_me;
    uint32 track_num;
    UNICODE *text;
    CallbackFunction callback;
    struct tfb_soundchunk *next;
};
```

#### 8.6 Source Index Constants (sound.h:27–31)

```c
#define FIRST_SFX_SOURCE 0
#define LAST_SFX_SOURCE  (FIRST_SFX_SOURCE + NUM_SFX_CHANNELS - 1)  // = 4
#define MUSIC_SOURCE     (LAST_SFX_SOURCE + 1)                       // = 5
#define SPEECH_SOURCE    (MUSIC_SOURCE + 1)                           // = 6
#define NUM_SOUNDSOURCES (SPEECH_SOURCE + 1)                          // = 7
```

### 9. Thread Safety Model

#### 9.1 Mutexes

Each `TFB_SoundSource` has its own `stream_mutex`. This mutex is used:

- **By `StreamDecoderTaskFunc`** (decoder thread): Locked around each call to `process_stream(source)` (stream.c:552–566)
- **By `PlayStream`**, **`StopStream`**, **`PauseStream`**, **`ResumeStream`**, **`SeekStream`**: Called from the main/DoInput thread — callers are expected to lock the appropriate mutex. The stream functions themselves do NOT lock.
- **By `PLRPlaySong`**, **`PLRStop`**, **`PLRSeek`**, **`PLRPause`**, **`PLRResume`** (music.c): Lock `soundSource[MUSIC_SOURCE].stream_mutex`
- **By `snd_PlaySpeech`**, **`snd_StopSpeech`** (music.c): Lock `soundSource[SPEECH_SOURCE].stream_mutex`
- **By `PlayTrack`**, **`StopTrack`**, **`JumpTrack`**, **`PauseTrack`**, **`ResumeTrack`**, **`FastReverse_*`**, **`FastForward_*`**, **`PlayingTrack`**, **`GetTrackPosition`**, **`GetTrackSubtitle`** (trackplayer.c): Lock `soundSource[SPEECH_SOURCE].stream_mutex`

A separate `fade_mutex` (stream.c:37) protects the four fade variables, locked by both `processMusicFade()` (decoder thread) and `SetMusicStreamFade()` (main thread).

#### 9.2 Volatile Flags

- `tracks_length` is declared `volatile uint32` (trackplayer.c:39) — read without mutex by `GetTrackPosition()` which detaches it into a local variable to avoid divide-by-zero races
- `stream_should_be_playing` (bool in `TFB_SoundSource`) is read/written from both threads but always under `stream_mutex`

#### 9.3 Thread Ownership Rules

From the comment at trackplayer.c:48–55:
> Accesses to `cur_chunk` and `cur_sub_chunk` are guarded by `stream_mutex`. Other data structures are unguarded and should only be accessed from the DoInput thread at certain times (nothing modified between `StartTrack()` and `JumpTrack()`/`StopTrack()` calls).

The implied contract:
- **Decoder thread** may access: `source->sample`, `source->sample->decoder`, `source->stream_should_be_playing`, scope buffers, and call callbacks — all under `stream_mutex`
- **Main/DoInput thread** may call stream control functions — must lock `stream_mutex` first
- **Chunk list manipulation** (`SpliceTrack`, `SpliceMultiTrack`, `StopTrack` destroying chunks) — only on the DoInput thread, never while stream is being processed
- **SFX channels** have no threading concerns — `PlayChannel` / `StopChannel` are main-thread only, and SFX sources are never touched by the decoder thread (it only iterates `MUSIC_SOURCE` to `NUM_SOUNDSOURCES-1`)

### 10. Integration Points with Existing Rust Code

#### 10.1 Rust Mixer (already ported)

The Rust mixer (`rust/src/sound/mixer/`) implements the OpenAL-like API:
- `rust_mixer_GenSources`, `rust_mixer_GenBuffers`
- `rust_mixer_SourcePlay`, `rust_mixer_SourceStop`, `rust_mixer_SourcePause`
- `rust_mixer_SourceQueueBuffers`, `rust_mixer_SourceUnqueueBuffers`
- `rust_mixer_BufferData`, `rust_mixer_GetBufferi`, `rust_mixer_GetSourcei`

The C `audiocore.h` API is #define-redirected to these Rust functions via `rust_audiocore.h` and `rust_mixer.h` when `USE_RUST_AUDIO` / `USE_RUST_MIXER` are defined.

**Impact on porting:** The stream.c / sfx.c / music.c code calls `audio_*` functions extensively. When ported to Rust, these should call the Rust mixer directly without FFI overhead — the mixer types (`mixer_Object`, etc.) can be used natively.

#### 10.2 Rust Decoders (already ported)

All four decoder types are ported:
- `rust/src/sound/wav.rs` — WAV decoder
- `rust/src/sound/ogg.rs` — OGG Vorbis decoder
- `rust/src/sound/mod_decoder.rs` — MOD/tracker decoder
- `rust/src/sound/dukaud.rs` — DukAud (Duke Nukem-style) decoder
- `rust/src/sound/null.rs` — Null decoder (for subtitle-only tracks)

Each has FFI wrappers (`wav_ffi.rs`, `mod_ffi.rs`, `dukaud_ffi.rs`, `ffi.rs`) exposing C-compatible vtables matching the `TFB_SoundDecoderFuncs` structure.

**Impact on porting:** The ported stream code can call Rust decoder methods directly (no FFI), using the `SoundDecoder` trait. Key methods needed:
- `decode()` → `SoundDecoder_Decode`
- `decode_all()` → `SoundDecoder_DecodeAll`
- `seek()` → `SoundDecoder_Seek`
- `rewind()` → `SoundDecoder_Rewind`
- `get_time()` → `SoundDecoder_GetTime`

#### 10.3 Rust Rodio Backend (already ported)

`rust/src/sound/rodio_audio.rs` and `rodio_backend.rs` provide higher-level audio through the `rodio` crate. These are an alternative playback path but the stream.c architecture uses the lower-level mixer API with buffer queuing.

#### 10.4 Boundary: What Calls In From C

These C functions are called from game logic (comm screens, menus, combat, etc.) that is still in C:
- `PLRPlaySong`, `PLRStop`, `PLRPlaying`, `PLRSeek`, `PLRPause`, `PLRResume`
- `snd_PlaySpeech`, `snd_StopSpeech`
- `PlayTrack`, `StopTrack`, `JumpTrack`, `PauseTrack`, `ResumeTrack`, `PlayingTrack`
- `SpliceTrack`, `SpliceMultiTrack`
- `FastReverse_Smooth`, `FastForward_Smooth`, `FastReverse_Page`, `FastForward_Page`
- `GetTrackPosition`, `GetTrackSubtitle`, `GetFirstTrackSubtitle`, `GetNextTrackSubtitle`, `GetTrackSubtitleText`
- `PlayChannel`, `StopChannel`, `ChannelPlaying`, `SetChannelVolume`
- `UpdateSoundPosition`, `GetPositionalObject`, `SetPositionalObject`
- `LoadSoundFile`, `LoadMusicFile`, `DestroySound`, `DestroyMusic`
- `SetMusicVolume`, `SetSFXVolume`, `SetSpeechVolume`
- `FadeMusic`, `GraphForegroundStream`
- `StopSound`, `SoundPlaying`, `WaitForSoundEnd`
- `InitSound`, `UninitSound`, `InitStreamDecoder`, `UninitStreamDecoder`

All of these will need `#[no_mangle] pub extern "C"` FFI exports.

---

## Part 2: EARS Requirements

### STREAM — Streaming Audio Engine (`stream.c`)

#### STREAM-INIT: Initialization and Shutdown

**STREAM-INIT-01:** The streaming system shall create a dedicated mutex for music fade state protection upon initialization.
*(stream.c:788, `CreateMutex("Stream fade mutex", SYNC_CLASS_AUDIO)`)*

**STREAM-INIT-02:** The streaming system shall spawn a background decoder task thread with a 1024-byte stack upon initialization.
*(stream.c:792–793, `AssignTask(StreamDecoderTaskFunc, 1024, "audio stream decoder")`)*

**STREAM-INIT-03:** The streaming system shall return -1 if either the fade mutex or the decoder task fails to initialize.
*(stream.c:789–796)*

**STREAM-INIT-04:** When the streaming system is uninitialized, the system shall signal the decoder task to exit and wait for it to complete.
*(stream.c:803–807, `ConcludeTask(decoderTask)`)*

**STREAM-INIT-05:** When the streaming system is uninitialized, the system shall destroy the fade mutex.
*(stream.c:809–813)*

**STREAM-INIT-06:** When the decoder task is NULL during uninitialization, the system shall skip task termination without error.
*(stream.c:803)*

**STREAM-INIT-07:** When the fade mutex is NULL during uninitialization, the system shall skip mutex destruction without error.
*(stream.c:809)*

#### STREAM-PLAY: Stream Playback Control

**STREAM-PLAY-01:** When `PlayStream` is called, the system shall first stop any existing stream on the given source.
*(stream.c:53, `StopStream(source)`)*

**STREAM-PLAY-02:** When `PlayStream` is called with a NULL sample, the system shall return immediately without action.
*(stream.c:50–51)*

**STREAM-PLAY-03:** When `PlayStream` is called and the `OnStartStream` callback exists and returns false, the system shall abort stream start.
*(stream.c:54–56)*

**STREAM-PLAY-04:** When `PlayStream` is called, the system shall clear all buffer tags by zeroing the `buffer_tag` array.
*(stream.c:58–61)*

**STREAM-PLAY-05:** When `PlayStream` is called with `rewind=true`, the system shall rewind the decoder to its start position.
*(stream.c:64–65)*

**STREAM-PLAY-06:** When `PlayStream` is called with `rewind=false`, the system shall compute the time offset as `sample->offset + SoundDecoder_GetTime(decoder) * ONE_SECOND`.
*(stream.c:66–67)*

**STREAM-PLAY-07:** The system shall set the source's sample pointer and configure the decoder's `looping` flag, while always setting the audio source's `audio_LOOPING` to false.
*(stream.c:69–71)*

**STREAM-PLAY-08:** When `PlayStream` is called with `scope=true`, the system shall pre-allocate a scope buffer of size `num_buffers * buffer_size + PAD_SCOPE_BYTES` bytes.
*(stream.c:73–79, `PAD_SCOPE_BYTES = 256`)*

**STREAM-PLAY-09:** The system shall pre-fill up to `num_buffers` audio buffers by decoding audio data, uploading each to the audio backend via `audio_BufferData`, and queuing each via `audio_SourceQueueBuffers`.
*(stream.c:81–118)*

**STREAM-PLAY-10:** When a buffer is queued and the `OnQueueBuffer` callback exists, the system shall invoke it with the sample and buffer handle.
*(stream.c:99–100)*

**STREAM-PLAY-11:** When the decoder returns an error during pre-filling and the error is `SOUNDDECODER_EOF`, the system shall invoke the `OnEndChunk` callback. Where the callback returns true, the system shall continue with the new decoder from `sample->decoder`.
*(stream.c:106–118)*

**STREAM-PLAY-12:** When decoder returns 0 decoded bytes during pre-filling, the system shall stop pre-filling.
*(stream.c:92–93)*

**STREAM-PLAY-13:** After pre-filling, the system shall record the scope buffer last-time, compute `start_time = GetTimeCounter() - offset`, set `pause_time = 0`, set `stream_should_be_playing = TRUE`, and call `audio_SourcePlay`.
*(stream.c:121–128)*

**STREAM-PLAY-14:** When `StopStream` is called, the system shall call `StopSource`, set `stream_should_be_playing = FALSE`, set `sample = NULL`, free the scope buffer (if any), and zero all scope-related fields and `pause_time`.
*(stream.c:130–148)*

**STREAM-PLAY-15:** When `PauseStream` is called, the system shall set `stream_should_be_playing = FALSE`, record the current time in `pause_time` (only if not already paused), and call `audio_SourcePause`.
*(stream.c:151–157)*

**STREAM-PLAY-16:** When `ResumeStream` is called and `pause_time` is nonzero, the system shall adjust `start_time` by adding `GetTimeCounter() - pause_time` to account for the pause duration.
*(stream.c:162–167)*

**STREAM-PLAY-17:** When `ResumeStream` is called, the system shall set `pause_time = 0`, set `stream_should_be_playing = TRUE`, and call `audio_SourcePlay`.
*(stream.c:168–171)*

**STREAM-PLAY-18:** When `SeekStream` is called, the system shall stop the source, seek the decoder to `pos` milliseconds, and restart playback via `PlayStream` with the same looping and scope settings.
*(stream.c:173–188)*

**STREAM-PLAY-19:** When `SeekStream` is called with a NULL sample, the system shall return immediately without action.
*(stream.c:180–181)*

**STREAM-PLAY-20:** The `PlayingStream` function shall return the value of `soundSource[source].stream_should_be_playing`.
*(stream.c:190–194)*

#### STREAM-THREAD: Background Decoder Thread

**STREAM-THREAD-01:** While the decoder task has not received the `TASK_EXIT` signal, the decoder thread shall continuously loop.
*(stream.c:542)*

**STREAM-THREAD-02:** On each iteration, the decoder thread shall call `processMusicFade()` to advance any active music fade.
*(stream.c:546)*

**STREAM-THREAD-03:** On each iteration, the decoder thread shall iterate over sources from `MUSIC_SOURCE` to `NUM_SOUNDSOURCES - 1` (inclusive).
*(stream.c:548)*

**STREAM-THREAD-04:** For each source, the decoder thread shall lock `source->stream_mutex` before checking or modifying any source state.
*(stream.c:552)*

**STREAM-THREAD-05:** Where a source has no sample, no decoder, `stream_should_be_playing` is false, or the decoder has `SOUNDDECODER_ERROR`, the decoder thread shall skip that source.
*(stream.c:554–561)*

**STREAM-THREAD-06:** Where no active streams exist in an iteration, the decoder thread shall sleep for `ONE_SECOND / 10` (100ms at 840 ticks/sec).
*(stream.c:569–571)*

**STREAM-THREAD-07:** Where at least one active stream exists, the decoder thread shall yield via `TaskSwitch()` rather than sleeping.
*(stream.c:573–574)*

**STREAM-THREAD-08:** When the decoder task exits its loop, the system shall call `FinishTask(task)` and return 0.
*(stream.c:577–578)*

#### STREAM-PROCESS: Per-Source Stream Processing

**STREAM-PROCESS-01:** The system shall query `audio_BUFFERS_PROCESSED` and `audio_BUFFERS_QUEUED` from the audio backend for the source.
*(stream.c:362–363)*

**STREAM-PROCESS-02:** When `processed == 0` and the audio source state is not `audio_PLAYING`: where `queued == 0` and `decoder->error == SOUNDDECODER_EOF`, the system shall set `stream_should_be_playing = FALSE` and call the `OnEndStream` callback if it exists.
*(stream.c:365–380)*

**STREAM-PROCESS-03:** When `processed == 0` and the audio source is not playing but has queued buffers, the system shall log a buffer underrun warning and call `audio_SourcePlay` to restart playback.
*(stream.c:381–386)*

**STREAM-PROCESS-04:** For each processed buffer, the system shall unqueue it from the audio source via `audio_SourceUnqueueBuffers`.
*(stream.c:400)*

**STREAM-PROCESS-05:** When `audio_SourceUnqueueBuffers` returns an error, the system shall log the error and break out of the processing loop.
*(stream.c:401–408)*

**STREAM-PROCESS-06:** When an `OnTaggedBuffer` callback exists and the unqueued buffer has a tag (found via `TFB_FindTaggedBuffer`), the system shall invoke the callback with the sample and tag.
*(stream.c:411–416)*

**STREAM-PROCESS-07:** When the source has a scope buffer, the system shall call `remove_scope_data` for each unqueued buffer.
*(stream.c:418–419)*

**STREAM-PROCESS-08:** When the decoder's error state is `SOUNDDECODER_EOF` and no `OnEndChunk` callback exists or it returns false, the system shall set `end_chunk_failed = true` and skip further EOF handling in this iteration.
*(stream.c:422–435)*

**STREAM-PROCESS-09:** When the decoder's error state is `SOUNDDECODER_EOF` and the `OnEndChunk` callback returns true, the system shall re-read `sample->decoder` to get the new decoder.
*(stream.c:436–441)*

**STREAM-PROCESS-10:** When the decoder has a non-EOF error, the system shall skip that buffer without attempting to decode.
*(stream.c:443–451)*

**STREAM-PROCESS-11:** The system shall decode new audio via `SoundDecoder_Decode(decoder)` for each recycled buffer.
*(stream.c:455)*

**STREAM-PROCESS-12:** When `SoundDecoder_Decode` returns `SOUNDDECODER_ERROR`, the system shall log the error, set `stream_should_be_playing = FALSE`, and skip the buffer.
*(stream.c:456–463)*

**STREAM-PROCESS-13:** When decoded_bytes is 0, the system shall skip the buffer (and the buffer is permanently lost from the queue).
*(stream.c:465–470)*

**STREAM-PROCESS-14:** The system shall upload decoded data via `audio_BufferData` and queue the buffer via `audio_SourceQueueBuffers`, checking for errors after each call.
*(stream.c:473–493)*

**STREAM-PROCESS-15:** The system shall record the last queued buffer handle as `source->last_q_buf` and invoke the `OnQueueBuffer` callback if it exists.
*(stream.c:496–498)*

**STREAM-PROCESS-16:** When the source has a scope buffer, the system shall call `add_scope_data` with the newly decoded byte count after queueing.
*(stream.c:500–501)*

#### STREAM-SAMPLE: Sound Sample Management

**STREAM-SAMPLE-01:** `TFB_CreateSoundSample` shall allocate a `TFB_SoundSample` structure, set its decoder and `num_buffers`, allocate an array of `num_buffers` audio objects, generate audio buffers via `audio_GenBuffers`, and copy callbacks if provided.
*(stream.c:197–212)*

**STREAM-SAMPLE-02:** `TFB_DestroySoundSample` shall delete audio buffers via `audio_DeleteBuffers`, free the buffer array, free the buffer_tag array, and free the sample structure. The decoder is NOT freed.
*(stream.c:215–225)*

**STREAM-SAMPLE-03:** `TFB_SetSoundSampleData` shall store user-defined data in `sample->data`, and `TFB_GetSoundSampleData` shall retrieve it.
*(stream.c:227–237)*

**STREAM-SAMPLE-04:** `TFB_SetSoundSampleCallbacks` shall copy the provided callbacks, or zero the callbacks structure if NULL is passed.
*(stream.c:239–247)*

**STREAM-SAMPLE-05:** `TFB_GetSoundSampleDecoder` shall return `sample->decoder`.
*(stream.c:249–253)*

#### STREAM-TAG: Buffer Tagging

**STREAM-TAG-01:** `TFB_FindTaggedBuffer` shall search the `buffer_tag` array for a tag where `in_use == 1` and `buf_name == buffer`. Where no tags are allocated, it shall return NULL. Where no matching tag is found, it shall return NULL.
*(stream.c:255–273)*

**STREAM-TAG-02:** `TFB_TagBuffer` shall find an unused slot or a slot matching the same `buffer` in the `buffer_tag` array. Where no `buffer_tag` array exists, it shall allocate one. Where no slot is available, it shall return false. Otherwise it shall set `in_use = 1`, `buf_name = buffer`, `data = data`, and return true.
*(stream.c:275–299)*

**STREAM-TAG-03:** `TFB_ClearBufferTag` shall set `in_use = 0` and `buf_name = 0` on the given tag.
*(stream.c:301–306)*

#### STREAM-SCOPE: Oscilloscope Ring Buffer

**STREAM-SCOPE-01:** `add_scope_data` shall copy decoded bytes from the decoder buffer to the scope ring buffer's tail position, wrapping around to the beginning when the tail reaches the end.
*(stream.c:321–351)*

**STREAM-SCOPE-02:** `remove_scope_data` shall advance the scope ring buffer's head position by the byte size of the given audio buffer (queried via `audio_GetBufferi(buffer, audio_SIZE)`), wrapping modulo `sbuf_size`. It shall update `sbuf_lasttime` to the current time.
*(stream.c:308–319)*

**STREAM-SCOPE-03:** `GraphForegroundStream` shall prefer the speech source when `wantSpeech` is true and a non-null decoder is available on the speech source. Otherwise it shall fall back to the music source.
*(stream.c:634–657)*

**STREAM-SCOPE-04:** When no playable stream, sample, decoder, scope buffer, or scope size is available, `GraphForegroundStream` shall return 0.
*(stream.c:659–665)*

**STREAM-SCOPE-05:** `GraphForegroundStream` shall compute the step size normalized to 11025 Hz: step 1 for speech, step 4 for music, scaled by `decoder->frequency / 11025`, minimum 1, multiplied by `full_sample` (bytes per full sample across all channels).
*(stream.c:644, 656, 695–698)*

**STREAM-SCOPE-06:** `GraphForegroundStream` shall compute the current read position as `sbuf_head + delta` where delta is derived from `(GetTimeCounter() - sbuf_lasttime) * frequency * full_sample / ONE_SECOND`, clamped to `[0, sbuf_size]`.
*(stream.c:678–692)*

**STREAM-SCOPE-07:** `GraphForegroundStream` shall read samples from the scope buffer, scaling them by `avg_amp / target_amp` (where `target_amp = height/4`), centering at `height/2`, clamping to `[0, height-1]`, and writing results to the output `data` array.
*(stream.c:705–733)*

**STREAM-SCOPE-08:** The `readSoundSample` helper shall convert 8-bit unsigned samples to signed 16-bit range by subtracting 128 and shifting left by 8. For 16-bit samples it shall return the value directly.
*(stream.c:581–588)*

**STREAM-SCOPE-09:** `GraphForegroundStream` shall implement Automatic Gain Control (AGC) using a 16-page running average of 8-frame maximum amplitude pages, with a default page maximum of 28000.
*(stream.c:613–755)*

**STREAM-SCOPE-10:** The AGC shall include Voice Activity Detection: when the per-frame signal energy is below `VAD_MIN_ENERGY = 100`, the frame shall not be counted toward the running average.
*(stream.c:737)*

**STREAM-SCOPE-11:** When multi-channel audio is detected (channels > 1), `GraphForegroundStream` shall sum both channel samples for each step position.
*(stream.c:718–719)*

#### STREAM-FADE: Music Fade

**STREAM-FADE-01:** `SetMusicStreamFade` shall lock the fade mutex, record `musicFadeStartTime` as the current time, `musicFadeInterval` as `howLong` (clamped to ≥0), `musicFadeStartVolume` as the current `musicVolume`, and `musicFadeDelta` as `endVolume - musicVolume`.
*(stream.c:763–783)*

**STREAM-FADE-02:** When `howLong` is 0 after clamping, `SetMusicStreamFade` shall return false (reject the fade).
*(stream.c:777–778)*

**STREAM-FADE-03:** While a fade is active (`musicFadeInterval != 0`), `processMusicFade` shall compute elapsed time clamped to `[0, musicFadeInterval]`, linearly interpolate the volume as `startVolume + delta * elapsed / interval`, and call `SetMusicVolume` with the result.
*(stream.c:505–533)*

**STREAM-FADE-04:** When the elapsed time reaches or exceeds `musicFadeInterval`, `processMusicFade` shall set `musicFadeInterval = 0` to end the fade.
*(stream.c:529–530)*

**STREAM-FADE-05:** When no fade is active (`musicFadeInterval == 0`), `processMusicFade` shall return immediately.
*(stream.c:514–518)*

---

### TRACK — Track Player (`trackplayer.c`)

#### TRACK-ASSEMBLE: Track Assembly

**TRACK-ASSEMBLE-01:** `SpliceTrack` shall split subtitle text into pages at `\r\n` boundaries using `SplitSubPages`.
*(trackplayer.c:458, 513)*

**TRACK-ASSEMBLE-02:** `SplitSubPages` shall calculate a display timestamp for each page as `character_count * TEXT_SPEED` where `TEXT_SPEED = 80` ms/char, with a minimum of 1000ms.
*(trackplayer.c:346–348)*

**TRACK-ASSEMBLE-03:** `SplitSubPages` shall prepend `..` to continuation pages (pages following a mid-word break) and append `...` to pages that end at a mid-word break (detected by checking that the character before the newline is not punctuation or whitespace).
*(trackplayer.c:336–349)*

**TRACK-ASSEMBLE-04:** When `SpliceTrack` is called with a NULL `TrackName`, the system shall append subtitle text to the last existing track's subtitle. The first page shall be concatenated to `last_sub->text`.
*(trackplayer.c:441–505)*

**TRACK-ASSEMBLE-05:** When `SpliceTrack` is called with a NULL `TrackName` and `track_count` is 0, the system shall log a warning and return.
*(trackplayer.c:444–449)*

**TRACK-ASSEMBLE-06:** When `SpliceTrack` is called with a NULL `TrackText`, the system shall return immediately.
*(trackplayer.c:437–438)*

**TRACK-ASSEMBLE-07:** When `SpliceTrack` is called with a non-NULL `TrackName`, the system shall create a new track with a new decoder loaded from the content directory. The first call shall also create the `sound_sample` with 8 buffers and track player callbacks.
*(trackplayer.c:507–602)*

**TRACK-ASSEMBLE-08:** `SpliceTrack` shall load decoder segments via `SoundDecoder_Load(contentDir, TrackName, 4096, dec_offset, time_stamps[page])` where `dec_offset` accumulates as `decoder->length * 1000` ms for each segment.
*(trackplayer.c:561–562, 580)*

**TRACK-ASSEMBLE-09:** When timestamps are provided via the `TimeStamp` parameter, `SpliceTrack` shall parse them via `GetTimeStamps` (comma/newline separated unsigned integers) and use them instead of calculated page timestamps.
*(trackplayer.c:538–551)*

**TRACK-ASSEMBLE-10:** `SpliceTrack` shall always make the last page's timestamp negative, indicating it is a suggested minimum rather than a hard cutoff.
*(trackplayer.c:466, 521)*

**TRACK-ASSEMBLE-11:** When `no_page_break` is set and tracks exist, `SpliceTrack` shall concatenate the first page to the last subtitle instead of creating a new track.
*(trackplayer.c:523–532)*

**TRACK-ASSEMBLE-12:** Each chunk created by `SpliceTrack` shall have `tag_me = 1` (unless `no_page_break`), its subtitle text, callback function, and appropriate `track_num`.
*(trackplayer.c:588–599)*

**TRACK-ASSEMBLE-13:** `SpliceTrack` shall reset `no_page_break = 0` after processing each chunk.
*(trackplayer.c:600)*

**TRACK-ASSEMBLE-14:** `GetTimeStamps` shall parse a string of comma/CR/LF-separated unsigned integers, skipping zero values, and return the count of parsed timestamps.
*(trackplayer.c:293–317)*

**TRACK-ASSEMBLE-15:** `SpliceMultiTrack` shall load up to `MAX_MULTI_TRACKS = 20` decoders with buffer size 32768 and runTime `-3 * TEXT_SPEED`, fully pre-decode each via `SoundDecoder_DecodeAll`, and append each as a new chunk sharing the current track number.
*(trackplayer.c:363–423)*

**TRACK-ASSEMBLE-16:** `SpliceMultiTrack` shall concatenate `TrackText` to `last_sub->text` and set `no_page_break = 1`.
*(trackplayer.c:417–422)*

**TRACK-ASSEMBLE-17:** When `SpliceMultiTrack` is called before any `SpliceTrack`, the system shall log a warning and return.
*(trackplayer.c:377–381)*

**TRACK-ASSEMBLE-18:** `create_SoundChunk` shall allocate a zeroed `TFB_SoundChunk`, set its decoder and start_time.
*(trackplayer.c:783–791)*

**TRACK-ASSEMBLE-19:** `destroy_SoundChunk_list` shall walk the linked list, freeing each chunk's decoder (via `SoundDecoder_Free`), subtitle text, and the chunk structure itself.
*(trackplayer.c:793–805)*

#### TRACK-PLAY: Track Playback Control

**TRACK-PLAY-01:** `PlayTrack` shall compute `tracks_length` as the end time of the last chunk, set `cur_chunk = chunks_head`, and call `PlayStream(sound_sample, SPEECH_SOURCE, false, true, true)` (no looping, with scope, with rewind).
*(trackplayer.c:103–116)*

**TRACK-PLAY-02:** When `PlayTrack` is called and `sound_sample` is NULL, the system shall return immediately.
*(trackplayer.c:106–107)*

**TRACK-PLAY-03:** `StopTrack` shall lock `SPEECH_SOURCE`'s stream mutex, stop the stream, reset `track_count`, `tracks_length`, `cur_chunk`, and `cur_sub_chunk` to zero/NULL.
*(trackplayer.c:172–180)*

**TRACK-PLAY-04:** `StopTrack` shall destroy the entire chunk list (all decoders and subtitle text), set `chunks_head`, `chunks_tail`, and `last_sub` to NULL.
*(trackplayer.c:182–188)*

**TRACK-PLAY-05:** `StopTrack` shall set `sound_sample->decoder = NULL` before destroying the sample to prevent double-free (decoders are owned by chunks).
*(trackplayer.c:192)*

**TRACK-PLAY-06:** `JumpTrack` shall lock `SPEECH_SOURCE`'s stream mutex and call `seek_track(tracks_length + 1)` to advance past the end of all tracks.
*(trackplayer.c:92–100)*

**TRACK-PLAY-07:** When `JumpTrack` is called and `sound_sample` is NULL, the system shall return immediately.
*(trackplayer.c:94–95)*

**TRACK-PLAY-08:** `PauseTrack` shall lock `SPEECH_SOURCE`'s stream mutex and call `PauseStream(SPEECH_SOURCE)`.
*(trackplayer.c:118–127)*

**TRACK-PLAY-09:** `ResumeTrack` shall lock `SPEECH_SOURCE`'s stream mutex, check that `cur_chunk` is not NULL and the audio source state is `audio_PAUSED`, then call `ResumeStream(SPEECH_SOURCE)`.
*(trackplayer.c:129–151)*

**TRACK-PLAY-10:** `PlayingTrack` shall return `cur_chunk->track_num + 1` (1-indexed) under the stream mutex. Where `cur_chunk` is NULL or `sound_sample` is NULL, it shall return 0.
*(trackplayer.c:153–169)*

#### TRACK-SEEK: Seeking and Navigation

**TRACK-SEEK-01:** `seek_track` shall clamp the offset to `[0, tracks_length + 1]`.
*(trackplayer.c:621–624)*

**TRACK-SEEK-02:** `seek_track` shall adjust the speech source's `start_time` to `GetTimeCounter() - offset`.
*(trackplayer.c:628)*

**TRACK-SEEK-03:** `seek_track` shall walk the chunk list from head to find the chunk whose end time exceeds `offset`, tracking the last tagged chunk encountered.
*(trackplayer.c:631–640)*

**TRACK-SEEK-04:** When a chunk is found at the seek position, `seek_track` shall seek the chunk's decoder to the correct position within the chunk (in milliseconds), set `sound_sample->decoder` to that chunk's decoder, and call `DoTrackTag` on the last tagged chunk.
*(trackplayer.c:642–656)*

**TRACK-SEEK-05:** When the offset exceeds all chunks, `seek_track` shall stop the speech stream and set `cur_chunk` and `cur_sub_chunk` to NULL.
*(trackplayer.c:658–662)*

**TRACK-SEEK-06:** `get_current_track_pos` shall return `GetTimeCounter() - start_time`, clamped to `[0, tracks_length]`.
*(trackplayer.c:665–678)*

**TRACK-SEEK-07:** `FastReverse_Smooth` shall subtract `ACCEL_SCROLL_SPEED = 300` from the current position and seek to the result. Where the stream was not playing, it shall restart playback.
*(trackplayer.c:680–698)*

**TRACK-SEEK-08:** `FastForward_Smooth` shall add `ACCEL_SCROLL_SPEED = 300` to the current position and seek to the result.
*(trackplayer.c:700–716)*

**TRACK-SEEK-09:** `FastReverse_Page` shall find the previous page via `find_prev_page(cur_sub_chunk)` and restart playback from that chunk. Where no previous page exists, it shall do nothing.
*(trackplayer.c:718–736)*

**TRACK-SEEK-10:** `FastForward_Page` shall find the next page via `find_next_page(cur_sub_chunk)` and restart playback from that chunk. Where no next page exists, it shall seek past the end of all tracks.
*(trackplayer.c:738–760)*

**TRACK-SEEK-11:** `find_next_page` shall return the next chunk in the list with `tag_me` set. Where the input is NULL or no next tagged chunk exists, it shall return NULL.
*(trackplayer.c:808–816)*

**TRACK-SEEK-12:** `find_prev_page` shall return the last chunk before `cur` with `tag_me` set, defaulting to `chunks_head`. Where `cur == chunks_head`, it shall return `chunks_head`.
*(trackplayer.c:820–835)*

**TRACK-SEEK-13:** All seek and navigation functions shall lock `SPEECH_SOURCE`'s stream mutex before modifying shared state.
*(trackplayer.c:688, 708, 726, 746)*

#### TRACK-CALLBACK: Stream Callbacks

**TRACK-CALLBACK-01:** `OnStreamStart` shall verify the sample matches `sound_sample` and that `cur_chunk` is not NULL. Where either check fails, it shall return false.
*(trackplayer.c:215–219)*

**TRACK-CALLBACK-02:** `OnStreamStart` shall set `sample->decoder = cur_chunk->decoder` and `sample->offset = cur_chunk->start_time * ONE_SECOND`.
*(trackplayer.c:222–223)*

**TRACK-CALLBACK-03:** When `cur_chunk->tag_me` is set in `OnStreamStart`, the system shall call `DoTrackTag(cur_chunk)`.
*(trackplayer.c:225–226)*

**TRACK-CALLBACK-04:** `OnChunkEnd` shall return false when the sample doesn't match or when `cur_chunk` is NULL or has no `next` pointer (all chunks done).
*(trackplayer.c:237–243)*

**TRACK-CALLBACK-05:** `OnChunkEnd` shall advance `cur_chunk` to `cur_chunk->next`, set `sample->decoder` to the new chunk's decoder, and rewind the new decoder.
*(trackplayer.c:246–249)*

**TRACK-CALLBACK-06:** When the new chunk in `OnChunkEnd` has `tag_me` set, the system shall tag the buffer with the chunk pointer using `TFB_TagBuffer(sample, buffer, (intptr_t)cur_chunk)`.
*(trackplayer.c:254–257)*

**TRACK-CALLBACK-07:** `OnStreamEnd` shall set `cur_chunk = NULL` and `cur_sub_chunk = NULL` when the stream finishes.
*(trackplayer.c:263–272)*

**TRACK-CALLBACK-08:** `OnBufferTag` shall extract the `TFB_SoundChunk*` from `tag->data`, clear the buffer tag, and call `DoTrackTag` on the extracted chunk.
*(trackplayer.c:277–289)*

**TRACK-CALLBACK-09:** `DoTrackTag` shall lock the speech stream mutex, call `chunk->callback(0)` if a callback exists, and set `cur_sub_chunk = chunk`.
*(trackplayer.c:198–207)*

#### TRACK-SUBTITLE: Subtitle Access

**TRACK-SUBTITLE-01:** `GetTrackSubtitle` shall lock the speech stream mutex and return `cur_sub_chunk->text`. Where `sound_sample` is NULL or `cur_sub_chunk` is NULL, it shall return NULL.
*(trackplayer.c:867–881)*

**TRACK-SUBTITLE-02:** `GetFirstTrackSubtitle` shall return `chunks_head`.
*(trackplayer.c:839–843)*

**TRACK-SUBTITLE-03:** `GetNextTrackSubtitle` shall return `find_next_page(LastRef)`. Where `LastRef` is NULL, it shall return NULL.
*(trackplayer.c:846–853)*

**TRACK-SUBTITLE-04:** `GetTrackSubtitleText` shall return `SubRef->text`. Where `SubRef` is NULL, it shall return NULL.
*(trackplayer.c:856–863)*

#### TRACK-POSITION: Position Tracking

**TRACK-POSITION-01:** `GetTrackPosition` shall return the current playback position scaled to `in_units`, computed as `in_units * offset / tracks_length`. Where `sound_sample` is NULL or `tracks_length` is 0, it shall return 0.
*(trackplayer.c:765–781)*

**TRACK-POSITION-02:** `GetTrackPosition` shall copy `tracks_length` to a local variable before use to avoid division-by-zero from concurrent modification.
*(trackplayer.c:769–771)*

---

### MUSIC — Music Playback API (`music.c`)

#### MUSIC-PLAY: Playback Control

**MUSIC-PLAY-01:** `PLRPlaySong` shall dereference `MusicRef` to get the `TFB_SoundSample*`, lock the music source mutex, call `PlayStream` with the sample on `MUSIC_SOURCE` with looping as `Continuous`, scope always true, and rewind always true, then unlock and store `curMusicRef`.
*(music.c:31–46)*

**MUSIC-PLAY-02:** When `PLRPlaySong` is called with a NULL `MusicRef`, the system shall not attempt playback.
*(music.c:35)*

**MUSIC-PLAY-03:** The `Priority` parameter of `PLRPlaySong` shall be accepted but ignored.
*(music.c:45)*

**MUSIC-PLAY-04:** `PLRStop` shall stop the music stream and clear `curMusicRef` when the provided `MusicRef` matches `curMusicRef` or is `(MUSIC_REF)~0` (wildcard).
*(music.c:48–59)*

**MUSIC-PLAY-05:** `PLRPlaying` shall return TRUE when `curMusicRef` is set and `MusicRef` matches (or is wildcard) and `PlayingStream(MUSIC_SOURCE)` returns true.
*(music.c:61–76)*

**MUSIC-PLAY-06:** `PLRSeek` shall call `SeekStream(MUSIC_SOURCE, pos)` under the music source mutex when `MusicRef` matches or is wildcard.
*(music.c:78–87)*

**MUSIC-PLAY-07:** `PLRPause` shall call `PauseStream(MUSIC_SOURCE)` under the music source mutex when `MusicRef` matches or is wildcard.
*(music.c:89–98)*

**MUSIC-PLAY-08:** `PLRResume` shall call `ResumeStream(MUSIC_SOURCE)` under the music source mutex when `MusicRef` matches or is wildcard.
*(music.c:100–109)*

#### MUSIC-SPEECH: Speech-as-Music

**MUSIC-SPEECH-01:** `snd_PlaySpeech` shall call `PlayStream` on `SPEECH_SOURCE` with no looping, no scope, and with rewind.
*(music.c:112–125)*

**MUSIC-SPEECH-02:** `snd_StopSpeech` shall stop the speech stream and clear `curSpeechRef`. When `curSpeechRef` is already 0, it shall return immediately.
*(music.c:127–138)*

#### MUSIC-LOAD: Music Loading

**MUSIC-LOAD-01:** `_GetMusicData` shall return NULL if `_cur_resfile_name` is NULL.
*(music.c:170–171)*

**MUSIC-LOAD-02:** `_GetMusicData` shall load a decoder via `SoundDecoder_Load(contentDir, filename, 4096, 0, 0)` and create a sample with 64 buffers and no callbacks.
*(music.c:178, 192)*

**MUSIC-LOAD-03:** `_GetMusicData` shall allocate `MUSIC_REF` (a `TFB_SoundSample**`) via `AllocMusicData`, store the sample pointer, and return it.
*(music.c:185–201)*

**MUSIC-LOAD-04:** When the decoder fails to load, `_GetMusicData` shall return NULL.
*(music.c:179–183)*

**MUSIC-LOAD-05:** When `AllocMusicData` fails, `_GetMusicData` shall free the decoder and return NULL.
*(music.c:186–190)*

**MUSIC-LOAD-06:** `CheckMusicResName` shall log a warning if the file does not exist in `contentDir`, but shall still return the filename.
*(music.c:154–160)*

#### MUSIC-RELEASE: Music Release

**MUSIC-RELEASE-01:** `_ReleaseMusicData` shall return FALSE when passed a NULL pointer.
*(music.c:210–211)*

**MUSIC-RELEASE-02:** When the sample has a decoder: `_ReleaseMusicData` shall lock the music source mutex, check if the sample is currently playing on `MUSIC_SOURCE`, and if so, stop the stream.
*(music.c:218–223)*

**MUSIC-RELEASE-03:** `_ReleaseMusicData` shall set `sample->decoder = NULL`, free the decoder, destroy the sample, and free the `MUSIC_REF` allocation.
*(music.c:225–229)*

**MUSIC-RELEASE-04:** `DestroyMusic` shall delegate to `_ReleaseMusicData`.
*(music.c:141–144)*

#### MUSIC-VOLUME: Music Volume

**MUSIC-VOLUME-01:** `SetMusicVolume` shall compute gain as `(Volume / 255.0) * musicVolumeScale` and apply it to `MUSIC_SOURCE` via `audio_Sourcef(handle, audio_GAIN, gain)`. It shall store `Volume` in the global `musicVolume`.
*(music.c:147–152)*

---

### SFX — Sound Effects (`sfx.c`)

#### SFX-PLAY: Effect Playback

**SFX-PLAY-01:** `PlayChannel` shall stop any existing playback on the channel via `StopSource(channel)` before starting new playback.
*(sfx.c:41)*

**SFX-PLAY-02:** `PlayChannel` shall call `CheckFinishedChannels()` to clean up all stopped SFX sources before playback.
*(sfx.c:44)*

**SFX-PLAY-03:** When `GetSoundAddress(snd)` returns NULL, `PlayChannel` shall return without playing.
*(sfx.c:46–47)*

**SFX-PLAY-04:** `PlayChannel` shall dereference the `SOUNDPTR` to get the `TFB_SoundSample*`, set `soundSource[channel].sample` and `soundSource[channel].positional_object`.
*(sfx.c:49–53)*

**SFX-PLAY-05:** Where `optStereoSFX` is enabled, `PlayChannel` shall apply the provided `SoundPosition`; otherwise it shall apply the non-positional constant `{FALSE, 0, 0}`.
*(sfx.c:54)*

**SFX-PLAY-06:** `PlayChannel` shall bind the sample's single buffer to the source via `audio_Sourcei(handle, audio_BUFFER, sample->buffer[0])` and call `audio_SourcePlay`.
*(sfx.c:56–58)*

**SFX-PLAY-07:** `StopChannel` shall call `StopSource(channel)`. The `Priority` parameter shall be ignored.
*(sfx.c:63–67)*

**SFX-PLAY-08:** `CheckFinishedChannels` shall iterate all SFX source indices (`FIRST_SFX_SOURCE` to `LAST_SFX_SOURCE`) and call `CleanSource` on any source whose state is `audio_STOPPED`.
*(sfx.c:69–87)*

**SFX-PLAY-09:** `ChannelPlaying` shall query `audio_SOURCE_STATE` and return TRUE only when the state is `audio_PLAYING`.
*(sfx.c:89–99)*

#### SFX-POSITION: Positional Audio

**SFX-POSITION-01:** When `pos.positional` is true, `UpdateSoundPosition` shall compute audio position as `(pos.x / 160.0, 0.0, pos.y / 160.0)`.
*(sfx.c:124–126)*

**SFX-POSITION-02:** When the computed distance from origin is less than `MIN_DISTANCE = 0.5`, `UpdateSoundPosition` shall scale the position vector outward to exactly `MIN_DISTANCE`.
*(sfx.c:128–134)*

**SFX-POSITION-03:** When `pos.positional` is false, `UpdateSoundPosition` shall set the audio position to `(0, 0, -1)`.
*(sfx.c:142–144)*

**SFX-POSITION-04:** `GetPositionalObject` shall return `soundSource[channel].positional_object`.
*(sfx.c:101–105)*

**SFX-POSITION-05:** `SetPositionalObject` shall set `soundSource[channel].positional_object`.
*(sfx.c:107–111)*

#### SFX-VOLUME: Channel Volume

**SFX-VOLUME-01:** `SetChannelVolume` shall compute gain as `(volume / 255.0) * sfxVolumeScale` and apply via `audio_Sourcef(handle, audio_GAIN, gain)`. The `priority` parameter shall be ignored.
*(sfx.c:148–156)*

#### SFX-LOAD: Sound Bank Loading

**SFX-LOAD-01:** `_GetSoundBankData` shall extract the directory prefix from `_cur_resfile_name` by finding the last `/` or `\` separator.
*(sfx.c:173–187)*

**SFX-LOAD-02:** `_GetSoundBankData` shall read lines from the file, parsing each as a filename (up to `MAX_FX = 256` sound effects).
*(sfx.c:190–233)*

**SFX-LOAD-03:** For each sound effect, the system shall load a decoder with `SoundDecoder_Load(contentDir, filename, 4096, 0, 0)`, create a sample with 1 buffer and no callbacks (`TFB_CreateSoundSample(NULL, 1, NULL)`), fully pre-decode via `SoundDecoder_DecodeAll`, upload the decoded data to the sample's single buffer, record the length, and free the decoder.
*(sfx.c:206–229)*

**SFX-LOAD-04:** When no sound effects are successfully decoded, `_GetSoundBankData` shall return NULL.
*(sfx.c:235–236)*

**SFX-LOAD-05:** `_GetSoundBankData` shall allocate a `STRING_TABLE` and populate each entry with a heap-allocated pointer to the `TFB_SoundSample*`.
*(sfx.c:238–256)*

**SFX-LOAD-06:** When `STRING_TABLE` allocation fails, `_GetSoundBankData` shall destroy all already-created samples and return NULL.
*(sfx.c:239–244)*

**SFX-LOAD-07:** `GetSoundAddress` shall delegate to `GetStringAddress` to retrieve the `SOUNDPTR` from a `SOUND` handle.
*(sfx.c:304–308)*

#### SFX-RELEASE: Sound Bank Release

**SFX-RELEASE-01:** `_ReleaseSoundBankData` shall return FALSE when passed a NULL pointer.
*(sfx.c:265–266)*

**SFX-RELEASE-02:** For each sample in the sound bank, `_ReleaseSoundBankData` shall check all `NUM_SOUNDSOURCES` to see if the sample is currently playing, and if so, stop the source and clear its sample pointer.
*(sfx.c:275–282)*

**SFX-RELEASE-03:** `_ReleaseSoundBankData` shall free each sample's decoder (if any), destroy the sample, and free the `STRING_TABLE`.
*(sfx.c:284–291)*

**SFX-RELEASE-04:** `DestroySound` shall delegate to `_ReleaseSoundBankData`.
*(sfx.c:296–300)*

---

### VOLUME — Volume & Global Control (`sound.c`)

#### VOLUME-INIT: Initialization

**VOLUME-INIT-01:** The system shall declare a global `soundSource` array of `NUM_SOUNDSOURCES` (7) `TFB_SoundSource` entries.
*(sound.c:30)*

**VOLUME-INIT-02:** The system shall initialize `musicVolume` to `NORMAL_VOLUME` (160).
*(sound.c:26)*

**VOLUME-INIT-03:** The system shall declare global volume scale floats: `musicVolumeScale`, `sfxVolumeScale`, `speechVolumeScale`.
*(sound.c:27–29)*

**VOLUME-INIT-04:** `InitSound` shall accept `argc`/`argv` but ignore them and return TRUE.
*(sound.c:128–134)*

**VOLUME-INIT-05:** `UninitSound` shall be a no-op.
*(sound.c:137–140)*

#### VOLUME-CONTROL: Volume Control

**VOLUME-CONTROL-01:** `SetSFXVolume` shall apply the given volume to all SFX sources (indices `FIRST_SFX_SOURCE` through `LAST_SFX_SOURCE`) via `audio_Sourcef(handle, audio_GAIN, volume)`.
*(sound.c:143–150)*

**VOLUME-CONTROL-02:** `SetSpeechVolume` shall apply the given volume to `SPEECH_SOURCE` via `audio_Sourcef(handle, audio_GAIN, volume)`.
*(sound.c:153–156)*

**VOLUME-CONTROL-03:** `FadeMusic` shall clamp `TimeInterval` to 0 when `QuitPosted` is true or `TimeInterval < 0`.
*(sound.c:161–165)*

**VOLUME-CONTROL-04:** `FadeMusic` shall call `SetMusicStreamFade(TimeInterval, end_vol)`. When the fade is rejected, it shall immediately call `SetMusicVolume(end_vol)` and return the current time.
*(sound.c:167–170)*

**VOLUME-CONTROL-05:** When `FadeMusic`'s fade is accepted, it shall return `GetTimeCounter() + TimeInterval + 1`.
*(sound.c:174)*

#### VOLUME-SOURCE: Source Management

**VOLUME-SOURCE-01:** `StopSource` shall call `audio_SourceStop(handle)` followed by `CleanSource(iSource)`.
*(sound.c:75–79)*

**VOLUME-SOURCE-02:** `CleanSource` shall clear `positional_object`, query `audio_BUFFERS_PROCESSED`, unqueue all processed buffers, and call `audio_SourceRewind` to reset the source to initial state.
*(sound.c:45–72)*

**VOLUME-SOURCE-03:** When more than `MAX_STACK_BUFFERS = 64` buffers need unqueuing, `CleanSource` shall heap-allocate the buffer array; otherwise it shall use a stack-allocated array.
*(sound.c:55–62)*

**VOLUME-SOURCE-04:** `StopSound` shall call `StopSource` on all SFX source indices (`FIRST_SFX_SOURCE` through `LAST_SFX_SOURCE`).
*(sound.c:34–42)*

#### VOLUME-QUERY: Playback Queries

**VOLUME-QUERY-01:** `SoundPlaying` shall iterate all `NUM_SOUNDSOURCES` sources. For sources with a sample and decoder, it shall check `PlayingStream` under the stream mutex. For sources without, it shall check `audio_SOURCE_STATE == audio_PLAYING`. It shall return TRUE if any source is playing.
*(sound.c:81–109)*

**VOLUME-QUERY-02:** `WaitForSoundEnd` shall poll in a loop, sleeping `ONE_SECOND / 20` (50ms at 840 ticks/sec) per iteration. When `Channel == TFBSOUND_WAIT_ALL`, it shall wait for `SoundPlaying()` to return false; otherwise it shall wait for `ChannelPlaying(Channel)` to return false.
*(sound.c:113–123)*

**VOLUME-QUERY-03:** `WaitForSoundEnd` shall break immediately when `QuitPosted` is true, to avoid blocking during application shutdown.
*(sound.c:120–121)*

---

### FILEINST — File-Based Loading (`fileinst.c`)

#### FILEINST-LOAD: File Loading

**FILEINST-LOAD-01:** `LoadSoundFile` shall check that `_cur_resfile_name` is NULL (no concurrent loading). Where it is non-NULL, the function shall return 0.
*(fileinst.c:32–34)*

**FILEINST-LOAD-02:** `LoadSoundFile` shall open the resource file from `contentDir` in read-binary mode, set `_cur_resfile_name` to the path string, call `_GetSoundBankData(fp, length)`, clear `_cur_resfile_name`, close the file, and return the result.
*(fileinst.c:36–48)*

**FILEINST-LOAD-03:** When the resource file fails to open, `LoadSoundFile` shall return NULL.
*(fileinst.c:49–50)*

**FILEINST-LOAD-04:** `LoadMusicFile` shall check that `_cur_resfile_name` is NULL (no concurrent loading). Where it is non-NULL, the function shall return 0.
*(fileinst.c:60–62)*

**FILEINST-LOAD-05:** `LoadMusicFile` shall copy the filename (up to 255 chars, null-terminated), validate it with `CheckMusicResName`, open the resource file from `contentDir`, set `_cur_resfile_name`, call `_GetMusicData(fp, length)`, clear `_cur_resfile_name`, close the file, and return the result.
*(fileinst.c:64–83)*

**FILEINST-LOAD-06:** When the resource file fails to open, `LoadMusicFile` shall return 0.
*(fileinst.c:84–85)*

**FILEINST-LOAD-07:** Both `LoadSoundFile` and `LoadMusicFile` shall reset `_cur_resfile_name` to 0 (NULL) after loading completes, regardless of success or failure of the data loading function.
*(fileinst.c:43, 78)*

---

### CROSS-CUTTING: Cross-Cutting Requirements

#### CROSS-THREAD: Thread Safety

**CROSS-THREAD-01:** All streaming functions that modify `TFB_SoundSource` fields shall be called with the appropriate `stream_mutex` held by the caller.

**CROSS-THREAD-02:** The decoder thread shall lock each source's `stream_mutex` individually before processing and unlock it after.

**CROSS-THREAD-03:** The fade system shall use a dedicated `fade_mutex` to protect all four fade state variables (`musicFadeStartTime`, `musicFadeInterval`, `musicFadeStartVolume`, `musicFadeDelta`).

**CROSS-THREAD-04:** The `_cur_resfile_name` global shall be treated as a crude mutual-exclusion guard for resource loading (noted as FIXME in original code — proper synchronization is needed).

#### CROSS-MEMORY: Memory Management

**CROSS-MEMORY-01:** All heap allocations shall use `HCalloc` (zeroed allocation) or `HMalloc` (unzeroed) and be freed with `HFree`.

**CROSS-MEMORY-02:** `TFB_SoundSample` ownership: the sample owns its buffer array and buffer_tag array. The decoder is NOT owned by the sample (it must be freed separately).

**CROSS-MEMORY-03:** `TFB_SoundChunk` ownership: each chunk owns its decoder and its subtitle text string. The linked list is owned by the trackplayer's static state.

**CROSS-MEMORY-04:** Audio buffer handles (`audio_Object`) shall be created with `audio_GenBuffers` and freed with `audio_DeleteBuffers`.

#### CROSS-CONST: Constants

**CROSS-CONST-01:** The system shall define `MAX_VOLUME = 255`.

**CROSS-CONST-02:** The system shall define `NORMAL_VOLUME = 160`.

**CROSS-CONST-03:** The system shall define `NUM_SFX_CHANNELS = 5` (= `MIN_FX_CHANNEL + NUM_FX_CHANNELS = 1 + 4`).

**CROSS-CONST-04:** The system shall define source indices: `FIRST_SFX_SOURCE = 0`, `LAST_SFX_SOURCE = 4`, `MUSIC_SOURCE = 5`, `SPEECH_SOURCE = 6`, `NUM_SOUNDSOURCES = 7`.

**CROSS-CONST-05:** The system shall define `PAD_SCOPE_BYTES = 256`.

**CROSS-CONST-06:** The system shall define `ACCEL_SCROLL_SPEED = 300`.

**CROSS-CONST-07:** The system shall define `TEXT_SPEED = 80` (ms per character for subtitle timing).

**CROSS-CONST-08:** The system shall define `ONE_SECOND = 840` (game time units per second).

#### CROSS-FFI: Foreign Function Interface

**CROSS-FFI-01:** All public API functions shall be exposed as `extern "C"` with `#[no_mangle]` for C callers.

**CROSS-FFI-02:** Type mappings shall preserve C ABI compatibility: `MUSIC_REF` as `*mut *mut TFB_SoundSample`, `SOUND_REF` as `*mut STRING_TABLE`, `SOUNDPTR` as `*mut c_void`, `audio_Object` as `uintptr_t`, etc.

**CROSS-FFI-03:** The Rust implementation shall call the Rust mixer API directly (not through FFI) where possible, eliminating the C audiocore indirection layer.

**CROSS-FFI-04:** The Rust implementation shall call Rust decoder methods directly via the `SoundDecoder` trait rather than through C vtable dispatch.

#### CROSS-ERROR: Error Handling

**CROSS-ERROR-01:** Where audio backend operations fail (non-zero `audio_GetError()`), the system shall log a warning and continue operating without crashing.

**CROSS-ERROR-02:** Where a decoder returns `SOUNDDECODER_ERROR`, the system shall log the error and either skip the current operation or halt the stream, but shall not panic.

**CROSS-ERROR-03:** Where resource loading functions fail (NULL decoder, file not found), the system shall log a warning and return NULL/0 to the caller.

---

*End of requirements. Total requirements: 137 (STREAM: 52, TRACK: 37, MUSIC: 16, SFX: 18, VOLUME: 12, FILEINST: 7, CROSS-CUTTING: 15).*

---

## Review Notes

*Reviewed by cross-referencing against the actual C source files: stream.c, trackplayer.c, music.c, sfx.c, sound.c, fileinst.c and their headers.*

### Accuracy Issues

1. **stream.c fade mutex**: The document should emphasize that `fade_mutex` protects `musicFadeStartTime`, `musicFadeInterval`, `musicFadeStartVolume`, and `musicFadeDelta` — but the actual fade *application* in `process_stream()` (around line 514-530) reads these variables **without** holding the mutex in some paths. The Rust port must decide whether to fix this race or preserve it.

2. **StopStream vs PauseStream**: `StopStream()` (line 131) calls `audio_SourceStop()` then clears `sample->decoder->looping`, resets `sample->read_chain_ptr`, and clears `source->sample`. `PauseStream()` (line 187) actually calls `PlayStream()` with `rewind=false` — it's a "resume from current position" not a traditional pause. This nuance should be verified in the EARS requirements.

3. **StreamDecoderTaskFunc buffer management**: The task (line 536+) uses `audio_SourceUnqueueBuffers` to reclaim processed buffers, then re-fills them via `SoundDecoder_Decode`. The double-buffering scheme uses `sample->num_buffers` buffers (typically 2-4). The exact buffer count and its relationship to latency should be noted.

4. **sfx.c CheckFinishedChannels**: This function (line 70) iterates `soundSource[]` and checks if sources have stopped playing (`audio_STOPPED`). It then cleans up by calling `audio_DeleteBuffers` and clearing the source's sample pointer. This cleanup is called from `PlaySoundEffect` — meaning SFX cleanup is lazy/on-demand, not proactive.

5. **sound.c volume scaling**: Volume functions use `sfxVolumeScale`, `speechVolumeScale`, and `musicVolume` globals. The `musicVolume` is the "master" music volume (0-100 range), while `sfxVolumeScale` and `speechVolumeScale` are 0-255 fixed-point scales applied via `audio_Sourcef` with `audio_GAIN`. The distinction matters for the Rust port.

### Completeness Gaps

1. **Missing: `TFB_SetScopeData` in stream.c**: The `add_scope_data()` function (line 322) fills a circular scope buffer used for the oscilloscope display in comm screens. This visual feedback loop (audio→scope→rendering) is important for the comm system and should have explicit EARS requirements.

2. **Missing: `ResumeStream()` semantics**: `PauseStream()` at line 187 doesn't actually pause — it calls `PlayStream()` with `rewind=false`. The document should clarify that there's no true "pause/resume" in stream.c; pausing is done at the `audio_SourcePause`/`audio_SourcePlay` level in `trackplayer.c`.

3. **Missing: decoder error recovery in StreamDecoderTaskFunc**: Around lines 370-490, the task has multiple error paths: decoder returns 0 bytes (EOF), decoder returns negative (error), queue full, unqueue failure. Each has different recovery behavior. The EARS requirements should cover each error path individually.

4. **Missing: fileinst.c `_GetSoundBankData` and `_GetMusicData`**: These are the resource-loading callbacks registered with the resource system. The document should clarify these are `RESOURCE_FREE_FUNC` / `STRING_TABLE_LOAD_FUNC` registered callbacks, not directly called functions.

5. **Missing: sfx.c `AUDIO_NUM_FX` constant**: The maximum number of concurrent sound effects is limited by `NUM_SOUNDSOURCES` (defined in sndintrn.h, typically 8-16). Some sources are reserved for music/speech. The EARS requirements should specify source allocation strategy.

### EARS Issues

1. **Several requirements lack measurable values**: E.g., "the system shall fade music" — over what time range? The C code uses `musicFadeInterval` in `ONE_SECOND` units (typically 1/60th second ticks). Requirements should reference these constants.

2. **Threading requirements need mutex specification**: Requirements about thread safety should specify which mutex protects which data. The C code uses `stream_mutex` (per-source), `fade_mutex` (global fade state), and `GraphicsLock` (for scope data). The Rust port needs to know exactly which `Mutex<>` wraps which state.

3. **EARS pattern consistency**: Some requirements use "the system" generically. For porting clarity, they should specify the exact Rust module: "the streaming subsystem", "the track player", "the SFX manager", etc.

### Integration Notes

1. **Mixer is already Rust**: The `audio_*` calls in stream.c (SourcePlay, SourceStop, SourceQueueBuffers, etc.) already route through `audiocore_rust.c` → `rodio_backend.rs` → Rust mixer. The Rust port of stream.c can call the mixer directly without FFI.

2. **Decoders are already Rust**: `SoundDecoder_Decode()` in stream.c calls through the vtable which is already Rust for wav/ogg/mod/dukaud. The Rust streaming thread can call decoders directly.

3. **The streaming thread is the key integration point**: Once `stream.c` is ported, the entire audio path from resource load → decode → stream → mix → output will be Rust. The C `audiocore_rust.c` shim can potentially be eliminated.

4. **Scope data feeds the comm oscilloscope**: The `add_scope_data` → `TFB_ScopeData` path connects audio to the graphics system. The Rust port must maintain this cross-subsystem interface, likely via a shared `Arc<Mutex<>>` ring buffer.

5. **trackplayer.c depends on stream.c**: Port stream.c first, then trackplayer.c. music.c/sfx.c/sound.c can be ported in parallel once stream.c is done.
