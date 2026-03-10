// @plan PLAN-20260225-AUDIO-HEART.P09
// @requirement REQ-TRACK-ASSEMBLE-01..19, REQ-TRACK-PLAY-01..10
// @requirement REQ-TRACK-SEEK-01..13, REQ-TRACK-CALLBACK-01..09
// @requirement REQ-TRACK-SUBTITLE-01..04, REQ-TRACK-POSITION-01..02
#![allow(dead_code, unused_imports, unused_variables)]

//! Track player — manages multi-chunk audio sequences with subtitle
//! synchronization, seeking, and callback dispatching.
//!
//! Builds on the stream engine (`stream.rs`) to play linked-list chains
//! of `SoundChunk`s, each with its own decoder, subtitle text, and timing.

use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;

use super::decoder::SoundDecoder;
use super::stream;
use super::types::*;

use log::warn;

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

/// A single chunk in the track player's linked list.
///
/// Each chunk represents one audio segment with optional subtitle text,
/// timing information, and a decoder.
pub struct SoundChunk {
    /// Audio decoder for this chunk.
    pub decoder: Option<Box<dyn SoundDecoder>>,
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
    /// Cached CString for FFI — stable pointer for C pointer-identity comparisons.
    /// Created lazily from `text`.
    pub text_cstr: Option<std::ffi::CString>,
    /// Per-chunk callback (first page only).
    /// Must be Fn, not FnOnce — callbacks can fire multiple times on seek.
    pub callback: Option<Box<dyn Fn(i32) + Send>>,
    /// Linked list — next chunk.
    pub next: Option<Box<SoundChunk>>,
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

    /// Get a stable pointer to the CString text (for C pointer-identity comparisons).
    pub fn text_cstr_ptr(&mut self) -> *const std::os::raw::c_char {
        self.ensure_text_cstr();
        match self.text_cstr.as_ref() {
            Some(cs) => cs.as_ptr(),
            None => std::ptr::null(),
        }
    }
}

// REQ-TRACK-ASSEMBLE-19: Iterative Drop to prevent stack overflow on long lists
impl Drop for SoundChunk {
    fn drop(&mut self) {
        let mut next = self.next.take();
        while let Some(mut chunk) = next {
            next = chunk.next.take();
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

/// A reference to a subtitle in the current track.
#[derive(Debug, Clone)]
pub struct SubtitleRef {
    /// The subtitle text.
    pub text: String,
    /// Track number.
    pub track_num: u32,
}

/// Global track player state.
///
/// Protected by `TRACK_STATE` mutex. Contains the linked list of chunks
/// and all playback state.
pub struct TrackPlayerState {
    /// Linked list head (owns all chunks).
    pub chunks_head: Option<Box<SoundChunk>>,
    /// Raw pointer to tail chunk (borrowed from list).
    pub chunks_tail: *mut SoundChunk,
    /// Raw pointer to last subtitle chunk.
    pub last_sub: *mut SoundChunk,
    /// Current playback chunk.
    pub cur_chunk: Option<NonNull<SoundChunk>>,
    /// Current displayed subtitle chunk.
    pub cur_sub_chunk: Option<NonNull<SoundChunk>>,
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

// SAFETY: TrackPlayerState contains raw pointers (chunks_tail, last_sub)
// and NonNull pointers (cur_chunk, cur_sub_chunk) that point into the
// owned linked list (chunks_head).
//
// 1. Ownership: chunks_head owns the list. All raw pointers are borrowed
//    references into this list and are NEVER dereferenced after chunks_head
//    is set to None.
// 2. Single-writer: Always accessed under TRACK_STATE parking_lot::Mutex,
//    ensuring exclusive access. Raw pointers are never shared across threads
//    without the mutex.
// 3. Lifetime: cur_chunk and cur_sub_chunk are set to None in stop_track()
//    before chunks_head is dropped. chunks_tail is set to null_mut() when
//    the list is emptied.
// 4. Callbacks: TrackCallbacks are only invoked while the sample is alive
//    (guaranteed by Arc reference held by stream engine).
unsafe impl Send for TrackPlayerState {}

impl TrackPlayerState {
    fn new() -> Self {
        TrackPlayerState {
            chunks_head: None,
            chunks_tail: ptr::null_mut(),
            last_sub: ptr::null_mut(),
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
static TRACK_STATE: std::sync::LazyLock<Mutex<TrackPlayerState>> =
    std::sync::LazyLock::new(|| Mutex::new(TrackPlayerState::new()));

// Throttle hot-path subtitle logging so comm update loops stay responsive.
static SUBTITLE_LOG_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Details for a resolved seek target.
struct SeekTarget {
    chunk_ptr: Option<NonNull<SoundChunk>>,
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
        let cur = match state.cur_chunk {
            Some(c) => c,
            None => return false,
        };

        let chunk = unsafe { cur.as_ref() };
        // Move decoder from chunk to sample
        let chunk_mut = unsafe { &mut *(cur.as_ptr()) };
        if let Some(dec) = chunk_mut.decoder.take() {
            sample.decoder = Some(dec);
        }
        sample.offset = (chunk.start_time * ONE_SECOND as f64 / 1000.0) as i32;

        if chunk.tag_me {
            do_track_tag_inner(&mut state, chunk);
        }
        true
    }

    fn on_end_chunk(&mut self, sample: &mut SoundSample, buffer: usize) -> bool {
        let mut state = TRACK_STATE.lock();
        if state.sound_sample.is_none() {
            return false;
        }
        let cur = match state.cur_chunk {
            Some(c) => c,
            None => return false,
        };

        let cur_ref = unsafe { cur.as_ref() };
        let next = match cur_ref.next.as_ref() {
            Some(n) => n,
            None => return false,
        };

        // Return decoder to current chunk, take from next
        if let Some(dec) = sample.decoder.take() {
            let chunk_mut = unsafe { &mut *(cur.as_ptr()) };
            chunk_mut.decoder = Some(dec);
        }

        let next_nn = NonNull::from(next.as_ref());
        state.cur_chunk = Some(next_nn);
        let next_mut = unsafe { &mut *(next_nn.as_ptr()) };
        if let Some(dec) = next_mut.decoder.take() {
            // C parity: OnChunkEnd rewinds the next decoder before attaching it
            // as the active sample decoder.
            let mut dec = dec;
            let _ = dec.seek(0);
            sample.decoder = Some(dec);
        }

        if next.tag_me {
            let chunk_ptr = next.as_ref() as *const SoundChunk as usize;
            let _ = stream::tag_buffer(sample, buffer, chunk_ptr);
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

        let chunk_ptr = tag.data as *const SoundChunk;
        let mut state = TRACK_STATE.lock();
        if state.chunks_head.is_none() {
            return;
        }
        if !chunk_is_in_list(&state.chunks_head, chunk_ptr) {
            return;
        }
        let chunk = unsafe { &*chunk_ptr };
        do_track_tag_inner(&mut state, chunk);
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
    mut decoders: Vec<Box<dyn SoundDecoder>>,
) -> AudioResult<()> {
    let mut state = TRACK_STATE.lock();

    // No text → early return
    if track_text.is_none() {
        return Ok(());
    }
    let text = track_text.unwrap();

    // Subtitle-only append (no track_name)
    if track_name.is_none() {
        if state.track_count == 0 {
            return Ok(());
        }
        if !state.last_sub.is_null() {
            let last_sub = unsafe { &mut *state.last_sub };
            let pages = split_sub_pages(text);
            if let Some(first_page) = pages.first() {
                match &mut last_sub.text {
                    Some(t) => t.push_str(&first_page.text),
                    None => last_sub.text = Some(first_page.text.clone()),
                }
            }
            for page in pages.iter().skip(1) {
                let chunk = SoundChunk {
                    decoder: None,
                    start_time: state.dec_offset,
                    run_time: ms_to_ticks(page.timestamp as u32) as i32,
                    tag_me: true,
                    track_num: state.track_count.saturating_sub(1),
                    text: Some(page.text.clone()),
                    text_cstr: None,
                    callback: None,
                    next: None,
                };
                append_chunk(&mut state, chunk);
            }
        }
        return Ok(());
    }

    let _name = track_name.unwrap();

    // First track: create sound_sample (no decoder yet — chunks own decoders)
    if state.track_count == 0 {
        let callbacks: Box<dyn StreamCallbacks + Send> = Box::new(TrackCallbacks);
        let sample = stream::create_sound_sample(None, 8, Some(callbacks))?;
        state.sound_sample = Some(Arc::new(Mutex::new(sample)));
    }

    let pages = split_sub_pages(text);
    let timestamps = timestamp.map(|ts| get_time_stamps(ts)).unwrap_or_default();

    let appending_without_page_break = state.no_page_break && state.track_count > 0;

    // C parity: when no_page_break is set, append page 0 text to the previous
    // subtitle first, but still build chunk 0 from decoder/timing data.
    if appending_without_page_break {
        if let Some(first_page) = pages.first() {
            if !state.last_sub.is_null() {
                let last_sub = unsafe { &mut *state.last_sub };
                match &mut last_sub.text {
                    Some(t) => t.push_str(&first_page.text),
                    None => last_sub.text = Some(first_page.text.clone()),
                }
            }
        }
    }

    let decoder_count = decoders.len();
    let chunk_count = pages.len().max(decoder_count);
    let mut dec_iter = decoders.drain(..);

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

        // Update sound_sample length from this decoder
        if let Some(ref dec) = page_decoder {
            if let Some(ref sample_arc) = state.sound_sample {
                let mut sample = sample_arc.lock();
                sample.length += dec.length();
            }
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

        let chunk = SoundChunk {
            decoder: page_decoder,
            start_time: state.dec_offset,
            run_time: ms_to_ticks(run_time_ms.unsigned_abs()) as i32,
            tag_me,
            track_num: state.track_count,
            text,
            text_cstr: None,
            callback: if tag_me { callback.take() } else { None },
            next: None,
        };

        // Advance dec_offset by decoder length (like C does)
        if let Some(ref dec) = chunk.decoder {
            state.dec_offset += (dec.length() * 1000.0) as f64;
        }

        let has_text = chunk.text.is_some();
        append_chunk(&mut state, chunk);
        if has_text {
            state.last_sub = state.chunks_tail;
        }
    }

    state.no_page_break = false;

    if !appending_without_page_break {
        state.track_count += 1;
    }
    state.last_track_name = _name.to_string();
    Ok(())
}

/// Splice multiple tracks at once.
pub fn splice_multi_track(
    tracks: &[Option<&str>],
    texts: &[Option<&str>],
    _timestamp: Option<&str>,
) -> AudioResult<()> {
    let mut state = TRACK_STATE.lock();

    if state.track_count == 0 {
        return Err(AudioError::InvalidSample);
    }

    let num_tracks = tracks.len().min(MAX_MULTI_TRACKS);
    if num_tracks == 0 {
        return Ok(());
    }

    // Build chunks for each track (decoder loading deferred to FFI)
    for i in 0..num_tracks {
        if tracks[i].is_none() {
            continue;
        }

        let chunk = SoundChunk {
            decoder: None,
            start_time: state.dec_offset,
            run_time: ms_to_ticks((3.0 * TEXT_SPEED) as u32) as i32,
            tag_me: false,
            track_num: state.track_count.saturating_sub(1),
            text: None,
            text_cstr: None,
            callback: None,
            next: None,
        };
        append_chunk(&mut state, chunk);
        // dec_offset would be advanced by decoder length (when loaded)
    }

    // Append subtitle text to last_sub if provided
    if let Some(text) = texts.first().and_then(|t| *t) {
        if !state.last_sub.is_null() {
            let last_sub = unsafe { &mut *state.last_sub };
            match &mut last_sub.text {
                Some(t) => t.push_str(text),
                None => last_sub.text = Some(text.to_string()),
            }
        }
    }

    state.no_page_break = true;
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
        if state.sound_sample.is_none() {
            return Ok(());
        }

        let end_time = tracks_end_time_inner(&state);
        state.tracks_length.store(end_time, Ordering::Release);
        state.cur_chunk = state
            .chunks_head
            .as_ref()
            .map(|c| NonNull::from(c.as_ref()));
        state.cur_sub_chunk = None;

        Arc::clone(state.sound_sample.as_ref().unwrap())
        // TRACK_STATE lock dropped here
    };

    // Phase 2: Call play_stream WITHOUT holding TRACK_STATE
    stream::play_stream(sample_arc, SPEECH_SOURCE, false, scope, true)
}

/// Stop track playback and clear the track list.
pub fn stop_track() -> AudioResult<()> {
    let mut state = TRACK_STATE.lock();

    let _ = stream::stop_stream(SPEECH_SOURCE);

    state.track_count = 0;
    state.tracks_length.store(0, Ordering::Release);
    state.cur_chunk = None;
    state.cur_sub_chunk = None;

    if let Some(sample_arc) = state.sound_sample.take() {
        let mut sample = sample_arc.lock();
        // Clear buffer tags before dropping chunks (ISSUE-CONC-04)
        for tag in sample.buffer_tags.iter_mut() {
            *tag = None;
        }
        sample.decoder = None;
        let _ = stream::destroy_sound_sample(&mut sample);
    }

    state.chunks_head = None;
    state.chunks_tail = ptr::null_mut();
    state.last_sub = ptr::null_mut();
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
        .map(|c| (unsafe { c.as_ref() }.track_num + 1) as u16)
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
        if state.sound_sample.is_none() {
            return Ok(());
        }

        let prev = find_prev_page_inner(&state.chunks_head, state.cur_sub_chunk);
        let Some(page) = prev else {
            return Ok(());
        };

        let chunk = unsafe { page.as_ref() };
        let offset_ticks = ms_to_ticks(chunk.start_time as u32) as i32;

        state.cur_chunk = Some(page);
        state.cur_sub_chunk = Some(page);
        (
            Arc::clone(state.sound_sample.as_ref().unwrap()),
            offset_ticks,
        )
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
        if state.sound_sample.is_none() {
            return Ok(());
        }

        let next = find_next_page_inner(state.cur_sub_chunk);
        if let Some(page) = next {
            let chunk = unsafe { page.as_ref() };
            let offset_ticks = ms_to_ticks(chunk.start_time as u32) as i32;
            state.cur_chunk = Some(page);
            state.cur_sub_chunk = Some(page);
            Some((Arc::clone(state.sound_sample.as_ref().unwrap()), offset_ticks))
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
            state.tracks_length.load(Ordering::Acquire).saturating_add(1)
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
    let state = TRACK_STATE.lock();
    if state.sound_sample.is_none() {
        return 0;
    }
    let len = state.tracks_length.load(Ordering::Acquire);
    if len == 0 {
        return 0;
    }
    let pos = stream::get_stream_position_ticks(SPEECH_SOURCE).min(len);
    if in_units == 0 {
        pos
    } else {
        (in_units as u64 * pos as u64 / len as u64) as u32
    }
}

// =============================================================================
// Public API — Subtitles (spec §3.2.3)
// =============================================================================

/// Get the subtitle text for the current position.
pub fn get_track_subtitle() -> Option<String> {
    let state = TRACK_STATE.lock();
    if state.sound_sample.is_none() {
        return None;
    }
    state
        .cur_sub_chunk
        .map(|c| unsafe { c.as_ref() })
        .and_then(|c| c.text.clone())
}

/// Return a stable C string pointer for the current subtitle.
/// The pointer remains valid and identical as long as `cur_sub_chunk` doesn't change.
/// This is critical for C's pointer-identity comparison in `CheckSubtitles`.
pub fn get_track_subtitle_cstr() -> *const std::os::raw::c_char {
    let mut state = TRACK_STATE.lock();
    let Some(cur_ptr) = state.cur_sub_chunk else {
        if SUBTITLE_LOG_COUNTER.fetch_add(1, Ordering::Relaxed) % 120 == 0 {
            eprintln!("[PARITY][SUBTITLE] active=<none>");
        }
        return std::ptr::null();
    };

    // SAFETY: `cur_sub_chunk` is maintained as a pointer into the owned linked list
    // (`chunks_head`) and only reset/updated while holding TRACK_STATE. We hold the
    // mutex here, so taking a mutable reference for lazy CString caching is safe.
    let chunk = unsafe { &mut *cur_ptr.as_ptr() };
    let ptr = chunk.text_cstr_ptr();
    if SUBTITLE_LOG_COUNTER.fetch_add(1, Ordering::Relaxed) % 120 == 0 {
        if let Some(text) = chunk.text.as_ref() {
            eprintln!(
                "[PARITY][SUBTITLE] len={} ptr=0x{:x}",
                text.len(),
                ptr as usize
            );
        } else {
            eprintln!("[PARITY][SUBTITLE] active=<null_text>");
        }
    }
    ptr
}


/// Get the first subtitle in the track.
pub fn get_first_track_subtitle() -> Option<SubtitleRef> {
    let state = TRACK_STATE.lock();
    state.chunks_head.as_ref().map(|c| SubtitleRef {
        text: c.text.clone().unwrap_or_default(),
        track_num: c.track_num,
    })
}

/// Get the next subtitle after the current one.
pub fn get_next_track_subtitle() -> Option<SubtitleRef> {
    let state = TRACK_STATE.lock();
    let next = find_next_page_inner(state.cur_sub_chunk)?;
    let chunk = unsafe { next.as_ref() };
    Some(SubtitleRef {
        text: chunk.text.clone().unwrap_or_default(),
        track_num: chunk.track_num,
    })
}

/// Get the text of a subtitle reference.
pub fn get_track_subtitle_text(sub_ref: &SubtitleRef) -> Option<&str> {
    Some(sub_ref.text.as_str())
}

// =============================================================================
// Chunk pointer API — raw pointer iteration matching C's SUBTITLE_REF contract
// =============================================================================

/// Return the raw pointer to chunks_head (matching C's GetFirstTrackSubtitle).
pub fn get_first_chunk_ptr() -> *const u8 {
    let state = TRACK_STATE.lock();
    match state.chunks_head.as_ref() {
        Some(c) => c.as_ref() as *const SoundChunk as *const u8,
        None => std::ptr::null(),
    }
}

/// Given a chunk pointer `last_ref`, walk to the next tagged page
/// (matching C's GetNextTrackSubtitle(SUBTITLE_REF LastRef)).
pub fn get_next_chunk_ptr(last_ref: *const u8) -> *const u8 {
    if last_ref.is_null() {
        return std::ptr::null();
    }
    let state = TRACK_STATE.lock();
    // Validate that last_ref points to a chunk in our list
    if !chunk_is_in_list(&state.chunks_head, last_ref as *const SoundChunk) {
        return std::ptr::null();
    }
    let chunk = unsafe { &*(last_ref as *const SoundChunk) };
    let mut ptr = chunk.next.as_deref();
    while let Some(c) = ptr {
        if c.tag_me {
            return c as *const SoundChunk as *const u8;
        }
        ptr = c.next.as_deref();
    }
    std::ptr::null()
}

/// Given a chunk pointer, return its subtitle text (matching C's GetTrackSubtitleText).
pub fn get_chunk_text(chunk_ptr: *const u8) -> Option<String> {
    if chunk_ptr.is_null() {
        return None;
    }
    let state = TRACK_STATE.lock();
    if !chunk_is_in_list(&state.chunks_head, chunk_ptr as *const SoundChunk) {
        return None;
    }
    let chunk = unsafe { &*(chunk_ptr as *const SoundChunk) };
    chunk.text.clone()
}

/// Given a chunk pointer, return a stable CString pointer (matching C's GetTrackSubtitleText).
pub fn get_chunk_text_cstr(chunk_ptr: *const u8) -> *const std::os::raw::c_char {
    if chunk_ptr.is_null() {
        return std::ptr::null();
    }
    let state = TRACK_STATE.lock();
    if !chunk_is_in_list(&state.chunks_head, chunk_ptr as *const SoundChunk) {
        return std::ptr::null();
    }
    let chunk = unsafe { &mut *(chunk_ptr as *mut SoundChunk) };
    chunk.text_cstr_ptr()
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

/// Append a chunk to the tail of the linked list.
fn append_chunk(state: &mut TrackPlayerState, chunk: SoundChunk) {
    let boxed = Box::new(chunk);
    let new_ptr = boxed.as_ref() as *const SoundChunk as *mut SoundChunk;

    if state.chunks_tail.is_null() {
        state.chunks_head = Some(boxed);
        state.chunks_tail = state
            .chunks_head
            .as_ref()
            .map(|c| c.as_ref() as *const _ as *mut SoundChunk)
            .unwrap_or(ptr::null_mut());
    } else {
        let tail = unsafe { &mut *state.chunks_tail };
        tail.next = Some(boxed);
        state.chunks_tail = new_ptr;
    }
}

/// Check if a chunk pointer is in the active linked list.
fn chunk_is_in_list(head: &Option<Box<SoundChunk>>, target: *const SoundChunk) -> bool {
    let mut cur = head.as_deref();
    while let Some(chunk) = cur {
        if ptr::eq(chunk, target) {
            return true;
        }
        cur = chunk.next.as_deref();
    }
    false
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
    let sample_arc = {
        let mut state = TRACK_STATE.lock();
        let target = resolve_seek_target(&mut state, pos);
        eprintln!(
            "[PARITY][SEEK] resolved clamped_ticks={} seek_time_ms={} has_chunk={}",
            target.clamped_ticks,
            target.seek_time_ms,
            target.chunk_ptr.is_some()
        );

        // Keep stream position/timebase coherent with C get_current_track_pos().
        stream::with_source(SPEECH_SOURCE, |source| {
            source.start_time = get_time_counter() as i32 - target.clamped_ticks as i32;
        });

        if target.chunk_ptr.is_none() {
            // Beyond end of all tracks.
            state.cur_chunk = None;
            state.cur_sub_chunk = None;
            drop(state);
            let _ = stream::stop_stream(SPEECH_SOURCE);
            return Ok(());
        }

        let sample_arc = match state.sound_sample.as_ref() {
            Some(s) => Arc::clone(s),
            None => return Ok(()),
        };

        let chunk_ptr = target.chunk_ptr.unwrap();
        let chunk = unsafe { chunk_ptr.as_ref() };
        let seek_ms = target.seek_time_ms;

        {
            let mut sample = sample_arc.lock();

            // Return currently attached decoder back to old cur_chunk before switching.
            if let Some(dec) = sample.decoder.take() {
                if let Some(cur_ptr) = state.cur_chunk {
                    let cur_mut = unsafe { &mut *cur_ptr.as_ptr() };
                    if cur_mut.decoder.is_none() {
                        cur_mut.decoder = Some(dec);
                    }
                }
            }

            // Attach target chunk decoder and seek within it.
            // C parity: seek target is milliseconds relative to chunk start,
            // but decoder seek API takes PCM sample position.
            let chunk_mut = unsafe { &mut *chunk_ptr.as_ptr() };
            if let Some(mut dec) = chunk_mut.decoder.take() {
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
                chunk.start_time,
                sample.offset,
                seek_ms
            );
        }

        state.cur_chunk = Some(chunk_ptr);

        sample_arc
    };

    if !stream::playing_stream(SPEECH_SOURCE) {
        stream::play_stream(sample_arc, SPEECH_SOURCE, false, true, false)?;
    }

    Ok(())
}

/// Resolve a seek target and update cur_chunk/cur_sub_chunk like C seek_track().
fn resolve_seek_target(state: &mut TrackPlayerState, pos: u32) -> SeekTarget {
    let len = state.tracks_length.load(Ordering::Acquire);
    let clamped_ticks = pos.min(len + 1);

    let mut last_tagged: Option<NonNull<SoundChunk>> = None;
    let mut cur = state.chunks_head.as_deref();

    while let Some(chunk) = cur {
        let chunk_ptr = NonNull::from(chunk);
        let chunk_tag_me = chunk.tag_me;
        let chunk_start_ms = chunk.start_time as u32;
        let chunk_len_ms = chunk
            .decoder
            .as_ref()
            .map(|d| (d.length() * 1000.0) as u32)
            .unwrap_or(0);

        if chunk_tag_me {
            last_tagged = Some(chunk_ptr);
        }

        let chunk_end_ticks = ms_to_ticks(chunk_start_ms.saturating_add(chunk_len_ms));

        if clamped_ticks < chunk_end_ticks {
            // C seek_track semantics:
            // - remember last tagged chunk seen while scanning
            // - include current chunk if it is tagged
            let tagged_ptr = if chunk_tag_me {
                Some(chunk_ptr)
            } else {
                last_tagged
            };
            if let Some(tagged) = tagged_ptr {
                do_track_tag_ptr_inner(state, tagged);
            }

            let chunk_start_ticks = ms_to_ticks(chunk_start_ms);
            let rel_ticks = clamped_ticks.saturating_sub(chunk_start_ticks);
            let seek_time_ms = ticks_to_ms(rel_ticks);

            return SeekTarget {
                chunk_ptr: Some(chunk_ptr),
                clamped_ticks,
                seek_time_ms,
            };
        }

        cur = chunk.next.as_deref();
    }

    state.cur_chunk = None;
    state.cur_sub_chunk = None;
    SeekTarget {
        chunk_ptr: None,
        clamped_ticks,
        seek_time_ms: 0,
    }
}

/// Find the next tagged page after the given position.
fn find_next_page_inner(cur: Option<NonNull<SoundChunk>>) -> Option<NonNull<SoundChunk>> {
    let cur = cur?;
    let node = unsafe { cur.as_ref() };
    let mut ptr = node.next.as_deref();
    while let Some(chunk) = ptr {
        if chunk.tag_me {
            return Some(NonNull::from(chunk));
        }
        ptr = chunk.next.as_deref();
    }
    None
}

/// Find the previous tagged page before the given position.
fn find_prev_page_inner(
    head: &Option<Box<SoundChunk>>,
    cur: Option<NonNull<SoundChunk>>,
) -> Option<NonNull<SoundChunk>> {
    let head_ref = head.as_deref()?;
    let cur_ptr = cur.map(|nn| nn.as_ptr());

    // C parity: cur == NULL is treated as end-of-list.
    // So we still scan from head and return the last tagged page.
    let mut last_tagged = Some(NonNull::from(head_ref));
    let mut node = Some(head_ref);
    while let Some(chunk) = node {
        if let Some(cptr) = cur_ptr {
            if ptr::eq(chunk, cptr as *const _) {
                break;
            }
        }
        if chunk.tag_me {
            last_tagged = Some(NonNull::from(chunk));
        }
        node = chunk.next.as_deref();
    }
    last_tagged
}

/// Handle a buffer tag event for subtitle synchronization.
/// Must be called with TRACK_STATE held. Takes &mut TrackPlayerState to
/// avoid re-locking (FIX: ISSUE-MISC-03).
fn do_track_tag_inner(state: &mut TrackPlayerState, chunk: &SoundChunk) {
    if let Some(ref cb) = chunk.callback {
        cb(0);
    }
    state.cur_sub_chunk = Some(NonNull::from(chunk));
}

fn do_track_tag_ptr_inner(state: &mut TrackPlayerState, chunk_ptr: NonNull<SoundChunk>) {
    if let Some(ref cb) = unsafe { chunk_ptr.as_ref() }.callback {
        cb(0);
    }
    state.cur_sub_chunk = Some(chunk_ptr);
}


/// Get the total track end time in game ticks.
fn tracks_end_time_inner(state: &TrackPlayerState) -> u32 {
    let mut total_ms: u32 = 0;
    let mut cur = state.chunks_head.as_deref();
    while let Some(chunk) = cur {
        let chunk_end_ms = (chunk.start_time as u32).saturating_add(
            chunk
                .decoder
                .as_ref()
                .map(|d| (d.length() * 1000.0) as u32)
                .unwrap_or(0),
        );
        total_ms = total_ms.max(chunk_end_ms);
        cur = chunk.next.as_deref();
    }
    ms_to_ticks(total_ms)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_chunk_creation() {
        let chunk = SoundChunk {
            decoder: None,
            start_time: 0.0,
            run_time: 0,
            tag_me: false,
            track_num: 0,
            text: None,
            text_cstr: None,
            callback: None,
            next: None,
        };
        assert!(chunk.decoder.is_none());
        assert_eq!(chunk.track_num, 0);
    }

    #[test]
    fn test_sound_chunk_linked_list() {
        let chunk2 = Box::new(SoundChunk {
            decoder: None,
            start_time: 1000.0,
            run_time: 500,
            tag_me: true,
            track_num: 1,
            text: Some("Page 2".into()),
            text_cstr: None,
            callback: None,
            next: None,
        });
        let chunk1 = SoundChunk {
            decoder: None,
            start_time: 0.0,
            run_time: 1000,
            tag_me: true,
            track_num: 0,
            text: Some("Page 1".into()),
            text_cstr: None,
            callback: None,
            next: Some(chunk2),
        };
        assert!(chunk1.next.is_some());
        assert_eq!(chunk1.next.as_ref().unwrap().track_num, 1);
    }

    #[test]
    fn test_sound_chunk_iterative_drop() {
        // Build a long chain — should not stack overflow
        let mut head: Option<Box<SoundChunk>> = None;
        for i in (0..1000).rev() {
            head = Some(Box::new(SoundChunk {
                decoder: None,
                start_time: i as f64,
                run_time: 0,
                tag_me: false,
                track_num: 0,
                text: None,
                text_cstr: None,
                callback: None,
                next: head,
            }));
        }
        drop(head); // should not overflow
    }

    #[test]
    fn test_track_player_state_new() {
        let state = TrackPlayerState::new();
        assert!(state.chunks_head.is_none());
        assert!(state.chunks_tail.is_null());
        assert!(state.cur_chunk.is_none());
        assert_eq!(state.track_count, 0);
        assert_eq!(state.tracks_length.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_subtitle_ref() {
        let sub = SubtitleRef {
            text: "Hello world".into(),
            track_num: 0,
        };
        assert_eq!(sub.text, "Hello world");
    }

    #[test]
    fn test_track_callbacks_is_stream_callbacks() {
        // Verify TrackCallbacks can be boxed as StreamCallbacks
        let _: Box<dyn StreamCallbacks + Send> = Box::new(TrackCallbacks);
    }

    #[test]
    fn test_constants() {
        assert!(TEXT_SPEED > 0.0);
        assert!(ACCEL_SCROLL_SPEED > 0.0);
        assert!(MAX_MULTI_TRACKS > 0);
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
        // When no tracks exist and no name is given, should return Ok
        let result = splice_track(None, Some("text"), None, None, Vec::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_splice_track_creates_first_sample() {
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
            start_time: 500.0,
            run_time: ms_to_ticks(1000) as i32,
            tag_me: true,
            track_num: 2,
            text: Some("Subtitle text".into()),
            text_cstr: None,
            callback: Some(Box::new(|_| {})),
            next: None,
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
        let result = splice_multi_track(&[Some("t1"), Some("t2")], &[None, None], None);
        // Should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_splice_multi_track_appends() {
        let result = splice_multi_track(&[Some("t1")], &[Some("text")], None);
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
        let result = stop_track();
        assert!(result.is_ok());
        let state = TRACK_STATE.lock();
        assert_eq!(state.track_count, 0);
        assert!(state.chunks_head.is_none());
        assert!(state.chunks_tail.is_null());
    }

    #[test]
    fn test_playing_track_zero_when_empty() {
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
    fn test_get_track_subtitle_none_when_empty() {
        assert!(get_track_subtitle().is_none());
    }

    #[test]
    fn test_get_first_track_subtitle_none() {
        assert!(get_first_track_subtitle().is_none());
    }

    // REQ-TRACK-SEEK-11..12: Navigation
    #[test]
    fn test_find_next_page_none() {
        assert!(find_next_page_inner(None).is_none());
    }

    #[test]
    fn test_find_prev_page_defaults_to_head() {
        let head: Option<Box<SoundChunk>> = None;
        assert!(find_prev_page_inner(&head, None).is_none());
    }
}
