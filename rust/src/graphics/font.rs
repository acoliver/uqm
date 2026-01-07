//!
//! Phase 2: Font system
//!
//! Core font types and page management, translated from C code:
//! - sc2/src/libs/graphics/font.h
//! - sc2/src/libs/graphics/font.c
//!
//! Provides the Font/FontPage/TFChar types with efficient character lookup.

use std::sync::Arc;

/// Unicode character code point (32-bit UTF-32).
pub type UniChar = u32;

/// Character page mask - matches C CHARACTER_PAGE_MASK (0xfffff800).
///
/// Used to extract the page identifier from a Unicode character.
/// Characters are grouped into pages of 2048 characters each.
const CHARACTER_PAGE_MASK: UniChar = 0xfffff800;

/// Character page size - 2048 characters per page.
const CHARACTER_PAGE_SIZE: usize = 2048;

/// Character descriptor representing a single glyph.
///
/// Corresponds to C TFB_Char from tfb_draw.h. The actual pixel data is stored
/// as raw bytes with a pitch offset between character rows.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TFChar {
    /// Full bounding box extent of the character.
    pub extent: Extent,
    /// Display extent - actual occupied pixels (may be smaller than extent).
    pub disp: Extent,
    /// Hot spot/origin offset relative to the baseline.
    pub hotspot: Point,
    /// Pointer to raw pixel data (RLE or bitmap, format depends on context).
    pub data: Option<Arc<[u8]>>,
    /// Pitch (bytes per row) for data access.
    pub pitch: u32,
}

impl Default for TFChar {
    fn default() -> Self {
        Self {
            extent: Extent::ZERO,
            disp: Extent::ZERO,
            hotspot: Point::ZERO,
            data: None,
            pitch: 0,
        }
    }
}

/// 2D extent with width and height.
///
/// Corresponds to C EXTENT from gfxlib.h.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Extent {
    pub width: i16,
    pub height: i16,
}

impl Default for Extent {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Extent {
    /// Zero extent constant.
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    /// Create a new extent.
    pub const fn new(width: i16, height: i16) -> Self {
        Self { width, height }
    }
}

/// 2D point with x and y coordinates.
///
/// Corresponds to C POINT and HOT_SPOT from gfxlib.h.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

impl Default for Point {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Point {
    /// Zero point constant.
    pub const ZERO: Self = Self { x: 0, y: 0 };

    /// Create a new point.
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

/// A font page containing a contiguous range of characters.
///
/// Corresponds to C FONT_PAGE from font.h. Characters are organized into
/// pages for efficient lookup. Each page contains up to 2048 characters
/// starting at a page-aligned Unicode code point.
#[derive(Debug)]
pub struct FontPage {
    /// Page starting character code (page-aligned with CHARACTER_PAGE_MASK).
    page_start: UniChar,
    /// First character code in this page (may be > page_start).
    first_char: UniChar,
    /// Number of character descriptors in this page.
    num_chars: usize,
    /// Character descriptors for this page.
    /// The array index from first_char is: ch - first_char.
    chars: Box<[Option<TFChar>]>,
    /// Next page in the linked list.
    next: Option<Box<FontPage>>,
}

impl FontPage {
    /// Create a new font page for a character range.
    ///
    /// # Arguments
    /// * `page_start` - Starting character code (will be masked to page boundary).
    /// * `first_char` - First actual character code in this page (>= page_start).
    /// * `num_chars` - Number of characters in this page.
    ///
    /// # Returns
    /// A new FontPage with None-initialized character slots.
    pub fn new(page_start: UniChar, first_char: UniChar, num_chars: usize) -> Self {
        assert!(num_chars > 0, "num_chars must be positive");
        assert!(
            num_chars <= CHARACTER_PAGE_SIZE,
            "num_chars cannot exceed page size"
        );

        let page_start = page_start & CHARACTER_PAGE_MASK;
        let first_char = first_char.max(page_start);
        assert!(
            first_char + (num_chars as UniChar) <= page_start + (CHARACTER_PAGE_SIZE as UniChar),
            "page range exceeds CHARACTER_PAGE_SIZE"
        );

        let chars = vec![None; num_chars].into_boxed_slice();

        Self {
            page_start,
            first_char,
            num_chars,
            chars,
            next: None,
        }
    }

    /// Get the page start code.
    #[must_use]
    pub const fn page_start(&self) -> UniChar {
        self.page_start
    }

    /// Get the first character code in this page.
    #[must_use]
    pub const fn first_char(&self) -> UniChar {
        self.first_char
    }

    /// Get the number of characters in this page.
    #[must_use]
    pub const fn num_chars(&self) -> usize {
        self.num_chars
    }

    /// Check if a character code is within this page's character range.
    #[must_use]
    pub const fn contains_char(&self, ch: UniChar) -> bool {
        ch >= self.first_char && (ch - self.first_char) < (self.num_chars as UniChar)
    }

    /// Get a character descriptor by Unicode code point.
    ///
    /// Returns None if the character is out of page bounds or not present.
    #[must_use]
    pub fn get_char(&self, ch: UniChar) -> Option<&TFChar> {
        if !self.contains_char(ch) {
            return None;
        }
        self.chars.get((ch - self.first_char) as usize)?.as_ref()
    }

    /// Get a mutable reference to a character descriptor by Unicode code point.
    ///
    /// Returns None if the character is out of page bounds.
    pub fn get_char_mut(&mut self, ch: UniChar) -> Option<&mut TFChar> {
        if !self.contains_char(ch) {
            return None;
        }
        self.chars
            .get_mut((ch - self.first_char) as usize)?
            .as_mut()
    }

    /// Set a character descriptor for a Unicode code point.
    ///
    /// Returns Err if the character is out of page bounds.
    pub fn set_char(&mut self, ch: UniChar, char_desc: TFChar) -> Result<(), FontError> {
        if !self.contains_char(ch) {
            return Err(FontError::CharOutOfRange(ch));
        }
        self.chars[(ch - self.first_char) as usize] = Some(char_desc);
        Ok(())
    }
}

/// Main font descriptor containing all font metadata and pages.
///
/// Corresponds to C FONT_DESC from font.h. A font is composed of
/// a linked list of FontPage structures, each containing a subset
/// of character glyphs.
#[derive(Debug)]
pub struct Font {
    /// Leading (line height) for this font.
    leading: u16,
    /// Leading width (character width) for this font.
    leading_width: u16,
    /// Linked list of font pages.
    head_page: Option<Box<FontPage>>,
}

impl Font {
    /// Create a new font with specified metrics and no pages.
    ///
    /// # Arguments
    /// * `leading` - Line spacing (vertical distance between baselines).
    /// * `leading_width` - Character width scaling factor.
    ///
    /// # Returns
    /// A new empty Font.
    #[must_use]
    pub fn new(leading: u16, leading_width: u16) -> Self {
        Self {
            leading,
            leading_width,
            head_page: None,
        }
    }

    /// Get the leading (line height) for this font.
    #[must_use]
    pub const fn leading(&self) -> u16 {
        self.leading
    }

    /// Get the leading width for this font.
    #[must_use]
    pub const fn leading_width(&self) -> u16 {
        self.leading_width
    }

    /// Look up a character descriptor by Unicode code point.
    ///
    /// O(n) where n is the number of pages in the font. This follows
    /// the C implementation's linked list traversal.
    ///
    /// # Arguments
    /// * `ch` - Unicode character code point.
    ///
    /// # Returns
    /// Some(&TFChar) if the character is found, None otherwise.
    #[must_use]
    pub fn lookup_char(&self, ch: UniChar) -> Option<&TFChar> {
        let page_start = ch & CHARACTER_PAGE_MASK;

        let mut current = self.head_page.as_deref();
        while let Some(page) = current {
            if page.page_start == page_start {
                return page.get_char(ch);
            }
            current = page.next.as_deref();
        }
        None
    }

    /// Add a new font page to the font.
    ///
    /// # Arguments
    /// * `page` - The FontPage to add.
    pub fn add_page(&mut self, mut page: FontPage) {
        let page_start = page.page_start;
        let mut current = &mut self.head_page;

        loop {
            match current {
                Some(existing) if existing.page_start == page_start => {
                    page.next = existing.next.take();
                    *existing = Box::new(page);
                    return;
                }
                Some(existing) => {
                    current = &mut existing.next;
                }
                None => {
                    *current = Some(Box::new(page));
                    return;
                }
            }
        }
    }

    /// Get an iterator over all pages in this font.
    #[must_use]
    pub fn pages(&self) -> PageIter<'_> {
        PageIter {
            current: self.head_page.as_deref(),
        }
    }
}

/// Iterator over font pages.
///
/// Returns pages in the order they were added.
pub struct PageIter<'a> {
    current: Option<&'a FontPage>,
}

impl<'a> Iterator for PageIter<'a> {
    type Item = &'a FontPage;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        self.current = current.next.as_deref();
        Some(current)
    }
}

/// Font loading metrics.
///
/// Represents the dimensions needed to measure and draw text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontMetrics {
    /// Width of measured text.
    pub width: i16,
    /// Top of text bounding box (negative offset from baseline).
    pub top_y: i16,
    /// Bottom of text bounding box (positive offset from baseline).
    pub bot_y: i16,
}

impl FontMetrics {
    /// Create new font metrics.
    pub const fn new(width: i16, top_y: i16, bot_y: i16) -> Self {
        Self {
            width,
            top_y,
            bot_y,
        }
    }

    /// Get total height of the text bounding box.
    #[must_use]
    pub const fn height(&self) -> i16 {
        self.bot_y - self.top_y
    }
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self::ZERO
    }
}

impl FontMetrics {
    /// Empty metrics constant.
    pub const ZERO: Self = Self {
        width: 0,
        top_y: 0,
        bot_y: 0,
    };
}

/// Measure text dimensions for a string.
///
/// This function calculates the bounding box for a text string.
/// It iterates through characters in order, accumulating width and
/// tracking the minimum/maximum vertical bounds.
///
/// # Arguments
/// * `text` - A slice of Unicode characters to measure.
///
/// # Returns
/// FontMetrics containing width, top_y, and bot_y. Returns FontMetrics::ZERO
/// if the font has no characters defined or the text is empty.
#[must_use]
pub fn measure_text(font: &Font, text: &[UniChar]) -> FontMetrics {
    if text.is_empty() {
        return FontMetrics::ZERO;
    }

    let mut width: i16 = 0;
    let mut top_y: i16 = 0;
    let mut bot_y: i16 = 0;
    let mut has_char = false;

    for &ch in text {
        if let Some(tf_char) = font.lookup_char(ch) {
            if tf_char.disp.width == 0 || tf_char.disp.height == 0 {
                continue;
            }

            let char_top = -tf_char.hotspot.y;
            let char_bottom = tf_char.disp.height + tf_char.hotspot.y;

            if char_top < top_y {
                top_y = char_top;
            }
            if char_bottom > bot_y {
                bot_y = char_bottom;
            }

            width += tf_char.disp.width;
            has_char = true;
        }
    }

    if !has_char {
        return FontMetrics::ZERO;
    }

    // Subtract spacing if we have characters
    if width > 0 {
        width -= 1;
    }

    FontMetrics {
        width,
        top_y,
        bot_y,
    }
}

/// Errors that can occur during font operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontError {
    /// Character code is out of the page's valid range.
    CharOutOfRange(UniChar),
    /// Font file loading failed (placeholder for future use).
    LoadFailed(String),
}

impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontError::CharOutOfRange(ch) => {
                write!(f, "Character code 0x{:04X} is out of range", ch)
            }
            FontError::LoadFailed(msg) => {
                write!(f, "Font loading failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for FontError {}

/// Load a font from data (placeholder for resource loading).
///
/// This is a stub implementation for Phase 2. In a full implementation,
/// this would read font data from a file or resource, parse the format,
/// and populate FontPage structures with TFChar descriptors.
///
/// # Arguments
/// * `_data` - Raw font data bytes (unused in Phase 2).
///
/// # Returns
/// A Result containing the loaded Font or a FontError.
pub fn load_font(_data: &[u8]) -> Result<Font, FontError> {
    // Phase 2: Return a default empty font
    // Future phases will implement actual font file parsing
    Ok(Font::new(12, 8)) // Default leading and leading_width
}

/// Draw text at the specified position (placeholder).
///
/// This is a stub implementation for Phase 2. The actual drawing
/// implementation will come in later phases when SDL integration
/// is available.
///
/// # Arguments
/// * `_font` - The font to use for drawing.
/// * `_text` - The text characters to draw.
/// * `_x` - X coordinate of the baseline starting point.
/// * `_y` - Y coordinate of the baseline starting point.
///
/// # Returns
/// The FontMetrics for the drawn text.
#[allow(clippy::too_many_arguments)]
pub fn draw_text(_font: &Font, _text: &[UniChar], _x: i16, _y: i16) -> FontMetrics {
    // Phase 2: Return empty metrics
    // Future phases will implement actual text rendering via SDL/DCQ
    FontMetrics::ZERO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extent_new() {
        let extent = Extent::new(10, 20);
        assert_eq!(extent.width, 10);
        assert_eq!(extent.height, 20);
    }

    #[test]
    fn test_extent_zero() {
        let extent = Extent::ZERO;
        assert_eq!(extent.width, 0);
        assert_eq!(extent.height, 0);
    }

    #[test]
    fn test_extent_default() {
        let extent = Extent::default();
        assert_eq!(extent, Extent::ZERO);
    }

    #[test]
    fn test_point_new() {
        let point = Point::new(5, 10);
        assert_eq!(point.x, 5);
        assert_eq!(point.y, 10);
    }

    #[test]
    fn test_point_zero() {
        let point = Point::ZERO;
        assert_eq!(point.x, 0);
        assert_eq!(point.y, 0);
    }

    #[test]
    fn test_point_default() {
        let point = Point::default();
        assert_eq!(point, Point::ZERO);
    }

    #[test]
    fn test_tfchar_default() {
        let tf_char = TFChar::default();
        assert_eq!(tf_char.extent, Extent::ZERO);
        assert_eq!(tf_char.disp, Extent::ZERO);
        assert_eq!(tf_char.hotspot, Point::ZERO);
        assert!(tf_char.data.is_none());
        assert_eq!(tf_char.pitch, 0);
    }

    #[test]
    fn test_character_page_mask() {
        assert_eq!(CHARACTER_PAGE_MASK, 0xfffff800);
        assert_eq!(CHARACTER_PAGE_SIZE, 2048);
    }

    #[test]
    fn test_font_page_new() {
        let page = FontPage::new(0x0000, 0x0020, 128);

        // Page start should be the masked value
        assert_eq!(page.page_start(), 0x0000);
        assert_eq!(page.first_char(), 0x0020);
        assert_eq!(page.num_chars(), 128);
    }

    #[test]
    fn test_font_page_new_masks_page_start() {
        // Page starting at 0x1000 should be masked to 0x1000 (2048-char page boundary)
        let page = FontPage::new(0x1000, 0x1000, 256);
        assert_eq!(page.page_start(), 0x1000);
    }

    #[test]
    #[should_panic(expected = "num_chars must be positive")]
    fn test_font_page_new_zero_chars_panics() {
        let _page = FontPage::new(0x0000, 0x0020, 0);
    }

    #[test]
    fn test_font_page_contains_char() {
        let page = FontPage::new(0x0000, 0x0020, 10);

        assert!(page.contains_char(0x0020));
        assert!(page.contains_char(0x0029));
        assert!(!page.contains_char(0x001F));
        assert!(!page.contains_char(0x002A));
    }

    #[test]
    fn test_font_page_get_char_no_data() {
        let page = FontPage::new(0x0000, 0x0020, 10);

        // All chars start as None
        assert!(page.get_char(0x0020).is_none());
        assert!(page.get_char(0x0025).is_none());
    }

    #[test]
    fn test_font_page_get_char_out_of_bounds() {
        let page = FontPage::new(0x0000, 0x0020, 10);

        assert!(page.get_char(0x001F).is_none());
        assert!(page.get_char(0x002A).is_none());
    }

    #[test]
    fn test_font_page_set_and_get_char() {
        let mut page = FontPage::new(0x0000, 0x0020, 10);

        let tf_char = TFChar {
            extent: Extent::new(8, 12),
            disp: Extent::new(8, 12),
            hotspot: Point::new(0, -10),
            data: None,
            pitch: 8,
        };

        assert!(page.set_char(0x0023, tf_char.clone()).is_ok());
        let retrieved = page.get_char(0x0023);
        assert!(retrieved.is_some());

        let tf_char_ref = retrieved.unwrap();
        assert_eq!(tf_char_ref.extent.width, 8);
        assert_eq!(tf_char_ref.extent.height, 12);
        assert_eq!(tf_char_ref.hotspot.y, -10);
    }

    #[test]
    fn test_font_page_set_char_out_of_range() {
        let mut page = FontPage::new(0x0000, 0x0020, 10);

        let tf_char = TFChar::default();

        let result = page.set_char(0x0030, tf_char);
        assert!(matches!(result, Err(FontError::CharOutOfRange(_))));
    }

    #[test]
    fn test_font_page_accessors() {
        let page = FontPage::new(0x0000, 0x0040, 256);

        assert_eq!(page.page_start(), 0x0000);
        assert_eq!(page.first_char(), 0x0040);
        assert_eq!(page.num_chars(), 256);
    }

    #[test]
    fn test_font_new() {
        let font = Font::new(16, 10);

        assert_eq!(font.leading(), 16);
        assert_eq!(font.leading_width(), 10);
    }

    #[test]
    fn test_font_lookup_char_no_pages() {
        let font = Font::new(16, 10);

        assert!(font.lookup_char('A' as UniChar).is_none());
    }

    #[test]
    fn test_font_add_page() {
        let mut font = Font::new(16, 10);
        let page = FontPage::new(0x0000, 0x0020, 128);

        font.add_page(page);

        // Page was added
        assert!(font.head_page.is_some());
    }

    #[test]
    fn test_font_lookup_char_with_page() {
        let mut font = Font::new(16, 10);
        let mut page = FontPage::new(0x0000, 0x0041, 26); // 'A' through 'Z'

        // Add character 'A'
        let mut tf_char = TFChar::default();
        tf_char.disp = Extent::new(8, 12);
        tf_char.hotspot = Point::new(0, -10);
        page.set_char('A' as UniChar, tf_char).unwrap();

        font.add_page(page);

        let result = font.lookup_char('A' as UniChar);
        assert!(result.is_some());

        let tf_char_ref = result.unwrap();
        assert_eq!(tf_char_ref.disp.width, 8);
        assert_eq!(tf_char_ref.disp.height, 12);
    }

    #[test]
    fn test_font_lookup_char_wrong_page() {
        let mut font = Font::new(16, 10);
        let page = FontPage::new(0x0000, 0x0041, 26); // 'A' through 'Z'

        font.add_page(page);

        // Character in a different page should return None
        assert!(font.lookup_char(0x3000).is_none());
    }

    #[test]
    fn test_font_pages_iterator() {
        let mut font = Font::new(16, 10);
        let page = FontPage::new(0x0000, 0x0020, 128);

        font.add_page(page);

        let mut iter = font.pages();
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_font_metrics_new() {
        let metrics = FontMetrics::new(100, -5, 10);

        assert_eq!(metrics.width, 100);
        assert_eq!(metrics.top_y, -5);
        assert_eq!(metrics.bot_y, 10);
    }

    #[test]
    fn test_font_metrics_height() {
        let metrics = FontMetrics::new(100, -5, 10);
        assert_eq!(metrics.height(), 15);
    }

    #[test]
    fn test_font_metrics_zero() {
        let metrics = FontMetrics::ZERO;
        assert_eq!(metrics.width, 0);
        assert_eq!(metrics.top_y, 0);
        assert_eq!(metrics.bot_y, 0);
    }

    #[test]
    fn test_font_metrics_default() {
        let metrics = FontMetrics::default();
        assert_eq!(metrics, FontMetrics::ZERO);
    }

    #[test]
    fn test_measure_text_empty() {
        let font = Font::new(16, 10);
        let text: &[UniChar] = &[];

        let metrics = measure_text(&font, text);
        assert_eq!(metrics, FontMetrics::ZERO);
    }

    #[test]
    fn test_measure_text_no_pages() {
        let font = Font::new(16, 10);
        let text = "hello".chars().map(|c| c as UniChar).collect::<Vec<_>>();

        let metrics = measure_text(&font, &text);
        assert_eq!(metrics, FontMetrics::ZERO);
    }

    #[test]
    fn test_measure_text_single_char() {
        let mut font = Font::new(16, 10);
        let mut page = FontPage::new(0x0000, 0x0041, 1);

        let mut tf_char = TFChar::default();
        tf_char.disp = Extent::new(8, 12);
        tf_char.hotspot = Point::new(0, -10);
        page.set_char('A' as UniChar, tf_char).unwrap();

        font.add_page(page);

        let text = "A".chars().map(|c| c as UniChar).collect::<Vec<_>>();
        let metrics = measure_text(&font, &text);

        // Width is disp.width - 1
        assert_eq!(metrics.width, 7);
        assert_eq!(metrics.top_y, 0); // 0 - (-10)
        assert_eq!(metrics.bot_y, 2); // 0 - (-10) + 12
    }

    #[test]
    fn test_measure_text_multiple_chars() {
        let mut font = Font::new(16, 10);
        let mut page = FontPage::new(0x0000, 0x0041, 2);

        // Add 'A'
        let mut tf_char = TFChar::default();
        tf_char.disp = Extent::new(8, 12);
        tf_char.hotspot = Point::new(0, -10);
        page.set_char('A' as UniChar, tf_char).unwrap();

        // Add 'B'
        let mut tf_char = TFChar::default();
        tf_char.disp = Extent::new(10, 12);
        tf_char.hotspot = Point::new(0, -10);
        page.set_char('B' as UniChar, tf_char).unwrap();

        font.add_page(page);

        let text = "AB".chars().map(|c| c as UniChar).collect::<Vec<_>>();
        let metrics = measure_text(&font, &text);

        // Width is 8 + 10 - 1 = 17
        assert_eq!(metrics.width, 17);
        assert_eq!(metrics.top_y, 0);
        assert_eq!(metrics.bot_y, 2);
    }

    #[test]
    fn test_measure_text_mixed_height() {
        let mut font = Font::new(16, 10);
        let mut page = FontPage::new(0x0000, 0x0041, 2);

        // Add character with different vertical extents
        let mut tf_char = TFChar::default();
        tf_char.disp = Extent::new(8, 10);
        tf_char.hotspot = Point::new(0, -8);
        page.set_char('A' as UniChar, tf_char).unwrap();

        let mut tf_char = TFChar::default();
        tf_char.disp = Extent::new(8, 15);
        tf_char.hotspot = Point::new(0, -12);
        page.set_char('B' as UniChar, tf_char).unwrap();

        font.add_page(page);

        let text = "AB".chars().map(|c| c as UniChar).collect::<Vec<_>>();
        let metrics = measure_text(&font, &text);

        // Top should be min(8, 12) = 8
        assert_eq!(metrics.top_y, 0);
        // Bottom should be max(2, 3) = 3
        assert_eq!(metrics.bot_y, 3);
    }

    #[test]
    fn test_measure_text_missing_chars() {
        let mut font = Font::new(16, 10);
        let mut page = FontPage::new(0x0000, 0x0041, 1);

        // Only add 'A'
        let mut tf_char = TFChar::default();
        tf_char.disp = Extent::new(8, 12);
        tf_char.hotspot = Point::new(0, -10);
        page.set_char('A' as UniChar, tf_char).unwrap();

        font.add_page(page);

        // 'B' is not defined, should be skipped
        let text = "AB".chars().map(|c| c as UniChar).collect::<Vec<_>>();
        let metrics = measure_text(&font, &text);

        // Only 'A' contributes: 8 - 1 = 7
        assert_eq!(metrics.width, 7);
    }

    #[test]
    fn test_page_lookup_correct_page() {
        let mut font = Font::new(16, 10);

        // First page at 0x0000, first_char=0x0020, 32 chars (0x0020-0x003F)
        let mut page1 = FontPage::new(0x0000, 0x0020, 32);
        let mut tf_char = TFChar::default();
        tf_char.disp = Extent::new(8, 12);
        page1.set_char(0x0025, tf_char).unwrap(); // '%' character
        font.add_page(page1);

        // Second page at 0x2000, first_char=0x2000, 32 chars (0x2000-0x201F)
        let mut page2 = FontPage::new(0x2000, 0x2000, 32);
        tf_char = TFChar::default();
        tf_char.disp = Extent::new(10, 14);
        page2.set_char(0x2005, tf_char).unwrap();
        font.add_page(page2);

        // Lookup should find character in first page
        let result1 = font.lookup_char(0x0025);
        assert!(result1.is_some());
        assert_eq!(result1.unwrap().disp.width, 8);

        // Lookup should find character in second page
        let result2 = font.lookup_char(0x2005);
        assert!(result2.is_some());
        assert_eq!(result2.unwrap().disp.width, 10);
    }

    #[test]
    fn test_font_error_display() {
        let err = FontError::CharOutOfRange(0xFFFF);
        let s = format!("{}", err);
        assert!(s.contains("0xFFFF"));
        assert!(s.contains("out of range"));

        let err = FontError::LoadFailed("test error".to_string());
        let s = format!("{}", err);
        // Display impl says "Font loading failed: <msg>"
        assert!(s.contains("test error"));
    }

    #[test]
    fn test_load_font_stub() {
        let data = &[0u8; 100];
        let result = load_font(data);
        assert!(result.is_ok());

        let font = result.unwrap();
        assert_eq!(font.leading(), 12);
        assert_eq!(font.leading_width(), 8);
    }

    #[test]
    fn test_draw_text_stub() {
        let font = Font::new(16, 10);
        let text = "test".chars().map(|c| c as UniChar).collect::<Vec<_>>();

        let metrics = draw_text(&font, &text, 0, 0);
        assert_eq!(metrics, FontMetrics::ZERO);
    }

    #[test]
    fn test_page_lookup_algorithm() {
        // Verify the page lookup algorithm matches C implementation
        let test_cases = vec![
            (0x0000, 0x0000),
            (0x07FF, 0x0000),
            (0x0800, 0x0800),
            (0x0FFF, 0x0800),
            (0x1000, 0x1000),
            (0x1800, 0x1800),
            (0x1FFF, 0x1800),
        ];

        for (ch, expected_page) in test_cases {
            assert_eq!(ch & CHARACTER_PAGE_MASK, expected_page);
        }
    }
}
