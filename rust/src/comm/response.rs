//! Response system for alien dialogue
//!
//! Handles player response options during conversations.

/// A response option in a conversation
#[derive(Debug, Clone)]
pub struct ResponseEntry {
    /// Reference ID for this response
    pub response_ref: u32,
    /// Display text for the response
    pub response_text: String,
    /// Callback function address (as usize for FFI)
    pub response_func: Option<usize>,
}

impl ResponseEntry {
    /// Create a new response entry
    pub fn new(response_ref: u32, text: &str, func: Option<usize>) -> Self {
        Self {
            response_ref,
            response_text: text.to_string(),
            response_func: func,
        }
    }

    /// Create a response with no callback
    pub fn text_only(response_ref: u32, text: &str) -> Self {
        Self::new(response_ref, text, None)
    }
}

/// Maximum number of response options
pub const MAX_RESPONSES: usize = 8;

/// Response selection system
#[derive(Debug, Default)]
pub struct ResponseSystem {
    /// Available response options
    responses: Vec<ResponseEntry>,
    /// Currently selected response index (-1 for none)
    selected: i32,
    /// Whether responses are currently being displayed
    displaying: bool,
}

impl ResponseSystem {
    /// Create a new response system
    pub fn new() -> Self {
        Self {
            responses: Vec::with_capacity(MAX_RESPONSES),
            selected: -1,
            displaying: false,
        }
    }

    /// Add a response option
    ///
    /// Returns false if maximum responses reached
    pub fn add_response(&mut self, entry: ResponseEntry) -> bool {
        if self.responses.len() >= MAX_RESPONSES {
            return false;
        }
        self.responses.push(entry);
        true
    }

    /// Add a response with all parameters
    pub fn do_response_phrase(
        &mut self,
        response_ref: u32,
        text: &str,
        func: Option<usize>,
    ) -> bool {
        self.add_response(ResponseEntry::new(response_ref, text, func))
    }

    /// Clear all responses
    pub fn clear(&mut self) {
        self.responses.clear();
        self.selected = -1;
        self.displaying = false;
    }

    /// Get all responses
    pub fn responses(&self) -> &[ResponseEntry] {
        &self.responses
    }

    /// Get number of responses
    pub fn count(&self) -> usize {
        self.responses.len()
    }

    /// Check if there are any responses
    pub fn is_empty(&self) -> bool {
        self.responses.is_empty()
    }

    /// Get a response by index
    pub fn get(&self, index: usize) -> Option<&ResponseEntry> {
        self.responses.get(index)
    }

    /// Get currently selected index
    pub fn selected(&self) -> i32 {
        self.selected
    }

    /// Set selected response
    pub fn select(&mut self, index: i32) -> bool {
        if index < 0 || index as usize >= self.responses.len() {
            return false;
        }
        self.selected = index;
        true
    }

    /// Move selection up
    pub fn select_prev(&mut self) -> bool {
        if self.responses.is_empty() {
            return false;
        }

        if self.selected <= 0 {
            self.selected = (self.responses.len() - 1) as i32;
        } else {
            self.selected -= 1;
        }
        true
    }

    /// Move selection down
    pub fn select_next(&mut self) -> bool {
        if self.responses.is_empty() {
            return false;
        }

        self.selected = (self.selected + 1) % self.responses.len() as i32;
        true
    }

    /// Get the currently selected response
    pub fn get_selected(&self) -> Option<&ResponseEntry> {
        if self.selected < 0 {
            return None;
        }
        self.responses.get(self.selected as usize)
    }

    /// Start displaying responses
    pub fn start_display(&mut self) {
        self.displaying = true;
        if !self.responses.is_empty() && self.selected < 0 {
            self.selected = 0;
        }
    }

    /// Stop displaying responses
    pub fn stop_display(&mut self) {
        self.displaying = false;
    }

    /// Check if displaying
    pub fn is_displaying(&self) -> bool {
        self.displaying
    }

    /// Execute the selected response's callback
    ///
    /// # Safety
    /// Caller must ensure the callback address is a valid function pointer
    pub unsafe fn execute_selected(&self) -> Option<u32> {
        let response = self.get_selected()?;
        let func_addr = response.response_func?;

        // Cast to function pointer and call
        let func: extern "C" fn() = std::mem::transmute(func_addr);
        func();

        Some(response.response_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_entry_new() {
        let entry = ResponseEntry::new(1, "Hello", Some(0x1000));
        assert_eq!(entry.response_ref, 1);
        assert_eq!(entry.response_text, "Hello");
        assert_eq!(entry.response_func, Some(0x1000));
    }

    #[test]
    fn test_response_entry_text_only() {
        let entry = ResponseEntry::text_only(2, "Goodbye");
        assert_eq!(entry.response_ref, 2);
        assert_eq!(entry.response_text, "Goodbye");
        assert!(entry.response_func.is_none());
    }

    #[test]
    fn test_response_system_new() {
        let sys = ResponseSystem::new();
        assert!(sys.is_empty());
        assert_eq!(sys.count(), 0);
        assert_eq!(sys.selected(), -1);
    }

    #[test]
    fn test_add_response() {
        let mut sys = ResponseSystem::new();
        assert!(sys.add_response(ResponseEntry::text_only(1, "Option 1")));
        assert!(sys.add_response(ResponseEntry::text_only(2, "Option 2")));

        assert_eq!(sys.count(), 2);
        assert!(!sys.is_empty());
    }

    #[test]
    fn test_max_responses() {
        let mut sys = ResponseSystem::new();

        for i in 0..MAX_RESPONSES {
            assert!(sys.add_response(ResponseEntry::text_only(i as u32, "Option")));
        }

        // Should fail at max
        assert!(!sys.add_response(ResponseEntry::text_only(99, "Extra")));
        assert_eq!(sys.count(), MAX_RESPONSES);
    }

    #[test]
    fn test_clear() {
        let mut sys = ResponseSystem::new();
        sys.add_response(ResponseEntry::text_only(1, "Option"));
        sys.select(0);
        sys.start_display();

        sys.clear();

        assert!(sys.is_empty());
        assert_eq!(sys.selected(), -1);
        assert!(!sys.is_displaying());
    }

    #[test]
    fn test_select() {
        let mut sys = ResponseSystem::new();
        sys.add_response(ResponseEntry::text_only(1, "A"));
        sys.add_response(ResponseEntry::text_only(2, "B"));
        sys.add_response(ResponseEntry::text_only(3, "C"));

        assert!(sys.select(1));
        assert_eq!(sys.selected(), 1);
        assert_eq!(sys.get_selected().unwrap().response_text, "B");

        // Invalid selection
        assert!(!sys.select(-1));
        assert!(!sys.select(10));
    }

    #[test]
    fn test_select_prev_next() {
        let mut sys = ResponseSystem::new();
        sys.add_response(ResponseEntry::text_only(1, "A"));
        sys.add_response(ResponseEntry::text_only(2, "B"));
        sys.add_response(ResponseEntry::text_only(3, "C"));

        sys.select(0);

        sys.select_next();
        assert_eq!(sys.selected(), 1);

        sys.select_next();
        assert_eq!(sys.selected(), 2);

        sys.select_next(); // Wrap around
        assert_eq!(sys.selected(), 0);

        sys.select_prev(); // Wrap around backwards
        assert_eq!(sys.selected(), 2);
    }

    #[test]
    fn test_select_prev_next_empty() {
        let mut sys = ResponseSystem::new();
        assert!(!sys.select_next());
        assert!(!sys.select_prev());
    }

    #[test]
    fn test_display_state() {
        let mut sys = ResponseSystem::new();
        sys.add_response(ResponseEntry::text_only(1, "A"));

        assert!(!sys.is_displaying());

        sys.start_display();
        assert!(sys.is_displaying());
        assert_eq!(sys.selected(), 0); // Auto-selects first

        sys.stop_display();
        assert!(!sys.is_displaying());
    }

    #[test]
    fn test_do_response_phrase() {
        let mut sys = ResponseSystem::new();
        assert!(sys.do_response_phrase(1, "Hello", None));
        assert!(sys.do_response_phrase(2, "Goodbye", Some(0x1000)));

        assert_eq!(sys.count(), 2);
        assert!(sys.get(1).unwrap().response_func.is_some());
    }

    #[test]
    fn test_get_response() {
        let mut sys = ResponseSystem::new();
        sys.add_response(ResponseEntry::text_only(1, "First"));
        sys.add_response(ResponseEntry::text_only(2, "Second"));

        assert_eq!(sys.get(0).unwrap().response_text, "First");
        assert_eq!(sys.get(1).unwrap().response_text, "Second");
        assert!(sys.get(5).is_none());
    }

    #[test]
    fn test_responses_slice() {
        let mut sys = ResponseSystem::new();
        sys.add_response(ResponseEntry::text_only(1, "A"));
        sys.add_response(ResponseEntry::text_only(2, "B"));

        let responses = sys.responses();
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].response_ref, 1);
        assert_eq!(responses[1].response_ref, 2);
    }
}
