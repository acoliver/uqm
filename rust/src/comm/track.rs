//! Audio track management for speech playback
//!
//! Handles speech track loading, playback, and chunk management.

/// A sound chunk in the speech track
#[derive(Debug, Clone)]
pub struct SoundChunk {
    /// Audio data handle (decoder ID or buffer reference)
    pub audio_handle: u32,
    /// Start time in seconds
    pub start_time: f32,
    /// Duration in seconds
    pub duration: f32,
    /// Optional subtitle text
    pub subtitle: Option<String>,
    /// Optional tag for identification
    pub tag: Option<String>,
}

impl SoundChunk {
    /// Create a new sound chunk
    pub fn new(audio_handle: u32, start_time: f32, duration: f32) -> Self {
        Self {
            audio_handle,
            start_time,
            duration,
            subtitle: None,
            tag: None,
        }
    }

    /// Create a sound chunk with subtitle
    pub fn with_subtitle(audio_handle: u32, start_time: f32, duration: f32, subtitle: &str) -> Self {
        Self {
            audio_handle,
            start_time,
            duration,
            subtitle: Some(subtitle.to_string()),
            tag: None,
        }
    }

    /// Set the subtitle
    pub fn set_subtitle(&mut self, text: &str) {
        self.subtitle = Some(text.to_string());
    }

    /// Set the tag
    pub fn set_tag(&mut self, tag: &str) {
        self.tag = Some(tag.to_string());
    }

    /// Get end time
    pub fn end_time(&self) -> f32 {
        self.start_time + self.duration
    }
}

/// Track playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrackState {
    #[default]
    Stopped,
    Playing,
    Paused,
    Finished,
}

/// Audio track manager
#[derive(Debug, Default)]
pub struct TrackManager {
    /// All chunks in the track
    chunks: Vec<SoundChunk>,
    /// Current playback position in seconds
    position: f32,
    /// Current playback state
    state: TrackState,
    /// Current chunk index
    current_chunk: i32,
    /// Whether track can be interrupted
    interruptible: bool,
    /// Total track length (cached)
    total_length: f32,
    /// Whether to skip on no speech
    no_page_break: bool,
}

impl TrackManager {
    /// Create a new track manager
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            position: 0.0,
            state: TrackState::Stopped,
            current_chunk: -1,
            interruptible: true,
            total_length: 0.0,
            no_page_break: false,
        }
    }

    /// Add a chunk to the track
    pub fn splice(&mut self, chunk: SoundChunk) {
        // Update total length if this chunk extends past current end
        let end = chunk.end_time();
        if end > self.total_length {
            self.total_length = end;
        }
        self.chunks.push(chunk);
    }

    /// Add audio with subtitle
    pub fn splice_track(
        &mut self,
        audio_handle: u32,
        subtitle: Option<&str>,
        start_time: f32,
        duration: f32,
    ) {
        let mut chunk = SoundChunk::new(audio_handle, start_time, duration);
        if let Some(text) = subtitle {
            chunk.set_subtitle(text);
        }
        self.splice(chunk);
    }

    /// Add text-only subtitle (no audio)
    pub fn splice_text(&mut self, text: &str, start_time: f32, duration: f32) {
        let mut chunk = SoundChunk::new(0, start_time, duration);
        chunk.set_subtitle(text);
        self.splice(chunk);
    }

    /// Clear all chunks
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.position = 0.0;
        self.state = TrackState::Stopped;
        self.current_chunk = -1;
        self.total_length = 0.0;
    }

    /// Start playback
    pub fn start(&mut self) {
        if !self.chunks.is_empty() {
            self.state = TrackState::Playing;
            self.current_chunk = 0;
        }
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.state = TrackState::Stopped;
        self.position = 0.0;
        self.current_chunk = -1;
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.state == TrackState::Playing {
            self.state = TrackState::Paused;
        }
    }

    /// Resume playback
    pub fn resume(&mut self) {
        if self.state == TrackState::Paused {
            self.state = TrackState::Playing;
        }
    }

    /// Rewind to beginning
    pub fn rewind(&mut self) {
        self.position = 0.0;
        self.current_chunk = if self.chunks.is_empty() { -1 } else { 0 };
    }

    /// Jump to a position
    pub fn jump(&mut self, offset: f32) {
        self.position = (self.position + offset).max(0.0).min(self.total_length);
        self.update_current_chunk();
    }

    /// Seek to absolute position
    pub fn seek(&mut self, position: f32) {
        self.position = position.max(0.0).min(self.total_length);
        self.update_current_chunk();
    }

    /// Update current chunk based on position
    fn update_current_chunk(&mut self) {
        self.current_chunk = -1;
        for (i, chunk) in self.chunks.iter().enumerate() {
            if self.position >= chunk.start_time && self.position < chunk.end_time() {
                self.current_chunk = i as i32;
                break;
            }
        }
    }

    /// Update playback (call each frame)
    pub fn update(&mut self, delta_time: f32) {
        if self.state != TrackState::Playing {
            return;
        }

        self.position += delta_time;

        // Check if we've reached the end
        if self.position >= self.total_length {
            self.state = TrackState::Finished;
            self.position = self.total_length;
            return;
        }

        self.update_current_chunk();
    }

    /// Get current state
    pub fn state(&self) -> TrackState {
        self.state
    }

    /// Check if playing
    pub fn is_playing(&self) -> bool {
        self.state == TrackState::Playing
    }

    /// Check if finished
    pub fn is_finished(&self) -> bool {
        self.state == TrackState::Finished
    }

    /// Get current position
    pub fn position(&self) -> f32 {
        self.position
    }

    /// Get total length
    pub fn length(&self) -> f32 {
        self.total_length
    }

    /// Get all chunks
    pub fn chunks(&self) -> &[SoundChunk] {
        &self.chunks
    }

    /// Get current chunk
    pub fn current(&self) -> Option<&SoundChunk> {
        if self.current_chunk < 0 {
            return None;
        }
        self.chunks.get(self.current_chunk as usize)
    }

    /// Get current subtitle
    pub fn current_subtitle(&self) -> Option<&str> {
        self.current().and_then(|c| c.subtitle.as_deref())
    }

    /// Get number of chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Check if track is empty
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Set interruptible flag
    pub fn set_interruptible(&mut self, interruptible: bool) {
        self.interruptible = interruptible;
    }

    /// Check if interruptible
    pub fn is_interruptible(&self) -> bool {
        self.interruptible
    }

    /// Set no page break flag
    pub fn set_no_page_break(&mut self, no_page_break: bool) {
        self.no_page_break = no_page_break;
    }

    /// Get no page break flag
    pub fn no_page_break(&self) -> bool {
        self.no_page_break
    }

    /// Wait for track to finish (returns true when done)
    pub fn wait(&self) -> bool {
        matches!(self.state, TrackState::Finished | TrackState::Stopped)
    }

    /// Commit current position (for save/restore)
    pub fn commit(&mut self) -> f32 {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_chunk_new() {
        let chunk = SoundChunk::new(1, 0.0, 2.5);
        assert_eq!(chunk.audio_handle, 1);
        assert_eq!(chunk.start_time, 0.0);
        assert_eq!(chunk.duration, 2.5);
        assert!(chunk.subtitle.is_none());
    }

    #[test]
    fn test_sound_chunk_with_subtitle() {
        let chunk = SoundChunk::with_subtitle(1, 0.0, 2.0, "Hello");
        assert_eq!(chunk.subtitle, Some("Hello".to_string()));
    }

    #[test]
    fn test_sound_chunk_end_time() {
        let chunk = SoundChunk::new(1, 1.0, 2.5);
        assert_eq!(chunk.end_time(), 3.5);
    }

    #[test]
    fn test_track_manager_new() {
        let tm = TrackManager::new();
        assert!(tm.is_empty());
        assert_eq!(tm.state(), TrackState::Stopped);
        assert_eq!(tm.position(), 0.0);
    }

    #[test]
    fn test_track_splice() {
        let mut tm = TrackManager::new();
        tm.splice(SoundChunk::new(1, 0.0, 2.0));
        tm.splice(SoundChunk::new(2, 2.0, 3.0));

        assert_eq!(tm.chunk_count(), 2);
        assert_eq!(tm.length(), 5.0);
    }

    #[test]
    fn test_track_splice_track() {
        let mut tm = TrackManager::new();
        tm.splice_track(1, Some("Subtitle"), 0.0, 2.0);

        assert_eq!(tm.chunk_count(), 1);
        assert_eq!(tm.chunks()[0].subtitle, Some("Subtitle".to_string()));
    }

    #[test]
    fn test_track_splice_text() {
        let mut tm = TrackManager::new();
        tm.splice_text("Text only", 0.0, 1.0);

        assert_eq!(tm.chunks()[0].audio_handle, 0);
        assert_eq!(tm.chunks()[0].subtitle, Some("Text only".to_string()));
    }

    #[test]
    fn test_track_start_stop() {
        let mut tm = TrackManager::new();
        tm.splice(SoundChunk::new(1, 0.0, 2.0));

        tm.start();
        assert!(tm.is_playing());
        assert_eq!(tm.current_chunk, 0);

        tm.stop();
        assert!(!tm.is_playing());
        assert_eq!(tm.position(), 0.0);
    }

    #[test]
    fn test_track_pause_resume() {
        let mut tm = TrackManager::new();
        tm.splice(SoundChunk::new(1, 0.0, 2.0));

        tm.start();
        tm.pause();
        assert_eq!(tm.state(), TrackState::Paused);

        tm.resume();
        assert_eq!(tm.state(), TrackState::Playing);
    }

    #[test]
    fn test_track_update() {
        let mut tm = TrackManager::new();
        tm.splice(SoundChunk::new(1, 0.0, 2.0));
        tm.splice(SoundChunk::new(2, 2.0, 1.0));

        tm.start();
        tm.update(1.5);
        assert_eq!(tm.current_chunk, 0);
        assert_eq!(tm.position(), 1.5);

        tm.update(1.0);
        assert_eq!(tm.current_chunk, 1);
        assert_eq!(tm.position(), 2.5);

        tm.update(1.0);
        assert!(tm.is_finished());
    }

    #[test]
    fn test_track_rewind() {
        let mut tm = TrackManager::new();
        tm.splice(SoundChunk::new(1, 0.0, 2.0));

        tm.start();
        tm.update(1.0);
        tm.rewind();

        assert_eq!(tm.position(), 0.0);
        assert_eq!(tm.current_chunk, 0);
    }

    #[test]
    fn test_track_jump() {
        let mut tm = TrackManager::new();
        tm.splice(SoundChunk::new(1, 0.0, 5.0));

        tm.start();
        tm.jump(2.0);
        assert_eq!(tm.position(), 2.0);

        tm.jump(-1.0);
        assert_eq!(tm.position(), 1.0);

        tm.jump(-10.0); // Should clamp to 0
        assert_eq!(tm.position(), 0.0);
    }

    #[test]
    fn test_track_seek() {
        let mut tm = TrackManager::new();
        tm.splice(SoundChunk::new(1, 0.0, 5.0));

        tm.start();
        tm.seek(3.0);
        assert_eq!(tm.position(), 3.0);
    }

    #[test]
    fn test_track_current_subtitle() {
        let mut tm = TrackManager::new();
        tm.splice_track(1, Some("First"), 0.0, 2.0);
        tm.splice_track(2, Some("Second"), 2.0, 2.0);

        tm.start();
        assert_eq!(tm.current_subtitle(), Some("First"));

        tm.update(2.5);
        assert_eq!(tm.current_subtitle(), Some("Second"));
    }

    #[test]
    fn test_track_clear() {
        let mut tm = TrackManager::new();
        tm.splice(SoundChunk::new(1, 0.0, 2.0));
        tm.start();
        tm.update(1.0);

        tm.clear();

        assert!(tm.is_empty());
        assert_eq!(tm.position(), 0.0);
        assert_eq!(tm.state(), TrackState::Stopped);
    }

    #[test]
    fn test_track_wait() {
        let mut tm = TrackManager::new();
        tm.splice(SoundChunk::new(1, 0.0, 1.0));

        assert!(tm.wait()); // Stopped

        tm.start();
        assert!(!tm.wait()); // Playing

        tm.update(2.0);
        assert!(tm.wait()); // Finished
    }

    #[test]
    fn test_track_interruptible() {
        let mut tm = TrackManager::new();
        assert!(tm.is_interruptible());

        tm.set_interruptible(false);
        assert!(!tm.is_interruptible());
    }

    #[test]
    fn test_track_no_page_break() {
        let mut tm = TrackManager::new();
        assert!(!tm.no_page_break());

        tm.set_no_page_break(true);
        assert!(tm.no_page_break());
    }
}
