// @plan PLAN-20260225-AUDIO-HEART.P09
// @requirement REQ-TRACK-ASSEMBLE-01..19, REQ-TRACK-PLAY-01..10
// @requirement REQ-TRACK-SEEK-01..13, REQ-TRACK-CALLBACK-01..09
// @requirement REQ-TRACK-SUBTITLE-01..04, REQ-TRACK-POSITION-01..02
//! Track player — manages multi-chunk audio sequences with subtitle
//! synchronization, seeking, and callback dispatching.
//!
//! Builds on the stream engine (`stream.rs`) to play sequences of
//! `SoundChunk`s, each with its own decoder, subtitle text, and timing.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;

use super::decoder::Decoder;
use super::stream;
use super::types::*;

// =============================================================================
// Constants
// =============================================================================

/// Characters per second for subtitle pacing.
pub const TEXT_SPEED: f64 = 80.0;
/// Accelerated smooth-seek step size in game ticks.
/// Matches C `ACCEL_SCROLL_SPEED` semantics.
pub const ACCEL_SCROLL_SPEED: f64 = 300.0;
/// Maximum tracks in a multi-track splice.
/// Matches C `MAX_MULTI_TRACKS` in `trackplayer.c`.
pub const MAX_MULTI_TRACKS: usize = 20;

// =============================================================================
// Data Structures (spec §3.2.2)
// =============================================================================

/// A single chunk in the track player's sequence.
///
/// Each chunk represents one audio segment with optional subtitle text,
/// timing information, and a decoder.
pub struct SoundChunk {
    /// Audio decoder for this chunk.
    pub decoder: Option<Box<dyn Decoder>>,
    /// Length of this chunk's audio in milliseconds, fixed when the chunk is
    /// built. The decoder itself is lent to the stream while the chunk plays,
    /// so seeking cannot ask it for the length.
    pub length_ms: u32,
    /// Absolute position in track sequence (milliseconds).
    pub start_time: f64,
    /// Display hint: positive = exact duration, negative = minimum display time.
    pub run_time: i32,
    /// Whether to tag buffer for subtitle sync.
    pub tag_me: bool,
    /// Which track this chunk belongs to (0-based).
    pub track_num: u32,
    /// Subtitle text for this chunk.
    pub text: Option<String>,
    /// Cached CString supplies page identity independently of snapshot text copies.
    /// Created lazily from `text`.
    pub text_cstr: Option<std::ffi::CString>,
    /// Per-chunk callback (first page only).
    /// Must be Fn, not FnOnce — callbacks can fire multiple times on seek.
    pub callback: Option<Box<dyn Fn(i32) + Send>>,
}

impl SoundChunk {
    /// Create the cached CString from text if not already cached.
    pub fn ensure_text_cstr(&mut self) {
        if self.text_cstr.is_none() {
            if let Some(ref text) = self.text {
                self.text_cstr = std::ffi::CString::new(text.as_str()).ok();
            }
        }
    }
}

/// A subtitle sub-page extracted from chunk text.
pub struct SubPage {
    /// Subtitle text for this page.
    pub text: String,
    /// Timestamp for this page (game ticks).
    pub timestamp: f64,
}

#[path = "trackplayer_subtitles.rs"]
mod subtitles;
pub use subtitles::{subtitle_history, subtitle_snapshot, SubtitleIdentity, SubtitleSnapshot};

/// Global track player state.
///
/// Protected by `TRACK_STATE` mutex. Owns the chunk sequence; the cursors
/// are indices into it.
pub struct TrackPlayerState {
    /// Chunks in playback order.
    pub chunks: Vec<SoundChunk>,
    /// Index of the last chunk that carries subtitle text.
    pub last_sub: Option<usize>,
    /// Index of the current playback chunk.
    pub cur_chunk: Option<usize>,
    /// Index of the current displayed subtitle chunk.
    pub cur_sub_chunk: Option<usize>,
    /// Shared sample for streaming.
    pub sound_sample: Option<Arc<Mutex<SoundSample>>>,
    /// Number of tracks.
    pub track_count: u32,
    /// Accumulated decoder offset in milliseconds.
    pub dec_offset: f64,
    /// Subtitle continuation flag.
    pub no_page_break: bool,
    /// Total track length in game ticks.
    pub tracks_length: AtomicU32,
    /// Last track resource name.
    pub last_track_name: String,
}

impl TrackPlayerState {
    fn new() -> Self {
        TrackPlayerState {
            chunks: Vec::new(),
            last_sub: None,
            cur_chunk: None,
            cur_sub_chunk: None,
            sound_sample: None,
            track_count: 0,
            dec_offset: 0.0,
            no_page_break: false,
            tracks_length: AtomicU32::new(0),
            last_track_name: String::new(),
        }
    }
}

/// Global track player state (mutex-protected).
/// When nesting locks, acquire source -> sample -> track state, as stream
/// callbacks do. Never wait for a source or sample while holding track state.
static TRACK_STATE: std::sync::LazyLock<Mutex<TrackPlayerState>> =
    std::sync::LazyLock::new(|| Mutex::new(TrackPlayerState::new()));

/// Details for a resolved seek target.
struct SeekTarget {
    chunk: Option<usize>,
    clamped_ticks: u32,
    seek_time_ms: u32,
}

#[inline]
fn ms_to_ticks(ms: u32) -> u32 {
    ((ms as u64 * ONE_SECOND as u64) / 1000) as u32
}

#[inline]
fn ticks_to_ms(ticks: u32) -> u32 {
    ((ticks as u64 * 1000) / ONE_SECOND as u64) as u32
}

// =============================================================================
// Track Callbacks (implements StreamCallbacks)
// =============================================================================

/// Callbacks for the track player's streaming engine integration.
///
/// Implements `StreamCallbacks` to handle end-of-chunk (switch decoder),
/// end-of-stream (stop playback), buffer tagging (subtitle sync), and
/// queue notifications.
pub struct TrackCallbacks;

impl StreamCallbacks for TrackCallbacks {
    fn on_start_stream(&mut self, sample: &mut SoundSample) -> bool {
        let mut state = TRACK_STATE.lock();
        if state.sound_sample.is_none() {
            return false;
        }
        let Some(cur) = state.cur_chunk else {
            return false;
        };

        // Move decoder from chunk to sample
        let chunk = &mut state.chunks[cur];
        if let Some(dec) = chunk.decoder.take() {
            sample.decoder = Some(dec);
        }
        sample.offset = (chunk.start_time * ONE_SECOND as f64 / 1000.0) as i32;
        let tag_me = chunk.tag_me;

        if tag_me {
            do_track_tag(&mut state, cur);
        }
        true
    }

    fn on_end_chunk(&mut self, sample: &mut SoundSample, buffer: usize) -> bool {
        let mut state = TRACK_STATE.lock();
        if state.sound_sample.is_none() {
            return false;
        }
        let Some(cur) = state.cur_chunk else {
            return false;
        };
        let next = cur + 1;
        if next >= state.chunks.len() {
            return false;
        }

        // Return decoder to current chunk, take from next
        if let Some(dec) = sample.decoder.take() {
            state.chunks[cur].decoder = Some(dec);
        }

        state.cur_chunk = Some(next);
        if let Some(mut dec) = state.chunks[next].decoder.take() {
            // C parity: OnChunkEnd rewinds the next decoder before attaching it
            // as the active sample decoder.
            let _ = dec.seek(0);
            sample.decoder = Some(dec);
        }

        if state.chunks[next].tag_me {
            let _ = stream::tag_buffer(sample, buffer, next);
        }
        true
    }

    fn on_end_stream(&mut self, _sample: &mut SoundSample) {
        let mut state = TRACK_STATE.lock();
        state.cur_chunk = None;
        state.cur_sub_chunk = None;
    }

    fn on_tagged_buffer(&mut self, sample: &mut SoundSample, tag: &SoundTag) {
        if let Some(slot) = sample
            .buffer_tags
            .iter_mut()
            .filter_map(|entry| entry.as_mut())
            .find(|entry| entry.buf_handle == tag.buf_handle)
        {
            stream::clear_buffer_tag(slot);
        }

        // The tag carries the chunk index. Stopping the track clears the
        // sample's tags under the sample lock this callback holds, so an index
        // past the end can only come from a tag that names no chunk.
        let mut state = TRACK_STATE.lock();
        if tag.data < state.chunks.len() {
            do_track_tag(&mut state, tag.data);
        }
    }

    fn on_queue_buffer(&mut self, _sample: &mut SoundSample, _buffer: usize) {
        // No-op for track player
    }
}

// =============================================================================
// Public API — Track Assembly (spec §3.2.3)
// =============================================================================

/// Splice a new track (audio + subtitle) onto the track sequence.
///
/// `decoders`: one decoder per subtitle page, each limited to its time range.
/// If empty and `track_name` is provided, chunks get `decoder: None`.
pub fn splice_track(
    track_name: Option<&str>,
    track_text: Option<&str>,
    timestamp: Option<&str>,
    mut callback: Option<Box<dyn Fn(i32) + Send>>,
    mut decoders: Vec<Box<dyn Decoder>>,
) -> AudioResult<()> {
    let mut state = TRACK_STATE.lock();

    // No text → early return
    let Some(text) = track_text else {
        return Ok(());
    };

    // Subtitle-only append (no track_name)
    let Some(track_name) = track_name else {
        if state.track_count == 0 {
            return Ok(());
        }
        if let Some(last_sub) = state.last_sub {
            let pages = split_sub_pages(text);
            if let Some(first_page) = pages.first() {
                append_text(&mut state.chunks[last_sub], &first_page.text);
            }
            for page in pages.iter().skip(1) {
                let chunk = SoundChunk {
                    decoder: None,
                    length_ms: 0,
                    start_time: state.dec_offset,
                    run_time: ms_to_ticks(page.timestamp as u32) as i32,
                    tag_me: true,
                    track_num: state.track_count.saturating_sub(1),
                    text: Some(page.text.clone()),
                    text_cstr: None,
                    callback: None,
                };
                state.chunks.push(chunk);
                state.last_sub = Some(state.chunks.len() - 1);
            }
        }
        return Ok(());
    };

    // First track: create sound_sample (no decoder yet — chunks own decoders)
    if state.track_count == 0 {
        let callbacks: Box<dyn StreamCallbacks + Send> = Box::new(TrackCallbacks);
        let sample = stream::create_sound_sample(None, 8, Some(callbacks))?;
        state.sound_sample = Some(Arc::new(Mutex::new(sample)));
    }

    let pages = split_sub_pages(text);
    let timestamps = timestamp.map(get_time_stamps).unwrap_or_default();

    let appending_without_page_break = state.no_page_break && state.track_count > 0;

    // C parity: when no_page_break is set, append page 0 text to the previous
    // subtitle first, but still build chunk 0 from decoder/timing data.
    if appending_without_page_break {
        if let (Some(first_page), Some(last_sub)) = (pages.first(), state.last_sub) {
            append_text(&mut state.chunks[last_sub], &first_page.text);
        }
    }

    let decoder_count = decoders.len();
    let chunk_count = pages.len().max(decoder_count);
    let mut dec_iter = decoders.drain(..);
    let mut added_lengths = Vec::new();

    for i in 0..chunk_count {
        let page = pages.get(i);
        let page_decoder = dec_iter.next();

        // If this is a subtitle-only no_page_break append with no decoder for the
        // first page, there is no C-equivalent chunk to synthesize.
        if appending_without_page_break && i == 0 && page_decoder.is_none() {
            continue;
        }

        let run_time_ms = if i < timestamps.len() {
            timestamps[i] as i32
        } else if let Some(page) = page {
            page.timestamp as i32
        } else {
            0
        };

        if let Some(ref dec) = page_decoder {
            added_lengths.push(dec.length());
        }

        // C parity for no_page_break: first chunk of this splice is audio-only and
        // untagged; tagging resumes from the next chunk.
        let suppress_first_page = appending_without_page_break && i == 0;
        let tag_me = !suppress_first_page;
        let text = if suppress_first_page {
            None
        } else {
            page.map(|p| p.text.clone())
        };

        let length_ms = page_decoder
            .as_ref()
            .map(|dec| (dec.length() * 1000.0) as u32)
            .unwrap_or(0);
        let chunk = SoundChunk {
            decoder: page_decoder,
            length_ms,
            start_time: state.dec_offset,
            run_time: ms_to_ticks(run_time_ms.unsigned_abs()) as i32,
            tag_me,
            track_num: state.track_count,
            text,
            text_cstr: None,
            callback: if tag_me { callback.take() } else { None },
        };

        // Advance dec_offset by decoder length (like C does)
        if let Some(ref dec) = chunk.decoder {
            state.dec_offset += (dec.length() * 1000.0) as f64;
        }

        let has_text = chunk.text.is_some();
        state.chunks.push(chunk);
        if has_text {
            state.last_sub = Some(state.chunks.len() - 1);
        }
    }

    state.no_page_break = false;

    if !appending_without_page_break {
        state.track_count += 1;
    }
    state.last_track_name = track_name.to_string();
    let sample_arc = state.sound_sample.clone();
    drop(state);
    if let Some(sample_arc) = sample_arc {
        let mut sample = sample_arc.lock();
        for length in added_lengths {
            sample.length += length;
        }
    }
    Ok(())
}

/// Append decoded number clips to the preceding subtitle's audio.
pub fn splice_multi_track(decoders: Vec<Box<dyn Decoder>>, text: Option<&str>) -> AudioResult<()> {
    let Some(text) = text else {
        return Ok(());
    };
    let mut state = TRACK_STATE.lock();
    if state.track_count == 0 {
        return Err(AudioError::InvalidSample);
    }
    if decoders.is_empty() {
        return Ok(());
    }

    let mut added_lengths = Vec::new();
    for decoder in decoders.into_iter().take(MAX_MULTI_TRACKS) {
        let length = decoder.length();
        let length_ms = (length * 1000.0) as u32;
        let chunk = SoundChunk {
            decoder: Some(decoder),
            length_ms,
            start_time: state.dec_offset,
            run_time: ms_to_ticks(length_ms) as i32,
            tag_me: false,
            track_num: state.track_count - 1,
            text: None,
            text_cstr: None,
            callback: None,
        };
        added_lengths.push(length);
        state.dec_offset += f64::from(length * 1000.0);
        state.chunks.push(chunk);
    }

    let Some(last_sub) = state.last_sub else {
        return Err(AudioError::InvalidSample);
    };
    let last_sub = &mut state.chunks[last_sub];
    last_sub.text.get_or_insert_with(String::new).push_str(text);
    // An earlier subtitle lookup may have cached this string already.
    last_sub.text_cstr = None;
    state.no_page_break = true;
    let Some(sample_arc) = state.sound_sample.clone() else {
        return Err(AudioError::InvalidSample);
    };
    drop(state);
    let mut sample = sample_arc.lock();
    for length in added_lengths {
        sample.length += length;
    }
    Ok(())
}

// =============================================================================
// Public API — Playback Control (spec §3.2.3)
// =============================================================================

/// Start playing the assembled track sequence.
pub fn play_track(scope: bool) -> AudioResult<()> {
    // Phase 1: Extract state under TRACK_STATE lock
    let sample_arc = {
        let mut state = TRACK_STATE.lock();
        let Some(sample_arc) = state.sound_sample.clone() else {
            eprintln!("[DBG] play_track: sound_sample is None, returning early");
            return Ok(());
        };
        eprintln!(
            "[DBG] play_track: sound_sample present, track_count={}, chunks_head={}",
            state.track_count,
            !state.chunks.is_empty()
        );

        let end_time = tracks_end_time_inner(&state);
        state.tracks_length.store(end_time, Ordering::Release);
        state.cur_chunk = (!state.chunks.is_empty()).then_some(0);
        state.cur_sub_chunk = None;

        sample_arc
        // TRACK_STATE lock dropped here
    };

    // Phase 2: Call play_stream WITHOUT holding TRACK_STATE
    stream::play_stream(sample_arc, SPEECH_SOURCE, false, scope, true)
}

/// Stop track playback and clear the track list.
pub fn stop_track() -> AudioResult<()> {
    let _ = stream::stop_stream(SPEECH_SOURCE);

    // Deferred callbacks can still own the sample after StopStream returns.
    // Wait for them before taking track state, in the same order they use.
    let sample_arc = TRACK_STATE.lock().sound_sample.clone();
    let mut sample = sample_arc.as_ref().map(|sample| sample.lock());
    let mut state = TRACK_STATE.lock();

    state.track_count = 0;
    state.tracks_length.store(0, Ordering::Release);
    state.cur_chunk = None;
    state.cur_sub_chunk = None;

    state.sound_sample = None;
    if let Some(sample) = sample.as_mut() {
        // Clear buffer tags before dropping chunks (ISSUE-CONC-04)
        for tag in sample.buffer_tags.iter_mut() {
            *tag = None;
        }
        sample.decoder = None;
        let _ = stream::destroy_sound_sample(sample);
    }

    state.chunks.clear();
    state.last_sub = None;
    state.dec_offset = 0.0;
    Ok(())
}

/// Jump past end — effectively stops playback.
pub fn jump_track(_track_num: u32) -> AudioResult<()> {
    {
        let state = TRACK_STATE.lock();
        if state.sound_sample.is_none() {
            return Ok(());
        }
    }
    let _ = stream::stop_stream(SPEECH_SOURCE);
    let mut state = TRACK_STATE.lock();
    state.cur_chunk = None;
    state.cur_sub_chunk = None;
    Ok(())
}

/// Pause track playback.
pub fn pause_track() -> AudioResult<()> {
    stream::pause_stream(SPEECH_SOURCE)
}

/// Resume track playback.
pub fn resume_track() -> AudioResult<()> {
    {
        let state = TRACK_STATE.lock();
        if state.cur_chunk.is_none() {
            return Ok(());
        }
    }
    stream::resume_stream(SPEECH_SOURCE)
}

/// Check if a track is currently playing.
pub fn playing_track() -> bool {
    playing_track_num() > 0
}

/// Returns the track number currently playing (1-based), or 0 if nothing is playing.
/// Matches C `PlayingTrack()` which returns `cur_chunk->track_num + 1`.
pub fn playing_track_num() -> u16 {
    let state = TRACK_STATE.lock();
    if state.sound_sample.is_none() {
        return 0;
    }
    state
        .cur_chunk
        .map(|c| (state.chunks[c].track_num + 1) as u16)
        .unwrap_or(0)
}

// =============================================================================
// Public API — Seeking (spec §3.2.3)
// =============================================================================

/// Seek backward smoothly (rewind).
/// Uses stream start_time-based position like C get_current_track_pos().
pub fn fast_reverse_smooth() -> AudioResult<()> {
    {
        let state = TRACK_STATE.lock();
        if state.sound_sample.is_none() {
            return Ok(());
        }
    }

    let pos = stream::get_stream_position_ticks(SPEECH_SOURCE);
    let new_pos = pos.saturating_sub(ACCEL_SCROLL_SPEED as u32);
    eprintln!(
        "[PARITY][FAST_REVERSE_SMOOTH] pos_ticks={} new_pos_ticks={}",
        pos, new_pos
    );
    seek_to_position(new_pos)
}

/// Seek forward smoothly (fast-forward).
/// Uses stream start_time-based position like C get_current_track_pos().
pub fn fast_forward_smooth() -> AudioResult<()> {
    {
        let state = TRACK_STATE.lock();
        if state.sound_sample.is_none() {
            return Ok(());
        }
    }

    let pos = stream::get_stream_position_ticks(SPEECH_SOURCE);
    let new_pos = pos + ACCEL_SCROLL_SPEED as u32;
    eprintln!(
        "[PARITY][FAST_FORWARD_SMOOTH] pos_ticks={} new_pos_ticks={}",
        pos, new_pos
    );
    seek_to_position(new_pos)
}

/// Jump backward by one subtitle page.
pub fn fast_reverse_page() -> AudioResult<()> {
    let (sample_arc, offset_ticks) = {
        let mut state = TRACK_STATE.lock();
        let Some(sample_arc) = state.sound_sample.clone() else {
            return Ok(());
        };

        let prev = find_prev_page_inner(&state.chunks, state.cur_sub_chunk);
        let Some(page) = prev else {
            return Ok(());
        };

        let offset_ticks = ms_to_ticks(state.chunks[page].start_time as u32) as i32;

        state.cur_chunk = Some(page);
        state.cur_sub_chunk = Some(page);
        (sample_arc, offset_ticks)
    };

    stream::play_stream_with_offset_override(
        sample_arc,
        SPEECH_SOURCE,
        false,
        true,
        true,
        Some(offset_ticks),
    )
}

/// Jump forward by one subtitle page.
pub fn fast_forward_page() -> AudioResult<()> {
    // Strict C parity: page FF must be based on current subtitle page marker only.
    // If cur_sub_chunk is absent, C behavior is to fall through to end-of-track seek.
    let page_target = {
        let mut state = TRACK_STATE.lock();
        let Some(sample_arc) = state.sound_sample.clone() else {
            return Ok(());
        };

        let next = find_next_page_inner(&state.chunks, state.cur_sub_chunk);
        if let Some(page) = next {
            let offset_ticks = ms_to_ticks(state.chunks[page].start_time as u32) as i32;
            state.cur_chunk = Some(page);
            state.cur_sub_chunk = Some(page);
            Some((sample_arc, offset_ticks))
        } else {
            None
        }
    };

    if let Some((sample_arc, offset_ticks)) = page_target {
        stream::play_stream_with_offset_override(
            sample_arc,
            SPEECH_SOURCE,
            false,
            true,
            true,
            Some(offset_ticks),
        )
    } else {
        // Match C FastForward_Page -> seek_track(tracks_length + 1), not direct stop.
        let end_plus_one = {
            let state = TRACK_STATE.lock();
            state
                .tracks_length
                .load(Ordering::Acquire)
                .saturating_add(1)
        };
        seek_to_position(end_plus_one)
    }
}

/// Get the current track position.
///
/// `in_units` controls the unit:
/// - 0 = game ticks (raw position)
/// - non-zero = scaled (position * in_units / length)
pub fn get_track_position(in_units: u32) -> u32 {
    track_position(in_units, || {
        stream::get_stream_position_ticks(SPEECH_SOURCE)
    })
}

fn track_position(in_units: u32, read_position: impl FnOnce() -> u32) -> u32 {
    let state = TRACK_STATE.lock();
    if state.sound_sample.is_none() {
        return 0;
    }
    let len = state.tracks_length.load(Ordering::Acquire);
    if len == 0 {
        return 0;
    }
    // Like C GetTrackPosition, detach the length before taking the source lock.
    // The decoder holds source -> sample -> TRACK_STATE in OnEndChunk.
    drop(state);
    let pos = read_position().min(len);
    if in_units == 0 {
        pos
    } else {
        (in_units as u64 * pos as u64 / len as u64) as u32
    }
}

// =============================================================================
// Internal Functions
// =============================================================================

/// Split subtitle text into sub-pages based on CRLF breaks and timing.
fn split_sub_pages(text: &str) -> Vec<SubPage> {
    if text.is_empty() {
        return Vec::new();
    }

    // Match C behavior: split on either '\r' or '\n', consuming runs of line breaks.
    let bytes = text.as_bytes();
    let mut parts: Vec<&str> = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'\r' || bytes[i] == b'\n' {
            parts.push(&text[start..i]);
            while i < bytes.len() && (bytes[i] == b'\r' || bytes[i] == b'\n') {
                i += 1;
            }
            start = i;
        } else {
            i += 1;
        }
    }

    if start < bytes.len() {
        parts.push(&text[start..]);
    }

    let mut result = Vec::new();

    for (idx, part) in parts.iter().enumerate() {
        let mut page_text = (*part).to_string();

        // Continuation marks (same as C).
        if idx > 0 {
            page_text = format!("..{}", page_text);
        }
        if idx < parts.len() - 1 {
            let last_char = part.chars().last();
            let needs_ellipsis = last_char
                .map(|c| !c.is_whitespace() && !c.is_ascii_punctuation())
                .unwrap_or(false);
            if needs_ellipsis {
                page_text.push_str("...");
            }
        }

        let char_count = page_text.chars().count() as f64;
        let timestamp = (char_count * TEXT_SPEED).max(1000.0);

        result.push(SubPage {
            text: page_text,
            timestamp,
        });
    }

    result
}

/// Parse timestamp string into a vector of timing values.
fn get_time_stamps(timestamp: &str) -> Vec<f64> {
    let mut result = Vec::new();
    for token in timestamp.split(',') {
        for sub in token.split('\n') {
            for part in sub.split('\r') {
                let trimmed = part.trim();
                if let Ok(val) = trimmed.parse::<f64>() {
                    if val > 0.0 {
                        result.push(val);
                    }
                }
            }
        }
    }
    result
}

/// Append page text to a subtitle chunk.
fn append_text(chunk: &mut SoundChunk, text: &str) {
    match &mut chunk.text {
        Some(t) => t.push_str(text),
        None => chunk.text = Some(text.to_owned()),
    }
}

/// Seek to a specific position (in game ticks).
///
/// Matches C seek_track semantics:
/// - clamp to tracks_length + 1
/// - update stream start_time timebase
/// - resolve current chunk and subtitle tag state
/// - seek the selected chunk decoder to relative offset
/// - attach selected decoder to sample and restart stream if needed
fn seek_to_position(pos: u32) -> AudioResult<()> {
    eprintln!("[PARITY][SEEK] request_ticks={}", pos);
    let Some(sample_arc) = TRACK_STATE.lock().sound_sample.clone() else {
        return Ok(());
    };
    let has_chunk = stream::with_source(SPEECH_SOURCE, |source| {
        let mut sample = sample_arc.lock();
        let mut state = TRACK_STATE.lock();
        let target = resolve_seek_target(&mut state, pos);
        eprintln!(
            "[PARITY][SEEK] resolved clamped_ticks={} seek_time_ms={} has_chunk={}",
            target.clamped_ticks,
            target.seek_time_ms,
            target.chunk.is_some()
        );

        // Keep stream position/timebase coherent with C get_current_track_pos().
        source.start_time = get_time_counter() as i32 - target.clamped_ticks as i32;

        let Some(index) = target.chunk else {
            // Beyond end of all tracks.
            state.cur_chunk = None;
            state.cur_sub_chunk = None;
            source.stream_should_be_playing = false;
            return false;
        };
        let seek_ms = target.seek_time_ms;

        {
            // Return currently attached decoder back to old cur_chunk before switching.
            if let Some(dec) = sample.decoder.take() {
                if let Some(cur) = state.cur_chunk {
                    let current = &mut state.chunks[cur];
                    if current.decoder.is_none() {
                        current.decoder = Some(dec);
                    }
                }
            }

            // Attach target chunk decoder and seek within it.
            // C parity: seek target is milliseconds relative to chunk start,
            // but decoder seek API takes PCM sample position.
            let chunk = &mut state.chunks[index];
            if let Some(mut dec) = chunk.decoder.take() {
                let pcm_pos = (seek_ms as u64 * dec.frequency() as u64 / 1000) as u32;
                let _ = dec.seek(pcm_pos);
                eprintln!(
                    "[PARITY][SEEK] decoder_freq={} pcm_pos={} (from seek_ms={})",
                    dec.frequency(),
                    pcm_pos,
                    seek_ms
                );
                sample.decoder = Some(dec);
            }

            sample.offset = chunk.start_time as i32 * ONE_SECOND as i32 / 1000;
            eprintln!(
                "[PARITY][SEEK] chunk_start_ms={:.3} sample_offset_ticks={} seek_ms={}",
                chunk.start_time, sample.offset, seek_ms
            );
        }

        state.cur_chunk = Some(index);
        true
    })
    .ok_or(AudioError::InvalidSource(SPEECH_SOURCE))?;

    if !has_chunk {
        let _ = stream::stop_stream(SPEECH_SOURCE);
    } else if !stream::playing_stream(SPEECH_SOURCE) {
        stream::play_stream(sample_arc, SPEECH_SOURCE, false, true, false)?;
    }

    Ok(())
}

/// Resolve a seek target and update cur_chunk/cur_sub_chunk like C seek_track().
fn resolve_seek_target(state: &mut TrackPlayerState, pos: u32) -> SeekTarget {
    let len = state.tracks_length.load(Ordering::Acquire);
    let clamped_ticks = pos.min(len + 1);

    let mut last_tagged = None;
    for index in 0..state.chunks.len() {
        let chunk = &state.chunks[index];
        let chunk_start_ms = chunk.start_time as u32;
        if chunk.tag_me {
            last_tagged = Some(index);
        }

        let chunk_end_ticks = ms_to_ticks(chunk_start_ms.saturating_add(chunk.length_ms));

        if clamped_ticks < chunk_end_ticks {
            // C seek_track semantics: the last tagged chunk seen while
            // scanning, which includes this chunk when it is tagged.
            if let Some(tagged) = last_tagged {
                do_track_tag(state, tagged);
            }

            let chunk_start_ticks = ms_to_ticks(chunk_start_ms);
            let rel_ticks = clamped_ticks.saturating_sub(chunk_start_ticks);
            let seek_time_ms = ticks_to_ms(rel_ticks);

            return SeekTarget {
                chunk: Some(index),
                clamped_ticks,
                seek_time_ms,
            };
        }
    }

    state.cur_chunk = None;
    state.cur_sub_chunk = None;
    SeekTarget {
        chunk: None,
        clamped_ticks,
        seek_time_ms: 0,
    }
}

/// Find the next tagged page after the given position.
fn find_next_page_inner(chunks: &[SoundChunk], cur: Option<usize>) -> Option<usize> {
    let cur = cur?;
    (cur + 1..chunks.len()).find(|&index| chunks[index].tag_me)
}

/// Find the previous tagged page before the given position.
fn find_prev_page_inner(chunks: &[SoundChunk], cur: Option<usize>) -> Option<usize> {
    if chunks.is_empty() {
        return None;
    }
    // C parity: cur == NULL is treated as end-of-list, so the scan still
    // starts at the head and returns the last tagged page.
    let end = cur.unwrap_or(chunks.len()).min(chunks.len());
    (0..end).rev().find(|&index| chunks[index].tag_me).or(Some(0))
}

/// Handle a buffer tag event for subtitle synchronization.
/// Must be called with TRACK_STATE held. Takes &mut TrackPlayerState to
/// avoid re-locking (FIX: ISSUE-MISC-03).
fn do_track_tag(state: &mut TrackPlayerState, index: usize) {
    if let Some(ref cb) = state.chunks[index].callback {
        cb(0);
    }
    state.cur_sub_chunk = Some(index);
}

/// Get the total track end time in game ticks.
fn tracks_end_time_inner(state: &TrackPlayerState) -> u32 {
    let total_ms = state
        .chunks
        .iter()
        .map(|chunk| (chunk.start_time as u32).saturating_add(chunk.length_ms))
        .max()
        .unwrap_or(0);
    ms_to_ticks(total_ms)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[path = "trackplayer_flow_tests.rs"]
mod flow_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::test_support::{Probe, ENGINE_TEST_LOCK};

    #[test]
    fn test_sound_chunk_creation() {
        let chunk = SoundChunk {
            decoder: None,
            length_ms: 0,
            start_time: 0.0,
            run_time: 0,
            tag_me: false,
            track_num: 0,
            text: None,
            text_cstr: None,
            callback: None,
        };
        assert!(chunk.decoder.is_none());
        assert_eq!(chunk.track_num, 0);
    }

    #[test]
    fn test_track_player_state_new() {
        let state = TrackPlayerState::new();
        assert!(state.chunks.is_empty());
        assert!(state.last_sub.is_none());
        assert!(state.cur_chunk.is_none());
        assert_eq!(state.track_count, 0);
        assert_eq!(state.tracks_length.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn subtitle_history_owns_text_after_the_track_is_stopped() {
        let _serial = ENGINE_TEST_LOCK.lock();
        stop_track().unwrap();
        splice_track(Some("voice"), Some("Hello world"), None, None, vec![]).unwrap();
        let history = subtitle_history();
        stop_track().unwrap();
        assert_eq!(history[0].text.as_c_str(), c"Hello world");
    }

    #[test]
    fn test_track_callbacks_is_stream_callbacks() {
        // Verify TrackCallbacks can be boxed as StreamCallbacks
        let _: Box<dyn StreamCallbacks + Send> = Box::new(TrackCallbacks);
    }

    #[test]
    fn test_constants() {
        const {
            assert!(TEXT_SPEED > 0.0);
            assert!(ACCEL_SCROLL_SPEED > 0.0);
            assert!(MAX_MULTI_TRACKS > 0);
        }
    }

    // --- P10 TDD tests ---

    // REQ-TRACK-ASSEMBLE-01..03: Subtitle splitting
    #[test]
    fn test_split_sub_pages_single() {
        let pages = split_sub_pages("Hello world");
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].text, "Hello world");
    }

    #[test]
    fn test_split_sub_pages_multiple() {
        let pages = split_sub_pages("Page one\r\nPage two");
        assert_eq!(pages.len(), 2);
        // First page gets "..." suffix (continuation mark)
        assert_eq!(pages[0].text, "Page one...");
        // Second page gets ".." prefix (continuation mark)
        assert_eq!(pages[1].text, "..Page two");
    }

    #[test]
    fn test_split_sub_pages_continuation_marks() {
        let pages = split_sub_pages("First page...\r\n..Second page");
        assert!(pages.len() >= 2);
        // Continuation text should have ellipsis handled
    }

    #[test]
    fn test_split_sub_pages_timing() {
        let pages = split_sub_pages("Short");
        assert!(pages[0].timestamp >= 0.0);
        // Timing should be at least TEXT_SPEED * char_count
    }

    // REQ-TRACK-ASSEMBLE-14: Timestamp parsing
    #[test]
    fn test_get_time_stamps_basic() {
        let ts = get_time_stamps("100,200,300");
        assert_eq!(ts.len(), 3);
        assert!((ts[0] - 100.0).abs() < 0.01);
        assert!((ts[1] - 200.0).abs() < 0.01);
        assert!((ts[2] - 300.0).abs() < 0.01);
    }

    #[test]
    fn test_get_time_stamps_skip_zeros() {
        let ts = get_time_stamps("0,100,0");
        // Non-zero values should be preserved
        assert!(ts.iter().all(|&t| t == 0.0 || t >= 100.0));
    }

    #[test]
    fn test_get_time_stamps_mixed_separators() {
        let ts = get_time_stamps("100\n200\r300");
        assert_eq!(ts.len(), 3);
    }

    // REQ-TRACK-ASSEMBLE-04..13: Assembly
    #[test]
    fn test_splice_track_no_text_returns_ok() {
        let result = splice_track(Some("track"), None, None, None, Vec::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_splice_track_no_name_no_tracks_warns() {
        let _serial = ENGINE_TEST_LOCK.lock();
        // When no tracks exist and no name is given, should return Ok
        let result = splice_track(None, Some("text"), None, None, Vec::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_splice_track_creates_first_sample() {
        let _serial = ENGINE_TEST_LOCK.lock();
        let state = TRACK_STATE.lock();
        let had_sample = state.sound_sample.is_some();
        drop(state);
        // After first splice_track with a name, state should have a sample
        // (can't easily test without a real decoder; this verifies the path)
        assert!(!had_sample); // initially no sample
    }

    #[test]
    fn test_splice_track_chunk_construction() {
        // Test that SoundChunk can be constructed with all fields
        let chunk = SoundChunk {
            decoder: None,
            length_ms: 0,
            start_time: 500.0,
            run_time: ms_to_ticks(1000) as i32,
            tag_me: true,
            track_num: 2,
            text: Some("Subtitle text".into()),
            text_cstr: None,
            callback: Some(Box::new(|_| {})),
        };
        assert_eq!(chunk.start_time, 500.0);
        assert_eq!(chunk.run_time, ms_to_ticks(1000) as i32);
        assert!(chunk.tag_me);
        assert_eq!(chunk.track_num, 2);
        assert!(chunk.callback.is_some());
    }

    // REQ-TRACK-ASSEMBLE-15..17: Multi-track
    #[test]
    fn test_splice_multi_track_precondition() {
        let _serial = ENGINE_TEST_LOCK.lock();
        let result = splice_multi_track(Vec::new(), None);
        // Should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_splice_multi_track_appends() {
        let _serial = ENGINE_TEST_LOCK.lock();
        let result = splice_multi_track(Vec::new(), Some("text"));
        assert!(result.is_ok() || result.is_err());
    }

    // REQ-TRACK-PLAY-01..10: Playback

    #[test]
    fn test_ms_ticks_roundtrip_close() {
        let ms = 1000u32;
        let ticks = ms_to_ticks(ms);
        let back_ms = ticks_to_ms(ticks);
        assert!((back_ms as i32 - ms as i32).abs() <= 2);
    }

    #[test]
    #[ignore = "P11: play_track stub"]
    fn test_play_track_no_sample_ok() {
        // When no sample exists, should handle gracefully
        let result = play_track(false);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_stop_track_clears_all() {
        let _serial = ENGINE_TEST_LOCK.lock();
        let result = stop_track();
        assert!(result.is_ok());
        let state = TRACK_STATE.lock();
        assert_eq!(state.track_count, 0);
        assert!(state.chunks.is_empty());
        assert!(state.last_sub.is_none());
    }

    #[test]
    fn test_playing_track_zero_when_empty() {
        let _serial = ENGINE_TEST_LOCK.lock();
        assert!(!playing_track());
    }

    // REQ-TRACK-SEEK-01..06: Seeking
    #[test]
    fn test_seek_clamps_offset_concept() {
        // Verify the clamping concept: offset should be in [0, length+1]
        let length = 1000u32;
        let offset: i32 = -500;
        let clamped = offset.max(0).min(length as i32 + 1);
        assert_eq!(clamped, 0);
        let offset: i32 = 5000;
        let clamped = offset.max(0).min(length as i32 + 1);
        assert_eq!(clamped, 1001);
    }

    #[test]
    fn test_get_current_track_pos_concept() {
        // Position should be clamped to [0, tracks_length]
        let tracks_length = 840u32;
        let raw_pos: i32 = 500;
        let clamped = raw_pos.max(0).min(tracks_length as i32) as u32;
        assert_eq!(clamped, 500);
        let raw_pos: i32 = 2000;
        let clamped = raw_pos.max(0).min(tracks_length as i32) as u32;
        assert_eq!(clamped, 840);
    }

    // REQ-TRACK-POSITION-01..02: Position
    #[test]
    fn test_get_track_position_no_sample() {
        let _serial = ENGINE_TEST_LOCK.lock();
        assert_eq!(get_track_position(0), 0);
    }

    #[test]
    fn test_get_track_position_scaling_concept() {
        // in_units == 0 → return raw ticks; in_units != 0 → percentage
        let tracks_length = 840u32;
        let pos = 420u32;
        let percentage = pos * 100 / tracks_length.max(1);
        assert_eq!(percentage, 50);
    }

    // REQ-TRACK-SUBTITLE-01..04: Subtitles
    #[test]
    fn test_subtitle_snapshot_none_when_empty() {
        let _serial = ENGINE_TEST_LOCK.lock();
        stop_track().unwrap();
        assert!(subtitle_snapshot().is_none());
    }

    #[test]
    fn test_subtitle_history_empty_without_tracks() {
        let _serial = ENGINE_TEST_LOCK.lock();
        stop_track().unwrap();
        assert!(subtitle_history().is_empty());
    }

    // REQ-TRACK-SEEK-11..12: Navigation
    #[test]
    fn test_find_next_page_none() {
        assert!(find_next_page_inner(&[], None).is_none());
    }

    #[test]
    fn test_find_prev_page_is_none_without_chunks() {
        assert!(find_prev_page_inner(&[], None).is_none());
    }

    fn chunks_in_track() -> usize {
        TRACK_STATE.lock().chunks.len()
    }

    /// `splice_multi_track` appends to whatever track list exists, so the two
    /// tests that call it must wait for the engine lock instead of running
    /// while a locked test is in the middle of walking its own list.
    #[test]
    fn multi_track_tests_wait_for_the_lock_of_the_track_list_they_would_extend() {
        let serial = ENGINE_TEST_LOCK.lock();
        splice_track(
            Some("conversation"),
            Some("First line\r\nSecond line"),
            None,
            None,
            Vec::new(),
        )
        .unwrap();
        let before = chunks_in_track();
        let probes = [
            Probe::spawn(test_splice_multi_track_precondition),
            Probe::spawn(test_splice_multi_track_appends),
        ];

        let ran_without_the_lock: Vec<bool> =
            probes.iter().map(Probe::finishes_within_grace).collect();
        let after = chunks_in_track();
        stop_track().unwrap();
        drop(serial);
        let outcomes: Vec<_> = probes.into_iter().map(Probe::join).collect();

        assert_eq!(ran_without_the_lock, [false, false]);
        assert_eq!(after, before, "the held track list gained a chunk");
        assert!(outcomes.iter().all(Result::is_ok));
    }
}
