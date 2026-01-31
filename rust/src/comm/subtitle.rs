//! Subtitle timing and display
//!
//! Handles subtitle synchronization with audio playback.

/// A subtitle chunk with timing information
#[derive(Debug, Clone)]
pub struct SubtitleChunk {
    /// Start time in seconds
    pub start_time: f32,
    /// Duration in seconds (0 means until next chunk)
    pub duration: f32,
    /// The subtitle text
    pub text: String,
    /// Optional tag for identification
    pub tag: Option<String>,
}

impl SubtitleChunk {
    /// Create a new subtitle chunk
    pub fn new(start_time: f32, text: &str) -> Self {
        Self {
            start_time,
            duration: 0.0,
            text: text.to_string(),
            tag: None,
        }
    }

    /// Create a subtitle chunk with duration
    pub fn with_duration(start_time: f32, duration: f32, text: &str) -> Self {
        Self {
            start_time,
            duration,
            text: text.to_string(),
            tag: None,
        }
    }

    /// Create a subtitle chunk with a tag
    pub fn with_tag(start_time: f32, text: &str, tag: &str) -> Self {
        Self {
            start_time,
            duration: 0.0,
            text: text.to_string(),
            tag: Some(tag.to_string()),
        }
    }

    /// Check if this chunk is active at the given time
    pub fn is_active_at(&self, time: f32, next_start: Option<f32>) -> bool {
        if time < self.start_time {
            return false;
        }

        if self.duration > 0.0 {
            // Has explicit duration
            time < self.start_time + self.duration
        } else if let Some(next) = next_start {
            // Use next chunk's start time
            time < next
        } else {
            // Last chunk - always active after start
            true
        }
    }
}

/// Subtitle tracker for managing subtitle display
#[derive(Debug, Default)]
pub struct SubtitleTracker {
    /// All subtitle chunks in order
    chunks: Vec<SubtitleChunk>,
    /// Current chunk index
    current_index: i32,
    /// Current playback time
    current_time: f32,
    /// Whether subtitles are enabled
    enabled: bool,
}

impl SubtitleTracker {
    /// Create a new subtitle tracker
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            current_index: -1,
            current_time: 0.0,
            enabled: true,
        }
    }

    /// Add a subtitle chunk
    pub fn add_chunk(&mut self, chunk: SubtitleChunk) {
        // Insert in sorted order by start time
        let pos = self
            .chunks
            .iter()
            .position(|c| c.start_time > chunk.start_time)
            .unwrap_or(self.chunks.len());
        self.chunks.insert(pos, chunk);
    }

    /// Add a subtitle at a given time
    pub fn add_subtitle(&mut self, start_time: f32, text: &str) {
        self.add_chunk(SubtitleChunk::new(start_time, text));
    }

    /// Add a subtitle with duration
    pub fn add_subtitle_with_duration(&mut self, start_time: f32, duration: f32, text: &str) {
        self.add_chunk(SubtitleChunk::with_duration(start_time, duration, text));
    }

    /// Clear all subtitles
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.current_index = -1;
        self.current_time = 0.0;
    }

    /// Update current time and return current subtitle (if any)
    pub fn update(&mut self, time: f32) -> Option<&str> {
        self.current_time = time;
        self.get_subtitle_at(time)
    }

    /// Get subtitle at a specific time
    pub fn get_subtitle_at(&self, time: f32) -> Option<&str> {
        if !self.enabled || self.chunks.is_empty() {
            return None;
        }

        for (i, chunk) in self.chunks.iter().enumerate() {
            let next_start = self.chunks.get(i + 1).map(|c| c.start_time);
            if chunk.is_active_at(time, next_start) {
                return Some(&chunk.text);
            }
        }

        None
    }

    /// Get the current subtitle
    pub fn current_subtitle(&self) -> Option<&str> {
        self.get_subtitle_at(self.current_time)
    }

    /// Get all chunks
    pub fn chunks(&self) -> &[SubtitleChunk] {
        &self.chunks
    }

    /// Get number of chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Enable/disable subtitles
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if subtitles are enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Reset playback position
    pub fn reset(&mut self) {
        self.current_index = -1;
        self.current_time = 0.0;
    }

    /// Seek to a specific time
    pub fn seek(&mut self, time: f32) {
        self.current_time = time;
        // Find the chunk that should be active
        self.current_index = -1;
        for (i, chunk) in self.chunks.iter().enumerate() {
            if chunk.start_time <= time {
                self.current_index = i as i32;
            } else {
                break;
            }
        }
    }

    /// Get total duration (end of last subtitle)
    pub fn total_duration(&self) -> f32 {
        if self.chunks.is_empty() {
            return 0.0;
        }

        let last = self.chunks.last().unwrap();
        if last.duration > 0.0 {
            last.start_time + last.duration
        } else {
            // Estimate 3 seconds for last subtitle if no duration
            last.start_time + 3.0
        }
    }

    /// Get chunk by tag
    pub fn get_by_tag(&self, tag: &str) -> Option<&SubtitleChunk> {
        self.chunks.iter().find(|c| c.tag.as_deref() == Some(tag))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtitle_chunk_new() {
        let chunk = SubtitleChunk::new(1.5, "Hello world");
        assert_eq!(chunk.start_time, 1.5);
        assert_eq!(chunk.duration, 0.0);
        assert_eq!(chunk.text, "Hello world");
        assert!(chunk.tag.is_none());
    }

    #[test]
    fn test_subtitle_chunk_with_duration() {
        let chunk = SubtitleChunk::with_duration(2.0, 3.5, "Text");
        assert_eq!(chunk.start_time, 2.0);
        assert_eq!(chunk.duration, 3.5);
    }

    #[test]
    fn test_subtitle_chunk_with_tag() {
        let chunk = SubtitleChunk::with_tag(1.0, "Tagged", "intro");
        assert_eq!(chunk.tag, Some("intro".to_string()));
    }

    #[test]
    fn test_is_active_at_with_duration() {
        let chunk = SubtitleChunk::with_duration(1.0, 2.0, "Text");

        assert!(!chunk.is_active_at(0.5, None)); // Before start
        assert!(chunk.is_active_at(1.0, None)); // At start
        assert!(chunk.is_active_at(2.0, None)); // During
        assert!(!chunk.is_active_at(3.0, None)); // After end
    }

    #[test]
    fn test_is_active_at_with_next() {
        let chunk = SubtitleChunk::new(1.0, "Text");

        assert!(!chunk.is_active_at(0.5, Some(3.0)));
        assert!(chunk.is_active_at(1.0, Some(3.0)));
        assert!(chunk.is_active_at(2.0, Some(3.0)));
        assert!(!chunk.is_active_at(3.0, Some(3.0)));
    }

    #[test]
    fn test_is_active_at_last_chunk() {
        let chunk = SubtitleChunk::new(5.0, "Last");

        assert!(!chunk.is_active_at(4.0, None));
        assert!(chunk.is_active_at(5.0, None));
        assert!(chunk.is_active_at(100.0, None)); // Always active
    }

    #[test]
    fn test_tracker_new() {
        let tracker = SubtitleTracker::new();
        assert!(tracker.chunks.is_empty());
        assert!(tracker.is_enabled());
    }

    #[test]
    fn test_tracker_add_subtitle() {
        let mut tracker = SubtitleTracker::new();
        tracker.add_subtitle(1.0, "First");
        tracker.add_subtitle(3.0, "Second");

        assert_eq!(tracker.chunk_count(), 2);
        assert_eq!(tracker.chunks()[0].text, "First");
        assert_eq!(tracker.chunks()[1].text, "Second");
    }

    #[test]
    fn test_tracker_add_out_of_order() {
        let mut tracker = SubtitleTracker::new();
        tracker.add_subtitle(3.0, "Third");
        tracker.add_subtitle(1.0, "First");
        tracker.add_subtitle(2.0, "Second");

        assert_eq!(tracker.chunks()[0].text, "First");
        assert_eq!(tracker.chunks()[1].text, "Second");
        assert_eq!(tracker.chunks()[2].text, "Third");
    }

    #[test]
    fn test_tracker_get_subtitle_at() {
        let mut tracker = SubtitleTracker::new();
        tracker.add_subtitle(0.0, "Intro");
        tracker.add_subtitle(2.0, "Middle");
        tracker.add_subtitle(4.0, "End");

        assert_eq!(tracker.get_subtitle_at(0.5), Some("Intro"));
        assert_eq!(tracker.get_subtitle_at(2.5), Some("Middle"));
        assert_eq!(tracker.get_subtitle_at(5.0), Some("End"));
    }

    #[test]
    fn test_tracker_update() {
        let mut tracker = SubtitleTracker::new();
        tracker.add_subtitle(1.0, "Hello");
        tracker.add_subtitle(3.0, "World");

        assert_eq!(tracker.update(0.5), None);
        assert_eq!(tracker.update(1.5), Some("Hello"));
        assert_eq!(tracker.update(3.5), Some("World"));
    }

    #[test]
    fn test_tracker_clear() {
        let mut tracker = SubtitleTracker::new();
        tracker.add_subtitle(1.0, "Test");
        tracker.current_time = 1.5;

        tracker.clear();

        assert!(tracker.chunks.is_empty());
        assert_eq!(tracker.current_time, 0.0);
    }

    #[test]
    fn test_tracker_disabled() {
        let mut tracker = SubtitleTracker::new();
        tracker.add_subtitle(0.0, "Text");
        tracker.set_enabled(false);

        assert!(tracker.get_subtitle_at(0.5).is_none());
    }

    #[test]
    fn test_tracker_seek() {
        let mut tracker = SubtitleTracker::new();
        tracker.add_subtitle(0.0, "Start");
        tracker.add_subtitle(2.0, "Middle");
        tracker.add_subtitle(4.0, "End");

        tracker.seek(3.0);
        assert_eq!(tracker.current_subtitle(), Some("Middle"));

        tracker.seek(0.5);
        assert_eq!(tracker.current_subtitle(), Some("Start"));
    }

    #[test]
    fn test_tracker_total_duration() {
        let mut tracker = SubtitleTracker::new();
        assert_eq!(tracker.total_duration(), 0.0);

        tracker.add_subtitle(0.0, "Start");
        tracker.add_subtitle_with_duration(2.0, 1.5, "End");

        assert_eq!(tracker.total_duration(), 3.5);
    }

    #[test]
    fn test_tracker_get_by_tag() {
        let mut tracker = SubtitleTracker::new();
        tracker.add_chunk(SubtitleChunk::with_tag(0.0, "Tagged", "special"));
        tracker.add_subtitle(1.0, "Normal");

        let found = tracker.get_by_tag("special");
        assert!(found.is_some());
        assert_eq!(found.unwrap().text, "Tagged");

        assert!(tracker.get_by_tag("notfound").is_none());
    }

    #[test]
    fn test_tracker_reset() {
        let mut tracker = SubtitleTracker::new();
        tracker.add_subtitle(0.0, "Test");
        tracker.seek(5.0);

        tracker.reset();

        assert_eq!(tracker.current_time, 0.0);
        assert_eq!(tracker.current_index, -1);
    }
}
