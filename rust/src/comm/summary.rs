//! Conversation summary model and subtitle history pagination.
//!
//! This module provides comm-local pagination over subtitle history
//! that is sourced from the authoritative trackplayer. It does not
//! own the subtitle history — that belongs to the trackplayer.
//!
//! @plan PLAN-20260314-COMM.P06
//! @requirement SS-REQ-013, SS-REQ-014, SS-REQ-015, SS-REQ-017

/// A single page of conversation subtitles.
#[derive(Debug, Clone)]
pub struct SummaryPage {
    /// Subtitle entries on this page.
    pub entries: Vec<String>,
    /// Page number (0-based).
    pub page_index: usize,
}

/// Conversation summary — pagination over trackplayer subtitle history.
///
/// This is a derived cache; the trackplayer is the source of truth.
/// Call `rebuild()` to re-read from the trackplayer history.
#[derive(Debug, Default)]
pub struct ConversationSummary {
    /// All subtitle entries (sourced from trackplayer).
    entries: Vec<String>,
    /// Paginated view.
    pages: Vec<SummaryPage>,
    /// Current page index.
    current_page: usize,
    /// Lines per page for pagination.
    lines_per_page: usize,
}

impl ConversationSummary {
    /// Create a new summary with the given lines-per-page limit.
    pub fn new(lines_per_page: usize) -> Self {
        Self {
            entries: Vec::new(),
            pages: Vec::new(),
            current_page: 0,
            lines_per_page: lines_per_page.max(1),
        }
    }

    /// Rebuild the summary from a list of subtitle entries.
    /// In production, this is called with entries from
    /// `CTrackBridge::enumerate_subtitle_history()`.
    pub fn rebuild(&mut self, entries: Vec<String>) {
        self.entries = entries;
        self.pages = paginate_subtitles(&self.entries, self.lines_per_page);
        // Jump to last page when content updates
        if !self.pages.is_empty() {
            self.current_page = self.pages.len() - 1;
        } else {
            self.current_page = 0;
        }
    }

    /// Clear all cached state. Does not affect the trackplayer.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.pages.clear();
        self.current_page = 0;
    }

    /// Get the current page (if any).
    pub fn current_page(&self) -> Option<&SummaryPage> {
        self.pages.get(self.current_page)
    }

    /// Get page by index.
    pub fn page(&self, index: usize) -> Option<&SummaryPage> {
        self.pages.get(index)
    }

    /// Total number of pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Current page index.
    pub fn current_page_index(&self) -> usize {
        self.current_page
    }

    /// Navigate to the next page. Returns true if moved.
    pub fn next_page(&mut self) -> bool {
        if self.current_page + 1 < self.pages.len() {
            self.current_page += 1;
            true
        } else {
            false
        }
    }

    /// Navigate to the previous page. Returns true if moved.
    pub fn prev_page(&mut self) -> bool {
        if self.current_page > 0 {
            self.current_page -= 1;
            true
        } else {
            false
        }
    }

    /// All subtitle entries (for enumeration).
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Whether the summary is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Paginate subtitle entries into pages of at most `lines_per_page`.
pub fn paginate_subtitles(entries: &[String], lines_per_page: usize) -> Vec<SummaryPage> {
    if entries.is_empty() || lines_per_page == 0 {
        return Vec::new();
    }

    entries
        .chunks(lines_per_page)
        .enumerate()
        .map(|(i, chunk)| SummaryPage {
            entries: chunk.to_vec(),
            page_index: i,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginate_empty() {
        let pages = paginate_subtitles(&[], 5);
        assert!(pages.is_empty());
    }

    #[test]
    fn test_paginate_single_page() {
        let entries: Vec<String> = vec!["Hello".into(), "World".into()];
        let pages = paginate_subtitles(&entries, 5);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].entries.len(), 2);
        assert_eq!(pages[0].page_index, 0);
    }

    #[test]
    fn test_paginate_multi_page() {
        let entries: Vec<String> = (0..7).map(|i| format!("Line {}", i)).collect();
        let pages = paginate_subtitles(&entries, 3);
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].entries.len(), 3);
        assert_eq!(pages[1].entries.len(), 3);
        assert_eq!(pages[2].entries.len(), 1);
    }

    #[test]
    fn test_summary_rebuild_and_navigate() {
        let mut summary = ConversationSummary::new(3);
        let entries: Vec<String> = (0..7).map(|i| format!("Sub {}", i)).collect();

        summary.rebuild(entries);
        assert_eq!(summary.page_count(), 3);
        // Should be on last page after rebuild
        assert_eq!(summary.current_page_index(), 2);

        assert!(summary.prev_page());
        assert_eq!(summary.current_page_index(), 1);
        assert!(summary.prev_page());
        assert_eq!(summary.current_page_index(), 0);
        assert!(!summary.prev_page()); // Can't go before 0

        assert!(summary.next_page());
        assert_eq!(summary.current_page_index(), 1);
    }

    #[test]
    fn test_summary_clear() {
        let mut summary = ConversationSummary::new(5);
        summary.rebuild(vec!["A".into(), "B".into()]);
        assert!(!summary.is_empty());

        summary.clear();
        assert!(summary.is_empty());
        assert_eq!(summary.page_count(), 0);
    }

    #[test]
    fn test_summary_entries() {
        let mut summary = ConversationSummary::new(10);
        let entries = vec!["One".to_string(), "Two".to_string(), "Three".to_string()];
        summary.rebuild(entries.clone());
        assert_eq!(summary.entries(), &entries);
    }

    #[test]
    fn test_summary_current_page_content() {
        let mut summary = ConversationSummary::new(2);
        summary.rebuild(vec!["A".into(), "B".into(), "C".into()]);

        // Last page (page 1) has "C"
        let page = summary.current_page().unwrap();
        assert_eq!(page.entries, vec!["C".to_string()]);

        summary.prev_page();
        let page = summary.current_page().unwrap();
        assert_eq!(page.entries, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn test_summary_empty_operations() {
        let mut summary = ConversationSummary::new(5);
        assert!(summary.current_page().is_none());
        assert!(!summary.next_page());
        assert!(!summary.prev_page());
    }
}
