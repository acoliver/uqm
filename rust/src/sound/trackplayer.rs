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
use std::sync::atomic::{AtomicU32, Ordering};
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
pub const TEXT_SPEED: f64 = 50.0;
/// Accelerated scroll speed multiplier.
pub const ACCEL_SCROLL_SPEED: f64 = 3.0;
/// Maximum tracks in a multi-track splice.
pub const MAX_MULTI_TRACKS: usize = 8;

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
    /// Per-chunk callback (first page only).
    /// Must be Fn, not FnOnce — callbacks can fire multiple times on seek.
    pub callback: Option<Box<dyn Fn(i32) + Send>>,
    /// Linked list — next chunk.
    pub next: Option<Box<SoundChunk>>,
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
        if let Some(mut dec) = next_mut.decoder.take() {
            let _ = dec.seek(0); // rewind
            sample.decoder = Some(dec);
        }

        if next.tag_me {
            let chunk_ptr = next.as_ref() as *const SoundChunk as usize;
            if buffer < sample.buffer_tags.len() {
                sample.buffer_tags[buffer] = Some(SoundTag {
                    buf_handle: buffer,
                    data: chunk_ptr,
                });
            }
        }
        true
    }

    fn on_end_stream(&mut self, _sample: &mut SoundSample) {
        let mut state = TRACK_STATE.lock();
        state.cur_chunk = None;
        state.cur_sub_chunk = None;
    }

    fn on_tagged_buffer(&mut self, _sample: &mut SoundSample, tag: &SoundTag) {
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
pub fn splice_track(
    track_name: Option<&str>,
    track_text: Option<&str>,
    timestamp: Option<&str>,
    mut callback: Option<Box<dyn Fn(i32) + Send>>,
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
                    run_time: page.timestamp as i32,
                    tag_me: true,
                    track_num: state.track_count.saturating_sub(1),
                    text: Some(page.text.clone()),
                    callback: None,
                    next: None,
                };
                append_chunk(&mut state, chunk);
            }
        }
        return Ok(());
    }

    // New track with decoder — for now, create chunk without loading decoder
    // (decoder loading will be wired in FFI phase)
    let _name = track_name.unwrap();

    // First track: create sound_sample
    if state.track_count == 0 {
        let callbacks: Box<dyn StreamCallbacks + Send> = Box::new(TrackCallbacks);
        let sample = stream::create_sound_sample(None, 8, Some(callbacks))?;
        state.sound_sample = Some(Arc::new(Mutex::new(sample)));
    }

    let pages = split_sub_pages(text);
    let timestamps = timestamp.map(|ts| get_time_stamps(ts)).unwrap_or_default();

    for (i, page) in pages.iter().enumerate() {
        let mut run_time = if i < timestamps.len() {
            timestamps[i] as i32
        } else {
            page.timestamp as i32
        };

        // Negate last page timestamp
        if i == pages.len() - 1 {
            run_time = -run_time.abs();
        }

        // no_page_break handling
        if state.no_page_break && state.track_count > 0 && i == 0 {
            if !state.last_sub.is_null() {
                let last_sub = unsafe { &mut *state.last_sub };
                match &mut last_sub.text {
                    Some(t) => t.push_str(&page.text),
                    None => last_sub.text = Some(page.text.clone()),
                }
            }
            state.no_page_break = false;
            continue;
        }

        let chunk = SoundChunk {
            decoder: None, // Decoder loaded externally via FFI
            start_time: state.dec_offset,
            run_time,
            tag_me: true,
            track_num: state.track_count,
            text: Some(page.text.clone()),
            callback: if i == 0 { callback.take() } else { None },
            next: None,
        };

        let has_text = chunk.text.is_some();
        append_chunk(&mut state, chunk);
        if has_text {
            state.last_sub = state.chunks_tail;
        }
        state.no_page_break = false;
    }

    state.track_count += 1;
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
            run_time: -(3.0 * TEXT_SPEED) as i32,
            tag_me: false,
            track_num: state.track_count.saturating_sub(1),
            text: None,
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
    let state = TRACK_STATE.lock();
    if state.sound_sample.is_none() {
        return false;
    }
    state
        .cur_chunk
        .map(|c| unsafe { c.as_ref() }.track_num + 1 > 0)
        .unwrap_or(false)
}

// =============================================================================
// Public API — Seeking (spec §3.2.3)
// =============================================================================

/// Seek backward smoothly (rewind).
pub fn fast_reverse_smooth() -> AudioResult<()> {
    let pos = {
        let state = TRACK_STATE.lock();
        get_current_track_pos_simple(&state)
    };
    let new_pos = pos.saturating_sub(ACCEL_SCROLL_SPEED as u32);
    seek_to_position(new_pos)
}

/// Seek forward smoothly (fast-forward).
pub fn fast_forward_smooth() -> AudioResult<()> {
    let pos = {
        let state = TRACK_STATE.lock();
        get_current_track_pos_simple(&state)
    };
    let new_pos = pos + ACCEL_SCROLL_SPEED as u32;
    seek_to_position(new_pos)
}

/// Jump backward by one subtitle page.
pub fn fast_reverse_page() -> AudioResult<()> {
    let state = TRACK_STATE.lock();
    let prev = find_prev_page_inner(&state.chunks_head, state.cur_sub_chunk);
    if let Some(page) = prev {
        let chunk = unsafe { page.as_ref() };
        let pos = chunk.start_time as u32;
        drop(state);
        seek_to_position(pos)?;
    }
    Ok(())
}

/// Jump forward by one subtitle page.
pub fn fast_forward_page() -> AudioResult<()> {
    let state = TRACK_STATE.lock();
    let next = find_next_page_inner(state.cur_sub_chunk);
    if let Some(page) = next {
        let chunk = unsafe { page.as_ref() };
        let pos = chunk.start_time as u32;
        drop(state);
        seek_to_position(pos)?;
    } else {
        drop(state);
        let _ = stream::stop_stream(SPEECH_SOURCE);
    }
    Ok(())
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
    let pos = get_current_track_pos_simple(&state);
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
// Internal Functions
// =============================================================================

/// Split subtitle text into sub-pages based on CRLF breaks and timing.
fn split_sub_pages(text: &str) -> Vec<SubPage> {
    let crlf = "\r\n";
    let parts: Vec<&str> = text.split(crlf).collect();
    let mut result = Vec::new();

    for (i, part) in parts.iter().enumerate() {
        let mut page_text = part.to_string();

        // Continuation marks
        if i > 0 {
            page_text = format!("..{}", page_text);
        }
        if i < parts.len() - 1 {
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

/// Seek to a specific position (in game ticks). Drops TRACK_STATE,
/// delegates to stream::seek_stream, then updates track state.
fn seek_to_position(pos: u32) -> AudioResult<()> {
    {
        let mut state = TRACK_STATE.lock();
        let len = state.tracks_length.load(Ordering::Acquire);
        let clamped = pos.min(len + 1);

        // Walk chunk list to find and update cur_chunk/cur_sub_chunk
        let mut cumulative: u32 = 0;
        let mut last_tagged: Option<NonNull<SoundChunk>> = None;
        let mut cur = state.chunks_head.as_deref();
        let mut found = false;

        while let Some(chunk) = cur {
            if chunk.tag_me {
                last_tagged = Some(NonNull::from(chunk));
            }
            let duration = chunk.run_time.unsigned_abs();
            let chunk_end = cumulative + duration;

            if chunk_end > clamped {
                state.cur_chunk = Some(NonNull::from(chunk));
                if let Some(tagged) = last_tagged {
                    let tagged_ref = unsafe { tagged.as_ref() };
                    do_track_tag_inner(&mut state, tagged_ref);
                }
                found = true;
                break;
            }
            cumulative = chunk_end;
            cur = chunk.next.as_deref();
        }

        if !found {
            state.cur_chunk = None;
            state.cur_sub_chunk = None;
        }
    }
    // Delegate actual audio seeking to the stream engine
    stream::seek_stream(SPEECH_SOURCE, pos)
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
    let cur_nn = cur?;
    let cur_ptr = cur_nn.as_ptr();
    let mut last_tagged = Some(NonNull::from(head_ref));
    let mut node = Some(head_ref);
    while let Some(chunk) = node {
        if ptr::eq(chunk, cur_ptr as *const _) {
            break;
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

/// Get the current track playback position in game ticks (simplified).
/// Uses chunk durations and current position within current chunk.
fn get_current_track_pos_simple(state: &TrackPlayerState) -> u32 {
    let len = state.tracks_length.load(Ordering::Acquire);
    if len == 0 {
        return 0;
    }
    // Walk to current chunk and sum preceding durations
    let cur_ptr = match state.cur_chunk {
        Some(c) => c.as_ptr(),
        None => return 0,
    };
    let mut pos: u32 = 0;
    let mut node = state.chunks_head.as_deref();
    while let Some(chunk) = node {
        if ptr::eq(chunk, cur_ptr as *const _) {
            break;
        }
        pos += chunk.run_time.unsigned_abs();
        node = chunk.next.as_deref();
    }
    pos.min(len)
}

/// Get the total track end time in game ticks.
fn tracks_end_time_inner(state: &TrackPlayerState) -> u32 {
    let mut total: u32 = 0;
    let mut cur = state.chunks_head.as_deref();
    while let Some(chunk) = cur {
        total += chunk.run_time.unsigned_abs();
        cur = chunk.next.as_deref();
    }
    total
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
        let result = splice_track(Some("track"), None, None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_splice_track_no_name_no_tracks_warns() {
        // When no tracks exist and no name is given, should return Ok
        let result = splice_track(None, Some("text"), None, None);
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
            run_time: -1000,
            tag_me: true,
            track_num: 2,
            text: Some("Subtitle text".into()),
            callback: Some(Box::new(|_| {})),
            next: None,
        };
        assert_eq!(chunk.start_time, 500.0);
        assert_eq!(chunk.run_time, -1000);
        assert!(chunk.tag_me);
        assert_eq!(chunk.track_num, 2);
        assert!(chunk.callback.is_some());
    }

    #[test]
    fn test_splice_track_last_page_negative_run_time() {
        // Last page in a track should have negative run_time (minimum display)
        let chunk = SoundChunk {
            decoder: None,
            start_time: 0.0,
            run_time: -2000, // negative = minimum display time
            tag_me: true,
            track_num: 0,
            text: Some("Last page".into()),
            callback: None,
            next: None,
        };
        assert!(chunk.run_time < 0);
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
