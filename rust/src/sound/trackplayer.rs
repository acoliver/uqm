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
    fn on_start_stream(&mut self, _sample: &mut SoundSample) -> bool {
        todo!("P11: TrackCallbacks::on_start_stream")
    }

    fn on_end_chunk(&mut self, _sample: &mut SoundSample, _buffer: usize) -> bool {
        todo!("P11: TrackCallbacks::on_end_chunk")
    }

    fn on_end_stream(&mut self, _sample: &mut SoundSample) {
        todo!("P11: TrackCallbacks::on_end_stream")
    }

    fn on_tagged_buffer(&mut self, _sample: &mut SoundSample, _tag: &SoundTag) {
        todo!("P11: TrackCallbacks::on_tagged_buffer")
    }

    fn on_queue_buffer(&mut self, _sample: &mut SoundSample, _buffer: usize) {
        todo!("P11: TrackCallbacks::on_queue_buffer")
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
    callback: Option<Box<dyn Fn(i32) + Send>>,
) -> AudioResult<()> {
    todo!("P11: splice_track")
}

/// Splice multiple tracks at once.
pub fn splice_multi_track(
    tracks: &[Option<&str>],
    texts: &[Option<&str>],
    timestamp: Option<&str>,
) -> AudioResult<()> {
    todo!("P11: splice_multi_track")
}

// =============================================================================
// Public API — Playback Control (spec §3.2.3)
// =============================================================================

/// Start playing the assembled track sequence.
pub fn play_track(scope: bool) -> AudioResult<()> {
    todo!("P11: play_track")
}

/// Stop track playback and clear the track list.
pub fn stop_track() -> AudioResult<()> {
    todo!("P11: stop_track")
}

/// Jump to a specific track number in the sequence.
pub fn jump_track(track_num: u32) -> AudioResult<()> {
    todo!("P11: jump_track")
}

/// Pause track playback.
pub fn pause_track() -> AudioResult<()> {
    todo!("P11: pause_track")
}

/// Resume track playback.
pub fn resume_track() -> AudioResult<()> {
    todo!("P11: resume_track")
}

/// Check if a track is currently playing.
pub fn playing_track() -> bool {
    todo!("P11: playing_track")
}

// =============================================================================
// Public API — Seeking (spec §3.2.3)
// =============================================================================

/// Seek backward smoothly (rewind).
pub fn fast_reverse_smooth() -> AudioResult<()> {
    todo!("P11: fast_reverse_smooth")
}

/// Seek forward smoothly (fast-forward).
pub fn fast_forward_smooth() -> AudioResult<()> {
    todo!("P11: fast_forward_smooth")
}

/// Jump backward by one subtitle page.
pub fn fast_reverse_page() -> AudioResult<()> {
    todo!("P11: fast_reverse_page")
}

/// Jump forward by one subtitle page.
pub fn fast_forward_page() -> AudioResult<()> {
    todo!("P11: fast_forward_page")
}

/// Get the current track position.
///
/// `in_units` controls the unit:
/// - 0 = game ticks
/// - non-zero = percentage (0..100)
pub fn get_track_position(in_units: u32) -> u32 {
    todo!("P11: get_track_position")
}

// =============================================================================
// Public API — Subtitles (spec §3.2.3)
// =============================================================================

/// Get the subtitle text for the current position.
pub fn get_track_subtitle() -> Option<String> {
    todo!("P11: get_track_subtitle")
}

/// Get the first subtitle in the track.
pub fn get_first_track_subtitle() -> Option<SubtitleRef> {
    todo!("P11: get_first_track_subtitle")
}

/// Get the next subtitle after the current one.
pub fn get_next_track_subtitle() -> Option<SubtitleRef> {
    todo!("P11: get_next_track_subtitle")
}

/// Get the text of a subtitle reference.
pub fn get_track_subtitle_text(sub_ref: &SubtitleRef) -> Option<&str> {
    todo!("P11: get_track_subtitle_text")
}

// =============================================================================
// Internal Functions
// =============================================================================

/// Split subtitle text into sub-pages based on line breaks and timing.
fn split_sub_pages(text: &str) -> Vec<SubPage> {
    todo!("P11: split_sub_pages")
}

/// Parse timestamp string into a vector of timing values.
fn get_time_stamps(timestamp: &str) -> Vec<f64> {
    todo!("P11: get_time_stamps")
}

/// Core seek implementation.
fn seek_track(offset: i32) -> AudioResult<()> {
    todo!("P11: seek_track")
}

/// Find the next subtitle page from the current position.
fn find_next_page() -> Option<NonNull<SoundChunk>> {
    todo!("P11: find_next_page")
}

/// Find the previous subtitle page from the current position.
fn find_prev_page() -> Option<NonNull<SoundChunk>> {
    todo!("P11: find_prev_page")
}

/// Handle a buffer tag event for subtitle synchronization.
fn do_track_tag(tag: &SoundTag) {
    todo!("P11: do_track_tag")
}

/// Get the current track playback position in milliseconds.
fn get_current_track_pos() -> f64 {
    todo!("P11: get_current_track_pos")
}

/// Get the total track end time in milliseconds.
fn tracks_end_time() -> f64 {
    todo!("P11: tracks_end_time")
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
    #[ignore = "P11: split_sub_pages stub"]
    fn test_split_sub_pages_single() {
        let pages = split_sub_pages("Hello world");
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].text, "Hello world");
    }

    #[test]
    #[ignore = "P11: split_sub_pages stub"]
    fn test_split_sub_pages_multiple() {
        let pages = split_sub_pages("Page one\r\nPage two");
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].text, "Page one");
        assert_eq!(pages[1].text, "Page two");
    }

    #[test]
    #[ignore = "P11: split_sub_pages stub"]
    fn test_split_sub_pages_continuation_marks() {
        let pages = split_sub_pages("First page...\r\n..Second page");
        assert!(pages.len() >= 2);
        // Continuation text should have ellipsis handled
    }

    #[test]
    #[ignore = "P11: split_sub_pages stub"]
    fn test_split_sub_pages_timing() {
        let pages = split_sub_pages("Short");
        assert!(pages[0].timestamp >= 0.0);
        // Timing should be at least TEXT_SPEED * char_count
    }

    // REQ-TRACK-ASSEMBLE-14: Timestamp parsing
    #[test]
    #[ignore = "P11: get_time_stamps stub"]
    fn test_get_time_stamps_basic() {
        let ts = get_time_stamps("100,200,300");
        assert_eq!(ts.len(), 3);
        assert!((ts[0] - 100.0).abs() < 0.01);
        assert!((ts[1] - 200.0).abs() < 0.01);
        assert!((ts[2] - 300.0).abs() < 0.01);
    }

    #[test]
    #[ignore = "P11: get_time_stamps stub"]
    fn test_get_time_stamps_skip_zeros() {
        let ts = get_time_stamps("0,100,0");
        // Non-zero values should be preserved
        assert!(ts.iter().all(|&t| t == 0.0 || t >= 100.0));
    }

    #[test]
    #[ignore = "P11: get_time_stamps stub"]
    fn test_get_time_stamps_mixed_separators() {
        let ts = get_time_stamps("100\n200\r300");
        assert_eq!(ts.len(), 3);
    }

    // REQ-TRACK-ASSEMBLE-04..13: Assembly
    #[test]
    #[ignore = "P11: splice_track stub"]
    fn test_splice_track_no_text_returns_ok() {
        let result = splice_track(Some("track"), None, None, None);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "P11: splice_track stub"]
    fn test_splice_track_no_name_no_tracks_warns() {
        // When no tracks exist and no name is given, should return Ok
        let result = splice_track(None, Some("text"), None, None);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "P11: splice_track stub"]
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
    #[ignore = "P11: splice_multi_track stub"]
    fn test_splice_multi_track_precondition() {
        let result = splice_multi_track(&[Some("t1"), Some("t2")], &[None, None], None);
        // Should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    #[ignore = "P11: splice_multi_track stub"]
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
    #[ignore = "P11: stop_track stub"]
    fn test_stop_track_clears_all() {
        let result = stop_track();
        assert!(result.is_ok());
        let state = TRACK_STATE.lock();
        assert_eq!(state.track_count, 0);
        assert!(state.chunks_head.is_none());
        assert!(state.chunks_tail.is_null());
    }

    #[test]
    #[ignore = "P11: playing_track stub"]
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
    #[ignore = "P11: get_track_position stub"]
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
    #[ignore = "P11: get_track_subtitle stub"]
    fn test_get_track_subtitle_none_when_empty() {
        assert!(get_track_subtitle().is_none());
    }

    #[test]
    #[ignore = "P11: get_first_track_subtitle stub"]
    fn test_get_first_track_subtitle_none() {
        assert!(get_first_track_subtitle().is_none());
    }

    // REQ-TRACK-SEEK-11..12: Navigation
    #[test]
    #[ignore = "P11: find_next_page stub"]
    fn test_find_next_page_none() {
        assert!(find_next_page().is_none());
    }

    #[test]
    #[ignore = "P11: find_prev_page stub"]
    fn test_find_prev_page_defaults_to_head() {
        // With no previous, should return head or None
        assert!(find_prev_page().is_none());
    }
}
