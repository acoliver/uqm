//! Conversation summary model and subtitle history pagination.
//!
//! This module provides comm-local pagination over subtitle history
//! that is sourced from the authoritative trackplayer. It does not
//! own the subtitle history — that belongs to the trackplayer.
//!
//! @plan PLAN-20260314-COMM.P06
//! @plan PLAN-20260314-COMM.P10
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

// ============================================================================
// SummaryResult
// ============================================================================

/// Navigation result returned by `SummaryView::advance_page`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryResult {
    /// More pages remain; display the next page.
    NextPage,
    /// All pages shown; exit the summary view.
    Exit,
    /// Summary was aborted (e.g. player pressed cancel mid-view).
    Aborted,
}

// ============================================================================
// SummaryView
// ============================================================================

/// Summary view state for the conversation summary overlay.
///
/// Wraps a `ConversationSummary` with a simple page-navigation cursor and
/// derived page count so callers never need to touch the underlying
/// `ConversationSummary` directly during summary display.
#[derive(Debug)]
pub struct SummaryView {
    initialized: bool,
    current_page: usize,
    total_pages: usize,
    lines_per_page: usize,
}

impl Default for SummaryView {
    fn default() -> Self {
        Self::new(10)
    }
}

impl SummaryView {
    /// Create a new summary view with the given lines-per-page limit.
    pub fn new(lines_per_page: usize) -> Self {
        Self {
            initialized: false,
            current_page: 0,
            total_pages: 0,
            lines_per_page: lines_per_page.max(1),
        }
    }

    /// Initialize the view from a `ConversationSummary`.
    ///
    /// Computes the number of pages and resets to page 0. Returns the total
    /// page count.
    pub fn init(&mut self, summary: &ConversationSummary) -> usize {
        let entry_count = summary.entries().len();
        self.total_pages = if entry_count == 0 {
            0
        } else {
            (entry_count + self.lines_per_page - 1) / self.lines_per_page
        };
        self.current_page = 0;
        self.initialized = true;
        self.total_pages
    }

    /// Current page index (0-based).
    pub fn current_page(&self) -> usize {
        self.current_page
    }

    /// Total page count computed during `init`.
    pub fn total_pages(&self) -> usize {
        self.total_pages
    }

    /// Advance to the next page.
    ///
    /// Returns `NextPage` if there are more pages, or `Exit` when the last
    /// page has been shown.
    pub fn advance_page(&mut self) -> SummaryResult {
        if self.total_pages == 0 {
            return SummaryResult::Exit;
        }

        if self.current_page + 1 < self.total_pages {
            self.current_page += 1;
            SummaryResult::NextPage
        } else {
            SummaryResult::Exit
        }
    }

    /// Return the lines for the current page, sourced from `summary`.
    pub fn get_page_lines(&self, summary: &ConversationSummary) -> Vec<String> {
        let entries = summary.entries();
        let start = self.current_page * self.lines_per_page;
        if start >= entries.len() {
            return Vec::new();
        }
        let end = (start + self.lines_per_page).min(entries.len());
        entries[start..end].to_vec()
    }

    /// Reset view to initial (uninitialized) state.
    pub fn reset(&mut self) {
        self.initialized = false;
        self.current_page = 0;
        self.total_pages = 0;
    }

    /// Whether the view has been initialized with a summary.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

// ============================================================================
// Tests
// ============================================================================

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

    // ---- SummaryView -------------------------------------------------------

    #[test]
    fn test_single_page() {
        let mut summary = ConversationSummary::new(10);
        summary.rebuild(vec!["A".into(), "B".into()]);

        let mut view = SummaryView::new(10);
        let pages = view.init(&summary);
        assert_eq!(pages, 1);

        // Only 1 page — advance returns Exit.
        let result = view.advance_page();
        assert_eq!(result, SummaryResult::Exit);
    }

    #[test]
    fn test_multi_page_navigation() {
        let entries: Vec<String> = (0..7).map(|i| format!("Line {}", i)).collect();
        let mut summary = ConversationSummary::new(10);
        summary.rebuild(entries);

        let mut view = SummaryView::new(3);
        let pages = view.init(&summary);
        assert_eq!(pages, 3);
        assert_eq!(view.current_page(), 0);

        assert_eq!(view.advance_page(), SummaryResult::NextPage);
        assert_eq!(view.current_page(), 1);

        assert_eq!(view.advance_page(), SummaryResult::NextPage);
        assert_eq!(view.current_page(), 2);

        assert_eq!(view.advance_page(), SummaryResult::Exit);
        // current_page stays at 2 after Exit.
        assert_eq!(view.current_page(), 2);
    }

    #[test]
    fn test_lines_per_page_respected() {
        let entries: Vec<String> = (0..5).map(|i| format!("L{}", i)).collect();
        let mut summary = ConversationSummary::new(10);
        summary.rebuild(entries);

        let mut view = SummaryView::new(2);
        view.init(&summary);

        let page0 = view.get_page_lines(&summary);
        assert_eq!(page0.len(), 2);
        assert_eq!(page0[0], "L0");
        assert_eq!(page0[1], "L1");

        view.advance_page();
        let page1 = view.get_page_lines(&summary);
        assert_eq!(page1.len(), 2);

        view.advance_page();
        let page2 = view.get_page_lines(&summary);
        assert_eq!(page2.len(), 1, "last page has 1 remaining entry");
    }

    #[test]
    fn test_empty_summary() {
        let summary = ConversationSummary::new(5);

        let mut view = SummaryView::new(5);
        let pages = view.init(&summary);
        assert_eq!(pages, 0);

        let result = view.advance_page();
        assert_eq!(result, SummaryResult::Exit);
    }

    #[test]
    fn test_reset() {
        let entries: Vec<String> = vec!["A".into(), "B".into(), "C".into()];
        let mut summary = ConversationSummary::new(10);
        summary.rebuild(entries);

        let mut view = SummaryView::new(2);
        view.init(&summary);
        view.advance_page();
        assert!(view.is_initialized());
        assert_eq!(view.current_page(), 1);

        view.reset();

        assert!(!view.is_initialized());
        assert_eq!(view.current_page(), 0);
        assert_eq!(view.total_pages(), 0);
    }
}
